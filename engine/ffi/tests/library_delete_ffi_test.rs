use std::ffi::{CStr, CString};
use std::fs;
use tempfile::tempdir;
use video_sniffing_engine::Engine;
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{engine_list_library, engine_remove_library_item};

#[test]
fn library_delete_ffi_remove_library_item_ok_json() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("f.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("片", media.to_str().unwrap(), None)
        .unwrap();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    let item_id = CString::new(item.id).unwrap();
    let ptr = unsafe { engine_remove_library_item(handle, item_id.as_ptr(), 1) };
    assert!(!ptr.is_null());
    let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(v["ok"], true);
    unsafe { engine_free_string(ptr) };

    let lib_ptr = unsafe { engine_list_library(handle) };
    let lib_json = unsafe { CStr::from_ptr(lib_ptr).to_str().unwrap() };
    let lib: serde_json::Value = serde_json::from_str(lib_json).unwrap();
    assert_eq!(lib["data"].as_array().unwrap().len(), 0);
    unsafe { engine_free_string(lib_ptr) };
    unsafe { engine_destroy(handle) };
}
