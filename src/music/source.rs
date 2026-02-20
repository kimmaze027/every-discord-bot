use serde::Deserialize;
use tokio::process::Command;

use super::Song;

#[derive(Deserialize)]
struct YtDlpOutput {
    title: Option<String>,
    duration: Option<f64>,
    webpage_url: Option<String>,
    original_url: Option<String>,
}

pub(crate) fn is_url(query: &str) -> bool {
    query.starts_with("http://") || query.starts_with("https://")
}

pub(crate) fn parse_yt_dlp_output(
    stdout: &[u8],
) -> Result<Song, Box<dyn std::error::Error + Send + Sync>> {
    let info: YtDlpOutput = serde_json::from_slice(stdout)?;

    let duration = info.duration.map(|d| {
        let secs = d as u64;
        let mins = secs / 60;
        let remaining = secs % 60;
        format!("{mins}:{remaining:02}")
    });

    let url = info
        .webpage_url
        .or(info.original_url)
        .unwrap_or_default();

    Ok(Song {
        title: info.title.unwrap_or_else(|| "알 수 없음".to_string()),
        url,
        duration,
        requester: String::new(),
    })
}

pub async fn get_song_info(
    query: &str,
) -> Result<Song, Box<dyn std::error::Error + Send + Sync>> {
    let search_query = if is_url(query) {
        query.to_string()
    } else {
        format!("ytsearch1:{query}")
    };

    let output = Command::new("yt-dlp")
        .args([
            "-j",
            "-f",
            "bestaudio",
            "--no-playlist",
            "--no-warnings",
            &search_query,
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp 오류: {stderr}").into());
    }

    parse_yt_dlp_output(&output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Full JSON with all fields
    #[test]
    fn test_parse_full_output() {
        let json = br#"{"title":"Test Song","duration":185.0,"webpage_url":"https://youtube.com/watch?v=abc","original_url":"https://youtube.com/watch?v=abc"}"#;
        let song = parse_yt_dlp_output(json).unwrap();
        assert_eq!(song.title, "Test Song");
        assert_eq!(song.url, "https://youtube.com/watch?v=abc");
        assert_eq!(song.duration, Some("3:05".to_string()));
    }

    // 2. Missing title defaults to "알 수 없음"
    #[test]
    fn test_parse_missing_title() {
        let json = br#"{"duration":60.0,"webpage_url":"https://example.com"}"#;
        let song = parse_yt_dlp_output(json).unwrap();
        assert_eq!(song.title, "알 수 없음");
    }

    // 3. Missing duration → None
    #[test]
    fn test_parse_missing_duration() {
        let json = br#"{"title":"No Duration","webpage_url":"https://example.com"}"#;
        let song = parse_yt_dlp_output(json).unwrap();
        assert!(song.duration.is_none());
    }

    // 4. Duration formatting edge cases
    #[test]
    fn test_parse_duration_formatting() {
        let cases = [
            (0.0_f64, "0:00"),
            (59.0, "0:59"),
            (60.0, "1:00"),
            (3661.0, "61:01"),
        ];
        for (secs, expected) in cases {
            let json = format!(r#"{{"duration":{secs},"webpage_url":"https://example.com"}}"#);
            let song = parse_yt_dlp_output(json.as_bytes()).unwrap();
            assert_eq!(
                song.duration.as_deref(),
                Some(expected),
                "failed for {secs} seconds"
            );
        }
    }

    // 5. No webpage_url but has original_url → uses original_url
    #[test]
    fn test_parse_webpage_url_fallback() {
        let json = br#"{"title":"Fallback","original_url":"https://original.example.com"}"#;
        let song = parse_yt_dlp_output(json).unwrap();
        assert_eq!(song.url, "https://original.example.com");
    }

    // 6. Neither URL field → falls back to empty string
    #[test]
    fn test_parse_no_urls() {
        let json = br#"{"title":"No URL"}"#;
        let song = parse_yt_dlp_output(json).unwrap();
        assert_eq!(song.url, "");
    }

    // 7. is_url detection
    #[test]
    fn test_is_url_detection() {
        assert!(is_url("http://example.com"));
        assert!(is_url("https://youtube.com/watch?v=abc"));
        assert!(!is_url("my search query"));
        assert!(!is_url("lofi hip hop"));
        assert!(!is_url("ftp://example.com"));
    }
}
