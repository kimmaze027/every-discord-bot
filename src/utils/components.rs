use serenity::builder::{CreateActionRow, CreateButton};
use serenity::model::application::ButtonStyle;

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
