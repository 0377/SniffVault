mod support;

use std::ffi::CString;

use tempfile::tempdir;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_enqueue_single, engine_start_downloads,
};

#[test]
fn start_downloads_prepare_returns_immediately() {
    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let title = CString::new("ffi smoke").unwrap();
    let url = CString::new("https://cdn.example/clip.mp4").unwrap();
    let enqueue = unsafe {
        engine_enqueue_single(handle, title.as_ptr(), url.as_ptr(), std::ptr::null())
    };
    assert!(!enqueue.is_null());
    unsafe { engine_free_string(enqueue) };

    let start = unsafe { engine_start_downloads(handle) };
    assert!(!start.is_null());
    unsafe { engine_free_string(start) };

    unsafe { engine_destroy(handle) };
}
