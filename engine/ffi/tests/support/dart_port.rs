use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use allo_isolate::ffi::{run_destructors, DartCObject, DartCObjectType};

static SETUP: OnceLock<()> = OnceLock::new();
static PORTS: OnceLock<Mutex<HashMap<i64, Sender<String>>>> = OnceLock::new();
static NEXT_PORT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

fn ports() -> &'static Mutex<HashMap<i64, Sender<String>>> {
    PORTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn setup() {
    SETUP.get_or_init(|| unsafe {
        allo_isolate::store_dart_post_cobject(dart_post_cobject);
    });
}

unsafe fn extract_string(object: &DartCObject) -> Option<String> {
    if object.ty != DartCObjectType::DartString {
        return None;
    }
    let ptr = object.value.as_string;
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr)
        .to_str()
        .ok()
        .map(|value| value.to_string())
}

extern "C" fn dart_post_cobject(port: i64, object: *mut DartCObject) -> bool {
    let message = unsafe {
        let object = &*object;
        let message = extract_string(object);
        run_destructors(object);
        message
    };

    let Some(message) = message else {
        return false;
    };

    if let Ok(ports) = ports().lock() {
        if let Some(tx) = ports.get(&port) {
            return tx.send(message).is_ok();
        }
    }
    false
}

pub struct Isolate;

impl Isolate {
    pub fn create_dart_port() -> (i64, Receiver<String>) {
        setup();
        let (tx, rx) = std::sync::mpsc::channel();
        let port = NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        ports().lock().expect("ports lock").insert(port, tx);
        (port, rx)
    }
}
