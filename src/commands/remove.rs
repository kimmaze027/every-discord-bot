use poise::CreateReply;

use crate::music::queue;
use crate::utils::embed;
use crate::{Context, Error};

async fn remove_impl(ctx: Context<'_>, position: usize) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let removed = queue::remove_at(&ctx.data().queue_manager, guild_id, position).await;

    match removed {
        Some(song) => {
            ctx.say(format!("🗑️ **{}** 제거됨 (#{position})", song.title))
                .await?;
        }
        None => {
            ctx.send(
                CreateReply::default()
                    .embed(embed::error(&format!("#{position} 위치에 곡이 없습니다."))),
            )
            .await?;
        }
    }

    Ok(())
}

/// 큐에서 곡을 제거합니다
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "제거할 곡 번호"] position: usize,
) -> Result<(), Error> {
    remove_impl(ctx, position).await
}

/// 큐에서 곡을 제거합니다 (/remove 단축)
#[poise::command(slash_command)]
pub async fn rm(
    ctx: Context<'_>,
    #[description = "제거할 곡 번호"] position: usize,
) -> Result<(), Error> {
    remove_impl(ctx, position).await
}
