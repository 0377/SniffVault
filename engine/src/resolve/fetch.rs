use reqwest::StatusCode;

use crate::download::http::{HttpClient, PageFetchOptions};
use crate::error::EngineError;
use crate::types::ResolveOptions;

pub(crate) async fn fetch_playlist_or_page(
    http: &HttpClient,
    url: &str,
    opts: &ResolveOptions,
) -> Result<(StatusCode, String), EngineError> {
    let page_opts = PageFetchOptions {
        cookies: opts.cookies.clone(),
        referer: opts.referer.clone(),
    };
    http.get_page_text(url, &page_opts).await
}
