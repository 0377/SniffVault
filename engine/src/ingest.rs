use crate::error::EngineError;
use crate::library::poster;
use crate::library::LibraryStore;
use crate::types::{LibraryEpisode, LibraryItem, LibraryItemKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub fn ensure_path_in_media_dir(media_dir: &Path, file_path: &str) -> Result<PathBuf, EngineError> {
    let path = PathBuf::from(file_path);
    let media = media_dir
        .canonicalize()
        .unwrap_or_else(|_| media_dir.to_path_buf());
    let canon = path
        .canonicalize()
        .map_err(|e| EngineError::InvalidArg(format!("file path not accessible: {e}")))?;
    if !canon.starts_with(&media) {
        return Err(EngineError::InvalidArg(
            "file path must be under media_dir".into(),
        ));
    }
    Ok(canon)
}

pub(crate) fn maybe_attach_poster(
    library: &LibraryStore,
    media_dir: &Path,
    item_id: &str,
    poster_url: &str,
    user_agent: Option<&str>,
) {
    if let Ok(item) = library.get_item(item_id) {
        if item.poster_path.is_some() {
            return;
        }
    }
    let client = match reqwest::blocking::Client::builder().build() {
        Ok(c) => c,
        Err(_) => return,
    };
    match poster::download_poster(&client, media_dir, item_id, poster_url, user_agent) {
        Ok(path) => {
            if let Err(e) = ensure_path_in_media_dir(media_dir, &path) {
                eprintln!("poster path rejected: {e}");
                return;
            }
            if let Err(e) = library.update_item_poster_path(item_id, &path) {
                eprintln!("poster db update failed: {e}");
            }
        }
        Err(e) => eprintln!("poster download skipped: {e}"),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn register_completed_episode_with_poster(
    library: &LibraryStore,
    media_dir: &Path,
    series_title: &str,
    season: Option<u32>,
    episode_index: u32,
    episode_title: &str,
    file_path: &str,
    source_url: Option<&str>,
    poster_url: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
    let canon = ensure_path_in_media_dir(media_dir, file_path)?;
    let existing = library.find_series_by_title_season(series_title, season)?;
    let item = if let Some(item) = existing {
        item
    } else {
        let item = LibraryItem {
            id: Uuid::new_v4().to_string(),
            kind: LibraryItemKind::Series,
            title: series_title.to_string(),
            season,
            poster_path: None,
            created_at_ms: now_ms(),
        };
        library.upsert_item(&item)?;
        item
    };

    let episode_id =
        if let Some(prev) = library.get_episode_by_item_index(&item.id, episode_index)? {
            prev.id
        } else {
            Uuid::new_v4().to_string()
        };

    let episode = LibraryEpisode {
        id: episode_id,
        item_id: item.id.clone(),
        index: episode_index,
        title: episode_title.to_string(),
        file_path: canon.to_string_lossy().into(),
        duration_ms: None,
        position_ms: 0,
        source_url: source_url.map(|s| s.to_string()),
    };
    library.upsert_episode(&episode)?;
    let episode = library
        .get_episode_by_item_index(&item.id, episode_index)?
        .ok_or_else(|| EngineError::Message("episode missing after upsert".into()))?;
    if let Some(url) = poster_url {
        maybe_attach_poster(library, media_dir, &item.id, url, user_agent);
    }
    let item = library.get_item(&item.id)?;
    Ok((item, episode))
}

pub(crate) fn register_completed_single_with_poster(
    library: &LibraryStore,
    media_dir: &Path,
    title: &str,
    file_path: &str,
    source_url: Option<&str>,
    poster_url: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
    let canon = ensure_path_in_media_dir(media_dir, file_path)?;
    let item = LibraryItem {
        id: Uuid::new_v4().to_string(),
        kind: LibraryItemKind::Single,
        title: title.to_string(),
        season: None,
        poster_path: None,
        created_at_ms: now_ms(),
    };
    library.upsert_item(&item)?;
    let episode = LibraryEpisode {
        id: Uuid::new_v4().to_string(),
        item_id: item.id.clone(),
        index: 1,
        title: title.to_string(),
        file_path: canon.to_string_lossy().into(),
        duration_ms: None,
        position_ms: 0,
        source_url: source_url.map(|s| s.to_string()),
    };
    library.upsert_episode(&episode)?;
    if let Some(url) = poster_url {
        maybe_attach_poster(library, media_dir, &item.id, url, user_agent);
    }
    let item = library.get_item(&item.id)?;
    Ok((item, episode))
}

#[allow(clippy::too_many_arguments)]
pub fn register_completed_episode(
    library: &LibraryStore,
    media_dir: &Path,
    series_title: &str,
    season: Option<u32>,
    episode_index: u32,
    episode_title: &str,
    file_path: &str,
    source_url: Option<&str>,
) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
    register_completed_episode_with_poster(
        library,
        media_dir,
        series_title,
        season,
        episode_index,
        episode_title,
        file_path,
        source_url,
        None,
        None,
    )
}

pub fn register_completed_single(
    library: &LibraryStore,
    media_dir: &Path,
    title: &str,
    file_path: &str,
    source_url: Option<&str>,
) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
    register_completed_single_with_poster(
        library, media_dir, title, file_path, source_url, None, None,
    )
}
