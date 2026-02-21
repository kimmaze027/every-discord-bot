pub mod ai;
pub mod commands;
pub mod config;
pub mod events;
pub mod music;
pub mod tarkov;
pub mod utils;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    pub queue_manager: music::QueueManager,
    pub http_client: reqwest::Client,
    pub tarkov_cache: tarkov::Cache,
    pub gemini_api_key: Option<String>,
    pub tv_channel_id: Option<u64>,
    pub chat_db: Option<ai::db::ChatDb>,
    pub pending_queries: ai::PendingQueries,
}
