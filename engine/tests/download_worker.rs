mod support;

use std::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::time::Duration;
use support::engine_download::interruptible_mp4_fixture_bytes as build_interruptible_mp4;
use support::fixture_server;
use tempfile::tempdir;
use uuid::Uuid;
use video_sniffing_engine::test_api::{
    run_worker, BundledFfmpegLocator, DownloadCommand, LibraryStore, TaskStore, WorkerConfig,
};
use video_sniffing_engine::{DownloadTask, TaskStatus};

fn large_mp4_fixture_bytes() -> Vec<u8> {
    let sample = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
    build_interruptible_mp4(&sample)
}

fn serve_throttled_fixture(
    fixture_dir: std::path::PathBuf,
) -> impl std::future::Future<Output = (std::net::SocketAddr, fixture_server::ServerGuard)> {
    fixture_server::serve_dir_throttled(fixture_dir, 8_192, Duration::from_millis(5))
}

fn fixtures_hls_dir() -> std::path::PathBuf {
    fixture_server::fixtures_dir().join("hls")
}

static WORKER_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_worker_tests() -> std::sync::MutexGuard<'static, ()> {
    WORKER_TEST_LOCK.lock().unwrap()
}

fn spawn_worker(
    data_dir: &std::path::Path,
) -> (mpsc::Sender<DownloadCommand>, std::thread::JoinHandle<()>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let config = WorkerConfig {
        data_dir: data_dir.to_path_buf(),
        media_dir: data_dir.join("media"),
        max_concurrency: 1,
        user_agent: None,
        default_quality_label: Some("highest".into()),
        ffmpeg: Arc::new(BundledFfmpegLocator),
        task_event_tx: None,
    };
    let worker = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_worker(config, cmd_rx));
    });
    (cmd_tx, worker)
}

fn stop_worker(cmd_tx: &mpsc::Sender<DownloadCommand>, worker: std::thread::JoinHandle<()>) {
    let (stop_tx, stop_rx) = mpsc::channel();
    cmd_tx.send(DownloadCommand::Stop { ack: stop_tx }).unwrap();
    let _ = stop_rx.recv();
    worker.join().unwrap();
}

async fn wait_for_task_status(
    store: &TaskStore,
    task_id: &str,
    want: TaskStatus,
    timeout: Duration,
) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let task = store.get(task_id).unwrap();
        if task.status == want {
            return;
        }
        if task.status == TaskStatus::Failed {
            panic!("task failed: {:?}", task.error_message);
        }
        if tokio::time::Instant::now() > deadline {
            panic!("timeout waiting for {:?}, got {:?}", want, task.status);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_until_running_or_progress(
    store: &TaskStore,
    task_id: &str,
    timeout: Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let task = store.get(task_id).unwrap();
        if matches!(
            task.status,
            TaskStatus::Completed | TaskStatus::Cancelled | TaskStatus::Failed
        ) {
            return false;
        }
        if task.status == TaskStatus::Running || task.progress_bytes > 0 {
            return true;
        }
        if tokio::time::Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_for_running(store: &TaskStore, task_id: &str, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let task = store.get(task_id).unwrap();
        if task.status == TaskStatus::Running {
            return true;
        }
        if matches!(
            task.status,
            TaskStatus::Completed | TaskStatus::Cancelled | TaskStatus::Failed
        ) {
            return false;
        }
        if tokio::time::Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn worker_downloads_mp4_and_registers_library() {
    let _guard = lock_worker_tests();
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

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let config = WorkerConfig {
        data_dir: data_dir.to_path_buf(),
        media_dir: data_dir.join("media"),
        max_concurrency: 1,
        user_agent: None,
        default_quality_label: Some("highest".into()),
        ffmpeg: Arc::new(BundledFfmpegLocator),
        task_event_tx: None,
    };

    let worker = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_worker(config, cmd_rx));
    });

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

    let (stop_tx, stop_rx) = mpsc::channel();
    cmd_tx.send(DownloadCommand::Stop { ack: stop_tx }).unwrap();
    let _ = stop_rx.recv();
    worker.join().unwrap();

    let task = store.get(&task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.output_path.is_some());
    assert!(task.library_item_id.is_some());

    let library = LibraryStore::open(&data_dir.join("library.db")).unwrap();
    let items = library.list_items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "sample");
}

