pub mod player;
pub mod queue;
pub mod source;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use serenity::model::id::GuildId;
use songbird::tracks::TrackHandle;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct Song {
    pub title: String,
    pub url: String,
    pub duration: Option<String>,
    pub requester: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoopMode {
    #[default]
    Off,
    Song,
    Queue,
}

impl std::fmt::Display for LoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "끔"),
            Self::Song => write!(f, "한 곡 반복"),
            Self::Queue => write!(f, "전체 반복"),
        }
    }
}

pub struct GuildQueue {
    pub songs: VecDeque<Song>,
    pub current_song: Option<Song>,
    pub loop_mode: LoopMode,
    pub volume: f32,
    pub track_handle: Option<TrackHandle>,
}

impl Default for GuildQueue {
    fn default() -> Self {
        Self {
            songs: VecDeque::new(),
            current_song: None,
            loop_mode: LoopMode::Off,
            volume: 0.5,
            track_handle: None,
        }
    }
}

pub type QueueManager = Arc<RwLock<HashMap<GuildId, GuildQueue>>>;

pub fn new_queue_manager() -> QueueManager {
    Arc::new(RwLock::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_mode_display() {
        assert_eq!(LoopMode::Off.to_string(), "끔");
        assert_eq!(LoopMode::Song.to_string(), "한 곡 반복");
        assert_eq!(LoopMode::Queue.to_string(), "전체 반복");
    }

    #[test]
    fn test_guild_queue_default() {
        let q = GuildQueue::default();
        assert!(q.songs.is_empty());
        assert!(q.current_song.is_none());
        assert_eq!(q.loop_mode, LoopMode::Off);
        assert!((q.volume - 0.5).abs() < f32::EPSILON);
        assert!(q.track_handle.is_none());
    }
}
