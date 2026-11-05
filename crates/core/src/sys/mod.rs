//! The platform layer: the few things that differ between Unix and Windows. Each is a function here with a Unix body
//! (unix.rs, kept as it was in the code it replaces) and a Windows body (windows.rs). Callers use `sys::…` and never a
//! platform API directly.
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
/// The real path of a file or folder that exists, as `std::fs::canonicalize` gives it but without Windows' `\\?\` prefix
/// (which a path shows with and no program needs). On Unix this is `std::fs::canonicalize` itself.
pub fn canonicalize(p: impl AsRef<std::path::Path>) -> std::io::Result<std::path::PathBuf> { std::fs::canonicalize(p).map(plain_path) }
