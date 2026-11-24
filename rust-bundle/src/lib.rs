use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use lazy_static::lazy_static;
use log::{error, info, debug};
use tokio::net::UdpSocket;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use once_cell::sync::OnceCell;

// -----------------------------------------------------------------------------
// Types / C Callback Definitions
// -----------------------------------------------------------------------------

type CallbackPtr = extern "C" fn(
    id: u32, 
    event_type: c_int, 
    data: *const u8, 
    len: usize, 
    address: *const c_char, 
    port: c_int, 
    context: *mut c_void
);

const EVENT_DATA: c_int = 1;
const EVENT_ERROR: c_int = 2;
const EVENT_CLOSE: c_int = 3;
const EVENT_CONNECT: c_int = 4;

#[repr(transparent)]
#[derive(Clone, Copy)]
struct SendPtr(*mut c_void);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

// -----------------------------------------------------------------------------
// Global Runtime & State
// -----------------------------------------------------------------------------

// Resource limits for send queue
const MAX_PENDING_SENDS: u32 = 1000;
const MAX_PENDING_BYTES: usize = 10 * 1024 * 1024; // 10MB

struct SocketContext {
    id: u32,
    shutdown_tx: Option<mpsc::Sender<()>>,
    socket: Arc<UdpSocket>,
    pending_sends: Arc<AtomicU32>,
    pending_bytes: Arc<AtomicUsize>,
}

/// Runtime 配置结构
#[repr(C)]
pub struct UdpConfig {
    /// 工作线程数，0 表示使用默认值 (CPU 核心数)
    pub worker_threads: u32,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0,
        }
    }
}

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        init_runtime_with_config(UdpConfig::default())
    })
}

fn init_runtime_with_config(config: UdpConfig) -> Runtime {
    let mut builder = Builder::new_multi_thread();
    
    if config.worker_threads > 0 {
        builder.worker_threads(config.worker_threads as usize);
        info!("UDP Runtime initialized with {} worker threads", config.worker_threads);
    } else {
        info!("UDP Runtime initialized with default worker threads");
    }
    
    builder.enable_all().build().expect("Failed to create Tokio runtime")
}

lazy_static! {
    static ref SOCKET_MAP: DashMap<u32, SocketContext> = DashMap::new();
    static ref NEXT_ID: AtomicU32 = AtomicU32::new(1);
    static ref GLOBAL_CALLBACK: Mutex<Option<(CallbackPtr, SendPtr)>> = Mutex::new(None);
}

fn emit_event(id: u32, event_type: c_int, data: &[u8], address: &str, port: i32) {
    let cb_guard = GLOBAL_CALLBACK.lock().unwrap();
    if let Some((cb, context_ptr)) = *cb_guard {
        let data_ptr = if data.is_empty() { std::ptr::null() } else { data.as_ptr() };
        let addr_c = CString::new(address).unwrap_or_default();
        cb(id, event_type, data_ptr, data.len(), addr_c.as_ptr(), port as c_int, context_ptr.0);
    }
}

// -----------------------------------------------------------------------------
// Socket Logic
// -----------------------------------------------------------------------------

async fn handle_udp_receive(id: u32, socket: Arc<UdpSocket>, mut shutdown_rx: mpsc::Receiver<()>) {
    let mut buf = [0u8; 65535]; // Max UDP packet size
    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((n, addr)) => {
                        let addr_str = addr.ip().to_string();
                        let port = addr.port() as i32;
                        emit_event(id, EVENT_DATA, &buf[..n], &addr_str, port);
                    }
                    Err(e) => {
                        error!("UDP socket {} receive error: {}", id, e);
                        emit_event(id, EVENT_ERROR, e.to_string().as_bytes(), "", 0);
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                debug!("UDP socket {} shutdown received", id);
                break;
            }
        }
    }
    emit_event(id, EVENT_CLOSE, &[], "", 0);
    SOCKET_MAP.remove(&id);
}

