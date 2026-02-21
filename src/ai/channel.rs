use crate::{Data, Error};
use poise::serenity_prelude::{self as serenity, CreateMessage};
use serde::Deserialize;
use std::time::Instant;

const PENDING_TIMEOUT_SECS: u64 = 120;

pub async fn handle(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    data: &Data,
) -> Result<(), Error> {
    let (api_key, tv_channel_id, db) =
        match (&data.gemini_api_key, data.tv_channel_id, &data.chat_db) {
            (Some(key), Some(id), Some(db)) => (key, id, db),
            _ => return Ok(()),
        };

    if msg.channel_id.get() != tv_channel_id {
        return Ok(());
    }

    if msg.author.bot {
        return Ok(());
    }

    let channel_id_str = tv_channel_id.to_string();
    let has_image = msg.attachments.iter().any(is_image_attachment);
    let bot_id = ctx.cache.current_user().id;
    let mentioned = msg.mentions.iter().any(|u| u.id == bot_id);

    // 모든 메시지를 DB에 저장 (컨텍스트용)
    db.insert_message(
        &channel_id_str,
        &msg.author.id.to_string(),
        &msg.author.name,
        &msg.content,
        false,
        has_image,
    )?;

    // 1. 이미지 첨부 시 Gemini 이미지 분석
    if has_image {
        handle_image(ctx, msg, data, api_key, &channel_id_str, db).await?;
    }
    // 2. 대기 중인 아이템 선택이 있으면 처리 (태그 불필요)
    else if handle_pending_selection(ctx, msg, data, &channel_id_str, db).await? {
        // handled
    }
    // 3. 멘션 시 텍스트 처리
    else if mentioned {
        handle_text(ctx, msg, data, api_key, &channel_id_str, db, bot_id).await?;
    }

    // 오래된 메시지 정리
    db.cleanup_old(&channel_id_str, 200);

    Ok(())
}

fn is_image_attachment(a: &serenity::Attachment) -> bool {
    a.content_type
        .as_ref()
        .is_some_and(|ct| ct.starts_with("image/"))
}

/// 멘션 텍스트에서 봇 멘션과 불필요한 한국어 접미사를 제거하고 검색어 추출
fn extract_query(content: &str, bot_id: serenity::UserId) -> String {
    let raw = content
        .replace(&format!("<@{}>", bot_id), "")
        .replace(&format!("<@!{}>", bot_id), "");
    let trimmed = raw.trim();

    const SUFFIXES: &[&str] = &[
        "가격",
        "시세",
        "정보",
        "어디",
        "뭐야",
        "알려줘",
        "검색",
        "찾아줘",
        "얼마",
        "어때",
        "좀",
        "가격이",
        "가격은",
        "시세는",
        "정보는",
    ];

    let mut query = trimmed.to_string();
    for suffix in SUFFIXES {
        if let Some(stripped) = query.strip_suffix(suffix) {
            query = stripped.trim().to_string();
        }
    }
    query
}

/// 텍스트에서 번호 추출 ("1", "1번", "1번이요", "2요" 등)
fn extract_number(text: &str) -> Option<usize> {
    const SUFFIXES: &[&str] = &["번이요", "번요", "번이", "번째", "번", "이요", "요"];
    let mut s = text.to_string();
    for suffix in SUFFIXES {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
            break;
        }
    }
    s.trim().parse().ok()
}

/// 대기 중인 아이템 선택 처리 (번호 입력 시 임베드 응답)
async fn handle_pending_selection(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    data: &Data,
    channel_id_str: &str,
    db: &crate::ai::db::ChatDb,
) -> Result<bool, Error> {
    let key = (msg.channel_id.get(), msg.author.id.get());

    // 번호인지 확인 ("1", "1번", "1번이요", "1이요" 등)
    let content = msg.content.trim();
    let number: usize = match extract_number(content) {
        Some(n) => n,
        None => return Ok(false),
    };

    // pending query 확인
    let item = {
        let mut pending = data.pending_queries.lock().unwrap();
        match pending.get(&key) {
            Some(pq) => {
                // 만료 확인
                if pq.created_at.elapsed().as_secs() > PENDING_TIMEOUT_SECS {
                    pending.remove(&key);
                    return Ok(false);
                }
                // 유효한 번호인지 확인
                if number == 0 || number > pq.items.len() {
                    return Ok(false);
                }
                let item = pq.items[number - 1].clone();
                pending.remove(&key);
                item
            }
            None => return Ok(false),
        }
    };

    // 아이템 가격 임베드 전송
    let embed = crate::tarkov::embed::item_price(&item);
    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;

    db.insert_message(
        channel_id_str,
        &ctx.cache.current_user().id.to_string(),
        "EveryBot",
        &format!("[아이템 가격 정보: {}]", item.name),
        true,
        false,
    )?;

    Ok(true)
}

