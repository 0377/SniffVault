use crate::error::EngineError;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub trait FfmpegLocator: Send + Sync {
    fn resolve(&self) -> Result<PathBuf, EngineError>;
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct BundledFfmpegLocator;

/// Vendor 目录名，与 `scripts/fetch_ffmpeg.sh` 中 `{os}-{arch}` 一致。
///
/// `uname -s` 在 macOS 上为 `darwin`，此处统一为 Rust `std::env::consts::OS` 的 `macos`。
pub fn platform_dir_name() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub fn ffmpeg_binary_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

pub fn vendor_ffmpeg_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor/ffmpeg")
        .join(platform_dir_name())
        .join(ffmpeg_binary_name())
}

/// macOS 应用包内 `Contents/Resources/ffmpeg` 路径（相对给定可执行文件）。
pub fn macos_bundle_ffmpeg_path_from_exe(exe: &Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?;
    if macos_dir.file_name().and_then(|name| name.to_str()) != Some("MacOS") {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return None;
    }
    let ffmpeg = contents.join("Resources").join(ffmpeg_binary_name());
    ffmpeg.is_file().then_some(ffmpeg)
}

fn macos_app_bundle_ffmpeg_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| macos_bundle_ffmpeg_path_from_exe(&exe))
}

#[cfg_attr(not(test), allow(dead_code))]
impl BundledFfmpegLocator {
    pub fn candidate_path() -> PathBuf {
        vendor_ffmpeg_path()
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl FfmpegLocator for BundledFfmpegLocator {
    fn resolve(&self) -> Result<PathBuf, EngineError> {
        if let Some(path) = macos_app_bundle_ffmpeg_path() {
            return Ok(path);
        }

        let path = Self::candidate_path();
        if path.is_file() {
            return Ok(path);
        }
        Err(EngineError::Message(format!(
            "未找到 ffmpeg，请执行 engine/scripts/fetch_ffmpeg.sh，或确保应用包内存在 Resources/{}",
            ffmpeg_binary_name()
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn bundled_path_format() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = BundledFfmpegLocator::candidate_path();
        let expected = Path::new("vendor")
            .join("ffmpeg")
            .join(platform_dir_name())
            .join(ffmpeg_binary_name());
        let rel = path
            .strip_prefix(&manifest)
            .expect("candidate path should be under CARGO_MANIFEST_DIR");
        assert_eq!(rel, expected, "got {}", path.display());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolve_prefers_app_bundle_ffmpeg() {
        let dir = tempdir().unwrap();
        let exe = dir
            .path()
            .join("video_sniffing.app/Contents/MacOS/video_sniffing");
        let bundle_ffmpeg = dir
            .path()
            .join("video_sniffing.app/Contents/Resources/ffmpeg");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::create_dir_all(bundle_ffmpeg.parent().unwrap()).unwrap();
        fs::write(&bundle_ffmpeg, b"fake").unwrap();

        let resolved = macos_bundle_ffmpeg_path_from_exe(&exe);
        assert_eq!(resolved.as_deref(), Some(bundle_ffmpeg.as_path()));
    }
}
