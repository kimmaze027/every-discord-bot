use poise::CreateReply;

use crate::music::queue;
use crate::music::LoopMode;
use crate::utils::embed;
use crate::{Context, Error};

async fn loop_impl(ctx: Context<'_>, mode: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let loop_mode = match mode.to_lowercase().as_str() {
        "off" | "끔" => LoopMode::Off,
        "song" | "곡" | "한곡" => LoopMode::Song,
        "queue" | "큐" | "전체" => LoopMode::Queue,
        _ => {
            ctx.send(CreateReply::default().embed(embed::error(
                "올바른 모드를 선택해주세요: `off`, `song`, `queue`",
            )))
            .await?;
            return Ok(());
        }
    };

    let mode = queue::set_loop_mode(&ctx.data().queue_manager, guild_id, loop_mode).await;

    let emoji = match mode {
        LoopMode::Off => "➡️",
        LoopMode::Song => "🔂",
        LoopMode::Queue => "🔁",
    };

    ctx.say(format!("{emoji} 반복 모드: **{mode}**")).await?;

    Ok(())
}

/// 반복 모드를 설정합니다
#[poise::command(slash_command, rename = "loop")]
pub async fn loop_cmd(
    ctx: Context<'_>,
    #[description = "반복 모드 (off/song/queue)"] mode: String,
) -> Result<(), Error> {
    loop_impl(ctx, mode).await
}

/// 반복 모드를 설정합니다 (/loop 단축)
#[poise::command(slash_command)]
pub async fn l(
    ctx: Context<'_>,
    #[description = "반복 모드 (off/song/queue)"] mode: String,
) -> Result<(), Error> {
    loop_impl(ctx, mode).await
}
