mod support;

use std::ffi::{CStr, CString};
use std::time::Duration;

use support::dart_port::Isolate;
use support::fixture_server;
use tempfile::tempdir;
use video_sniffing_engine_ffi::async_resolve::engine_resolve_url_async;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};

#[test]
fn resolve_url_async_posts_result() {
    let server_rt = tokio::runtime::Runtime::new().unwrap();
    let (addr, _guard) = server_rt
        .block_on(async { fixture_server::serve_dir(fixture_server::fixtures_dir()).await });
    let url = format!("http://{addr}/sample.mp4");

    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let (port_id, rx) = Isolate::create_dart_port();
    let url_c = CString::new(url).unwrap();
    let request_id = CString::new("req-1").unwrap();

    let immediate = unsafe {
        engine_resolve_url_async(
            handle,
            url_c.as_ptr(),
            std::ptr::null(),
            port_id,
            request_id.as_ptr(),
        )
    };
    assert!(!immediate.is_null());
    let immediate_str = unsafe { CStr::from_ptr(immediate).to_str().unwrap() };
    let immediate_json: serde_json::Value = serde_json::from_str(immediate_str).unwrap();
    assert_eq!(immediate_json["ok"], true);
    assert_eq!(immediate_json["data"]["accepted"], true);
    assert_eq!(immediate_json["data"]["request_id"], "req-1");
    unsafe { engine_free_string(immediate) };

    let msg = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(msg.contains("req-1") && msg.contains("\"ok\":true"));

    unsafe { engine_destroy(handle) };
}
