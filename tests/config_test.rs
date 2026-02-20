use std::env;

#[test]
#[should_panic(expected = "DISCORD_TOKEN")]
fn test_config_missing_token_panics() {
    // Remove the env var if set
    env::remove_var("DISCORD_TOKEN");
    every_discord_bot::config::Config::from_env();
}
