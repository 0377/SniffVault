use crate::download::mp4::mp4_part_path;
use crate::download::worker::output_filename;
use crate::error::EngineError;
use crate::tasks::TaskStore;
use crate::types::DownloadTask;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

fn is_hls_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains(".m3u8") || lower.ends_with("m3u8")
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

pub(crate) fn rebuild_checkpoint_from_temp(
    media_dir: &Path,
    task: &DownloadTask,
) -> Result<Option<Checkpoint>, EngineError> {
    let temp_dir = media_dir.join(".dl").join(&task.id);
    if !temp_dir.is_dir() {
        return Ok(None);
    }

    if let Some((segments_done, segment_paths)) = scan_hls_segments(&temp_dir)? {
        let media_playlist_url = task
            .resolved_media_url
            .clone()
            .filter(|url| is_hls_url(url))
            .or_else(|| {
                if is_hls_url(&task.source_url) {
                    Some(task.source_url.clone())
                } else {
                    None
                }
            });
        if let Some(media_playlist_url) = media_playlist_url {
            return Ok(Some(Checkpoint {
                version: 1,
                body: CheckpointBody::Hls {
                    temp_dir: temp_dir.to_string_lossy().into_owned(),
                    media_playlist_url,
                    variant_url: None,
                    segments_done,
                    segment_paths,
                    encryption: None,
                },
            }));
        }
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
) -> Result<(), EngineError> {
    if store.load_checkpoint(&task.id)?.is_some() {
        return Ok(());
    }
    let Some(checkpoint) = rebuild_checkpoint_from_temp(media_dir, task)? else {
        return Ok(());
    };
    let progress = match &checkpoint.body {
        CheckpointBody::Hls { segments_done, .. } => segments_done.len() as u64,
        CheckpointBody::Mp4 { bytes_done, .. } => *bytes_done,
    };
    store.save_checkpoint(&task.id, &checkpoint)?;
    store.update_progress(&task.id, progress, task.total_bytes, task.status)?;
    Ok(())
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
            resolved_media_url: Some("https://example.com/media.m3u8".into()),
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
}
