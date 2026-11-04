//! The Windows side of the platform layer. Windows has no numeric user id the board keeps, and no HOME variable.
use std::path::PathBuf;
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
