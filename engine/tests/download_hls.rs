mod support;

use support::fixture_server;
use tempfile::tempdir;
use video_sniffing_engine::test_api::download_hls_with_new_client;
use video_sniffing_engine::test_api::{Checkpoint, CheckpointBody};

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

#[tokio::test]
async fn hls_resumes_from_checkpoint() {
    let dir = tempdir().unwrap();
    let hls_dir = dir.path().join("hls");
    let fixtures = fixtures_hls_dir();
    let seg = std::fs::read(fixtures.join("segments/seg0.ts")).unwrap();
    std::fs::create_dir_all(hls_dir.join("segments")).unwrap();
    std::fs::write(hls_dir.join("segments/seg0.ts"), &seg).unwrap();
    std::fs::write(hls_dir.join("segments/seg1.ts"), &seg).unwrap();
    std::fs::write(
        hls_dir.join("two.m3u8"),
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n#EXT-X-MEDIA-SEQUENCE:0\n#EXTINF:1.5,\nsegments/seg0.ts\n#EXTINF:1.5,\nsegments/seg1.ts\n#EXT-X-ENDLIST\n",
    )
    .unwrap();

    let temp = dir.path().join(".dl/task-hls-resume");
    let output = dir.path().join("media/resumed.mp4");
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(temp.join("seg0000.ts"), &seg).unwrap();

    let (addr, _guard) = fixture_server::serve_dir(hls_dir).await;
    let playlist_url = format!("http://{addr}/two.m3u8");
    let checkpoint = Checkpoint {
        version: 1,
        body: CheckpointBody::Hls {
            temp_dir: temp.to_string_lossy().into_owned(),
            media_playlist_url: playlist_url.clone(),
            variant_url: None,
            segments_done: vec![0],
            segment_paths: vec![temp.join("seg0000.ts").to_string_lossy().into_owned()],
            encryption: None,
        },
    };

    let final_path =
        download_hls_with_new_client(&playlist_url, &temp, &output, None, Some(checkpoint))
            .await
            .unwrap();

    assert!(final_path.exists());
    assert!(output_contains_ftyp(&final_path));
    assert!(temp.join("seg0001.ts").is_file());
}
