use serenity::model::id::GuildId;

use super::{LoopMode, QueueManager, Song};

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

pub async fn get_queue_list(
    manager: &QueueManager,
    guild_id: GuildId,
) -> (Option<Song>, Vec<Song>) {
    let queues = manager.read().await;
    match queues.get(&guild_id) {
        Some(queue) => (
            queue.current_song.clone(),
            queue.songs.iter().cloned().collect(),
        ),
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

pub async fn remove_at(manager: &QueueManager, guild_id: GuildId, position: usize) -> Option<Song> {
    let mut queues = manager.write().await;
    let queue = queues.entry(guild_id).or_default();
    if position > 0 && position <= queue.songs.len() {
        queue.songs.remove(position - 1)
    } else {
        None
    }
}

pub async fn set_loop_mode(manager: &QueueManager, guild_id: GuildId, mode: LoopMode) -> LoopMode {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::new_queue_manager;
    use serenity::model::id::GuildId;

    fn test_song(title: &str) -> Song {
        Song {
            title: title.to_string(),
            url: format!("https://example.com/{title}"),
            duration: Some("3:00".to_string()),
            requester: "tester".to_string(),
        }
    }

    const GUILD: GuildId = GuildId::new(1);

    // 1. add_song - adds song to queue, returns correct position
    #[tokio::test]
    async fn test_add_song_returns_position() {
        let manager = new_queue_manager();

        let pos1 = add_song(&manager, GUILD, test_song("Song A")).await;
        assert_eq!(pos1, 1);

        let pos2 = add_song(&manager, GUILD, test_song("Song B")).await;
        assert_eq!(pos2, 2);

        let pos3 = add_song(&manager, GUILD, test_song("Song C")).await;
        assert_eq!(pos3, 3);
    }

    // 2. get_next_song - normal mode: pops from front, sets current_song
    #[tokio::test]
    async fn test_get_next_song_normal_mode() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("First")).await;
        add_song(&manager, GUILD, test_song("Second")).await;

        let next = get_next_song(&manager, GUILD, false).await;
        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "First");

        // current_song should now be set
        let current = get_current(&manager, GUILD).await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().title, "First");

        // queue should have one song left
        let (_, remaining) = get_queue_list(&manager, GUILD).await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "Second");
    }

    // 3. get_next_song with LoopMode::Song - repeats current song (not skipped)
    #[tokio::test]
    async fn test_get_next_song_loop_song_repeats_current() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("Looping")).await;
        add_song(&manager, GUILD, test_song("Next")).await;

        // Advance once to set current_song
        get_next_song(&manager, GUILD, false).await;

        set_loop_mode(&manager, GUILD, LoopMode::Song).await;

        // was_skipped = false: should return current song again
        let repeated = get_next_song(&manager, GUILD, false).await;
        assert!(repeated.is_some());
        assert_eq!(repeated.unwrap().title, "Looping");

        // Queue should remain unchanged
        let (_, remaining) = get_queue_list(&manager, GUILD).await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "Next");
    }

    // 4. get_next_song with LoopMode::Song + was_skipped=true - advances to next
    #[tokio::test]
    async fn test_get_next_song_loop_song_skipped_advances() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("First")).await;
        add_song(&manager, GUILD, test_song("Second")).await;

        // Advance once to set current_song to "First"
        get_next_song(&manager, GUILD, false).await;

        set_loop_mode(&manager, GUILD, LoopMode::Song).await;

        // was_skipped = true: should advance past current song
        let next = get_next_song(&manager, GUILD, true).await;
        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "Second");

        let current = get_current(&manager, GUILD).await;
        assert_eq!(current.unwrap().title, "Second");
    }

    // 5. get_next_song with LoopMode::Queue - cycles current back to end
    #[tokio::test]
    async fn test_get_next_song_loop_queue_cycles() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("A")).await;
        add_song(&manager, GUILD, test_song("B")).await;
        add_song(&manager, GUILD, test_song("C")).await;

        // Advance once: current = "A", queue = [B, C]
        get_next_song(&manager, GUILD, false).await;

        set_loop_mode(&manager, GUILD, LoopMode::Queue).await;

        // Next call: "A" should be pushed to back, "B" becomes current
        let next = get_next_song(&manager, GUILD, false).await;
        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "B");

        let (_, remaining) = get_queue_list(&manager, GUILD).await;
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].title, "C");
        assert_eq!(remaining[1].title, "A");
    }

    // 6. get_next_song when queue is empty - returns None
    #[tokio::test]
    async fn test_get_next_song_empty_queue_returns_none() {
        let manager = new_queue_manager();

        let next = get_next_song(&manager, GUILD, false).await;
        assert!(next.is_none());

        // current_song should remain None
        let current = get_current(&manager, GUILD).await;
        assert!(current.is_none());
    }

    // 7. clear - empties queue and current_song
    #[tokio::test]
    async fn test_clear_empties_queue_and_current() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("Song 1")).await;
        add_song(&manager, GUILD, test_song("Song 2")).await;

        // Advance to populate current_song
        get_next_song(&manager, GUILD, false).await;

        clear(&manager, GUILD).await;

        let current = get_current(&manager, GUILD).await;
        assert!(current.is_none());

        let (cur, remaining) = get_queue_list(&manager, GUILD).await;
        assert!(cur.is_none());
        assert!(remaining.is_empty());

        assert!(is_empty(&manager, GUILD).await);
    }

    // 8. shuffle - changes order, preserves count
    #[tokio::test]
    async fn test_shuffle_preserves_count() {
        let manager = new_queue_manager();
        let titles = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"];
        for title in &titles {
            add_song(&manager, GUILD, test_song(title)).await;
        }

        let count = shuffle(&manager, GUILD).await;
        assert_eq!(count, titles.len());

        let (_, remaining) = get_queue_list(&manager, GUILD).await;
        assert_eq!(remaining.len(), titles.len());

        // All original titles must still be present
        let mut shuffled_titles: Vec<String> = remaining.iter().map(|s| s.title.clone()).collect();
        shuffled_titles.sort();
        let mut original: Vec<String> = titles.iter().map(|s| s.to_string()).collect();
        original.sort();
        assert_eq!(shuffled_titles, original);
    }

    // 9. remove_at - removes correct song, returns it
    #[tokio::test]
    async fn test_remove_at_removes_correct_song() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("First")).await;
        add_song(&manager, GUILD, test_song("Second")).await;
        add_song(&manager, GUILD, test_song("Third")).await;

        // Remove position 2 (1-indexed), which is "Second"
        let removed = remove_at(&manager, GUILD, 2).await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().title, "Second");

        let (_, remaining) = get_queue_list(&manager, GUILD).await;
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].title, "First");
        assert_eq!(remaining[1].title, "Third");
    }

    // 10. remove_at with invalid position - returns None
    #[tokio::test]
    async fn test_remove_at_invalid_position_returns_none() {
        let manager = new_queue_manager();
        add_song(&manager, GUILD, test_song("Only")).await;

        // Position 0 is invalid
        let result = remove_at(&manager, GUILD, 0).await;
        assert!(result.is_none());

        // Position beyond queue length is invalid
        let result = remove_at(&manager, GUILD, 5).await;
        assert!(result.is_none());

        // Queue should be untouched
        let (_, remaining) = get_queue_list(&manager, GUILD).await;
        assert_eq!(remaining.len(), 1);
    }

    // 11. set_volume - updates volume
    #[tokio::test]
    async fn test_set_volume_updates_volume() {
        let manager = new_queue_manager();

        set_volume(&manager, GUILD, 0.8).await;
        let vol = get_volume(&manager, GUILD).await;
        assert!((vol - 0.8).abs() < f32::EPSILON);

        set_volume(&manager, GUILD, 0.25).await;
        let vol = get_volume(&manager, GUILD).await;
        assert!((vol - 0.25).abs() < f32::EPSILON);
    }

    // 12. get_volume - returns default 0.5 when no guild queue
    #[tokio::test]
    async fn test_get_volume_default_when_no_guild() {
        let manager = new_queue_manager();
        let vol = get_volume(&manager, GUILD).await;
        assert!((vol - 0.5).abs() < f32::EPSILON);
    }

    // 13. is_empty - true when no current_song and no songs in queue
    #[tokio::test]
    async fn test_is_empty() {
        let manager = new_queue_manager();

        // No guild entry at all -> empty
        assert!(is_empty(&manager, GUILD).await);

        add_song(&manager, GUILD, test_song("A")).await;
        assert!(!is_empty(&manager, GUILD).await);

        // Pop the song into current_song
        get_next_song(&manager, GUILD, false).await;
        // current_song is set, queue is empty -> not empty
        assert!(!is_empty(&manager, GUILD).await);

        clear(&manager, GUILD).await;
        assert!(is_empty(&manager, GUILD).await);
    }

    // 14. set_loop_mode / get_loop_mode
    #[tokio::test]
    async fn test_set_and_get_loop_mode() {
        let manager = new_queue_manager();

        // Default is Off
        let mode = get_loop_mode(&manager, GUILD).await;
        assert_eq!(mode, LoopMode::Off);

        set_loop_mode(&manager, GUILD, LoopMode::Song).await;
        assert_eq!(get_loop_mode(&manager, GUILD).await, LoopMode::Song);

        set_loop_mode(&manager, GUILD, LoopMode::Queue).await;
        assert_eq!(get_loop_mode(&manager, GUILD).await, LoopMode::Queue);

        set_loop_mode(&manager, GUILD, LoopMode::Off).await;
        assert_eq!(get_loop_mode(&manager, GUILD).await, LoopMode::Off);
    }
}
