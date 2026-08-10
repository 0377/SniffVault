pub mod html;
pub mod media;

mod fetch;

use reqwest::StatusCode;

use crate::download::hls::playlist::list_master_variants;
use crate::download::http::HttpClient;
use crate::error::EngineError;
use crate::types::{MediaKind, Quality, ResolveOptions, ResolveOutcome};

use html::{extract_episode_list, scan_media_urls};
use media::{
    classify_entry_url, candidates_from_m3u8_body, make_candidate, EntryKind, ResolveMediaResult,
};

pub async fn resolve_url(
    http: &HttpClient,
    url: &str,
    opts: ResolveOptions,
) -> Result<ResolveOutcome, EngineError> {
    let page_url = opts.page_url.clone().or_else(|| Some(url.to_string()));
    let page_url_ref = page_url.as_deref();
    let default_title = page_url_ref.unwrap_or(url);

    match classify_entry_url(url) {
        EntryKind::DirectMp4 => {
            return Ok(ResolveOutcome::Single(
                make_candidate(url.to_string(), MediaKind::Mp4, None, page_url_ref),
            ));
        }
        EntryKind::M3u8 => {
            let (status, body) = fetch::fetch_playlist_or_page(http, url, &opts).await?;
            if is_auth_required(status) {
                return Ok(ResolveOutcome::NeedsBrowser {
                    reason: "auth_required".into(),
                });
            }
            let result = candidates_from_m3u8_body(&body, url, page_url_ref)?;
            return Ok(map_media_result(result));
        }
        EntryKind::WebPage => {}
    }

    let (status, html) = fetch::fetch_playlist_or_page(http, url, &opts).await?;
    if is_auth_required(status) {
        return Ok(ResolveOutcome::NeedsBrowser {
            reason: "auth_required".into(),
        });
    }

    if let Some(episode_list) = extract_episode_list(&html, url, default_title) {
        return Ok(ResolveOutcome::EpisodeList(episode_list));
    }

    let media_urls = scan_media_urls(&html, url);
    if media_urls.is_empty() {
        return Ok(ResolveOutcome::NeedsBrowser {
            reason: "no_media_found".into(),
        });
    }

    let candidates: Vec<_> = media_urls
        .into_iter()
        .map(|media_url| {
            let kind = media_kind_from_url(&media_url);
            make_candidate(media_url, kind, None, page_url_ref)
        })
        .collect();

    if candidates.len() == 1 {
        Ok(ResolveOutcome::Single(candidates[0].clone()))
    } else {
        Ok(ResolveOutcome::Candidates(candidates))
    }
}

pub async fn resolve_qualities(
    http: &HttpClient,
    media_url: &str,
    opts: ResolveOptions,
) -> Result<Vec<Quality>, EngineError> {
    if classify_entry_url(media_url) != EntryKind::M3u8 {
        return Err(EngineError::InvalidArg("not an m3u8 URL".into()));
    }

    let (status, body) = fetch::fetch_playlist_or_page(http, media_url, &opts).await?;
    if is_auth_required(status) {
        return Err(EngineError::InvalidArg("auth required".into()));
    }

    let variants = list_master_variants(&body, media_url)?;
    Ok(variants.into_iter().map(|(_, quality)| quality).collect())
}

fn is_auth_required(status: StatusCode) -> bool {
    status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN
}

fn map_media_result(result: ResolveMediaResult) -> ResolveOutcome {
    match result {
        ResolveMediaResult::Single(candidate) => ResolveOutcome::Single(candidate),
        ResolveMediaResult::Candidates(candidates) => ResolveOutcome::Candidates(candidates),
    }
}

fn media_kind_from_url(url: &str) -> MediaKind {
    if url.to_ascii_lowercase().contains(".m3u8") {
        MediaKind::Hls
    } else {
        MediaKind::Mp4
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::download::http::HttpClient;

    #[tokio::test]
    async fn direct_mp4_returns_single_without_network() {
        let http = HttpClient::new(None).unwrap();
        let outcome = resolve_url(
            &http,
            "https://cdn.example/clip.mp4",
            ResolveOptions::default(),
        )
        .await
        .unwrap();
        assert!(matches!(outcome, ResolveOutcome::Single(_)));
    }

    #[tokio::test]
    async fn empty_html_page_returns_needs_browser() {
        use axum::{routing::get, Router};
        use tokio::net::TcpListener;

        async fn empty_page() -> &'static str {
            "<html><body>no media</body></html>"
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, Router::new().route("/empty", get(empty_page)))
                .await
                .unwrap();
        });

        let http = HttpClient::new(None).unwrap();
        let url = format!("http://{addr}/empty");
        let outcome = resolve_url(&http, &url, ResolveOptions::default())
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            ResolveOutcome::NeedsBrowser { reason } if reason == "no_media_found"
        ));
        handle.abort();
    }
}
