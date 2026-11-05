//! The tests' stand-in for a Unix socket on Windows: a named pipe at the socket's path, which accepts clients one at a
//! time and gives each its stream. The World's socket is one of these, so the herdr socket "is there" as it is on Unix.
#![cfg(windows)]
use std::cell::Cell;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::io::FromRawHandle;

type Handle = *mut c_void;
const PIPE_ACCESS_DUPLEX: u32 = 0x3;
/// Byte-mode, blocking pipe: the default of PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT, all zero.
const PIPE_BYTE_WAIT: u32 = 0;
const PIPE_UNLIMITED_INSTANCES: u32 = 255;
const ERROR_PIPE_CONNECTED: i32 = 535;

unsafe extern "system" {
    fn CreateNamedPipeW(name: *const u16, open_mode: u32, pipe_mode: u32, max_instances: u32, out_buffer: u32, in_buffer: u32, default_timeout: u32, security: *const c_void) -> Handle;
    fn ConnectNamedPipe(pipe: Handle, overlapped: *mut c_void) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

/// A named pipe listening at `path`. `accept` waits for one client and returns its stream; the next instance is made
/// before it returns, so a client that comes meanwhile waits for it, as on a listening socket.
pub struct Listener { name: Vec<u16>, next: Cell<Handle> }

impl Listener {
    pub fn bind(path: &str) -> io::Result<Self> {
        let name: Vec<u16> = format!(r"\\.\pipe\{path}").encode_utf16().chain(Some(0)).collect();
        let next = Cell::new(instance(&name)?);
        Ok(Self { name, next })
    }
    pub fn accept(&self) -> io::Result<File> {
        let handle = self.next.get();
        if unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } == 0 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() != Some(ERROR_PIPE_CONNECTED) { return Err(e); }
        }
        self.next.set(instance(&self.name)?);
        Ok(unsafe { File::from_raw_handle(handle as _) })
    }
}

// A pipe handle may be used from any thread: a listener is moved to the thread that serves it, and used only there.
unsafe impl Send for Listener {}

impl Drop for Listener {
    fn drop(&mut self) { unsafe { CloseHandle(self.next.get()) }; }
}

fn instance(name: &[u16]) -> io::Result<Handle> {
    let h = unsafe { CreateNamedPipeW(name.as_ptr(), PIPE_ACCESS_DUPLEX, PIPE_BYTE_WAIT, PIPE_UNLIMITED_INSTANCES, 65536, 65536, 0, std::ptr::null()) };
    if h as isize == -1 { Err(io::Error::last_os_error()) } else { Ok(h) }
}
