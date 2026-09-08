use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::error::EngineError;
use crate::lan::StreamTokenStore;
use crate::types::LibraryEpisode;

pub type GetEpisodeFn = Arc<dyn Fn(&str) -> Option<LibraryEpisode> + Send + Sync>;

pub struct SenderState {
    pub device_id: String,
    pub stream_token_store: Arc<Mutex<StreamTokenStore>>,
    pub media_dir: PathBuf,
    pub get_episode: GetEpisodeFn,
}

pub struct SenderHttp {
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl SenderHttp {
    pub async fn start(bind: SocketAddr, state: SenderState) -> Result<(Self, u16), EngineError> {
        let listener = TcpListener::bind(bind).await?;
        let port = listener.local_addr()?.port();
        let app_state = AppState {
            sender: Arc::new(state),
        };
        let router = build_router(app_state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router)
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
    sender: Arc<SenderState>,
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/stream/:token", get(stream).head(stream_head))
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    device_id: String,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        device_id: state.sender.device_id.clone(),
    })
}

async fn stream(
    State(state): State<AppState>,
    AxumPath(token): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    stream_response(state, token, headers, false).await
}

async fn stream_head(
    State(state): State<AppState>,
    AxumPath(token): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    stream_response(state, token, headers, true).await
}

async fn stream_response(
    state: AppState,
    token: String,
    headers: HeaderMap,
    head_only: bool,
) -> Result<Response, StatusCode> {
    let path = resolve_stream_path(&state, &token)?;
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let total = metadata.len() as usize;
    let content_type = content_type_for_path(&path);

    if let Some(range) = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        if let Some((start, end)) = parse_range(range, total) {
            let body_len = end - start + 1;
            let builder = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{total}"),
                )
                .header(header::CONTENT_LENGTH, body_len);
            let body = if head_only {
                Body::empty()
            } else {
                let data = tokio::fs::read(&path)
                    .await
                    .map_err(|_| StatusCode::NOT_FOUND)?;
                Body::from(data[start..=end].to_vec())
            };
            return builder
                .body(body)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, total);
    let body = if head_only {
        Body::empty()
    } else {
        let data = tokio::fs::read(&path)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
        Body::from(data)
    };
    builder
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn resolve_stream_path(state: &AppState, token: &str) -> Result<PathBuf, StatusCode> {
    let store = state
        .sender
        .stream_token_store
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .resolve_path(token, now_secs(), &state.sender.media_dir, |episode_id| {
            (state.sender.get_episode)(episode_id)
        })
        .ok_or(StatusCode::NOT_FOUND)
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("mp4") => "video/mp4",
        Some("m4v") => "video/x-m4v",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

fn parse_range(value: &str, total: usize) -> Option<(usize, usize)> {
    let value = value.strip_prefix("bytes=")?;
    let (start_str, end_str) = value.split_once('-')?;
    let start: usize = start_str.parse().ok()?;
    if start >= total {
        return None;
    }
    let end = if end_str.is_empty() {
        total - 1
    } else {
        end_str.parse::<usize>().ok()?.min(total - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
