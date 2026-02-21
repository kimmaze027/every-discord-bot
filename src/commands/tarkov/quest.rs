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

async fn quest_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
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

    // Filter by name (case-insensitive partial match)
    let name_lower = name.to_lowercase();
    let filtered: Vec<&Task> = all_tasks
        .iter()
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
        let task = filtered[0];
        let (prev, next) = build_quest_chain(task, &all_tasks);
        let prev_refs: Vec<&str> = prev.iter().map(|s| s.as_str()).collect();
        let next_refs: Vec<&str> = next.iter().map(|s| s.as_str()).collect();

        ctx.send(CreateReply::default().embed(embed::quest_info(task, &prev_refs, &next_refs)))
            .await?;
        return Ok(());
    }

    // Multiple results: show select menu
    let options: Vec<(String, String, String)> = filtered
        .iter()
        .take(25)
        .map(|t| {
            let desc = format!("{} | Lv.{}", t.trader.name, t.min_player_level.unwrap_or(0));
            (t.id.clone(), t.name.clone(), desc)
        })
        .collect();

    let over_25 = filtered.len() > 25;
    let mut embed_desc = format!("**{}건**의 검색 결과", filtered.len());
    if over_25 {
        embed_desc.push_str("\n결과가 많습니다. 더 구체적으로 검색해주세요.");
    }

    let search_embed = serenity::CreateEmbed::new()
        .title(format!("퀘스트 검색: {name}"))
        .description(embed_desc)
        .color(0xC8AA6E);

    let select_row =
        components::item_select_menu("tarkov_quest_select", "퀘스트를 선택하세요", options);

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
        if interaction.data.custom_id == "tarkov_quest_select" {
            if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                &interaction.data.kind
            {
                if let Some(selected_id) = values.first() {
                    if let Some(task) = all_tasks.iter().find(|t| t.id == *selected_id) {
                        let (prev, next) = build_quest_chain(task, &all_tasks);
                        let prev_refs: Vec<&str> = prev.iter().map(|s| s.as_str()).collect();
                        let next_refs: Vec<&str> = next.iter().map(|s| s.as_str()).collect();

                        interaction
                            .create_response(
                                ctx.serenity_context(),
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embed(embed::quest_info(task, &prev_refs, &next_refs))
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

/// Build quest chain data: previous quests (from task_requirements) and
/// next quests (by scanning all tasks for those that require this quest).
fn build_quest_chain(task: &Task, all_tasks: &[Task]) -> (Vec<String>, Vec<String>) {
    // Previous quests: directly from task_requirements
    let prev: Vec<String> = task
        .task_requirements
        .iter()
        .map(|r| r.task.name.clone())
        .collect();

    // Next quests: find all tasks whose task_requirements contain this quest's ID
    let next: Vec<String> = all_tasks
        .iter()
        .filter(|t| t.task_requirements.iter().any(|r| r.task.id == task.id))
        .map(|t| t.name.clone())
        .collect();

    (prev, next)
}

/// 퀘스트 정보를 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn quest(
    ctx: Context<'_>,
    #[description = "퀘스트 이름"] name: String,
) -> Result<(), Error> {
    quest_impl(ctx, name).await
}

/// 퀘스트 정보를 검색합니다 (/quest 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 퀘스트(
    ctx: Context<'_>,
    #[description = "퀘스트 이름"] name: String,
) -> Result<(), Error> {
    quest_impl(ctx, name).await
}
