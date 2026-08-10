use serde::{Deserialize, Serialize};

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
}
