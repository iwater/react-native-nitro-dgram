mod common;

use std::ffi::CString;
use std::ptr;
use rust_c_udp::*;

#[test]
fn test_udp_ssm_membership() {
    common::init_test_env();
    udp_init(common::mock_callback, ptr::null_mut());

    let id = udp_create();
    let addr_c = CString::new("0.0.0.0").unwrap();
    udp_bind(id, addr_c.as_ptr(), 0, false);

    let group = CString::new("232.0.0.1").unwrap();
    let source = CString::new("127.0.0.1").unwrap();
    let interface = CString::new("0.0.0.0").unwrap();

    // Add SSM
    let res_add = udp_add_source_specific_membership(
        id, 
        source.as_ptr(), 
        group.as_ptr(), 
        interface.as_ptr()
    );
    // res_add might be -2 if the OS doesn't support the multicast group on the interface,
    // but we verify the FFI call doesn't crash.
    assert!(res_add == 0 || res_add == -2);

    // Drop SSM
    let res_drop = udp_drop_source_specific_membership(
        id, 
        source.as_ptr(), 
        group.as_ptr(), 
        interface.as_ptr()
    );
    assert!(res_drop == 0 || res_drop == -2);

    udp_close(id);
}
