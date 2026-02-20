pub mod commands;
pub mod config;
pub mod events;
pub mod music;
pub mod utils;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    pub queue_manager: music::QueueManager,
    pub http_client: reqwest::Client,
}
