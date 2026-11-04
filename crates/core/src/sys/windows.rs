//! The Windows side of the platform layer: each function's Windows body, with the same names as unix.rs. Where Windows has
//! no such thing (a numeric user id, HOME, mode bits, an inode, an atomic swap of two paths), the body says what it does
//! instead.
use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, STILL_ACTIVE};
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetFileInformationByHandle, LockFileEx, BY_HANDLE_FILE_INFORMATION, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
};
use windows_sys::Win32::System::IO::OVERLAPPED;
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
/// The variable that names the user's home folder: Windows's profile folder.
pub const HOME_VAR: &str = "USERPROFILE";
/// Where a path starts when nothing else says where: the root of the system drive.
pub fn root_dir() -> PathBuf {
    PathBuf::from(format!("{}\\", std::env::var("SystemDrive").ok().filter(|d| !d.is_empty()).unwrap_or_else(|| "C:".into())))
}
/// Windows keeps no numeric user id the board uses: every user is 0.
pub fn uid() -> u32 { 0 }
/// The user's profile folder, from the environment (Windows has no list of users read here).
pub fn passwd_home() -> Option<String> { std::env::var("USERPROFILE").ok() }
/// The user's name, from the environment.
pub fn passwd_name() -> Option<String> { std::env::var("USERNAME").ok() }
/// A link to a folder or to a file: Windows needs a different link for each. A relative target is relative to the
/// link's own folder, as on Unix.
pub fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    let at = if target.is_absolute() { target.to_path_buf() } else { link.parent().unwrap_or(Path::new(".")).join(target) };
    if at.is_dir() { std::os::windows::fs::symlink_dir(target, link) } else { std::os::windows::fs::symlink_file(target, link) }
}
/// The (volume, file index) pair of a file. Windows keeps no inode in the metadata, so the creation time stands in:
/// enough for the cache keys that use it. same_open_file is the identity used for locks.
pub fn file_key(m: &Metadata) -> (u64, u64) { (0, m.creation_time()) }
/// Windows counts 100 ns ticks from 1601; this gives (seconds, nanoseconds) since 1970.
fn unix_time(ticks: u64) -> (i64, i64) {
    let t = ticks as i64 - 116_444_736_000_000_000;
    (t.div_euclid(10_000_000), t.rem_euclid(10_000_000) * 100)
}
/// The time of the last write, as (seconds, nanoseconds) since the epoch.
pub fn mtime(m: &Metadata) -> (i64, i64) { unix_time(m.last_write_time()) }
/// Windows has no inode change time: the last write stands in for it, and a write changes it.
pub fn ctime(m: &Metadata) -> (i64, i64) { mtime(m) }
/// Windows has no mode bits: a file gets the permissions of its folder.
pub fn private_mode(o: &mut OpenOptions) -> &mut OpenOptions { o }
/// Windows creates no file through a link here: the callers check for a link first.
pub fn no_follow(o: &mut OpenOptions) -> &mut OpenOptions { o }
/// Opens a lock file for `lock_file`: read and write, since LockFileEx refuses a handle with neither.
pub fn open_lock(path: &Path) -> io::Result<File> { OpenOptions::new().create(true).read(true).write(true).open(path) }
/// Takes an exclusive lock on an open file, waiting for it when `wait`. False when another process holds it.
pub fn lock_file(f: &File, wait: bool) -> bool {
    let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
    let flags = LOCKFILE_EXCLUSIVE_LOCK | if wait { 0 } else { LOCKFILE_FAIL_IMMEDIATELY };
    unsafe { LockFileEx(f.as_raw_handle() as _, flags, 0, u32::MAX, u32::MAX, &mut ov) != 0 }
}
/// Whether an open file and the file at this path are the same file: both are asked for their volume and file index.
pub fn same_open_file(f: &File, path: &Path) -> bool {
    match (file_id(f), OpenOptions::new().read(true).open(path).ok().and_then(|g| file_id(&g))) { (Some(a), Some(b)) => a == b, _ => false }
}
fn file_id(f: &File) -> Option<(u32, u32, u32)> {
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(f.as_raw_handle() as _, &mut info) } == 0 { return None; }
    Some((info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow))
}
/// Windows has no atomic swap of two paths: the caller takes the other way (false here).
pub fn swap_paths(_a: &Path, _b: &Path) -> bool { false }
/// A running program's file gets no " (deleted)" suffix on Windows.
pub fn without_deleted(p: PathBuf) -> PathBuf { p }
/// Strips the `\\?\` prefix a canonical path gets on Windows, which git and most tools refuse: `\\?\C:\x` is `C:\x`, and
/// `\\?\UNC\server\x` is `\\server\x`.
pub fn plain_path(p: PathBuf) -> PathBuf {
    let plain: Option<String> = match p.to_str() {
        Some(s) => match s.strip_prefix(r"\\?\UNC\") {
            Some(rest) => Some(format!(r"\\{rest}")),
            None => s.strip_prefix(r"\\?\").filter(|r| r.len() >= 2 && r.as_bytes()[1] == b':').map(str::to_string),
        },
        None => None,
    };
    plain.map(PathBuf::from).unwrap_or(p)
}
/// Whether a process with this id is running. One this user cannot open runs too (another user's).
pub fn pid_alive(pid: i64) -> bool {
    if pid <= 0 { return false; }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32);
        if h.is_null() { return io::Error::last_os_error().raw_os_error() == Some(5); } // access denied: it runs
        let mut code = 0u32;
        let known = GetExitCodeProcess(h, &mut code) != 0;
        CloseHandle(h);
        !known || code == STILL_ACTIVE as u32
    }
}
/// A process's start time: its creation time from the process handle. A pid reused later has another one.
pub fn proc_start(pid: i64) -> Option<String> {
    if pid <= 0 { return None; }
    let zero = || FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32);
        if h.is_null() { return None; }
        let (mut created, mut exited, mut kernel, mut user) = (zero(), zero(), zero(), zero());
        let ok = GetProcessTimes(h, &mut created, &mut exited, &mut kernel, &mut user) != 0;
        CloseHandle(h);
        ok.then(|| ((created.dwHighDateTime as u64) << 32 | created.dwLowDateTime as u64).to_string())
    }
}
/// The disk's free bytes available to this user and its whole size, for the disk that holds `p`.
pub fn space(p: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let (mut free, mut total) = (0u64, 0u64);
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) } != 0;
    ok.then_some((free, total))
}
/// Windows does not say how many bytes a file takes on disk: its size, rounded up to a 4 KB cluster.
pub fn allocated_bytes(m: &Metadata) -> u64 { const CLUSTER: u64 = 4096; m.file_size().div_ceil(CLUSTER) * CLUSTER }
/// A Windows folder is private to its user already (so is the temporary folder): nothing to set.
pub fn restrict_dir(_p: &Path) {}
/// This machine's own name, as Windows gives it.
pub fn hostname() -> String { std::env::var("COMPUTERNAME").unwrap_or_default() }
