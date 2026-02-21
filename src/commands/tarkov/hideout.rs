use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::models::HideoutStation;
use crate::tarkov::{client, embed, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct HideoutData {
    #[serde(rename = "hideoutStations")]
    hideout_stations: Vec<HideoutStation>,
}

async fn hideout_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let result: Result<HideoutData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::HIDEOUT_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_stations = match result {
        Ok(data) => data.hideout_stations,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Filter by name (case-insensitive partial match)
    let name_lower = name.to_lowercase();
    let filtered: Vec<HideoutStation> = all_stations
        .into_iter()
        .filter(|s| s.name.to_lowercase().contains(&name_lower))
        .collect();

    if filtered.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    if filtered.len() == 1 {
        let station = &filtered[0];
        return show_hideout_with_pagination(ctx, station).await;
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = filtered
        .iter()
        .take(25)
        .map(|s| {
            let desc = format!("레벨 {}단계", s.levels.len());
            (s.id.clone(), s.name.clone(), desc)
        })
        .collect();

    let over_25 = filtered.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", filtered.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("은신처 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_hideout_select", "시설을 선택하세요", options);

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
        if interaction.data.custom_id == "tarkov_hideout_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_id) = values.first() {
                    if let Some(station) = filtered.iter().find(|s| s.id == *selected_id) {
                        // Acknowledge first, then send new message with pagination
                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::hideout_info(station, 0))
                                        .components(if station.levels.len() > 1 {
                                            vec![components::pagination_row(
                                                "hideout",
                                                0,
                                                station.levels.len(),
                                            )]
                                        } else {
                                            vec![]
                                        }),
                                ),
                            )
                            .await?;

                        // Continue pagination loop if multiple levels
                        if station.levels.len() > 1 {
                            let mut inner_msg = reply.message().await?.into_owned();
                            let mut level_idx: usize = 0;
                            let total_levels = station.levels.len();

                            while let Some(inner_interaction) =
                                components::await_component_interaction(ctx, &inner_msg, 300).await
                            {
                                let custom_id = &inner_interaction.data.custom_id;

                                if let Some(page_str) =
                                    custom_id.strip_prefix("tarkov_hideout_prev_")
                                {
                                    if let Ok(p) = page_str.parse::<usize>() {
                                        level_idx = p;
                                    }
                                } else if let Some(page_str) =
                                    custom_id.strip_prefix("tarkov_hideout_next_")
                                {
                                    if let Ok(p) = page_str.parse::<usize>() {
                                        level_idx = p;
                                    }
                                }

                                level_idx = level_idx.min(total_levels.saturating_sub(1));

                                let new_embed = embed::hideout_info(station, level_idx);
                                let new_page_row =
                                    components::pagination_row("hideout", level_idx, total_levels);

                                inner_interaction
                                    .create_response(
                                        ctx.serenity_context(),
                                        serenity::CreateInteractionResponse::UpdateMessage(
                                            serenity::CreateInteractionResponseMessage::new()
                                                .embed(new_embed)
                                                .components(vec![new_page_row]),
                                        ),
                                    )
                                    .await?;
                            }

                            // Timeout: remove components
                            inner_msg
                                .edit(
                                    ctx.serenity_context(),
                                    serenity::EditMessage::new().components(vec![]),
                                )
                                .await
                                .ok();
                        }

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

/// Show hideout station with level pagination.
async fn show_hideout_with_pagination(
    ctx: Context<'_>,
    station: &HideoutStation,
) -> Result<(), Error> {
    let total_levels = station.levels.len();
    let mut level_idx: usize = 0;

    let initial_embed = embed::hideout_info(station, level_idx);

    if total_levels <= 1 {
        ctx.send(CreateReply::default().embed(initial_embed))
            .await?;
        return Ok(());
    }

    let page_row = components::pagination_row("hideout", level_idx, total_levels);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(initial_embed)
                .components(vec![page_row]),
        )
        .await?;

    let mut msg = reply.message().await?.into_owned();

    // Interaction loop for pagination
    while let Some(interaction) = components::await_component_interaction(ctx, &msg, 300).await {
        let custom_id = &interaction.data.custom_id;

        if let Some(page_str) = custom_id.strip_prefix("tarkov_hideout_prev_") {
            if let Ok(p) = page_str.parse::<usize>() {
                level_idx = p;
            }
        } else if let Some(page_str) = custom_id.strip_prefix("tarkov_hideout_next_") {
            if let Ok(p) = page_str.parse::<usize>() {
                level_idx = p;
            }
        }

        level_idx = level_idx.min(total_levels.saturating_sub(1));

        let new_embed = embed::hideout_info(station, level_idx);
        let new_page_row = components::pagination_row("hideout", level_idx, total_levels);

        interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(new_embed)
                        .components(vec![new_page_row]),
                ),
            )
            .await?;
    }

    // Timeout: remove components
    msg.edit(
        ctx.serenity_context(),
        serenity::EditMessage::new().components(vec![]),
    )
    .await
    .ok();

    Ok(())
}

/// 은신처 시설 정보를 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn hideout(
    ctx: Context<'_>,
    #[description = "시설 이름"] name: String,
) -> Result<(), Error> {
    hideout_impl(ctx, name).await
}

/// 은신처 시설 정보를 검색합니다 (/hideout 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 은신처(
    ctx: Context<'_>,
    #[description = "시설 이름"] name: String,
) -> Result<(), Error> {
    hideout_impl(ctx, name).await
}
