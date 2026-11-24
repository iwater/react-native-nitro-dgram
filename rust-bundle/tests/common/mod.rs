use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::{Mutex, Once};
use lazy_static::lazy_static;

pub const EVENT_DATA: c_int = 1;
pub const EVENT_ERROR: c_int = 2;
pub const EVENT_CLOSE: c_int = 3;
pub const EVENT_CONNECT: c_int = 4;

#[derive(Debug, Clone)]
pub struct ReceivedEvent {
    pub id: u32,
    pub event_type: c_int,
    pub data: Vec<u8>,
    pub address: String,
    pub port: i32,
}

lazy_static! {
    pub static ref RECEIVED_EVENTS: Mutex<Vec<ReceivedEvent>> = Mutex::new(Vec::new());
}

static INIT: Once = Once::new();

pub fn init_test_env() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

pub fn clear_events() {
    RECEIVED_EVENTS.lock().unwrap().clear();
}

pub fn get_events() -> Vec<ReceivedEvent> {
    RECEIVED_EVENTS.lock().unwrap().clone()
}

pub fn get_event(id: u32, event_type: c_int) -> Option<ReceivedEvent> {
    RECEIVED_EVENTS
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|e| e.id == id && e.event_type == event_type)
        .cloned()
}

pub async fn wait_for_event(id: u32, event_type: c_int, timeout_ms: u64) -> Option<ReceivedEvent> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    loop {
        if let Some(event) = get_event(id, event_type) {
            return Some(event);
        }
        if start.elapsed() > timeout {
            return None;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

pub extern "C" fn mock_callback(
    id: u32,
    event_type: c_int,
    data: *const u8,
    len: usize,
    address: *const c_char,
    port: c_int,
    _context: *mut c_void,
) {
    let data_vec = if data.is_null() || len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, len).to_vec() }
    };

    let addr_str = if address.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(address).to_string_lossy().into_owned() }
    };

    // println!("[MOCK CB] Received event {} for ID {}, data len: {}", event_type, id, data_vec.len());

    let event = ReceivedEvent {
        id,
        event_type,
        data: data_vec,
        address: addr_str,
        port,
    };

    RECEIVED_EVENTS.lock().unwrap().push(event);
}
