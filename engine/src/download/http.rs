use reqwest::{Client, Response, StatusCode};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::error::EngineError;

#[cfg_attr(not(test), allow(dead_code))]
const MAX_ATTEMPTS: u32 = 4;
#[cfg_attr(not(test), allow(dead_code))]
const RETRY_DELAY: Duration = Duration::from_millis(50);

pub(crate) const MAX_PAGE_BYTES: usize = 8 * 1024 * 1024;

pub(crate) struct PageFetchOptions {
    pub cookies: Option<String>,
    pub referer: Option<String>,
}

#[derive(Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct HttpClient {
    client: Client,
    cancel: CancellationToken,
}

#[cfg_attr(not(test), allow(dead_code))]
impl HttpClient {
    pub fn new(user_agent: Option<&str>) -> Result<Self, EngineError> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10));
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

    pub async fn get_page_text(
        &self,
        url: &str,
        opts: &PageFetchOptions,
    ) -> Result<(StatusCode, String), EngineError> {
        let url = url.to_string();
        let cookies = opts.cookies.clone();
        let referer = opts.referer.clone();
        let mut attempts = 0u32;
        loop {
            if self.cancel.is_cancelled() {
                return Err(EngineError::Message("download cancelled".into()));
            }

            let mut request = self.client.get(&url);
            if let Some(cookies) = &cookies {
                request = request.header(reqwest::header::COOKIE, cookies);
            }
            if let Some(referer) = &referer {
                request = request.header(reqwest::header::REFERER, referer);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_server_error() {
                        attempts += 1;
                        if attempts >= MAX_ATTEMPTS {
                            return Err(response.error_for_status().unwrap_err().into());
                        }
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                    let bytes = response.bytes().await?;
                    if bytes.len() > MAX_PAGE_BYTES {
                        return Err(EngineError::InvalidArg(
                            "page body exceeds 8MB limit".into(),
                        ));
                    }
                    let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
                        EngineError::InvalidArg("page body is not valid utf-8".into())
                    })?;
                    return Ok((status, text));
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

    async fn auth_handler(headers: HeaderMap) -> impl IntoResponse {
        if headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("sid=ok"))
        {
            return (StatusCode::OK, "authenticated-body");
        }
        (StatusCode::FORBIDDEN, "forbidden")
    }

    #[tokio::test]
    async fn get_page_text_sends_cookie_and_returns_status() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/auth", get(auth_handler))).await;
        let client = HttpClient::new(None).unwrap();
        let (status, body) = client
            .get_page_text(
                &format!("{base_url}/auth"),
                &PageFetchOptions {
                    cookies: Some("sid=ok".into()),
                    referer: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "authenticated-body");
    }

    #[tokio::test]
    async fn get_page_text_without_cookie_gets_forbidden() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/auth", get(auth_handler))).await;
        let client = HttpClient::new(None).unwrap();
        let (status, _) = client
            .get_page_text(
                &format!("{base_url}/auth"),
                &PageFetchOptions {
                    cookies: None,
                    referer: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    async fn huge_body_handler() -> impl IntoResponse {
        (StatusCode::OK, vec![0u8; MAX_PAGE_BYTES + 1])
    }

    async fn referer_handler(headers: HeaderMap) -> impl IntoResponse {
        let referer = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("none")
            .to_string();
        (StatusCode::OK, referer)
    }

    async fn page_flaky_handler(State(counter): State<HitCounter>) -> impl IntoResponse {
        let attempt = counter.0.fetch_add(1, Ordering::SeqCst);
        if attempt < 2 {
            return (StatusCode::SERVICE_UNAVAILABLE, "retry me");
        }
        (StatusCode::OK, "page-recovered")
    }

    async fn redirect_chain_handler(uri: axum::http::Uri) -> axum::response::Response {
        let depth = uri
            .query()
            .and_then(|q| q.strip_prefix("d="))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if depth >= 12 {
            return (StatusCode::OK, "done").into_response();
        }
        let next = depth + 1;
        (
            StatusCode::TEMPORARY_REDIRECT,
            [(reqwest::header::LOCATION, format!("/chain?d={next}"))],
        )
            .into_response()
    }

    #[tokio::test]
    async fn get_page_text_rejects_body_over_8mb() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/huge", get(huge_body_handler))).await;
        let client = HttpClient::new(None).unwrap();
        let err = client
            .get_page_text(
                &format!("{base_url}/huge"),
                &PageFetchOptions {
                    cookies: None,
                    referer: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArg(_)));
    }

    #[tokio::test]
    async fn get_page_text_sends_referer_header() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/referer", get(referer_handler))).await;
        let client = HttpClient::new(None).unwrap();
        let (status, body) = client
            .get_page_text(
                &format!("{base_url}/referer"),
                &PageFetchOptions {
                    cookies: None,
                    referer: Some("http://ref.example/page".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "http://ref.example/page");
    }

    #[tokio::test]
    async fn get_page_text_retries_server_errors() {
        let counter = HitCounter(Arc::new(AtomicUsize::new(0)));
        let (base_url, _guard) = spawn_server(
            Router::new()
                .route("/page-flaky", get(page_flaky_handler))
                .with_state(counter.clone()),
        )
        .await;
        let client = HttpClient::new(None).unwrap();
        let (status, body) = client
            .get_page_text(
                &format!("{base_url}/page-flaky"),
                &PageFetchOptions {
                    cookies: None,
                    referer: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "page-recovered");
        assert_eq!(counter.0.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn get_page_text_rejects_redirect_chain_over_limit() {
        let (base_url, _guard) =
            spawn_server(Router::new().route("/chain", get(redirect_chain_handler))).await;
        let client = HttpClient::new(None).unwrap();
        let err = client
            .get_page_text(
                &format!("{base_url}/chain"),
                &PageFetchOptions {
                    cookies: None,
                    referer: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::Http(_)));
    }

    #[tokio::test]
    async fn get_page_text_cancellation_stops_fetch() {
        let counter = HitCounter(Arc::new(AtomicUsize::new(0)));
        let (base_url, _guard) = spawn_server(
            Router::new()
                .route("/page-flaky", get(page_flaky_handler))
                .with_state(counter.clone()),
        )
        .await;
        let token = CancellationToken::new();
        let client = HttpClient::new(None)
            .unwrap()
            .with_cancellation(token.clone());
        token.cancel();
        let err = client
            .get_page_text(
                &format!("{base_url}/page-flaky"),
                &PageFetchOptions {
                    cookies: None,
                    referer: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::Message(_)));
        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
    }
}