/// 아이템 선택지 메시지 생성
fn format_item_choices(items: &[crate::tarkov::models::Item]) -> String {
    let mut text = String::from("**여러 아이템이 검색되었습니다. 번호를 입력해주세요:**\n");
    for (i, item) in items.iter().enumerate().take(10) {
        text.push_str(&format!(
            "**{}**. {} ({})\n",
            i + 1,
            item.name,
            item.short_name
        ));
    }
    text
}

#[derive(Deserialize)]
struct ItemsResponse {
    items: Vec<crate::tarkov::models::Item>,
}

/// 카탈로그 매칭 우선, fallback으로 API 직접 조회
async fn try_find_item(
    client: &reqwest::Client,
    cache: &crate::tarkov::client::Cache,
    catalog: Option<&crate::tarkov::catalog::ItemCatalog>,
    name: &str,
) -> Option<crate::tarkov::models::Item> {
    // 1단계: 카탈로그에서 공식 이름 매칭
    let search_name = if let Some(cat) = catalog {
        match cat.find_match(name) {
            Some(entry) => entry.name,
            None => name.to_string(),
        }
    } else {
        name.to_string()
    };

    // 2단계: 공식 이름으로 가격 API 호출
    if let Ok(resp) = crate::tarkov::client::query::<ItemsResponse>(
        client,
        cache,
        crate::tarkov::queries::ITEMS_QUERY,
        &serde_json::json!({"name": search_name, "lang": "en"}),
    )
    .await
    {
        if !resp.items.is_empty() {
            return Some(resp.items.into_iter().next().unwrap());
        }
    }

    // 3단계: fallback — 원본 이름으로도 시도 (카탈로그 매칭과 다른 경우만)
    if search_name != name {
        if let Ok(resp) = crate::tarkov::client::query::<ItemsResponse>(
            client,
            cache,
            crate::tarkov::queries::ITEMS_QUERY,
            &serde_json::json!({"name": name, "lang": "en"}),
        )
        .await
        {
            if !resp.items.is_empty() {
                return Some(resp.items.into_iter().next().unwrap());
            }
        }
    }

    None
}

/// 숫자를 쉼표로 구분된 문자열로 변환
fn format_number(n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let negative = n < 0;
    let mut num = n.unsigned_abs();
    let mut parts = Vec::new();
    while num > 0 {
        parts.push(format!("{:03}", num % 1000));
        num /= 1000;
    }
    parts.reverse();
    if let Some(first) = parts.first_mut() {
        *first = first.trim_start_matches('0').to_string();
        if first.is_empty() {
            *first = "0".to_string();
        }
    }
    let result = parts.join(",");
    if negative {
        format!("-{result}")
    } else {
        result
    }
}

