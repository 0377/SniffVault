use video_sniffing_engine::lan::{
    begin_pairing, sign_request, verify_pin, verify_request, PAIRING_PIN_TTL_MS,
    SIGNATURE_TIMESTAMP_WINDOW_SECS,
};

#[test]
fn l4_expired_pin_rejected() {
    let session = begin_pairing(1_000);
    let pin = session.pin.clone();
    let err = verify_pin(&session, &pin, 1_000 + PAIRING_PIN_TTL_MS + 1).unwrap_err();
    assert!(
        err.to_string().contains("expired"),
        "expected expired pin error, got: {err}"
    );
}

#[test]
fn signature_roundtrip() {
    let secret = [7u8; 32];
    let device_id = "sender-abc";
    let timestamp = 1_700_000_000_i64;
    let method = "POST";
    let path = "/v1/cast/play";
    let body = br#"{"session_id":"s1"}"#;

    let signature = sign_request(&secret, device_id, timestamp, method, path, body);
    verify_request(
        &secret, device_id, timestamp, timestamp, method, path, body, &signature,
    )
    .unwrap();

    let stale = timestamp - SIGNATURE_TIMESTAMP_WINDOW_SECS - 1;
    let err = verify_request(
        &secret,
        device_id,
        stale,
        timestamp,
        method,
        path,
        body,
        &sign_request(&secret, device_id, stale, method, path, body),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("timestamp"),
        "expected timestamp window error, got: {err}"
    );
}
