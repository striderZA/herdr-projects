//! Whether macOS says this process is in a sandbox, by its own check (sandbox_check in libsystem_sandbox, which every Mac
//! process has). Unix: the function is looked up when it is asked for, not linked, so the program builds everywhere, and
//! where there is no such function there is no sandbox it could see. Windows: no such check, so false.
#[cfg(unix)]
pub fn os_says() -> bool {
    type Check = unsafe extern "C" fn(libc::pid_t, *const libc::c_char, libc::c_int, ...) -> libc::c_int;
    let f = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"sandbox_check".as_ptr()) };
    if f.is_null() { return false; }
    let check: Check = unsafe { std::mem::transmute::<*mut libc::c_void, Check>(f) };
    unsafe { check(libc::getpid(), std::ptr::null(), 0) == 1 }
}

#[cfg(windows)]
pub fn os_says() -> bool { false }
