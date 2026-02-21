use std::time::Duration;

use poise::serenity_prelude as serenity;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::collector::ComponentInteractionCollector;
use serenity::model::application::ButtonStyle;

use crate::music::Song;

// ── Music components (PR #21) ───────────────────────────────────────────────

pub fn music_buttons(is_paused: bool) -> CreateActionRow {
    let pause_resume = if is_paused {
        CreateButton::new("music_resume")
            .label("재개")
            .emoji('▶')
            .style(ButtonStyle::Success)
    } else {
        CreateButton::new("music_pause")
            .label("일시정지")
            .emoji('⏸')
            .style(ButtonStyle::Primary)
    };

    let skip = CreateButton::new("music_skip")
        .label("스킵")
        .emoji('⏭')
        .style(ButtonStyle::Secondary);

    let stop = CreateButton::new("music_stop")
        .label("정지")
        .emoji('⏹')
        .style(ButtonStyle::Danger);

    CreateActionRow::Buttons(vec![pause_resume, skip, stop])
}

pub fn music_buttons_disabled() -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new("music_pause")
            .label("일시정지")
            .emoji('⏸')
            .style(ButtonStyle::Primary)
            .disabled(true),
        CreateButton::new("music_skip")
            .label("스킵")
            .emoji('⏭')
            .style(ButtonStyle::Secondary)
            .disabled(true),
        CreateButton::new("music_stop")
            .label("정지")
            .emoji('⏹')
            .style(ButtonStyle::Danger)
            .disabled(true),
    ])
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{truncated}...")
    }
}

fn queue_select_menu(upcoming: &[Song]) -> CreateActionRow {
    let count = upcoming.len().min(25);
    let options: Vec<CreateSelectMenuOption> = upcoming
        .iter()
        .take(25)
        .enumerate()
        .map(|(i, song)| {
            let label = truncate_str(&song.title, 100);
            let desc = match &song.duration {
                Some(d) => format!("{}번째 · {d}", i + 1),
                None => format!("{}번째", i + 1),
            };
            CreateSelectMenuOption::new(label, format!("queue_{i}"))
                .description(truncate_str(&desc, 100))
        })
        .collect();

    let placeholder = if upcoming.len() > 25 {
        format!("대기열 ({count}/{}곡)", upcoming.len())
    } else {
        format!("대기열 ({count}곡)")
    };

    let menu = CreateSelectMenu::new(
        "music_queue_select",
        CreateSelectMenuKind::String { options },
    )
    .placeholder(placeholder);

    CreateActionRow::SelectMenu(menu)
}

pub fn music_components(is_paused: bool, upcoming: &[Song]) -> Vec<CreateActionRow> {
    let mut rows = vec![music_buttons(is_paused)];
    if !upcoming.is_empty() {
        rows.push(queue_select_menu(upcoming));
    }
    rows
}

pub fn music_components_disabled() -> Vec<CreateActionRow> {
    vec![music_buttons_disabled()]
}

// ── Tarkov components (PR #22) ──────────────────────────────────────────────

/// Build a pagination action row with Previous / Page indicator / Next buttons.
///
/// Custom ID patterns:
/// - Previous: `tarkov_{cmd}_prev_{current_page - 1}`
/// - Next: `tarkov_{cmd}_next_{current_page + 1}`
///
/// The page indicator button is always disabled (non-interactive label).
/// Previous is disabled on page 0, Next is disabled on the last page.
pub fn pagination_row(cmd: &str, current_page: usize, total_pages: usize) -> CreateActionRow {
    let prev_disabled = current_page == 0;
    let next_disabled = total_pages == 0 || current_page >= total_pages - 1;

    let prev_page = current_page.saturating_sub(1);
    let next_page = current_page + 1;

    let prev_button = CreateButton::new(format!("tarkov_{cmd}_prev_{prev_page}"))
        .label("<< 이전")
        .style(ButtonStyle::Secondary)
        .disabled(prev_disabled);

    let page_indicator = CreateButton::new(format!("tarkov_{cmd}_page_{current_page}"))
        .label(format!("{}/{}", current_page + 1, total_pages.max(1)))
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let next_button = CreateButton::new(format!("tarkov_{cmd}_next_{next_page}"))
        .label("다음 >>")
        .style(ButtonStyle::Secondary)
        .disabled(next_disabled);

    CreateActionRow::Buttons(vec![prev_button, page_indicator, next_button])
}

/// Build a select menu action row from a list of (value, label, description) tuples.
///
/// Discord limits select menus to 25 options maximum.
/// If more than 25 options are provided, only the first 25 are used.
pub fn item_select_menu(
    custom_id: &str,
    placeholder: &str,
    options: Vec<(String, String, String)>,
) -> CreateActionRow {
    let menu_options: Vec<CreateSelectMenuOption> = options
        .into_iter()
        .take(25)
        .map(|(value, label, description)| {
            let mut opt = CreateSelectMenuOption::new(&label, &value);
            if !description.is_empty() {
                opt = opt.description(&description);
            }
            opt
        })
        .collect();

    let select = CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::String {
            options: menu_options,
        },
    )
    .placeholder(placeholder);

    CreateActionRow::SelectMenu(select)
}

