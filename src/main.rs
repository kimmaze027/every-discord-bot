use every_discord_bot::{ai, commands, config, events, music, tarkov, Data};
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let config = config::Config::from_env();

    // AI 채팅용 DB 초기화
    let chat_db = if config.gemini_api_key.is_some() && config.tv_channel_id.is_some() {
        match ai::db::ChatDb::new(&config.db_path) {
            Ok(db) => {
                tracing::info!("AI 채팅 DB 초기화 완료: {}", config.db_path);
                Some(db)
            }
            Err(e) => {
                tracing::error!("AI 채팅 DB 초기화 실패: {e}");
                None
            }
        }
    } else {
        None
    };

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let gemini_api_key = config.gemini_api_key.clone();
    let tv_channel_id = config.tv_channel_id;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(events::handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tracing::info!("봇이 준비되었습니다!");
                Ok(Data {
                    queue_manager: music::new_queue_manager(),
                    http_client: reqwest::Client::new(),
                    tarkov_cache: tarkov::new_cache(),
                    gemini_api_key,
                    tv_channel_id,
                    chat_db,
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
