//! The Windows console: the stop callback that Ctrl+C, Ctrl+Break and the console's close run, and the standard input read
//! that waits for a key press with a timeout.
use std::io;
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
static STOP: std::sync::OnceLock<extern "C" fn(i32)> = std::sync::OnceLock::new();
/// A console event (Ctrl+C, Ctrl+Break, the console closed): the stop callback runs, and the event is handled.
unsafe extern "system" fn console_event(_kind: u32) -> i32 {
    if let Some(f) = STOP.get() { f(0); }
    1
}
/// Calls `f` on a stop, which here is a console event. The first callback registered is the one kept.
pub fn on_stop(f: extern "C" fn(i32), _sigs: &[i32]) {
    if STOP.set(f).is_ok() { unsafe { SetConsoleCtrlHandler(Some(console_event), 1); } }
}
/// Reads standard input for up to `ms` milliseconds: what came in (at most `buf` long); 0 when nothing did, or it ended.
/// On a console the wait is for a key: the console signals for every record (a resize, a focus change, a mouse move), and
/// a read started for one of those would then wait on for the next key. So the records are looked at, the ones that are
/// not keys are dropped, and the wait goes on. A pipe is polled for bytes (a pipe cannot be waited on); a file is read.
pub fn read_input_within(buf: &mut [u8], ms: i32) -> usize {
    use std::io::Read;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
    use windows_sys::Win32::Storage::FileSystem::{GetFileType, FILE_TYPE_CHAR, FILE_TYPE_PIPE};
    use windows_sys::Win32::System::Console::{GetStdHandle, PeekConsoleInputW, ReadConsoleInputW, INPUT_RECORD, KEY_EVENT, STD_INPUT_HANDLE};
    use windows_sys::Win32::System::Pipes::PeekNamedPipe;
    use windows_sys::Win32::System::Threading::WaitForSingleObject;
    let until = Instant::now() + Duration::from_millis(ms.max(0) as u64);
    // SAFETY: the standard input handle is the process's own; every call below gets a buffer it sized itself.
    unsafe {
        let h = GetStdHandle(STD_INPUT_HANDLE);
        let kind = GetFileType(h);
        if kind == FILE_TYPE_CHAR {
            loop {
                let left = until.saturating_duration_since(Instant::now()).as_millis() as u32;
                if WaitForSingleObject(h, left) != WAIT_OBJECT_0 { return 0; }
                let mut recs: [INPUT_RECORD; 32] = std::mem::zeroed();
                let mut n = 0u32;
                if PeekConsoleInputW(h, recs.as_mut_ptr(), recs.len() as u32, &mut n) == 0 { return 0; }
                let key = recs[..n as usize].iter().any(|r| r.EventType == KEY_EVENT as u16 && r.Event.KeyEvent.bKeyDown != 0 && r.Event.KeyEvent.uChar.UnicodeChar != 0);
                if key { break; }
                if n > 0 {
                    let mut dropped = 0u32;
                    if ReadConsoleInputW(h, recs.as_mut_ptr(), n, &mut dropped) == 0 { return 0; }
                }
                if Instant::now() >= until { return 0; }
            }
        } else if kind == FILE_TYPE_PIPE {
            loop {
                let mut avail = 0u32;
                if PeekNamedPipe(h, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut avail, std::ptr::null_mut()) == 0 { return 0; }
                if avail > 0 { break; }
                if Instant::now() >= until { return 0; }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    io::stdin().read(buf).unwrap_or(0)
}
