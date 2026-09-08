use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Bytes,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::error::EngineError;
use crate::lan::{
    verify_pin, verify_request, CastEvent, CastPlayRequest, PairingSession, TrustStore, TrustedPeer,
};

const HDR_DEVICE_ID: &str = "x-sniffvault-device-id";
const HDR_TIMESTAMP: &str = "x-sniffvault-timestamp";
const HDR_SIGNATURE: &str = "x-sniffvault-signature";

#[derive(Clone)]
pub struct ReceiverState {
    pub device_id: String,
    pub trust_store: Arc<Mutex<TrustStore>>,
    pub pairing_session: Arc<Mutex<Option<PairingSession>>>,
    pub event_tx: mpsc::Sender<CastEvent>,
}

pub struct ReceiverHttp {
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl ReceiverHttp {
    pub async fn start(bind: SocketAddr, state: ReceiverState) -> Result<(Self, u16), EngineError> {
        let listener = TcpListener::bind(bind).await?;
        let port = listener.local_addr()?.port();
        let app_state = AppState { receiver: state };
        let router = build_router(app_state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
        });
        Ok((
            Self {
                shutdown_tx: Some(shutdown_tx),
                handle,
            },
            port,
        ))
    }

    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

#[derive(Clone)]
struct AppState {
    receiver: ReceiverState,
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/pair", post(pair))
        .route("/v1/cast/play", post(cast_play))
        .route("/v1/cast/stop", post(cast_stop))
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    device_id: String,
}

#[derive(Deserialize)]
struct PairRequest {
    device_id: String,
    device_name: String,
    pin: String,
}

#[derive(Serialize)]
struct PairResponse {
    session_secret: String,
}

#[derive(Serialize)]
struct CastPlayResponse {
    session_id: String,
}

#[derive(Deserialize)]
struct CastStopRequest {
    session_id: String,
}

#[derive(Serialize)]
struct OkResponse {
    ok: bool,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        device_id: state.receiver.device_id.clone(),
    })
}

async fn pair(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<PairRequest>,
) -> Result<Json<PairResponse>, StatusCode> {
    let now_ms = now_ms();
    let session = {
        let guard = state
            .receiver
            .pairing_session
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.clone()
    };
    let session = session.ok_or(StatusCode::BAD_REQUEST)?;
    verify_pin(&session, &body.pin, now_ms).map_err(|_| StatusCode::BAD_REQUEST)?;

    let session_secret = random_session_secret();
    let peer = TrustedPeer {
        peer_device_id: body.device_id,
        peer_name: body.device_name,
        peer_host: addr.ip().to_string(),
        peer_port: addr.port(),
        paired_at_ms: now_ms,
        session_secret: Some(session_secret),
    };
    state
        .receiver
        .trust_store
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .upsert_peer(&peer)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PairResponse {
        session_secret: hex::encode(session_secret),
    }))
}

async fn cast_play(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<CastPlayResponse>, StatusCode> {
    let (device_id, timestamp_secs, signature) = parse_signed_headers(&headers)?;
    let trusted = lookup_trusted_peer(&state, &device_id)?;
    let secret = trusted.session_secret.ok_or(StatusCode::FORBIDDEN)?;
    verify_request(
        &secret,
        &device_id,
        timestamp_secs,
        now_secs(),
        "POST",
        "/v1/cast/play",
        &body,
        &signature,
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let request: CastPlayRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    if request.sender_device_id != device_id {
        return Err(StatusCode::FORBIDDEN);
    }

    refresh_peer_host(&state, &trusted, addr)?;

    let session_id = request.session_id.clone();
    state
        .receiver
        .event_tx
        .send(CastEvent::IncomingPlay { request })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CastPlayResponse { session_id }))
}

async fn cast_stop(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<OkResponse>, StatusCode> {
    let (device_id, timestamp_secs, signature) = parse_signed_headers(&headers)?;
    let trusted = lookup_trusted_peer(&state, &device_id)?;
    let secret = trusted.session_secret.ok_or(StatusCode::FORBIDDEN)?;
    verify_request(
        &secret,
        &device_id,
        timestamp_secs,
        now_secs(),
        "POST",
        "/v1/cast/stop",
        &body,
        &signature,
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let request: CastStopRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    refresh_peer_host(&state, &trusted, addr)?;

    state
        .receiver
        .event_tx
        .send(CastEvent::SessionEnded {
            session_id: request.session_id,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(OkResponse { ok: true }))
}

fn lookup_trusted_peer(state: &AppState, device_id: &str) -> Result<TrustedPeer, StatusCode> {
    let store = state
        .receiver
        .trust_store
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let peer = store
        .list_peers()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|peer| peer.peer_device_id == device_id)
        .ok_or(StatusCode::FORBIDDEN)?;
    Ok(peer)
}

fn refresh_peer_host(
    state: &AppState,
    trusted: &TrustedPeer,
    addr: SocketAddr,
) -> Result<(), StatusCode> {
    let updated = TrustedPeer {
        peer_device_id: trusted.peer_device_id.clone(),
        peer_name: trusted.peer_name.clone(),
        peer_host: addr.ip().to_string(),
        peer_port: addr.port(),
        paired_at_ms: trusted.paired_at_ms,
        session_secret: trusted.session_secret,
    };
    state
        .receiver
        .trust_store
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .upsert_peer(&updated)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

fn parse_signed_headers(headers: &HeaderMap) -> Result<(String, i64, String), StatusCode> {
    let device_id = header_value(headers, HDR_DEVICE_ID).ok_or(StatusCode::UNAUTHORIZED)?;
    let timestamp_raw = header_value(headers, HDR_TIMESTAMP).ok_or(StatusCode::UNAUTHORIZED)?;
    let timestamp_secs = timestamp_raw
        .parse::<i64>()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let signature = header_value(headers, HDR_SIGNATURE).ok_or(StatusCode::UNAUTHORIZED)?;
    Ok((device_id, timestamp_secs, signature))
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn random_session_secret() -> [u8; 32] {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut secret = [0u8; 32];
    secret[..16].copy_from_slice(a.as_bytes());
    secret[16..].copy_from_slice(b.as_bytes());
    secret
}

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
