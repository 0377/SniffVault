use tempfile::tempdir;
use video_sniffing_engine::Engine;

#[test]
fn enqueue_series_persists_season_on_parent_and_children() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();

    let (parent_id, children) = engine
        .enqueue_episodes(
            "示意剧",
            Some(1),
            &[
                (1, "第1集".into(), "https://ex/1.m3u8".into()),
                (2, "第2集".into(), "https://ex/2.m3u8".into()),
            ],
            Some("1080p"),
        )
        .unwrap();

    assert_eq!(children.len(), 2);
    let tasks = engine.list_tasks().unwrap();
    assert_eq!(tasks.len(), 3);
    let parent = tasks.iter().find(|t| t.id == parent_id).unwrap();
    assert!(parent.parent_id.is_none());
    assert_eq!(parent.season, Some(1));
    for id in &children {
        let child = tasks.iter().find(|t| t.id == *id).unwrap();
        assert_eq!(child.season, Some(1));
        assert_eq!(child.parent_id.as_deref(), Some(parent_id.as_str()));
    }
}

#[test]
fn register_completed_merges_and_dedupes_episode_index() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("ep1.mp4");
    std::fs::create_dir_all(media.parent().unwrap()).unwrap();
    std::fs::write(&media, b"fake").unwrap();

    let (item1, ep1a) = engine
        .register_completed_episode(
            "示意剧",
            Some(1),
            1,
            "第1集",
            media.to_str().unwrap(),
            Some("https://ex/1.m3u8"),
        )
        .unwrap();

    let media2 = engine.media_dir().join("ep2.mp4");
    std::fs::write(&media2, b"fake").unwrap();
    let (item2, _) = engine
        .register_completed_episode(
            "示意剧",
            Some(1),
            2,
            "第2集",
            media2.to_str().unwrap(),
            None,
        )
        .unwrap();
    assert_eq!(item1.id, item2.id);

    let media1b = engine.media_dir().join("ep1b.mp4");
    std::fs::write(&media1b, b"fake2").unwrap();
    let (_, ep1b) = engine
        .register_completed_episode(
            "示意剧",
            Some(1),
            1,
            "第1集-重下",
            media1b.to_str().unwrap(),
            None,
        )
        .unwrap();
    assert_eq!(ep1a.id, ep1b.id);
    assert_eq!(engine.list_episodes(&item1.id).unwrap().len(), 2);
}

#[test]
fn register_rejects_path_outside_media_dir() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let outside = dir.path().join("outside.mp4");
    std::fs::write(&outside, b"x").unwrap();
    let err = engine
        .register_completed_episode(
            "示意剧",
            Some(1),
            1,
            "第1集",
            outside.to_str().unwrap(),
            None,
        )
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("media") || msg.contains("path"), "{msg}");
}

#[test]
fn settings_roundtrip_and_media_dir() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    assert!(engine.media_dir().ends_with("media"));
    let mut s = engine.settings();
    s.device_name = "LivingRoom".into();
    s.media_dir = "videos".into();
    engine.save_settings(s).unwrap();
    drop(engine);
    let engine2 = Engine::open(dir.path()).unwrap();
    assert_eq!(engine2.settings().device_name, "LivingRoom");
    assert!(engine2.media_dir().ends_with("videos"));
    assert!(engine2.media_dir().is_dir());
}
