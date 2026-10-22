//! The system's random bytes: /dev/urandom on Unix, the system's own generator (BCryptGenRandom) on Windows.

/// Fills `buf` with random bytes from the system. `false` when the system could not give them.
#[cfg(unix)]
pub fn fill(buf: &mut [u8]) -> bool {
    use std::io::Read;
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(buf)).is_ok()
}

/// Fills `buf` with random bytes from the system's own generator, which needs no seed from the program.
#[cfg(windows)]
pub fn fill(buf: &mut [u8]) -> bool {
    use windows_sys::Win32::Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG};
    let Ok(len) = u32::try_from(buf.len()) else { return false };
    // SAFETY: `buf` is writable for `len` bytes; a null algorithm handle selects the system's preferred generator.
    unsafe { BCryptGenRandom(std::ptr::null_mut(), buf.as_mut_ptr(), len, BCRYPT_USE_SYSTEM_PREFERRED_RNG) == 0 }
}
