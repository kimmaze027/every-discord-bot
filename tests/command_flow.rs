use every_discord_bot::music::{self, queue, LoopMode, Song};
use serenity::model::id::GuildId;

fn test_song(n: u32) -> Song {
    Song {
        title: format!("Song {n}"),
        url: format!("https://youtube.com/watch?v=test{n}"),
        duration: Some(format!("{n}:00")),
        requester: "user".to_string(),
    }
}

#[tokio::test]
async fn test_play_queue_skip_flow() {
    // Simulates: /play song1, /play song2, /play song3
    // Then: /nowplaying → song1, /queue → [song2, song3]
    // Then: /skip → song2 becomes current
    // Then: /skip → song3 becomes current
    // Then: /skip → queue empty
    let qm = music::new_queue_manager();
    let gid = GuildId::new(1);

    // User plays 3 songs
    queue::add_song(&qm, gid, test_song(1)).await;
    queue::add_song(&qm, gid, test_song(2)).await;
    queue::add_song(&qm, gid, test_song(3)).await;

    // First song starts (get_next_song simulates play_next)
    let current = queue::get_next_song(&qm, gid, false).await;
    assert_eq!(current.as_ref().unwrap().title, "Song 1");

    // /nowplaying
    let np = queue::get_current(&qm, gid).await;
    assert_eq!(np.unwrap().title, "Song 1");

    // /queue shows 2 remaining
    let (cur, songs) = queue::get_queue_list(&qm, gid).await;
    assert_eq!(cur.unwrap().title, "Song 1");
    assert_eq!(songs.len(), 2);

    // /skip → song 2
    let next = queue::get_next_song(&qm, gid, true).await;
    assert_eq!(next.unwrap().title, "Song 2");

    // /skip → song 3
    let next = queue::get_next_song(&qm, gid, true).await;
    assert_eq!(next.unwrap().title, "Song 3");

    // /skip → empty
    let next = queue::get_next_song(&qm, gid, true).await;
    assert!(next.is_none());
}

#[tokio::test]
async fn test_loop_song_flow() {
    // /play song1, /play song2, /loop song
    // Song ends naturally → repeats song1
    // /skip → moves to song2
    let qm = music::new_queue_manager();
    let gid = GuildId::new(2);

    queue::add_song(&qm, gid, test_song(1)).await;
    queue::add_song(&qm, gid, test_song(2)).await;

    let current = queue::get_next_song(&qm, gid, false).await;
    assert_eq!(current.unwrap().title, "Song 1");

    // /loop song
    queue::set_loop_mode(&qm, gid, LoopMode::Song).await;

    // Song ends naturally (not skipped) → repeats
    let repeated = queue::get_next_song(&qm, gid, false).await;
    assert_eq!(repeated.unwrap().title, "Song 1");

    // User skips → advances despite loop
    let next = queue::get_next_song(&qm, gid, true).await;
    assert_eq!(next.unwrap().title, "Song 2");
}

#[tokio::test]
async fn test_loop_queue_flow() {
    // /play song1, /play song2, /loop queue
    // Songs cycle: 1→2→1→2...
    let qm = music::new_queue_manager();
    let gid = GuildId::new(3);

    queue::add_song(&qm, gid, test_song(1)).await;
    queue::add_song(&qm, gid, test_song(2)).await;

    queue::set_loop_mode(&qm, gid, LoopMode::Queue).await;

    let s1 = queue::get_next_song(&qm, gid, false).await;
    assert_eq!(s1.unwrap().title, "Song 1");

    let s2 = queue::get_next_song(&qm, gid, false).await;
    assert_eq!(s2.unwrap().title, "Song 2");

    // Should cycle back to Song 1
    let s1_again = queue::get_next_song(&qm, gid, false).await;
    assert_eq!(s1_again.unwrap().title, "Song 1");
}

#[tokio::test]
async fn test_shuffle_remove_flow() {
    // /play 5 songs, /shuffle, /remove 2
    let qm = music::new_queue_manager();
    let gid = GuildId::new(4);

    for i in 1..=5 {
        queue::add_song(&qm, gid, test_song(i)).await;
    }

    // Start playing (pops first song)
    queue::get_next_song(&qm, gid, false).await;

    // /shuffle
    let count = queue::shuffle(&qm, gid).await;
    assert_eq!(count, 4); // 4 remaining in queue

    // /remove 1 (removes first in queue)
    let removed = queue::remove_at(&qm, gid, 1).await;
    assert!(removed.is_some());

    // 3 remaining
    let (_, songs) = queue::get_queue_list(&qm, gid).await;
    assert_eq!(songs.len(), 3);
}

#[tokio::test]
async fn test_volume_flow() {
    // Default volume → set to 80% → verify
    let qm = music::new_queue_manager();
    let gid = GuildId::new(5);

    // Default
    let vol = queue::get_volume(&qm, gid).await;
    assert!((vol - 0.5).abs() < f32::EPSILON);

    // /volume 80
    queue::set_volume(&qm, gid, 0.8).await;
    let vol = queue::get_volume(&qm, gid).await;
    assert!((vol - 0.8).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_stop_clears_everything() {
    // /play songs, /stop → everything cleared
    let qm = music::new_queue_manager();
    let gid = GuildId::new(6);

    queue::add_song(&qm, gid, test_song(1)).await;
    queue::add_song(&qm, gid, test_song(2)).await;
    queue::get_next_song(&qm, gid, false).await;

    // /stop
    queue::clear(&qm, gid).await;

    assert!(queue::is_empty(&qm, gid).await);
    assert!(queue::get_current(&qm, gid).await.is_none());
}

#[tokio::test]
async fn test_multiple_guilds_isolated() {
    // Two guilds don't interfere with each other
    let qm = music::new_queue_manager();
    let g1 = GuildId::new(100);
    let g2 = GuildId::new(200);

    queue::add_song(&qm, g1, test_song(1)).await;
    queue::add_song(&qm, g2, test_song(2)).await;

    let s1 = queue::get_next_song(&qm, g1, false).await;
    assert_eq!(s1.unwrap().title, "Song 1");

    let s2 = queue::get_next_song(&qm, g2, false).await;
    assert_eq!(s2.unwrap().title, "Song 2");

    // Each guild has current_song set but songs queue is empty
    let (cur1, songs1) = queue::get_queue_list(&qm, g1).await;
    assert_eq!(cur1.unwrap().title, "Song 1");
    assert!(songs1.is_empty());

    let (cur2, songs2) = queue::get_queue_list(&qm, g2).await;
    assert_eq!(cur2.unwrap().title, "Song 2");
    assert!(songs2.is_empty());

    // Clearing g1 doesn't affect g2
    queue::clear(&qm, g1).await;
    assert!(queue::is_empty(&qm, g1).await);
    assert!(!queue::is_empty(&qm, g2).await);
}
