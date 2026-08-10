use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Mp4,
    Hls,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quality {
    pub label: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bandwidth: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceCandidate {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub kind: MediaKind,
    pub quality: Option<Quality>,
    pub page_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub index: u32,
    pub title: String,
    pub url: String,
    pub quality_options: Vec<Quality>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodeList {
    pub title: String,
    pub season: Option<u32>,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryItemKind {
    Single,
    Series,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryItem {
    pub id: String,
    pub kind: LibraryItemKind,
    pub title: String,
    pub season: Option<u32>,
    pub poster_path: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryEpisode {
    pub id: String,
    pub item_id: String,
    pub index: u32,
    pub title: String,
    pub file_path: String,
    pub duration_ms: Option<i64>,
    pub position_ms: i64,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub parent_id: Option<String>,
    /// 剧集标题侧的季号；父任务必填（若有季），子任务与父任务相同，供完成后入库合并。
    pub season: Option<u32>,
    pub title: String,
    pub source_url: String,
    pub quality_label: Option<String>,
    pub status: TaskStatus,
    pub progress_bytes: u64,
    pub total_bytes: Option<u64>,
    pub error_message: Option<String>,
    pub output_path: Option<String>,
    pub library_item_id: Option<String>,
    pub episode_index: Option<u32>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 设置。LAN 信任设备列表留到 Plan 7，本期不预留半截字段。
/// `media_dir`：相对 `data_dir` 的子目录名，默认 `"media"`；`Engine::open` 必须按此创建目录。
/// `default_quality_label`：`"highest"` 表示选最高可用清晰度；具体如 `"1080p"` 则精确匹配 label。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSettings {
    pub media_dir: String,
    pub max_concurrency: u32,
    pub default_quality_label: Option<String>,
    pub user_agent: Option<String>,
    pub device_name: String,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            media_dir: "media".into(),
            max_concurrency: 2,
            default_quality_label: Some("highest".into()),
            user_agent: None,
            device_name: "VideoSniffing".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SniffInitiator {
    Navigation,
    SubResource,
    Media,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SniffEvent {
    pub url: String,
    pub page_url: Option<String>,
    pub initiator: SniffInitiator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResolveOptions {
    pub cookies: Option<String>,
    pub referer: Option<String>,
    pub page_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveOutcome {
    Single(ResourceCandidate),
    Candidates(Vec<ResourceCandidate>),
    EpisodeList(EpisodeList),
    NeedsBrowser { reason: String },
}
