use poise::CreateReply;

use crate::music::queue;
use crate::utils::embed;
use crate::{Context, Error};

async fn shuffle_impl(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let count = queue::shuffle(&ctx.data().queue_manager, guild_id).await;

    if count == 0 {
        ctx.send(CreateReply::default().embed(embed::error("큐가 비어있습니다.")))
            .await?;
    } else {
        ctx.say(format!("🔀 {count}곡을 셔플했습니다.")).await?;
    }

    Ok(())
}

/// 큐를 셔플합니다
#[poise::command(slash_command, guild_only)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    shuffle_impl(ctx).await
}

/// 큐를 셔플합니다 (/shuffle 단축)
#[poise::command(slash_command, guild_only)]
pub async fn sh(ctx: Context<'_>) -> Result<(), Error> {
    shuffle_impl(ctx).await
}
