use reqwest::{Client, Response};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::error::EngineError;

#[cfg_attr(not(test), allow(dead_code))]
const MAX_ATTEMPTS: u32 = 4;
#[cfg_attr(not(test), allow(dead_code))]
const RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct HttpClient {
    client: Client,
    cancel: CancellationToken,
}

#[cfg_attr(not(test), allow(dead_code))]
impl HttpClient {
    pub fn new(user_agent: Option<&str>) -> Result<Self, EngineError> {
        let mut builder = Client::builder().timeout(Duration::from_secs(30));
        if let Some(ua) = user_agent {
            builder = builder.user_agent(ua);
        }
        let client = builder.build()?;
        Ok(Self {
            client,
            cancel: CancellationToken::new(),
        })
    }

    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancel = token;
        self
    }

    pub async fn get_text(&self, url: &str) -> Result<String, EngineError> {
        let response = self
            .execute_with_retry(|| self.client.get(url).send())
            .await?;
        Ok(response.text().await?)
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, EngineError> {
        let response = self
            .execute_with_retry(|| self.client.get(url).send())
            .await?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_range(
        &self,
        url: &str,
        start: u64,
        end: Option<u64>,
    ) -> Result<Vec<u8>, EngineError> {
        let range_value = match end {
            Some(end) => format!("bytes={start}-{end}"),
            None => format!("bytes={start}-"),
        };
        let response = self
            .execute_with_retry(|| {
                self.client
                    .get(url)
                    .header(reqwest::header::RANGE, range_value.clone())
                    .send()
            })
            .await?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn head_size_and_ranges(
        &self,
        url: &str,
    ) -> Result<(Option<u64>, bool), EngineError> {
        let response = self
            .execute_with_retry(|| self.client.head(url).send())
            .await?;
        let size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok());
        let supports_ranges = response
            .headers()
            .get(reqwest::header::ACCEPT_RANGES)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("bytes"));
        Ok((size, supports_ranges))
    }

    pub(crate) async fn get_stream(&self, url: &str) -> Result<Response, EngineError> {
        self.execute_with_retry(|| self.client.get(url).send())
            .await
    }

    pub(crate) async fn get_stream_range(
        &self,
        url: &str,
        start: u64,
    ) -> Result<Response, EngineError> {
        let range_value = format!("bytes={start}-");
        self.execute_with_retry(|| {
            self.client
                .get(url)
                .header(reqwest::header::RANGE, range_value.clone())
                .send()
        })
        .await
    }

    async fn execute_with_retry<F, Fut>(&self, mut send: F) -> Result<Response, EngineError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
    {
        let mut attempts = 0u32;
        loop {
            if self.cancel.is_cancelled() {
                return Err(EngineError::Message("download cancelled".into()));
            }

            match send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_client_error() {
                        return Err(response.error_for_status().unwrap_err().into());
                    }
                    if status.is_server_error() {
                        attempts += 1;
                        if attempts >= MAX_ATTEMPTS {
                            return Err(response.error_for_status().unwrap_err().into());
                        }
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                    return Ok(response);
                }
                Err(err) => {
                    let engine_err = EngineError::from(err);
                    if is_retryable(&engine_err) {
                        attempts += 1;
                        if attempts >= MAX_ATTEMPTS {
                            return Err(engine_err);
                        }
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                    return Err(engine_err);
                }
            }
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn is_retryable(err: &EngineError) -> bool {
    match err {
        EngineError::Http(err) => {
            if err.is_timeout() || err.is_connect() || err.is_request() {
                return true;
            }
            err.status().is_some_and(|status| status.is_server_error())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::State,
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, head},
        Router,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::net::TcpListener;

    struct ServerGuard(tokio::task::JoinHandle<()>);

    impl Drop for ServerGuard {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    #[derive(Clone)]
    struct HitCounter(Arc<AtomicUsize>);

    async fn spawn_server(router: Router) -> (String, ServerGuard) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (base_url, ServerGuard(handle))
    }

    async fn text_handler() -> &'static str {
        "hello-http"
    }

    async fn bytes_handler() -> &'static [u8] {
        b"byte-payload"
    }

    async fn range_handler(headers: HeaderMap) -> impl IntoResponse {
        let body = b"0123456789";
        if let Some(range) = headers.get("range").and_then(|value| value.to_str().ok()) {
            if range == "bytes=2-5" {
                return (StatusCode::PARTIAL_CONTENT, &body[2..=5]);
            }
        }
        (StatusCode::OK, body.as_slice())
    }

    async fn head_handler() -> impl IntoResponse {
        (
            StatusCode::OK,
            [
                (reqwest::header::CONTENT_LENGTH, "42"),
                (reqwest::header::ACCEPT_RANGES, "bytes"),
            ],
        )
    }

    async fn not_found_handler(State(counter): State<HitCounter>) -> StatusCode {
        counter.0.fetch_add(1, Ordering::SeqCst);
        StatusCode::NOT_FOUND
    }

    async fn flaky_handler(State(counter): State<HitCounter>) -> impl IntoResponse {
        let attempt = counter.0.fetch_add(1, Ordering::SeqCst);
        if attempt < 2 {
            return (StatusCode::SERVICE_UNAVAILABLE, "retry me");
        }
        (StatusCode::OK, "recovered")
    }

    #[tokio::test]
    async fn get_text_returns_body() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/text", get(text_handler))).await;
        let client = HttpClient::new(None).unwrap();

        let text = client.get_text(&format!("{base_url}/text")).await.unwrap();

        assert_eq!(text, "hello-http");
    }

    #[tokio::test]
    async fn get_bytes_returns_payload() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/bytes", get(bytes_handler))).await;
        let client = HttpClient::new(None).unwrap();

        let bytes = client
            .get_bytes(&format!("{base_url}/bytes"))
            .await
            .unwrap();

        assert_eq!(bytes, b"byte-payload");
    }

    #[tokio::test]
    async fn get_range_returns_partial_content() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/range", get(range_handler))).await;
        let client = HttpClient::new(None).unwrap();

        let bytes = client
            .get_range(&format!("{base_url}/range"), 2, Some(5))
            .await
            .unwrap();

        assert_eq!(bytes, b"2345");
    }

    #[tokio::test]
    async fn head_size_and_ranges_reads_headers() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/head", head(head_handler))).await;
        let client = HttpClient::new(None).unwrap();

        let (size, supports_ranges) = client
            .head_size_and_ranges(&format!("{base_url}/head"))
            .await
            .unwrap();

        assert_eq!(size, Some(42));
        assert!(supports_ranges);
    }

    #[tokio::test]
    async fn client_error_is_not_retried() {
        let counter = HitCounter(Arc::new(AtomicUsize::new(0)));
        let (base_url, _guard) = spawn_server(
            Router::new()
                .route("/missing", get(not_found_handler))
                .with_state(counter.clone()),
        )
        .await;
        let client = HttpClient::new(None).unwrap();

        let err = client
            .get_text(&format!("{base_url}/missing"))
            .await
            .unwrap_err();

        assert!(matches!(err, EngineError::Http(_)));
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn server_error_is_retried() {
        let counter = HitCounter(Arc::new(AtomicUsize::new(0)));
        let (base_url, _guard) = spawn_server(
            Router::new()
                .route("/flaky", get(flaky_handler))
                .with_state(counter.clone()),
        )
        .await;
        let client = HttpClient::new(None).unwrap();

        let text = client.get_text(&format!("{base_url}/flaky")).await.unwrap();

        assert_eq!(text, "recovered");
        assert_eq!(counter.0.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn get_stream_returns_successful_response() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/text", get(text_handler))).await;
        let client = HttpClient::new(None).unwrap();

        let response = client
            .get_stream(&format!("{base_url}/text"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "hello-http");
    }

    #[tokio::test]
    async fn cancellation_stops_retry_loop() {
        let counter = HitCounter(Arc::new(AtomicUsize::new(0)));
        let (base_url, _guard) = spawn_server(
            Router::new()
                .route("/flaky", get(flaky_handler))
                .with_state(counter.clone()),
        )
        .await;
        let token = CancellationToken::new();
        let client = HttpClient::new(None)
            .unwrap()
            .with_cancellation(token.clone());
        token.cancel();

        let err = client
            .get_text(&format!("{base_url}/flaky"))
            .await
            .unwrap_err();

        assert!(matches!(err, EngineError::Message(_)));
        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
    }
}
