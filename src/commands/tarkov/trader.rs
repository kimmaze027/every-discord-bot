use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::{client, embed, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct TradersData {
    traders: Vec<crate::tarkov::models::Trader>,
}

async fn trader_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let result: Result<TradersData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::TRADERS_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_traders = match result {
        Ok(data) => data.traders,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Filter by name (case-insensitive partial match)
    let name_lower = name.to_lowercase();
    let filtered: Vec<_> = all_traders
        .into_iter()
        .filter(|t| t.name.to_lowercase().contains(&name_lower))
        .collect();

    if filtered.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    if filtered.len() == 1 {
        let trader = &filtered[0];
        ctx.send(CreateReply::default().embed(embed::trader_info(trader)))
            .await?;
        return Ok(());
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = filtered
        .iter()
        .take(25)
        .map(|t| {
            let desc = t
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(50)
                .collect::<String>();
            (t.id.clone(), t.name.clone(), desc)
        })
        .collect();

    let over_25 = filtered.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", filtered.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("상인 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_trader_select", "상인을 선택하세요", options);

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
        if interaction.data.custom_id == "tarkov_trader_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_id) = values.first() {
                    if let Some(trader) = filtered.iter().find(|t| t.id == *selected_id) {
                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::trader_info(trader))
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

/// 상인 정보를 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn trader(
    ctx: Context<'_>,
    #[description = "상인 이름"] name: String,
) -> Result<(), Error> {
    trader_impl(ctx, name).await
}

/// 상인 정보를 검색합니다 (/trader 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 상인(
    ctx: Context<'_>,
    #[description = "상인 이름"] name: String,
) -> Result<(), Error> {
    trader_impl(ctx, name).await
}
