//! The Unix side of the platform layer: each function's Unix body, as it was in the code it replaces.
use std::path::PathBuf;
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
