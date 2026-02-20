use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::models::Task;
use crate::tarkov::{client, embed, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct TasksData {
    tasks: Vec<Task>,
}

async fn questitem_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let result: Result<TasksData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::TASKS_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_tasks = match result {
        Ok(data) => data.tasks,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Find all tasks whose objectives mention the item name (case-insensitive)
    let name_lower = name.to_lowercase();
    let matching_quests: Vec<(String, String)> = all_tasks
        .iter()
        .filter(|t| {
            t.objectives
                .iter()
                .any(|obj| obj.description.to_lowercase().contains(&name_lower))
        })
        .map(|t| (t.name.clone(), t.trader.name.clone()))
        .collect();

    if matching_quests.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    let total_pages = matching_quests.len().div_ceil(10).max(1);
    let mut page: usize = 0;

    let quest_embed = embed::questitem_list(&matching_quests, &name, page);

    if total_pages <= 1 {
        // Single page, no pagination needed
        ctx.send(CreateReply::default().embed(quest_embed)).await?;
        return Ok(());
    }

    let page_row = components::pagination_row("questitem", page, total_pages);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(quest_embed)
                .components(vec![page_row]),
        )
        .await?;

    let mut msg = reply.message().await?.into_owned();

    // Interaction loop for pagination
    while let Some(interaction) = components::await_component_interaction(ctx, &msg, 300).await {
        let custom_id = &interaction.data.custom_id;

        if let Some(page_str) = custom_id.strip_prefix("tarkov_questitem_prev_") {
            if let Ok(p) = page_str.parse::<usize>() {
                page = p;
            }
        } else if let Some(page_str) = custom_id.strip_prefix("tarkov_questitem_next_") {
            if let Ok(p) = page_str.parse::<usize>() {
                page = p;
            }
        }

        page = page.min(total_pages.saturating_sub(1));

        let new_embed = embed::questitem_list(&matching_quests, &name, page);
        let new_page_row = components::pagination_row("questitem", page, total_pages);

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

/// 퀘스트에 필요한 아이템을 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn questitem(
    ctx: Context<'_>,
    #[description = "아이템 이름"] name: String,
) -> Result<(), Error> {
    questitem_impl(ctx, name).await
}

/// 퀘스트에 필요한 아이템을 검색합니다 (/questitem 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 퀘스트아이템(
    ctx: Context<'_>,
    #[description = "아이템 이름"] name: String,
) -> Result<(), Error> {
    questitem_impl(ctx, name).await
}
