//! The platform layer's end signal, on Unix: TERM sent to this process sets the flag the program gave. The Windows half is
//! checked by a real Ctrl+C in Phase 4, and recorded there.
#[cfg(unix)]
#[test]
fn a_term_signal_sets_the_flag_the_program_gave() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};
    use sw::platform::signal;
    static FLAG: AtomicBool = AtomicBool::new(false);
    signal::on_end(&FLAG);
    unsafe { libc::kill(libc::getpid(), libc::SIGTERM) };
    let until = Instant::now() + Duration::from_secs(5);
    while !FLAG.load(Ordering::SeqCst) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(FLAG.load(Ordering::SeqCst), "TERM set the flag");
}
