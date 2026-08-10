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

#[derive(Clone, Default)]
pub struct ServeOptions {
    pub chunk_size: usize,
    pub chunk_delay: Duration,
}

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
    serve_dir_with_options(root, ServeOptions::default()).await
}

#[allow(dead_code)]
pub async fn serve_dir_throttled(
    root: PathBuf,
    chunk_size: usize,
    chunk_delay: Duration,
) -> (SocketAddr, ServerGuard) {
    serve_dir_with_options(
        root,
        ServeOptions {
            chunk_size,
            chunk_delay,
        },
    )
    .await
}

#[allow(dead_code)]
pub async fn serve_dir_with_options(
    root: PathBuf,
    options: ServeOptions,
) -> (SocketAddr, ServerGuard) {
    let router = Router::new()
        .fallback(any(serve_file))
        .with_state((root, options));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (addr, ServerGuard(handle))
}

async fn serve_file(
    State((root, options)): State<(PathBuf, ServeOptions)>,
    request: Request,
) -> Response {
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
            if options.chunk_delay > Duration::ZERO {
                return throttled_response(
                    slice,
                    options,
                    StatusCode::PARTIAL_CONTENT,
                    vec![
                        (header::ACCEPT_RANGES, "bytes".to_string()),
                        (header::CONTENT_LENGTH, len.to_string()),
                        (
                            header::CONTENT_RANGE,
                            format!("bytes {start}-{end}/{total}"),
                        ),
                    ],
                );
            }
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
    } else if options.chunk_delay > Duration::ZERO {
        throttled_response(
            data,
            options,
            StatusCode::OK,
            vec![
                (header::ACCEPT_RANGES, "bytes".to_string()),
                (header::CONTENT_LENGTH, total.to_string()),
            ],
        )
    } else {
        response.body(Body::from(data)).unwrap()
    }
}

fn throttled_response(
    data: Vec<u8>,
    options: ServeOptions,
    status: StatusCode,
    extra_headers: Vec<(header::HeaderName, String)>,
) -> Response {
    let chunk_size = options.chunk_size.max(1);
    let delay = options.chunk_delay;
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
