use std::collections::HashMap;

use crate::types::{MediaKind, ResourceCandidate};

pub(crate) fn normalize_sniff_url(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    parsed.set_fragment(None);
    Some(parsed.as_str().to_ascii_lowercase())
}

pub(crate) fn dedup_candidates(candidates: Vec<ResourceCandidate>) -> Vec<ResourceCandidate> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ResourceCandidate;

    #[test]
    fn dedup_removes_fragment_and_prefers_hls() {
        let a = ResourceCandidate {
            id: "1".into(),
            url: "https://X/Stream.m3u8#t=1".into(),
            title: None,
            kind: MediaKind::Mp4,
            quality: None,
            page_url: None,
        };
        let b = ResourceCandidate {
            id: "2".into(),
            url: "https://x/stream.m3u8".into(),
            title: None,
            kind: MediaKind::Hls,
            quality: None,
            page_url: None,
        };
        let out = dedup_candidates(vec![a, b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, MediaKind::Hls);
    }
}
