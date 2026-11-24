mod common;

use std::ffi::CString;
use std::ptr;
use rust_c_udp::*;

#[test]
fn test_udp_send_receive_echo() {
    common::init_test_env();
    common::clear_events();
    udp_init(common::mock_callback, ptr::null_mut());

    let addr_str = "127.0.0.1";
    let addr_c = CString::new(addr_str).unwrap();

    // 1. Create Server (Receiver)
    let s_id = udp_create();
    let res_s = udp_bind(s_id, addr_c.as_ptr(), 0, false);
    assert_eq!(res_s, 0);
    let s_port = udp_get_local_port(s_id);

    // 2. Create Client (Sender)
    let c_id = udp_create();
    let res_c = udp_bind(c_id, addr_c.as_ptr(), 0, false);
    assert_eq!(res_c, 0);

    // 3. Send from Client to Server
    let msg = b"Hello UDP Test";
    let sent = udp_send(c_id, msg.as_ptr(), msg.len(), addr_c.as_ptr(), s_port);
    assert_eq!(sent, msg.len() as i32);

    // 4. Wait for message on Server side (Sync Loop)
    let mut found = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(2) {
        if let Some(event) = common::get_event(s_id, common::EVENT_DATA) {
            assert_eq!(event.data, msg);
            found = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(found, "Server should have received data within 2s");

    udp_close(s_id);
    udp_close(c_id);
}

#[test]
fn test_udp_queue_monitoring() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let addr_c = CString::new("127.0.0.1").unwrap();
    udp_bind(id, addr_c.as_ptr(), 0, false);

    // Send multiple messages to self (async)
    let msg = vec![0u8; 100];
    for _ in 0..5 {
        udp_send(id, msg.as_ptr(), msg.len(), addr_c.as_ptr(), 9999);
    }

    // Since it's async, we might see some pending if we are fast
    let count = udp_get_send_queue_count(id);
    let size = udp_get_send_queue_size(id);
    assert!(count >= 0);
    assert!(size >= 0);

    udp_close(id);
}

#[test]
fn test_udp_send_multiple() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let addr_c = CString::new("127.0.0.1").unwrap();
    udp_bind(id, addr_c.as_ptr(), 0, false);

    let parts = vec![b"Part1 ".as_ptr(), b"Part2".as_ptr()];
    let lens = vec![6, 5];
    let total_len = 11;

    let sent = udp_send_multiple(id, parts.as_ptr(), lens.as_ptr(), 2, addr_c.as_ptr(), 9999);
    assert_eq!(sent, total_len as i32);

    udp_close(id);
}

#[test]
fn test_udp_connect_disconnect() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let addr_c = CString::new("127.0.0.1").unwrap();
    udp_bind(id, addr_c.as_ptr(), 0, false);

    // Connect just sets default destination in OS
    let res = udp_connect(id, addr_c.as_ptr(), 8888);
    assert_eq!(res, 0);

    let res_disc = udp_disconnect(id);
    assert_eq!(res_disc, -5);

    udp_close(id);
}
