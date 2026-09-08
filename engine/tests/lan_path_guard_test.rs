use std::time::{SystemTime, UNIX_EPOCH};
use video_sniffing_engine::ingest;
use video_sniffing_engine::lan::{StreamTokenStore, LAN_STREAM_TOKEN_TTL_SECS};
use video_sniffing_engine::LibraryEpisode;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn l2_rejects_path_outside_media_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let media = tmp.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    let outside = tmp.path().join("outside.mp4");
    std::fs::write(&outside, b"x").unwrap();
    let err = ingest::ensure_path_in_media_dir(&media, outside.to_str().unwrap());
    assert!(err.is_err(), "path outside media_dir must be rejected");
}

#[test]
#[cfg(unix)]
fn l3_symlink_outside_media_dir_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let media = tmp.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    let outside = tmp.path().join("secret.mp4");
    std::fs::write(&outside, b"x").unwrap();
    let link = media.join("link.mp4");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let err = ingest::ensure_path_in_media_dir(&media, link.to_str().unwrap());
    assert!(err.is_err(), "symlink escape must be rejected");
}

#[test]
fn l8_expired_token_resolve_path_returns_none() {
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
    let issued_at = now_secs();
    let token = store.issue("ep-1", issued_at);

    let expired_at = issued_at + LAN_STREAM_TOKEN_TTL_SECS + 1;
    let path = store.resolve_path(&token, expired_at, &media, |id| {
        if id == "ep-1" {
            Some(episode.clone())
        } else {
            None
        }
    });
    assert!(path.is_none(), "expired token must not resolve to a path");
}
