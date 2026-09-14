use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use tempfile::tempdir;
use video_sniffing_engine::Engine;

#[path = "support/library_merge_seed.rs"]
mod library_merge_seed;

fn spawn_file_server(root: PathBuf) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            serve_one_request(&mut stream, &root);
        }
    });
    (format!("127.0.0.1:{port}"), handle)
}

fn serve_one_request(stream: &mut TcpStream, root: &Path) {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let rel = path.trim_start_matches('/');
    let file_path = root.join(rel);
    if !file_path.is_file() {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    }
    let body = std::fs::read(&file_path).unwrap();
    let ct = if rel.ends_with(".png") {
        "image/png"
    } else if rel.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&body);
}

#[test]
fn merge_migrates_poster_when_target_empty() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let poster = engine.media_dir().join(".posters").join("p.jpg");
    std::fs::create_dir_all(poster.parent().unwrap()).unwrap();
    std::fs::write(&poster, b"x").unwrap();
    let canon = poster
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let seed = library_merge_seed::seed_duplicate_series_with_source_poster(
        &engine,
        "剧",
        Some(1),
        &canon,
    );
    library_merge_seed::add_episode(&engine, &seed.source_item_id, 1, "源1", "a.mp4", 0);
    library_merge_seed::add_episode(&engine, &seed.target_item_id, 1, "目标1", "b.mp4", 0);
    engine
        .merge_library_items(&seed.source_item_id, &seed.target_item_id, false)
        .unwrap();
    let target = engine
        .list_library()
        .unwrap()
        .into_iter()
        .find(|i| i.id == seed.target_item_id)
        .unwrap();
    assert_eq!(target.poster_path.as_deref(), Some(canon.as_str()));
    assert!(engine
        .list_library()
        .unwrap()
        .iter()
        .all(|i| i.id != seed.source_item_id));
}

#[test]
fn register_single_with_poster_url_sets_poster_path() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters");
    let (addr, server) = spawn_file_server(fixtures);
    let poster_url = format!("http://{}/sample.jpg", addr);
    let (item, _) = engine
        .register_completed_single(
            "片",
            media.to_str().unwrap(),
            Some("https://example.com/page"),
            Some(&poster_url),
        )
        .unwrap();
    server.join().unwrap();
    let listed = engine.list_library().unwrap();
    assert_eq!(listed[0].id, item.id);
    assert!(listed[0].poster_path.is_some());
    assert!(std::path::Path::new(listed[0].poster_path.as_ref().unwrap()).exists());
}

#[test]
fn register_single_with_invalid_poster_url_still_registers() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();
    let (item, _) = engine
        .register_completed_single(
            "片",
            media.to_str().unwrap(),
            Some("https://example.com/page"),
            Some("http://127.0.0.1:9/none"),
        )
        .unwrap();
    let listed = engine.list_library().unwrap();
    assert_eq!(listed[0].id, item.id);
    assert!(listed[0].poster_path.is_none());
}

#[test]
fn register_series_second_episode_does_not_overwrite_poster() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters");
    let (addr, server) = spawn_file_server(fixtures);
    let poster_a = format!("http://{}/sample.jpg", addr);
    let poster_b = format!("http://{}/sample.jpg", addr);

    let media1 = engine.media_dir().join("ep1.mp4");
    std::fs::write(&media1, b"v1").unwrap();
    let (item, _) = engine
        .register_completed_episode(
            "剧",
            Some(1),
            1,
            "第1集",
            media1.to_str().unwrap(),
            None,
            Some(&poster_a),
        )
        .unwrap();
    let first_poster = item.poster_path.clone().expect("first episode poster");

    let media2 = engine.media_dir().join("ep2.mp4");
    std::fs::write(&media2, b"v2").unwrap();
    let (item2, _) = engine
        .register_completed_episode(
            "剧",
            Some(1),
            2,
            "第2集",
            media2.to_str().unwrap(),
            None,
            Some(&poster_b),
        )
        .unwrap();
    server.join().unwrap();
    assert_eq!(item2.id, item.id);
    assert_eq!(item2.poster_path.as_deref(), Some(first_poster.as_str()));
}

#[test]
fn register_single_skips_oversized_poster_body() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();

    let fixture_root = dir.path().join("fixtures");
    std::fs::create_dir_all(&fixture_root).unwrap();
    let huge = fixture_root.join("huge.jpg");
    std::fs::write(&huge, vec![0u8; 5 * 1024 * 1024 + 1]).unwrap();

    let (addr, server) = spawn_file_server(fixture_root);
    let poster_url = format!("http://{}/huge.jpg", addr);
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None, Some(&poster_url))
        .unwrap();
    server.join().unwrap();
    assert!(item.poster_path.is_none());
}
