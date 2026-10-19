//! Exclusive file locks, taken without waiting or with it, and let go when the file's handle is dropped. Unix: flock(2),
//! as the program has always taken them. Windows: LockFileEx over the whole file, with LOCKFILE_FAIL_IMMEDIATELY when it
//! must not wait.
use std::fs::File;

/// Takes the exclusive lock of an open file without waiting. False when another handle holds it.
pub fn try_lock(f: &File) -> bool { take(f, false) }

/// Takes the exclusive lock of an open file, waiting until it is free. False when it cannot be taken at all.
pub fn lock(f: &File) -> bool { take(f, true) }

#[cfg(unix)]
fn take(f: &File, wait: bool) -> bool {
    use std::os::fd::AsRawFd;
    unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | if wait { 0 } else { libc::LOCK_NB }) == 0 }
}

#[cfg(windows)]
fn take(f: &File, wait: bool) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY};
    use windows_sys::Win32::System::IO::OVERLAPPED;
    let flags = LOCKFILE_EXCLUSIVE_LOCK | if wait { 0 } else { LOCKFILE_FAIL_IMMEDIATELY };
    let mut at: OVERLAPPED = unsafe { std::mem::zeroed() };
    // The whole file: every byte from offset 0, the low and high halves of the length all ones.
    unsafe { LockFileEx(f.as_raw_handle(), flags, 0, u32::MAX, u32::MAX, &mut at) != 0 }
}
