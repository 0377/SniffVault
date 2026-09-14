//! Engine 下载集成测试共享辅助（各 integration test crate 按需引用）。
#![allow(dead_code)]

use std::time::Duration;
use tempfile::TempDir;
use video_sniffing_engine::{DownloadTask, Engine, TaskStatus};

pub fn large_mp4_fixture_bytes(sample: &[u8]) -> Vec<u8> {
    sample.repeat(4_096)
}

/// 用于 pause/cancel 等需中断下载的测试：限速 HTTP + 约 4MB 文件。
pub fn interruptible_mp4_fixture_bytes(sample: &[u8]) -> Vec<u8> {
    const TARGET_BYTES: usize = 4 * 1024 * 1024;
    let repeat = TARGET_BYTES.div_ceil(sample.len());
    let mut bytes = sample.repeat(repeat);
    bytes.truncate(TARGET_BYTES);
    bytes
}

pub async fn wait_for_task_failed(engine: &Engine, task_id: &str, timeout: Duration) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let task = engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap_or_else(|| panic!("task {task_id} not found"));
        if task.status == TaskStatus::Failed {
            return;
        }
        if task.status == TaskStatus::Completed {
            panic!("task {task_id} completed unexpectedly");
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "timeout waiting for task {task_id} to fail, got {:?}",
                task.status
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn wait_for_task(engine: &Engine, task_id: &str, want: TaskStatus, timeout: Duration) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let task = engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap_or_else(|| panic!("task {task_id} not found"));
        if task.status == want {
            return;
        }
        if task.status == TaskStatus::Failed {
            panic!("task {task_id} failed: {:?}", task.error_message);
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "timeout waiting for task {task_id} to reach {want:?}, got {:?}",
                task.status
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn wait_for_hls_segment_file(
    media_dir: &std::path::Path,
    task_id: &str,
    segment_index: u32,
    engine: &Engine,
    timeout: Duration,
) {
    let seg_path = media_dir
        .join(".dl")
        .join(task_id)
        .join(format!("seg{segment_index:04}.ts"));
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if seg_path.is_file() {
            return;
        }
        let task = engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap_or_else(|| panic!("task {task_id} not found"));
        if task.status == TaskStatus::Completed {
            panic!("download finished before partial progress could be captured");
        }
        if tokio::time::Instant::now() > deadline {
            panic!("timeout waiting for HLS segment {segment_index}");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub async fn wait_for_any_running_or_progress(
    engine: &Engine,
    task_id: &str,
    timeout: Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let task = engine
            .list_tasks()
            .unwrap()
            .into_iter()
            .find(|t| t.id == task_id)
            .unwrap();
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

pub fn task_by_title<'a>(tasks: &'a [DownloadTask], title: &str) -> &'a DownloadTask {
    tasks
        .iter()
        .find(|t| t.title == title)
        .unwrap_or_else(|| panic!("task with title {title} not found"))
}

pub fn output_contains_ftyp(path: &std::path::Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    bytes.windows(4).any(|window| window == b"ftyp")
}

pub struct EngineFixture {
    pub dir: TempDir,
    pub engine: Engine,
}

impl EngineFixture {
    pub fn open() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path()).unwrap();
        Self { dir, engine }
    }

    pub fn data_dir(&self) -> &std::path::Path {
        self.dir.path()
    }

    pub fn media_dir(&self) -> std::path::PathBuf {
        self.engine.media_dir()
    }
}
