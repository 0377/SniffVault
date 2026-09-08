use video_sniffing_engine::lan::LanTestConfig;
use video_sniffing_engine::Engine;

fn enable_lan(engine: &mut Engine) {
    let mut settings = engine.settings();
    settings.lan_enabled = true;
    engine.save_settings(settings).unwrap();
}

#[test]
fn lan_pair_and_cast_metadata_has_no_source_url() {
    let sender_dir = tempfile::tempdir().unwrap();
    let receiver_dir = tempfile::tempdir().unwrap();
    let mut receiver = Engine::open(receiver_dir.path()).unwrap();
    let mut sender = Engine::open(sender_dir.path()).unwrap();

    sender.set_lan_test_config(LanTestConfig {
        advertise_ip: Some("127.0.0.1".into()),
    });
    receiver.set_lan_test_config(LanTestConfig {
        advertise_ip: Some("127.0.0.1".into()),
    });

    enable_lan(&mut receiver);
    receiver.apply_lan_settings(true).unwrap();
    let pin = receiver.begin_pairing().unwrap();
    let receiver_port = receiver.lan_http_port().unwrap();

    enable_lan(&mut sender);
    sender.apply_lan_settings(false).unwrap();
    sender.pair_peer("127.0.0.1", receiver_port, &pin).unwrap();

    let media_dir = sender.media_dir();
    std::fs::create_dir_all(&media_dir).unwrap();
    let file = media_dir.join("episode.mp4");
    std::fs::write(&file, b"fake mp4 payload").unwrap();

    let (_item, episode) = sender
        .register_completed_episode(
            "Test Series",
            Some(1),
            1,
            "Pilot",
            file.to_str().unwrap(),
            Some("https://secret.example/stream?token=abc"),
        )
        .unwrap();

    sender
        .cast_episode(&episode.id, &receiver.settings().device_id)
        .unwrap();

    let event = receiver.drain_cast_event().unwrap();
    let json = serde_json::to_string(&event).unwrap();
    assert!(
        !json.contains("source_url"),
        "cast metadata must not leak source_url: {json}"
    );

    sender.stop_cast().unwrap();
    sender.stop_lan().unwrap();
    receiver.stop_lan().unwrap();
}
