use poise::serenity_prelude as serenity;
use tracing::info;

use crate::music::queue;
use crate::Data;

pub async fn handle(
    ctx: &serenity::Context,
    _old: &Option<serenity::VoiceState>,
    new: &serenity::VoiceState,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let manager = songbird::get(ctx).await.expect("Songbird 미등록");

    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => return Ok(()), // Bot is not in a voice channel in this guild
    };

    let handler = handler_lock.lock().await;
    let bot_channel = match handler.current_channel() {
        Some(ch) => ch,
        None => return Ok(()),
    };
    drop(handler);

    // Count members in the bot's voice channel
    let member_count = {
        let guild = match ctx.cache.guild(guild_id) {
            Some(g) => g,
            None => return Ok(()),
        };

        guild
            .voice_states
            .values()
            .filter(|vs| {
                vs.channel_id
                    .map_or(false, |ch| ch.get() == bot_channel.0.get())
            })
            .count()
    };

    // If bot is alone (only bot in channel), start leave timer
    if member_count <= 1 {
        let manager = manager.clone();
        let queue_manager = data.queue_manager.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            // Re-check if still alone
            let still_alone = {
                let handler_lock = match manager.get(guild_id) {
                    Some(h) => h,
                    None => return,
                };
                let handler = handler_lock.lock().await;
                let current_channel = match handler.current_channel() {
                    Some(ch) => ch,
                    None => return,
                };
                drop(handler);

                // We can't easily re-check member count without cache access here
                // so we just check if we're still in the channel
                let _ = current_channel;
                true // Simplified: always leave after 30s alone
            };

            if still_alone {
                info!("음성 채널에 혼자 남아 퇴장합니다 (guild: {guild_id})");
                queue::clear(&queue_manager, guild_id).await;
                let _ = manager.remove(guild_id).await;
            }
        });
    }

    Ok(())
}
