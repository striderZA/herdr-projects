//! The Unix side of the platform layer: each function's Unix body, as it was in the code it replaces.
use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
/// The variable that names the user's home folder.
pub const HOME_VAR: &str = "HOME";
/// Where a path starts when nothing else says where: the root of the file system.
pub fn root_dir() -> PathBuf { PathBuf::from("/") }
/// The user's numeric id (libc's getuid).
pub fn uid() -> u32 { unsafe { libc::getuid() } }
/// The home folder the system's list of users gives this user, not what the environment says.
pub fn passwd_home() -> Option<String> {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_dir.is_null() { return None; }
        Some(std::ffi::CStr::from_ptr((*pw).pw_dir).to_string_lossy().into_owned())
    }
}
/// The user's name from the system's list of users (None when it has no entry).
pub fn passwd_name() -> Option<String> {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() { None } else { Some(std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned()) }
    }
}
/// A symbolic link to a file or a folder: the same kind of link for both.
pub fn symlink(target: &Path, link: &Path) -> io::Result<()> { std::os::unix::fs::symlink(target, link) }
/// The (device, inode) pair of a file: what tells one file from another.
pub fn file_key(m: &Metadata) -> (u64, u64) { use std::os::unix::fs::MetadataExt; (m.dev(), m.ino()) }
/// The time of the last write, as (seconds, nanoseconds) since the epoch.
pub fn mtime(m: &Metadata) -> (i64, i64) { use std::os::unix::fs::MetadataExt; (m.mtime(), m.mtime_nsec()) }
/// The time of the last change to the file's inode: every write makes one.
pub fn ctime(m: &Metadata) -> (i64, i64) { use std::os::unix::fs::MetadataExt; (m.ctime(), m.ctime_nsec()) }
/// Options for a file only its owner may read: created with mode 0600.
pub fn private_mode(o: &mut OpenOptions) -> &mut OpenOptions { use std::os::unix::fs::OpenOptionsExt; o.mode(0o600) }
/// Options that refuse a link at the path: the open fails rather than writing through the link.
pub fn no_follow(o: &mut OpenOptions) -> &mut OpenOptions { use std::os::unix::fs::OpenOptionsExt; o.custom_flags(libc::O_NOFOLLOW) }
/// Opens a lock file for `lock_file`: appending, as the lock files always were opened.
pub fn open_lock(path: &Path) -> io::Result<File> { OpenOptions::new().create(true).append(true).open(path) }
/// Takes an exclusive lock on an open file, waiting for it when `wait`. False when another process holds it.
pub fn lock_file(f: &File, wait: bool) -> bool { unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | if wait { 0 } else { libc::LOCK_NB }) == 0 } }
/// Whether an open file and the file at this path are the same file: the same device and inode.
pub fn same_open_file(f: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (f.metadata(), std::fs::metadata(path)) { (Ok(a), Ok(b)) => a.ino() == b.ino() && a.dev() == b.dev(), _ => false }
}
/// Two paths change places in one step: there is no moment at which either is missing. False: not possible here.
pub fn swap_paths(a: &Path, b: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let (Ok(ca), Ok(cb)) = (CString::new(a.as_os_str().as_bytes()), CString::new(b.as_os_str().as_bytes())) else { return false };
    #[cfg(target_os = "linux")]
    unsafe { libc::syscall(libc::SYS_renameat2, libc::AT_FDCWD, ca.as_ptr(), libc::AT_FDCWD, cb.as_ptr(), 2u32) == 0 }
    #[cfg(target_os = "macos")]
    unsafe { libc::renamex_np(ca.as_ptr(), cb.as_ptr(), libc::RENAME_SWAP) == 0 }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    { let _ = (ca, cb); false }
}
/// This program's path without the " (deleted)" a running program's file gets once an update put a new file in its place.
pub fn without_deleted(p: PathBuf) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    match p.as_os_str().as_bytes().strip_suffix(b" (deleted)") { Some(b) => PathBuf::from(std::ffi::OsStr::from_bytes(b)), None => p }
}
/// Whether a process with this id runs (or runs as another user): signal 0 sends nothing.
pub fn pid_alive(pid: i64) -> bool {
    if pid <= 0 { return false; }
    let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
/// A process's start time, from the stat file of Linux: the field after the command name. A pid reused after a reboot
/// has another one.
pub fn proc_start(pid: i64) -> Option<String> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = &s[s.rfind(')')? + 2..];
    rest.split(' ').nth(19).map(|x| x.to_string())
}
/// A canonical path has no prefix to strip on Unix.
pub fn plain_path(p: PathBuf) -> PathBuf { p }
/// A disk's free and whole size in bytes, as the disk itself says it.
pub fn space(p: &Path) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(p.as_os_str().as_encoded_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some((s.f_bavail as u64 * s.f_frsize as u64, s.f_blocks as u64 * s.f_frsize as u64))
}
/// The bytes a file takes on disk: its 512-byte blocks.
pub fn allocated_bytes(m: &Metadata) -> u64 { use std::os::unix::fs::MetadataExt; m.blocks() * 512 }
/// Makes a folder readable and writable by its owner only (the temporary folder Claude Code wants is one).
pub fn restrict_dir(p: &Path) { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)); }
/// This machine's own name, as the system gives it.
pub fn hostname() -> String {
    let mut b = [0u8; 256];
    unsafe { libc::gethostname(b.as_mut_ptr() as *mut libc::c_char, b.len()); }
    String::from_utf8_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(0)]).into_owned()
}
/// A connection to a herdr socket: a Unix domain socket here.
pub type LocalStream = std::os::unix::net::UnixStream;
/// Connects to a herdr socket by its path.
pub fn connect_local(path: &str) -> io::Result<LocalStream> { LocalStream::connect(path) }
/// Whether a herdr socket is there: a socket file at this path.
pub fn socket_exists(path: &str) -> bool { use std::os::unix::fs::FileTypeExt; std::fs::metadata(path).map(|m| m.file_type().is_socket()).unwrap_or(false) }
/// Whether a path is a program that runs: a file with an execute bit set.
pub fn is_program(p: &Path) -> bool { use std::os::unix::fs::PermissionsExt; std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false) }
/// A file's mode bits, as the research check records them.
pub fn mode_bits(m: &Metadata) -> u32 { use std::os::unix::fs::MetadataExt; m.mode() }
/// The signals that ask a process to end (soft), and that end it at once (hard).
pub const TERM_SIGNAL: i32 = libc::SIGTERM;
pub const KILL_SIGNAL: i32 = libc::SIGKILL;
/// Sends `sig` to a process: false where it cannot be (gone, another user's).
pub fn send_signal(pid: i64, sig: i32) -> bool { pid > 1 && unsafe { libc::kill(pid as i32, sig) } == 0 }
/// Whether a process is a zombie: it has ended and waits for its parent to read it.
pub fn zombie(pid: i64) -> bool { std::fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rfind(')').map(|i| s[i + 2..].starts_with('Z'))).unwrap_or(false) }
/// A process's command line, where the system tells (`ps`).
pub fn command_line(pid: i64) -> Option<String> {
    let o = std::process::Command::new("ps").args(["-o", "args=", "-p", &pid.to_string()]).output().ok()?;
    Some(String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty())
}
/// The processes started by one process (`pgrep -P`): Err when the system cannot say.
pub fn children(pid: i64) -> Result<Vec<i64>, ()> {
    let o = std::process::Command::new("pgrep").args(["-P", &pid.to_string()]).output().map_err(|_| ())?;
    if o.status.code().unwrap_or(2) > 1 { return Err(()); }
    Ok(String::from_utf8_lossy(&o.stdout).lines().filter_map(|l| l.trim().parse().ok()).collect())
}
/// The owner's numeric id of a file.
pub fn owner_uid(m: &Metadata) -> u32 { use std::os::unix::fs::MetadataExt; m.uid() }
/// Sets a file's mode bits, as chmod does.
pub fn set_mode(p: &Path, mode: u32) -> io::Result<()> { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)) }
/// Options for a program the owner runs and anyone may read: created with mode 0755.
pub fn executable_mode(o: &mut OpenOptions) -> &mut OpenOptions { use std::os::unix::fs::OpenOptionsExt; o.mode(0o755) }
/// Whether the owner cannot write to this path (access(2) says so): a plan's locked worktree.
pub fn not_writable(p: &Path) -> bool { let c = std::ffi::CString::new(p.display().to_string()).unwrap_or_default(); unsafe { libc::access(c.as_ptr(), libc::W_OK) != 0 } }
/// A process of its own session, detached from this one: it outlives whoever started it.
pub fn detach(c: &mut std::process::Command) { use std::os::unix::process::CommandExt; unsafe { c.pre_exec(|| { libc::setsid(); Ok(()) }); } }
/// The stop signals that interrupt (SIGINT) and hang up (SIGHUP).
pub const INT_SIGNAL: i32 = libc::SIGINT;
pub const HUP_SIGNAL: i32 = libc::SIGHUP;
/// Sends `sig` to a process group (`pid` 0: this process's own group). True when it was sent.
pub fn kill_group(pid: i32, sig: i32) -> bool { unsafe { libc::kill(-pid, sig) == 0 } }
/// Calls `f` when one of `sigs` arrives. A signal handler does only what is safe in one.
pub fn on_stop(f: extern "C" fn(i32), sigs: &[i32]) { for s in sigs { unsafe { libc::signal(*s, f as *const () as usize); } } }
/// Ends this process at once, skipping what exit would run (safe in a signal handler).
pub fn exit_now(code: i32) -> ! { unsafe { libc::_exit(code) } }
/// Reads standard input for up to `ms` milliseconds: what came in (at most `buf` long); 0 when nothing did, or it ended.
pub fn read_input_within(buf: &mut [u8], ms: i32) -> usize {
    use std::io::Read;
    let mut p = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
    if unsafe { libc::poll(&mut p, 1, ms) } <= 0 { return 0; }
    std::io::stdin().read(buf).unwrap_or(0)
}
/// Runs a command in place of this program (same process, terminal and arguments). It returns only when that fails.
pub fn exec_replace(c: &mut std::process::Command) -> std::io::Error { use std::os::unix::process::CommandExt; c.exec() }
