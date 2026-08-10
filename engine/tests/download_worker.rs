mod support;

use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use support::fixture_server;
use tempfile::tempdir;
use uuid::Uuid;
use video_sniffing_engine::test_api::{
    run_worker, BundledFfmpegLocator, DownloadCommand, LibraryStore, TaskStore, WorkerConfig,
};
use video_sniffing_engine::{DownloadTask, Engine, TaskStatus};

fn large_mp4_fixture_bytes() -> Vec<u8> {
    let sample = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
    sample.repeat(4_096)
}

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

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let config = WorkerConfig {
        data_dir: data_dir.to_path_buf(),
        media_dir: data_dir.join("media"),
        max_concurrency: 1,
        user_agent: None,
        default_quality_label: Some("highest".into()),
        ffmpeg: Arc::new(BundledFfmpegLocator),
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

#[test]
fn mp4_enqueue_start_completes_and_registers() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let dir = tempdir().unwrap();
        let mut engine = Engine::open(dir.path()).unwrap();
        let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
        let url = format!("http://{addr}/sample.mp4");

        engine.enqueue_single("sample", &url, None).unwrap();
        engine.start_downloads().unwrap();

        let deadline = Duration::from_secs(30);
        let start = std::time::Instant::now();
        loop {
            let tasks = engine.list_tasks().unwrap();
            let task = tasks.iter().find(|t| t.title == "sample").unwrap();
            if task.status == TaskStatus::Completed {
                break;
            }
            if task.status == TaskStatus::Failed {
                panic!("task failed: {:?}", task.error_message);
            }
            if start.elapsed() > deadline {
                panic!("timeout waiting for completion");
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        engine.stop_downloads().unwrap();

        assert_eq!(engine.list_library().unwrap().len(), 1);
        let tasks = engine.list_tasks().unwrap();
        let task = tasks.iter().find(|t| t.title == "sample").unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output_path.is_some());
        assert!(task.library_item_id.is_some());
    });
}

#[test]
fn mp4_resume_after_stop() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let dir = tempdir().unwrap();
        let fixture_dir = dir.path().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).unwrap();
        std::fs::write(fixture_dir.join("large.mp4"), large_mp4_fixture_bytes()).unwrap();

        let mut engine = Engine::open(dir.path()).unwrap();
        let (addr, _guard) = fixture_server::serve_dir(fixture_dir).await;
        let url = format!("http://{addr}/large.mp4");
        let task_id = engine.enqueue_single("large", &url, None).unwrap();

        engine.start_downloads().unwrap();

        let mut interrupted = false;
        let interrupt_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < interrupt_deadline {
            let task = engine
                .list_tasks()
                .unwrap()
                .into_iter()
                .find(|t| t.id == task_id)
                .unwrap();
            if task.status == TaskStatus::Running || task.progress_bytes > 0 {
                engine.stop_downloads().unwrap();
                interrupted = true;
                break;
            }
            if task.status == TaskStatus::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        if !interrupted {
            let task = engine
                .list_tasks()
                .unwrap()
                .into_iter()
                .find(|t| t.id == task_id)
                .unwrap();
            if task.status != TaskStatus::Completed {
                engine.stop_downloads().unwrap();
            }
        }

        let task = engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap();
        if task.status != TaskStatus::Completed {
            assert!(
                matches!(task.status, TaskStatus::Paused | TaskStatus::Queued),
                "unexpected status after stop: {:?}",
                task.status
            );

            engine.start_downloads().unwrap();
            engine
                .drain_downloads_for_test(Duration::from_secs(30))
                .unwrap();

            let deadline = Duration::from_secs(30);
            let start = std::time::Instant::now();
            loop {
                let task = engine
                    .list_tasks()
                    .unwrap()
                    .into_iter()
                    .find(|t| t.id == task_id)
                    .unwrap();
                if task.status == TaskStatus::Completed {
                    break;
                }
                if task.status == TaskStatus::Failed {
                    panic!("task failed: {:?}", task.error_message);
                }
                if start.elapsed() > deadline {
                    panic!("timeout waiting for completion after resume");
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            engine.stop_downloads().unwrap();
        }

        let task = engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output_path.is_some());
        assert_eq!(engine.list_library().unwrap().len(), 1);

        let expected = large_mp4_fixture_bytes();
        let output = std::fs::read(task.output_path.as_ref().unwrap()).unwrap();
        assert_eq!(output, expected);
    });
}
