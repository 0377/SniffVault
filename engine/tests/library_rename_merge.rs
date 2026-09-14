use tempfile::tempdir;
use video_sniffing_engine::Engine;
use video_sniffing_engine::EngineError;

#[path = "support/library_merge_seed.rs"]
mod library_merge_seed;

#[path = "support/library_dirty.rs"]
mod library_dirty;

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

#[test]
fn merge_moves_episodes_and_deletes_source_shell() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "s1.mp4", 0);
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 2, "源2", "s2.mp4", 0);

    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();

    let items = engine.list_library().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, seed.target_item_id);
    let eps = engine.list_episodes(&seed.target_item_id).unwrap();
    assert_eq!(eps.len(), 2);
}

#[test]
fn merge_idx_conflict_keeps_target_progress() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    library_merge_seed::add_episode(&engine, &seed.target_item_id, 1, "目标1", "t1.mp4", 5000);
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "s1.mp4", 100);

    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();

    let eps = engine.list_episodes(&seed.target_item_id).unwrap();
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].position_ms, 5000);
    assert!(engine.media_dir().join("s1.mp4").exists());
}

#[test]
fn merge_delete_orphan_files_removes_conflict_source_file() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    library_merge_seed::add_episode(&engine, &seed.target_item_id, 1, "目标1", "t1.mp4", 5000);
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "s1.mp4", 100);
    let s1 = engine.media_dir().join("s1.mp4");
    let t1 = engine.media_dir().join("t1.mp4");
    assert!(s1.exists());
    assert!(t1.exists());

    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, true)
        .unwrap();

    assert!(!s1.exists());
    assert!(t1.exists());
}

#[test]
fn merge_rejects_mismatched_title_season_or_single() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "剧A", Some(1));
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源", "s.mp4", 0);
    let err = engine
        .merge_library_items(&seed.source_item_id, &seed.source_item_id, false)
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));

    let media = engine.media_dir().join("single.mp4");
    std::fs::write(&media, b"x").unwrap();
    let (single, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    let err = engine
        .merge_library_items(&seed.source_item_id, &single.id, false)
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidArg(_)));
}

#[test]
fn merge_aborts_when_orphan_path_outside_media_dir() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let seed = library_merge_seed::seed_duplicate_series(&engine, "示意剧", Some(1));
    let target_ep = library_merge_seed::add_episode(
        &engine,
        &seed.target_item_id,
        1,
        "目标1",
        "t1.mp4",
        5000,
    );
    let source_ep = library_merge_seed::add_episode(
        &engine,
        &seed.source_item_id,
        1,
        "源1",
        "s1.mp4",
        100,
    );
    let outside = dir.path().join("outside.mp4");
    std::fs::write(&outside, b"x").unwrap();
    library_dirty::inject_outside_file_path(&engine, &seed.source_item_id, outside.to_str().unwrap());
    let _ = source_ep;
    let _ = target_ep;

    let err = engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, true)
        .unwrap_err();
    assert!(
        err.to_string().contains("media") || err.to_string().contains("media_dir")
    );
    assert_eq!(engine.list_library().unwrap().len(), 2);
}

#[test]
fn merge_orphan_after_cast_allows_new_cast() {
    use video_sniffing_engine::lan::LanTestConfig;

    let sender_dir = tempfile::tempdir().unwrap();
    let receiver_dir = tempfile::tempdir().unwrap();
    let mut receiver = Engine::open(receiver_dir.path()).unwrap();
    let mut sender = Engine::open(sender_dir.path()).unwrap();
    for e in [&mut sender, &mut receiver] {
        let mut s = e.settings();
        s.lan_enabled = true;
        e.save_settings(s).unwrap();
        e.set_lan_test_config(LanTestConfig {
            advertise_ip: Some("127.0.0.1".into()),
        });
    }
    receiver.apply_lan_settings(true).unwrap();
    let pin = receiver.begin_pairing().unwrap();
    let port = receiver.lan_http_port().unwrap();
    sender.apply_lan_settings(false).unwrap();
    sender.pair_peer("127.0.0.1", port, &pin).unwrap();

    let seed = library_merge_seed::seed_duplicate_series(&sender, "示意剧", Some(1));
    let orphan = library_merge_seed::add_episode(
        &sender,
        &seed.source_item_id,
        1,
        "源1",
        "orphan.mp4",
        0,
    );
    library_merge_seed::add_episode(&sender, &seed.target_item_id, 1, "目标1", "keep.mp4", 0);
    sender
        .cast_episode(&orphan.id, &receiver.settings().device_id)
        .unwrap();

    sender
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();
    assert!(!sender.has_active_cast());

    let migrated = library_merge_seed::add_episode(
        &sender,
        &seed.target_item_id,
        2,
        "第2集",
        "ep2.mp4",
        0,
    );
    sender
        .cast_episode(&migrated.id, &receiver.settings().device_id)
        .unwrap();
    sender.stop_cast().unwrap();
    sender.stop_lan().unwrap();
    receiver.stop_lan().unwrap();
}
