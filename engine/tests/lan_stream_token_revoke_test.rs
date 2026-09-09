use std::time::{SystemTime, UNIX_EPOCH};
use video_sniffing_engine::lan::StreamTokenStore;
use video_sniffing_engine::LibraryEpisode;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn revoke_for_episode_invalidates_token() {
    let tmp = tempfile::tempdir().unwrap();
    let media = tmp.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    let file = media.join("clip.mp4");
    std::fs::write(&file, b"video").unwrap();

    let episode = LibraryEpisode {
        id: "ep-1".into(),
        item_id: "item-1".into(),
        index: 1,
        title: "第1集".into(),
        file_path: file.to_string_lossy().into(),
        duration_ms: None,
        position_ms: 0,
        source_url: None,
    };

    let mut store = StreamTokenStore::new();
    let now = now_secs();
    store.register("tok-a", "ep-1", now);
    store.revoke_for_episode("ep-1");
    assert!(
        store
            .resolve_path("tok-a", now, &media, |id| {
                if id == "ep-1" {
                    Some(episode.clone())
                } else {
                    None
                }
            })
            .is_none(),
        "revoked token must not resolve to a path"
    );
    let new_tok = store.issue("ep-1", now);
    assert_ne!(new_tok, "tok-a");
}
