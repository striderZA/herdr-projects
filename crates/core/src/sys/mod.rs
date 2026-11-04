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
