use crate::error::EngineError;
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};

pub const SIGNATURE_TIMESTAMP_WINDOW_SECS: i64 = 60;

type HmacSha256 = Hmac<Sha256>;

fn body_sha256_hex(body: &[u8]) -> String {
    hex::encode(Sha256::digest(body))
}

fn signature_payload(
    device_id: &str,
    timestamp_secs: i64,
    method: &str,
    path: &str,
    body: &[u8],
) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        device_id,
        timestamp_secs,
        method,
        path,
        body_sha256_hex(body)
    )
}

pub fn sign_request(
    session_secret: &[u8; 32],
    device_id: &str,
    timestamp_secs: i64,
    method: &str,
    path: &str,
    body: &[u8],
) -> String {
    let payload = signature_payload(device_id, timestamp_secs, method, path, body);
    let mut mac = HmacSha256::new_from_slice(session_secret).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[allow(clippy::too_many_arguments)]
pub fn verify_request(
    session_secret: &[u8; 32],
    device_id: &str,
    timestamp_secs: i64,
    now_secs: i64,
    method: &str,
    path: &str,
    body: &[u8],
    signature_hex: &str,
) -> Result<(), EngineError> {
    if (timestamp_secs - now_secs).abs() > SIGNATURE_TIMESTAMP_WINDOW_SECS {
        return Err(EngineError::InvalidArg(
            "signature timestamp out of window".into(),
        ));
    }
    let expected = sign_request(
        session_secret,
        device_id,
        timestamp_secs,
        method,
        path,
        body,
    );
    if expected != signature_hex {
        return Err(EngineError::InvalidArg("invalid signature".into()));
    }
    Ok(())
}
