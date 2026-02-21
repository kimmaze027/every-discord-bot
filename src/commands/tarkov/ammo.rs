use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use crate::tarkov::models::Ammo;
use crate::tarkov::{client, embed, queries};
use crate::utils::{self, components};
use crate::{Context, Error};

#[derive(Deserialize)]
struct AmmoData {
    ammo: Vec<Ammo>,
}

/// Sort criteria for ammo table display.
#[derive(Clone, Copy)]
enum AmmoSort {
    Penetration,
    Damage,
    ArmorDamage,
}

impl AmmoSort {
    fn label(self) -> &'static str {
        match self {
            AmmoSort::Penetration => "관통력순",
            AmmoSort::Damage => "데미지순",
            AmmoSort::ArmorDamage => "방어력 피해순",
        }
    }

    fn id(self) -> &'static str {
        match self {
            AmmoSort::Penetration => "pen",
            AmmoSort::Damage => "dmg",
            AmmoSort::ArmorDamage => "armor",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "pen" => Some(AmmoSort::Penetration),
            "dmg" => Some(AmmoSort::Damage),
            "armor" => Some(AmmoSort::ArmorDamage),
            _ => None,
        }
    }

    fn sort(self, ammo_list: &mut [Ammo]) {
        match self {
            AmmoSort::Penetration => {
                ammo_list.sort_by(|a, b| b.penetration_power.cmp(&a.penetration_power))
            }
            AmmoSort::Damage => ammo_list.sort_by(|a, b| b.damage.cmp(&a.damage)),
            AmmoSort::ArmorDamage => ammo_list.sort_by(|a, b| b.armor_damage.cmp(&a.armor_damage)),
        }
    }
}

async fn ammo_impl(ctx: Context<'_>, name: String) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let result: Result<AmmoData, _> = client::query(
        &data.http_client,
        &data.tarkov_cache,
        queries::AMMO_QUERY,
        &serde_json::json!({"lang": "ko"}),
    )
    .await;

    let all_ammo = match result {
        Ok(data) => data.ammo,
        Err(e) => {
            ctx.send(CreateReply::default().embed(utils::embed::error(&e.to_string())))
                .await?;
            return Ok(());
        }
    };

    // Filter by name (case-insensitive match on caliber or ammo name)
    let name_lower = name.to_lowercase();
    let mut filtered: Vec<Ammo> = all_ammo
        .into_iter()
        .filter(|a| {
            a.caliber.to_lowercase().contains(&name_lower)
                || a.item.name.to_lowercase().contains(&name_lower)
                || a.item.short_name.to_lowercase().contains(&name_lower)
        })
        .collect();

    if filtered.is_empty() {
        ctx.send(CreateReply::default().embed(utils::embed::error(&format!(
            "검색 결과가 없습니다: {name}"
        ))))
        .await?;
        return Ok(());
    }

    // Default sort: penetration
    let mut current_sort = AmmoSort::Penetration;
    current_sort.sort(&mut filtered);
    let mut page: usize = 0;

    let total_pages = filtered.len().div_ceil(10).max(1);
    let ammo_embed = embed::ammo_table(&filtered, current_sort.label(), page);
    let action_rows = build_ammo_components(current_sort, page, total_pages);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(ammo_embed)
                .components(action_rows),
        )
        .await?;

    let mut msg = reply.message().await?.into_owned();

    // Interaction loop
    while let Some(interaction) = components::await_component_interaction(ctx, &msg, 300).await {
        let custom_id = &interaction.data.custom_id;

        // Parse sort buttons: tarkov_ammo_sort_{sort_id}
        if let Some(sort_id) = custom_id.strip_prefix("tarkov_ammo_sort_") {
            if let Some(new_sort) = AmmoSort::from_id(sort_id) {
                current_sort = new_sort;
                current_sort.sort(&mut filtered);
                page = 0;
            }
        }
        // Parse pagination: tarkov_ammo_prev_{page} or tarkov_ammo_next_{page}
        else if let Some(page_str) = custom_id.strip_prefix("tarkov_ammo_prev_") {
            if let Ok(p) = page_str.parse::<usize>() {
                page = p;
            }
        } else if let Some(page_str) = custom_id.strip_prefix("tarkov_ammo_next_") {
            if let Ok(p) = page_str.parse::<usize>() {
                page = p;
            }
        }

        let total_pages = filtered.len().div_ceil(10).max(1);
        page = page.min(total_pages.saturating_sub(1));

        let new_embed = embed::ammo_table(&filtered, current_sort.label(), page);
        let new_components = build_ammo_components(current_sort, page, total_pages);

        interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(new_embed)
                        .components(new_components),
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

/// Build sort buttons and pagination rows for ammo display.
fn build_ammo_components(
    current_sort: AmmoSort,
    page: usize,
    total_pages: usize,
) -> Vec<serenity::CreateActionRow> {
    let sorts = [
        AmmoSort::Penetration,
        AmmoSort::Damage,
        AmmoSort::ArmorDamage,
    ];

    let sort_buttons: Vec<serenity::CreateButton> = sorts
        .iter()
        .map(|s| {
            serenity::CreateButton::new(format!("tarkov_ammo_sort_{}", s.id()))
                .label(s.label())
                .style(if s.id() == current_sort.id() {
                    serenity::ButtonStyle::Primary
                } else {
                    serenity::ButtonStyle::Secondary
                })
        })
        .collect();

    let sort_row = serenity::CreateActionRow::Buttons(sort_buttons);
    let page_row = components::pagination_row("ammo", page, total_pages);

    vec![sort_row, page_row]
}

/// 탄약 정보를 검색합니다
#[poise::command(slash_command, guild_only)]
pub async fn ammo(
    ctx: Context<'_>,
    #[description = "탄약 이름 또는 구경"] name: String,
) -> Result<(), Error> {
    ammo_impl(ctx, name).await
}

/// 탄약 정보를 검색합니다 (/ammo 한국어)
#[poise::command(slash_command, guild_only)]
pub async fn 탄약(
    ctx: Context<'_>,
    #[description = "탄약 이름 또는 구경"] name: String,
) -> Result<(), Error> {
    ammo_impl(ctx, name).await
}
