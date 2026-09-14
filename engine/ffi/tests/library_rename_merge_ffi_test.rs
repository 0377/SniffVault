use std::ffi::{CStr, CString};
use std::fs;
use tempfile::tempdir;
use video_sniffing_engine::test_api::LibraryStore;
use video_sniffing_engine::{Engine, LibraryEpisode, LibraryItem, LibraryItemKind};
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_list_library, engine_merge_library_items, engine_rename_library_item,
};

fn seed_dup(engine: &Engine) -> (String, String) {
    let store = LibraryStore::open(
        &engine.media_dir().parent().unwrap().join("library.db"),
    )
    .unwrap();
    let a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string();
    let b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb".to_string();
    for (id, created) in [(&a, 1_i64), (&b, 2_i64)] {
        store
            .upsert_item(&LibraryItem {
                id: id.clone(),
                kind: LibraryItemKind::Series,
                title: "示意剧".into(),
                season: Some(1),
                poster_path: None,
                created_at_ms: created,
            })
            .unwrap();
    }
    (a, b)
}

#[test]
fn rename_and_merge_ffi_ok_json() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("旧", media.to_str().unwrap(), None)
        .unwrap();
    let (src, tgt) = seed_dup(&engine);
    let store = LibraryStore::open(
        &engine.media_dir().parent().unwrap().join("library.db"),
    )
    .unwrap();
    let ep = LibraryEpisode {
        id: "cccccccc-cccc-cccc-cccc-cccccccccccc".to_string(),
        item_id: src.clone(),
        index: 1,
        title: "源1".into(),
        file_path: engine.media_dir().join("s1.mp4").to_string_lossy().into(),
        duration_ms: None,
        position_ms: 0,
        source_url: None,
    };
    fs::write(engine.media_dir().join("s1.mp4"), b"x").unwrap();
    store.upsert_episode(&ep).unwrap();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    let item_id = CString::new(item.id).unwrap();
    let new_title = CString::new("新").unwrap();
    let rename_ptr =
        unsafe { engine_rename_library_item(handle, item_id.as_ptr(), new_title.as_ptr()) };
    let rename_json: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(rename_ptr).to_str().unwrap() }).unwrap();
    assert_eq!(rename_json["ok"], true);
    unsafe { engine_free_string(rename_ptr) };

    let src_id = CString::new(src).unwrap();
    let tgt_id = CString::new(tgt).unwrap();
    let merge_ptr =
        unsafe { engine_merge_library_items(handle, src_id.as_ptr(), tgt_id.as_ptr(), 0) };
    let merge_json: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(merge_ptr).to_str().unwrap() }).unwrap();
    assert_eq!(merge_json["ok"], true);
    unsafe { engine_free_string(merge_ptr) };

    let lib_ptr = unsafe { engine_list_library(handle) };
    let lib: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(lib_ptr).to_str().unwrap() }).unwrap();
    assert_eq!(lib["data"].as_array().unwrap().len(), 2); // single + merged series
    unsafe { engine_free_string(lib_ptr) };
    unsafe { engine_destroy(handle) };
}

#[test]
fn rename_ffi_invalid_title_returns_error_json() {
    let dir = tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let media = engine.media_dir().join("a.mp4");
    fs::write(&media, b"x").unwrap();
    let (item, _) = engine
        .register_completed_single("x", media.to_str().unwrap(), None)
        .unwrap();
    drop(engine);

    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    let item_id = CString::new(item.id).unwrap();
    let blank = CString::new("   ").unwrap();
    let ptr = unsafe { engine_rename_library_item(handle, item_id.as_ptr(), blank.as_ptr()) };
    let v: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(ptr).to_str().unwrap() }).unwrap();
    assert_ne!(v.get("ok"), Some(&serde_json::Value::Bool(true)));
    unsafe { engine_free_string(ptr) };
    unsafe { engine_destroy(handle) };
}
