use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, Engine, EngineError, TaskStatus};

fn sample(id: &str, parent: Option<&str>, status: TaskStatus) -> DownloadTask {
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
        error_message: None,
        output_path: None,
        library_item_id: None,
        episode_index: if parent.is_some() { Some(1) } else { None },
        created_at_ms: 1,
        updated_at_ms: 1,
        cookie_header: None,
        referer: None,
        resolved_media_url: None,
    }
}

#[test]
fn list_runnable_tasks_excludes_parent_container() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    store
        .upsert(&sample("parent", None, TaskStatus::Queued))
        .unwrap();
    let parent = store.get("parent").unwrap();
    let parent_with_empty_url = DownloadTask {
        source_url: String::new(),
        ..parent
    };
    store.upsert(&parent_with_empty_url).unwrap();

    store
        .upsert(&sample("child", Some("parent"), TaskStatus::Queued))
        .unwrap();
    store
        .upsert(&DownloadTask {
            id: "single".into(),
            parent_id: None,
            season: None,
            title: "movie".into(),
            source_url: "https://example.com/m.mp4".into(),
            quality_label: None,
            status: TaskStatus::Queued,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 2,
            updated_at_ms: 2,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
        })
        .unwrap();

    let runnable = store.list_runnable_tasks(10).unwrap();
    assert_eq!(runnable.len(), 2);
    let ids: Vec<&str> = runnable.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"child"));
    assert!(ids.contains(&"single"));
    assert!(!ids.contains(&"parent"));
}

#[test]
fn sync_parent_status_aggregates_children() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    store
        .upsert(&sample("parent", None, TaskStatus::Queued))
        .unwrap();
    store
        .upsert(&sample("c1", Some("parent"), TaskStatus::Completed))
        .unwrap();
    store
        .upsert(&sample("c2", Some("parent"), TaskStatus::Queued))
        .unwrap();

    store.sync_parent_status("parent").unwrap();
    let parent = store.get("parent").unwrap();
    assert_eq!(parent.status, TaskStatus::Queued);

    store
        .upsert(&sample("c2", Some("parent"), TaskStatus::Running))
        .unwrap();
    store.sync_parent_status("parent").unwrap();
    let parent = store.get("parent").unwrap();
    assert_eq!(parent.status, TaskStatus::Running);

    store
        .upsert(&sample("c2", Some("parent"), TaskStatus::Completed))
        .unwrap();
    store.sync_parent_status("parent").unwrap();
    let parent = store.get("parent").unwrap();
    assert_eq!(parent.status, TaskStatus::Completed);
}

#[test]
fn sync_parent_status_when_all_children_paused() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    store
        .upsert(&sample("parent", None, TaskStatus::Running))
        .unwrap();
    store
        .upsert(&sample("c1", Some("parent"), TaskStatus::Paused))
        .unwrap();
    store
        .upsert(&sample("c2", Some("parent"), TaskStatus::Paused))
        .unwrap();

    store.sync_parent_status("parent").unwrap();
    let parent = store.get("parent").unwrap();
    assert_eq!(parent.status, TaskStatus::Paused);
}

#[test]
fn list_runnable_tasks_excludes_needs_sniff() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    store
        .upsert(&sample("sniff", None, TaskStatus::NeedsSniff))
        .unwrap();
    store
        .upsert(&sample("queued", None, TaskStatus::Queued))
        .unwrap();

    let runnable = store.list_runnable_tasks(10).unwrap();
    assert_eq!(runnable.len(), 1);
    assert_eq!(runnable[0].id, "queued");
}

#[test]
fn sync_parent_status_with_needs_sniff_child_not_completed() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    store
        .upsert(&sample("parent", None, TaskStatus::Running))
        .unwrap();
    store
        .upsert(&sample("c1", Some("parent"), TaskStatus::Completed))
        .unwrap();
    store
        .upsert(&sample("c2", Some("parent"), TaskStatus::NeedsSniff))
        .unwrap();

    store.sync_parent_status("parent").unwrap();
    let parent = store.get("parent").unwrap();
    assert_ne!(parent.status, TaskStatus::Completed);
    assert_eq!(parent.status, TaskStatus::Running);
}

