use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, Engine, EngineError, TaskStatus};

fn sample(
    id: &str,
    parent: Option<&str>,
    status: TaskStatus,
    error_message: Option<&str>,
) -> DownloadTask {
    DownloadTask {
        id: id.into(),
        parent_id: parent.map(|p| p.into()),
        season: Some(1),
        title: id.into(),
        source_url: format!("https://example.com/{id}.m3u8"),
        quality_label: Some("720p".into()),
        status,
        progress_bytes: 0,
        total_bytes: None,
        error_message: error_message.map(|s| s.into()),
        output_path: None,
        library_item_id: None,
        episode_index: if parent.is_some() { Some(1) } else { None },
        created_at_ms: 1,
        updated_at_ms: 1,
        cookie_header: None,
        referer: None,
        resolved_media_url: None,
        poster_url: None,
    }
}

#[test]
fn retry_task_failed_to_queued_clears_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "failed",
                None,
                TaskStatus::Failed,
                Some("network error"),
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.retry_task("failed").unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "failed")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert!(task.error_message.is_none());
}

#[test]
fn retry_task_syncs_parent_status() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("parent", None, TaskStatus::Failed, None))
            .unwrap();
        store
            .upsert(&sample(
                "child",
                Some("parent"),
                TaskStatus::Failed,
                Some("http error"),
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.retry_task("child").unwrap();

    let parent = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "parent")
        .unwrap();
    assert_eq!(parent.status, TaskStatus::Queued);
}

#[test]
fn retry_task_rejects_non_failed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("queued", None, TaskStatus::Queued, None))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine.retry_task("queued").unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn retry_task_rejects_needs_sniff_failure() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "sniff",
                None,
                TaskStatus::Failed,
                Some("needs_sniff"),
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine.retry_task("sniff").unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}
