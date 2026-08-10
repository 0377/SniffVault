use crate::error::EngineError;
use url::Url;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentEntry {
    pub duration: f64,
    pub uri: String,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTag {
    pub method: String,
    pub uri: String,
    pub iv_hex: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq)]
pub struct MediaPlaylist {
    pub segments: Vec<SegmentEntry>,
    pub encryption: Option<KeyTag>,
}

#[derive(Debug, Clone, PartialEq)]
struct VariantEntry {
    bandwidth: u64,
    resolution: Option<String>,
    name: Option<String>,
    uri: String,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn select_media_playlist_url(
    master_body: &str,
    base_url: &str,
    quality_label: Option<&str>,
) -> Result<String, EngineError> {
    let variants = parse_master_variants(master_body)?;
    if variants.is_empty() {
        return Err(EngineError::InvalidArg(
            "not a master playlist: no variants found".into(),
        ));
    }

    let selected = match quality_label {
        None | Some("highest") => variants
            .into_iter()
            .max_by_key(|v| v.bandwidth)
            .expect("variants non-empty"),
        Some(label) => variants
            .into_iter()
            .find(|v| {
                quality_matches_label(
                    v.resolution.as_deref().unwrap_or(""),
                    v.name.as_deref(),
                    label,
                )
            })
            .ok_or_else(|| {
                EngineError::Message(format!("no variant matches quality label '{label}'"))
            })?,
    };

    resolve_url(base_url, &selected.uri)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn quality_matches_label(resolution: &str, name: Option<&str>, label: &str) -> bool {
    if label == "highest" {
        return false;
    }

    if let Some(name) = name {
        if name == label {
            return true;
        }
    }

    if label == "1080p" {
        return resolution_height(resolution).is_some_and(|h| h >= 1080);
    }

    resolution == label
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_media_playlist(body: &str, base_url: &str) -> Result<MediaPlaylist, EngineError> {
    let mut segments = Vec::new();
    let mut encryption: Option<KeyTag> = None;
    let mut pending_duration: Option<f64> = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("#EXT-X-MAP:") {
            return Err(EngineError::Message(
                "fMP4 playlists with #EXT-X-MAP are not supported".into(),
            ));
        }

        if line.starts_with("#EXT-X-KEY:") {
            encryption = Some(parse_key_tag(line)?);
            continue;
        }

        if let Some(duration) = parse_extinf(line) {
            pending_duration = Some(duration);
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        if let Some(duration) = pending_duration.take() {
            segments.push(SegmentEntry {
                duration,
                uri: resolve_url(base_url, line)?,
            });
        }
    }

    if segments.is_empty() {
        return Err(EngineError::InvalidArg(
            "media playlist contains no segments".into(),
        ));
    }

    Ok(MediaPlaylist {
        segments,
        encryption,
    })
}

fn parse_master_variants(body: &str) -> Result<Vec<VariantEntry>, EngineError> {
    let mut variants = Vec::new();
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let bandwidth = parse_bandwidth(attrs)?;
            let resolution = parse_attr(attrs, "RESOLUTION");
            let name = parse_attr(attrs, "NAME");
            index += 1;
            if index >= lines.len() {
                return Err(EngineError::InvalidArg(
                    "master playlist variant missing URI".into(),
                ));
            }
            let uri_line = lines[index];
            if uri_line.starts_with('#') {
                return Err(EngineError::InvalidArg(
                    "master playlist variant missing URI".into(),
                ));
            }
            variants.push(VariantEntry {
                bandwidth,
                resolution,
                name,
                uri: uri_line.to_string(),
            });
        }
        index += 1;
    }

    Ok(variants)
}

fn parse_bandwidth(attrs: &str) -> Result<u64, EngineError> {
    let value = parse_attr(attrs, "BANDWIDTH").ok_or_else(|| {
        EngineError::InvalidArg("master playlist variant missing BANDWIDTH".into())
    })?;
    value
        .parse()
        .map_err(|_| EngineError::InvalidArg(format!("invalid BANDWIDTH value: {value}")))
}

fn parse_attr(attrs: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=");
    for part in attrs.split(',') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&needle) {
            return Some(strip_quotes(value));
        }
    }
    None
}

