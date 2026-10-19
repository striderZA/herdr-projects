//! herdr's endpoint: where its socket is, whether a path is one, how to connect to it, and how to read a reply within a
//! deadline. Unix: a Unix socket, with a read timeout on the stream. Windows: herdr listens on a named pipe named by its
//! socket path (\\.\pipe\ + the path), so the endpoint is that pipe opened as a file, and a reply is read on a thread of its
//! own, which CancelSynchronousIo stops at the deadline (the per-request pattern that Spike S2 passed).
use std::io;
use std::path::PathBuf;
use std::time::Duration;

/// The connection to herdr: a Unix stream on Unix, the pipe opened as a file on Windows.
#[cfg(unix)]
pub type Conn = std::os::unix::net::UnixStream;

#[cfg(windows)]
pub type Conn = std::fs::File;

/// The folder herdr keeps its settings in; `herdr` sits inside it. Unix: XDG_CONFIG_HOME, else ~/.config. Windows: %APPDATA%.
#[cfg(unix)]
pub fn config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| crate::store::files::user_home().join(".config"))
}

#[cfg(windows)]
pub fn config_home() -> PathBuf {
    std::env::var("APPDATA").ok().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| crate::store::files::user_home().join("AppData").join("Roaming"))
}

/// herdr's socket for the default session: `herdr/herdr.sock` in the config folder. On Windows this is the path that names
/// herdr's pipe.
pub fn default_socket() -> PathBuf { config_home().join("herdr").join("herdr.sock") }

/// Whether herdr's endpoint is at this path. Unix: a socket. Windows: a regular file, which is herdr's marker.
#[cfg(unix)]
pub fn exists(path: &str) -> bool {
    use std::os::unix::fs::FileTypeExt;
    std::fs::metadata(path).map(|m| m.file_type().is_socket()).unwrap_or(false)
}

#[cfg(windows)]
pub fn exists(path: &str) -> bool { std::fs::metadata(path).map(|m| m.is_file()).unwrap_or(false) }

/// Connects to herdr at this path. Windows: the named pipe `\\.\pipe\` + the path, which is how herdr names it.
#[cfg(unix)]
pub fn connect(path: &str) -> io::Result<Conn> { std::os::unix::net::UnixStream::connect(path) }

#[cfg(windows)]
pub fn connect(path: &str) -> io::Result<Conn> { std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\{path}")) }

/// Reads one line from the connection, waiting at most `d`. Unix: a read timeout on the stream. Windows: the read runs on a
/// thread of its own, and at the deadline CancelSynchronousIo stops it; the connection stays usable for the next request.
#[cfg(unix)]
pub fn read_line_within(c: &mut Conn, d: Duration) -> io::Result<String> {
    use std::io::{BufRead, BufReader};
    c.set_read_timeout(Some(d))?;
    let mut line = String::new();
    BufReader::new(&mut *c).read_line(&mut line)?;
    Ok(line)
}

#[cfg(windows)]
pub fn read_line_within(c: &mut Conn, d: Duration) -> io::Result<String> {
    use std::io::{BufRead, BufReader};
    use std::sync::mpsc;
    use windows_sys::Win32::Foundation::{CloseHandle, DuplicateHandle, HANDLE};
    use windows_sys::Win32::System::IO::CancelSynchronousIo;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentThread};
    const DUPLICATE_SAME_ACCESS: u32 = 0x2;
    let reader_side = c.try_clone()?;
    let (line_tx, line_rx) = mpsc::channel::<io::Result<String>>();
    let (handle_tx, handle_rx) = mpsc::channel::<usize>();
    std::thread::spawn(move || {
        // A real handle to this thread, so that the thread waiting below can cancel the read.
        let mut this: HANDLE = std::ptr::null_mut();
        unsafe { DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &mut this, 0, 0, DUPLICATE_SAME_ACCESS) };
        let _ = handle_tx.send(this as usize);
        let mut line = String::new();
        let r = BufReader::new(reader_side).read_line(&mut line).map(|_| line);
        let _ = line_tx.send(r);
    });
    let this = handle_rx.recv().unwrap_or(0) as HANDLE;
    match line_rx.recv_timeout(d) {
        Ok(r) => {
            unsafe { CloseHandle(this) };
            r
        }
        Err(_) => {
            unsafe { CancelSynchronousIo(this) };
            let _ = line_rx.recv_timeout(Duration::from_secs(2));
            unsafe { CloseHandle(this) };
            Err(io::Error::new(io::ErrorKind::TimedOut, "no reply within the deadline"))
        }
    }
}
