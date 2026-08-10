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
        episode_index: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
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
