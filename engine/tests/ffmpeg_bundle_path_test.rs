use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use video_sniffing_engine::test_api::{macos_bundle_ffmpeg_path_from_exe, vendor_ffmpeg_path};

fn touch(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"fake").unwrap();
}

fn fake_macos_app_exe(dir: &Path) -> PathBuf {
    dir.join("video_sniffing.app/Contents/MacOS/video_sniffing")
}

#[test]
fn vendor_ffmpeg_path_is_under_engine_vendor() {
    let path = vendor_ffmpeg_path();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rel = path
        .strip_prefix(&manifest)
        .expect("vendor ffmpeg path should be under engine crate");
    assert!(rel.starts_with(Path::new("vendor/ffmpeg")));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_bundle_ffmpeg_path_from_exe_reads_resources_binary() {
    let dir = tempdir().unwrap();
    let exe = fake_macos_app_exe(dir.path());
    let ffmpeg = dir
        .path()
        .join("video_sniffing.app/Contents/Resources/ffmpeg");
    touch(&ffmpeg);

    let resolved = macos_bundle_ffmpeg_path_from_exe(&exe);
    assert_eq!(resolved.as_deref(), Some(ffmpeg.as_path()));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_bundle_ffmpeg_path_from_exe_returns_none_without_resources_binary() {
    let dir = tempdir().unwrap();
    let exe = fake_macos_app_exe(dir.path());
    fs::create_dir_all(exe.parent().unwrap()).unwrap();

    assert!(macos_bundle_ffmpeg_path_from_exe(&exe).is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_bundle_ffmpeg_path_from_exe_returns_none_outside_app_bundle() {
    let dir = tempdir().unwrap();
    let exe = dir.path().join("plain-binary");
    touch(&exe);

    assert!(macos_bundle_ffmpeg_path_from_exe(&exe).is_none());
}
