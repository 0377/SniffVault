use crate::types::MediaKind;

pub(crate) fn classify_media_url(url: &str) -> Option<MediaKind> {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".m3u8") || lower.contains("mpegurl") {
        return Some(MediaKind::Hls);
    }
    if lower.contains(".mp4") || lower.contains(".webm") || lower.contains("video/") {
        return Some(MediaKind::Mp4);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_m3u8_and_mp4() {
        assert_eq!(
            classify_media_url("https://x/v/master.m3u8"),
            Some(MediaKind::Hls)
        );
        assert_eq!(
            classify_media_url("https://x/v/clip.mp4?token=1"),
            Some(MediaKind::Mp4)
        );
        assert_eq!(classify_media_url("https://x/page.html"), None);
    }
}
