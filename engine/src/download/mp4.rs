use crate::download::checkpoint::{Checkpoint, CheckpointBody};
use crate::download::http::HttpClient;
use crate::error::EngineError;
use reqwest::StatusCode;
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[cfg_attr(not(test), allow(dead_code))]
pub struct Mp4Context<'a> {
    pub(crate) http: &'a HttpClient,
    pub temp_dir: &'a Path,
}

pub(crate) fn mp4_part_path(temp_dir: &Path, output_mp4: &Path) -> PathBuf {
    part_path(temp_dir, output_mp4)
}

fn part_path(temp_dir: &Path, output_mp4: &Path) -> PathBuf {
    let name = output_mp4
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file.mp4".into());
    temp_dir.join(format!("{}.part", name))
}

#[cfg_attr(not(test), allow(dead_code))]
pub async fn download_mp4(
    ctx: &Mp4Context<'_>,
    url: &str,
    output_mp4: &Path,
    checkpoint: Option<Checkpoint>,
) -> Result<(PathBuf, u64), EngineError> {
    std::fs::create_dir_all(ctx.temp_dir)?;
    if let Some(parent) = output_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let part = part_path(ctx.temp_dir, output_mp4);
    let mut start = 0u64;
    if let Some(Checkpoint {
        body:
            CheckpointBody::Mp4 {
                bytes_done,
                part_path: checkpoint_part,
                ..
            },
        ..
    }) = checkpoint
    {
        if Path::new(&checkpoint_part) == part.as_path() {
            start = bytes_done;
        }
    }

    let (total, supports_range) = ctx.http.head_size_and_ranges(url).await?;

    if start > 0 && !supports_range {
        start = 0;
        if tokio::fs::try_exists(&part).await? {
            tokio::fs::remove_file(&part).await?;
        }
    }

    if start > 0 {
        let part_ok = tokio::fs::metadata(&part)
            .await
            .map(|meta| meta.len() == start)
            .unwrap_or(false);
        if !part_ok {
            start = 0;
            if tokio::fs::try_exists(&part).await? {
                tokio::fs::remove_file(&part).await?;
            }
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(start > 0)
        .truncate(start == 0)
        .open(&part)
        .await?;

    let response = if start > 0 && supports_range {
        let response = ctx.http.get_stream_range(url, start).await?;
        if response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(EngineError::Message(format!(
                "expected 206 Partial Content for range resume at byte {start}, got {}",
                response.status()
            )));
        }
        response
    } else {
        ctx.http.get_stream(url).await?
    };

    let mut bytes_done = start;
    let mut response = response;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        bytes_done += chunk.len() as u64;
    }
    file.flush().await?;

    if let Some(total) = total {
        if bytes_done != total {
            return Err(EngineError::Message(format!(
                "incomplete download: got {bytes_done} of {total}"
            )));
        }
    }

    let final_path = output_mp4.to_path_buf();
    tokio::fs::rename(&part, &final_path).await?;
    Ok((final_path, bytes_done))
}
