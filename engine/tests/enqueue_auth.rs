use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadAuth, DownloadTask, Engine, TaskStatus};

fn sample_task(id: &str) -> DownloadTask {
    DownloadTask {
        id: id.into(),
        parent_id: None,
        season: Some(1),
        title: id.into(),
        source_url: format!("https://example.com/{id}.m3u8"),
        quality_label: Some("720p".into()),
        status: TaskStatus::Queued,
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
    }
}

#[test]
fn enqueue_single_persists_auth_on_task() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let auth = DownloadAuth {
        cookies: Some("sid=ok".into()),
        referer: Some("https://x/page".into()),
    };
    let id = engine
        .enqueue_single("t", "https://x/v.mp4", None, Some(&auth))
        .unwrap();
    drop(engine);
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    let task = store.get(&id).unwrap();
    assert_eq!(task.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(task.referer.as_deref(), Some("https://x/page"));
}

#[test]
fn enqueue_episodes_copies_auth_to_parent_and_children() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let auth = DownloadAuth {
        cookies: Some("sid=ok".into()),
        referer: Some("https://x/page".into()),
    };
    let (parent_id, child_ids) = engine
        .enqueue_episodes(
            "show",
            Some(1),
            &[(1, "e1".into(), "https://x/1.mp4".into())],
            None,
            Some(&auth),
        )
        .unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    let parent = store.get(&parent_id).unwrap();
    let child = store.get(&child_ids[0]).unwrap();
    assert_eq!(parent.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(child.cookie_header.as_deref(), Some("sid=ok"));
    assert_eq!(child.referer.as_deref(), Some("https://x/page"));
}

#[test]
fn enqueue_episodes_empty_writes_nothing() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    assert!(engine
        .enqueue_episodes("show", None, &[], None, None)
        .is_err());
    assert!(engine.list_tasks().unwrap().is_empty());
}

#[test]
fn upsert_parent_rolls_back_when_child_id_empty() {
    let dir = tempdir().unwrap();
    let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
    let parent = sample_task("p");
    let mut child = sample_task("c1");
    child.parent_id = Some("p".into());
    child.id.clear();
    assert!(store
        .upsert_parent_with_children(&parent, &[child])
        .is_err());
    assert!(store.get("p").is_err());
}
