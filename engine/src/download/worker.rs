use crate::download::checkpoint::{Checkpoint, CheckpointBody};
use crate::download::ffmpeg::FfmpegLocator;
use crate::download::hls::{download_hls_to_mp4, HlsContext};
use crate::download::http::HttpClient;
use crate::download::mp4::{download_mp4, mp4_part_path, Mp4Context};
use crate::download::scheduler::Scheduler;
use crate::error::EngineError;
use crate::ingest;
use crate::library::LibraryStore;
use crate::tasks::TaskStore;
use crate::types::{DownloadTask, TaskStatus};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct WorkerConfig {
    pub data_dir: PathBuf,
    pub media_dir: PathBuf,
    pub max_concurrency: u32,
    pub user_agent: Option<String>,
    pub default_quality_label: Option<String>,
    pub ffmpeg: Arc<dyn FfmpegLocator>,
}

pub enum DownloadCommand {
    Pause { task_id: String },
    Resume { task_id: String },
    Cancel { task_id: String },
    Stop { ack: mpsc::Sender<()> },
}

enum TaskRunOutcome {
    Success,
    Cancelled,
    Failed(EngineError),
    DiskFull,
}

pub async fn run_worker(config: WorkerConfig, cmd_rx: mpsc::Receiver<DownloadCommand>) {
    let config = Arc::new(config);
    let scheduler = Arc::new(tokio::sync::Mutex::new(Scheduler::new(
        config.max_concurrency,
    )));
    let active = Arc::new(AtomicUsize::new(0));
    let task_cancels: Arc<tokio::sync::Mutex<HashMap<String, CancellationToken>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let mut shutdown_ack: Option<mpsc::Sender<()>> = None;
    let mut stopping = false;

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                DownloadCommand::Pause { task_id } => {
                    if let Some(token) = task_cancels.lock().await.remove(&task_id) {
                        token.cancel();
                    }
                    let tasks_path = config.data_dir.join("tasks.db");
                    if let Ok(store) = TaskStore::open(&tasks_path) {
                        let _ = store.set_task_status(&task_id, TaskStatus::Paused, None);
                    }
                }
                DownloadCommand::Resume { task_id } => {
                    let tasks_path = config.data_dir.join("tasks.db");
                    if let Ok(store) = TaskStore::open(&tasks_path) {
                        let _ = store.set_task_status(&task_id, TaskStatus::Queued, None);
                    }
                }
                DownloadCommand::Cancel { task_id } => {
                    if let Some(token) = task_cancels.lock().await.remove(&task_id) {
                        token.cancel();
                    }
                    let tasks_path = config.data_dir.join("tasks.db");
                    if let Ok(store) = TaskStore::open(&tasks_path) {
                        let _ = store.set_task_status(&task_id, TaskStatus::Cancelled, None);
                        if let Ok(task) = store.get(&task_id) {
                            cleanup_temp_dir(&config.media_dir, &task.id);
                        }
                    }
                }
                DownloadCommand::Stop { ack } => {
                    stopping = true;
                    shutdown_ack = Some(ack);
                    let tasks_path = config.data_dir.join("tasks.db");
                    if let Ok(store) = TaskStore::open(&tasks_path) {
                        if let Ok(all) = store.list_all() {
                            for task in all {
                                if task.status == TaskStatus::Running {
                                    let _ =
                                        store.set_task_status(&task.id, TaskStatus::Paused, None);
                                }
                            }
                        }
                    }
                    let tokens: Vec<CancellationToken> =
                        task_cancels.lock().await.drain().map(|(_, t)| t).collect();
                    for token in tokens {
                        token.cancel();
                    }
                }
            }
        }

        if stopping && active.load(Ordering::SeqCst) == 0 {
            if let Some(ack) = shutdown_ack.take() {
                let _ = ack.send(());
            }
            break;
        }

        let slots = {
            let sched = scheduler.lock().await;
            sched.available_slots(active.load(Ordering::SeqCst))
        };

        if slots > 0 {
            let tasks_path = config.data_dir.join("tasks.db");
            if let Ok(store) = TaskStore::open(&tasks_path) {
                let sched = scheduler.lock().await;
                if let Ok(runnable) = sched.pick_next(&store, active.load(Ordering::SeqCst), slots)
                {
                    for task in runnable {
                        let token = CancellationToken::new();
                        task_cancels
                            .lock()
                            .await
                            .insert(task.id.clone(), token.clone());

                        if let Err(e) = store.set_task_status(&task.id, TaskStatus::Running, None) {
                            tracing_log(&format!("set running failed: {e}"));
                            task_cancels.lock().await.remove(&task.id);
                            continue;
                        }
                        if let Some(parent_id) = &task.parent_id {
                            let _ = store.sync_parent_status(parent_id);
                        }

                        active.fetch_add(1, Ordering::SeqCst);
                        let cfg = config.clone();
                        let scheduler_ref = scheduler.clone();
                        let active_ref = active.clone();
                        let cancels_ref = task_cancels.clone();

                        tokio::spawn(async move {
                            let outcome = run_one_task(&cfg, &task, token).await;
                            handle_outcome(&cfg, &task, outcome, scheduler_ref, cancels_ref).await;
                            active_ref.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn tracing_log(_msg: &str) {}

async fn handle_outcome(
    config: &WorkerConfig,
    task: &DownloadTask,
    outcome: TaskRunOutcome,
    scheduler: Arc<tokio::sync::Mutex<Scheduler>>,
    task_cancels: Arc<tokio::sync::Mutex<HashMap<String, CancellationToken>>>,
) {
    task_cancels.lock().await.remove(&task.id);

    let tasks_path = config.data_dir.join("tasks.db");
    let store = match TaskStore::open(&tasks_path) {
        Ok(s) => s,
        Err(_) => return,
    };

    match outcome {
        TaskRunOutcome::Success => {}
        TaskRunOutcome::Cancelled => {
            if let Ok(current) = store.get(&task.id) {
                if current.status == TaskStatus::Paused {
                    let _ = save_interrupt_checkpoint(config, task);
                    return;
                }
            }
            let _ = store.set_task_status(&task.id, TaskStatus::Cancelled, None);
            cleanup_temp_dir(&config.media_dir, &task.id);
        }
        TaskRunOutcome::DiskFull => {
            scheduler.lock().await.pause_globally();
            let _ = store.set_task_status(&task.id, TaskStatus::Paused, None);
        }
        TaskRunOutcome::Failed(err) => {
            let msg = err.to_string();
            let _ = store.mark_failed(&task.id, &msg);
        }
    }

    if let Some(parent_id) = &task.parent_id {
        let _ = store.sync_parent_status(parent_id);
    }
}

async fn run_one_task(
    config: &WorkerConfig,
    task: &DownloadTask,
    cancel: CancellationToken,
) -> TaskRunOutcome {
    if cancel.is_cancelled() {
        return TaskRunOutcome::Cancelled;
    }

    let tasks_path = config.data_dir.join("tasks.db");
    let library_path = config.data_dir.join("library.db");
    let mut tasks = match TaskStore::open(&tasks_path) {
        Ok(s) => s,
        Err(e) => return TaskRunOutcome::Failed(e),
    };
    let library = match LibraryStore::open(&library_path) {
        Ok(s) => s,
        Err(e) => return TaskRunOutcome::Failed(e),
    };

    let checkpoint = match tasks.load_checkpoint(&task.id) {
        Ok(cp) => cp,
        Err(e) => return TaskRunOutcome::Failed(e),
    };

    let output_path = config.media_dir.join(output_filename(task));
    let temp_dir = config.media_dir.join(".dl").join(&task.id);
    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        return TaskRunOutcome::Failed(EngineError::from(e));
    }
    if let Some(parent) = output_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return TaskRunOutcome::Failed(EngineError::from(e));
        }
    }

    let http = match HttpClient::new(config.user_agent.as_deref()) {
        Ok(c) => c.with_cancellation(cancel.clone()),
        Err(e) => return TaskRunOutcome::Failed(e),
    };

    let quality = task
        .quality_label
        .as_deref()
        .or(config.default_quality_label.as_deref());

    let download_result = if is_hls_url(&task.source_url) {
        let ffmpeg = match config.ffmpeg.resolve() {
            Ok(p) => p,
            Err(e) => return TaskRunOutcome::Failed(e),
        };
        let ctx = HlsContext {
            http: &http,
            temp_dir: &temp_dir,
            ffmpeg: &ffmpeg,
        };
        download_hls_to_mp4(&ctx, &task.source_url, &output_path, quality, checkpoint)
            .await
            .map(|p| (p, 0u64))
    } else {
        let ctx = Mp4Context {
            http: &http,
            temp_dir: &temp_dir,
        };
        download_mp4(&ctx, &task.source_url, &output_path, checkpoint).await
    };

    if cancel.is_cancelled() {
        let _ = save_interrupt_checkpoint_from_store(config, task, &tasks);
        return TaskRunOutcome::Cancelled;
    }

    match download_result {
        Ok((final_path, _bytes)) => {
            let ingest_result = if let Some(parent_id) = &task.parent_id {
                let parent = match tasks.get(parent_id) {
                    Ok(p) => p,
                    Err(e) => return TaskRunOutcome::Failed(e),
                };
                let episode_index = task.episode_index.unwrap_or(1);
                ingest::register_completed_episode(
                    &library,
                    &config.media_dir,
                    &parent.title,
                    task.season,
                    episode_index,
                    &task.title,
                    final_path.to_str().unwrap_or_default(),
                    Some(&task.source_url),
                )
            } else {
                ingest::register_completed_single(
                    &library,
                    &config.media_dir,
                    &task.title,
                    final_path.to_str().unwrap_or_default(),
                    Some(&task.source_url),
                )
            };

            match ingest_result {
                Ok((item, _episode)) => {
                    let path_str = final_path.to_string_lossy().into_owned();
                    if let Err(e) = tasks.set_output_path(&task.id, &path_str) {
                        return TaskRunOutcome::Failed(e);
                    }
                    if let Err(e) = tasks.set_library_item_id(&task.id, &item.id) {
                        return TaskRunOutcome::Failed(e);
                    }
                    if let Err(e) = tasks.set_task_status(&task.id, TaskStatus::Completed, None) {
                        return TaskRunOutcome::Failed(e);
                    }
                    if let Err(e) = tasks.clear_checkpoint(&task.id) {
                        return TaskRunOutcome::Failed(e);
                    }
                    cleanup_temp_dir(&config.media_dir, &task.id);
                    TaskRunOutcome::Success
                }
                Err(e) => TaskRunOutcome::Failed(e),
            }
        }
        Err(e) => {
            if cancel.is_cancelled() {
                let _ = save_interrupt_checkpoint_from_store(config, task, &tasks);
                TaskRunOutcome::Cancelled
            } else {
                classify_error(e)
            }
        }
    }
}

fn save_interrupt_checkpoint_from_store(
    config: &WorkerConfig,
    task: &DownloadTask,
    store: &TaskStore,
) -> Result<(), EngineError> {
    let current = store.get(&task.id)?;
    if current.status != TaskStatus::Paused {
        return Ok(());
    }
    save_interrupt_checkpoint(config, task)
}

fn save_interrupt_checkpoint(
    config: &WorkerConfig,
    task: &DownloadTask,
) -> Result<(), EngineError> {
    if is_hls_url(&task.source_url) {
        return Ok(());
    }

    let temp_dir = config.media_dir.join(".dl").join(&task.id);
    let output_path = config.media_dir.join(output_filename(task));
    let part = mp4_part_path(&temp_dir, &output_path);
    if !part.is_file() {
        return Ok(());
    }

    let bytes_done = std::fs::metadata(&part)?.len();
    if bytes_done == 0 {
        return Ok(());
    }

    let checkpoint = Checkpoint {
        version: 1,
        body: CheckpointBody::Mp4 {
            temp_dir: temp_dir.to_string_lossy().into_owned(),
            part_path: part.to_string_lossy().into_owned(),
            bytes_done,
        },
    };
    let mut store = TaskStore::open(&config.data_dir.join("tasks.db"))?;
    store.save_checkpoint(&task.id, &checkpoint)?;
    store.update_progress(&task.id, bytes_done, None, TaskStatus::Paused)?;
    Ok(())
}

fn classify_error(err: EngineError) -> TaskRunOutcome {
    if is_disk_full(&err) {
        TaskRunOutcome::DiskFull
    } else if err.to_string().contains("cancelled") {
        TaskRunOutcome::Cancelled
    } else {
        TaskRunOutcome::Failed(err)
    }
}

fn is_disk_full(err: &EngineError) -> bool {
    match err {
        EngineError::Io(e) => e.kind() == io::ErrorKind::StorageFull,
        _ => false,
    }
}

pub fn sanitize_filename(title: &str) -> String {
    let mut out: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.len() > 120 {
        out.truncate(120);
    }
    if out.is_empty() {
        "download".into()
    } else {
        out
    }
}

pub fn output_filename(task: &DownloadTask) -> String {
    let base = sanitize_filename(&task.title);
    if task.parent_id.is_some() {
        if let Some(index) = task.episode_index {
            if let Some(season) = task.season {
                return format!("{base}_S{season}E{index}.mp4");
            }
            return format!("{base}_E{index}.mp4");
        }
    }
    format!("{base}.mp4")
}

fn is_hls_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains(".m3u8") || lower.ends_with("m3u8")
}

fn cleanup_temp_dir(media_dir: &Path, task_id: &str) {
    let temp = media_dir.join(".dl").join(task_id);
    if temp.exists() {
        let _ = std::fs::remove_dir_all(&temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_truncates_and_replaces() {
        assert_eq!(sanitize_filename("Hello World!"), "Hello_World_");
        let long = "a".repeat(200);
        assert_eq!(sanitize_filename(&long).len(), 120);
    }

    #[test]
    fn output_filename_for_series_episode() {
        let task = DownloadTask {
            id: "c1".into(),
            parent_id: Some("p".into()),
            season: Some(1),
            title: "第1集".into(),
            source_url: "https://ex/1.m3u8".into(),
            quality_label: None,
            status: TaskStatus::Queued,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: Some(3),
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        assert_eq!(output_filename(&task), "第1集_S1E3.mp4");
    }
}
