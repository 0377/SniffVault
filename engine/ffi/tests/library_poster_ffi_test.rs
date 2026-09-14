use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use video_sniffing_engine::{
    Engine, MediaKind, ResolveOutcome, ResolveUrlResult, ResourceCandidate,
};
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::engine_refresh_library_poster;

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

fn write_poster_page(page_root: &Path, poster_url: &str) -> PathBuf {
    let page = page_root.join("page.html");
    let html = format!(
        r#"<!DOCTYPE html><html><head>
<meta property="og:image" content="{poster_url}">
</head><body></body></html>"#
    );
    std::fs::write(&page, html).unwrap();
    page
}

#[test]
fn refresh_library_poster_ffi_returns_item() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/posters");
    let (poster_addr, poster_server) = spawn_file_server_for(Duration::from_secs(5), fixtures, 4);

    let page_root = dir.path().join("pages");
    std::fs::create_dir_all(&page_root).unwrap();
    let poster_url = format!("http://{}/sample.jpg", poster_addr);
    write_poster_page(&page_root, &poster_url);
    let (page_addr, page_server) = spawn_file_server_for(Duration::from_secs(5), page_root, 4);

    let media = engine.media_dir().join("m.mp4");
    std::fs::write(&media, b"v").unwrap();
    let page_source = format!("http://{}/page.html", page_addr);
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), Some(&page_source), None)
        .unwrap();
    assert!(item.poster_path.is_none());
    let item_id = item.id.clone();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let item_id_c = CString::new(item_id).unwrap();
    let result =
        unsafe { engine_refresh_library_poster(handle, item_id_c.as_ptr(), std::ptr::null()) };
    assert!(!result.is_null());

    let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(parsed["ok"], true);
    assert!(parsed["data"]["item"]["poster_path"]
        .as_str()
        .unwrap()
        .contains(".posters"));

    unsafe { engine_free_string(result) };
    poster_server.join().unwrap();
    page_server.join().unwrap();
    unsafe { engine_destroy(handle) };
}

#[test]
fn resolve_url_result_serde_roundtrip_includes_poster_url() {
    let original = ResolveUrlResult {
        outcome: ResolveOutcome::Single(ResourceCandidate {
            id: "c1".into(),
            url: "https://example.com/v.mp4".into(),
            title: Some("t".into()),
            kind: MediaKind::Mp4,
            quality: None,
            page_url: None,
        }),
        poster_url: Some("https://example.com/p.jpg".into()),
    };
    let json = serde_json::to_string(&original).unwrap();
    assert!(json.contains("poster_url"));
    let back: ResolveUrlResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.poster_url, original.poster_url);
    assert_eq!(back.outcome, original.outcome);
}
