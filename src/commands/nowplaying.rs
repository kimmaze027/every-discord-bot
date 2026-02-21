use poise::CreateReply;

use crate::music::queue;
use crate::utils::{components, embed};
use crate::{Context, Error};

async fn nowplaying_impl(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let current = queue::get_current(&ctx.data().queue_manager, guild_id).await;

    match current {
        Some(song) => {
            let loop_mode = queue::get_loop_mode(&ctx.data().queue_manager, guild_id).await;
            let vol = queue::get_volume(&ctx.data().queue_manager, guild_id).await;

            let mut e = embed::now_playing(&song);
            e = e.field("반복", format!("{loop_mode}"), true);
            e = e.field("볼륨", format!("{}%", (vol * 100.0) as u32), true);

            let is_paused = {
                let handle = {
                    let queues = ctx.data().queue_manager.read().await;
                    queues.get(&guild_id).and_then(|q| q.track_handle.clone())
                };
                match handle {
                    Some(h) => h
                        .get_info()
                        .await
                        .map(|info| info.playing == songbird::tracks::PlayMode::Pause)
                        .unwrap_or(false),
                    None => false,
                }
            };

            let (_, upcoming) = queue::get_queue_list(&ctx.data().queue_manager, guild_id).await;

            ctx.send(
                CreateReply::default()
                    .embed(e)
                    .components(components::music_components(is_paused, &upcoming)),
            )
            .await?;
        }
        None => {
            ctx.send(CreateReply::default().embed(embed::error("재생 중인 곡이 없습니다.")))
                .await?;
        }
    }

    Ok(())
}

/// 현재 재생 중인 곡 정보를 표시합니다
#[poise::command(slash_command, guild_only)]
pub async fn nowplaying(ctx: Context<'_>) -> Result<(), Error> {
    nowplaying_impl(ctx).await
}

/// 현재 재생 중인 곡 정보를 표시합니다 (/nowplaying 단축)
#[poise::command(slash_command, guild_only)]
pub async fn np(ctx: Context<'_>) -> Result<(), Error> {
    nowplaying_impl(ctx).await
}
