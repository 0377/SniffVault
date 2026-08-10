mod support;

use support::fixture_server;
use tempfile::tempdir;
use video_sniffing_engine::test_api::{download_mp4_with_new_client, Checkpoint, CheckpointBody};

#[tokio::test]
async fn mp4_download_completes() {
    let dir = tempdir().unwrap();
    let temp = dir.path().join(".dl/task-1");
    let output = dir.path().join("media/sample.mp4");
    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");

    let (final_path, bytes) = download_mp4_with_new_client(&url, &temp, &output, None)
        .await
        .unwrap();

    let expected = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
    assert_eq!(bytes as usize, expected.len());
    assert!(final_path.exists());
    assert_eq!(std::fs::read(&final_path).unwrap(), expected);
}

#[tokio::test]
async fn mp4_resume_after_interrupt() {
    let dir = tempdir().unwrap();
    let temp = dir.path().join(".dl/task-1");
    let output = dir.path().join("media/sample.mp4");
    let fixture_bytes = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
    let half = fixture_bytes.len() / 2;

    let part = temp.join("sample.mp4.part");
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(&part, &fixture_bytes[..half]).unwrap();

    let checkpoint = Checkpoint {
        version: 1,
        body: CheckpointBody::Mp4 {
            temp_dir: temp.to_string_lossy().into(),
            part_path: part.to_string_lossy().into(),
            bytes_done: half as u64,
        },
    };

    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");

    let (final_path, bytes) = download_mp4_with_new_client(&url, &temp, &output, Some(checkpoint))
        .await
        .unwrap();

    assert_eq!(bytes as usize, fixture_bytes.len());
    assert_eq!(std::fs::read(&final_path).unwrap(), fixture_bytes);
}

#[tokio::test]
async fn mp4_resume_missing_part_resets() {
    let dir = tempdir().unwrap();
    let temp = dir.path().join(".dl/task-1");
    let output = dir.path().join("media/sample.mp4");
    let fixture_bytes = std::fs::read(fixture_server::fixtures_dir().join("sample.mp4")).unwrap();
    let half = fixture_bytes.len() / 2;

    std::fs::create_dir_all(&temp).unwrap();
    let part = temp.join("sample.mp4.part");

    let checkpoint = Checkpoint {
        version: 1,
        body: CheckpointBody::Mp4 {
            temp_dir: temp.to_string_lossy().into(),
            part_path: part.to_string_lossy().into(),
            bytes_done: half as u64,
        },
    };

    let (addr, _guard) = fixture_server::serve_dir(fixture_server::fixtures_dir()).await;
    let url = format!("http://{addr}/sample.mp4");

    let (final_path, bytes) = download_mp4_with_new_client(&url, &temp, &output, Some(checkpoint))
        .await
        .unwrap();

    assert_eq!(bytes as usize, fixture_bytes.len());
    assert_eq!(std::fs::read(&final_path).unwrap(), fixture_bytes);
    assert!(!part.exists());
}
