use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::ComponentInteraction;

use crate::music::{player, queue};
use crate::utils::{components, embed};
use crate::{Data, Error};

async fn respond_ephemeral(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    message: &str,
) -> Result<(), Error> {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed::error(message))
            .ephemeral(true),
    );
    interaction.create_response(&ctx.http, response).await?;
    Ok(())
}

async fn update_message(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    embed: CreateEmbed,
    buttons: serenity::builder::CreateActionRow,
) -> Result<(), Error> {
    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![buttons]),
    );
    interaction.create_response(&ctx.http, response).await?;
    Ok(())
}

pub async fn handle(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Result<(), Error> {
    let guild_id = interaction
        .guild_id
        .ok_or("서버에서만 사용할 수 있습니다")?;

    let manager = songbird::get(ctx).await.expect("Songbird 미등록");

    // Check bot is in a voice channel
    let bot_channel = {
        let handler_lock = match manager.get(guild_id) {
            Some(h) => h,
            None => {
                respond_ephemeral(ctx, interaction, "봇이 음성 채널에 없습니다.").await?;
                return Ok(());
            }
        };
        let handler = handler_lock.lock().await;
        handler.current_channel()
    };

    // Check user is in the same voice channel
    let user_in_bot_channel = {
        let guild = ctx
            .cache
            .guild(guild_id)
            .ok_or("서버 정보를 가져올 수 없습니다")?;
        match bot_channel {
            Some(bot_ch) => guild
                .voice_states
                .get(&interaction.user.id)
                .and_then(|vs| vs.channel_id)
                .is_some_and(|ch| ch.get() == bot_ch.0.get()),
            None => false,
        }
    };

    if !user_in_bot_channel {
        respond_ephemeral(ctx, interaction, "봇과 같은 음성 채널에 있어야 합니다.").await?;
        return Ok(());
    }

    match interaction.data.custom_id.as_str() {
        "music_pause" => {
            let queues = data.queue_manager.read().await;
            if let Some(h) = queues
                .get(&guild_id)
                .and_then(|q| q.track_handle.as_ref())
            {
                let _ = h.pause();
            }
            drop(queues);

            let e = match queue::get_current(&data.queue_manager, guild_id).await {
                Some(song) => embed::now_playing(&song).title("⏸️ 일시정지"),
                None => embed::error("재생 중인 곡이 없습니다."),
            };
            update_message(ctx, interaction, e, components::music_buttons(true)).await?;
        }
        "music_resume" => {
            let queues = data.queue_manager.read().await;
            if let Some(h) = queues
                .get(&guild_id)
                .and_then(|q| q.track_handle.as_ref())
            {
                let _ = h.play();
            }
            drop(queues);

            let e = match queue::get_current(&data.queue_manager, guild_id).await {
                Some(song) => embed::now_playing(&song),
                None => embed::error("재생 중인 곡이 없습니다."),
            };
            update_message(ctx, interaction, e, components::music_buttons(false)).await?;
        }
        "music_skip" => {
            let call = match manager.get(guild_id) {
                Some(c) => c,
                None => {
                    let e = embed::error("재생 중인 곡이 없습니다.");
                    update_message(ctx, interaction, e, components::music_buttons_disabled())
                        .await?;
                    return Ok(());
                }
            };

            match player::play_next(
                guild_id,
                &data.queue_manager,
                &data.http_client,
                &call,
                true,
            )
            .await
            {
                Ok(()) => {
                    let next = queue::get_current(&data.queue_manager, guild_id).await;
                    let (e, buttons) = match next {
                        Some(song) => {
                            (embed::now_playing(&song), components::music_buttons(false))
                        }
                        None => (
                            CreateEmbed::new()
                                .title("⏭️ 스킵 완료")
                                .description("큐가 비어있습니다.")
                                .color(0x5865F2),
                            components::music_buttons_disabled(),
                        ),
                    };
                    update_message(ctx, interaction, e, buttons).await?;
                }
                Err(e) => {
                    respond_ephemeral(ctx, interaction, &format!("스킵 실패: {e}")).await?;
                }
            }
        }
        "music_stop" => {
            queue::clear(&data.queue_manager, guild_id).await;
            let _ = manager.remove(guild_id).await;

            let e = CreateEmbed::new()
                .title("⏹️ 재생 중지")
                .description("재생을 중지하고 퇴장합니다.")
                .color(0xED4245);
            update_message(ctx, interaction, e, components::music_buttons_disabled()).await?;
        }
        _ => {}
    }

    Ok(())
}
