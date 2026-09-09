use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{atomic::Ordering, Arc};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use video_sniffing_engine::{DownloadAuth, Engine, EngineError, EngineSettings, SniffEvent};

use crate::events::start_event_forwarder;
use crate::handle::{rust_to_c_string, EngineHandle};
use crate::json_api::{err_json, ok_json};

const DISCOVER_PEERS_FFI_TIMEOUT: Duration = Duration::from_millis(3250);

fn ffi_call<R, F>(handle: *mut EngineHandle, f: F) -> *mut c_char
where
    R: Serialize,
    F: FnOnce(&Engine) -> Result<R, EngineError>,
{
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }
    let handle = unsafe { &*handle };
    let engine = match handle.engine.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return rust_to_c_string(err_json(EngineError::Message(
                "engine lock poisoned".into(),
            )));
        }
    };
    match f(&engine) {
        Ok(data) => rust_to_c_string(ok_json(data)),
        Err(err) => rust_to_c_string(err_json(err)),
    }
}

fn ffi_call_mut<R, F>(handle: *mut EngineHandle, f: F) -> *mut c_char
where
    R: Serialize,
    F: FnOnce(&mut Engine) -> Result<R, EngineError>,
{
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }
    let handle = unsafe { &*handle };
    let mut engine = match handle.engine.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return rust_to_c_string(err_json(EngineError::Message(
                "engine lock poisoned".into(),
            )));
        }
    };
    match f(&mut engine) {
        Ok(data) => rust_to_c_string(ok_json(data)),
        Err(err) => rust_to_c_string(err_json(err)),
    }
}

pub(crate) fn parse_c_str(ptr: *const c_char, name: &str) -> Result<String, EngineError> {
    if ptr.is_null() {
        return Err(EngineError::InvalidArg(format!("{name} is null")));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| EngineError::InvalidArg(format!("{name} is not valid UTF-8")))
}

pub(crate) fn parse_optional_c_str(
    ptr: *const c_char,
    name: &str,
) -> Result<Option<String>, EngineError> {
    if ptr.is_null() {
        return Ok(None);
    }
    parse_c_str(ptr, name).map(Some)
}

pub(crate) fn parse_json_c_str<T: for<'de> Deserialize<'de>>(
    ptr: *const c_char,
    name: &str,
) -> Result<T, EngineError> {
    let json_str = parse_c_str(ptr, name)?;
    serde_json::from_str(&json_str).map_err(EngineError::Serde)
}

#[derive(Debug, Deserialize)]
struct EnqueueAuthJson {
    cookies: Option<String>,
    referer: Option<String>,
}

fn parse_enqueue_auth(opts_json: *const c_char) -> Result<Option<DownloadAuth>, EngineError> {
    if opts_json.is_null() {
        return Ok(None);
    }
    let parsed: EnqueueAuthJson = parse_json_c_str(opts_json, "opts_json")?;
    Ok(Some(DownloadAuth {
        cookies: parsed.cookies,
        referer: parsed.referer,
    }))
}

#[derive(Debug, Deserialize)]
struct EnqueueEpisodesArgs {
    list_title: String,
    season: Option<u32>,
    episodes: Vec<(u32, String, String)>,
    quality_label: Option<String>,
    #[serde(default)]
    cookies: Option<String>,
    #[serde(default)]
    referer: Option<String>,
}

