use crate::download::runtime::{worker_config, DownloadRuntime};
use crate::download::worker::DownloadCommand;
use crate::error::EngineError;
use crate::ingest;
use crate::library::LibraryStore;
use crate::settings;
use crate::tasks::TaskStore;
use crate::types::{DownloadTask, EngineSettings, LibraryEpisode, LibraryItem, TaskStatus};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Engine {
    data_dir: PathBuf,
    settings: EngineSettings,
    settings_path: PathBuf,
    library: LibraryStore,
    tasks: TaskStore,
    download: Option<DownloadRuntime>,
}

fn absolute_data_dir(path: &Path) -> Result<PathBuf, EngineError> {
    if path.is_absolute() {
        if path.exists() {
            return Ok(path.canonicalize()?);
        }
        return Ok(path.to_path_buf());
    }
    let abs = std::env::current_dir()?.join(path);
    if abs.exists() {
        Ok(abs.canonicalize()?)
    } else {
        Ok(abs)
    }
}

impl Engine {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self, EngineError> {
        let data_dir = absolute_data_dir(data_dir.as_ref())?;
        let settings_path = data_dir.join("settings.json");
        let settings = settings::load_or_default(&settings_path)?;
        std::fs::create_dir_all(data_dir.join(&settings.media_dir))?;
        let library = LibraryStore::open(&data_dir.join("library.db"))?;
        let tasks = TaskStore::open(&data_dir.join("tasks.db"))?;
        Ok(Self {
            data_dir,
            settings,
            settings_path,
            library,
            tasks,
            download: None,
        })
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    pub fn media_dir(&self) -> PathBuf {
        self.data_dir.join(&self.settings.media_dir)
    }

    pub fn settings(&self) -> EngineSettings {
        self.settings.clone()
    }

    pub fn save_settings(&mut self, settings: EngineSettings) -> Result<(), EngineError> {
        settings::validate_media_dir(&settings.media_dir)?;
        std::fs::create_dir_all(self.data_dir.join(&settings.media_dir))?;
        settings::save(&self.settings_path, &settings)?;
        self.settings = settings;
        Ok(())
    }

    pub fn enqueue_episodes(
        &mut self,
        list_title: &str,
        season: Option<u32>,
        episodes: &[(u32, String, String)],
        quality_label: Option<&str>,
    ) -> Result<(String, Vec<String>), EngineError> {
        if episodes.is_empty() {
            return Err(EngineError::InvalidArg("episodes must not be empty".into()));
        }
        let now = Self::now_ms();
        let parent_id = Uuid::new_v4().to_string();
        self.tasks.upsert(&DownloadTask {
            id: parent_id.clone(),
            parent_id: None,
            season,
            title: list_title.to_string(),
            source_url: String::new(),
            quality_label: quality_label.map(|s| s.to_string()),
            status: TaskStatus::Queued,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: now,
            updated_at_ms: now,
        })?;

        let mut child_ids = Vec::new();
        for (index, title, url) in episodes {
            let id = Uuid::new_v4().to_string();
            self.tasks.upsert(&DownloadTask {
                id: id.clone(),
                parent_id: Some(parent_id.clone()),
                season,
                title: title.clone(),
                source_url: url.clone(),
                quality_label: quality_label.map(|s| s.to_string()),
                status: TaskStatus::Queued,
                progress_bytes: 0,
                total_bytes: None,
                error_message: None,
                output_path: None,
                library_item_id: None,
                episode_index: Some(*index),
                created_at_ms: now,
                updated_at_ms: now,
            })?;
            child_ids.push(id);
        }
        Ok((parent_id, child_ids))
    }

    pub fn enqueue_single(
        &mut self,
        title: &str,
        url: &str,
        quality_label: Option<&str>,
    ) -> Result<String, EngineError> {
        if url.is_empty() {
            return Err(EngineError::InvalidArg("url must not be empty".into()));
        }
        let now = Self::now_ms();
        let id = Uuid::new_v4().to_string();
        self.tasks.upsert(&DownloadTask {
            id: id.clone(),
            parent_id: None,
            season: None,
            title: title.to_string(),
            source_url: url.to_string(),
            quality_label: quality_label.map(|s| s.to_string()),
            status: TaskStatus::Queued,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: now,
            updated_at_ms: now,
        })?;
        Ok(id)
    }