/// Build tab switching buttons.
///
/// Each tab is `(id_suffix, label, is_active)`.
/// Active tabs use `ButtonStyle::Primary`, inactive use `ButtonStyle::Secondary`.
///
/// Custom ID pattern: `tarkov_{cmd}_{id_suffix}_{item_id}`
pub fn tab_buttons(cmd: &str, item_id: &str, tabs: &[(&str, &str, bool)]) -> CreateActionRow {
    let buttons: Vec<CreateButton> = tabs
        .iter()
        .map(|(id_suffix, label, is_active)| {
            CreateButton::new(format!("tarkov_{cmd}_{id_suffix}_{item_id}"))
                .label(*label)
                .style(if *is_active {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                })
        })
        .collect();

    CreateActionRow::Buttons(buttons)
}

/// Wait for a single component interaction on the given message.
///
/// Uses `ComponentInteractionCollector` from serenity with a timeout.
/// Returns `None` if the timeout expires before any interaction.
pub async fn await_component_interaction(
    ctx: crate::Context<'_>,
    msg: &serenity::Message,
    timeout_secs: u64,
) -> Option<serenity::ComponentInteraction> {
    ComponentInteractionCollector::new(ctx.serenity_context())
        .message_id(msg.id)
        .timeout(Duration::from_secs(timeout_secs))
        .next()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_row_first_page() {
        let row = pagination_row("ammo", 0, 5);
        match &row {
            CreateActionRow::Buttons(buttons) => {
                assert_eq!(buttons.len(), 3);
            }
            _ => panic!("Expected Buttons action row"),
        }
    }

    #[test]
    fn test_pagination_row_last_page() {
        let row = pagination_row("ammo", 4, 5);
        match &row {
            CreateActionRow::Buttons(buttons) => {
                assert_eq!(buttons.len(), 3);
            }
            _ => panic!("Expected Buttons action row"),
        }
    }

    #[test]
    fn test_pagination_row_single_page() {
        let row = pagination_row("quest", 0, 1);
        match &row {
            CreateActionRow::Buttons(buttons) => {
                assert_eq!(buttons.len(), 3);
            }
            _ => panic!("Expected Buttons action row"),
        }
    }

    #[test]
    fn test_pagination_row_zero_pages() {
        let row = pagination_row("quest", 0, 0);
        match &row {
            CreateActionRow::Buttons(buttons) => {
                assert_eq!(buttons.len(), 3);
            }
            _ => panic!("Expected Buttons action row"),
        }
    }

    #[test]
    fn test_item_select_menu_basic() {
        let options = vec![
            ("id1".into(), "Item 1".into(), "Desc 1".into()),
            ("id2".into(), "Item 2".into(), "Desc 2".into()),
        ];
        let row = item_select_menu("tarkov_item_select", "아이템을 선택하세요", options);
        match &row {
            CreateActionRow::SelectMenu(_) => {}
            _ => panic!("Expected SelectMenu action row"),
        }
    }

    #[test]
    fn test_item_select_menu_respects_25_limit() {
        let options: Vec<(String, String, String)> = (0..30)
            .map(|i| (format!("id{i}"), format!("Item {i}"), format!("Desc {i}")))
            .collect();
        assert_eq!(options.len(), 30);

        let row = item_select_menu("tarkov_item_select", "선택하세요", options);
        match &row {
            CreateActionRow::SelectMenu(_) => {}
            _ => panic!("Expected SelectMenu action row"),
        }
    }

    #[test]
    fn test_item_select_menu_empty_description() {
        let options = vec![("id1".into(), "Item 1".into(), String::new())];
        let row = item_select_menu("test_select", "선택하세요", options);
        match &row {
            CreateActionRow::SelectMenu(_) => {}
            _ => panic!("Expected SelectMenu action row"),
        }
    }

    #[test]
    fn test_tab_buttons_active_highlight() {
        let tabs = vec![
            ("info", "기본정보", true),
            ("price", "가격", false),
            ("trader", "트레이더", false),
        ];
        let row = tab_buttons("item", "abc123", &tabs);
        match &row {
            CreateActionRow::Buttons(buttons) => {
                assert_eq!(buttons.len(), 3);
            }
            _ => panic!("Expected Buttons action row"),
        }
    }

    #[test]
    fn test_tab_buttons_multiple_inactive() {
        let tabs = vec![
            ("info", "기본정보", false),
            ("equip", "장비", false),
            ("drops", "드롭 아이템", true),
            ("spawns", "스폰 위치", false),
        ];
        let row = tab_buttons("boss", "boss1", &tabs);
        match &row {
            CreateActionRow::Buttons(buttons) => {
                assert_eq!(buttons.len(), 4);
            }
            _ => panic!("Expected Buttons action row"),
        }
    }
}
