use poise::CreateReply;

use crate::music::queue;
use crate::utils::embed;
use crate::{Context, Error};

async fn volume_impl(ctx: Context<'_>, level: u32) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    if level > 100 {
        ctx.send(CreateReply::default().embed(embed::error("볼륨은 0~100 사이로 설정해주세요.")))
            .await?;
        return Ok(());
    }

    let volume = level as f32 / 100.0;
    queue::set_volume(&ctx.data().queue_manager, guild_id, volume).await;

    ctx.say(format!("🔊 볼륨: **{level}%**")).await?;

    Ok(())
}

/// 볼륨을 조절합니다
#[poise::command(slash_command, guild_only)]
pub async fn volume(
    ctx: Context<'_>,
    #[description = "볼륨 (0-100)"] level: u32,
) -> Result<(), Error> {
    volume_impl(ctx, level).await
}

/// 볼륨을 조절합니다 (/volume 단축)
#[poise::command(slash_command, guild_only)]
pub async fn v(
    ctx: Context<'_>, #[description = "볼륨 (0-100)"] level: u32
) -> Result<(), Error> {
    volume_impl(ctx, level).await
}
