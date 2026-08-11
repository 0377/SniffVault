use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use bytes::Bytes;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_stream::wrappers::ReceiverStream;

pub struct ServerGuard(tokio::task::JoinHandle<()>);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

pub async fn serve_dir(root: PathBuf) -> (SocketAddr, ServerGuard) {
    let router = Router::new().fallback(any(serve_file)).with_state(root);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (addr, ServerGuard(handle))
}

async fn serve_file(State(root): State<PathBuf>, request: Request) -> Response {
    let method = request.method().clone();
    let path = request.uri().path();
    let rel = path.trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") {
        return StatusCode::NOT_FOUND.into_response();
    }

    let file_path = root.join(rel);
    if !file_path.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let data = match tokio::fs::read(&file_path).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let total = data.len();
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, total.to_string());

    if method == Method::HEAD {
        response.body(Body::empty()).unwrap()
    } else {
        response.body(Body::from(data)).unwrap()
    }
}

#[allow(dead_code)]
fn throttled_response(
    data: Vec<u8>,
    chunk_size: usize,
    delay: Duration,
    status: StatusCode,
    extra_headers: Vec<(header::HeaderName, String)>,
) -> Response {
    let chunk_size = chunk_size.max(1);
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(4);
    tokio::spawn(async move {
        for chunk in data.chunks(chunk_size) {
            if tx.send(Ok(Bytes::copy_from_slice(chunk))).await.is_err() {
                break;
            }
            if delay > Duration::ZERO {
                tokio::time::sleep(delay).await;
            }
        }
    });
    let mut builder = Response::builder().status(status);
    for (name, value) in extra_headers {
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap()
}