    pub fn start_downloads(&mut self) -> Result<(), EngineError> {
        if self.download.is_some() {
            return Err(EngineError::InvalidArg("downloads already running".into()));
        }
        for task in self.tasks.list_all()? {
            if task.status == TaskStatus::Paused {
                self.tasks
                    .set_task_status(&task.id, TaskStatus::Queued, None)?;
            }
        }
        let config = worker_config(
            self.data_dir.clone(),
            self.media_dir(),
            self.settings.max_concurrency,
            self.settings.user_agent.clone(),
            self.settings.default_quality_label.clone(),
        );
        self.download = Some(DownloadRuntime::spawn(config));
        Ok(())
    }

    pub fn stop_downloads(&mut self) -> Result<(), EngineError> {
        if let Some(runtime) = self.download.take() {
            runtime.stop_and_join()?;
        }
        Ok(())
    }

    pub fn pause_task(&mut self, task_id: &str) -> Result<(), EngineError> {
        if let Some(runtime) = &self.download {
            runtime.send_command(DownloadCommand::Pause {
                task_id: task_id.to_string(),
            })?;
        } else {
            self.tasks
                .set_task_status(task_id, TaskStatus::Paused, None)?;
        }
        Ok(())
    }

    pub fn resume_task(&mut self, task_id: &str) -> Result<(), EngineError> {
        if let Some(runtime) = &self.download {
            runtime.send_command(DownloadCommand::Resume {
                task_id: task_id.to_string(),
            })?;
        } else {
            self.tasks
                .set_task_status(task_id, TaskStatus::Queued, None)?;
        }
        Ok(())
    }

    pub fn cancel_task(&mut self, task_id: &str) -> Result<(), EngineError> {
        if let Some(runtime) = &self.download {
            runtime.send_command(DownloadCommand::Cancel {
                task_id: task_id.to_string(),
            })?;
        } else {
            self.tasks
                .set_task_status(task_id, TaskStatus::Cancelled, None)?;
            crate::download::worker::cleanup_download_temp(&self.media_dir(), task_id);
        }
        Ok(())
    }

    /// 阻塞直到无 Running/Queued 任务或超时（集成测试专用）。
    #[doc(hidden)]
    pub fn drain_downloads_for_test(&self, timeout: Duration) -> Result<(), EngineError> {
        let started = std::time::Instant::now();
        loop {
            let running = self.tasks.count_by_status(TaskStatus::Running)?;
            let queued = self.tasks.count_by_status(TaskStatus::Queued)?;
            if running == 0 && queued == 0 {
                return Ok(());
            }
            if started.elapsed() > timeout {
                return Err(EngineError::Message(
                    "drain_downloads_for_test timed out waiting for downloads to finish".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn list_tasks(&self) -> Result<Vec<DownloadTask>, EngineError> {
        self.tasks.list_all()
    }

    pub fn list_library(&self) -> Result<Vec<LibraryItem>, EngineError> {
        self.library.list_items()
    }

    pub fn list_episodes(&self, item_id: &str) -> Result<Vec<LibraryEpisode>, EngineError> {
        self.library.list_episodes(item_id)
    }

    pub fn set_episode_position(
        &self,
        episode_id: &str,
        position_ms: i64,
    ) -> Result<(), EngineError> {
        self.library.set_position(episode_id, position_ms)
    }

    pub fn register_completed_episode(
        &mut self,
        series_title: &str,
        season: Option<u32>,
        episode_index: u32,
        episode_title: &str,
        file_path: &str,
        source_url: Option<&str>,
    ) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
        ingest::register_completed_episode(
            &self.library,
            &self.media_dir(),
            series_title,
            season,
            episode_index,
            episode_title,
            file_path,
            source_url,
        )
    }

    pub fn register_completed_single(
        &mut self,
        title: &str,
        file_path: &str,
        source_url: Option<&str>,
    ) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
        ingest::register_completed_single(
            &self.library,
            &self.media_dir(),
            title,
            file_path,
            source_url,
        )
    }
}
