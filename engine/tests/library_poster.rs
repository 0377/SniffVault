use tempfile::tempdir;
use video_sniffing_engine::Engine;

#[path = "support/library_merge_seed.rs"]
mod library_merge_seed;

#[test]
fn merge_migrates_poster_when_target_empty() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let poster = engine.media_dir().join(".posters").join("p.jpg");
    std::fs::create_dir_all(poster.parent().unwrap()).unwrap();
    std::fs::write(&poster, b"x").unwrap();
    let canon = poster
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let seed = library_merge_seed::seed_duplicate_series_with_source_poster(
        &engine,
        "剧",
        Some(1),
        &canon,
    );
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "a.mp4", 0);
    library_merge_seed::add_episode(&engine, &seed.target_item_id, 1, "目标1", "b.mp4", 0);
    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();
    let target = engine
        .list_library()
        .unwrap()
        .into_iter()
        .find(|i| i.id == seed.target_item_id)
        .unwrap();
    assert_eq!(target.poster_path.as_deref(), Some(canon.as_str()));
    assert!(engine
        .list_library()
        .unwrap()
        .iter()
        .all(|i| i.id != seed.source_item_id));
}
