use tempfile::TempDir;
use video_sniffing_engine::Engine;

#[test]
fn settings_gets_stable_device_id_on_first_open() {
    let dir = TempDir::new().unwrap();
    let id1 = Engine::open(dir.path()).unwrap().settings().device_id;
    let id2 = Engine::open(dir.path()).unwrap().settings().device_id;
    assert!(!id1.is_empty());
    assert_eq!(id1, id2);
}

#[test]
fn legacy_settings_json_without_device_id_migrates() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        r#"{"media_dir":"media","max_concurrency":2,"default_quality_label":"highest","device_name":"Legacy"}"#,
    )
    .unwrap();
    let s = Engine::open(dir.path()).unwrap().settings();
    assert!(!s.device_id.is_empty());
    assert!(!s.lan_enabled);
}
