pub(crate) mod merge;
pub(crate) mod playlist;
pub(crate) mod segments;

use crate::download::checkpoint::{Checkpoint, CheckpointBody, HlsEncryption};
use crate::download::ffmpeg::{BundledFfmpegLocator, FfmpegLocator};
use crate::download::hls::merge::merge_segments_to_mp4;
use crate::download::hls::playlist::{
    parse_media_playlist, select_media_playlist_url, MediaPlaylist,
};
use crate::download::hls::segments::download_segments;
use crate::download::http::HttpClient;
use crate::error::EngineError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub(crate) struct HlsDownloadState {
    pub temp_dir: String,
    pub media_playlist_url: String,
    pub variant_url: Option<String>,
    pub segments_done: Vec<u32>,
    pub segment_paths: Vec<String>,
    pub encryption: Option<HlsEncryption>,
}

impl HlsDownloadState {
    pub(crate) fn to_checkpoint(&self) -> Option<Checkpoint> {
        if self.segments_done.is_empty() {
            return None;
        }
        Some(Checkpoint {
            version: 1,
            body: CheckpointBody::Hls {
                temp_dir: self.temp_dir.clone(),
                media_playlist_url: self.media_playlist_url.clone(),
                variant_url: self.variant_url.clone(),
                segments_done: self.segments_done.clone(),
                segment_paths: self.segment_paths.clone(),
                encryption: self.encryption.clone(),
            },
        })
    }
}

fn encryption_from_playlist(playlist: &MediaPlaylist) -> Option<HlsEncryption> {
    playlist.encryption.as_ref().map(|key| HlsEncryption {
        method: key.method.clone(),
        key_uri: key.uri.clone(),
        iv_hex: key.iv_hex.clone(),
    })
}

fn is_master_playlist(body: &str) -> bool {
    body.lines()
        .map(str::trim)
        .any(|line| line.starts_with("#EXT-X-STREAM-INF:"))
}

pub(crate) struct HlsContext<'a> {
    pub(crate) http: &'a HttpClient,
    pub(crate) temp_dir: &'a Path,
    pub(crate) ffmpeg: &'a Path,
}

async fn resolve_media_playlist(
    http: &HttpClient,
    source_url: &str,
    quality_label: Option<&str>,
) -> Result<(String, MediaPlaylist), EngineError> {
    let body = http.get_text(source_url).await?;
    if is_master_playlist(&body) {
        let media_url = select_media_playlist_url(&body, source_url, quality_label)?;
        let media_body = http.get_text(&media_url).await?;
        let playlist = parse_media_playlist(&media_body, &media_url)?;
        Ok((media_url, playlist))
    } else {
        let playlist = parse_media_playlist(&body, source_url)?;
        Ok((source_url.to_string(), playlist))
    }
}

pub(crate) async fn download_hls_to_mp4(
    ctx: &HlsContext<'_>,
    source_url: &str,
    output_mp4: &Path,
    quality_label: Option<&str>,
    checkpoint: Option<Checkpoint>,
    progress: Option<Arc<Mutex<HlsDownloadState>>>,
) -> Result<PathBuf, EngineError> {
    std::fs::create_dir_all(ctx.temp_dir)?;
    let temp_dir_str = ctx.temp_dir.to_string_lossy().into_owned();
    let from_master = {
        let body = ctx.http.get_text(source_url).await?;
        is_master_playlist(&body)
    };

    let (media_playlist_url, playlist, skip_indices, existing_paths, _variant_url) =
        match checkpoint.and_then(|cp| match cp.body {
            CheckpointBody::Hls {
                temp_dir,
                media_playlist_url,
                variant_url,
                segments_done,
                segment_paths,
                encryption,
                ..
            } if Path::new(&temp_dir) == ctx.temp_dir => Some((
                media_playlist_url,
                variant_url,
                segments_done,
                segment_paths
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>(),
                encryption,
            )),
            _ => None,
        }) {
            Some((media_url, variant_url, done, paths, encryption)) => {
                let body = ctx.http.get_text(&media_url).await?;
                let playlist = parse_media_playlist(&body, &media_url)?;
                if let Some(state) = &progress {
                    let mut s = state.lock().await;
                    s.temp_dir = temp_dir_str.clone();
                    s.media_playlist_url = media_url.clone();
                    s.variant_url = variant_url.clone();
                    s.segments_done = done.clone();
                    s.segment_paths = paths
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect();
                    s.encryption = encryption;
                }
                (media_url, playlist, done, paths, variant_url)
            }
            None => {
                let (media_url, playlist) =
                    resolve_media_playlist(ctx.http, source_url, quality_label).await?;
                let variant_url = if from_master {
                    Some(source_url.to_string())
                } else {
                    None
                };
                if let Some(state) = &progress {
                    let mut s = state.lock().await;
                    s.temp_dir = temp_dir_str.clone();
                    s.media_playlist_url = media_url.clone();
                    s.variant_url = variant_url.clone();
                    s.segments_done.clear();
                    s.segment_paths.clear();
                    s.encryption = encryption_from_playlist(&playlist);
                }
                (media_url, playlist, Vec::new(), Vec::new(), variant_url)
            }
        };

    let segment_paths = download_segments(
        ctx.http,
        &playlist,
        &media_playlist_url,
        ctx.temp_dir,
        &skip_indices,
        &existing_paths,
        progress.clone(),
    )
    .await?;

    merge_segments_to_mp4(ctx.ffmpeg, &segment_paths, ctx.temp_dir, output_mp4)?;

    Ok(output_mp4.to_path_buf())
}

pub(crate) async fn download_hls_to_mp4_with_bundled_ffmpeg(
    http: &HttpClient,
    temp_dir: &Path,
    source_url: &str,
    output_mp4: &Path,
    quality_label: Option<&str>,
    checkpoint: Option<Checkpoint>,
) -> Result<PathBuf, EngineError> {
    let ffmpeg = BundledFfmpegLocator.resolve()?;
    let ctx = HlsContext {
        http,
        temp_dir,
        ffmpeg: &ffmpeg,
    };
    download_hls_to_mp4(
        &ctx,
        source_url,
        output_mp4,
        quality_label,
        checkpoint,
        None,
    )
    .await
}
