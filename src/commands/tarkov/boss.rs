use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::{client, embed, models, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct BossesData {
    bosses: Vec<models::Boss>,
}

#[derive(Deserialize)]
struct MapsData {
    maps: Vec<models::GameMap>,
}

async fn boss_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();

    // Query bosses
    let bosses_result: Result<BossesData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::BOSSES_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_bosses = match bosses_result {
        Ok(d) => d.bosses,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Query maps (for spawn location cross-reference)
    let maps_result: Result<MapsData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::MAPS_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_maps = match maps_result {
        Ok(d) => d.maps,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Filter bosses by name (case-insensitive partial match)
    let name_lower = name.to_lowercase();
    let filtered: Vec<_> = all_bosses
        .into_iter()
        .filter(|b| b.name.to_lowercase().contains(&name_lower))
        .collect();

    if filtered.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    if filtered.len() == 1 {
        let boss = &filtered[0];
        let spawn_maps = build_spawn_maps(boss, &all_maps);
        let tabs = tab_components("info", &boss.name);
        let reply = ctx
            .send(
                CreateReply::default()
                    .embed(embed::boss_info(boss, "info", &spawn_maps))
                    .components(tabs),
            )
            .await?;

        let mut msg = reply.message().await?.into_owned();
        handle_boss_tabs(ctx, &mut msg, &filtered, &all_maps).await?;
        return Ok(());
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = filtered
        .iter()
        .take(25)
        .map(|b| {
            let desc = if let Some(ref health) = b.health {
                let total: i32 = health.iter().map(|h| h.max).sum();
                format!("체력 {total} HP")
            } else {
                String::new()
            };
            (b.name.clone(), b.name.clone(), desc)
        })
        .collect();

    let over_25 = filtered.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", filtered.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("보스 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_boss_select", "보스를 선택하세요", options);

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
        if interaction.data.custom_id == "tarkov_boss_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_name) = values.first() {
                    if let Some(boss) = filtered.iter().find(|b| b.name == *selected_name) {
                        let spawn_maps = build_spawn_maps(boss, &all_maps);
                        let tabs = tab_components("info", &boss.name);
                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::boss_info(boss, "info", &spawn_maps))
                                        .components(tabs),
                                ),
                            )
                            .await?;

                        let mut msg = reply.message().await?.into_owned();
                        handle_boss_tabs(ctx, &mut msg, &filtered, &all_maps).await?;
                        return Ok(());
                    }
                }
            }
        }

        // Fallback: acknowledge unknown interaction
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

/// Build spawn_maps by cross-referencing map data for a given boss.
fn build_spawn_maps(boss: &models::Boss, maps: &[models::GameMap]) -> Vec<(String, f64)> {
    maps.iter()
        .filter_map(|m| {
            m.bosses
                .iter()
                .find(|b| b.name == boss.name)
                .map(|b| (m.name.clone(), b.spawn_chance))
        })
        .collect()
}

/// Handle tab button interactions on a boss detail view.
async fn handle_boss_tabs(
    ctx: Context<'_>,
    msg: &mut serenity::Message,
    bosses: &[models::Boss],
    maps: &[models::GameMap],
) -> Result<(), Error> {
    while let Some(interaction) = components::await_component_interaction(ctx, msg, 300).await {
        let custom_id = &interaction.data.custom_id;

        // Parse custom_id: tarkov_boss_{tab}_{boss_name}
        // boss names may contain underscores, so use splitn(4, '_') and take parts[3] as name
        let parts: Vec<&str> = custom_id.splitn(4, '_').collect();
        if parts.len() < 4 || parts[0] != "tarkov" || parts[1] != "boss" {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::Acknowledge,
                )
                .await?;
            continue;
        }

        let tab = parts[2];
        let boss_name = parts[3];

        if let Some(boss) = bosses.iter().find(|b| b.name == boss_name) {
            let spawn_maps = build_spawn_maps(boss, maps);
            let new_embed = embed::boss_info(boss, tab, &spawn_maps);
            let tabs = tab_components(tab, boss_name);

            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(new_embed)
                            .components(tabs),
                    ),
                )
                .await?;
        } else {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::Acknowledge,
                )
                .await?;
        }
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

fn tab_components(active_tab: &str, boss_name: &str) -> Vec<serenity::CreateActionRow> {
    let tabs = vec![
        ("info", "기본정보", active_tab == "info"),
        ("equip", "장비", active_tab == "equip"),
        ("drops", "드롭 아이템", active_tab == "drops"),
        ("spawns", "스폰 위치", active_tab == "spawns"),
    ];
    vec![components::tab_buttons("boss", boss_name, &tabs)]
}

/// 보스 정보를 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn boss(
    ctx: Context<'_>,
    #[description = "보스 이름"] name: String,
) -> Result<(), Error> {
    boss_impl(ctx, name).await
}

/// 보스 정보를 검색합니다 (/boss 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 보스(
    ctx: Context<'_>,
    #[description = "보스 이름"] name: String,
) -> Result<(), Error> {
    boss_impl(ctx, name).await
}
