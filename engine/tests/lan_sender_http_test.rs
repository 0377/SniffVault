use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::StatusCode;
use video_sniffing_engine::lan::{SenderHttp, SenderState, StreamTokenStore};
use video_sniffing_engine::LibraryEpisode;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

async fn start_test_sender_with_token(token: &str, file: &Path) -> (SenderHttp, u16) {
    let media_dir = file.parent().unwrap().to_path_buf();
    let episode_id = "ep-test";
    let episode = LibraryEpisode {
        id: episode_id.into(),
        item_id: "item-test".into(),
        index: 1,
        title: "Test".into(),
        file_path: file.to_string_lossy().into(),
        duration_ms: None,
        position_ms: 0,
        source_url: None,
    };

    let mut store = StreamTokenStore::new();
    store.register(token, episode_id, now_secs());

    let episode_for_lookup = episode.clone();
    let get_episode = move |id: &str| {
        if id == episode_id {
            Some(episode_for_lookup.clone())
        } else {
            None
        }
    };

    let state = SenderState {
        device_id: "sender-test".into(),
        stream_token_store: Arc::new(Mutex::new(store)),
        media_dir,
        get_episode: Arc::new(get_episode),
    };

    let (server, port) = SenderHttp::start("127.0.0.1:0".parse().unwrap(), state)
        .await
        .unwrap();
    (server, port)
}

#[tokio::test]
async fn l6_range_request_returns_partial_content() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("clip.mp4");
    std::fs::write(&file, b"0123456789").unwrap();
    let token = "test-token";
    let (server, port) = start_test_sender_with_token(token, &file).await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{port}/v1/stream/{token}"))
        .header("Range", "bytes=0-3")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(resp.bytes().await.unwrap().as_ref(), b"0123");
    server.stop().await;
}

#[tokio::test]
async fn l6_head_request_returns_content_length_without_body() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("clip.mp4");
    std::fs::write(&file, b"0123456789").unwrap();
    let token = "test-token";
    let (server, port) = start_test_sender_with_token(token, &file).await;
    let client = reqwest::Client::new();
    let resp = client
        .head(format!("http://127.0.0.1:{port}/v1/stream/{token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-length").and_then(|v| v.to_str().ok()),
        Some("10")
    );
    assert!(resp.bytes().await.unwrap().is_empty());
    server.stop().await;
}
