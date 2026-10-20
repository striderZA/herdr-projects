//! The fake herdr's listener on each platform: a Unix socket on Unix, and on Windows the named pipe herdr itself serves
//! (`\\.\pipe\` and the socket path). Both `bind`, `accept` as `(stream, _)` and `set_nonblocking`, and a stream reads and
//! writes as a file, so a test does not know which it has.
#[cfg(unix)]
pub use std::os::unix::net::UnixListener as Listener;

#[cfg(windows)]
pub use windows::Listener;

#[cfg(windows)]
mod windows {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::fs::File;
    use std::io;
    use std::os::windows::io::FromRawHandle;
    use std::path::Path;

    type Handle = *mut c_void;
    const INVALID: isize = -1;
    const PIPE_ACCESS_DUPLEX: u32 = 0x3;
    const PIPE_TYPE_BYTE: u32 = 0x0;
    const PIPE_READMODE_BYTE: u32 = 0x0;
    const PIPE_WAIT: u32 = 0x0;
    const PIPE_NOWAIT: u32 = 0x1;
    const PIPE_UNLIMITED_INSTANCES: u32 = 255;
    const ERROR_PIPE_CONNECTED: u32 = 535;
    const ERROR_PIPE_LISTENING: u32 = 536;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateNamedPipeW(name: *const u16, open_mode: u32, pipe_mode: u32, max_instances: u32, out_size: u32, in_size: u32, timeout: u32, security: *const c_void) -> Handle;
        fn ConnectNamedPipe(pipe: Handle, overlapped: *mut c_void) -> i32;
        fn SetNamedPipeHandleState(pipe: Handle, mode: *const u32, max_collection: *const u32, collection_timeout: *const u32) -> i32;
        fn GetLastError() -> u32;
        fn CloseHandle(h: Handle) -> i32;
    }

    /// The name a program's endpoint connects to: `\\.\pipe\` and then the socket path.
    fn name(path: &Path) -> Vec<u16> {
        format!(r"\\.\pipe\{}", path.display()).encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn set_mode(h: isize, mode: u32) -> io::Result<()> {
        if unsafe { SetNamedPipeHandleState(h as Handle, &mode, std::ptr::null(), std::ptr::null()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Serves one client at a time. `bind` makes the name at once, so a client can connect when it returns. `accept`
    /// takes the instance a client connects to and makes the next one; in nonblocking mode it returns `WouldBlock`
    /// when no client is there, as a Unix listener does.
    pub struct Listener { name: Vec<u16>, next: Cell<isize>, nowait: Cell<bool> }

    impl Listener {
        pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<Listener> {
            let l = Listener { name: name(path.as_ref()), next: Cell::new(INVALID), nowait: Cell::new(false) };
            l.next.set(l.instance()?);
            Ok(l)
        }

        fn instance(&self) -> io::Result<isize> {
            let mode = if self.nowait.get() { PIPE_NOWAIT } else { PIPE_WAIT };
            let h = unsafe { CreateNamedPipeW(self.name.as_ptr(), PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | mode, PIPE_UNLIMITED_INSTANCES, 65536, 65536, 0, std::ptr::null()) };
            if h as isize == INVALID {
                return Err(io::Error::last_os_error());
            }
            Ok(h as isize)
        }

        pub fn set_nonblocking(&self, on: bool) -> io::Result<()> {
            self.nowait.set(on);
            let next = self.next.get();
            if next != INVALID {
                set_mode(next, if on { PIPE_NOWAIT } else { PIPE_WAIT })?;
            }
            Ok(())
        }

        /// The next client as a stream, with `()` where a Unix listener gives the peer's address.
        pub fn accept(&self) -> io::Result<(File, ())> {
            let h = self.next.get();
            if unsafe { ConnectNamedPipe(h as Handle, std::ptr::null_mut()) } == 0 {
                let e = unsafe { GetLastError() };
                if e == ERROR_PIPE_LISTENING {
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                if e != ERROR_PIPE_CONNECTED {
                    return Err(io::Error::from_raw_os_error(e as i32));
                }
            }
            // The stream blocks, as a connected socket does; the next client gets an instance of its own.
            set_mode(h, PIPE_WAIT)?;
            self.next.set(self.instance()?);
            Ok((unsafe { File::from_raw_handle(h as Handle) }, ()))
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let h = self.next.get();
            if h != INVALID {
                unsafe { CloseHandle(h as Handle) };
            }
        }
    }
}
