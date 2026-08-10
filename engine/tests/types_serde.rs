use video_sniffing_engine::{
    DownloadTask, Episode, EpisodeList, LibraryItem, LibraryItemKind, MediaKind, Quality,
    ResolveOptions, ResolveOutcome, ResourceCandidate, SniffEvent, SniffInitiator, TaskStatus,
};

#[test]
fn resource_candidate_roundtrip_json() {
    let c = ResourceCandidate {
        id: "c1".into(),
        url: "https://example.com/a.m3u8".into(),
        title: Some("Demo".into()),
        kind: MediaKind::Hls,
        quality: Some(Quality {
            label: "1080p".into(),
            width: Some(1920),
            height: Some(1080),
            bandwidth: Some(5_000_000),
        }),
        page_url: Some("https://example.com/watch/1".into()),
    };
    let s = serde_json::to_string(&c).expect("serialize");
    let back: ResourceCandidate = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back.url, c.url);
    assert_eq!(back.kind, MediaKind::Hls);
    assert_eq!(back.quality.as_ref().unwrap().label, "1080p");
}

#[test]
fn episode_list_and_task_status_defaults() {
    let list = EpisodeList {
        title: "示意剧".into(),
        season: Some(1),
        episodes: vec![Episode {
            index: 1,
            title: "第1集".into(),
            url: "https://example.com/ep1.m3u8".into(),
            quality_options: vec![],
        }],
    };
    assert_eq!(list.episodes.len(), 1);
    assert_eq!(TaskStatus::Queued, TaskStatus::Queued);
    let _ = LibraryItem {
        id: "i1".into(),
        kind: LibraryItemKind::Series,
        title: list.title.clone(),
        season: list.season,
        poster_path: None,
        created_at_ms: 0,
    };
    let _ = DownloadTask {
        id: "t1".into(),
        parent_id: None,
        season: Some(1),
        title: "ep1".into(),
        source_url: "https://example.com/ep1.m3u8".into(),
        quality_label: Some("1080p".into()),
        status: TaskStatus::Queued,
        progress_bytes: 0,
        total_bytes: None,
        error_message: None,
        output_path: None,
        library_item_id: None,
        episode_index: Some(1),
        created_at_ms: 0,
        updated_at_ms: 0,
    };
}

#[test]
fn resolve_outcome_and_sniff_event_roundtrip_json() {
    let event = SniffEvent {
        url: "http://x/v.m3u8".into(),
        page_url: Some("http://x/page".into()),
        initiator: SniffInitiator::SubResource,
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: SniffEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, event);

    let opts = ResolveOptions {
        cookies: Some("sid=abc".into()),
        referer: Some("http://x/page".into()),
        page_url: Some("http://x/page".into()),
    };
    let json = serde_json::to_string(&opts).unwrap();
    let back: ResolveOptions = serde_json::from_str(&json).unwrap();
    assert_eq!(back, opts);

    let outcome = ResolveOutcome::NeedsBrowser {
        reason: "auth_required".into(),
    };
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains("needs_browser"));
}
