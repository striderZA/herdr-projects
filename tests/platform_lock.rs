//! The platform layer's file locks: while one handle holds a lock, a second one cannot take it without waiting; once the
//! first handle is dropped the lock is free; a free file can be locked by waiting.
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use sw::platform::lock;

/// A file of this test's own, in a folder named for this test process.
fn lock_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sw-platform-lock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// Opened the way the program opens its lock files: created if missing, for reading and appending. A lock needs a handle
/// that can read or write (Windows' LockFileEx refuses one that only appends).
fn open(path: &Path) -> File {
    OpenOptions::new().create(true).read(true).append(true).open(path).unwrap()
}

#[test]
fn a_held_lock_refuses_a_second_handle_until_the_first_is_dropped() {
    let path = lock_path("held.lock");
    let first = open(&path);
    assert!(lock::try_lock(&first), "a free file can be locked");
    let second = open(&path);
    assert!(!lock::try_lock(&second), "a held lock refuses a second handle");
    drop(first);
    assert!(lock::try_lock(&second), "the lock is free once the first handle is dropped");
    drop(second);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_free_file_can_be_locked_by_waiting() {
    let path = lock_path("free.lock");
    let f = open(&path);
    assert!(lock::lock(&f), "a free file can be locked by waiting");
    drop(f);
    let _ = std::fs::remove_file(&path);
}
