use serenity::model::id::GuildId;

use super::{GuildQueue, LoopMode, QueueManager, Song};

pub async fn add_song(manager: &QueueManager, guild_id: GuildId, song: Song) -> usize {
    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();
    queue.songs.push_back(song);
    queue.songs.len()
}

pub async fn get_next_song(
    manager: &QueueManager,
    guild_id: GuildId,
    was_skipped: bool,
) -> Option<Song> {
    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();

    if !was_skipped && queue.loop_mode == LoopMode::Song {
        return queue.current_song.clone();
    }

    if queue.loop_mode == LoopMode::Queue {
        if let Some(current) = queue.current_song.take() {
            queue.songs.push_back(current);
        }
    }

    let next = queue.songs.pop_front();
    queue.current_song = next.clone();
    next
}

pub async fn clear(manager: &QueueManager, guild_id: GuildId) {
    let mut queues = manager.write().await;
    if let Some(queue) = queues.get_mut(&guild_id) {
        queue.songs.clear();
        queue.current_song = None;
        queue.track_handle = None;
    }
}

pub async fn get_queue_list(manager: &QueueManager, guild_id: GuildId) -> (Option<Song>, Vec<Song>) {
    let queues = manager.read().await;
    match queues.get(&guild_id) {
        Some(queue) => (queue.current_song.clone(), queue.songs.iter().cloned().collect()),
        None => (None, vec![]),
    }
}

pub async fn shuffle(manager: &QueueManager, guild_id: GuildId) -> usize {
    use rand::seq::SliceRandom;

    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();
    let mut songs: Vec<Song> = queue.songs.drain(..).collect();
    songs.shuffle(&mut rand::thread_rng());
    let len = songs.len();
    queue.songs = songs.into();
    len
}

pub async fn remove_at(
    manager: &QueueManager,
    guild_id: GuildId,
    position: usize,
) -> Option<Song> {
    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();
    if position > 0 && position <= queue.songs.len() {
        queue.songs.remove(position - 1)
    } else {
        None
    }
}

pub async fn set_loop_mode(
    manager: &QueueManager,
    guild_id: GuildId,
    mode: LoopMode,
) -> LoopMode {
    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();
    queue.loop_mode = mode.clone();
    mode
}

pub async fn set_volume(manager: &QueueManager, guild_id: GuildId, volume: f32) {
    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();
    queue.volume = volume;
    if let Some(handle) = &queue.track_handle {
        let _ = handle.set_volume(volume);
    }
}

pub async fn get_current(manager: &QueueManager, guild_id: GuildId) -> Option<Song> {
    let queues = manager.read().await;
    queues.get(&guild_id).and_then(|q| q.current_song.clone())
}

pub async fn get_volume(manager: &QueueManager, guild_id: GuildId) -> f32 {
    let queues = manager.read().await;
    queues.get(&guild_id).map_or(0.5, |q| q.volume)
}

pub async fn get_loop_mode(manager: &QueueManager, guild_id: GuildId) -> LoopMode {
    let queues = manager.read().await;
    queues
        .get(&guild_id)
        .map_or(LoopMode::Off, |q| q.loop_mode.clone())
}

pub async fn is_empty(manager: &QueueManager, guild_id: GuildId) -> bool {
    let queues = manager.read().await;
    queues
        .get(&guild_id)
        .map_or(true, |q| q.current_song.is_none() && q.songs.is_empty())
}
