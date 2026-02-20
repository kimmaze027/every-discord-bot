use discord_music_bot::music::source;

#[tokio::test]
#[ignore] // Requires yt-dlp installed and network access
async fn test_get_song_info_with_url() {
    // Use a well-known, stable YouTube video (Rick Astley - Never Gonna Give You Up)
    let result = source::get_song_info("https://www.youtube.com/watch?v=dQw4w9WgXcQ").await;
    assert!(result.is_ok(), "get_song_info failed: {:?}", result.err());
    let song = result.unwrap();
    assert!(!song.title.is_empty());
    assert!(song.url.contains("youtube.com") || song.url.contains("youtu.be"));
    assert!(song.duration.is_some());
}

#[tokio::test]
#[ignore] // Requires yt-dlp installed and network access
async fn test_get_song_info_with_search() {
    let result = source::get_song_info("never gonna give you up rick astley").await;
    assert!(result.is_ok(), "search failed: {:?}", result.err());
    let song = result.unwrap();
    assert!(!song.title.is_empty());
    assert!(song.duration.is_some());
}
