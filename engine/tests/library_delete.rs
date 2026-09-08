mod support;

use std::fs;
use support::library_dirty::inject_outside_file_path;
use tempfile::tempdir;
use video_sniffing_engine::Engine;

fn seed_single(engine: &mut Engine, file_name: &str) -> (String, String) {
    let media = engine.media_dir().join(file_name);
    fs::write(&media, b"video-bytes").unwrap();
    let (item, ep) = engine
        .register_completed_single("测试片", media.to_str().unwrap(), None)
        .unwrap();
    (item.id, ep.id)
}

#[test]
fn remove_library_item_deletes_db_and_file() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let (item_id, _) = seed_single(&mut engine, "clip.mp4");
    let media_path = engine.media_dir().join("clip.mp4");
    assert!(media_path.exists());

    engine.remove_library_item(&item_id, true).unwrap();
    assert!(engine.list_library().unwrap().is_empty());
    assert!(!media_path.exists());
}

#[test]
fn remove_library_item_keep_files_on_disk() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let (item_id, _) = seed_single(&mut engine, "clip.mp4");
    let media_path = engine.media_dir().join("clip.mp4");

    engine.remove_library_item(&item_id, false).unwrap();
    assert!(engine.list_library().unwrap().is_empty());
    assert!(media_path.exists());
}

#[test]
fn remove_rejects_outside_path_when_delete_files() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("good.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    let outside = dir.path().join("outside.mp4");
    fs::write(&outside, b"x").unwrap();
    inject_outside_file_path(&engine, &item.id, outside.to_str().unwrap());

    let err = engine.remove_library_item(&item.id, true).unwrap_err();
    assert!(err.to_string().contains("media_dir") || err.to_string().contains("media"));
    assert_eq!(engine.list_library().unwrap().len(), 1);
}

#[test]
fn remove_allows_dirty_path_when_keep_files() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("good.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    let outside = dir.path().join("outside.mp4");
    fs::write(&outside, b"x").unwrap();
    inject_outside_file_path(&engine, &item.id, outside.to_str().unwrap());

    engine.remove_library_item(&item.id, false).unwrap();
    assert!(engine.list_library().unwrap().is_empty());
    assert!(outside.exists());
}

#[test]
fn remove_last_episode_removes_item() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let (_, ep_id) = seed_single(&mut engine, "clip.mp4");

    engine.remove_episode(&ep_id, false).unwrap();
    assert!(engine.list_library().unwrap().is_empty());
}

#[test]
fn remove_missing_file_still_commits_db() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let (item_id, _) = seed_single(&mut engine, "clip.mp4");
    let media_path = engine.media_dir().join("clip.mp4");
    fs::remove_file(&media_path).unwrap();

    engine.remove_library_item(&item_id, true).unwrap();
    assert!(engine.list_library().unwrap().is_empty());
}

#[test]
fn remove_episode_after_cast_allows_new_cast() {
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

    let f1 = sender.media_dir().join("ep1.mp4");
    fs::write(&f1, b"x").unwrap();
    let (_, ep1) = sender
        .register_completed_single("片1", f1.to_str().unwrap(), None)
        .unwrap();
    sender
        .cast_episode(&ep1.id, &receiver.settings().device_id)
        .unwrap();

    sender.remove_episode(&ep1.id, true).unwrap();
    assert!(!sender.has_active_cast());

    let f2 = sender.media_dir().join("ep2.mp4");
    fs::write(&f2, b"x").unwrap();
    let (_, ep2) = sender
        .register_completed_single("片2", f2.to_str().unwrap(), None)
        .unwrap();
    sender
        .cast_episode(&ep2.id, &receiver.settings().device_id)
        .unwrap();

    sender.stop_cast().unwrap();
    sender.stop_lan().unwrap();
    receiver.stop_lan().unwrap();
}
