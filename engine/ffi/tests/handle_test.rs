use std::ffi::{CStr, CString};

use tempfile::tempdir;
use video_sniffing_engine_ffi::handle::{
    engine_destroy, engine_free_string, engine_last_error, engine_open,
};

#[test]
fn open_and_destroy_roundtrip() {
    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());
    unsafe { engine_destroy(handle) };
}

#[test]
fn open_invalid_path_sets_last_error() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"device_name":"test","media_dir":"../outside","max_concurrency":2,"default_quality_label":"highest"}"#,
    )
    .unwrap();

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(handle.is_null());

    let err_ptr = unsafe { engine_last_error() };
    assert!(!err_ptr.is_null());
    let err_str = unsafe { CStr::from_ptr(err_ptr).to_str().unwrap() };
    let parsed: serde_json::Value = serde_json::from_str(err_str).unwrap();
    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["error"]["kind"], "invalid_arg");
    unsafe { engine_free_string(err_ptr) };
}
