//! The platform layer's shell: sh runs a command, prints its output and is found on PATH; the home folder comes from HOME on
//! Unix and USERPROFILE on Windows; a program on PATH is found by its name, and on Windows by its extension.
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use sw::platform::{perms, shell};

fn folder() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sw-platform-shell-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn sh_runs_a_command_and_prints_its_output() {
    let out = shell::sh().args(["-c", "echo ok"]).stdin(Stdio::null()).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn sh_is_found_on_the_path() {
    assert!(shell::has_program("sh"));
}

#[cfg(unix)]
#[test]
fn the_home_folder_is_home_on_unix() {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            assert_eq!(shell::user_home(), PathBuf::from(home));
        }
    }
}

#[cfg(windows)]
#[test]
fn the_home_folder_is_user_profile_on_windows() {
    assert_eq!(shell::user_home(), PathBuf::from(std::env::var("USERPROFILE").unwrap()));
}

#[test]
fn a_program_on_the_path_is_found_by_its_name() {
    let dir = folder();
    #[cfg(unix)]
    let file = dir.join("hp-tool");
    #[cfg(windows)]
    let file = dir.join("hp-tool.cmd");
    fs::write(&file, "x").unwrap();
    perms::make_executable(&file).unwrap();
    let path = std::env::join_paths([&dir]).unwrap();
    assert_eq!(shell::find_in(&path, "hp-tool"), Some(file.clone()));
    assert_eq!(shell::find_in(&path, "hp-missing"), None);
    let _ = fs::remove_file(&file);
}
