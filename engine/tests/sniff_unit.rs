use video_sniffing_engine::{sniff, MediaKind, ResourceCandidate, SniffEvent, SniffInitiator};

#[test]
fn classify_m3u8_and_mp4() {
    assert_eq!(
        sniff::classify_media_url("https://x/v/master.m3u8"),
        Some(MediaKind::Hls)
    );
    assert_eq!(
        sniff::classify_media_url("https://x/v/clip.mp4?token=1"),
        Some(MediaKind::Mp4)
    );
    assert_eq!(sniff::classify_media_url("https://x/page.html"), None);
}

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
    let out = sniff::dedup_candidates(vec![a, b]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, MediaKind::Hls);
}

#[test]
fn sniff_urls_filters_and_dedups() {
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
    let out = sniff::sniff_urls(&events, Some("https://x/page"));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].url, "https://x/a.m3u8");
    assert_eq!(out[0].page_url.as_deref(), Some("https://x/page"));
}
