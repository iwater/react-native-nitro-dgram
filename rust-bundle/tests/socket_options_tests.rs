mod common;

use std::ffi::CString;
use std::ptr;
use rust_c_udp::*;

#[test]
fn test_udp_socket_options() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let addr_c = CString::new("127.0.0.1").unwrap();
    udp_bind(id, addr_c.as_ptr(), 0, false);

    // Broadcast
    assert_eq!(udp_set_broadcast(id, true), 0);
    
    // TTL
    assert_eq!(udp_set_ttl(id, 64), 0);
    
    // Multicast V4
    assert_eq!(udp_set_multicast_ttl_v4(id, 32), 0);
    assert_eq!(udp_set_multicast_loop_v4(id, true), 0);

    udp_close(id);
}

#[test]
fn test_udp_buffer_sizes() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let addr_c = CString::new("127.0.0.1").unwrap();
    udp_bind(id, addr_c.as_ptr(), 0, false);

    // Get current
    let recv_size = udp_get_buffer_size(id, false);
    let send_size = udp_get_buffer_size(id, true);
    assert!(recv_size > 0);
    assert!(send_size > 0);

    // Set new
    udp_set_buffer_size(id, false, recv_size * 2);
    udp_set_buffer_size(id, true, send_size * 2);

    // Note: OS might not give exactly what we ask, but it should be successful (0)
    assert_eq!(udp_set_buffer_size(id, false, 65536), 0);

    udp_close(id);
}

#[test]
fn test_udp_address_info() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let host = "127.0.0.1";
    let addr_c = CString::new(host).unwrap();
    let test_port = 0; // Random
    udp_bind(id, addr_c.as_ptr(), test_port, false);
    
    let local_port = udp_get_local_port(id);
    assert!(local_port > 0);

    let mut buf = [0; 64];
    let res = udp_get_local_addr(id, buf.as_mut_ptr() as *mut i8, buf.len());
    assert_eq!(res, 0);
    
    let addr_str = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const i8).to_str().unwrap() };
    assert_eq!(addr_str, host);

    udp_close(id);
}
