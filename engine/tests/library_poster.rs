use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use video_sniffing_engine::test_api::LibraryStore;
use video_sniffing_engine::Engine;

#[path = "support/library_merge_seed.rs"]
mod library_merge_seed;

fn spawn_file_server(root: PathBuf) -> (String, thread::JoinHandle<()>) {
    spawn_file_server_for(Duration::from_secs(5), root, 8)
}

fn spawn_file_server_for(
    timeout: Duration,
    root: PathBuf,
    max_requests: usize,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        let deadline = Instant::now() + timeout;
        let mut served = 0usize;
        while Instant::now() < deadline && served < max_requests {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    serve_one_request(&mut stream, &root);
                    served += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
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

fn write_poster_page(dir: &Path, poster_url: &str) -> PathBuf {
    let page = dir.join("page.html");
    let html = format!(
        r#"<!DOCTYPE html><html><head>
<meta property="og:image" content="{poster_url}">
</head><body></body></html>"#
    );
    std::fs::write(&page, html).unwrap();
    page
}

#[test]
fn refresh_library_poster_from_episode_source_url() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters");
    let (poster_addr, poster_server) = spawn_file_server_for(Duration::from_secs(10), fixtures, 8);

    let page_root = dir.path().join("pages");
    std::fs::create_dir_all(&page_root).unwrap();
    let poster_url = format!("http://{}/sample.jpg", poster_addr);
    write_poster_page(&page_root, &poster_url);
    let (page_addr, page_server) = spawn_file_server_for(Duration::from_secs(10), page_root, 8);

    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();
    let page_source = format!("http://{}/page.html", page_addr);
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), Some(&page_source), None)
        .unwrap();
    assert!(item.poster_path.is_none());

    let refreshed = engine.refresh_library_poster(&item.id, None).unwrap();
    poster_server.join().unwrap();
    page_server.join().unwrap();

    assert!(refreshed.poster_path.is_some());
    let path = refreshed.poster_path.as_ref().unwrap();
    assert!(std::path::Path::new(path).exists());
    assert!(path.contains(".posters"));
}

#[test]
fn remove_library_item_deletes_poster_file() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters");
    let (addr, server) = spawn_file_server(fixtures);
    let poster_url = format!("http://{}/sample.jpg", addr);
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None, Some(&poster_url))
        .unwrap();
    server.join().unwrap();

    let poster_path = item.poster_path.clone().expect("poster path");
    assert!(std::path::Path::new(&poster_path).exists());

    engine.remove_library_item(&item.id, true).unwrap();
    assert!(!std::path::Path::new(&poster_path).exists());
    assert!(engine.list_library().unwrap().is_empty());
}

#[test]
fn refresh_library_poster_overwrites_existing_poster() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters");
    let (poster_addr, poster_server) = spawn_file_server_for(Duration::from_secs(5), fixtures, 6);

    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();
    let poster_url = format!("http://{}/sample.jpg", poster_addr);
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None, Some(&poster_url))
        .unwrap();
    let initial_poster = item.poster_path.clone().expect("initial poster");

    let old_png = engine
        .media_dir()
        .join(".posters")
        .join(format!("{}.png", item.id));
    std::fs::create_dir_all(old_png.parent().unwrap()).unwrap();
    std::fs::write(&old_png, b"old-poster").unwrap();
    let old_canon = old_png
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let store =
        LibraryStore::open(&engine.media_dir().parent().unwrap().join("library.db")).unwrap();
    store.update_item_poster_path(&item.id, &old_canon).unwrap();
    assert!(std::path::Path::new(&old_canon).exists());

    let page_root = dir.path().join("pages");
    std::fs::create_dir_all(&page_root).unwrap();
    write_poster_page(&page_root, &poster_url);
    let (page_addr, page_server) = spawn_file_server_for(Duration::from_secs(5), page_root, 4);
    let page_source = format!("http://{}/page.html", page_addr);
    let store =
        LibraryStore::open(&engine.media_dir().parent().unwrap().join("library.db")).unwrap();
    let eps = store.list_episodes(&item.id).unwrap();
    let ep = eps[0].clone();
    store
        .upsert_episode(&video_sniffing_engine::LibraryEpisode {
            source_url: Some(page_source),
            ..ep
        })
        .unwrap();

    let refreshed = engine.refresh_library_poster(&item.id, None).unwrap();
    poster_server.join().unwrap();
    page_server.join().unwrap();

    let new_poster = refreshed.poster_path.expect("refreshed poster");
    assert_ne!(new_poster, old_canon);
    assert!(std::path::Path::new(&new_poster).exists());
    assert!(!std::path::Path::new(&old_canon).exists());
    let sample = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/posters/sample.jpg"),
    )
    .unwrap();
    assert_eq!(std::fs::read(&new_poster).unwrap(), sample);
    assert_eq!(new_poster, initial_poster);
}
