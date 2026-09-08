use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::StatusCode;
use tokio::sync::mpsc;
use video_sniffing_engine::lan::{
    begin_pairing, sign_request, CastEvent, CastMetadata, CastPlayRequest, ReceiverHttp,
    ReceiverState, TrustStore, TrustedPeer,
};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

async fn start_test_receiver_with_trust(
    trusted_peers: &[TrustedPeer],
) -> (ReceiverHttp, u16, mpsc::Receiver<CastEvent>) {
    let dir = tempfile::tempdir().unwrap();
    let trust_store = TrustStore::open(&dir.path().join("lan.db")).unwrap();
    for peer in trusted_peers {
        trust_store.upsert_peer(peer).unwrap();
    }

    let (event_tx, event_rx) = mpsc::channel(8);
    let pairing_session = begin_pairing(now_ms());
    let state = ReceiverState {
        device_id: "tv-test".into(),
        trust_store: Arc::new(Mutex::new(trust_store)),
        pairing_session: Arc::new(Mutex::new(Some(pairing_session))),
        event_tx,
    };

    let (server, port) = ReceiverHttp::start("127.0.0.1:0".parse().unwrap(), state)
        .await
        .unwrap();
    (server, port, event_rx)
}

fn sample_cast_play_request(sender_device_id: &str) -> CastPlayRequest {
    CastPlayRequest {
        session_id: "sess-1".into(),
        sender_device_id: sender_device_id.into(),
        sender_name: "Phone".into(),
        metadata: CastMetadata {
            title: "Show".into(),
            season: Some(1),
            episode_index: 1,
            episode_title: "Pilot".into(),
            duration_ms: Some(3_600_000),
            position_ms: 0,
            mime: "video/mp4".into(),
        },
        stream_url: "http://192.168.1.5:9000/v1/stream/tok".into(),
    }
}

async fn post_cast_play_signed(
    host: &str,
    port: u16,
    device_id: &str,
    secret: &[u8; 32],
) -> reqwest::Response {
    let body = serde_json::to_vec(&sample_cast_play_request(device_id)).unwrap();
    let timestamp = now_secs();
    let signature = sign_request(secret, device_id, timestamp, "POST", "/v1/cast/play", &body);
    let client = reqwest::Client::new();
    client
        .post(format!("http://{host}:{port}/v1/cast/play"))
        .header("X-SniffVault-Device-Id", device_id)
        .header("X-SniffVault-Timestamp", timestamp.to_string())
        .header("X-SniffVault-Signature", signature)
        .body(body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn l5_untrusted_sender_cast_play_returns_403() {
    let secret = [1u8; 32];
    let (server, port, _rx) = start_test_receiver_with_trust(&[]).await;
    let resp = post_cast_play_signed("127.0.0.1", port, "unknown-sender", &secret).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    server.stop().await;
}