// -----------------------------------------------------------------------------
// FFI Exports
// -----------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn udp_init(callback: CallbackPtr, context: *mut c_void) {
    let _ = env_logger::try_init();
    *GLOBAL_CALLBACK.lock().unwrap() = Some((callback, SendPtr(context)));
    info!("UDP layer initialized");
}

#[no_mangle]
pub extern "C" fn udp_init_with_config(callback: CallbackPtr, context: *mut c_void, worker_threads: u32) {
    let _ = env_logger::try_init();
    *GLOBAL_CALLBACK.lock().unwrap() = Some((callback, SendPtr(context)));
    
    // Try to initialize the runtime with custom config
    // Note: If already initialized, this will have no effect
    let _ = RUNTIME.get_or_init(|| {
        init_runtime_with_config(UdpConfig { worker_threads })
    });
    
    info!("UDP layer initialized with config (worker_threads={})", worker_threads);
}

#[no_mangle]
pub extern "C" fn udp_create() -> u32 {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    debug!("UDP socket {} created (placeholder)", id);
    id
}

#[no_mangle]
pub extern "C" fn udp_bind(id: u32, host: *const c_char, port: c_int, ipv6_only: bool) -> i32 {
    let host_str = if host.is_null() {
        if ipv6_only { "::" } else { "0.0.0.0" }
    } else {
        match unsafe { CStr::from_ptr(host).to_str() } {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let addr_str = format!("{}:{}", host_str, port);
    let runtime = get_runtime();
    let (tx, rx) = mpsc::channel(1);

    // Use socket2 for explicit socket options (SO_REUSEADDR, SO_REUSEPORT)
    let domain = if ipv6_only { socket2::Domain::IPV6 } else { socket2::Domain::IPV4 };
    let socket_result = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP));
    
    let sock = match socket_result {
        Ok(s) => s,
        Err(e) => {
            error!("UDP {} failed to create socket: {}", id, e);
            return -2;
        }
    };

    // Set socket options
    if let Err(e) = sock.set_reuse_address(true) {
        debug!("UDP {} failed to set SO_REUSEADDR: {}", id, e);
    }
    #[cfg(unix)]
    {
        if let Err(e) = sock.set_reuse_port(true) {
            debug!("UDP {} failed to set SO_REUSEPORT: {}", id, e);
        }
    }
    if ipv6_only {
        if let Err(e) = sock.set_only_v6(true) {
            debug!("UDP {} failed to set IPV6_V6ONLY: {}", id, e);
        }
    }

    // Parse and bind address with retry on AddrInUse
    let addr: std::net::SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            error!("UDP {} invalid address '{}': {}", id, addr_str, e);
            return -1;
        }
    };

    let bind_with_retry = |sock: &socket2::Socket, addr: &std::net::SocketAddr| -> std::io::Result<()> {
        match sock.bind(&(*addr).into()) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                debug!("UDP {} bind failed (AddrInUse), retrying after 100ms...", id);
                std::thread::sleep(std::time::Duration::from_millis(100));
                sock.bind(&(*addr).into())
            }
            Err(e) => Err(e),
        }
    };

    if let Err(e) = bind_with_retry(&sock, &addr) {
        error!("UDP {} bind failed: {}", id, e);
        return -2;
    }

    sock.set_nonblocking(true).ok();

    // Convert to tokio UdpSocket
    let std_socket: std::net::UdpSocket = sock.into();
    let bind_result = runtime.block_on(async {
        match UdpSocket::from_std(std_socket) {
            Ok(socket) => {
                let arc_socket = Arc::new(socket);
                let socket_clone = arc_socket.clone();
                
                SOCKET_MAP.insert(id, SocketContext {
                    id,
                    shutdown_tx: Some(tx),
                    socket: arc_socket,
                    pending_sends: Arc::new(AtomicU32::new(0)),
                    pending_bytes: Arc::new(AtomicUsize::new(0)),
                });

                runtime.spawn(handle_udp_receive(id, socket_clone, rx));
                debug!("UDP {} bound to {} (SO_REUSEADDR=true)", id, addr);
                0
            }
            Err(e) => {
                error!("Failed to convert UDP socket {}: {}", id, e);
                -2
            }
        }
    });

    bind_result
}

