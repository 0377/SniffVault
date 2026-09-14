mod support;

use support::fixture_server;
use video_sniffing_engine::{resolve_url_for_ffi, MediaKind, ResolveOptions, ResolveOutcome};

#[tokio::test]
async fn resolve_url_for_ffi_direct_mp4() {
    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");
    let result = resolve_url_for_ffi(None, &url, ResolveOptions::default())
        .await
        .unwrap();
    match result.outcome {
        ResolveOutcome::Single(c) => assert_eq!(c.kind, MediaKind::Mp4),
        _ => panic!("expected single mp4"),
    }
    assert!(result.poster_url.is_none());
}

#[tokio::test]
async fn resolve_url_for_ffi_includes_poster_from_html() {
    let (addr, _guard) =
        fixture_server::serve_dir(fixture_server::fixtures_dir().join("html")).await;
    let page_url = format!("http://{addr}/og_image_player_page.html");
    let result = resolve_url_for_ffi(None, &page_url, ResolveOptions::default())
        .await
        .unwrap();
    assert_eq!(
        result.poster_url.as_deref(),
        Some("https://cdn.example.com/poster.jpg")
    );
    assert!(matches!(result.outcome, ResolveOutcome::Single(_)));
}
