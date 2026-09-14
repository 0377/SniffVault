use std::io::Read;
use std::path::Path;

use crate::error::EngineError;

pub(crate) const POSTER_MAX_BYTES: usize = 5 * 1024 * 1024;

pub(crate) fn download_poster(
    http: &reqwest::blocking::Client,
    media_dir: &Path,
    item_id: &str,
    poster_url: &str,
    user_agent: Option<&str>,
) -> Result<String, EngineError> {
    let parsed = url::Url::parse(poster_url)
        .map_err(|e| EngineError::InvalidArg(format!("invalid poster url: {e}")))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(EngineError::InvalidArg("poster url must be http(s)".into()));
    }
    let mut req = http.get(poster_url);
    if let Some(ua) = user_agent {
        req = req.header(reqwest::header::USER_AGENT, ua);
    }
    let mut resp = req
        .send()
        .map_err(|e| EngineError::Message(format!("poster download failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(EngineError::Message(format!(
            "poster http {}",
            resp.status()
        )));
    }
    if let Some(len) = resp.content_length() {
        if len > POSTER_MAX_BYTES as u64 {
            return Err(EngineError::InvalidArg("poster exceeds 5 MiB".into()));
        }
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let ext = ext_from_content_type_or_url(ct, poster_url);
    let posters_dir = media_dir.join(".posters");
    std::fs::create_dir_all(&posters_dir)?;
    let dest = posters_dir.join(format!("{item_id}.{ext}"));
    let bytes = read_poster_body(&mut resp)?;
    std::fs::write(&dest, &bytes)?;
    let canon = dest
        .canonicalize()
        .map_err(|e| EngineError::InvalidArg(format!("poster path: {e}")))?;
    Ok(canon.to_string_lossy().into())
}

fn read_poster_body(resp: &mut reqwest::blocking::Response) -> Result<Vec<u8>, EngineError> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let n = resp
            .read(&mut chunk)
            .map_err(|e| EngineError::Message(format!("poster body: {e}")))?;
        if n == 0 {
            break;
        }
        if bytes.len() + n > POSTER_MAX_BYTES {
            return Err(EngineError::InvalidArg("poster exceeds 5 MiB".into()));
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    Ok(bytes)
}

fn ext_from_content_type_or_url(content_type: &str, url: &str) -> &'static str {
    if content_type.contains("image/png") {
        return "png";
    }
    if content_type.contains("image/webp") {
        return "webp";
    }
    if content_type.contains("image/jpeg") || content_type.contains("image/jpg") {
        return "jpg";
    }
    let lower = url.to_ascii_lowercase();
    if lower.ends_with(".png") {
        return "png";
    }
    if lower.ends_with(".webp") {
        return "webp";
    }
    "jpg"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn ext_from_content_type_png() {
        assert_eq!(
            ext_from_content_type_or_url("image/png", "https://x/p"),
            "png"
        );
    }

    #[test]
    fn ext_from_content_type_webp() {
        assert_eq!(
            ext_from_content_type_or_url("image/webp", "https://x/p"),
            "webp"
        );
    }

    #[test]
    fn ext_from_content_type_jpeg() {
        assert_eq!(
            ext_from_content_type_or_url("image/jpeg", "https://x/p"),
            "jpg"
        );
    }

    #[test]
    fn ext_from_url_suffix_when_no_content_type() {
        assert_eq!(
            ext_from_content_type_or_url("", "https://x/cover.png"),
            "png"
        );
        assert_eq!(
            ext_from_content_type_or_url("", "https://x/cover.webp"),
            "webp"
        );
    }

    #[test]
    fn ext_defaults_to_jpg() {
        assert_eq!(ext_from_content_type_or_url("", "https://x/poster"), "jpg");
    }

    #[test]
    fn rejects_non_http_scheme() {
        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::blocking::Client::new();
        let err =
            download_poster(&client, dir.path(), "item-1", "file:///tmp/x.jpg", None).unwrap_err();
        assert!(matches!(err, EngineError::InvalidArg(_)));
    }

    fn spawn_jpeg_server(jpeg_bytes: &[u8]) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = jpeg_bytes.to_vec();
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        (format!("http://127.0.0.1:{}/sample.jpg", port), handle)
    }

    #[test]
    fn download_poster_writes() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters/sample.jpg");
        let jpeg_bytes = std::fs::read(&fixture).unwrap();
        let (url, server) = spawn_jpeg_server(&jpeg_bytes);

        let dir = tempfile::tempdir().unwrap();
        let media_dir = dir.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();

        let client = reqwest::blocking::Client::new();
        let path = download_poster(&client, &media_dir, "item-abc", &url, Some("test-ua")).unwrap();

        server.join().unwrap();

        let expected = media_dir
            .join(".posters")
            .join("item-abc.jpg")
            .canonicalize()
            .unwrap();
        assert_eq!(path, expected.to_string_lossy());
        assert!(expected.exists());
        assert_eq!(std::fs::read(&expected).unwrap(), jpeg_bytes);
    }

    #[test]
    fn download_poster_rejects_oversized_body() {
        let oversized = vec![0u8; POSTER_MAX_BYTES + 1];
        let (url, server) = spawn_jpeg_server(&oversized);

        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::blocking::Client::new();
        let err = download_poster(&client, dir.path(), "item-big", &url, None).unwrap_err();

        server.join().unwrap();

        assert!(matches!(err, EngineError::InvalidArg(_)));
    }
}
