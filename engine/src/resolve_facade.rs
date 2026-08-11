use crate::download::http::HttpClient;
use crate::error::EngineError;
use crate::types::{Quality, ResolveOptions, ResolveOutcome};

pub async fn resolve_url_for_ffi(
    user_agent: Option<&str>,
    url: &str,
    opts: ResolveOptions,
) -> Result<ResolveOutcome, EngineError> {
    let http = HttpClient::new(user_agent)?;
    crate::resolve::resolve_url(&http, url, opts).await
}

pub async fn resolve_qualities_for_ffi(
    user_agent: Option<&str>,
    media_url: &str,
    opts: ResolveOptions,
) -> Result<Vec<Quality>, EngineError> {
    let http = HttpClient::new(user_agent)?;
    crate::resolve::resolve_qualities(&http, media_url, opts).await
}