#[no_mangle]
pub unsafe extern "C" fn engine_settings(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| Ok(engine.settings()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_save_settings(
    handle: *mut EngineHandle,
    json: *const c_char,
) -> *mut c_char {
    let json_str = match parse_c_str(json, "json") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let settings: EngineSettings = match serde_json::from_str(&json_str) {
        Ok(settings) => settings,
        Err(err) => return rust_to_c_string(err_json(EngineError::Serde(err))),
    };
    ffi_call_mut(handle, |engine| engine.save_settings(settings).map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_list_library(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| engine.list_library())
}

#[no_mangle]
pub unsafe extern "C" fn engine_list_episodes(
    handle: *mut EngineHandle,
    item_id: *const c_char,
) -> *mut c_char {
    let item_id = match parse_c_str(item_id, "item_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call(handle, |engine| engine.list_episodes(&item_id))
}

#[no_mangle]
pub unsafe extern "C" fn engine_set_episode_position(
    handle: *mut EngineHandle,
    episode_id: *const c_char,
    position_ms: i64,
) -> *mut c_char {
    let episode_id = match parse_c_str(episode_id, "episode_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call(handle, |engine| {
        engine
            .set_episode_position(&episode_id, position_ms)
            .map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_remove_library_item(
    handle: *mut EngineHandle,
    item_id: *const c_char,
    delete_files: u8,
) -> *mut c_char {
    let item_id = match parse_c_str(item_id, "item_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let delete_files = delete_files != 0;
    ffi_call_mut(handle, |engine| {
        engine
            .remove_library_item(&item_id, delete_files)
            .map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_remove_episode(
    handle: *mut EngineHandle,
    episode_id: *const c_char,
    delete_files: u8,
) -> *mut c_char {
    let episode_id = match parse_c_str(episode_id, "episode_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let delete_files = delete_files != 0;
    ffi_call_mut(handle, |engine| {
        engine.remove_episode(&episode_id, delete_files).map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_list_tasks(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| engine.list_tasks())
}

#[no_mangle]
pub unsafe extern "C" fn engine_enqueue_single(
    handle: *mut EngineHandle,
    title: *const c_char,
    url: *const c_char,
    quality_label: *const c_char,
    opts_json: *const c_char,
) -> *mut c_char {
    let title = match parse_c_str(title, "title") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let url = match parse_c_str(url, "url") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let quality_label = match parse_optional_c_str(quality_label, "quality_label") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let auth = match parse_enqueue_auth(opts_json) {
        Ok(auth) => auth,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| {
        engine.enqueue_single(&title, &url, quality_label.as_deref(), auth.as_ref())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_enqueue_episodes(
    handle: *mut EngineHandle,
    args_json: *const c_char,
) -> *mut c_char {
    let args: EnqueueEpisodesArgs = match parse_json_c_str(args_json, "args_json") {
        Ok(args) => args,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let auth = DownloadAuth {
        cookies: args.cookies,
        referer: args.referer,
    };
    ffi_call_mut(handle, |engine| {
        engine.enqueue_episodes(
            &args.list_title,
            args.season,
            &args.episodes,
            args.quality_label.as_deref(),
            Some(&auth),
        )
    })
}

fn spawn_download_worker_deferred(handle: &mut EngineHandle) {
    if let Some(join_handle) = handle.deferred_spawn.take() {
        let _ = join_handle.join();
    }

    let engine = Arc::clone(&handle.engine);
    let alive = Arc::clone(&handle.alive);
    let join_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        if !alive.load(Ordering::Acquire) {
            return;
        }
        let mut engine = match engine.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let _ = engine.spawn_download_worker();
    });
    handle.deferred_spawn = Some(join_handle);
}

#[no_mangle]
pub unsafe extern "C" fn engine_start_downloads(handle: *mut EngineHandle) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }
    let handle_ref = unsafe { &mut *handle };
    {
        let mut engine = match handle_ref.engine.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return rust_to_c_string(err_json(EngineError::Message(
                    "engine lock poisoned".into(),
                )));
            }
        };
        match engine.prepare_download_events() {
            Ok(()) => {}
            Err(err) => return rust_to_c_string(err_json(err)),
        }
    }

    if let Some(port_id) = handle_ref.event_port {
        start_event_forwarder(handle_ref, port_id);
    }

    spawn_download_worker_deferred(handle_ref);

    rust_to_c_string(ok_json(()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_spawn_download_worker(handle: *mut EngineHandle) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }
    let handle_ref = unsafe { &mut *handle };
    if let Some(port_id) = handle_ref.event_port {
        start_event_forwarder(handle_ref, port_id);
    }
    spawn_download_worker_deferred(handle_ref);
    rust_to_c_string(ok_json(()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_stop_downloads(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call_mut(handle, |engine| engine.stop_downloads().map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_pause_task(
    handle: *mut EngineHandle,
    task_id: *const c_char,
) -> *mut c_char {
    let task_id = match parse_c_str(task_id, "task_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| engine.pause_task(&task_id))
}

#[no_mangle]
pub unsafe extern "C" fn engine_resume_task(
    handle: *mut EngineHandle,
    task_id: *const c_char,
) -> *mut c_char {
    let task_id = match parse_c_str(task_id, "task_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| engine.resume_task(&task_id))
}

#[no_mangle]
pub unsafe extern "C" fn engine_cancel_task(
    handle: *mut EngineHandle,
    task_id: *const c_char,
) -> *mut c_char {
    let task_id = match parse_c_str(task_id, "task_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| engine.cancel_task(&task_id))
}

#[no_mangle]
pub unsafe extern "C" fn engine_set_task_media_url(
    handle: *mut EngineHandle,
    task_id: *const c_char,
    media_url: *const c_char,
) -> *mut c_char {
    let task_id = match parse_c_str(task_id, "task_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let media_url = match parse_c_str(media_url, "media_url") {
        Ok(url) => url,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| {
        engine.set_task_media_url(&task_id, &media_url).map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_sniff_urls(
    handle: *mut EngineHandle,
    events_json: *const c_char,
    page_url: *const c_char,
) -> *mut c_char {
    let events: Vec<SniffEvent> = match parse_json_c_str(events_json, "events_json") {
        Ok(events) => events,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let page_url = match parse_optional_c_str(page_url, "page_url") {
        Ok(s) => s,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call(handle, |engine| {
        Ok(engine.sniff_urls(&events, page_url.as_deref()))
    })
}

fn parse_is_receiver(is_receiver: u8) -> Result<bool, EngineError> {
    match is_receiver {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(EngineError::InvalidArg("is_receiver must be 0 or 1".into())),
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_start_lan(
    handle: *mut EngineHandle,
    is_receiver: u8,
) -> *mut c_char {
    let is_receiver = match parse_is_receiver(is_receiver) {
        Ok(v) => v,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| engine.start_lan(is_receiver).map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_stop_lan(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call_mut(handle, |engine| engine.stop_lan().map(|_| ()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_lan_http_port(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| Ok(engine.lan_http_port()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_apply_lan_settings(
    handle: *mut EngineHandle,
    is_receiver: u8,
) -> *mut c_char {
    let is_receiver = match parse_is_receiver(is_receiver) {
        Ok(v) => v,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| {
        engine.apply_lan_settings(is_receiver).map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_discover_peers(handle: *mut EngineHandle) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }
    let handle = unsafe { &*handle };
    let engine = Arc::clone(&handle.engine);

    let result = handle.runtime.block_on(async move {
        tokio::time::timeout(
            DISCOVER_PEERS_FFI_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                let mut engine = engine
                    .lock()
                    .map_err(|_| EngineError::Message("engine lock poisoned".into()))?;
                engine.discover_peers()
            }),
        )
        .await
        .map_err(|_| EngineError::Message("discover peers timed out".into()))?
        .map_err(|err| EngineError::Message(format!("discover peers failed: {err}")))?
    });

    match result {
        Ok(peers) => rust_to_c_string(ok_json(peers)),
        Err(err) => rust_to_c_string(err_json(err)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_begin_pairing(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call_mut(handle, |engine| engine.begin_pairing())
}

#[no_mangle]
pub unsafe extern "C" fn engine_pairing_pin(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| {
        let pin = engine.pairing_pin()?;
        Ok(pin.unwrap_or_default())
    })
}

#[derive(Debug, Deserialize)]
struct PairPeerArgs {
    host: String,
    port: u16,
    pin: String,
}

#[no_mangle]
pub unsafe extern "C" fn engine_pair_peer(
    handle: *mut EngineHandle,
    args_json: *const c_char,
) -> *mut c_char {
    let args: PairPeerArgs = match parse_json_c_str(args_json, "args_json") {
        Ok(args) => args,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| {
        engine
            .pair_peer(&args.host, args.port, &args.pin)
            .map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_list_trusted_peers(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| engine.list_trusted_peers())
}

#[no_mangle]
pub unsafe extern "C" fn engine_remove_trusted_peer(
    handle: *mut EngineHandle,
    peer_device_id: *const c_char,
) -> *mut c_char {
    let peer_device_id = match parse_c_str(peer_device_id, "peer_device_id") {
        Ok(id) => id,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call(handle, |engine| engine.remove_trusted_peer(&peer_device_id))
}

#[derive(Debug, Deserialize)]
struct CastEpisodeArgs {
    episode_id: String,
    peer_device_id: String,
}

#[no_mangle]
pub unsafe extern "C" fn engine_cast_episode(
    handle: *mut EngineHandle,
    args_json: *const c_char,
) -> *mut c_char {
    let args: CastEpisodeArgs = match parse_json_c_str(args_json, "args_json") {
        Ok(args) => args,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    ffi_call_mut(handle, |engine| {
        engine
            .cast_episode(&args.episode_id, &args.peer_device_id)
            .map(|_| ())
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_stop_cast(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call_mut(handle, |engine| engine.stop_cast().map(|_| ()))
}
