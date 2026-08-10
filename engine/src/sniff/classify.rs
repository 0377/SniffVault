use crate::types::MediaKind;

pub fn classify_media_url(url: &str) -> Option<MediaKind> {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".m3u8") || lower.contains("mpegurl") {
        return Some(MediaKind::Hls);
    }
    if lower.contains(".mp4") || lower.contains(".webm") || lower.contains("video/") {
        return Some(MediaKind::Mp4);
    }
    None
}
