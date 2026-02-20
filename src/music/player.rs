use std::sync::Arc;

use async_trait::async_trait;
use serenity::model::id::GuildId;
use songbird::events::{Event, EventContext, EventHandler, TrackEvent};
use songbird::input::YoutubeDl;
use songbird::Call;
use tokio::sync::Mutex;
use tracing::{error, info};

use super::queue;
use super::QueueManager;
use super::Song;

struct TrackEndNotifier {
    guild_id: GuildId,
    queue_manager: QueueManager,
    http_client: reqwest::Client,
    call: Arc<Mutex<Call>>,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let guild_id = self.guild_id;
        let queue_manager = self.queue_manager.clone();
        let http_client = self.http_client.clone();
        let call = self.call.clone();

        tokio::spawn(async move {
            if let Err(e) = play_next(guild_id, &queue_manager, &http_client, &call, false).await {
                error!("다음 곡 재생 실패: {e}");
            }
        });

        None
    }
}

pub async fn play_song(
    guild_id: GuildId,
    queue_manager: &QueueManager,
    http_client: &reqwest::Client,
    call: &Arc<Mutex<Call>>,
    song: &Song,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let src = YoutubeDl::new(http_client.clone(), song.url.clone());
    let volume = queue::get_volume(queue_manager, guild_id).await;

    let track_handle = {
        let mut handler = call.lock().await;
        let track_handle = handler.play_only(src.into());
        let _ = track_handle.set_volume(volume);

        track_handle.add_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                guild_id,
                queue_manager: queue_manager.clone(),
                http_client: http_client.clone(),
                call: call.clone(),
            },
        )?;

        track_handle
    }; // handler lock dropped here

    {
        let mut queues = queue_manager.write().await;
        if let Some(q) = queues.get_mut(&guild_id) {
            q.track_handle = Some(track_handle);
        }
    }

    info!("재생 시작: {}", song.title);
    Ok(())
}

pub async fn play_next(
    guild_id: GuildId,
    queue_manager: &QueueManager,
    http_client: &reqwest::Client,
    call: &Arc<Mutex<Call>>,
    was_skipped: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let next = queue::get_next_song(queue_manager, guild_id, was_skipped).await;

    match next {
        Some(song) => {
            play_song(guild_id, queue_manager, http_client, call, &song).await?;
        }
        None => {
            info!("큐가 비었습니다 (guild: {guild_id})");
            let mut queues = queue_manager.write().await;
            if let Some(q) = queues.get_mut(&guild_id) {
                q.track_handle = None;
            }
        }
    }

    Ok(())
}
