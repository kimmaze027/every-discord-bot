pub struct Config {
    pub discord_token: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN 환경변수가 필요합니다"),
        }
    }
}
