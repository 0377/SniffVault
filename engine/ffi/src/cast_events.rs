use std::os::raw::c_char;

use allo_isolate::Isolate;
use tokio::sync::{mpsc, oneshot};
use video_sniffing_engine::lan::CastEvent;
use video_sniffing_engine::EngineError;

use crate::handle::{rust_to_c_string, EngineHandle};
use crate::json_api::{err_json, ok_json};

pub fn post_cast_event(port_id: i64, event: &CastEvent) {
    if let Ok(json) = serde_json::to_string(event) {
        Isolate::new(port_id).post(json);
    }
}

pub fn start_cast_event_forwarder(handle: &mut EngineHandle, port_id: i64) {
    if handle.cast_event_forwarder.is_some() {
        return;
    }

    let rx = {
        let mut engine = match handle.engine.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        engine.take_cast_event_receiver()
    };

    if rx.is_none() {
        return;
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    handle.cast_event_shutdown = Some(shutdown_tx);

    let join_handle = std::thread::spawn(move || {
        forward_cast_events(port_id, rx.expect("receiver taken above"), shutdown_rx)
    });
    handle.cast_event_forwarder = Some(join_handle);
}

fn forward_cast_events(
    port_id: i64,
    mut rx: mpsc::Receiver<CastEvent>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("cast event forwarder runtime");
    let isolate = Isolate::new(port_id);
    runtime.block_on(async {
        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Some(event) => {
                            if let Ok(json) = serde_json::to_string(&event) {
                                isolate.post(json);
                            }
                        }
                        None => break,
                    }
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });
}

fn stop_cast_event_forwarder(handle: &mut EngineHandle) {
    if let Some(shutdown_tx) = handle.cast_event_shutdown.take() {
        let _ = shutdown_tx.send(());
    }

    if let Some(join_handle) = handle.cast_event_forwarder.take() {
        let _ = join_handle.join();
    }

    handle.cast_event_port = None;
}

#[no_mangle]
pub unsafe extern "C" fn engine_subscribe_cast_events(
    handle: *mut EngineHandle,
    port_id: i64,
) -> *mut c_char {
    if handle.is_null() {
        return rust_to_c_string(err_json(EngineError::InvalidArg("handle is null".into())));
    }

    let handle = unsafe { &mut *handle };
    stop_cast_event_forwarder(handle);

    handle.cast_event_port = Some(port_id);
    start_cast_event_forwarder(handle, port_id);

    rust_to_c_string(ok_json(()))
}

pub(crate) fn unsubscribe_cast_events_inner(handle: &mut EngineHandle) {
    if let Ok(mut engine) = handle.engine.lock() {
        let _ = engine.stop_cast();
    }

    stop_cast_event_forwarder(handle);
}

#[no_mangle]
pub unsafe extern "C" fn engine_unsubscribe_cast_events(handle: *mut EngineHandle) {
    if handle.is_null() {
        return;
    }

    let handle = unsafe { &mut *handle };
    unsubscribe_cast_events_inner(handle);
}
