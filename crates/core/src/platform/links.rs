//! Links between paths and places: a link from one path to another (a symbolic link on Unix; on Windows a junction for a
//! folder and a copy for a file, since a file link needs rights Windows does not give by default), two paths swapping places
//! in one step (renameat2 on Linux, renamex_np on macOS; not on Windows), and whether two paths name one place (canonical
//! paths, and on Windows blind to the \\?\ prefix that canonical paths carry).
use std::path::{Path, PathBuf};

/// Makes `at` a link to `target`. Unix: a symbolic link, exactly as before. Windows: a junction when the target is a folder,
/// a copy when it is a file. A relative target is read from the folder `at` is in, as a symbolic link's target is.
#[cfg(unix)]
pub fn make_link(target: &Path, at: &Path) -> std::io::Result<()> { std::os::unix::fs::symlink(target, at) }

#[cfg(windows)]
pub fn make_link(target: &Path, at: &Path) -> std::io::Result<()> {
    let full = if target.is_absolute() { target.to_path_buf() } else { at.parent().unwrap_or(Path::new(".")).join(target) };
    if full.is_dir() || !full.exists() { junction::create(&full, at) } else { std::fs::copy(&full, at).map(|_| ()) }
}

/// Reads a symbolic-link or junction target.
pub fn read_link(at: &Path) -> std::io::Result<PathBuf> { std::fs::read_link(at) }

/// Removes a directory link using the Windows directory operation; Unix symlinks use remove_file.
#[cfg(unix)]
pub fn remove_link(at: &Path) -> std::io::Result<()> { std::fs::remove_file(at) }

#[cfg(windows)]
pub fn remove_link(at: &Path) -> std::io::Result<()> {
    let m = std::fs::symlink_metadata(at)?;
    if m.is_dir() || crate::platform::meta::is_link(&m) { std::fs::remove_dir(at) } else { std::fs::remove_file(at) }
}

/// Two paths change places in one step, so neither is missing at any moment: renameat2 on Linux, renamex_np on macOS.
/// Elsewhere, and on Windows, false: the caller then does it in more steps.
#[cfg(unix)]
pub fn swap(a: &Path, b: &Path) -> bool {
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

#[cfg(windows)]
pub fn swap(_a: &Path, _b: &Path) -> bool { false }

/// Whether two paths name one place: their canonical forms are equal. A path that cannot be canonicalised is compared in
/// its absolute form, as the store always did.
pub fn same_place(a: &Path, b: &Path) -> bool { place(a) == place(b) }

/// The canonical form of a path, as a person reads it: no `\\?\` prefix on Windows; the absolute form when it cannot be
/// canonicalised.
pub fn place(p: &Path) -> PathBuf {
    let full = std::fs::canonicalize(p).unwrap_or_else(|_| {
        if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf()) }
    });
    strip_verbatim(full)
}

/// Windows: `\\?\C:\x` is `C:\x`, and `\\?\UNC\s\x` is `\\s\x`. Unix paths are left as they are.
#[cfg(unix)]
pub fn strip_verbatim(p: PathBuf) -> PathBuf { p }

#[cfg(windows)]
pub fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy().into_owned();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") { PathBuf::from(format!(r"\\{rest}")) }
    else if let Some(rest) = s.strip_prefix(r"\\?\") { PathBuf::from(rest) }
    else { p }
}
