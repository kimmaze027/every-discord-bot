use every_discord_bot::{commands, config, events, music, Data};
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let config = config::Config::from_env();

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(events::handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tracing::info!("봇이 준비되었습니다!");
                Ok(Data {
                    queue_manager: music::new_queue_manager(),
                    http_client: reqwest::Client::new(),
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(&config.discord_token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("클라이언트 생성 실패");

    if let Err(e) = client.start().await {
        tracing::error!("클라이언트 오류: {e}");
    }
}
