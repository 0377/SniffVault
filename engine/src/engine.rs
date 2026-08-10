use crate::error::EngineError;
use crate::library::LibraryStore;
use crate::settings;
use crate::tasks::TaskStore;
use crate::types::{
    DownloadTask, EngineSettings, LibraryEpisode, LibraryItem, LibraryItemKind, TaskStatus,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Engine {
    data_dir: PathBuf,
    settings: EngineSettings,
    settings_path: PathBuf,
    library: LibraryStore,
    tasks: TaskStore,
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

    fn ensure_path_in_media_dir(&self, file_path: &str) -> Result<PathBuf, EngineError> {
        let path = PathBuf::from(file_path);
        let media = self
            .media_dir()
            .canonicalize()
            .unwrap_or_else(|_| self.media_dir());
        let canon = path.canonicalize().map_err(|e| {
            EngineError::InvalidArg(format!("file path not accessible: {e}"))
        })?;
        if !canon.starts_with(&media) {
            return Err(EngineError::InvalidArg(
                "file path must be under media_dir".into(),
            ));
        }
        Ok(canon)
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
            return Err(EngineError::InvalidArg(
                "episodes must not be empty".into(),
            ));
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

    pub fn list_tasks(&self) -> Result<Vec<DownloadTask>, EngineError> {
        self.tasks.list_all()
    }

    pub fn list_library(&self) -> Result<Vec<LibraryItem>, EngineError> {
        self.library.list_items()
    }

    pub fn list_episodes(&self, item_id: &str) -> Result<Vec<LibraryEpisode>, EngineError> {
        self.library.list_episodes(item_id)
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
        let canon = self.ensure_path_in_media_dir(file_path)?;
        let existing = self
            .library
            .find_series_by_title_season(series_title, season)?;
        let item = if let Some(item) = existing {
            item
        } else {
            let item = LibraryItem {
                id: Uuid::new_v4().to_string(),
                kind: LibraryItemKind::Series,
                title: series_title.to_string(),
                season,
                poster_path: None,
                created_at_ms: Self::now_ms(),
            };
            self.library.upsert_item(&item)?;
            item
        };

        let episode_id = if let Some(prev) = self
            .library
            .get_episode_by_item_index(&item.id, episode_index)?
        {
            prev.id
        } else {
            Uuid::new_v4().to_string()
        };

        let episode = LibraryEpisode {
            id: episode_id,
            item_id: item.id.clone(),
            index: episode_index,
            title: episode_title.to_string(),
            file_path: canon.to_string_lossy().into(),
            duration_ms: None,
            position_ms: 0,
            source_url: source_url.map(|s| s.to_string()),
        };
        self.library.upsert_episode(&episode)?;
        let episode = self
            .library
            .get_episode_by_item_index(&item.id, episode_index)?
            .ok_or_else(|| EngineError::Message("episode missing after upsert".into()))?;
        Ok((item, episode))
    }

    pub fn register_completed_single(
        &mut self,
        title: &str,
        file_path: &str,
        source_url: Option<&str>,
    ) -> Result<(LibraryItem, LibraryEpisode), EngineError> {
        let canon = self.ensure_path_in_media_dir(file_path)?;
        let item = LibraryItem {
            id: Uuid::new_v4().to_string(),
            kind: LibraryItemKind::Single,
            title: title.to_string(),
            season: None,
            poster_path: None,
            created_at_ms: Self::now_ms(),
        };
        self.library.upsert_item(&item)?;
        let episode = LibraryEpisode {
            id: Uuid::new_v4().to_string(),
            item_id: item.id.clone(),
            index: 1,
            title: title.to_string(),
            file_path: canon.to_string_lossy().into(),
            duration_ms: None,
            position_ms: 0,
            source_url: source_url.map(|s| s.to_string()),
        };
        self.library.upsert_episode(&episode)?;
        Ok((item, episode))
    }
}
