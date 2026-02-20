use poise::CreateReply;

use crate::music::{player, queue};
use crate::utils::embed;
use crate::{Context, Error};

async fn skip_impl(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird 미등록");

    let call = match manager.get(guild_id) {
        Some(c) => c,
        None => {
            ctx.send(CreateReply::default().embed(embed::error("재생 중인 곡이 없습니다.")))
                .await?;
            return Ok(());
        }
    };

    let current = queue::get_current(&ctx.data().queue_manager, guild_id).await;

    match current {
        Some(song) => {
            // Play next with skip flag
            match player::play_next(
                guild_id,
                &ctx.data().queue_manager,
                &ctx.data().http_client,
                &call,
                true,
            )
            .await
            {
                Ok(()) => {
                    let next = queue::get_current(&ctx.data().queue_manager, guild_id).await;
                    let msg = match next {
                        Some(ref next_song) => {
                            format!("⏭️ **{}** 스킵 → **{}**", song.title, next_song.title)
                        }
                        None => format!("⏭️ **{}** 스킵 (큐 비어있음)", song.title),
                    };
                    ctx.say(msg).await?;
                }
                Err(e) => {
                    ctx.send(
                        CreateReply::default().embed(embed::error(&format!("스킵 실패: {e}"))),
                    )
                    .await?;
                }
            }
        }
        None => {
            ctx.send(CreateReply::default().embed(embed::error("재생 중인 곡이 없습니다.")))
                .await?;
        }
    }

    Ok(())
}

/// 현재 곡을 건너뜁니다
#[poise::command(slash_command)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    skip_impl(ctx).await
}

/// 현재 곡을 건너뜁니다 (/skip 단축)
#[poise::command(slash_command)]
pub async fn s(ctx: Context<'_>) -> Result<(), Error> {
    skip_impl(ctx).await
}
