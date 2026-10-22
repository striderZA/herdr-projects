//! Voice calls are off on Windows in v1: starting a call says so, and no sound is opened.

#[cfg(windows)]
#[test]
fn starting_a_call_says_calls_are_off_on_windows() {
    use sw::calls::native::audio;
    let err = audio::open().err().expect("no sound is opened on Windows");
    assert!(err.contains("off on Windows in v1"), "{err}");
}
