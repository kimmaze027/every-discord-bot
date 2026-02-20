use std::env;

#[test]
#[should_panic(expected = "DISCORD_TOKEN")]
fn test_config_missing_token_panics() {
    // Remove the env var if set
    env::remove_var("DISCORD_TOKEN");
    discord_music_bot::config::Config::from_env();
}
