use crate::error::EngineError;
use uuid::Uuid;

pub const PAIRING_PIN_TTL_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingSession {
    pub pin: String,
    pub expires_at_ms: i64,
    pub pairing_nonce: [u8; 16],
}

pub fn begin_pairing(now_ms: i64) -> PairingSession {
    let id = Uuid::new_v4();
    let bytes = *id.as_bytes();
    let pin_value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    PairingSession {
        pin: format!("{pin_value:06}"),
        expires_at_ms: now_ms + PAIRING_PIN_TTL_MS,
        pairing_nonce: bytes,
    }
}

pub fn verify_pin(session: &PairingSession, pin: &str, now_ms: i64) -> Result<(), EngineError> {
    if now_ms > session.expires_at_ms {
        return Err(EngineError::InvalidArg("pairing pin expired".into()));
    }
    if pin != session.pin {
        return Err(EngineError::InvalidArg("pairing pin incorrect".into()));
    }
    Ok(())
}
