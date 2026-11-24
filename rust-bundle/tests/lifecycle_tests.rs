mod common;

use std::ffi::CString;
use std::ptr;
use rust_c_udp::*;

#[test]
fn test_udp_lifecycle_basic() {
    common::init_test_env();
    common::clear_events();

    // 1. Init
    udp_init(common::mock_callback, ptr::null_mut());

    // 2. Create
    let id = udp_create();
    assert!(id > 0);

    // 3. Bind
    let host = CString::new("127.0.0.1").unwrap();
    let res_bind = udp_bind(id, host.as_ptr(), 0, false);
    assert_eq!(res_bind, 0);

    // 4. Get Port
    let port = udp_get_local_port(id);
    assert!(port > 0);

    // 5. Close
    udp_close(id);

    // Wait for the close event to be captured by mock_callback
    // (Rust core emits EVENT_CLOSE after the receive loop breaks)
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let close_event = common::get_event(id, common::EVENT_CLOSE);
    assert!(close_event.is_some(), "Should have received close event");
}

#[test]
fn test_udp_init_multiple_times() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());
    udp_init_with_config(common::mock_callback, ptr::null_mut(), 4);
    // Should not crash
}

#[test]
fn test_udp_port_reuse_regression() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let host = CString::new("127.0.0.1").unwrap();
    let test_port = 34567; // Use a fixed port for reuse test

    // First bind
    let id1 = udp_create();
    let res1 = udp_bind(id1, host.as_ptr(), test_port, false);
    if res1 != 0 {
        // Port might be busy in the environment, skip
        println!("Port {} busy, skipping reuse test", test_port);
        udp_close(id1);
        return;
    }

    // Close
    udp_close(id1);
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Second bind to same port
    let id2 = udp_create();
    let res2 = udp_bind(id2, host.as_ptr(), test_port, false);
    assert_eq!(res2, 0, "Should be able to rebind to same port");

    udp_close(id2);
}
