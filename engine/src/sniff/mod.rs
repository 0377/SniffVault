mod classify;
mod dedup;

use crate::types::{ResourceCandidate, SniffEvent};

pub(crate) fn sniff_urls(events: &[SniffEvent], page_url: Option<&str>) -> Vec<ResourceCandidate> {
    let mut candidates = Vec::new();
    for event in events {
        if let Some(kind) = classify::classify_media_url(&event.url) {
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
    dedup::dedup_candidates(candidates)
}
