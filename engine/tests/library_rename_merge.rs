use tempfile::tempdir;
use video_sniffing_engine::Engine;
use video_sniffing_engine::EngineError;

#[test]
fn rename_library_item_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("movie.mp4");
    std::fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("旧名", media.to_str().unwrap(), None)
        .unwrap();
    engine.rename_library_item(&item.id, "新名").unwrap();
    drop(engine);

    let engine2 = Engine::open(dir.path()).unwrap();
    let lib = engine2.list_library().unwrap();
    assert_eq!(lib[0].title, "新名");
}

#[test]
fn rename_single_item_syncs_episode_title() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("movie.mp4");
    std::fs::write(&media, b"x").unwrap();
    let (item, ep) = engine
        .register_completed_single("旧名", media.to_str().unwrap(), None)
        .unwrap();
    engine.rename_library_item(&item.id, "新名").unwrap();
    let eps = engine.list_episodes(&item.id).unwrap();
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].title, "新名");
    assert_eq!(eps[0].id, ep.id);
}

#[test]
fn rename_episode_only_changes_title() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let m1 = engine.media_dir().join("ep1.mp4");
    let m2 = engine.media_dir().join("ep2.mp4");
    std::fs::write(&m1, b"x").unwrap();
    std::fs::write(&m2, b"x").unwrap();
    let (item, ep1) = engine
        .register_completed_episode("剧", Some(1), 1, "第1集", m1.to_str().unwrap(), None)
        .unwrap();
    engine
        .register_completed_episode("剧", Some(1), 2, "第2集", m2.to_str().unwrap(), None)
        .unwrap();
    let original_path = ep1.file_path.clone();
    engine.rename_episode(&ep1.id, "新第1集").unwrap();
    let ep = engine.get_episode(&ep1.id).unwrap().unwrap();
    assert_eq!(ep.title, "新第1集");
    assert_eq!(ep.file_path, original_path);
    let item_after = engine.list_library().unwrap();
    assert_eq!(item_after[0].title, "剧");
    let _ = item;
}

#[test]
fn rename_rejects_invalid_title() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("movie.mp4");
    std::fs::write(&media, b"x").unwrap();
    let (item, ep) = engine
        .register_completed_single("x", media.to_str().unwrap(), None)
        .unwrap();
    let err = engine.rename_library_item(&item.id, "   ").unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
    let long = "a".repeat(513);
    let err = engine.rename_library_item(&item.id, &long).unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
    let err = engine.rename_episode(&ep.id, "").unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn rename_series_item_title_does_not_change_episode_titles() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let m1 = engine.media_dir().join("ep1.mp4");
    let m2 = engine.media_dir().join("ep2.mp4");
    std::fs::write(&m1, b"x").unwrap();
    std::fs::write(&m2, b"x").unwrap();
    let (item, _) = engine
        .register_completed_episode("剧", Some(1), 1, "第1集", m1.to_str().unwrap(), None)
        .unwrap();
    engine
        .register_completed_episode("剧", Some(1), 2, "第2集", m2.to_str().unwrap(), None)
        .unwrap();
    engine.rename_library_item(&item.id, "新剧名").unwrap();
    let eps = engine.list_episodes(&item.id).unwrap();
    assert_eq!(engine.list_library().unwrap()[0].title, "新剧名");
    assert_eq!(eps[0].title, "第1集");
    assert_eq!(eps[1].title, "第2集");
}
