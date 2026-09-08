use video_sniffing_engine::lan::{TrustStore, TrustedPeer};

#[test]
fn trust_store_persists_session_secret_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lan.db");
    let store = TrustStore::open(&path).unwrap();
    let secret = [9u8; 32];
    store
        .upsert_peer(&TrustedPeer {
            peer_device_id: "tv1".into(),
            peer_name: "TV".into(),
            peer_host: "192.168.1.10".into(),
            peer_port: 8080,
            paired_at_ms: 1,
            session_secret: Some(secret),
        })
        .unwrap();
    let store2 = TrustStore::open(&path).unwrap();
    let peers = store2.list_peers().unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].session_secret, Some(secret));
}
