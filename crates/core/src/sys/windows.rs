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
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};
mod local_stream;
pub use local_stream::LocalStream;
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
/// The pipe a herdr socket path names: the same path after `\\.\pipe\`, which is how herdr names its pipe.
fn pipe_name(path: &str) -> String { format!(r"\\.\pipe\{path}") }
/// Connects to a herdr socket by its path: its named pipe, read and written as one stream.
pub fn connect_local(path: &str) -> io::Result<LocalStream> {
    let file = OpenOptions::new().read(true).write(true).open(pipe_name(path))?;
    LocalStream::connect(file)
}
/// Whether a herdr socket is there: its pipe has an instance waiting, or every instance is taken (WaitNamedPipe times
/// out with ERROR_SEM_TIMEOUT, 121). No such pipe: ERROR_FILE_NOT_FOUND, so not there.
pub fn socket_exists(path: &str) -> bool {
    let name: Vec<u16> = pipe_name(path).encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { WaitNamedPipeW(name.as_ptr(), 0) != 0 || io::Error::last_os_error().raw_os_error() == Some(121) }
}
/// Whether a path is a program that runs: a file of a program's kind (Windows has no execute bit).
pub fn is_program(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file() && p.extension().and_then(|e| e.to_str()).map(|e| matches!(e.to_ascii_lowercase().as_str(), "exe" | "cmd" | "bat" | "com")).unwrap_or(false)).unwrap_or(false)
}
/// Windows has no mode bits: a read-only file says 0444, any other 0644.
pub fn mode_bits(m: &Metadata) -> u32 { if m.permissions().readonly() { 0o444 } else { 0o644 } }
/// Windows has no signals: each of these ends the process, as a stop does here.
pub const TERM_SIGNAL: i32 = 15;
pub const KILL_SIGNAL: i32 = 9;
/// Ends a process (TerminateProcess). Signal 0 asks instead whether it runs. False where it cannot be.
pub fn send_signal(pid: i64, sig: i32) -> bool {
    if pid <= 1 { return false; }
    if sig == 0 { return pid_alive(pid); }
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
        if h.is_null() { return false; }
        let ok = TerminateProcess(h, 1) != 0;
        CloseHandle(h);
        ok
    }
}
/// Windows has no zombie processes: one that ended is gone from the process table.
pub fn zombie(_pid: i64) -> bool { false }
/// The process table, read for one question (Windows has no `ps`; sysinfo reads the table). None when sysinfo cannot read it.
fn table() -> Option<sysinfo::System> {
    let mut s = sysinfo::System::new().ok()?;
    s.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    Some(s)
}
/// A process's command line: its arguments joined by spaces.
pub fn command_line(pid: i64) -> Option<String> {
    let s = table()?;
    let p = s.process(sysinfo::Pid::from_u32(pid as u32))?;
    let line = p.cmd().iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ");
    Some(line).filter(|l| !l.is_empty())
}
/// The processes whose parent is this one.
pub fn children(pid: i64) -> Result<Vec<i64>, ()> {
    let s = table().ok_or(())?;
    let parent = sysinfo::Pid::from_u32(pid as u32);
    Ok(s.processes().values().filter(|p| p.parent() == Some(parent)).map(|p| p.pid().as_u32() as i64).collect())
}
/// Windows has no numeric user id: every file's owner is 0 here.
pub fn owner_uid(_m: &Metadata) -> u32 { 0 }
/// Sets a file's mode: Windows keeps only whether a file is read-only (no write bit in the mode means read-only).
pub fn set_mode(p: &Path, mode: u32) -> io::Result<()> {
    let mut perm = std::fs::metadata(p)?.permissions();
    perm.set_readonly(mode & 0o222 == 0);
    std::fs::set_permissions(p, perm)
}
/// Windows has no mode to set on a new file: a program runs by its extension.
pub fn executable_mode(o: &mut OpenOptions) -> &mut OpenOptions { o }
/// Whether this path is read-only (a locked worktree). A folder's read-only flag does not stop writes in it here.
pub fn not_writable(p: &Path) -> bool { std::fs::metadata(p).map(|m| m.permissions().readonly()).unwrap_or(false) }
/// A process in a group of its own (CREATE_NEW_PROCESS_GROUP), so Ctrl+C in its starter's console does not reach it.
pub fn detach(c: &mut std::process::Command) { use std::os::windows::process::CommandExt; c.creation_flags(0x0000_0200); }
/// Windows has no signal numbers for these: each is a console event, and one handler takes them all.
pub const INT_SIGNAL: i32 = 2;
pub const HUP_SIGNAL: i32 = 1;
/// Ends a process and the processes it started, children first (`pid` 0: this process). True when any was ended.
pub fn kill_group(pid: i32, sig: i32) -> bool {
    let root = if pid == 0 { std::process::id() as i64 } else { pid as i64 };
    let mut todo = vec![root];
    let mut all = vec![];
    while let Some(p) = todo.pop() {
        all.push(p);
        if let Ok(kids) = children(p) { todo.extend(kids); }
    }
    all.into_iter().fold(false, |hit, p| send_signal(p, sig) || hit)
}
static STOP: std::sync::OnceLock<extern "C" fn(i32)> = std::sync::OnceLock::new();
/// A console event (Ctrl+C, Ctrl+Break, the console closed): the stop callback runs, and the event is handled.
unsafe extern "system" fn console_event(_kind: u32) -> i32 {
    if let Some(f) = STOP.get() { f(0); }
    1
}
/// Calls `f` on a stop, which here is a console event. The first callback registered is the one kept.
pub fn on_stop(f: extern "C" fn(i32), _sigs: &[i32]) {
    if STOP.set(f).is_ok() { unsafe { SetConsoleCtrlHandler(Some(console_event), 1); } }
}
/// Ends this process (Windows has no exit path that skips what exit runs, so this is the ordinary exit).
pub fn exit_now(code: i32) -> ! { std::process::exit(code) }
/// Reads standard input for up to `ms` milliseconds: what came in (at most `buf` long); 0 when nothing did, or it ended.
/// It waits on the console input first, so a read starts only when a record is there: a read started with no input would
/// take the next key press, later, when nothing is left to answer it.
pub fn read_input_within(buf: &mut [u8], ms: i32) -> usize {
    use std::io::Read;
    let h = unsafe { windows_sys::Win32::System::Console::GetStdHandle(windows_sys::Win32::System::Console::STD_INPUT_HANDLE) };
    if unsafe { windows_sys::Win32::System::Threading::WaitForSingleObject(h, ms.max(0) as u32) } != windows_sys::Win32::Foundation::WAIT_OBJECT_0 { return 0; }
    io::stdin().read(buf).unwrap_or(0)
}
/// Windows has no `ps`: no process table, so the board finds no views that hold a terminal (`held::holders` gives none).
pub fn process_table() -> String { String::new() }
/// Windows has no in-place exec: the command runs to its end with this program's terminal, and this program ends with its
/// exit code. It returns only when the command cannot be started.
pub fn exec_replace(c: &mut std::process::Command) -> std::io::Error {
    match c.status() { Ok(s) => std::process::exit(s.code().unwrap_or(1)), Err(e) => e }
}
