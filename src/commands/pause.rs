use poise::CreateReply;

use crate::music::queue;
use crate::utils::embed;
use crate::{Context, Error};

async fn pause_impl(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let queues = ctx.data().queue_manager.read().await;
    let handle = queues.get(&guild_id).and_then(|q| q.track_handle.as_ref());

    match handle {
        Some(h) => {
            let _ = h.pause();
            drop(queues);
            let current = queue::get_current(&ctx.data().queue_manager, guild_id).await;
            let title = current.map_or("알 수 없음".to_string(), |s| s.title);
            ctx.say(format!("⏸️ **{title}** 일시정지")).await?;
        }
        None => {
            drop(queues);
            ctx.send(CreateReply::default().embed(embed::error("재생 중인 곡이 없습니다.")))
                .await?;
        }
    }

    Ok(())
}

/// 현재 곡을 일시정지합니다
#[poise::command(slash_command, guild_only)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    pause_impl(ctx).await
}

/// 현재 곡을 일시정지합니다 (/pause 단축)
#[poise::command(slash_command, guild_only)]
pub async fn pa(ctx: Context<'_>) -> Result<(), Error> {
    pause_impl(ctx).await
}
