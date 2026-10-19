//! What is on a disk: the space free to this user and the size of the disk a path is on, and which volume a path is on. Unix:
//! statvfs, and the device number. Windows: GetDiskFreeSpaceExW, and the volume serial number.
use std::path::Path;

/// (available, total) bytes of the disk a path is on: the space this user may use, and the disk's whole size.
#[cfg(unix)]
pub fn space(path: &Path) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some((s.f_bavail as u64 * s.f_frsize as u64, s.f_blocks as u64 * s.f_frsize as u64))
}

#[cfg(windows)]
pub fn space(path: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let (mut available, mut total, mut all_free) = (0u64, 0u64, 0u64);
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut all_free) } != 0;
    ok.then_some((available, total))
}

/// The volume a path is on: its device number on Unix, its volume serial number on Windows.
pub fn volume(path: &Path) -> Option<u64> { crate::platform::meta::file_id(path).map(|(v, _)| v) }
