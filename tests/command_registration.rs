use std::collections::HashSet;

use every_discord_bot::commands;

#[test]
fn test_all_commands_returns_correct_count() {
    let cmds = commands::all();
    assert_eq!(
        cmds.len(),
        43,
        "Expected 43 commands (1 help + 11 music + 11 aliases + 10 tarkov + 10 aliases), got {}",
        cmds.len()
    );
}

#[test]
fn test_all_commands_contain_expected_names() {
    let cmds = commands::all();
    let names: HashSet<&str> = cmds.iter().map(|cmd| cmd.name.as_str()).collect();

    let expected = [
        "help",
        "play",
        "p",
        "skip",
        "s",
        "stop",
        "st",
        "queue",
        "q",
        "pause",
        "pa",
        "resume",
        "r",
        "nowplaying",
        "np",
        "loop",
        "l",
        "shuffle",
        "sh",
        "remove",
        "rm",
        "volume",
        "v",
        // Tarkov commands (Phase 3)
        "item",
        "아이템",
        "price",
        "가격",
        "ammo",
        "탄약",
        // Tarkov commands (Phase 4)
        "trader",
        "상인",
        "quest",
        "퀘스트",
        "questitem",
        "퀘스트아이템",
        // Tarkov commands (Phase 5)
        "hideout",
        "은신처",
        "craft",
        "제작",
        // Tarkov commands (Phase 6)
        "map",
        "맵",
        "boss",
        "보스",
    ];

    for name in &expected {
        assert!(
            names.contains(name),
            "Expected command '{}' not found in commands::all(). Present names: {:?}",
            name,
            names
        );
    }
}

#[test]
fn test_no_duplicate_command_names() {
    let cmds = commands::all();
    let mut seen = HashSet::new();

    for cmd in &cmds {
        assert!(
            seen.insert(cmd.name.as_str()),
            "Duplicate command name found: '{}'",
            cmd.name
        );
    }
}

#[test]
fn test_all_commands_are_slash_commands() {
    let cmds = commands::all();

    for cmd in &cmds {
        assert!(
            cmd.slash_action.is_some(),
            "Command '{}' does not have slash_action set (not a slash command)",
            cmd.name
        );
    }
}
