use std::collections::{HashSet};
use std::sync::OnceLock;

use regex::Regex;
use url::Url;

use crate::types::{Episode, EpisodeList};

pub fn parse_episode_index(text: &str, href: &str) -> Option<u32> {
    if let Some(caps) = ep_text_re().captures(text) {
        if let Some(index) = caps.get(1) {
            if let Ok(n) = index.as_str().parse::<u32>() {
                return Some(n);
            }
        }
    }

    for re in href_index_res() {
        if let Some(caps) = re.captures(href) {
            if let Some(index) = caps.get(1) {
                if let Ok(n) = index.as_str().parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }

    None
}

pub fn extract_episode_list(
    html: &str,
    base_url: &str,
    default_title: &str,
) -> Option<EpisodeList> {
    let base = Url::parse(base_url).ok()?;
    let mut seen_urls = HashSet::new();
    let mut seen_indices = HashSet::new();
    let mut episodes = Vec::new();

    for caps in link_re().captures_iter(html) {
        let href = caps.get(1)?.as_str();
        let text = caps
            .get(2)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .trim();

        if is_blacklisted_href(href) {
            continue;
        }

        let index = parse_episode_index(text, href)?;
        if !seen_indices.insert(index) {
            continue;
        }

        let resolved = resolve_href(href, &base)?;
        let canonical = canonical_url(&resolved)?;
        if !seen_urls.insert(canonical) {
            continue;
        }

        episodes.push(Episode {
            index,
            title: text.to_string(),
            url: resolved,
            quality_options: Vec::new(),
        });
    }

    if episodes.len() < 2 {
        return None;
    }

    Some(EpisodeList {
        title: default_title.to_string(),
        season: None,
        episodes,
    })
}

pub fn scan_media_urls(html: &str, base_url: &str) -> Vec<String> {
    let base = match Url::parse(base_url) {
        Ok(url) => url,
        Err(_) => return Vec::new(),
    };

    let mut seen = HashSet::new();
    let mut urls = Vec::new();

    for matched in abs_media_re().find_iter(html) {
        let url = matched.as_str().to_string();
        if seen.insert(url.clone()) {
            urls.push(url);
        }
    }

    for caps in rel_media_re().captures_iter(html) {
        let Some(path) = caps.get(1) else {
            continue;
        };
        let path = path.as_str();
        if path.starts_with("http://") || path.starts_with("https://") {
            continue;
        }
        if let Ok(joined) = base.join(path) {
            let url = joined.to_string();
            if seen.insert(url.clone()) {
                urls.push(url);
            }
        }
    }

    urls
}

fn is_blacklisted_href(href: &str) -> bool {
    let trimmed = href.trim();
    if trimmed.is_empty() || trimmed == "#" || trimmed.starts_with('#') {
        return true;
    }
    if trimmed.starts_with("javascript:") {
        return true;
    }

    let path = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Url::parse(trimmed)
            .ok()
            .map(|url| url.path().to_string())
    } else {
        Some(trimmed.to_string())
    };

    matches!(path, Some(ref p) if p.starts_with("/login") || p.starts_with("/search"))
}

fn resolve_href(href: &str, base: &Url) -> Option<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }
    base.join(href).ok().map(|url| url.to_string())
}

fn canonical_url(url: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    parsed.set_fragment(None);
    Some(parsed.to_string())
}

fn link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<a\s+[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap()
    })
}

fn ep_text_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"第(\d+)集").unwrap())
}

fn href_index_res() -> &'static [Regex] {
    static RES: OnceLock<Vec<Regex>> = OnceLock::new();
    RES.get_or_init(|| {
        [
            r"/ep/(\d+)",
            r"ep[=/](\d+)",
            r"episode[=/](\d+)",
            r"[?&]ep=(\d+)",
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).unwrap())
        .collect()
    })
}

fn abs_media_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"https?://[^\s"'<>]+\.(?:m3u8|mp4)"#).unwrap())
}

fn rel_media_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"["']([^"']+\.(?:m3u8|mp4))["']"#).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_episode_list_from_fixture_shape() {
        let html = include_str!("../../tests/fixtures/html/series_page.html");
        let list = extract_episode_list(html, "http://127.0.0.1/series", "测试剧")
            .expect("episode list");
        assert_eq!(list.title, "测试剧");
        assert_eq!(list.season, None);
        assert_eq!(list.episodes.len(), 3);
        assert_eq!(list.episodes[0].index, 1);
        assert!(list.episodes[0].url.contains("/ep/1"));
        assert!(list.episodes[0].quality_options.is_empty());
    }

    #[test]
    fn scan_media_urls_finds_m3u8_in_script() {
        let html = include_str!("../../tests/fixtures/html/embedded_m3u8.html");
        let urls = scan_media_urls(html, "http://127.0.0.1/page");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "http://127.0.0.1/master.m3u8");
    }

    #[test]
    fn parse_episode_index_from_text_and_href() {
        assert_eq!(parse_episode_index("第2集 发展", "/watch"), Some(2));
        assert_eq!(parse_episode_index("开端", "/ep/3"), Some(3));
        assert_eq!(parse_episode_index("unknown", "/login"), None);
    }

    #[test]
    fn extract_episode_list_skips_blacklisted_links() {
        let html = r#"
        <a href="/ep/1">第1集</a>
        <a href="/login">第2集</a>
        <a href="/ep/3">第3集</a>
        "#;
        let list = extract_episode_list(html, "http://example.com/", "剧")
            .expect("episode list");
        assert_eq!(list.episodes.len(), 2);
        assert_eq!(list.episodes[0].index, 1);
        assert_eq!(list.episodes[1].index, 3);
    }

    #[test]
    fn extract_episode_list_requires_at_least_two_episodes() {
        let html = r#"<a href="/ep/1">第1集</a>"#;
        assert!(extract_episode_list(html, "http://example.com/", "剧").is_none());
    }

    #[test]
    fn scan_media_urls_deduplicates() {
        let html = r#"
        <script>var a="https://cdn.example.com/v.m3u8";</script>
        <script>var b="https://cdn.example.com/v.m3u8";</script>
        "#;
        let urls = scan_media_urls(html, "http://example.com/page");
        assert_eq!(urls.len(), 1);
    }
}
