use crate::types::{LibraryEpisode, LibraryItem};
use std::path::Path;

use super::types::CastMetadata;

pub fn sanitize_episode(item: &LibraryItem, episode: &LibraryEpisode) -> CastMetadata {
    CastMetadata {
        title: item.title.clone(),
        season: item.season,
        episode_index: episode.index,
        episode_title: episode.title.clone(),
        duration_ms: episode.duration_ms,
        position_ms: episode.position_ms,
        mime: mime_from_path(&episode.file_path),
    }
}

fn mime_from_path(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_ascii_lowercase().as_str() {
            "mp4" | "m4v" => "video/mp4",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            _ => "application/octet-stream",
        })
        .unwrap_or("application/octet-stream")
        .to_string()
}
