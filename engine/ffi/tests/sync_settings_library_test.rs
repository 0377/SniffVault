use std::ffi::{CStr, CString};

use tempfile::tempdir;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_list_library, engine_save_settings, engine_settings,
};

#[test]
fn settings_roundtrip_json() {
    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let settings_ptr = unsafe { engine_settings(handle) };
    assert!(!settings_ptr.is_null());
    let settings_str = unsafe { CStr::from_ptr(settings_ptr).to_str().unwrap() };
    let parsed: serde_json::Value = serde_json::from_str(settings_str).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["media_dir"], "media");
    unsafe { engine_free_string(settings_ptr) };

    let new_settings = serde_json::json!({
        "media_dir": "videos",
        "max_concurrency": 4,
        "default_quality_label": "1080p",
        "user_agent": null,
        "device_name": "TestDevice"
    });
    let json = CString::new(new_settings.to_string()).unwrap();
    let save_ptr = unsafe { engine_save_settings(handle, json.as_ptr()) };
    assert!(!save_ptr.is_null());
    let save_str = unsafe { CStr::from_ptr(save_ptr).to_str().unwrap() };
    let save_parsed: serde_json::Value = serde_json::from_str(save_str).unwrap();
    assert_eq!(save_parsed["ok"], true);
    unsafe { engine_free_string(save_ptr) };

    let settings_ptr2 = unsafe { engine_settings(handle) };
    let settings_str2 = unsafe { CStr::from_ptr(settings_ptr2).to_str().unwrap() };
    let parsed2: serde_json::Value = serde_json::from_str(settings_str2).unwrap();
    assert_eq!(parsed2["data"]["device_name"], "TestDevice");
    assert_eq!(parsed2["data"]["media_dir"], "videos");
    unsafe { engine_free_string(settings_ptr2) };

    unsafe { engine_destroy(handle) };
}

#[test]
fn list_library_empty() {
    let dir = tempdir().unwrap();
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let lib_ptr = unsafe { engine_list_library(handle) };
    assert!(!lib_ptr.is_null());
    let lib_str = unsafe { CStr::from_ptr(lib_ptr).to_str().unwrap() };
    let parsed: serde_json::Value = serde_json::from_str(lib_str).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"], serde_json::json!([]));
    unsafe { engine_free_string(lib_ptr) };

    unsafe { engine_destroy(handle) };
}
