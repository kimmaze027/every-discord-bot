use serenity::builder::{
    CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
use serenity::model::application::ButtonStyle;

use crate::music::Song;

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
