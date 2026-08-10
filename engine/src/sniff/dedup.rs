use std::collections::HashMap;

use crate::types::{MediaKind, ResourceCandidate};

pub fn normalize_sniff_url(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    parsed.set_fragment(None);
    Some(parsed.as_str().to_ascii_lowercase())
}

pub fn dedup_candidates(candidates: Vec<ResourceCandidate>) -> Vec<ResourceCandidate> {
    let mut by_key: HashMap<String, ResourceCandidate> = HashMap::new();
    for candidate in candidates {
        let Some(key) = normalize_sniff_url(&candidate.url) else {
            continue;
        };
        match by_key.get(&key) {
            None => {
                by_key.insert(key, candidate);
            }
            Some(existing) if prefer_kind(&candidate.kind, &existing.kind) => {
                by_key.insert(key, candidate);
            }
            Some(_) => {}
        }
    }
    by_key.into_values().collect()
}

fn prefer_kind(new: &MediaKind, existing: &MediaKind) -> bool {
    matches!((new, existing), (MediaKind::Hls, MediaKind::Mp4))
}
