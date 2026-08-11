use crate::download::checkpoint::{Checkpoint, CheckpointBody};
use crate::download::ffmpeg::FfmpegLocator;
use crate::download::hls::{download_hls_to_mp4, HlsContext, HlsDownloadState};
use crate::download::http::HttpClient;
use crate::download::mp4::{download_mp4, mp4_part_path, Mp4Context};
use crate::download::scheduler::Scheduler;
use crate::error::EngineError;
use crate::ingest;
use crate::library::LibraryStore;
use crate::tasks::TaskStore;
use crate::types::{DownloadTask, TaskEvent, TaskEventKind, TaskStatus};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

type HlsStateCell = Arc<tokio::sync::Mutex<HlsDownloadState>>;
type HlsStatesMap = Arc<tokio::sync::Mutex<HashMap<String, HlsStateCell>>>;
type TaskCancelsMap = Arc<tokio::sync::Mutex<HashMap<String, CancellationToken>>>;
type PendingResumes = Arc<tokio::sync::Mutex<HashSet<String>>>;
type InFlightTasks = Arc<tokio::sync::Mutex<HashSet<String>>>;

struct WorkerHandles {
    task_cancels: TaskCancelsMap,
    pending_resumes: PendingResumes,
    in_flight: InFlightTasks,
    hls_states: HlsStatesMap,
}

pub struct WorkerConfig {
    pub data_dir: PathBuf,
    pub media_dir: PathBuf,
    pub max_concurrency: u32,
    pub user_agent: Option<String>,
    pub default_quality_label: Option<String>,
    pub ffmpeg: Arc<dyn FfmpegLocator>,
    pub task_event_tx: Option<mpsc::Sender<TaskEvent>>,
}

fn emit_task_event(config: &WorkerConfig, kind: TaskEventKind, task: Option<DownloadTask>) {
    if let Some(tx) = &config.task_event_tx {
        let _ = tx.send(TaskEvent { kind, task });
    }
}

fn emit_task_updated(config: &WorkerConfig, store: &TaskStore, id: &str) {
    if config.task_event_tx.is_some() {
        if let Ok(task) = store.get(id) {
            emit_task_event(config, TaskEventKind::TaskUpdated, Some(task));
        }
    }
}

fn worker_set_task_status(
    store: &TaskStore,
    config: &WorkerConfig,
    id: &str,
    status: TaskStatus,
    error_message: Option<&str>,
) -> Result<(), EngineError> {
    store.set_task_status(id, status, error_message)?;
    emit_task_updated(config, store, id);
    Ok(())
}

