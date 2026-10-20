//! What a file or folder may be: private to its user, a program, writable or not, and whether a folder can be written. Unix: the mode
//! bits, as the program sets them (0600 for a private file, 0755 for a program, the owner's write bit set or cleared), and
//! access(2). Windows: there are no mode bits, so a private file takes its folder's ACL like any other file; a program has
//! a program's extension (exe, cmd, bat, com); the read-only flag stands for the write bit; a folder is writable when a
//! file can be created in it.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// The options of a file only its user may read and write: a new file is created with mode 0600 on Unix.
#[cfg(unix)]
fn private_options() -> OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;
    let mut o = OpenOptions::new();
    o.mode(0o600);
    o
}

#[cfg(windows)]
fn private_options() -> OpenOptions { OpenOptions::new() }

/// Writes the text into a file that only its user can read and write. An existing file keeps its mode and is replaced
/// in full.
pub fn write_private(path: &Path, text: &str) -> io::Result<()> {
    let mut f = private_options().write(true).create(true).truncate(true).open(path)?;
    f.write_all(text.as_bytes())
}

/// Creates a new file that only its user can read and write. A path that exists already, a link included, is refused.
#[cfg(unix)]
pub fn create_new_private(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(path)
}

#[cfg(windows)]
pub fn create_new_private(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Makes a folder private to its user: mode 0700 on Unix. Windows: nothing is done, a new folder takes its profile's ACL.
#[cfg(unix)]
pub fn make_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(windows)]
pub fn make_private_dir(_path: &Path) -> io::Result<()> { Ok(()) }

/// Lets its owner change and remove what is in a folder, for a removal the board makes. Unix: a folder gets the owner's
/// rwx bits (a file keeps its mode). Windows: the read-only flag goes, for a file or a folder; a link is left alone.
#[cfg(unix)]
pub fn make_removable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() { return Ok(()); }
    fs::set_permissions(path, fs::Permissions::from_mode(m.permissions().mode() | 0o700))
}

#[cfg(windows)]
pub fn make_removable(path: &Path) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if crate::platform::meta::is_link(&m) || !m.permissions().readonly() { return Ok(()); }
    let mut p = m.permissions();
    p.set_readonly(false);
    fs::set_permissions(path, p)
}

/// Makes a file a program that can be run: mode 0755 on Unix. Windows runs a program by its extension, so nothing is done.
#[cfg(unix)]
pub fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(windows)]
pub fn make_executable(_path: &Path) -> io::Result<()> { Ok(()) }

/// Whether the file is a program that can be started: a regular file with an execute bit on Unix; on Windows a regular
/// file with a program's extension.
#[cfg(unix)]
pub fn is_program(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[cfg(windows)]
pub fn is_program(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    fs::metadata(path).map(|m| m.is_file()).unwrap_or(false) && matches!(ext.as_str(), "exe" | "cmd" | "bat" | "com")
}

/// Takes the owner's write bit away (`on` false) or gives it back (`on` true). Not for a link. Windows: the read-only flag.
#[cfg(unix)]
pub fn set_writable(path: &Path, on: bool) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::symlink_metadata(path)?.permissions().mode();
    fs::set_permissions(path, fs::Permissions::from_mode(if on { mode | 0o200 } else { mode & !0o222 }))
}

#[cfg(windows)]
pub fn set_writable(path: &Path, on: bool) -> io::Result<()> {
    let mut p = fs::metadata(path)?.permissions();
    p.set_readonly(!on);
    fs::set_permissions(path, p)
}

/// Whether this process can write into the folder: access(2) for write on Unix. On Windows a probe file is created in it
/// and removed again.
#[cfg(unix)]
pub fn can_write(dir: &Path) -> bool {
    let c = std::ffi::CString::new(dir.display().to_string()).unwrap_or_default();
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(windows)]
pub fn can_write(dir: &Path) -> bool {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    let probe = dir.join(format!(".write-probe-{}-{nanos}", std::process::id()));
    let ok = OpenOptions::new().write(true).create_new(true).open(&probe).is_ok();
    let _ = fs::remove_file(&probe);
    ok
}
