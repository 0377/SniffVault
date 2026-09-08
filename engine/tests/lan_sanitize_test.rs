use video_sniffing_engine::lan::sanitize_episode;
use video_sniffing_engine::{LibraryEpisode, LibraryItem, LibraryItemKind};

#[test]
fn l1_sanitize_episode_json_has_no_source_url() {
    let item = LibraryItem {
        id: "item-1".into(),
        kind: LibraryItemKind::Series,
        title: "示意剧".into(),
        season: Some(2),
        poster_path: None,
        created_at_ms: 1,
    };
    let episode = LibraryEpisode {
        id: "ep-1".into(),
        item_id: item.id.clone(),
        index: 3,
        title: "第3集".into(),
        file_path: "media/show_s02e03.mp4".into(),
        duration_ms: Some(1_500_000),
        position_ms: 120_000,
        source_url: Some("https://example.com/stream?source_url=leak&token=secret".into()),
    };

    let metadata = sanitize_episode(&item, &episode);
    let json = serde_json::to_string(&metadata).unwrap();

    assert!(!json.contains("source_url"));
    assert_eq!(metadata.title, "示意剧");
    assert_eq!(metadata.season, Some(2));
    assert_eq!(metadata.episode_index, 3);
    assert_eq!(metadata.episode_title, "第3集");
    assert_eq!(metadata.duration_ms, Some(1_500_000));
    assert_eq!(metadata.position_ms, 120_000);
    assert_eq!(metadata.mime, "video/mp4");
}