fn worker_update_progress(
    store: &TaskStore,
    config: &WorkerConfig,
    id: &str,
    progress_bytes: u64,
    total_bytes: Option<u64>,
    status: TaskStatus,
) -> Result<(), EngineError> {
    store.update_progress(id, progress_bytes, total_bytes, status)?;
    emit_task_updated(config, store, id);
    Ok(())
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
    let task_cancels: TaskCancelsMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let pending_resumes: PendingResumes = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let in_flight: InFlightTasks = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let hls_states: HlsStatesMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let mut shutdown_ack: Option<mpsc::Sender<()>> = None;
    let mut stopping = false;

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                DownloadCommand::Pause { task_id } => {
                    let tasks_path = config.data_dir.join("tasks.db");
                    if let Ok(store) = TaskStore::open(&tasks_path) {
                        if let Ok(task) = store.get(&task_id) {
                            if task.status == TaskStatus::Running {
                                let _ = worker_set_task_status(
                                    &store,
                                    &config,
                                    &task_id,
                                    TaskStatus::Paused,
                                    None,
                                );
                                if let Some(token) = task_cancels.lock().await.get(&task_id) {
                                    token.cancel();
                                }
                            }
                        }
                    }
                }
                DownloadCommand::Resume { task_id } => {
                    if in_flight.lock().await.contains(&task_id) {
                        pending_resumes.lock().await.insert(task_id);
                    } else {
                        let tasks_path = config.data_dir.join("tasks.db");
                        if let Ok(store) = TaskStore::open(&tasks_path) {
                            let _ = worker_set_task_status(
                                &store,
                                &config,
                                &task_id,
                                TaskStatus::Queued,
                                None,
                            );
                        }
                    }
                }
                DownloadCommand::Cancel { task_id } => {
                    if let Some(token) = task_cancels.lock().await.remove(&task_id) {
                        token.cancel();
                    }
                    let tasks_path = config.data_dir.join("tasks.db");
                    if let Ok(store) = TaskStore::open(&tasks_path) {
                        let _ = worker_set_task_status(
                            &store,
                            &config,
                            &task_id,
                            TaskStatus::Cancelled,
                            None,
                        );
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
                                    let _ = worker_set_task_status(
                                        &store,
                                        &config,
                                        &task.id,
                                        TaskStatus::Paused,
                                        None,
                                    );
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
            emit_task_event(&config, TaskEventKind::WorkerStopped, None);
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
                        if in_flight.lock().await.contains(&task.id) {
                            continue;
                        }
                        in_flight.lock().await.insert(task.id.clone());
                        let token = CancellationToken::new();
                        task_cancels
                            .lock()
                            .await
                            .insert(task.id.clone(), token.clone());

                        if let Err(e) =
                            worker_set_task_status(&store, &config, &task.id, TaskStatus::Running, None)
                        {
                            tracing_log(&format!("set running failed: {e}"));
                            task_cancels.lock().await.remove(&task.id);
                            in_flight.lock().await.remove(&task.id);
                            continue;
                        }
                        if let Some(parent_id) = &task.parent_id {
                            let _ = store.sync_parent_status(parent_id);
                        }

                        active.fetch_add(1, Ordering::SeqCst);
                        let cfg = config.clone();
                        let scheduler_ref = scheduler.clone();
                        let active_ref = active.clone();
                        let handles = WorkerHandles {
                            task_cancels: task_cancels.clone(),
                            pending_resumes: pending_resumes.clone(),
                            in_flight: in_flight.clone(),
                            hls_states: hls_states.clone(),
                        };

                        tokio::spawn(async move {
                            let outcome =
                                run_one_task(&cfg, &task, token, handles.hls_states.clone()).await;
                            handle_outcome(&cfg, &task, outcome, scheduler_ref, handles).await;
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
    handles: WorkerHandles,
) {
    let tasks_path = config.data_dir.join("tasks.db");
    let mut save_checkpoint = false;

    if let Ok(store) = TaskStore::open(&tasks_path) {
        match outcome {
            TaskRunOutcome::Success => {}
            TaskRunOutcome::Cancelled => {
                if let Ok(current) = store.get(&task.id) {
                    match current.status {
                        TaskStatus::Cancelled => {
                            cleanup_temp_dir(&config.media_dir, &task.id);
                        }
                        TaskStatus::Paused | TaskStatus::Queued => {
                            save_checkpoint = true;
                        }
                        _ => {
                            let _ = worker_set_task_status(
                                &store,
                                config,
                                &task.id,
                                TaskStatus::Cancelled,
                                None,
                            );
                            cleanup_temp_dir(&config.media_dir, &task.id);
                        }
                    }
                }
            }
            TaskRunOutcome::DiskFull => {
                scheduler.lock().await.pause_globally();
                let _ = worker_set_task_status(
                    &store,
                    config,
                    &task.id,
                    TaskStatus::Paused,
                    None,
                );
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

    if save_checkpoint {
        let _ = save_interrupt_checkpoint(config, task, &handles.hls_states).await;
    }

    finish_task_handles(config, &task.id, &handles).await;
}

async fn finish_task_handles(config: &WorkerConfig, task_id: &str, handles: &WorkerHandles) {
    handles.task_cancels.lock().await.remove(task_id);
    handles.in_flight.lock().await.remove(task_id);
    handles.hls_states.lock().await.remove(task_id);
    if handles.pending_resumes.lock().await.remove(task_id) {
        let tasks_path = config.data_dir.join("tasks.db");
        if let Ok(store) = TaskStore::open(&tasks_path) {
            let _ = worker_set_task_status(&store, config, task_id, TaskStatus::Queued, None);
        }
    }
}

async fn run_one_task(
    config: &WorkerConfig,
    task: &DownloadTask,
    cancel: CancellationToken,
    hls_states: HlsStatesMap,
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
        let hls_state = Arc::new(tokio::sync::Mutex::new(HlsDownloadState::default()));
        hls_states
            .lock()
            .await
            .insert(task.id.clone(), hls_state.clone());
        let ctx = HlsContext {
            http: &http,
            temp_dir: &temp_dir,
            ffmpeg: &ffmpeg,
        };
        download_hls_to_mp4(
            &ctx,
            &task.source_url,
            &output_path,
            quality,
            checkpoint,
            Some(hls_state),
        )
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
        let _ = save_interrupt_checkpoint_if_paused(config, task, &hls_states).await;
        return TaskRunOutcome::Cancelled;
    }

    match download_result {
        Ok((final_path, _bytes)) => {
            let current = match tasks.get(&task.id) {
                Ok(t) => t,
                Err(e) => return TaskRunOutcome::Failed(e),
            };
            if current.status == TaskStatus::Paused {
                let _ = save_interrupt_checkpoint(config, task, &hls_states).await;
                return TaskRunOutcome::Cancelled;
            }
            if current.status != TaskStatus::Running {
                return TaskRunOutcome::Cancelled;
            }

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
                    if let Err(e) = tasks.complete_download(&task.id, &path_str, &item.id) {
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
                let _ = save_interrupt_checkpoint_if_paused(config, task, &hls_states).await;
                TaskRunOutcome::Cancelled
            } else {
                classify_error(e)
            }
        }
    }
}

async fn save_interrupt_checkpoint_if_paused(
    config: &WorkerConfig,
    task: &DownloadTask,
    hls_states: &HlsStatesMap,
) -> Result<(), EngineError> {
    let tasks_path = config.data_dir.join("tasks.db");
    let should_save = TaskStore::open(&tasks_path)?
        .get(&task.id)
        .map(|t| matches!(t.status, TaskStatus::Paused | TaskStatus::Queued))
        .unwrap_or(false);
    if should_save {
        save_interrupt_checkpoint(config, task, hls_states).await?;
    }
    Ok(())
}

async fn save_interrupt_checkpoint(
    config: &WorkerConfig,
    task: &DownloadTask,
    hls_states: &HlsStatesMap,
) -> Result<(), EngineError> {
    let mut store = TaskStore::open(&config.data_dir.join("tasks.db"))?;

    if is_hls_url(&task.source_url) {
        let state = hls_states.lock().await.get(&task.id).cloned();
        if let Some(state) = state {
            let snapshot = state.lock().await;
            if let Some(checkpoint) = snapshot.to_checkpoint() {
                let progress = snapshot.segments_done.len() as u64;
                store.save_checkpoint(&task.id, &checkpoint)?;
                worker_update_progress(&store, config, &task.id, progress, None, TaskStatus::Paused)?;
            }
        }
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
    store.save_checkpoint(&task.id, &checkpoint)?;
    worker_update_progress(
        &store,
        config,
        &task.id,
        bytes_done,
        None,
        TaskStatus::Paused,
    )?;
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

pub(crate) fn cleanup_download_temp(media_dir: &Path, task_id: &str) {
    cleanup_temp_dir(media_dir, task_id);
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
    fn classify_error_maps_storage_full_and_cancelled() {
        let disk_full = EngineError::Io(io::Error::new(io::ErrorKind::StorageFull, "full"));
        assert!(matches!(
            classify_error(disk_full),
            TaskRunOutcome::DiskFull
        ));

        let cancelled = EngineError::Message("download cancelled".into());
        assert!(matches!(
            classify_error(cancelled),
            TaskRunOutcome::Cancelled
        ));
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