fn strip_quotes(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn parse_extinf(line: &str) -> Option<f64> {
    let rest = line.strip_prefix("#EXTINF:")?;
    let duration = rest.split(',').next()?.trim();
    duration.parse().ok()
}

fn parse_key_tag(line: &str) -> Result<KeyTag, EngineError> {
    let attrs = line
        .strip_prefix("#EXT-X-KEY:")
        .ok_or_else(|| EngineError::InvalidArg("invalid EXT-X-KEY tag".into()))?;

    let method = parse_attr(attrs, "METHOD")
        .ok_or_else(|| EngineError::InvalidArg("EXT-X-KEY missing METHOD".into()))?;
    let uri = parse_attr(attrs, "URI")
        .ok_or_else(|| EngineError::InvalidArg("EXT-X-KEY missing URI".into()))?;
    let iv_hex = parse_attr(attrs, "IV");

    Ok(KeyTag {
        method,
        uri,
        iv_hex,
    })
}

fn resolution_height(resolution: &str) -> Option<u32> {
    let (_, height) = resolution.split_once('x')?;
    height.parse().ok()
}

fn resolve_url(base_url: &str, reference: &str) -> Result<String, EngineError> {
    let base = Url::parse(base_url)
        .map_err(|err| EngineError::InvalidArg(format!("invalid base URL: {err}")))?;
    base.join(reference)
        .map(|url| url.to_string())
        .map_err(|err| EngineError::InvalidArg(format!("invalid playlist URI: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hls")
    }

    fn read_fixture(name: &str) -> String {
        std::fs::read_to_string(fixtures_dir().join(name)).unwrap()
    }

    #[test]
    fn selects_highest_bandwidth() {
        let master = read_fixture("master.m3u8");
        let base = "http://127.0.0.1/hls/master.m3u8";

        let url = select_media_playlist_url(&master, base, Some("highest")).unwrap();
        assert_eq!(url, "http://127.0.0.1/hls/1080p.m3u8");

        let url = select_media_playlist_url(&master, base, None).unwrap();
        assert_eq!(url, "http://127.0.0.1/hls/1080p.m3u8");
    }

    #[test]
    fn selects_1080p_by_height() {
        let master = read_fixture("master.m3u8");
        let base = "http://127.0.0.1/hls/master.m3u8";

        let url = select_media_playlist_url(&master, base, Some("1080p")).unwrap();
        assert_eq!(url, "http://127.0.0.1/hls/1080p.m3u8");
    }

    #[test]
    fn rejects_ext_x_map() {
        let body = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-MAP:URI="init.mp4"
#EXTINF:6.0,
segment0.m4s
"#;

        let err = parse_media_playlist(body, "http://127.0.0.1/media.m3u8").unwrap_err();
        assert!(matches!(err, EngineError::Message(_)));
        assert!(err
            .to_string()
            .contains("fMP4 playlists with #EXT-X-MAP are not supported"));
    }

    #[test]
    fn parse_media_playlist_resolves_segments() {
        let body = read_fixture("media.m3u8");
        let playlist = parse_media_playlist(&body, "http://127.0.0.1/hls/media.m3u8").unwrap();

        assert_eq!(playlist.segments.len(), 2);
        assert_eq!(playlist.segments[0].duration, 6.0);
        assert_eq!(
            playlist.segments[0].uri,
            "http://127.0.0.1/hls/segments/seg0.ts"
        );
        assert_eq!(
            playlist.segments[1].uri,
            "http://127.0.0.1/hls/segments/seg1.ts"
        );
        assert!(playlist.encryption.is_none());
    }

    #[test]
    fn quality_matches_label_prefers_name() {
        assert!(quality_matches_label("1280x720", Some("1080p"), "1080p"));
        assert!(quality_matches_label("1920x1080", None, "1080p"));
        assert!(!quality_matches_label("1280x720", None, "1080p"));
    }
}