#[test]
fn upsert_parent_with_children_is_atomic() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    let parent = sample("parent", None, TaskStatus::Queued);
    let child1 = sample("c1", Some("parent"), TaskStatus::Queued);
    let child2 = sample("c2", Some("parent"), TaskStatus::Queued);

    store
        .upsert_parent_with_children(&parent, &[child1, child2])
        .unwrap();

    assert_eq!(store.list_children("parent").unwrap().len(), 2);
    assert!(store.get("parent").is_ok());
}

#[test]
fn parent_child_progress_counts_completed() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();

    store
        .upsert(&sample("parent", None, TaskStatus::Running))
        .unwrap();
    store
        .upsert(&sample("c1", Some("parent"), TaskStatus::Completed))
        .unwrap();
    store
        .upsert(&sample("c2", Some("parent"), TaskStatus::Failed))
        .unwrap();
    store
        .upsert(&sample("c3", Some("parent"), TaskStatus::Queued))
        .unwrap();

    let (done, total) = store.parent_progress("parent").unwrap();
    assert_eq!(total, 3);
    assert_eq!(done, 1);

    store.mark_failed("c3", "network error").unwrap();
    let t = store.get("c3").unwrap();
    assert_eq!(t.status, TaskStatus::Failed);
    assert_eq!(t.error_message.as_deref(), Some("network error"));
}

#[test]
fn auth_snapshot_survives_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        let mut task = sample("a", None, TaskStatus::Queued);
        task.cookie_header = Some("sid=ok".into());
        task.referer = Some("http://x/page".into());
        store.upsert(&task).unwrap();
    }
    let store = TaskStore::open(&path).unwrap();
    let got = store.get("a").unwrap();
    assert_eq!(got.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(got.referer.as_deref(), Some("http://x/page"));
}

#[test]
fn set_resolved_media_url_survives_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("a", None, TaskStatus::Queued))
            .unwrap();
        store
            .set_resolved_media_url("a", "https://cdn.example.com/v.m3u8")
            .unwrap();
    }
    let store = TaskStore::open(&path).unwrap();
    let got = store.get("a").unwrap();
    assert_eq!(
        got.resolved_media_url.as_deref(),
        Some("https://cdn.example.com/v.m3u8")
    );
}

#[test]
fn update_progress_preserves_resolved_media_url() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    store
        .upsert(&sample("a", None, TaskStatus::Queued))
        .unwrap();
    store
        .set_resolved_media_url("a", "https://cdn.example.com/v.m3u8")
        .unwrap();
    store
        .update_progress("a", 1024, Some(4096), TaskStatus::Running)
        .unwrap();
    let got = store.get("a").unwrap();
    assert_eq!(
        got.resolved_media_url.as_deref(),
        Some("https://cdn.example.com/v.m3u8")
    );
    assert_eq!(got.progress_bytes, 1024);
}

#[test]
fn resume_task_rejects_needs_sniff() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    {
        let store = TaskStore::open(&path).unwrap();
        store
            .upsert(&sample("sniff", None, TaskStatus::NeedsSniff))
            .unwrap();
    }
    let mut engine = Engine::open(dir.path()).unwrap();
    let err = engine.resume_task("sniff").unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn upsert_preserves_resolved_media_url() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    store
        .upsert(&sample("a", None, TaskStatus::Queued))
        .unwrap();
    store
        .set_resolved_media_url("a", "https://cdn.example.com/v.m3u8")
        .unwrap();
    let updated = DownloadTask {
        title: "updated".into(),
        status: TaskStatus::Running,
        progress_bytes: 512,
        ..store.get("a").unwrap()
    };
    store.upsert(&updated).unwrap();
    let got = store.get("a").unwrap();
    assert_eq!(
        got.resolved_media_url.as_deref(),
        Some("https://cdn.example.com/v.m3u8")
    );
    assert_eq!(got.title, "updated");
    assert_eq!(got.status, TaskStatus::Running);
    assert_eq!(got.progress_bytes, 512);
}
