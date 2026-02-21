use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use serde::Deserialize;

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

    // 멘션되었거나 이미지가 있을 때만 응답
    if has_image {
        handle_image(ctx, msg, data, api_key, &channel_id_str, db).await?;
    } else if mentioned {
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

/// 멘션 텍스트에서 봇 멘션을 제거하고 검색어 추출
fn extract_query(content: &str, bot_id: serenity::UserId) -> String {
    content
        .replace(&format!("<@{}>", bot_id), "")
        .replace(&format!("<@!{}>", bot_id), "")
        .trim()
        .to_string()
}

/// tarkov.dev API 아이템 검색 결과를 Gemini 컨텍스트 문자열로 변환
fn format_items_context(items: &[crate::tarkov::models::Item]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut ctx = String::from("\n\n=== tarkov.dev API 실시간 검색 결과 ===\n");
    for item in items.iter().take(10) {
        ctx.push_str(&format!("\n■ {} ({})\n", item.name, item.short_name));
        ctx.push_str(&format!("  기본가: {}₽\n", item.base_price));
        if let Some(avg) = item.avg24h_price {
            ctx.push_str(&format!("  플리마켓 24시간 평균: {}₽\n", avg));
        }
        if let Some(low) = item.low24h_price {
            ctx.push_str(&format!("  플리마켓 24시간 최저: {}₽\n", low));
        }
        if let Some(high) = item.high24h_price {
            ctx.push_str(&format!("  플리마켓 24시간 최고: {}₽\n", high));
        }
        if !item.sell_for.is_empty() {
            ctx.push_str("  판매처:\n");
            for sf in &item.sell_for {
                ctx.push_str(&format!(
                    "    - {} → {} {}\n",
                    sf.vendor.name, sf.price, sf.currency
                ));
            }
        }
        if !item.categories.is_empty() {
            let cats: Vec<&str> = item.categories.iter().map(|c| c.name.as_str()).collect();
            ctx.push_str(&format!("  카테고리: {}\n", cats.join(", ")));
        }
    }
    ctx.push_str(&format!("\n총 {}개 검색됨", items.len()));
    ctx
}

#[derive(Deserialize)]
struct ItemsResponse {
    items: Vec<crate::tarkov::models::Item>,
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

    let user_text = if msg.content.is_empty() {
        None
    } else {
        Some(msg.content.as_str())
    };

    let _typing = msg.channel_id.start_typing(&ctx.http);

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
        Err(e) => {
            tracing::error!("Gemini 이미지 분석 오류: {e}");
        }
    }

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
    let recent = db.recent_messages(channel_id_str, 50);

    // 멘션에서 검색어 추출 후 tarkov API 검색
    let query = extract_query(&msg.content, bot_id);
    let tarkov_context = if !query.is_empty() {
        match crate::tarkov::client::query::<ItemsResponse>(
            &data.http_client,
            &data.tarkov_cache,
            crate::tarkov::queries::ITEMS_QUERY,
            &serde_json::json!({"name": query, "lang": "ko"}),
        )
        .await
        {
            Ok(resp) => {
                let ctx_str = format_items_context(&resp.items);
                if ctx_str.is_empty() {
                    None
                } else {
                    Some(ctx_str)
                }
            }
            Err(e) => {
                tracing::warn!("타르코프 API 검색 실패: {e}");
                None
            }
        }
    } else {
        None
    };

    let _typing = msg.channel_id.start_typing(&ctx.http);

    match super::gemini::chat(
        &data.http_client,
        api_key,
        &recent,
        tarkov_context.as_deref(),
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
