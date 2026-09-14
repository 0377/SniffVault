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
    let original_url = "https://example.com/failed.m3u8";
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample(
            "failed",
            None,
            TaskStatus::Failed,
            Some("network error"),
        );
        task.source_url = original_url.into();
        store.upsert(&task).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.retry_task("failed", None).unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "failed")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert!(task.error_message.is_none());
    assert_eq!(task.source_url, original_url);
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
    engine.retry_task("child", None).unwrap();

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
    let err = engine.retry_task("queued", None).unwrap_err();
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
    let err = engine.retry_task("sniff", None).unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn retry_task_with_new_url_updates_source_and_clears_progress() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("failed2", None, TaskStatus::Failed, Some("http error"));
        task.source_url = "https://example.com/old.mp4".into();
        task.resolved_media_url = Some("https://cdn.example/old.m3u8".into());
        task.progress_bytes = 50;
        task.total_bytes = Some(100);
        task.output_path = Some("/tmp/x.mp4".into());
        store.upsert(&task).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine
        .retry_task("failed2", Some("https://example.com/new.mp4"))
        .unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "failed2")
        .unwrap();
    assert_eq!(task.source_url, "https://example.com/new.mp4");
    assert_eq!(task.status, TaskStatus::Queued);
    assert!(task.resolved_media_url.is_none());
    assert_eq!(task.progress_bytes, 0);
    assert!(task.total_bytes.is_none());
    assert!(task.output_path.is_none());
    let store = TaskStore::open(&path).unwrap();
    assert!(store.load_checkpoint("failed2").unwrap().is_none());
}

#[test]
fn retry_task_with_new_url_cleans_dl_temp_dir() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media_dir = engine.media_dir();
    let dl_dir = media_dir.join(".dl").join("failed3");
    std::fs::create_dir_all(&dl_dir).unwrap();
    std::fs::write(dl_dir.join("part.bin"), b"partial").unwrap();

    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("failed3", None, TaskStatus::Failed, Some("http error"));
        task.source_url = "https://example.com/old.mp4".into();
        store.upsert(&task).unwrap();
    }

    engine
        .retry_task("failed3", Some("https://example.com/new.mp4"))
        .unwrap();

    assert!(!dl_dir.exists());
}

#[test]
fn retry_task_same_url_degrades_to_fast_path() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    let url = "https://example.com/same.mp4";
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("same", None, TaskStatus::Failed, Some("err"));
        task.source_url = url.into();
        task.resolved_media_url = Some("https://cdn.example/cached.m3u8".into());
        task.progress_bytes = 10;
        store.upsert(&task).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine.retry_task("same", Some(url)).unwrap();

    let task = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "same")
        .unwrap();
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.source_url, url);
    assert_eq!(
        task.resolved_media_url.as_deref(),
        Some("https://cdn.example/cached.m3u8")
    );
    assert_eq!(task.progress_bytes, 10);
}

#[test]
fn retry_task_rejects_empty_new_url() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("failed", None, TaskStatus::Failed, Some("e")))
            .unwrap();
    }
    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine.retry_task("failed", Some("   ")).unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn retry_task_with_new_url_syncs_parent_status() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("parent", None, TaskStatus::Failed, None))
            .unwrap();
        let mut child = sample("child", Some("parent"), TaskStatus::Failed, Some("e"));
        child.source_url = "https://example.com/old.mp4".into();
        store.upsert(&child).unwrap();
    }

    let mut engine = Engine::open(dir.path()).unwrap();
    engine
        .retry_task("child", Some("https://example.com/new.mp4"))
        .unwrap();

    let parent = engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == "parent")
        .unwrap();
    assert_eq!(parent.status, TaskStatus::Queued);
}
