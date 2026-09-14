use std::ffi::{CStr, CString};

use tempfile::tempdir;
use video_sniffing_engine::tasks::TaskStore;
use video_sniffing_engine::{DownloadTask, TaskStatus};
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{engine_list_tasks, engine_retry_task};

fn seed_failed(data_dir: &std::path::Path, id: &str, source_url: &str) {
    let store = TaskStore::open(&data_dir.join("tasks.db")).unwrap();
    store
        .upsert(&DownloadTask {
            id: id.into(),
            parent_id: None,
            season: None,
            title: id.into(),
            source_url: source_url.into(),
            quality_label: None,
            status: TaskStatus::Failed,
            progress_bytes: 0,
            total_bytes: None,
            error_message: Some("err".into()),
            output_path: None,
            library_item_id: None,
            episode_index: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            cookie_header: None,
            referer: None,
            resolved_media_url: None,
            poster_url: None,
        })
        .unwrap();
}

#[test]
fn engine_retry_task_null_url_fast_path() {
    let dir = tempdir().unwrap();
    seed_failed(dir.path(), "t1", "https://example.com/a.mp4");
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let task_id = CString::new("t1").unwrap();
    let result = unsafe { engine_retry_task(handle, task_id.as_ptr(), std::ptr::null()) };
    assert!(!result.is_null());
    let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
    assert!(json_str.contains("\"ok\":true"));
    unsafe { engine_free_string(result) };
    unsafe { engine_destroy(handle) };
}

#[test]
fn engine_retry_task_with_new_url_updates_list_tasks() {
    let dir = tempdir().unwrap();
    seed_failed(dir.path(), "t2", "https://example.com/old.mp4");
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let task_id = CString::new("t2").unwrap();
    let new_url = CString::new("https://new.example/v.mp4").unwrap();
    let result = unsafe { engine_retry_task(handle, task_id.as_ptr(), new_url.as_ptr()) };
    assert!(!result.is_null());
    unsafe { engine_free_string(result) };

    let listed = unsafe { engine_list_tasks(handle) };
    assert!(!listed.is_null());
    let json_str = unsafe { CStr::from_ptr(listed).to_str().unwrap() };
    assert!(json_str.contains("https://new.example/v.mp4"));
    assert!(json_str.contains("\"status\":\"queued\""));
    unsafe { engine_free_string(listed) };
    unsafe { engine_destroy(handle) };
}
