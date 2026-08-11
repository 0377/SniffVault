use std::ffi::{CStr, CString};

use tempfile::tempdir;
use video_sniffing_engine_ffi::events::engine_subscribe_task_events;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};

#[test]
fn subscribe_returns_ok() {
    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let result = unsafe { engine_subscribe_task_events(handle, 42) };
    assert!(!result.is_null());

    let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(parsed["ok"], true);

    unsafe { engine_free_string(result) };
    unsafe { engine_destroy(handle) };
}
