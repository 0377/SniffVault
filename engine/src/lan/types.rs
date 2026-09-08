use serde::{Deserialize, Serialize};

/// 投送元数据 — 禁止 source_url / cookie / referer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastMetadata {
    pub title: String,
    pub season: Option<u32>,
    pub episode_index: u32,
    pub episode_title: String,
    pub duration_ms: Option<i64>,
    pub position_ms: i64,
    pub mime: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastPlayRequest {
    pub session_id: String,
    pub sender_device_id: String,
    pub sender_name: String,
    pub metadata: CastMetadata,
    pub stream_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanPeer {
    pub device_id: String,
    pub device_name: String,
    pub host: String,
    pub port: u16,
    pub is_trusted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub peer_device_id: String,
    pub peer_name: String,
    pub peer_host: String,
    pub peer_port: u16,
    pub paired_at_ms: i64,
    pub session_secret: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CastEvent {
    IncomingPlay { request: CastPlayRequest },
    SessionEnded { session_id: String },
    Error { message: String },
}
