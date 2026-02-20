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

pub async fn get_song_info(
    query: &str,
) -> Result<Song, Box<dyn std::error::Error + Send + Sync>> {
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    let search_query = if is_url {
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

    let info: YtDlpOutput = serde_json::from_slice(&output.stdout)?;

    let duration = info.duration.map(|d| {
        let secs = d as u64;
        let mins = secs / 60;
        let remaining = secs % 60;
        format!("{mins}:{remaining:02}")
    });

    let url = info
        .webpage_url
        .or(info.original_url)
        .unwrap_or_else(|| query.to_string());

    Ok(Song {
        title: info.title.unwrap_or_else(|| "알 수 없음".to_string()),
        url,
        duration,
        requester: String::new(),
    })
}
