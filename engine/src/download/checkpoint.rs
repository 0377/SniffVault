use crate::download::hls::playlist::{parse_media_playlist, MediaPlaylist};
use crate::download::mp4::mp4_part_path;
use crate::download::paths::output_filename;
use crate::error::EngineError;
use crate::tasks::TaskStore;
use crate::types::DownloadTask;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const HLS_MEDIA_URL_FILE: &str = "media.url";
pub const HLS_MEDIA_PLAYLIST_FILE: &str = "media.m3u8";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub version: u32,
    #[serde(flatten)]
    pub body: CheckpointBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckpointBody {
    Mp4 {
        temp_dir: String,
        part_path: String,
        bytes_done: u64,
    },
    Hls {
        temp_dir: String,
        media_playlist_url: String,
        variant_url: Option<String>,
        segments_done: Vec<u32>,
        segment_paths: Vec<String>,
        #[serde(default)]
        encryption: Option<HlsEncryption>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HlsEncryption {
    pub method: String,
    pub key_uri: String,
    pub iv_hex: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointRebuildStatus {
    AlreadyPresent,
    Rebuilt,
    NoRecoverableData,
    MissingMediaUrl,
}

pub(crate) fn hls_encryption_from_playlist(playlist: &MediaPlaylist) -> Option<HlsEncryption> {
    playlist.encryption.as_ref().map(|key| HlsEncryption {
        method: key.method.clone(),
        key_uri: key.uri.clone(),
        iv_hex: key.iv_hex.clone(),
    })
}

pub(crate) fn persist_hls_snapshot(
    temp_dir: &Path,
    media_url: &str,
    playlist_body: &str,
) -> Result<(), EngineError> {
    std::fs::create_dir_all(temp_dir)?;
    std::fs::write(temp_dir.join(HLS_MEDIA_URL_FILE), media_url)?;
    std::fs::write(temp_dir.join(HLS_MEDIA_PLAYLIST_FILE), playlist_body)?;
    Ok(())
}

fn is_hls_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains(".m3u8") || lower.ends_with("m3u8")
}

fn read_hls_snapshot(temp_dir: &Path) -> Result<Option<(String, MediaPlaylist)>, EngineError> {
    let url_path = temp_dir.join(HLS_MEDIA_URL_FILE);
    let playlist_path = temp_dir.join(HLS_MEDIA_PLAYLIST_FILE);
    if !url_path.is_file() || !playlist_path.is_file() {
        return Ok(None);
    }
    let media_url = std::fs::read_to_string(&url_path)?;
    let body = std::fs::read_to_string(&playlist_path)?;
    let playlist = parse_media_playlist(&body, &media_url)?;
    Ok(Some((media_url, playlist)))
}

fn scan_hls_segments(temp_dir: &Path) -> Result<Option<(Vec<u32>, Vec<String>)>, EngineError> {
    let mut pairs = Vec::new();
    for entry in std::fs::read_dir(temp_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if let Some(index_str) = name.strip_prefix("seg").and_then(|s| s.strip_suffix(".ts")) {
            if let Ok(index) = index_str.parse::<u32>() {
                pairs.push((index, path.to_string_lossy().into_owned()));
            }
        }
    }
    if pairs.is_empty() {
        return Ok(None);
    }
    pairs.sort_by_key(|(index, _)| *index);
    let (segments_done, segment_paths) = pairs.into_iter().unzip();
    Ok(Some((segments_done, segment_paths)))
}

fn resolve_hls_media_url(task: &DownloadTask, snapshot_url: Option<String>) -> Option<String> {
    snapshot_url
        .filter(|url| is_hls_url(url))
        .or_else(|| {
            task.resolved_media_url
                .clone()
                .filter(|url| is_hls_url(url))
        })
        .or_else(|| {
            if is_hls_url(&task.source_url) {
                Some(task.source_url.clone())
            } else {
                None
            }
        })
}

pub(crate) fn rebuild_checkpoint_from_temp(
    media_dir: &Path,
    task: &DownloadTask,
) -> Result<Option<Checkpoint>, EngineError> {
    let temp_dir = media_dir.join(".dl").join(&task.id);
    if !temp_dir.is_dir() {
        return Ok(None);
    }

    if let Some((segments_done, segment_paths)) = scan_hls_segments(&temp_dir)? {
        let snapshot = read_hls_snapshot(&temp_dir)?;
        let snapshot_url = snapshot.as_ref().map(|(url, _)| url.clone());
        let media_playlist_url = resolve_hls_media_url(task, snapshot_url);
        if let Some(media_playlist_url) = media_playlist_url {
            let encryption = snapshot
                .as_ref()
                .and_then(|(_, playlist)| hls_encryption_from_playlist(playlist));
            return Ok(Some(Checkpoint {
                version: 1,
                body: CheckpointBody::Hls {
                    temp_dir: temp_dir.to_string_lossy().into_owned(),
                    media_playlist_url,
                    variant_url: None,
                    segments_done,
                    segment_paths,
                    encryption,
                },
            }));
        }
        return Ok(None);
    }

    let output_path = media_dir.join(output_filename(task));
    let part = mp4_part_path(&temp_dir, &output_path);
    let (checkpoint_part, bytes_done) = if part.is_file() {
        (part.clone(), std::fs::metadata(&part)?.len())
    } else if output_path.is_file() {
        (output_path.clone(), std::fs::metadata(&output_path)?.len())
    } else {
        return Ok(None);
    };
    if bytes_done == 0 {
        return Ok(None);
    }

    Ok(Some(Checkpoint {
        version: 1,
        body: CheckpointBody::Mp4 {
            temp_dir: temp_dir.to_string_lossy().into_owned(),
            part_path: checkpoint_part.to_string_lossy().into_owned(),
            bytes_done,
        },
    }))
}

pub(crate) fn ensure_checkpoint_from_temp(
    store: &mut TaskStore,
    media_dir: &Path,
    task: &DownloadTask,
) -> Result<CheckpointRebuildStatus, EngineError> {
    if store.load_checkpoint(&task.id)?.is_some() {
        return Ok(CheckpointRebuildStatus::AlreadyPresent);
    }

    let temp_dir = media_dir.join(".dl").join(&task.id);
    let has_segments = scan_hls_segments(&temp_dir)?
        .map(|(segments, _)| !segments.is_empty())
        .unwrap_or(false);

    let Some(checkpoint) = rebuild_checkpoint_from_temp(media_dir, task)? else {
        if has_segments {
            return Ok(CheckpointRebuildStatus::MissingMediaUrl);
        }
        return Ok(CheckpointRebuildStatus::NoRecoverableData);
    };

    let rebuilt_progress = match &checkpoint.body {
        CheckpointBody::Hls { segments_done, .. } => segments_done.len() as u64,
        CheckpointBody::Mp4 { bytes_done, .. } => *bytes_done,
    };
    let progress = task.progress_bytes.max(rebuilt_progress);
    let total_bytes = match &checkpoint.body {
        CheckpointBody::Hls { .. } => task.total_bytes.or_else(|| {
            read_hls_snapshot(&temp_dir)
                .ok()
                .flatten()
                .map(|(_, playlist)| playlist.segments.len() as u64)
        }),
        CheckpointBody::Mp4 { .. } => task.total_bytes,
    };
    store.save_checkpoint(&task.id, &checkpoint)?;
    store.update_progress(&task.id, progress, total_bytes, task.status)?;
    Ok(CheckpointRebuildStatus::Rebuilt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskStore;
    use crate::types::{DownloadTask, TaskStatus};
    use tempfile::tempdir;

    fn sample_task(id: &str) -> DownloadTask {
        DownloadTask {
            id: id.into(),
            parent_id: None,
            season: None,
            title: "test".into(),
            source_url: "https://example.com/video.mp4".into(),
            quality_label: None,
            status: TaskStatus::Running,
            progress_bytes: 0,
            total_bytes: Some(4096),
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
        }
    }

    fn sample_checkpoint() -> Checkpoint {
        Checkpoint {
            version: 1,
            body: CheckpointBody::Mp4 {
                temp_dir: "/data/media/.dl/task-1/".into(),
                part_path: "/data/media/.dl/task-1/video.mp4.part".into(),
                bytes_done: 1048576,
            },
        }
    }

    #[test]
    fn checkpoint_roundtrip_via_store() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let mut store = TaskStore::open(&db_path).unwrap();

        let task = sample_task("task-1");
        store.upsert(&task).unwrap();

        let checkpoint = sample_checkpoint();
        store.save_checkpoint("task-1", &checkpoint).unwrap();

        let loaded = store.load_checkpoint("task-1").unwrap().unwrap();
        assert_eq!(loaded, checkpoint);

        store
            .update_progress_and_checkpoint("task-1", 2048, Some(4096), &checkpoint)
            .unwrap();
        let task = store.get("task-1").unwrap();
        assert_eq!(task.progress_bytes, 2048);
        assert_eq!(store.load_checkpoint("task-1").unwrap(), Some(checkpoint));

        store.clear_checkpoint("task-1").unwrap();
        assert_eq!(store.load_checkpoint("task-1").unwrap(), None);
    }

    #[test]
    fn rebuild_hls_checkpoint_from_temp_segments() {
        let dir = tempdir().unwrap();
        let media_dir = dir.path().join("media");
        let temp_dir = media_dir.join(".dl/task-1");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("seg0000.ts"), b"ts").unwrap();
        std::fs::write(temp_dir.join("seg0001.ts"), b"ts").unwrap();
        persist_hls_snapshot(
            &temp_dir,
            "https://example.com/media.m3u8",
            "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n#EXT-X-MEDIA-SEQUENCE:0\n#EXTINF:1.5,\nsegments/seg0.ts\n#EXT-X-ENDLIST\n",
        )
        .unwrap();

        let task = DownloadTask {
            id: "task-1".into(),
            parent_id: None,
            season: None,
            title: "ep".into(),
            source_url: "https://example.com/master.m3u8".into(),
            quality_label: None,
            status: TaskStatus::Running,
            progress_bytes: 0,
            total_bytes: Some(3),
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
        };

        let checkpoint = rebuild_checkpoint_from_temp(&media_dir, &task)
            .unwrap()
            .expect("checkpoint");
        match checkpoint.body {
            CheckpointBody::Hls {
                segments_done,
                segment_paths,
                media_playlist_url,
                ..
            } => {
                assert_eq!(segments_done, vec![0, 1]);
                assert_eq!(segment_paths.len(), 2);
                assert_eq!(media_playlist_url, "https://example.com/media.m3u8");
            }
            other => panic!("expected HLS checkpoint, got {other:?}"),
        }
    }

    #[test]
    fn rebuild_hls_checkpoint_uses_snapshot_encryption() {
        let dir = tempdir().unwrap();
        let media_dir = dir.path().join("media");
        let temp_dir = media_dir.join(".dl/task-1");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("seg0000.ts"), b"ts").unwrap();
        let encrypted_playlist = "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:1.5,\nsegments/seg0.ts\n#EXT-X-ENDLIST\n";
        persist_hls_snapshot(
            &temp_dir,
            "https://example.com/encrypted.m3u8",
            encrypted_playlist,
        )
        .unwrap();

        let task = DownloadTask {
            id: "task-1".into(),
            parent_id: None,
            season: None,
            title: "ep".into(),
            source_url: "https://example.com/page".into(),
            quality_label: None,
            status: TaskStatus::Running,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
        };

        let checkpoint = rebuild_checkpoint_from_temp(&media_dir, &task)
            .unwrap()
            .expect("checkpoint");
        match checkpoint.body {
            CheckpointBody::Hls { encryption, .. } => {
                let encryption = encryption.expect("encryption");
                assert_eq!(encryption.method, "AES-128");
                assert_eq!(encryption.key_uri, "key.bin");
            }
            other => panic!("expected HLS checkpoint, got {other:?}"),
        }
    }

    #[test]
    fn rebuild_mp4_checkpoint_from_part_file() {
        let dir = tempdir().unwrap();
        let media_dir = dir.path().join("media");
        let temp_dir = media_dir.join(".dl/task-1");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("test.mp4.part"), vec![0u8; 2048]).unwrap();

        let task = DownloadTask {
            id: "task-1".into(),
            parent_id: None,
            season: None,
            title: "test".into(),
            source_url: "https://example.com/video.mp4".into(),
            quality_label: None,
            status: TaskStatus::Running,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
        };

        let checkpoint = rebuild_checkpoint_from_temp(&media_dir, &task)
            .unwrap()
            .expect("checkpoint");
        match checkpoint.body {
            CheckpointBody::Mp4 { bytes_done, .. } => assert_eq!(bytes_done, 2048),
            other => panic!("expected MP4 checkpoint, got {other:?}"),
        }
    }

    #[test]
    fn ensure_checkpoint_preserves_higher_progress_bytes() {
        let dir = tempdir().unwrap();
        let media_dir = dir.path().join("media");
        let temp_dir = media_dir.join(".dl/task-1");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("seg0000.ts"), b"ts").unwrap();
        persist_hls_snapshot(
            &temp_dir,
            "https://example.com/media.m3u8",
            "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n#EXT-X-MEDIA-SEQUENCE:0\n#EXTINF:1.5,\nsegments/seg0.ts\n#EXT-X-ENDLIST\n",
        )
        .unwrap();

        let mut store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        let task = DownloadTask {
            id: "task-1".into(),
            parent_id: None,
            season: None,
            title: "ep".into(),
            source_url: "https://example.com/media.m3u8".into(),
            quality_label: None,
            status: TaskStatus::Running,
            progress_bytes: 120,
            total_bytes: Some(507),
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: Some("https://example.com/media.m3u8".into()),
        };
        store.upsert(&task).unwrap();

        let status =
            ensure_checkpoint_from_temp(&mut store, &media_dir, &task).expect("ensure checkpoint");
        assert_eq!(status, CheckpointRebuildStatus::Rebuilt);
        let updated = store.get("task-1").unwrap();
        assert_eq!(updated.progress_bytes, 120);
    }
}
