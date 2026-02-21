use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::{client, embed, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct ItemsData {
    items: Vec<crate::tarkov::models::Item>,
}

async fn price_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let result: Result<ItemsData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::ITEMS_QUERY,
        &serde_json::json!({"name": name, "lang": "ko"}),
    )
    .await;

    let items = match result {
        Ok(data) => data.items,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    if items.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    if items.len() == 1 {
        let item = &items[0];
        ctx.send(CreateReply::default().embed(embed::item_price(item)))
            .await?;
        return Ok(());
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = items
        .iter()
        .take(25)
        .map(|item| {
            let price_desc = item
                .avg24h_price
                .filter(|&p| p > 0)
                .map(|p| format!("~{} RUB", format_price(p)))
                .unwrap_or_else(|| "가격 정보 없음".to_string());
            (item.id.clone(), item.name.clone(), price_desc)
        })
        .collect();

    let over_25 = items.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", items.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("가격 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_price_select", "아이템을 선택하세요", options);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(search_embed)
                .components(vec![select_row]),
        )
        .await?;

    let msg = reply.message().await?.into_owned();

    // Wait for select menu interaction
    if let Some(interaction) = components::await_component_interaction(ctx, &msg, 300).await {
        if interaction.data.custom_id == "tarkov_price_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_id) = values.first() {
                    if let Some(item) = items.iter().find(|i| i.id == *selected_id) {
                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::item_price(item))
                                        .components(vec![]),
                                ),
                            )
                            .await?;
                        return Ok(());
                    }
                }
            }
        }

        interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::Acknowledge,
            )
            .await?;
    }

    // Timeout: remove components
    let mut msg = reply.message().await?.into_owned();
    msg.edit(
        ctx.serenity_context(),
        serenity::EditMessage::new().components(vec![]),
    )
    .await
    .ok();

    Ok(())
}

/// Simple price formatter with comma separators.
fn format_price(n: i64) -> String {
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

/// 아이템 가격을 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn price(
    ctx: Context<'_>,
    #[description = "아이템 이름"] name: String,
) -> Result<(), Error> {
    price_impl(ctx, name).await
}

/// 아이템 가격을 검색합니다 (/price 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 가격(
    ctx: Context<'_>,
    #[description = "아이템 이름"] name: String,
) -> Result<(), Error> {
    price_impl(ctx, name).await
}
