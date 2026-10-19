//! The platform layer's permissions: a private file holds the text written to it, and on Unix only its owner can read and
//! write it; a program is recognised by its execute bit on Unix and by its extension on Windows; a text file is not a
//! program; a folder the test made can be written.
use std::fs;
use std::path::PathBuf;
use sw::platform::perms;

fn folder() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sw-platform-perms-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_private_file_holds_the_text_written_to_it() {
    let path = folder().join("private.json");
    perms::write_private(&path, "{\"a\":1}").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
    perms::write_private(&path, "{}").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "{}", "a rewrite replaces the text in full");
    let _ = fs::remove_file(&path);
}

#[cfg(unix)]
#[test]
fn a_private_file_is_readable_and_writable_by_its_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let path = folder().join("mode.json");
    let _ = fs::remove_file(&path);
    perms::write_private(&path, "x").unwrap();
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    let _ = fs::remove_file(&path);
}

#[test]
fn a_program_is_recognised_and_a_text_file_is_not() {
    let text = folder().join("note.txt");
    fs::write(&text, "x").unwrap();
    assert!(!perms::is_program(&text));
    #[cfg(unix)]
    let program = {
        let p = folder().join("tool");
        fs::write(&p, "#!/bin/sh\n").unwrap();
        perms::make_executable(&p).unwrap();
        p
    };
    #[cfg(windows)]
    let program = {
        let p = folder().join("tool.exe");
        fs::write(&p, "x").unwrap();
        p
    };
    assert!(perms::is_program(&program));
    let _ = fs::remove_file(&text);
    let _ = fs::remove_file(&program);
}

#[test]
fn a_folder_the_test_made_can_be_written() {
    assert!(perms::can_write(&folder()));
}
