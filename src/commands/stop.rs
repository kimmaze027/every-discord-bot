use poise::CreateReply;

use crate::music::queue;
use crate::utils::embed;
use crate::{Context, Error};

async fn stop_impl(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird 미등록");

    if manager.get(guild_id).is_none() {
        ctx.send(CreateReply::default().embed(embed::error("재생 중인 곡이 없습니다.")))
            .await?;
        return Ok(());
    }

    queue::clear(&ctx.data().queue_manager, guild_id).await;
    let _ = manager.remove(guild_id).await;

    ctx.say("⏹️ 재생을 중지하고 퇴장합니다.").await?;

    Ok(())
}

/// 재생을 중지하고 퇴장합니다
#[poise::command(slash_command)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    stop_impl(ctx).await
}

/// 재생을 중지하고 퇴장합니다 (/stop 단축)
#[poise::command(slash_command)]
pub async fn st(ctx: Context<'_>) -> Result<(), Error> {
    stop_impl(ctx).await
}
