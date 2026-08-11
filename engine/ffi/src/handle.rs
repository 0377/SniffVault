use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use std::thread::JoinHandle;

use tokio::runtime::Runtime;
use video_sniffing_engine::{Engine, EngineError};

use crate::json_api::err_json;

thread_local! {
    static LAST_OPEN_ERROR: RefCell<Option<EngineError>> = RefCell::new(None);
}

pub struct EngineHandle {
    pub engine: Mutex<Engine>,
    pub runtime: Runtime,
    pub event_port: Option<i64>,
    pub event_forwarder: Option<JoinHandle<()>>,
}

fn rust_to_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("").expect("empty CString"))
        .into_raw()
}

fn clear_last_open_error() {
    LAST_OPEN_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn set_last_open_error(err: EngineError) {
    LAST_OPEN_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(err);
    });
}

#[no_mangle]
pub unsafe extern "C" fn engine_open(data_dir: *const c_char) -> *mut EngineHandle {
    clear_last_open_error();

    if data_dir.is_null() {
        set_last_open_error(EngineError::InvalidArg("data_dir is null".into()));
        return std::ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(data_dir).to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_open_error(EngineError::InvalidArg("data_dir is not valid UTF-8".into()));
            return std::ptr::null_mut();
        }
    };

    let engine = match Engine::open(path_str) {
        Ok(engine) => engine,
        Err(err) => {
            set_last_open_error(err);
            return std::ptr::null_mut();
        }
    };

    let runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            set_last_open_error(EngineError::from(err));
            return std::ptr::null_mut();
        }
    };

    let handle = Box::new(EngineHandle {
        engine: Mutex::new(engine),
        runtime,
        event_port: None,
        event_forwarder: None,
    });

    Box::into_raw(handle)
}

#[no_mangle]
pub unsafe extern "C" fn engine_last_error() -> *mut c_char {
    let err = LAST_OPEN_ERROR.with(|cell| cell.borrow_mut().take());
    match err {
        Some(err) => rust_to_c_string(err_json(err)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_destroy(handle: *mut EngineHandle) {
    if handle.is_null() {
        return;
    }

    let mut boxed = Box::from_raw(handle);

    if let Some(join_handle) = boxed.event_forwarder.take() {
        let _ = join_handle.join();
    }

    if let Ok(mut engine) = boxed.engine.lock() {
        let _ = engine.stop_downloads();
    }

    drop(boxed);
}

#[no_mangle]
pub unsafe extern "C" fn engine_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}
