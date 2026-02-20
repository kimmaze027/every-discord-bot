use poise::serenity_prelude as serenity;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::id::GuildId;

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
    components: Vec<CreateActionRow>,
) -> Result<(), Error> {
    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(components),
    );
    interaction.create_response(&ctx.http, response).await?;
    Ok(())
}

async fn is_track_paused(data: &Data, guild_id: GuildId) -> bool {
    let handle = {
        let queues = data.queue_manager.read().await;
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
            if let Some(h) = queues.get(&guild_id).and_then(|q| q.track_handle.as_ref()) {
                let _ = h.pause();
            }
            drop(queues);

            let current = queue::get_current(&data.queue_manager, guild_id).await;
            let (_, upcoming) = queue::get_queue_list(&data.queue_manager, guild_id).await;
            let e = match current {
                Some(song) => embed::now_playing(&song).title("⏸️ 일시정지"),
                None => embed::error("재생 중인 곡이 없습니다."),
            };
            update_message(
                ctx,
                interaction,
                e,
                components::music_components(true, &upcoming),
            )
            .await?;
        }
        "music_resume" => {
            let queues = data.queue_manager.read().await;
            if let Some(h) = queues.get(&guild_id).and_then(|q| q.track_handle.as_ref()) {
                let _ = h.play();
            }
            drop(queues);

            let current = queue::get_current(&data.queue_manager, guild_id).await;
            let (_, upcoming) = queue::get_queue_list(&data.queue_manager, guild_id).await;
            let e = match current {
                Some(song) => embed::now_playing(&song),
                None => embed::error("재생 중인 곡이 없습니다."),
            };
            update_message(
                ctx,
                interaction,
                e,
                components::music_components(false, &upcoming),
            )
            .await?;
        }
        "music_skip" => {
            let call = match manager.get(guild_id) {
                Some(c) => c,
                None => {
                    let e = embed::error("재생 중인 곡이 없습니다.");
                    update_message(ctx, interaction, e, components::music_components_disabled())
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
                    let (_, upcoming) = queue::get_queue_list(&data.queue_manager, guild_id).await;
                    let (e, comps) = match next {
                        Some(song) => (
                            embed::now_playing(&song),
                            components::music_components(false, &upcoming),
                        ),
                        None => (
                            CreateEmbed::new()
                                .title("⏭️ 스킵 완료")
                                .description("큐가 비어있습니다.")
                                .color(0x5865F2),
                            components::music_components_disabled(),
                        ),
                    };
                    update_message(ctx, interaction, e, comps).await?;
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
            update_message(ctx, interaction, e, components::music_components_disabled()).await?;
        }
        "music_queue_select" => {
            // Informational dropdown - refresh message with current state
            let is_paused = is_track_paused(data, guild_id).await;
            let current = queue::get_current(&data.queue_manager, guild_id).await;
            let (_, upcoming) = queue::get_queue_list(&data.queue_manager, guild_id).await;
            let e = match current {
                Some(song) => {
                    let mut e = embed::now_playing(&song);
                    if is_paused {
                        e = e.title("⏸️ 일시정지");
                    }
                    e
                }
                None => embed::error("재생 중인 곡이 없습니다."),
            };
            update_message(
                ctx,
                interaction,
                e,
                components::music_components(is_paused, &upcoming),
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}