#[tokio::test]
async fn worker_downloads_hls_and_registers_library() {
    let _guard = lock_worker_tests();
    let dir = tempdir().unwrap();
    let data_dir = dir.path();
    std::fs::create_dir_all(data_dir.join("media")).unwrap();

    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    let now = 1i64;
    let task_id = Uuid::new_v4().to_string();
    let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
    let url = format!("http://{addr}/media.m3u8");

    store
        .upsert(&DownloadTask {
            id: task_id.clone(),
            parent_id: None,
            season: None,
            title: "hls-plain".into(),
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

    let (cmd_tx, worker) = spawn_worker(data_dir);
    wait_for_task_status(
        &store,
        &task_id,
        TaskStatus::Completed,
        Duration::from_secs(60),
    )
    .await;
    stop_worker(&cmd_tx, worker);

    let task = store.get(&task_id).unwrap();
    assert!(task.output_path.is_some());
    assert!(task.library_item_id.is_some());

    let library = LibraryStore::open(&data_dir.join("library.db")).unwrap();
    assert_eq!(library.list_items().unwrap().len(), 1);
}

#[tokio::test]
async fn worker_cancel_cleans_temp_dir() {
    let _guard = lock_worker_tests();
    let dir = tempdir().unwrap();
    let data_dir = dir.path();
    let fixture_dir = data_dir.join("fixtures");
    std::fs::create_dir_all(&fixture_dir).unwrap();
    std::fs::write(fixture_dir.join("large.mp4"), large_mp4_fixture_bytes()).unwrap();
    std::fs::create_dir_all(data_dir.join("media")).unwrap();

    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    let now = 1i64;
    let task_id = Uuid::new_v4().to_string();
    let (addr, _guard) = serve_throttled_fixture(fixture_dir).await;
    let url = format!("http://{addr}/large.mp4");

    store
        .upsert(&DownloadTask {
            id: task_id.clone(),
            parent_id: None,
            season: None,
            title: "large".into(),
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

    let (cmd_tx, worker) = spawn_worker(data_dir);
    if wait_until_running_or_progress(&store, &task_id, Duration::from_secs(5)).await {
        cmd_tx
            .send(DownloadCommand::Cancel {
                task_id: task_id.clone(),
            })
            .unwrap();
        wait_for_task_status(
            &store,
            &task_id,
            TaskStatus::Cancelled,
            Duration::from_secs(10),
        )
        .await;
    } else {
        panic!("download finished before cancel could be tested");
    }
    stop_worker(&cmd_tx, worker);

    let temp = data_dir.join("media").join(".dl").join(&task_id);
    assert!(!temp.exists(), "cancelled task should remove temp dir");
}

#[tokio::test]
async fn worker_pause_preserves_temp_dir() {
    let _guard = lock_worker_tests();
    let dir = tempdir().unwrap();
    let data_dir = dir.path();
    let fixture_dir = data_dir.join("fixtures");
    std::fs::create_dir_all(&fixture_dir).unwrap();
    std::fs::write(fixture_dir.join("large.mp4"), large_mp4_fixture_bytes()).unwrap();
    std::fs::create_dir_all(data_dir.join("media")).unwrap();

    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    let now = 1i64;
    let task_id = Uuid::new_v4().to_string();
    let (addr, _guard) = fixture_server::serve_dir_throttled(
        fixture_dir,
        1_024,
        Duration::from_millis(20),
    )
    .await;
    let url = format!("http://{addr}/large.mp4");

    store
        .upsert(&DownloadTask {
            id: task_id.clone(),
            parent_id: None,
            season: None,
            title: "large".into(),
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

    let (cmd_tx, worker) = spawn_worker(data_dir);
    if wait_for_running(&store, &task_id, Duration::from_secs(10)).await {
        cmd_tx
            .send(DownloadCommand::Pause {
                task_id: task_id.clone(),
            })
            .unwrap();
        wait_for_task_status(
            &store,
            &task_id,
            TaskStatus::Paused,
            Duration::from_secs(10),
        )
        .await;

        let temp = data_dir.join("media").join(".dl").join(&task_id);
        let has_checkpoint = matches!(
            store.load_checkpoint(&task_id),
            Ok(Some(_))
        );
        assert!(
            temp.exists() || has_checkpoint,
            "paused task should keep temp dir or checkpoint for resume"
        );
    } else {
        panic!("download finished before pause could be tested");
    }
    stop_worker(&cmd_tx, worker);

    let task = store.get(&task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Paused);
}
