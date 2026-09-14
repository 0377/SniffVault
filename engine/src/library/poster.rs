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
    let resp = req
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
    let bytes = read_poster_body(resp)?;
    std::fs::write(&dest, &bytes)?;
    let canon = dest
        .canonicalize()
        .map_err(|e| EngineError::InvalidArg(format!("poster path: {e}")))?;
    Ok(canon.to_string_lossy().into())
}

fn read_poster_body(resp: reqwest::blocking::Response) -> Result<Vec<u8>, EngineError> {
    let bytes = resp
        .bytes()
        .map_err(|e| EngineError::Message(format!("poster body: {e}")))?;
    if bytes.len() > POSTER_MAX_BYTES {
        return Err(EngineError::InvalidArg("poster exceeds 5 MiB".into()));
    }
    Ok(bytes.to_vec())
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
}
