use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, TaskStatus};

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