#[no_mangle]
pub extern "C" fn udp_send(id: u32, data: *const u8, len: usize, host: *const c_char, port: c_int) -> i32 {
    if host.is_null() || data.is_null() { return -1; }
    
    let host_str = match unsafe { CStr::from_ptr(host).to_str() } {
        Ok(s) => s,
        Err(e) => {
            error!("UDP {} send: invalid UTF-8 in host: {}", id, e);
            return -1;
        }
    };
    
    let addr_str = format!("{}:{}", host_str, port);
    let payload = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        // Check resource limits
        if ctx.pending_sends.load(Ordering::SeqCst) >= MAX_PENDING_SENDS {
            error!("UDP {} send: queue full (max {} messages)", id, MAX_PENDING_SENDS);
            return -4; // Queue full
        }
        if ctx.pending_bytes.load(Ordering::SeqCst) + len > MAX_PENDING_BYTES {
            error!("UDP {} send: memory limit exceeded (max {} bytes)", id, MAX_PENDING_BYTES);
            return -5; // Memory limit
        }
        
        let socket = ctx.socket.clone();
        let pending_sends = ctx.pending_sends.clone();
        let pending_bytes = ctx.pending_bytes.clone();
        
        pending_sends.fetch_add(1, Ordering::SeqCst);
        pending_bytes.fetch_add(len, Ordering::SeqCst);
        
        get_runtime().spawn(async move {
            if let Err(e) = socket.send_to(&payload, &addr_str).await {
                error!("UDP send error: {}", e);
            }
            pending_sends.fetch_sub(1, Ordering::SeqCst);
            pending_bytes.fetch_sub(len, Ordering::SeqCst);
        });
        let ret = len as i32;
        debug!("UDP {} send: returning {} bytes", id, ret);
        ret
    } else {
        error!("UDP {} send: socket not found (-3)", id);
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_get_send_queue_count(id: u32) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        ctx.pending_sends.load(Ordering::SeqCst) as i32
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_get_send_queue_size(id: u32) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        ctx.pending_bytes.load(Ordering::SeqCst) as i32
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_send_multiple(
    id: u32,
    datas: *const *const u8,
    lens: *const usize,
    count: usize,
    host: *const c_char,
    port: c_int,
) -> i32 {
    if host.is_null() || datas.is_null() || lens.is_null() { return -1; }
    
    let host_str = match parse_cstr(host) { Some(s) => s, None => return -1 };
    let addr_str = format!("{}:{}", host_str, port);

    let mut total_len = 0;
    let lens_slice = unsafe { std::slice::from_raw_parts(lens, count) };
    for &l in lens_slice { total_len += l; }

    let mut payload = Vec::with_capacity(total_len);
    let datas_slice = unsafe { std::slice::from_raw_parts(datas, count) };
    for i in 0..count {
        let d = unsafe { std::slice::from_raw_parts(datas_slice[i], lens_slice[i]) };
        payload.extend_from_slice(d);
    }
    
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        // Check resource limits
        if ctx.pending_sends.load(Ordering::SeqCst) >= MAX_PENDING_SENDS {
            error!("UDP {} send_multiple: queue full (max {} messages)", id, MAX_PENDING_SENDS);
            return -4;
        }
        if ctx.pending_bytes.load(Ordering::SeqCst) + total_len > MAX_PENDING_BYTES {
            error!("UDP {} send_multiple: memory limit exceeded (max {} bytes)", id, MAX_PENDING_BYTES);
            return -5;
        }
        
        let socket = ctx.socket.clone();
        let pending_sends = ctx.pending_sends.clone();
        let pending_bytes = ctx.pending_bytes.clone();
        
        pending_sends.fetch_add(1, Ordering::SeqCst);
        pending_bytes.fetch_add(total_len, Ordering::SeqCst);
        
        get_runtime().spawn(async move {
            if let Err(e) = socket.send_to(&payload, &addr_str).await {
                error!("UDP send error: {}", e);
            }
            pending_sends.fetch_sub(1, Ordering::SeqCst);
            pending_bytes.fetch_sub(total_len, Ordering::SeqCst);
        });
        total_len as i32
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_connect(id: u32, host: *const c_char, port: c_int) -> i32 {
    let host_str = if host.is_null() {
        return -1;
    } else {
        match unsafe { CStr::from_ptr(host).to_str() } {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let addr_str = format!("{}:{}", host_str, port);
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let socket = ctx.socket.clone();
        get_runtime().block_on(async move {
            match socket.connect(&addr_str).await {
                Ok(_) => {
                    emit_event(id, EVENT_CONNECT, &[], host_str, port);
                    0
                }
                Err(e) => {
                    error!("UDP connect error: {}", e);
                    -2
                }
            }
        })
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_get_buffer_size(id: u32, is_send: bool) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let sock_ref = socket2::SockRef::from(&*ctx.socket);
        let result = if is_send {
            sock_ref.send_buffer_size()
        } else {
            sock_ref.recv_buffer_size()
        };
        match result {
            Ok(size) => size as i32,
            Err(e) => {
                error!("UDP {} get_buffer_size error: {}", id, e);
                -2
            }
        }
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_set_buffer_size(id: u32, is_send: bool, size: i32) -> i32 {
    if size <= 0 { return -1; }
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let sock_ref = socket2::SockRef::from(&*ctx.socket);
        let result = if is_send {
            sock_ref.set_send_buffer_size(size as usize)
        } else {
            sock_ref.set_recv_buffer_size(size as usize)
        };
        match result {
            Ok(_) => 0,
            Err(e) => {
                error!("UDP {} set_buffer_size error: {}", id, e);
                -2
            }
        }
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_disconnect(_id: u32) -> i32 {
    // UDP disconnect is not directly supported by tokio in a cross-platform way.
    // Return -5 (ENOTSUP) for now.
    -5
}

#[no_mangle]
pub extern "C" fn udp_close(id: u32) {
    debug!("UDP {} close requested", id);
    
    // Extract the shutdown sender while holding the lock briefly
    let shutdown_tx = {
        if let Some(mut ctx) = SOCKET_MAP.get_mut(&id) {
            ctx.shutdown_tx.take()
        } else {
            error!("UDP {} NOT FOUND in SOCKET_MAP!", id);
            return;
        }
    }; // Lock is released here
    
    // Send shutdown signal asynchronously
    if let Some(tx) = shutdown_tx {
        get_runtime().spawn(async move {
            if let Err(e) = tx.send(()).await {
                error!("UDP {} failed to send shutdown signal: {}", id, e);
            }
            debug!("UDP {} shutdown signal sent", id);
        });
    } else {
        error!("UDP {} already closed (shutdown_tx was None)", id);
    }
}

#[no_mangle]
pub extern "C" fn udp_set_broadcast(id: u32, flag: bool) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Err(e) = ctx.socket.set_broadcast(flag) {
            error!("UDP {} set_broadcast error: {}", id, e);
            return -2;
        }
        0
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_set_ttl(id: u32, ttl: u32) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Err(e) = ctx.socket.set_ttl(ttl) {
            error!("UDP {} set_ttl error: {}", id, e);
            return -2;
        }
        0
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_set_multicast_ttl_v4(id: u32, ttl: u32) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Err(e) = ctx.socket.set_multicast_ttl_v4(ttl) {
            error!("UDP {} set_multicast_ttl_v4 error: {}", id, e);
            return -2;
        }
        0
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_set_multicast_loop_v4(id: u32, flag: bool) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Err(e) = ctx.socket.set_multicast_loop_v4(flag) {
            error!("UDP {} set_multicast_loop_v4 error: {}", id, e);
            return -2;
        }
        0
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_get_local_addr(id: u32, buf: *mut c_char, max_len: usize) -> i32 {
    if buf.is_null() || max_len == 0 { return -1; }
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Ok(addr) = ctx.socket.local_addr() {
            let s = addr.ip().to_string();
            let c_s = CString::new(s).unwrap_or_default();
            let bytes = c_s.as_bytes_with_nul();
            if bytes.len() > max_len { return -2; }
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buf, bytes.len()); }
            0
        } else { -2 }
    } else { -3 }
}

#[no_mangle]
pub extern "C" fn udp_get_local_port(id: u32) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Ok(addr) = ctx.socket.local_addr() {
            addr.port() as i32
        } else { -2 }
    } else { -3 }
}

#[no_mangle]
pub extern "C" fn udp_get_peer_addr(id: u32, buf: *mut c_char, max_len: usize) -> i32 {
    if buf.is_null() || max_len == 0 { return -1; }
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Ok(addr) = ctx.socket.peer_addr() {
            let s = addr.ip().to_string();
            let c_s = CString::new(s).unwrap_or_default();
            let bytes = c_s.as_bytes_with_nul();
            if bytes.len() > max_len { return -2; }
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buf, bytes.len()); }
            0
        } else { -2 }
    } else { -3 }
}

