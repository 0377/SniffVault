mod support;

use std::time::Duration;
use support::engine_download::EngineFixture;
use support::fixture_server;
use video_sniffing_engine::{TaskEventKind, TaskStatus};

#[test]
fn start_downloads_emits_task_updated() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut fx = EngineFixture::open();
        let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
        let url = format!("http://{addr}/sample.mp4");

        fx.engine.enqueue_single("sample", &url, None).unwrap();
        fx.engine.start_downloads().unwrap();
        let rx = fx.engine.take_task_event_receiver().unwrap();

        let mut saw_running = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while tokio::time::Instant::now() < deadline {
            match rx.try_recv() {
                Ok(event) => {
                    if event.kind == TaskEventKind::TaskUpdated {
                        if let Some(task) = event.task {
                            if task.title == "sample" && task.status == TaskStatus::Running {
                                saw_running = true;
                                break;
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("task event channel closed before Running update");
                }
            }
        }
        assert!(saw_running, "expected TaskUpdated with Running status");

        fx.engine.stop_downloads().unwrap();

        let mut saw_stopped = false;
        let stop_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < stop_deadline {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    if event.kind == TaskEventKind::WorkerStopped {
                        assert!(event.task.is_none());
                        saw_stopped = true;
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("task event channel closed before WorkerStopped");
                }
            }
        }
        assert!(saw_stopped, "expected WorkerStopped on stop");
    });
}
