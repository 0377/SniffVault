use crate::download::hls::playlist::{list_master_variants, parse_media_playlist};
use crate::error::EngineError;
use crate::types::{MediaKind, Quality, ResourceCandidate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum EntryKind {
    DirectMp4,
    M3u8,
    WebPage,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn classify_entry_url(url: &str) -> EntryKind {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".m3u8") {
        return EntryKind::M3u8;
    }
    if lower.contains(".mp4") {
        return EntryKind::DirectMp4;
    }
    EntryKind::WebPage
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum ResolveMediaResult {
    Single(ResourceCandidate),
    Candidates(Vec<ResourceCandidate>),
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn candidates_from_m3u8_body(
    body: &str,
    base_url: &str,
    page_url: Option<&str>,
) -> Result<ResolveMediaResult, EngineError> {
    if let Ok(variants) = list_master_variants(body, base_url) {
        if variants.len() == 1 {
            let (url, quality) = variants[0].clone();
            return Ok(ResolveMediaResult::Single(make_candidate(
                url,
                MediaKind::Hls,
                Some(quality),
                page_url,
            )));
        }
        let candidates = variants
            .into_iter()
            .map(|(url, quality)| make_candidate(url, MediaKind::Hls, Some(quality), page_url))
            .collect();
        return Ok(ResolveMediaResult::Candidates(candidates));
    }

    if parse_media_playlist(body, base_url).is_ok() {
        return Ok(ResolveMediaResult::Single(make_candidate(
            base_url.to_string(),
            MediaKind::Hls,
            None,
            page_url,
        )));
    }

    Err(EngineError::InvalidArg("unrecognized m3u8 playlist".into()))
}

pub(crate) fn make_candidate(
    url: String,
    kind: MediaKind,
    quality: Option<Quality>,
    page_url: Option<&str>,
) -> ResourceCandidate {
    ResourceCandidate {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        title: None,
        kind,
        quality,
        page_url: page_url.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_playlist_body_is_single_hls() {
        let body = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/hls/media.m3u8"),
        )
        .unwrap();
        let result =
            candidates_from_m3u8_body(&body, "http://127.0.0.1/hls/media.m3u8", None).unwrap();
        assert!(matches!(result, ResolveMediaResult::Single(_)));
    }
}
