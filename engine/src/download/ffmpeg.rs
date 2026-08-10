use crate::error::EngineError;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub trait FfmpegLocator: Send + Sync {
    fn resolve(&self) -> Result<PathBuf, EngineError>;
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct BundledFfmpegLocator;

#[cfg_attr(not(test), allow(dead_code))]
impl BundledFfmpegLocator {
    pub fn candidate_path() -> PathBuf {
        let target = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("vendor/ffmpeg")
            .join(target)
            .join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            })
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl FfmpegLocator for BundledFfmpegLocator {
    fn resolve(&self) -> Result<PathBuf, EngineError> {
        let path = Self::candidate_path();
        if path.is_file() {
            return Ok(path);
        }
        Err(EngineError::Message(format!(
            "未找到 ffmpeg，请将二进制放到 {}",
            path.display()
        )))
    }
}

#[allow(dead_code)]
pub fn run_concat(ffmpeg: &Path, concat_list: &Path, output_mp4: &Path) -> Result<(), EngineError> {
    let status = std::process::Command::new(ffmpeg)
        .args([
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            &concat_list.to_string_lossy(),
            "-c",
            "copy",
            &output_mp4.to_string_lossy(),
        ])
        .status()?;
    if !status.success() {
        return Err(EngineError::Message("ffmpeg 合并失败".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_path_format() {
        let path = BundledFfmpegLocator::candidate_path();
        assert!(path.to_string_lossy().contains("vendor/ffmpeg"));
    }
}