#[no_mangle]
pub extern "C" fn udp_get_peer_port(id: u32) -> i32 {
    if let Some(ctx) = SOCKET_MAP.get(&id) {
        if let Ok(addr) = ctx.socket.peer_addr() {
            addr.port() as i32
        } else { -2 }
    } else { -3 }
}

#[no_mangle]
pub extern "C" fn udp_add_membership(
    id: u32,
    multicast_addr: *const c_char,
    interface_addr: *const c_char,
) -> i32 {
    let multicast = match parse_cstr(multicast_addr) { Some(s) => s, None => return -1 };
    let interface = match parse_cstr(interface_addr) {
        Some(s) => s.parse::<std::net::IpAddr>().ok(),
        None => None,
    };

    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let sock_ref = socket2::SockRef::from(&*ctx.socket);
        let m_addr = match multicast.parse::<std::net::IpAddr>() { Ok(a) => a, Err(_) => return -1 };

        let result = match m_addr {
            std::net::IpAddr::V4(m) => {
                let m_interface = match interface {
                    Some(std::net::IpAddr::V4(i)) => i,
                    _ => std::net::Ipv4Addr::new(0, 0, 0, 0),
                };
                sock_ref.join_multicast_v4(&m, &m_interface)
            }
            std::net::IpAddr::V6(m) => {
                let m_interface = match interface {
                    Some(std::net::IpAddr::V6(_)) => 0,
                    _ => 0,
                };
                sock_ref.join_multicast_v6(&m, m_interface)
            }
        };

        if let Err(e) = result {
            error!("UDP join_multicast error: {}", e);
            -2
        } else {
            0
        }
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_drop_membership(
    id: u32,
    multicast_addr: *const c_char,
    interface_addr: *const c_char,
) -> i32 {
    let multicast = match parse_cstr(multicast_addr) { Some(s) => s, None => return -1 };
    let interface = match parse_cstr(interface_addr) {
        Some(s) => s.parse::<std::net::IpAddr>().ok(),
        None => None,
    };

    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let sock_ref = socket2::SockRef::from(&*ctx.socket);
        let m_addr = match multicast.parse::<std::net::IpAddr>() { Ok(a) => a, Err(_) => return -1 };

        let result = match m_addr {
            std::net::IpAddr::V4(m) => {
                let m_interface = match interface {
                    Some(std::net::IpAddr::V4(i)) => i,
                    _ => std::net::Ipv4Addr::new(0, 0, 0, 0),
                };
                sock_ref.leave_multicast_v4(&m, &m_interface)
            }
            std::net::IpAddr::V6(m) => {
                let m_interface = match interface {
                    Some(std::net::IpAddr::V6(_)) => 0,
                    _ => 0,
                };
                sock_ref.leave_multicast_v6(&m, m_interface)
            }
        };

        if let Err(e) = result {
            error!("UDP leave_multicast error: {}", e);
            -2
        } else {
            0
        }
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_add_source_specific_membership(
    id: u32,
    source_addr: *const c_char,
    group_addr: *const c_char,
    interface_addr: *const c_char,
) -> i32 {
    let source = match parse_cstr(source_addr) { Some(s) => s, None => return -1 };
    let group = match parse_cstr(group_addr) { Some(s) => s, None => return -1 };
    let interface = match parse_cstr(interface_addr) {
        Some(s) => s.parse::<std::net::IpAddr>().ok(),
        None => None,
    };

    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let sock_ref = socket2::SockRef::from(&*ctx.socket);
        let s_addr = match source.parse::<std::net::IpAddr>() { Ok(a) => a, Err(_) => return -1 };
        let g_addr = match group.parse::<std::net::IpAddr>() { Ok(a) => a, Err(_) => return -1 };

        let result = match (s_addr, g_addr) {
            (std::net::IpAddr::V4(_s), std::net::IpAddr::V4(_g)) => {
                let m_interface = match interface {
                    Some(std::net::IpAddr::V4(i)) => i,
                    _ => std::net::Ipv4Addr::new(0, 0, 0, 0),
                };
                sock_ref.join_ssm_v4(&_g, &m_interface, &_s)
            }
            (std::net::IpAddr::V6(_s), std::net::IpAddr::V6(_g)) => {
                // IPv6 SSM uses scope ID usually. For now handle basic or return error if not fully supported.
                // socket2 join_source_group_v6 takes index.
                // For simplicity, return ENOTSUP for v6 SSM if interface index is not provided or complex.
                // In Node.js it's usually source, group, interface.
                return -5; // ENOTSUP for v6 SSM for now
            }
            _ => return -1,
        };

        if let Err(e) = result {
            error!("UDP join_source_group error: {}", e);
            -2
        } else {
            0
        }
    } else {
        -3
    }
}

#[no_mangle]
pub extern "C" fn udp_drop_source_specific_membership(
    id: u32,
    source_addr: *const c_char,
    group_addr: *const c_char,
    interface_addr: *const c_char,
) -> i32 {
    let source = match parse_cstr(source_addr) { Some(s) => s, None => return -1 };
    let group = match parse_cstr(group_addr) { Some(s) => s, None => return -1 };
    let interface = match parse_cstr(interface_addr) {
        Some(s) => s.parse::<std::net::IpAddr>().ok(),
        None => None,
    };

    if let Some(ctx) = SOCKET_MAP.get(&id) {
        let sock_ref = socket2::SockRef::from(&*ctx.socket);
        let s_addr = match source.parse::<std::net::IpAddr>() { Ok(a) => a, Err(_) => return -1 };
        let g_addr = match group.parse::<std::net::IpAddr>() { Ok(a) => a, Err(_) => return -1 };

        let result = match (s_addr, g_addr) {
            (std::net::IpAddr::V4(_s), std::net::IpAddr::V4(_g)) => {
                let m_interface = match interface {
                    Some(std::net::IpAddr::V4(i)) => i,
                    _ => std::net::Ipv4Addr::new(0, 0, 0, 0),
                };
                sock_ref.leave_ssm_v4(&_g, &m_interface, &_s)
            }
            (std::net::IpAddr::V6(_s), std::net::IpAddr::V6(_g)) => {
                return -5; 
            }
            _ => return -1,
        };

        if let Err(e) = result {
            error!("UDP leave_source_group error: {}", e);
            -2
        } else {
            0
        }
    } else {
        -3
    }
}

fn parse_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() { return None; }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

