//! A file's facts as numbers: its identity (for "the same file" and "the same disk"), its modification and change times,
//! its mode, and whether it is a socket or a link. Unix: the stat fields, read as the program always read them. Windows:
//! the file index and volume serial from the handle; the change time is the last write, since Metadata does not carry
//! NTFS's change time; a link is a symlink or a reparse point (a junction); the mode stands for the read-only flag.
use std::fs::{File, Metadata};
use std::path::Path;

/// The file's identity at a path: (device, inode) on Unix, (volume serial, file index) on Windows. Follows links.
#[cfg(unix)]
pub fn file_id(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| (m.dev(), m.ino()))
}

#[cfg(windows)]
pub fn file_id(path: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let h = unsafe { CreateFileW(wide.as_ptr(), FILE_READ_ATTRIBUTES, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, std::ptr::null(), OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, std::ptr::null_mut()) };
    if h == INVALID_HANDLE_VALUE { return None; }
    let id = handle_id(h);
    unsafe { CloseHandle(h) };
    id
}

/// The identity of an open file.
#[cfg(unix)]
pub fn file_id_of(f: &File) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    f.metadata().ok().map(|m| (m.dev(), m.ino()))
}

#[cfg(windows)]
pub fn file_id_of(f: &File) -> Option<(u64, u64)> {
    use std::os::windows::io::AsRawHandle;
    handle_id(f.as_raw_handle())
}

/// The volume serial and file index of an open handle.
#[cfg(windows)]
fn handle_id(h: windows_sys::Win32::Foundation::HANDLE) -> Option<(u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(h, &mut info) } == 0 { return None; }
    Some((info.dwVolumeSerialNumber as u64, ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64))
}

/// Whether two paths are on one disk: the same device on Unix, the same volume on Windows.
pub fn same_disk(a: &Path, b: &Path) -> bool {
    matches!((file_id(a), file_id(b)), (Some(x), Some(y)) if x.0 == y.0)
}

/// The modification time as (seconds, nanoseconds) since 1970.
#[cfg(unix)]
pub fn mtime_parts(m: &Metadata) -> (i64, i64) {
    use std::os::unix::fs::MetadataExt;
    (m.mtime(), m.mtime_nsec())
}

#[cfg(windows)]
pub fn mtime_parts(m: &Metadata) -> (i64, i64) { since_epoch(m.modified()) }

/// The change time as (seconds, nanoseconds) since 1970. Windows: the last write (see the top of this file).
#[cfg(unix)]
pub fn ctime_parts(m: &Metadata) -> (i64, i64) {
    use std::os::unix::fs::MetadataExt;
    (m.ctime(), m.ctime_nsec())
}

#[cfg(windows)]
pub fn ctime_parts(m: &Metadata) -> (i64, i64) { since_epoch(m.modified()) }

/// A time before 1970 reads as zero on Windows: no file here is that old.
#[cfg(windows)]
fn since_epoch(t: std::io::Result<std::time::SystemTime>) -> (i64, i64) {
    match t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
        Some(d) => (d.as_secs() as i64, d.subsec_nanos() as i64),
        None => (0, 0),
    }
}

/// The modification time in milliseconds since 1970, with the fraction of a millisecond.
pub fn mtime_ms(m: &Metadata) -> f64 {
    let (s, n) = mtime_parts(m);
    s as f64 * 1000.0 + n as f64 / 1e6
}

/// The mode, as Unix's st_mode: file type and permission bits. Windows: the read-only flag stands for the write bits.
#[cfg(unix)]
pub fn mode(m: &Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    m.mode()
}

#[cfg(windows)]
pub fn mode(m: &Metadata) -> u32 {
    let kind = if m.file_type().is_symlink() { 0o120000 } else if m.is_dir() { 0o040000 } else { 0o100000 };
    let bits = if m.is_dir() { 0o755 } else if m.permissions().readonly() { 0o444 } else { 0o644 };
    kind | bits
}

/// A Unix socket. Windows has none of these here: the herdr endpoint is a named pipe.
#[cfg(unix)]
pub fn is_socket(m: &Metadata) -> bool {
    use std::os::unix::fs::FileTypeExt;
    m.file_type().is_socket()
}

#[cfg(windows)]
pub fn is_socket(_m: &Metadata) -> bool { false }

/// A symbolic link, or on Windows also a reparse point such as a junction.
#[cfg(unix)]
pub fn is_link(m: &Metadata) -> bool { m.file_type().is_symlink() }

#[cfg(windows)]
pub fn is_link(m: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    m.file_type().is_symlink() || (m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

/// The bytes a file takes on disk. Unix: the blocks allocated to it. Windows: its length (the cluster allocation is not read).
#[cfg(unix)]
pub fn bytes_on_disk(m: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    m.blocks() * 512
}

#[cfg(windows)]
pub fn bytes_on_disk(m: &Metadata) -> u64 { m.len() }

/// The owner of a file as the board compares it: the Unix uid, as text. Windows: empty, since no owner is read there, so
/// nothing is attributed to a user, and a file that cannot be attributed is never acted on.
#[cfg(unix)]
pub fn owner(m: &Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    m.uid().to_string()
}

#[cfg(windows)]
pub fn owner(_m: &Metadata) -> String { String::new() }
