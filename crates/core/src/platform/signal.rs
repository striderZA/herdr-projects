//! The end of the program, asked for: on Unix TERM, HUP and INT set a flag; on Windows the console's Ctrl+C, Ctrl+Break, close,
//! logoff and shutdown events do. The flag belongs to the caller, who decides what ending means for it.
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

/// The flag the last `on_end` was given. The end handler sets it.
static FLAG: AtomicPtr<AtomicBool> = AtomicPtr::new(std::ptr::null_mut());

fn set_flag() {
    let p = FLAG.load(Ordering::SeqCst);
    if !p.is_null() {
        // SAFETY: `on_end` stores only a `'static` flag, so the pointer stays valid for the rest of the program.
        unsafe { (*p).store(true, Ordering::SeqCst) };
    }
}

/// Makes the flag true when the program is asked to end. The flag given last is the one that is set.
pub fn on_end(flag: &'static AtomicBool) {
    FLAG.store(flag as *const AtomicBool as *mut AtomicBool, Ordering::SeqCst);
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(install);
}

#[cfg(unix)]
extern "C" fn on_signal(_sig: libc::c_int) { set_flag(); }

#[cfg(unix)]
fn install() {
    for sig in [libc::SIGTERM, libc::SIGHUP, libc::SIGINT] {
        unsafe { libc::signal(sig, on_signal as *const () as usize); }
    }
}

#[cfg(windows)]
unsafe extern "system" fn on_console_event(event: u32) -> i32 {
    // CTRL_C_EVENT, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT.
    if matches!(event, 0 | 1 | 2 | 5 | 6) {
        set_flag();
        1
    } else {
        0
    }
}

#[cfg(windows)]
fn install() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe { SetConsoleCtrlHandler(Some(on_console_event), 1) };
}
