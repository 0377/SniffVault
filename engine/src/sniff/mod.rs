mod classify;
mod dedup;

pub use classify::classify_media_url;
pub use dedup::{dedup_candidates, normalize_sniff_url};

use crate::types::{ResourceCandidate, SniffEvent};

pub fn sniff_urls(events: &[SniffEvent], page_url: Option<&str>) -> Vec<ResourceCandidate> {
    let mut candidates = Vec::new();
    for event in events {
        if let Some(kind) = classify_media_url(&event.url) {
            candidates.push(ResourceCandidate {
                id: uuid::Uuid::new_v4().to_string(),
                url: event.url.clone(),
                title: None,
                kind,
                quality: None,
                page_url: event
                    .page_url
                    .clone()
                    .or_else(|| page_url.map(str::to_string)),
            });
        }
    }
    dedup_candidates(candidates)
}
