pub struct Config {
    pub discord_token: String,
    pub gemini_api_key: Option<String>,
    pub tv_channel_id: Option<u64>,
    pub db_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN 환경변수가 필요합니다"),
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            tv_channel_id: std::env::var("EVERYBOT_TV_CHANNEL_ID")
                .ok()
                .and_then(|v| v.parse().ok()),
            db_path: std::env::var("EVERYBOT_DB_PATH")
                .unwrap_or_else(|_| "everybot.db".to_string()),
        }
    }
}
