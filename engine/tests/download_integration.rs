mod support;

use std::time::Duration;
use support::engine_download::{
    large_mp4_fixture_bytes, output_contains_ftyp, task_by_title, wait_for_any_running_or_progress,
    wait_for_task, EngineFixture,
};
use support::fixture_server;
use video_sniffing_engine::test_api::TaskStore;
use video_sniffing_engine::{Engine, TaskStatus};

fn fixtures_hls_dir() -> std::path::PathBuf {
    fixture_server::fixtures_dir().join("hls")
}

fn requeue_failed_task(data_dir: &std::path::Path, task_id: &str, new_url: &str) {
    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    let mut task = store.get(task_id).unwrap();
    task.source_url = new_url.to_string();
    task.status = TaskStatus::Queued;
    task.error_message = None;
    store.upsert(&task).unwrap();
}

#[test]
fn mp4_download_registers() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
        let url = format!("http://{addr}/sample.mp4");

        fx.engine.enqueue_single("sample", &url, None).unwrap();
        fx.engine.start_downloads().unwrap();
        wait_for_task(
            &fx.engine,
            &find_single_task_id(&fx.engine),
            TaskStatus::Completed,
            Duration::from_secs(30),
        )
        .await;
        fx.engine.stop_downloads().unwrap();

        assert_eq!(fx.engine.list_library().unwrap().len(), 1);
        let tasks = fx.engine.list_tasks().unwrap();
        let task = task_by_title(&tasks, "sample");
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output_path.is_some());
        assert!(task.library_item_id.is_some());
    });
}

#[test]
fn mp4_resume_after_stop() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let fixture_dir = fx.data_dir().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).unwrap();
        let sample = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
        std::fs::write(
            fixture_dir.join("large.mp4"),
            large_mp4_fixture_bytes(&sample),
        )
        .unwrap();

        let (addr, _guard) = fixture_server::serve_dir(fixture_dir).await;
        let url = format!("http://{addr}/large.mp4");
        let task_id = fx.engine.enqueue_single("large", &url, None).unwrap();

        fx.engine.start_downloads().unwrap();

        let mut interrupted = false;
        let interrupt_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < interrupt_deadline {
            let task = fx
                .engine
                .list_tasks()
                .unwrap()
                .into_iter()
                .find(|t| t.id == task_id)
                .unwrap();
            if task.status == TaskStatus::Running || task.progress_bytes > 0 {
                fx.engine.stop_downloads().unwrap();
                interrupted = true;
                break;
            }
            if task.status == TaskStatus::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        if !interrupted {
            let task = fx
                .engine
                .list_tasks()
                .unwrap()
                .into_iter()
                .find(|t| t.id == task_id)
                .unwrap();
            if task.status != TaskStatus::Completed {
                fx.engine.stop_downloads().unwrap();
            }
        }

        let task = fx
            .engine
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

            fx.engine.start_downloads().unwrap();
            fx.engine
                .drain_downloads_for_test(Duration::from_secs(30))
                .unwrap();
            wait_for_task(
                &fx.engine,
                &task_id,
                TaskStatus::Completed,
                Duration::from_secs(30),
            )
            .await;
            fx.engine.stop_downloads().unwrap();
        }

        let task = fx
            .engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output_path.is_some());
        assert_eq!(fx.engine.list_library().unwrap().len(), 1);

        let expected = large_mp4_fixture_bytes(&sample);
        let output = std::fs::read(task.output_path.as_ref().unwrap()).unwrap();
        assert_eq!(output, expected);
    });
}

#[test]
fn hls_plain_registers() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
        let url = format!("http://{addr}/media.m3u8");

        fx.engine.enqueue_single("hls-plain", &url, None).unwrap();
        fx.engine.start_downloads().unwrap();
        wait_for_task(
            &fx.engine,
            &find_single_task_id(&fx.engine),
            TaskStatus::Completed,
            Duration::from_secs(60),
        )
        .await;
        fx.engine.stop_downloads().unwrap();

        let tasks = fx.engine.list_tasks().unwrap();
        let task = task_by_title(&tasks, "hls-plain");
        assert!(task
            .output_path
            .as_ref()
            .is_some_and(|p| output_contains_ftyp(std::path::Path::new(p))));
        assert_eq!(fx.engine.list_library().unwrap().len(), 1);
    });
}

#[test]
fn hls_aes128_registers() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
        let url = format!("http://{addr}/encrypted.m3u8");

        fx.engine.enqueue_single("hls-aes128", &url, None).unwrap();
        fx.engine.start_downloads().unwrap();
        wait_for_task(
            &fx.engine,
            &find_single_task_id(&fx.engine),
            TaskStatus::Completed,
            Duration::from_secs(60),
        )
        .await;
        fx.engine.stop_downloads().unwrap();

        let tasks = fx.engine.list_tasks().unwrap();
        let task = task_by_title(&tasks, "hls-aes128");
        let path = std::path::Path::new(task.output_path.as_ref().unwrap());
        assert!(output_contains_ftyp(path));
        assert!(std::fs::metadata(path).unwrap().len() > 1024);
        assert_eq!(fx.engine.list_library().unwrap().len(), 1);
    });
}

