mod support;

use support::fixture_server;
use tempfile::tempdir;
use video_sniffing_engine::test_api::download_hls_with_new_client;

fn fixtures_hls_dir() -> std::path::PathBuf {
    fixture_server::fixtures_dir().join("hls")
}

fn output_contains_ftyp(path: &std::path::Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    bytes.windows(4).any(|window| window == b"ftyp")
}

#[tokio::test]
async fn hls_plain_merge_produces_ftyp() {
    let dir = tempdir().unwrap();
    let temp = dir.path().join(".dl/task-hls-plain");
    let output = dir.path().join("media/plain.mp4");
    let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
    let url = format!("http://{addr}/media.m3u8");

    let final_path = download_hls_with_new_client(&url, &temp, &output, None, None)
        .await
        .unwrap();

    assert!(final_path.exists());
    assert!(output_contains_ftyp(&final_path));
}

#[tokio::test]
async fn hls_aes128_to_mp4() {
    let dir = tempdir().unwrap();
    let temp = dir.path().join(".dl/task-hls-encrypted");
    let output = dir.path().join("media/encrypted.mp4");
    let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
    let url = format!("http://{addr}/encrypted.m3u8");

    let final_path = download_hls_with_new_client(&url, &temp, &output, None, None)
        .await
        .unwrap();

    assert!(final_path.exists());
    assert!(output_contains_ftyp(&final_path));
    assert!(std::fs::metadata(&final_path).unwrap().len() > 1024);
}

#[tokio::test]
async fn hls_master_highest() {
    let dir = tempdir().unwrap();
    let temp = dir.path().join(".dl/task-hls-master");
    let output = dir.path().join("media/master.mp4");
    let (addr, _guard) = fixture_server::serve_dir(fixtures_hls_dir()).await;
    let url = format!("http://{addr}/master.m3u8");

    let final_path = download_hls_with_new_client(&url, &temp, &output, Some("highest"), None)
        .await
        .unwrap();

    assert!(final_path.exists());
    assert!(output_contains_ftyp(&final_path));
}
