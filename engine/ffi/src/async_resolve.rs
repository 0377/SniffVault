use std::os::raw::c_char;

use allo_isolate::Isolate;
use serde::Serialize;
use serde_json::json;
use video_sniffing_engine::{
    resolve_qualities_for_ffi, resolve_url_for_ffi, EngineError, ResolveOptions,
};

use crate::handle::{rust_to_c_string, EngineHandle};
use crate::json_api::{err_json, map_engine_error, ok_json};
use crate::sync_dispatch::{parse_c_str, parse_json_c_str};

#[derive(Serialize)]
struct AcceptedResponse {
    accepted: bool,
    request_id: String,
}

fn parse_resolve_opts(opts_json: *const c_char) -> Result<ResolveOptions, EngineError> {
    if opts_json.is_null() {
        return Ok(ResolveOptions::default());
    }
    parse_json_c_str(opts_json, "opts_json")
}

fn spawn_resolve_url(
    handle: &EngineHandle,
    url: String,
    opts: ResolveOptions,
    port_id: i64,
    request_id: String,
) {
    let user_agent = {
        let engine = match handle.engine.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        engine.settings().user_agent.clone()
    };

    handle.runtime.spawn(async move {
        let result = resolve_url_for_ffi(user_agent.as_deref(), &url, opts).await;
        let callback = match result {
            Ok(data) => json!({ "request_id": request_id, "ok": true, "data": data }),
            Err(err) => json!({
                "request_id": request_id,
                "ok": false,
                "error": map_engine_error(err),
            }),
        };
        Isolate::new(port_id).post(callback.to_string());
    });
}

fn spawn_resolve_qualities(
    handle: &EngineHandle,
    media_url: String,
    opts: ResolveOptions,
    port_id: i64,
    request_id: String,
) {
    let user_agent = {
        let engine = match handle.engine.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        engine.settings().user_agent.clone()
    };

    handle.runtime.spawn(async move {
        let result = resolve_qualities_for_ffi(user_agent.as_deref(), &media_url, opts).await;
        let callback = match result {
            Ok(data) => json!({ "request_id": request_id, "ok": true, "data": data }),
            Err(err) => json!({
                "request_id": request_id,
                "ok": false,
                "error": map_engine_error(err),
            }),
        };
        Isolate::new(port_id).post(callback.to_string());
    });
}

#[no_mangle]
pub unsafe extern "C" fn engine_resolve_url_async(
    handle: *mut EngineHandle,
    url: *const c_char,
    opts_json: *const c_char,
    port_id: i64,
    request_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }

    let url = match parse_c_str(url, "url") {
        Ok(value) => value,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let opts = match parse_resolve_opts(opts_json) {
        Ok(value) => value,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let request_id = match parse_c_str(request_id, "request_id") {
        Ok(value) => value,
        Err(err) => return rust_to_c_string(err_json(err)),
    };

    let handle_ref = unsafe { &*handle };
    spawn_resolve_url(handle_ref, url, opts, port_id, request_id.clone());

    rust_to_c_string(ok_json(AcceptedResponse {
        accepted: true,
        request_id,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn engine_resolve_qualities_async(
    handle: *mut EngineHandle,
    media_url: *const c_char,
    opts_json: *const c_char,
    port_id: i64,
    request_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }

    let media_url = match parse_c_str(media_url, "media_url") {
        Ok(value) => value,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let opts = match parse_resolve_opts(opts_json) {
        Ok(value) => value,
        Err(err) => return rust_to_c_string(err_json(err)),
    };
    let request_id = match parse_c_str(request_id, "request_id") {
        Ok(value) => value,
        Err(err) => return rust_to_c_string(err_json(err)),
    };

    let handle_ref = unsafe { &*handle };
    spawn_resolve_qualities(handle_ref, media_url, opts, port_id, request_id.clone());

    rust_to_c_string(ok_json(AcceptedResponse {
        accepted: true,
        request_id,
    }))
}
