use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, Engine, EngineError, TaskStatus};

fn sample(id: &str, status: TaskStatus, error_message: Option<&str>) -> DownloadTask {
    DownloadTask {
        id: id.into(),
        parent_id: None,
        season: None,
        title: id.into(),
        source_url: "https://example.com/player.html".into(),
        quality_label: None,
        status,
        progress_bytes: 0,
        total_bytes: None,
        error_message: error_message.map(str::to_string),
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

#[test]
fn set_task_media_url_needs_sniff_to_queued() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    let media_url = "https://cdn.example.com/ep1.m3u8";

    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample(
                "sniff",
                TaskStatus::NeedsSniff,
                Some("needs_sniff"),
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.set_task_media_url("sniff", media_url).unwrap();

    let task = engine.list_tasks().unwrap().into_iter().next().unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.resolved_media_url.as_deref(), Some(media_url));
    assert!(task.error_message.is_none());

    let engine2 = Engine::open(dir.path()).unwrap();
    let persisted = engine2.list_tasks().unwrap().into_iter().next().unwrap();
    assert_eq!(persisted.status, TaskStatus::Queued);
    assert_eq!(persisted.resolved_media_url.as_deref(), Some(media_url));
}

#[test]
fn set_task_media_url_failed_needs_sniff_retry() {
    let dir = tempdir().unwrap();
    let media_url = "https://cdn.example.com/ep1.mp4";

    {
        let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        store
            .upsert(&sample("retry", TaskStatus::Failed, Some("needs_sniff")))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.set_task_media_url("retry", media_url).unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "retry")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.resolved_media_url.as_deref(), Some(media_url));
}

#[test]
fn set_task_media_url_rejects_wrong_status() {
    let dir = tempdir().unwrap();
    {
        let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        store
            .upsert(&sample("queued", TaskStatus::Queued, None))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine
        .set_task_media_url("queued", "https://cdn.example.com/v.mp4")
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn set_task_media_url_rejects_web_page() {
    let dir = tempdir().unwrap();
    {
        let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        store
            .upsert(&sample(
                "sniff",
                TaskStatus::NeedsSniff,
                Some("needs_sniff"),
            ))
            .unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine
        .set_task_media_url("sniff", "https://example.com/player.html")
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}
