use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, Engine, EngineError, TaskStatus};

fn sample(
    id: &str,
    parent: Option<&str>,
    status: TaskStatus,
    source_url: &str,
    resolved_media_url: Option<&str>,
) -> DownloadTask {
    DownloadTask {
        id: id.into(),
        parent_id: parent.map(|p| p.into()),
        season: Some(1),
        title: id.into(),
        source_url: source_url.into(),
        quality_label: Some("720p".into()),
        status,
        progress_bytes: 0,
        total_bytes: None,
        error_message: None,
        output_path: None,
        library_item_id: None,
        episode_index: if parent.is_some() { Some(1) } else { None },
        created_at_ms: 1,
        updated_at_ms: 1,
        cookie_header: None,
        referer: None,
        resolved_media_url: resolved_media_url.map(|s| s.into()),
    }
}

#[test]
fn restore_cancelled_web_page_to_needs_sniff() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "sniff",
                Some("parent"),
                TaskStatus::Cancelled,
                "https://example.com/play/1",
                None,
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.restore_task("sniff").unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "sniff")
        .unwrap();
    assert_eq!(task.status, TaskStatus::NeedsSniff);
    assert_eq!(task.error_message.as_deref(), Some("needs_sniff"));
}

#[test]
fn restore_cancelled_with_media_url_to_queued() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "dl",
                None,
                TaskStatus::Cancelled,
                "https://example.com/play/1",
                Some("https://cdn.example.com/video.m3u8"),
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.restore_task("dl").unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "dl")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert!(task.error_message.is_none());
}

#[test]
fn restore_cancelled_direct_media_to_queued() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "mp4",
                None,
                TaskStatus::Cancelled,
                "https://cdn.example.com/video.mp4",
                None,
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.restore_task("mp4").unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "mp4")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
}

#[test]
fn restore_cancelled_syncs_parent_status() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("parent", None, TaskStatus::Cancelled, "", None))
            .unwrap();
        store
            .upsert(&sample(
                "child",
                Some("parent"),
                TaskStatus::Cancelled,
                "https://example.com/play/1",
                None,
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.restore_task("child").unwrap();

    let parent = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "parent")
        .unwrap();
    assert_ne!(parent.status, TaskStatus::Cancelled);
}

#[test]
fn restore_task_rejects_non_cancelled() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "queued",
                None,
                TaskStatus::Queued,
                "https://cdn.example.com/video.mp4",
                None,
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine.restore_task("queued").unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}
