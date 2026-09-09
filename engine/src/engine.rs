use crate::download::runtime::{worker_config, DownloadRuntime};
use crate::download::worker::DownloadCommand;
use crate::error::EngineError;
use crate::ingest;
use crate::lan::{sanitize_episode, GetEpisodeFn, LanService, LanTestConfig};
use crate::lan::{CastEvent, LanPeer, TrustedPeer};
use crate::library::LibraryStore;
use crate::settings;
use crate::tasks::TaskStore;
use crate::types::{
    DownloadAuth, DownloadTask, EngineSettings, LibraryEpisode, LibraryItem, Quality,
    ResolveOptions, ResolveOutcome, ResourceCandidate, SniffEvent, TaskEvent, TaskStatus,
};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Engine {
    data_dir: PathBuf,
    settings: EngineSettings,
    settings_path: PathBuf,
    library: LibraryStore,
    tasks: TaskStore,
    lan: Option<LanService>,
    download: Option<DownloadRuntime>,
    task_event_rx: Option<mpsc::Receiver<TaskEvent>>,
    pending_task_event_tx: Option<mpsc::Sender<TaskEvent>>,
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
            lan: None,
            download: None,
            task_event_rx: None,
            pending_task_event_tx: None,
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

    pub fn save_settings(&mut self, mut settings: EngineSettings) -> Result<(), EngineError> {
        settings::validate_media_dir(&settings.media_dir)?;
        if settings.device_id.is_empty() {
            if !self.settings.device_id.is_empty() {
                settings.device_id = self.settings.device_id.clone();
            } else {
                settings::ensure_device_id(&mut settings);
            }
        }
        std::fs::create_dir_all(self.data_dir.join(&settings.media_dir))?;
        settings::save(&self.settings_path, &settings)?;
        self.settings = settings;
        Ok(())
    }

    pub async fn resolve_url(
        &self,
        url: &str,
        opts: ResolveOptions,
    ) -> Result<ResolveOutcome, EngineError> {
        let http = crate::download::http::HttpClient::new(self.settings.user_agent.as_deref())?;
        crate::resolve::resolve_url(&http, url, opts).await
    }

    pub async fn resolve_qualities(
        &self,
        media_url: &str,
        opts: ResolveOptions,
    ) -> Result<Vec<Quality>, EngineError> {
        let http = crate::download::http::HttpClient::new(self.settings.user_agent.as_deref())?;
        crate::resolve::resolve_qualities(&http, media_url, opts).await
    }

    pub fn sniff_urls(
        &self,
        events: &[SniffEvent],
        page_url: Option<&str>,
    ) -> Vec<ResourceCandidate> {
        crate::sniff::sniff_urls(events, page_url)
    }

    pub fn enqueue_episodes(
        &mut self,
        list_title: &str,
        season: Option<u32>,
        episodes: &[(u32, String, String)],
        quality_label: Option<&str>,
        auth: Option<&DownloadAuth>,
    ) -> Result<(String, Vec<String>), EngineError> {
        if episodes.is_empty() {
            return Err(EngineError::InvalidArg("episodes must not be empty".into()));
        }
        let now = Self::now_ms();
        let parent_id = Uuid::new_v4().to_string();
        let cookie_header = auth.and_then(|a| a.cookies.clone());
        let referer = auth.and_then(|a| a.referer.clone());
        let mut child_ids = Vec::new();
        let mut child_tasks = Vec::new();
        for (index, title, url) in episodes {
            let id = Uuid::new_v4().to_string();
            child_ids.push(id.clone());
            child_tasks.push(DownloadTask {
                id,
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
                cookie_header: cookie_header.clone(),
                referer: referer.clone(),
                resolved_media_url: None,
            });
        }
        self.tasks.upsert_parent_with_children(
            &DownloadTask {
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
                cookie_header,
                referer,
                resolved_media_url: None,
            },
            &child_tasks,
        )?;
        Ok((parent_id, child_ids))
    }

    pub fn enqueue_single(
        &mut self,
        title: &str,
        url: &str,
        quality_label: Option<&str>,
        auth: Option<&DownloadAuth>,
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
            cookie_header: auth.and_then(|a| a.cookies.clone()),
            referer: auth.and_then(|a| a.referer.clone()),
            resolved_media_url: None,
        })?;
        Ok(id)
    }

    pub fn prepare_download_events(&mut self) -> Result<(), EngineError> {
        if self.download.is_some() || self.pending_task_event_tx.is_some() {
            return Err(EngineError::InvalidArg("downloads already running".into()));
        }
        for task in self.tasks.list_all()? {
            if task.status == TaskStatus::Paused {
                self.tasks
                    .set_task_status(&task.id, TaskStatus::Queued, None)?;
            }
        }
        let (task_event_tx, task_event_rx) = mpsc::channel();
        self.pending_task_event_tx = Some(task_event_tx);
        self.task_event_rx = Some(task_event_rx);
        Ok(())
    }

    pub fn spawn_download_worker(&mut self) -> Result<(), EngineError> {
        if self.download.is_some() {
            return Err(EngineError::InvalidArg("downloads already running".into()));
        }
        let task_event_tx = self
            .pending_task_event_tx
            .take()
            .ok_or_else(|| EngineError::InvalidArg("download events not prepared".into()))?;
        let config = worker_config(
            self.data_dir.clone(),
            self.media_dir(),
            self.settings.max_concurrency,
            self.settings.user_agent.clone(),
            self.settings.default_quality_label.clone(),
            Some(task_event_tx),
        );
        self.download = Some(DownloadRuntime::spawn(config));
        Ok(())
    }

    pub fn start_downloads(&mut self) -> Result<(), EngineError> {
        self.prepare_download_events()?;
        self.spawn_download_worker()?;
        Ok(())
    }

    pub fn take_task_event_receiver(&mut self) -> Option<mpsc::Receiver<TaskEvent>> {
        self.task_event_rx.take()
    }

    pub fn stop_downloads(&mut self) -> Result<(), EngineError> {
        self.pending_task_event_tx = None;
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

    pub fn set_task_media_url(
        &mut self,
        task_id: &str,
        media_url: &str,
    ) -> Result<(), EngineError> {
        if media_url.is_empty() {
            return Err(EngineError::InvalidArg(
                "media_url must not be empty".into(),
            ));
        }
        let task = self.tasks.get(task_id)?;
        let allowed = task.status == TaskStatus::NeedsSniff
            || (task.status == TaskStatus::Failed
                && task.error_message.as_deref() == Some("needs_sniff"));
        if !allowed {
            return Err(EngineError::InvalidArg(
                "task must be in needs_sniff status".into(),
            ));
        }
        if crate::resolve::source_is_web_page(media_url) {
            return Err(EngineError::InvalidArg(
                "media_url must be a direct media link".into(),
            ));
        }
        self.tasks.set_resolved_media_url(task_id, media_url)?;
        self.tasks
            .set_task_status(task_id, TaskStatus::Queued, None)?;
        if let Some(parent_id) = &task.parent_id {
            let _ = self.tasks.sync_parent_status(parent_id);
        }
        Ok(())
    }

    pub fn resume_task(&mut self, task_id: &str) -> Result<(), EngineError> {
        let task = self.tasks.get(task_id)?;
        if task.status == TaskStatus::NeedsSniff {
            return Err(EngineError::InvalidArg(
                "cannot resume task in needs_sniff status".into(),
            ));
        }
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

    pub fn get_episode(&self, episode_id: &str) -> Result<Option<LibraryEpisode>, EngineError> {
        self.library.get_episode(episode_id)
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

    fn finalize_lan_for_episodes(&mut self, episode_ids: &[String]) -> Result<(), EngineError> {
        if self.lan.is_none() {
            return Ok(());
        }
        let lan = self.ensure_lan()?;
        for id in episode_ids {
            lan.finalize_episode_removal(id)?;
        }
        Ok(())
    }

    pub fn remove_library_item(
        &mut self,
        item_id: &str,
        delete_files: bool,
    ) -> Result<(), EngineError> {
        let item = self.library.get_item(item_id)?;
        let episodes = self.library.list_episodes(item_id)?;
        let episode_ids: Vec<String> = episodes.iter().map(|e| e.id.clone()).collect();
        self.finalize_lan_for_episodes(&episode_ids)?;
        if delete_files {
            let paths = crate::library::delete::collect_deletion_paths(
                &self.library,
                &item,
                &self.media_dir(),
            )?;
            crate::library::delete::delete_files(&paths)?;
        }
        crate::library::delete::remove_item_record(&self.library, item_id)?;
        Ok(())
    }

    pub fn remove_episode(
        &mut self,
        episode_id: &str,
        delete_files: bool,
    ) -> Result<(), EngineError> {
        let ep = self
            .library
            .get_episode(episode_id)?
            .ok_or_else(|| EngineError::NotFound(format!("episode {episode_id}")))?;
        if self.library.count_episodes(&ep.item_id)? == 1 {
            return self.remove_library_item(&ep.item_id, delete_files);
        }
        self.finalize_lan_for_episodes(&[episode_id.to_string()])?;
        if delete_files {
            let path =
                crate::library::delete::resolve_deletion_path(&self.media_dir(), &ep.file_path)?;
            crate::library::delete::delete_files(&[path])?;
        }
        self.library.remove_episode(episode_id)?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn has_active_cast(&self) -> bool {
        self.lan
            .as_ref()
            .map(|lan| lan.has_active_cast())
            .unwrap_or(false)
    }

    #[doc(hidden)]
    pub fn set_lan_test_config(&mut self, config: LanTestConfig) {
        if let Some(lan) = &mut self.lan {
            lan.set_test_config(config);
        } else {
            let mut lan = LanService::open(&self.data_dir.join("lan.db"))
                .expect("lan.db open for test config");
            lan.set_test_config(config);
            self.lan = Some(lan);
        }
    }

    fn ensure_lan(&mut self) -> Result<&mut LanService, EngineError> {
        if self.lan.is_none() {
            let mut lan = LanService::open(&self.data_dir.join("lan.db"))?;
            lan.set_device_identity(&self.settings.device_id, &self.settings.device_name);
            self.lan = Some(lan);
        }
        Ok(self.lan.as_mut().expect("lan initialized"))
    }

    pub fn lan_http_port(&self) -> Option<u16> {
        self.lan.as_ref().and_then(|lan| lan.lan_http_port())
    }

    pub fn start_lan(&mut self, is_receiver: bool) -> Result<(), EngineError> {
        let device_id = self.settings.device_id.clone();
        let device_name = self.settings.device_name.clone();
        let media_dir = self.media_dir();
        let get_episode = if is_receiver {
            None
        } else {
            Some(self.sender_get_episode_fn())
        };
        let lan = self.ensure_lan()?;
        lan.set_device_identity(&device_id, &device_name);
        lan.start(
            is_receiver,
            &device_id,
            &device_name,
            &media_dir,
            get_episode,
        )
    }

    pub fn stop_lan(&mut self) -> Result<(), EngineError> {
        match self.lan.as_mut() {
            Some(lan) => lan.stop(),
            None => Ok(()),
        }
    }

    pub fn apply_lan_settings(&mut self, is_receiver: bool) -> Result<(), EngineError> {
        if self.settings.lan_enabled {
            self.start_lan(is_receiver)?;
            if is_receiver {
                self.begin_pairing()?;
            }
        } else {
            self.stop_lan()?;
        }
        Ok(())
    }

    pub fn discover_peers(&mut self) -> Result<Vec<LanPeer>, EngineError> {
        self.ensure_lan()?.discover_peers()
    }

    pub fn begin_pairing(&mut self) -> Result<String, EngineError> {
        self.ensure_lan()?.begin_pairing()
    }

    pub fn pairing_pin(&self) -> Result<Option<String>, EngineError> {
        match self.lan.as_ref() {
            Some(lan) => lan.pairing_pin(),
            None => Ok(None),
        }
    }

    pub fn pair_peer(&mut self, host: &str, port: u16, pin: &str) -> Result<(), EngineError> {
        if self.lan_http_port().is_none() {
            self.start_lan(false)?;
        }
        let device_id = self.settings.device_id.clone();
        let device_name = self.settings.device_name.clone();
        self.ensure_lan()?
            .pair_peer(host, port, pin, &device_id, &device_name)
    }

    pub fn list_trusted_peers(&self) -> Result<Vec<TrustedPeer>, EngineError> {
        match self.lan.as_ref() {
            Some(lan) => lan.list_trusted_peers(),
            None => Ok(Vec::new()),
        }
    }

    pub fn remove_trusted_peer(&self, peer_device_id: &str) -> Result<bool, EngineError> {
        match self.lan.as_ref() {
            Some(lan) => lan.remove_trusted_peer(peer_device_id),
            None => Ok(false),
        }
    }

    pub fn cast_episode(
        &mut self,
        episode_id: &str,
        peer_device_id: &str,
    ) -> Result<(), EngineError> {
        if self.lan_http_port().is_none() {
            self.start_lan(false)?;
        }

        let episode = self
            .library
            .get_episode(episode_id)?
            .ok_or_else(|| EngineError::NotFound(format!("episode {episode_id}")))?;
        ingest::ensure_path_in_media_dir(&self.media_dir(), &episode.file_path)?;

        let item = self.library.get_item(&episode.item_id)?;
        let metadata = sanitize_episode(&item, &episode);
        let device_id = self.settings.device_id.clone();
        let device_name = self.settings.device_name.clone();

        self.ensure_lan()?.cast_episode(
            episode_id,
            peer_device_id,
            metadata,
            &device_id,
            &device_name,
        )
    }

    pub fn stop_cast(&mut self) -> Result<(), EngineError> {
        match self.lan.as_mut() {
            Some(lan) => lan.stop_cast(),
            None => Ok(()),
        }
    }

    pub fn take_cast_event_receiver(&mut self) -> Option<tokio::sync::mpsc::Receiver<CastEvent>> {
        self.lan
            .as_mut()
            .and_then(|lan| lan.take_cast_event_receiver())
    }

    pub fn drain_cast_event(&mut self) -> Result<CastEvent, EngineError> {
        self.ensure_lan()?.drain_cast_event()
    }

    fn sender_get_episode_fn(&self) -> GetEpisodeFn {
        let data_dir = self.data_dir.clone();
        Arc::new(move |episode_id| {
            let library = LibraryStore::open(&data_dir.join("library.db")).ok()?;
            library.get_episode(episode_id).ok().flatten()
        })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(lan) = self.lan.as_mut() {
            let _ = lan.stop_cast();
            let _ = lan.stop();
        }
        let _ = self.stop_downloads();
    }
}
