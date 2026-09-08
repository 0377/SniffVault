use crate::ingest;
use crate::types::LibraryEpisode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const LAN_STREAM_TOKEN_TTL_SECS: i64 = 4 * 60 * 60;

struct TokenRecord {
    episode_id: String,
    expires_at_secs: i64,
}

pub struct StreamTokenStore {
    by_token: HashMap<String, TokenRecord>,
    by_episode: HashMap<String, String>,
}

impl StreamTokenStore {
    pub fn new() -> Self {
        Self {
            by_token: HashMap::new(),
            by_episode: HashMap::new(),
        }
    }

    pub fn issue(&mut self, episode_id: &str, now_secs: i64) -> String {
        if let Some(old) = self.by_episode.remove(episode_id) {
            self.by_token.remove(&old);
        }
        let token = Uuid::new_v4().to_string();
        let expires_at_secs = now_secs + LAN_STREAM_TOKEN_TTL_SECS;
        self.by_token.insert(
            token.clone(),
            TokenRecord {
                episode_id: episode_id.to_string(),
                expires_at_secs,
            },
        );
        self.by_episode
            .insert(episode_id.to_string(), token.clone());
        token
    }

    pub fn revoke(&mut self, token: &str) {
        if let Some(rec) = self.by_token.remove(token) {
            self.by_episode.remove(&rec.episode_id);
        }
    }

    pub fn resolve_path<F>(
        &self,
        token: &str,
        now_secs: i64,
        media_dir: &Path,
        get_episode: F,
    ) -> Option<PathBuf>
    where
        F: FnOnce(&str) -> Option<LibraryEpisode>,
    {
        let rec = self.by_token.get(token)?;
        if now_secs >= rec.expires_at_secs {
            return None;
        }
        let episode = get_episode(&rec.episode_id)?;
        if episode.id != rec.episode_id {
            return None;
        }
        ingest::ensure_path_in_media_dir(media_dir, &episode.file_path).ok()
    }
}

impl Default for StreamTokenStore {
    fn default() -> Self {
        Self::new()
    }
}
