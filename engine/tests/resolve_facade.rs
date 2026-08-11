mod support;

use support::fixture_server;
use video_sniffing_engine::{
    MediaKind, ResolveOptions, ResolveOutcome, resolve_url_for_ffi,
};

#[tokio::test]
async fn resolve_url_for_ffi_direct_mp4() {
    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");
    let outcome = resolve_url_for_ffi(None, &url, ResolveOptions::default())
        .await
        .unwrap();
    match outcome {
        ResolveOutcome::Single(c) => assert_eq!(c.kind, MediaKind::Mp4),
        _ => panic!("expected single mp4"),
    }
}
