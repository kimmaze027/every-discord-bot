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

async fn item_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
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
        let tabs = tab_components("info", &item.id);
        let reply = ctx
            .send(
                CreateReply::default()
                    .embed(embed::item_detail(item))
                    .components(tabs),
            )
            .await?;

        let mut msg = reply.message().await?.into_owned();
        handle_item_tabs(ctx, &mut msg, &items).await?;
        return Ok(());
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = items
        .iter()
        .take(25)
        .map(|item| {
            let desc = item
                .categories
                .first()
                .map(|c| c.name.clone())
                .unwrap_or_default();
            (item.id.clone(), item.name.clone(), desc)
        })
        .collect();

    let over_25 = items.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", items.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("아이템 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_item_select", "아이템을 선택하세요", options);

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
        if interaction.data.custom_id == "tarkov_item_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_id) = values.first() {
                    if let Some(item) = items.iter().find(|i| i.id == *selected_id) {
                        let tabs = tab_components("info", &item.id);
                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::item_detail(item))
                                        .components(tabs),
                                ),
                            )
                            .await?;

                        let mut msg = reply.message().await?.into_owned();
                        handle_item_tabs(ctx, &mut msg, &items).await?;
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

/// Handle tab button interactions on an item detail view.
async fn handle_item_tabs(
    ctx: Context<'_>,
    msg: &mut serenity::Message,
    items: &[crate::tarkov::models::Item],
) -> Result<(), Error> {
    while let Some(interaction) = components::await_component_interaction(ctx, msg, 300).await {
        let custom_id = &interaction.data.custom_id;

        // Parse custom_id: tarkov_item_{tab}_{item_id}
        let parts: Vec<&str> = custom_id.splitn(4, '_').collect();
        if parts.len() < 4 || parts[0] != "tarkov" || parts[1] != "item" {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::Acknowledge,
                )
                .await?;
            continue;
        }

        let tab = parts[2];
        let item_id = parts[3];

        if let Some(item) = items.iter().find(|i| i.id == item_id) {
            let new_embed = match tab {
                "info" => embed::item_detail(item),
                "price" => embed::item_price(item),
                _ => embed::item_detail(item),
            };
            let tabs = tab_components(tab, item_id);

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

fn tab_components(active_tab: &str, item_id: &str) -> Vec<serenity::CreateActionRow> {
    let tabs = vec![
        ("info", "기본정보", active_tab == "info"),
        ("price", "가격", active_tab == "price"),
    ];
    vec![components::tab_buttons("item", item_id, &tabs)]
}

/// 아이템을 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn item(
    ctx: Context<'_>,
    #[description = "아이템 이름"] name: String,
) -> Result<(), Error> {
    item_impl(ctx, name).await
}

/// 아이템을 검색합니다 (/item 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 아이템(
    ctx: Context<'_>,
    #[description = "아이템 이름"] name: String,
) -> Result<(), Error> {
    item_impl(ctx, name).await
}