#[test]
fn hls_master_highest_registers() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
        let url = format!("http://{addr}/master.m3u8");

        fx.engine
            .enqueue_single("hls-master", &url, Some("highest"))
            .unwrap();
        fx.engine.start_downloads().unwrap();
        wait_for_task(
            &fx.engine,
            &find_single_task_id(&fx.engine),
            TaskStatus::Completed,
            Duration::from_secs(60),
        )
        .await;
        fx.engine.stop_downloads().unwrap();

        let tasks = fx.engine.list_tasks().unwrap();
        let task = task_by_title(&tasks, "hls-master");
        assert!(task
            .output_path
            .as_ref()
            .is_some_and(|p| output_contains_ftyp(std::path::Path::new(p))));
        assert_eq!(fx.engine.list_library().unwrap().len(), 1);
    });
}

#[test]
fn series_partial_failure_resume() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
        let good_mp4 = format!("http://{addr}/sample.mp4");
        let bad_url = format!("http://{addr}/missing.mp4");

        let (_parent_id, children) = fx
            .engine
            .enqueue_episodes(
                "示意剧",
                Some(1),
                &[
                    (1, "第1集".into(), good_mp4.clone()),
                    (2, "第2集".into(), bad_url),
                    (3, "第3集".into(), good_mp4.clone()),
                ],
                None,
            )
            .unwrap();

        fx.engine.start_downloads().unwrap();

        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let tasks = fx.engine.list_tasks().unwrap();
            let ep1 = task_by_title(&tasks, "第1集");
            let ep2 = task_by_title(&tasks, "第2集");
            let ep3 = task_by_title(&tasks, "第3集");
            if ep1.status == TaskStatus::Completed
                && ep2.status == TaskStatus::Failed
                && ep3.status == TaskStatus::Completed
            {
                break;
            }
            if tokio::time::Instant::now() > deadline {
                panic!(
                    "timeout: ep1={:?} ep2={:?} ep3={:?}",
                    ep1.status, ep2.status, ep3.status
                );
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        fx.engine.stop_downloads().unwrap();

        let failed_id = children[1].clone();
        requeue_failed_task(fx.data_dir(), &failed_id, &good_mp4);

        fx.engine.start_downloads().unwrap();
        wait_for_task(
            &fx.engine,
            &failed_id,
            TaskStatus::Completed,
            Duration::from_secs(30),
        )
        .await;
        fx.engine.stop_downloads().unwrap();

        assert_eq!(fx.engine.list_library().unwrap().len(), 1);
        assert_eq!(
            fx.engine
                .list_episodes(&fx.engine.list_library().unwrap()[0].id)
                .unwrap()
                .len(),
            3
        );
    });
}

#[test]
fn pause_and_cancel() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let fixture_dir = fx.data_dir().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).unwrap();
        let sample = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
        std::fs::write(
            fixture_dir.join("large.mp4"),
            large_mp4_fixture_bytes(&sample),
        )
        .unwrap();

        let (addr, _guard) = fixture_server::serve_dir(fixture_dir).await;
        let url = format!("http://{addr}/large.mp4");

        // Pause path: interrupt mid-download, resume, complete.
        let pause_task_id = fx.engine.enqueue_single("pause-me", &url, None).unwrap();
        fx.engine.start_downloads().unwrap();

        if wait_for_any_running_or_progress(&fx.engine, &pause_task_id, Duration::from_secs(5))
            .await
        {
            fx.engine.pause_task(&pause_task_id).unwrap();
            wait_for_task(
                &fx.engine,
                &pause_task_id,
                TaskStatus::Paused,
                Duration::from_secs(10),
            )
            .await;

            let temp = fx.media_dir().join(".dl").join(&pause_task_id);
            assert!(temp.exists(), "paused task should keep temp dir");

            fx.engine.resume_task(&pause_task_id).unwrap();
            wait_for_task(
                &fx.engine,
                &pause_task_id,
                TaskStatus::Completed,
                Duration::from_secs(60),
            )
            .await;
        }

        // Cancel path: start another download and cancel it.
        let cancel_task_id = fx.engine.enqueue_single("cancel-me", &url, None).unwrap();
        if wait_for_any_running_or_progress(&fx.engine, &cancel_task_id, Duration::from_secs(5))
            .await
        {
            fx.engine.cancel_task(&cancel_task_id).unwrap();
            wait_for_task(
                &fx.engine,
                &cancel_task_id,
                TaskStatus::Cancelled,
                Duration::from_secs(10),
            )
            .await;

            let temp = fx.media_dir().join(".dl").join(&cancel_task_id);
            assert!(!temp.exists(), "cancelled task should remove temp dir");
        }

        fx.engine.stop_downloads().unwrap();
    });
}

fn find_single_task_id(engine: &Engine) -> String {
    engine
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.parent_id.is_none() && t.source_url.contains("http"))
        .map(|t| t.id)
        .expect("single download task not found")
}
