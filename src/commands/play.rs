use poise::CreateReply;

use crate::music::{player, queue, source};
use crate::utils::{components, embed};
use crate::{Context, Error};

async fn play_impl(ctx: Context<'_>, query: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let channel_id = {
        let guild = ctx.guild().ok_or("서버 정보를 가져올 수 없습니다")?;
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|vs| vs.channel_id)
    };

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            ctx.send(CreateReply::default().embed(embed::error("음성 채널에 먼저 접속해주세요!")))
                .await?;
            return Ok(());
        }
    };

    ctx.defer().await?;

    let mut song = match source::get_song_info(&query).await {
        Ok(s) => s,
        Err(e) => {
            ctx.send(
                CreateReply::default()
                    .embed(embed::error(&format!("노래를 찾을 수 없습니다: {e}"))),
            )
            .await?;
            return Ok(());
        }
    };

    song.requester = ctx.author().name.clone();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird 미등록");

    let call = manager.join(guild_id, channel_id).await?;

    let is_first = queue::is_empty(&ctx.data().queue_manager, guild_id).await;
    let position = queue::add_song(&ctx.data().queue_manager, guild_id, song.clone()).await;

    if is_first {
        let next = queue::get_next_song(&ctx.data().queue_manager, guild_id, false).await;
        if let Some(song) = next {
            player::play_song(
                guild_id,
                &ctx.data().queue_manager,
                &ctx.data().http_client,
                &call,
                &song,
            )
            .await?;

            let (_, upcoming) = queue::get_queue_list(&ctx.data().queue_manager, guild_id).await;
            ctx.send(
                CreateReply::default()
                    .embed(embed::now_playing(&song))
                    .components(components::music_components(false, &upcoming)),
            )
            .await?;
        }
    } else {
        let (_, upcoming) = queue::get_queue_list(&ctx.data().queue_manager, guild_id).await;
        ctx.send(
            CreateReply::default()
                .embed(embed::added_to_queue(&song, position))
                .components(components::music_components(false, &upcoming)),
        )
        .await?;
    }

    Ok(())
}

/// 음악을 재생합니다
#[poise::command(slash_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "노래 제목 또는 URL"] query: String,
) -> Result<(), Error> {
    play_impl(ctx, query).await
}

/// 음악을 재생합니다 (/play 단축)
#[poise::command(slash_command, guild_only)]
pub async fn p(
    ctx: Context<'_>,
    #[description = "노래 제목 또는 URL"] query: String,
) -> Result<(), Error> {
    play_impl(ctx, query).await
}
