pub(crate) mod merge;
pub(crate) mod playlist;
pub(crate) mod segments;

use crate::download::checkpoint::{Checkpoint, CheckpointBody};
use crate::download::ffmpeg::{BundledFfmpegLocator, FfmpegLocator};
use crate::download::hls::merge::merge_segments_to_mp4;
use crate::download::hls::playlist::{
    parse_media_playlist, select_media_playlist_url, MediaPlaylist,
};
use crate::download::hls::segments::download_segments;
use crate::download::http::HttpClient;
use crate::error::EngineError;
use std::path::{Path, PathBuf};

pub(crate) struct HlsContext<'a> {
    pub(crate) http: &'a HttpClient,
    pub(crate) temp_dir: &'a Path,
    pub(crate) ffmpeg: &'a Path,
}

fn is_master_playlist(body: &str) -> bool {
    body.lines()
        .map(str::trim)
        .any(|line| line.starts_with("#EXT-X-STREAM-INF:"))
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
) -> Result<PathBuf, EngineError> {
    std::fs::create_dir_all(ctx.temp_dir)?;

    let (media_playlist_url, playlist, skip_indices, existing_paths) =
        match checkpoint.and_then(|cp| match cp.body {
            CheckpointBody::Hls {
                temp_dir,
                media_playlist_url,
                segments_done,
                segment_paths,
                ..
            } if Path::new(&temp_dir) == ctx.temp_dir => Some((
                media_playlist_url,
                segments_done,
                segment_paths
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>(),
            )),
            _ => None,
        }) {
            Some((media_url, done, paths)) => {
                let body = ctx.http.get_text(&media_url).await?;
                let playlist = parse_media_playlist(&body, &media_url)?;
                (media_url, playlist, done, paths)
            }
            None => {
                let (media_url, playlist) =
                    resolve_media_playlist(ctx.http, source_url, quality_label).await?;
                (media_url, playlist, Vec::new(), Vec::new())
            }
        };

    let segment_paths = download_segments(
        ctx.http,
        &playlist,
        &media_playlist_url,
        ctx.temp_dir,
        &skip_indices,
        &existing_paths,
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
    download_hls_to_mp4(&ctx, source_url, output_mp4, quality_label, checkpoint).await
}
