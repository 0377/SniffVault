use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

pub struct ServerGuard(tokio::task::JoinHandle<()>);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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
    let headers = request.headers().clone();
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
    if let Some(range) = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        if let Some((start, end)) = parse_range(range, total) {
            let slice = data[start..=end].to_vec();
            let len = slice.len();
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, len.to_string())
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{total}"),
                )
                .body(Body::from(slice))
                .unwrap();
        }
    }

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
