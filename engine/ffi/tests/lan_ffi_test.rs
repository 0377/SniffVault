use std::ffi::{CStr, CString};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use video_sniffing_engine_ffi::cast_events::{
    engine_subscribe_cast_events, engine_unsubscribe_cast_events,
};
use video_sniffing_engine_ffi::handle::{engine_destroy, engine_free_string, engine_open};
use video_sniffing_engine_ffi::sync_dispatch::{
    engine_apply_lan_settings, engine_discover_peers, engine_list_trusted_peers,
    engine_pairing_pin, engine_save_settings, engine_settings, engine_stop_lan,
};

static LAN_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lan_test_guard() -> std::sync::MutexGuard<'static, ()> {
    LAN_TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

fn parse_response(ptr: *mut std::os::raw::c_char) -> serde_json::Value {
    let json_str = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
    serde_json::from_str(json_str).unwrap()
}

fn open_handle_with_lan_enabled(
    dir: &tempfile::TempDir,
    is_receiver: u8,
) -> *mut video_sniffing_engine_ffi::handle::EngineHandle {
    let path = CString::new(dir.path().to_str().unwrap()).unwrap();
    let handle = unsafe { engine_open(path.as_ptr()) };
    assert!(!handle.is_null());

    let settings_ptr = unsafe { engine_settings(handle) };
    let parsed = parse_response(settings_ptr);
    unsafe { engine_free_string(settings_ptr) };
    assert_eq!(parsed["ok"], true);

    let device_id = parsed["data"]["device_id"].as_str().unwrap();
    let new_settings = serde_json::json!({
        "device_id": device_id,
        "media_dir": "media",
        "max_concurrency": 2,
        "default_quality_label": "highest",
        "user_agent": null,
        "device_name": "LanFfiTest",
        "lan_enabled": true
    });
    let json = CString::new(new_settings.to_string()).unwrap();
    let save_ptr = unsafe { engine_save_settings(handle, json.as_ptr()) };
    let save_parsed = parse_response(save_ptr);
    assert_eq!(save_parsed["ok"], true);
    unsafe { engine_free_string(save_ptr) };

    let apply_ptr = unsafe { engine_apply_lan_settings(handle, is_receiver) };
    let apply_parsed = parse_response(apply_ptr);
    assert_eq!(apply_parsed["ok"], true);
    unsafe { engine_free_string(apply_ptr) };

    handle
}

#[test]
fn apply_lan_settings_starts_sender_service() {
    let _guard = lan_test_guard();
    let dir = tempdir().unwrap();
    let handle = open_handle_with_lan_enabled(&dir, 0);

    let peers_ptr = unsafe { engine_list_trusted_peers(handle) };
    let peers_parsed = parse_response(peers_ptr);
    assert_eq!(peers_parsed["ok"], true);
    assert_eq!(peers_parsed["data"], serde_json::json!([]));
    unsafe { engine_free_string(peers_ptr) };

    let stop_ptr = unsafe { engine_stop_lan(handle) };
    let stop_parsed = parse_response(stop_ptr);
    assert_eq!(stop_parsed["ok"], true);
    unsafe { engine_free_string(stop_ptr) };

    unsafe { engine_destroy(handle) };
}

#[test]
fn apply_lan_settings_receiver_exposes_pairing_pin() {
    let _guard = lan_test_guard();
    let dir = tempdir().unwrap();
    let handle = open_handle_with_lan_enabled(&dir, 1);

    let pin_ptr = unsafe { engine_pairing_pin(handle) };
    let pin_parsed = parse_response(pin_ptr);
    assert_eq!(pin_parsed["ok"], true);
    let pin = pin_parsed["data"].as_str().unwrap();
    assert_eq!(pin.len(), 6);
    assert!(pin.chars().all(|c| c.is_ascii_digit()));
    unsafe { engine_free_string(pin_ptr) };

    unsafe { engine_destroy(handle) };
}

#[test]
fn discover_peers_completes_within_three_second_timeout() {
    let _guard = lan_test_guard();
    let dir = tempdir().unwrap();
    let handle = open_handle_with_lan_enabled(&dir, 0);

    let started = Instant::now();
    let peers_ptr = unsafe { engine_discover_peers(handle) };
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(4),
        "discover_peers took {:?}, expected under 4s",
        elapsed
    );

    let peers_parsed = parse_response(peers_ptr);
    assert_eq!(peers_parsed["ok"], true);
    assert!(peers_parsed["data"].is_array());
    unsafe { engine_free_string(peers_ptr) };

    unsafe { engine_destroy(handle) };
}

#[test]
fn destroy_stops_active_lan_without_hanging() {
    let _guard = lan_test_guard();
    let dir = tempdir().unwrap();
    let handle = open_handle_with_lan_enabled(&dir, 0);

    let started = Instant::now();
    unsafe { engine_destroy(handle) };
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "engine_destroy with active LAN took too long"
    );
}

#[test]
fn subscribe_cast_events_returns_ok() {
    let _guard = lan_test_guard();
    let dir = tempdir().unwrap();
    let handle = open_handle_with_lan_enabled(&dir, 1);

    let sub_ptr = unsafe { engine_subscribe_cast_events(handle, 4242) };
    let sub_parsed = parse_response(sub_ptr);
    assert_eq!(sub_parsed["ok"], true);
    unsafe { engine_free_string(sub_ptr) };

    unsafe { engine_unsubscribe_cast_events(handle) };
    unsafe { engine_destroy(handle) };
}
