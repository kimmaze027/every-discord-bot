use serenity::builder::CreateEmbed;

use crate::music::Song;

pub fn now_playing(song: &Song) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🎵 현재 재생 중")
        .description(format!("[{}]({})", song.title, song.url))
        .color(0x1DB954);

    if let Some(ref dur) = song.duration {
        embed = embed.field("길이", dur, true);
    }

    embed = embed.field("요청", &song.requester, true);
    embed
}

pub fn added_to_queue(song: &Song, position: usize) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("✅ 큐에 추가됨")
        .description(format!("[{}]({})", song.title, song.url))
        .color(0x5865F2);

    if let Some(ref dur) = song.duration {
        embed = embed.field("길이", dur, true);
    }

    embed = embed.field("위치", format!("#{position}"), true);
    embed
}

pub fn queue_list(current: Option<&Song>, songs: &[Song], page: usize) -> CreateEmbed {
    let per_page = 10;
    let total_pages = if songs.is_empty() {
        1
    } else {
        (songs.len() + per_page - 1) / per_page
    };
    let page = page.min(total_pages).max(1);

    let mut description = String::new();

    if let Some(song) = current {
        description.push_str(&format!(
            "**현재 재생:** [{}]({}){}\n\n",
            song.title,
            song.url,
            song.duration
                .as_ref()
                .map_or(String::new(), |d| format!(" `{d}`"))
        ));
    }

    if songs.is_empty() {
        description.push_str("큐가 비어있습니다.");
    } else {
        let start = (page - 1) * per_page;
        let end = (start + per_page).min(songs.len());

        for (i, song) in songs[start..end].iter().enumerate() {
            let num = start + i + 1;
            let dur = song
                .duration
                .as_ref()
                .map_or(String::new(), |d| format!(" `{d}`"));
            description.push_str(&format!("**{num}.** [{}]({}){dur}\n", song.title, song.url));
        }
    }

    CreateEmbed::new()
        .title(format!("📋 재생 목록 ({page}/{total_pages})"))
        .description(description)
        .color(0x5865F2)
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "총 {} 곡",
            songs.len()
        )))
}

pub fn error(message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("❌ 오류")
        .description(message)
        .color(0xED4245)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_song(title: &str) -> crate::music::Song {
        crate::music::Song {
            title: title.to_string(),
            url: format!("https://example.com/{title}"),
            duration: Some("3:00".to_string()),
            requester: "tester".to_string(),
        }
    }

    #[test]
    fn test_now_playing_creates_embed() {
        let song = test_song("Test Song");
        let _embed = now_playing(&song);
        // CreateEmbed is opaque; just verify creation doesn't panic
    }

    #[test]
    fn test_added_to_queue_creates_embed() {
        let song = test_song("Queued Song");
        let _embed = added_to_queue(&song, 3);
    }

    #[test]
    fn test_queue_list_empty() {
        let _embed = queue_list(None, &[], 1);
    }

    #[test]
    fn test_queue_list_with_songs() {
        let current = test_song("Current");
        let songs: Vec<crate::music::Song> = vec![
            test_song("Song 1"),
            test_song("Song 2"),
            test_song("Song 3"),
        ];
        let _embed = queue_list(Some(&current), &songs, 1);
    }

    #[test]
    fn test_queue_list_pagination() {
        let songs: Vec<crate::music::Song> = (1..=15)
            .map(|i| test_song(&format!("Song {i}")))
            .collect();
        // Page 2 should work without panicking
        let _embed = queue_list(None, &songs, 2);
    }

    #[test]
    fn test_error_embed_creates() {
        let _embed = error("something went wrong");
    }
}
