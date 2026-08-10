mod support;

use std::sync::Arc;
use std::time::Duration;
use support::fixture_server;
use tempfile::tempdir;
use tokio::sync::mpsc;
use uuid::Uuid;
use video_sniffing_engine::test_api::{
    run_worker, BundledFfmpegLocator, DownloadCommand, LibraryStore, TaskStore, WorkerConfig,
};
use video_sniffing_engine::{DownloadTask, TaskStatus};

#[tokio::test]
async fn worker_downloads_mp4_and_registers_library() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path();
    std::fs::create_dir_all(data_dir.join("media")).unwrap();

    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    let now = 1i64;
    let task_id = Uuid::new_v4().to_string();
    store
        .upsert(&DownloadTask {
            id: task_id.clone(),
            parent_id: None,
            season: None,
            title: "sample".into(),
            source_url: String::new(),
            quality_label: None,
            status: TaskStatus::Queued,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: now,
            updated_at_ms: now,
        })
        .unwrap();

    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");
    store
        .upsert(&DownloadTask {
            id: task_id.clone(),
            parent_id: None,
            season: None,
            title: "sample".into(),
            source_url: url,
            quality_label: None,
            status: TaskStatus::Queued,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: now,
            updated_at_ms: now,
        })
        .unwrap();

    let (cmd_tx, cmd_rx) = mpsc::channel(8);
    let config = WorkerConfig {
        data_dir: data_dir.to_path_buf(),
        media_dir: data_dir.join("media"),
        max_concurrency: 1,
        user_agent: None,
        default_quality_label: Some("highest".into()),
        ffmpeg: Arc::new(BundledFfmpegLocator),
    };

    let worker = tokio::spawn(run_worker(config, cmd_rx));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let task = store.get(&task_id).unwrap();
        if task.status == TaskStatus::Completed {
            break;
        }
        if task.status == TaskStatus::Failed {
            panic!("task failed: {:?}", task.error_message);
        }
        if tokio::time::Instant::now() > deadline {
            panic!("timeout waiting for completion");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    cmd_tx
        .send(DownloadCommand::Stop { ack: stop_tx })
        .await
        .unwrap();
    let _ = stop_rx.await;
    worker.await.unwrap();

    let task = store.get(&task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.output_path.is_some());
    assert!(task.library_item_id.is_some());

    let library = LibraryStore::open(&data_dir.join("library.db")).unwrap();
    let items = library.list_items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "sample");
}
