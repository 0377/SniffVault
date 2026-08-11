mod support;

use axum::{
    extract::Request,
    http::{header, StatusCode},
    routing::get,
    Router,
};
use support::fixture_server;
use tempfile::tempdir;
use tokio::net::TcpListener;
use video_sniffing_engine::{
    Engine, MediaKind, ResolveOptions, ResolveOutcome, SniffEvent, SniffInitiator,
};

#[tokio::test]
async fn r1_direct_mp4_is_single() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");
    let outcome = engine
        .resolve_url(&url, ResolveOptions::default())
        .await
        .unwrap();
    match outcome {
        ResolveOutcome::Single(c) => assert_eq!(c.kind, MediaKind::Mp4),
        _ => panic!("expected single mp4"),
    }
}

#[tokio::test]
async fn r2_master_m3u8_is_candidates_with_quality() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, _guard) =
        fixture_server::serve_dir(fixture_server::fixtures_dir().join("hls")).await;
    let url = format!("http://{addr}/master.m3u8");
    let outcome = engine
        .resolve_url(&url, ResolveOptions::default())
        .await
        .unwrap();
    match outcome {
        ResolveOutcome::Candidates(list) => {
            assert!(list.len() >= 2);
            assert!(list.iter().any(|c| c.quality.as_ref().is_some()));
        }
        _ => panic!("expected candidates"),
    }
}

#[tokio::test]
async fn r3_media_m3u8_is_single_hls() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, _guard) =
        fixture_server::serve_dir(fixture_server::fixtures_dir().join("hls")).await;
    let url = format!("http://{addr}/media.m3u8");
    let outcome = engine
        .resolve_url(&url, ResolveOptions::default())
        .await
        .unwrap();
    match outcome {
        ResolveOutcome::Single(c) => assert_eq!(c.kind, MediaKind::Hls),
        _ => panic!("expected single hls"),
    }
}

#[tokio::test]
async fn r4_series_page_is_episode_list() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, _guard) =
        fixture_server::serve_dir(fixture_server::fixtures_dir().join("html")).await;
    let url = format!("http://{addr}/series_page.html");
    let outcome = engine
        .resolve_url(
            &url,
            ResolveOptions {
                page_url: Some("测试剧".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    match outcome {
        ResolveOutcome::EpisodeList(list) => {
            assert_eq!(list.title, "测试剧");
            assert!(list.episodes.len() >= 3);
        }
        _ => panic!("expected episode list"),
    }
}

#[tokio::test]
async fn r5_embedded_m3u8_resolves_master() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, _guard) =
        fixture_server::serve_dir(fixture_server::fixtures_dir().join("html")).await;
    let page_url = format!("http://{addr}/embedded_m3u8.html");
    let expected_master = format!("http://{addr}/master.m3u8");

    let outcome = engine
        .resolve_url(&page_url, ResolveOptions::default())
        .await
        .unwrap();
    match outcome {
        ResolveOutcome::Single(c) => {
            assert_eq!(c.url, expected_master);
            assert_eq!(c.kind, MediaKind::Hls);
        }
        ResolveOutcome::Candidates(list) => {
            assert!(list.iter().any(|c| c.url == expected_master));
        }
        _ => panic!("expected single or candidates with master.m3u8"),
    }
}

async fn spawn_auth_gate_server(
    auth_body: &'static str,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let handler = move |request: Request| {
            let auth_body = auth_body;
            async move {
                let has_cookie = request
                    .headers()
                    .get(header::COOKIE)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.contains("sid=ok"));
                if has_cookie {
                    return (StatusCode::OK, auth_body);
                }
                (StatusCode::FORBIDDEN, "forbidden")
            }
        };
        axum::serve(listener, Router::new().route("/auth.html", get(handler)))
            .await
            .unwrap();
    });
    (addr, handle)
}

#[tokio::test]
async fn r6_cookie_auth() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, handle) = spawn_auth_gate_server("<html><body>authed</body></html>").await;
    let url = format!("http://{addr}/auth.html");

    let denied = engine
        .resolve_url(&url, ResolveOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        denied,
        ResolveOutcome::NeedsBrowser { reason } if reason == "auth_required"
    ));

    let ok = engine
        .resolve_url(
            &url,
            ResolveOptions {
                cookies: Some("sid=ok".into()),
                referer: None,
                page_url: None,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        ok,
        ResolveOutcome::NeedsBrowser { reason } if reason == "no_media_found"
    ));
    handle.abort();
}

#[tokio::test]
async fn r7_sniff_urls_dedups() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let events = vec![
        SniffEvent {
            url: "https://x/a.m3u8".into(),
            page_url: None,
            initiator: SniffInitiator::Media,
        },
        SniffEvent {
            url: "https://x/logo.png".into(),
            page_url: None,
            initiator: SniffInitiator::SubResource,
        },
        SniffEvent {
            url: "https://x/a.m3u8".into(),
            page_url: None,
            initiator: SniffInitiator::Media,
        },
    ];
    let out = engine.sniff_urls(&events, Some("http://x/page"));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].url, "https://x/a.m3u8");
    assert_eq!(out[0].page_url.as_deref(), Some("http://x/page"));
}

#[tokio::test]
async fn r8_resolve_qualities_labels() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let (addr, _guard) =
        fixture_server::serve_dir(fixture_server::fixtures_dir().join("hls")).await;
    let url = format!("http://{addr}/master.m3u8");
    let qualities = engine
        .resolve_qualities(&url, ResolveOptions::default())
        .await
        .unwrap();
    let labels: Vec<&str> = qualities.iter().map(|q| q.label.as_str()).collect();
    assert!(labels.contains(&"720p"));
    assert!(labels.contains(&"1080p"));
}

#[tokio::test]
async fn r9_not_found_page_returns_http_error() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        async fn not_found() -> StatusCode {
            StatusCode::NOT_FOUND
        }
        axum::serve(listener, Router::new().route("/missing", get(not_found)))
            .await
            .unwrap();
    });
    let url = format!("http://{addr}/missing");
    let err = engine
        .resolve_url(&url, ResolveOptions::default())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        video_sniffing_engine::EngineError::Message(_)
    ));
    handle.abort();
}
