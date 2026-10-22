//! herdr's endpoint: the default socket sits in herdr's config folder; a socket (Unix) or a regular file (Windows) is an
//! endpoint, and a missing path is not; a reply is read within a deadline, and a stalled one times out with the connection
//! still usable (a socket pair on Unix, a named pipe served by this test on Windows); the session name is found on both
//! separators.
use std::path::Path;
use std::time::Duration;
use sw::herdr::cli::session_name;
use sw::platform::endpoint;

#[test]
fn the_default_socket_is_herdr_sock_in_herdr_s_config_folder() {
    let socket = endpoint::default_socket();
    assert!(socket.ends_with(Path::new("herdr").join("herdr.sock")), "{}", socket.display());
    assert_eq!(socket.parent().unwrap().parent().unwrap(), endpoint::config_home());
}

#[test]
fn a_missing_path_is_no_endpoint() {
    let path = std::env::temp_dir().join(format!("sw-no-endpoint-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    assert!(!endpoint::exists(path.to_str().unwrap()));
}

#[cfg(unix)]
#[test]
fn a_socket_is_an_endpoint_on_unix() {
    use std::os::unix::net::UnixListener;
    let path = std::env::temp_dir().join(format!("sw-endpoint-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let _listener = UnixListener::bind(&path).unwrap();
    assert!(endpoint::exists(path.to_str().unwrap()));
    let _ = std::fs::remove_file(&path);
}

#[cfg(windows)]
#[test]
fn a_regular_file_is_an_endpoint_on_windows() {
    let path = std::env::temp_dir().join(format!("sw-endpoint-{}.sock", std::process::id()));
    std::fs::write(&path, "herdr's marker").unwrap();
    assert!(endpoint::exists(path.to_str().unwrap()));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_session_is_named_by_the_folder_its_socket_sits_in() {
    let named = Path::new("base").join("sessions").join("work").join("herdr.sock");
    assert_eq!(session_name(&named.display().to_string()), "work");
    let default = Path::new("base").join("herdr.sock");
    assert_eq!(session_name(&default.display().to_string()), "default");
}

#[cfg(unix)]
#[test]
fn a_reply_is_read_within_the_deadline_and_a_stalled_one_times_out() {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    let (mut server, mut client) = UnixStream::pair().unwrap();
    server.write_all(b"hello\n").unwrap();
    assert_eq!(endpoint::read_line_within(&mut client, Duration::from_secs(5)).unwrap(), "hello\n");
    assert!(endpoint::read_line_within(&mut client, Duration::from_millis(100)).is_err(), "nothing came: the deadline ends the read");
    server.write_all(b"again\n").unwrap();
    assert_eq!(endpoint::read_line_within(&mut client, Duration::from_secs(5)).unwrap(), "again\n", "the connection works after a timeout");
}

/// A named pipe served the way herdr serves its own: one client, sent a line at once and another when told.
#[cfg(windows)]
mod pipe {
    use std::ffi::c_void;
    use std::sync::mpsc::Receiver;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateNamedPipeW(name: *const u16, open_mode: u32, pipe_mode: u32, instances: u32, out_size: u32, in_size: u32, timeout: u32, security: *const c_void) -> *mut c_void;
        fn ConnectNamedPipe(pipe: *mut c_void, overlapped: *mut c_void) -> i32;
        fn WriteFile(file: *mut c_void, buf: *const u8, len: u32, written: *mut u32, overlapped: *mut c_void) -> i32;
        fn CloseHandle(h: *mut c_void) -> i32;
    }

    /// Serves `\\.\pipe\<name>`: sends `first` to the one client, then `second` once `go` is sent.
    pub fn serve(name: String, first: &'static str, second: &'static str, go: Receiver<()>) {
        const PIPE_ACCESS_DUPLEX: u32 = 3;
        let wide: Vec<u16> = format!(r"\\.\pipe\{name}").encode_utf16().chain(std::iter::once(0)).collect();
        let h = unsafe { CreateNamedPipeW(wide.as_ptr(), PIPE_ACCESS_DUPLEX, 0, 1, 4096, 4096, 0, std::ptr::null()) };
        assert!(h as isize != -1, "the pipe was made");
        unsafe { ConnectNamedPipe(h, std::ptr::null_mut()) };
        send(h, first);
        let _ = go.recv();
        send(h, second);
        unsafe { CloseHandle(h) };
    }

    fn send(h: *mut c_void, text: &str) {
        let mut written = 0u32;
        unsafe { WriteFile(h, text.as_ptr(), text.len() as u32, &mut written, std::ptr::null_mut()) };
    }
}

/// The pipe appears when the server thread gets to it, so the client tries until it is there.
#[cfg(windows)]
fn connect_when_served(name: &str) -> endpoint::Conn {
    let until = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match endpoint::connect(name) {
            Ok(c) => return c,
            Err(e) if std::time::Instant::now() < until => {
                let _ = e;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => panic!("the pipe was not served: {e}"),
        }
    }
}

#[cfg(windows)]
#[test]
fn a_reply_is_read_within_the_deadline_and_a_stalled_one_times_out() {
    let name = format!("sw-test-endpoint-{}", std::process::id());
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let server_name = name.clone();
    std::thread::spawn(move || pipe::serve(server_name, "hello\n", "again\n", go_rx));
    let mut c = connect_when_served(&name);
    assert_eq!(endpoint::read_line_within(&mut c, Duration::from_secs(5)).unwrap(), "hello\n");
    assert!(endpoint::read_line_within(&mut c, Duration::from_millis(300)).is_err(), "nothing came: the deadline ends the read");
    go_tx.send(()).unwrap();
    assert_eq!(endpoint::read_line_within(&mut c, Duration::from_secs(5)).unwrap(), "again\n", "the connection works after a timeout");
}
