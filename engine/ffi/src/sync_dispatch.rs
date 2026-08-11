use std::ffi::CStr;
use std::os::raw::c_char;

use serde::{Deserialize, Serialize};
use video_sniffing_engine::{Engine, EngineError, EngineSettings, SniffEvent};

use crate::events::start_event_forwarder;
use crate::handle::{rust_to_c_string, EngineHandle};
use crate::json_api::{err_json, ok_json};

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
struct EnqueueEpisodesArgs {
    list_title: String,
    season: Option<u32>,
    episodes: Vec<(u32, String, String)>,
    quality_label: Option<String>,
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
pub unsafe extern "C" fn engine_list_tasks(handle: *mut EngineHandle) -> *mut c_char {
    ffi_call(handle, |engine| engine.list_tasks())
}

#[no_mangle]
pub unsafe extern "C" fn engine_enqueue_single(
    handle: *mut EngineHandle,
    title: *const c_char,
    url: *const c_char,
    quality_label: *const c_char,
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
    ffi_call_mut(handle, |engine| {
        engine.enqueue_single(&title, &url, quality_label.as_deref())
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
    ffi_call_mut(handle, |engine| {
        engine.enqueue_episodes(
            &args.list_title,
            args.season,
            &args.episodes,
            args.quality_label.as_deref(),
        )
    })
}

#[no_mangle]
pub unsafe extern "C" fn engine_start_downloads(handle: *mut EngineHandle) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }
    let handle = unsafe { &mut *handle };
    let mut engine = match handle.engine.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return rust_to_c_string(err_json(EngineError::Message(
                "engine lock poisoned".into(),
            )));
        }
    };
    match engine.start_downloads() {
        Ok(()) => {
            let result = rust_to_c_string(ok_json(()));
            if let Some(port_id) = handle.event_port {
                drop(engine);
                start_event_forwarder(handle, port_id);
            }
            result
        }
        Err(err) => rust_to_c_string(err_json(err)),
    }
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
