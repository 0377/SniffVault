use std::os::raw::c_char;

use allo_isolate::Isolate;
use video_sniffing_engine::{EngineError, TaskEvent};

use crate::handle::{rust_to_c_string, EngineHandle};
use crate::json_api::{err_json, ok_json};

pub fn post_task_event(port_id: i64, event: &TaskEvent) {
    let json = serde_json::to_string(event).expect("serialize TaskEvent");
    let isolate = Isolate::new(port_id);
    isolate.post(json);
}

pub fn start_event_forwarder(handle: &mut EngineHandle, port_id: i64) {
    if handle.event_forwarder.is_some() {
        return;
    }

    let rx = {
        let mut engine = match handle.engine.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        engine.take_task_event_receiver()
    };

    if rx.is_none() {
        return;
    }

    let join_handle = std::thread::spawn(move || {
        let rx = rx.expect("receiver taken above");
        for event in rx {
            post_task_event(port_id, &event);
        }
    });

    handle.event_forwarder = Some(join_handle);
}

#[no_mangle]
pub unsafe extern "C" fn engine_subscribe_task_events(
    handle: *mut EngineHandle,
    port_id: i64,
) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }

    let handle = unsafe { &mut *handle };

    if let Some(join_handle) = handle.event_forwarder.take() {
        let _ = join_handle.join();
    }

    handle.event_port = Some(port_id);
    start_event_forwarder(handle, port_id);

    rust_to_c_string(ok_json(()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_unsubscribe_task_events(handle: *mut EngineHandle) {
    if handle.is_null() {
        return;
    }

    let handle = unsafe { &mut *handle };

    if let Some(join_handle) = handle.event_forwarder.take() {
        let _ = join_handle.join();
    }

    handle.event_port = None;
}
