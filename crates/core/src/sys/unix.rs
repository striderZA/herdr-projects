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
