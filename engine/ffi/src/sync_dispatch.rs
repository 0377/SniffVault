use std::ffi::CStr;
use std::os::raw::c_char;

use serde::Serialize;
use video_sniffing_engine::{Engine, EngineError, EngineSettings};

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

fn parse_c_str(ptr: *const c_char, name: &str) -> Result<String, EngineError> {
    if ptr.is_null() {
        return Err(EngineError::InvalidArg(format!("{name} is null")));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| EngineError::InvalidArg(format!("{name} is not valid UTF-8")))
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