async fn handle_image(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    data: &Data,
    api_key: &str,
    channel_id_str: &str,
    db: &crate::ai::db::ChatDb,
) -> Result<(), Error> {
    let attachment = match msg.attachments.iter().find(|a| is_image_attachment(a)) {
        Some(a) => a,
        None => return Ok(()),
    };

    let mime_type = attachment.content_type.as_deref().unwrap_or("image/jpeg");

    let image_bytes = data
        .http_client
        .get(&attachment.url)
        .send()
        .await?
        .bytes()
        .await?;

    let _typing = msg.channel_id.start_typing(&ctx.http);

    // 1단계: Gemini로 아이템 목록 식별
    let identified =
        match super::gemini::identify_items(&data.http_client, api_key, &image_bytes, mime_type)
            .await
        {
            Ok(items) if !items.is_empty() => items,
            Ok(_) | Err(_) => {
                // 아이템 식별 실패 시 일반 이미지 분석으로 fallback
                let user_text = if msg.content.is_empty() {
                    None
                } else {
                    Some(msg.content.as_str())
                };
                match super::gemini::analyze_image(
                    &data.http_client,
                    api_key,
                    &image_bytes,
                    mime_type,
                    user_text,
                )
                .await
                {
                    Ok(response) => {
                        let response = truncate_for_discord(&response);
                        msg.channel_id.say(&ctx.http, &response).await?;
                        db.insert_message(
                            channel_id_str,
                            &ctx.cache.current_user().id.to_string(),
                            "EveryBot",
                            &response,
                            true,
                            false,
                        )?;
                    }
                    Err(e) => tracing::error!("Gemini 이미지 분석 오류: {e}"),
                }
                return Ok(());
            }
        };

    // 2단계: 각 아이템 tarkov.dev API로 가격 조회
    let mut lines: Vec<String> = Vec::new();
    let mut total: i64 = 0;

    for item_info in &identified {
        match try_find_item(
            &data.http_client,
            &data.tarkov_cache,
            data.item_catalog.as_deref(),
            &item_info.name,
        )
        .await
        {
            Some(best) => {
                let price = best.avg24h_price.unwrap_or(best.base_price);
                let item_total = price * item_info.qty as i64;
                total += item_total;

                if item_info.qty > 1 {
                    lines.push(format!(
                        "- {} x{} — {}₽ (개당 {}₽)",
                        best.name,
                        item_info.qty,
                        format_number(item_total),
                        format_number(price)
                    ));
                } else {
                    lines.push(format!("- {} — {}₽", best.name, format_number(price)));
                }
            }
            None => {
                lines.push(format!("- {} — 가격 조회 실패", item_info.name));
            }
        }
    }

    // 3단계: 결과 메시지 전송
    let mut response = String::from("**아이템 가격 분석**\n");
    for line in &lines {
        response.push_str(line);
        response.push('\n');
    }
    response.push_str(&format!("\n**합계: {}₽**", format_number(total)));

    let response = truncate_for_discord(&response);
    msg.channel_id.say(&ctx.http, &response).await?;

    db.insert_message(
        channel_id_str,
        &ctx.cache.current_user().id.to_string(),
        "EveryBot",
        &response,
        true,
        false,
    )?;

    Ok(())
}

async fn handle_text(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    data: &Data,
    api_key: &str,
    channel_id_str: &str,
    db: &crate::ai::db::ChatDb,
    bot_id: serenity::UserId,
) -> Result<(), Error> {
    let query = extract_query(&msg.content, bot_id);

    // tarkov API 검색
    if !query.is_empty() {
        match crate::tarkov::client::query::<ItemsResponse>(
            &data.http_client,
            &data.tarkov_cache,
            crate::tarkov::queries::ITEMS_QUERY,
            &serde_json::json!({"name": query, "lang": "ko"}),
        )
        .await
        {
            Ok(resp) if resp.items.len() == 1 => {
                // 1개: 바로 가격 임베드
                let embed = crate::tarkov::embed::item_price(&resp.items[0]);
                msg.channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(embed))
                    .await?;
                db.insert_message(
                    channel_id_str,
                    &ctx.cache.current_user().id.to_string(),
                    "EveryBot",
                    &format!("[아이템 가격 정보: {}]", resp.items[0].name),
                    true,
                    false,
                )?;
                return Ok(());
            }
            Ok(resp) if resp.items.len() > 1 => {
                // 2개 이상: 선택지 제시 + pending 저장
                let choices = format_item_choices(&resp.items);
                msg.channel_id.say(&ctx.http, &choices).await?;

                let key = (msg.channel_id.get(), msg.author.id.get());
                let mut pending = data.pending_queries.lock().unwrap();
                pending.insert(
                    key,
                    crate::ai::PendingQuery {
                        items: resp.items.into_iter().take(10).collect(),
                        created_at: Instant::now(),
                    },
                );

                db.insert_message(
                    channel_id_str,
                    &ctx.cache.current_user().id.to_string(),
                    "EveryBot",
                    &choices,
                    true,
                    false,
                )?;
                return Ok(());
            }
            Ok(_) => {} // 0개: Gemini로 fallback
            Err(e) => {
                tracing::warn!("타르코프 API 검색 실패: {e}");
            }
        }
    }

    // Gemini chat fallback
    let recent = db.recent_messages(channel_id_str, 50);
    let _typing = msg.channel_id.start_typing(&ctx.http);

    match super::gemini::chat(&data.http_client, api_key, &recent, None).await {
        Ok(response) => {
            let response = truncate_for_discord(&response);
            msg.channel_id.say(&ctx.http, &response).await?;
            db.insert_message(
                channel_id_str,
                &ctx.cache.current_user().id.to_string(),
                "EveryBot",
                &response,
                true,
                false,
            )?;
        }
        Err(e) => {
            tracing::error!("Gemini 채팅 오류: {e}");
        }
    }

    Ok(())
}

/// Discord 메시지 2000자 제한 처리
fn truncate_for_discord(text: &str) -> String {
    if text.len() <= 2000 {
        return text.to_string();
    }
    let mut end = 1997;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &text[..end])
}
