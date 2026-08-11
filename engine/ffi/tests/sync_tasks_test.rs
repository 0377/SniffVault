use std::ffi::{CStr, CString};

use tempfile::tempdir;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::engine_enqueue_single;

#[test]
fn enqueue_single_returns_task_id() {
    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let title = CString::new("test").unwrap();
    let url = CString::new("https://example.com/video.mp4").unwrap();
    let result =
        unsafe { engine_enqueue_single(handle, title.as_ptr(), url.as_ptr(), std::ptr::null()) };
    assert!(!result.is_null());

    let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(parsed["ok"], true);
    assert!(parsed["data"].is_string());
    assert!(!parsed["data"].as_str().unwrap().is_empty());

    unsafe { engine_free_string(result) };
    unsafe { engine_destroy(handle) };
}
