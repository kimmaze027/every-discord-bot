use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::models::Craft;
use crate::tarkov::{client, embed, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct CraftsData {
    crafts: Vec<Craft>,
}

async fn craft_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let result: Result<CraftsData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::CRAFTS_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_crafts = match result {
        Ok(data) => data.crafts,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Filter by reward item name (case-insensitive partial match)
    let name_lower = name.to_lowercase();
    let filtered: Vec<Craft> = all_crafts
        .into_iter()
        .filter(|c| {
            c.reward_items
                .iter()
                .any(|item| item.item.name.to_lowercase().contains(&name_lower))
        })
        .collect();

    if filtered.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    if filtered.len() == 1 {
        ctx.send(CreateReply::default().embed(embed::craft_info(&filtered[0])))
            .await?;
        return Ok(());
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = filtered
        .iter()
        .take(25)
        .map(|c| {
            let reward_name = c
                .reward_items
                .first()
                .map(|r| r.item.name.as_str())
                .unwrap_or("제작품");
            let desc = format!("{} Lv.{}", c.station.name, c.level);
            (c.id.clone(), reward_name.to_string(), desc)
        })
        .collect();

    let over_25 = filtered.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", filtered.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("제작법 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_craft_select", "제작법을 선택하세요", options);

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
        if interaction.data.custom_id == "tarkov_craft_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_id) = values.first() {
                    if let Some(craft) = filtered.iter().find(|c| c.id == *selected_id) {
                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::craft_info(craft))
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

/// 제작법을 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn craft(
    ctx: Context<'_>,
    #[description = "생산품 이름"] name: String,
) -> Result<(), Error> {
    craft_impl(ctx, name).await
}

/// 제작법을 검색합니다 (/craft 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 제작(
    ctx: Context<'_>,
    #[description = "생산품 이름"] name: String,
) -> Result<(), Error> {
    craft_impl(ctx, name).await
}
