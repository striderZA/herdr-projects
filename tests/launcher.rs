//! The launcher (flow/launcher.rs): the files it is kept in, what each shell runs (the batch file for cmd and PowerShell,
//! the shell script for Git Bash), the older copy of the program that is removed, and the launcher running its program
//! with its environment and arguments (`real_programs_` tests: they start cmd, or sh, and the program they name).
use std::path::{Path, PathBuf};
use sw::flow::launcher::{launcher_files, launcher_target, remove_stale, stale_copies};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("herdr-launcher-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn env() -> Vec<(String, String)> {
    vec![("HERDR_PROJECTS_HOME".into(), r"C:\Users\Jaco\.herdr-projects".into()), ("HERDR_PROJECTS_DATA_DIR".into(), "/data/100%".into())]
}

#[test]
fn windows_launcher_is_a_batch_file_and_a_shell_script() {
    let files = launcher_files(true, &env(), Path::new(r"C:\Users\Jaco\herdr-wingman-windows-x86_64.exe"));
    let names: Vec<&str> = files.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, ["herdr-wingman.cmd", "herdr-wingman"]);
    let bat = &files[0].1;
    assert!(bat.starts_with("@echo off\r\nsetlocal\r\n"), "{bat:?}");
    assert!(bat.contains("set \"HERDR_PROJECTS_DATA_DIR=/data/100%%\"\r\n"), "% is doubled in a batch file: {bat:?}");
    assert!(bat.ends_with("\"C:\\Users\\Jaco\\herdr-wingman-windows-x86_64.exe\" %*\r\n"), "{bat:?}");
    let sh = &files[1].1;
    assert!(sh.contains("export HERDR_PROJECTS_HOME='C:\\Users\\Jaco\\.herdr-projects'\n"), "{sh:?}");
    assert!(sh.ends_with("exec 'C:/Users/Jaco/herdr-wingman-windows-x86_64.exe' \"$@\"\n"), "Git Bash takes forward slashes: {sh:?}");
}

#[test]
fn unix_launcher_is_one_shell_script() {
    let files = launcher_files(false, &env(), Path::new("/p/bin/herdr-wingman-linux-x86_64"));
    assert_eq!(files.iter().map(|(n, _)| *n).collect::<Vec<_>>(), ["herdr-wingman"]);
    assert_eq!(files[0].1, "#!/bin/sh\nexport HERDR_PROJECTS_HOME='C:\\Users\\Jaco\\.herdr-projects'\nexport HERDR_PROJECTS_DATA_DIR='/data/100%'\nexec '/p/bin/herdr-wingman-linux-x86_64' \"$@\"\n");
}

#[test]
fn the_launcher_names_its_program_back_in_both_forms() {
    let win = launcher_files(true, &env(), Path::new(r"C:\a%b\it's\herdr.exe"));
    assert_eq!(launcher_target(&win[0].1), Some(PathBuf::from(r"C:\a%b\it's\herdr.exe")), "the batch file's line");
    assert_eq!(launcher_target(&win[1].1), Some(PathBuf::from("C:/a%b/it's/herdr.exe")), "the shell script's line");
    let unix = launcher_files(false, &env(), Path::new("/a/it's/herdr-wingman"));
    assert_eq!(launcher_target(&unix[0].1), Some(PathBuf::from("/a/it's/herdr-wingman")));
    assert_eq!(launcher_target("#!/bin/sh\necho no program here\n"), None);
}

#[test]
fn the_copy_of_the_program_an_older_install_left_is_removed() {
    let dir = scratch("stale");
    for name in ["herdr-wingman.exe", "herdr-wingman.cmd", "herdr-wingman"] {
        std::fs::write(dir.join(name), "x").unwrap();
    }
    assert_eq!(stale_copies(&dir), vec![dir.join("herdr-wingman.exe")]);
    remove_stale(&dir);
    assert!(!dir.join("herdr-wingman.exe").exists());
    assert!(dir.join("herdr-wingman.cmd").exists() && dir.join("herdr-wingman").exists(), "the launcher files stay");
    assert!(stale_copies(&dir).is_empty());
}

#[test]
fn the_shell_script_beside_the_batch_file_is_the_launcher_on_the_path() {
    let dir = scratch("path");
    let shell = &launcher_files(false, &env(), Path::new("/p/herdr"))[0].1;
    let script = dir.join("herdr-wingman");
    std::fs::write(&script, shell).unwrap();
    sw::sys::set_mode(&script, 0o755).unwrap();
    let path = std::env::join_paths([&dir]).unwrap().to_string_lossy().into_owned();
    // The launcher is the batch file (Windows' path to it); the script beside it is the same launcher for Git Bash.
    let at = dir.join("herdr-wingman.cmd");
    assert_eq!(sw::install::path_problem(&at, Path::new("/p/herdr"), &path), None);
}

#[test]
fn a_stale_copy_beside_the_launcher_is_a_doctor_problem() {
    let dir = scratch("doctor");
    let launcher = dir.join("herdr-wingman");
    std::fs::write(dir.join("herdr-wingman.exe"), "old").unwrap();
    let h = sw::doctor::program::here(launcher, None, None);
    let found = sw::doctor::program::check(&h);
    assert!(found.iter().any(|f| f.problem && f.what.contains("older copy of the program")), "{:?}", found.iter().map(|f| &f.what).collect::<Vec<_>>());
}

#[cfg(unix)]
#[test]
fn real_programs_unix_launcher_runs_its_program_with_its_environment_and_arguments() {
    let dir = scratch("unix-run");
    let text = &launcher_files(false, &[("HERDR_PROJECTS_DATA_DIR".into(), "/data/100%".into())], Path::new("/usr/bin/env"))[0].1;
    let file = dir.join("herdr-wingman");
    std::fs::write(&file, text).unwrap();
    sw::sys::set_mode(&file, 0o755).unwrap();
    let out = std::process::Command::new(&file).arg("MARK=1").output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(said.contains("HERDR_PROJECTS_DATA_DIR=/data/100%\n") && said.contains("MARK=1"), "{said}");
}

#[cfg(windows)]
#[test]
fn real_programs_windows_batch_file_runs_its_program_with_its_environment_and_exit_code() {
    use std::os::windows::process::CommandExt;
    let dir = scratch("cmd-run");
    let env = vec![("HERDR_PROJECTS_DATA_DIR".to_string(), r"C:\board 100%\data".to_string())];
    let text = &launcher_files(true, &env, Path::new(r"C:\Windows\System32\cmd.exe"))[0].1;
    let file = dir.join("herdr-wingman.cmd");
    std::fs::write(&file, text).unwrap();
    // cmd runs the batch file with these arguments: cmd prints the variable the batch file set, then exits with 3.
    let out = std::process::Command::new("cmd.exe")
        .raw_arg(format!(r#"/d /c ""{}" /d /c "echo %HERDR_PROJECTS_DATA_DIR% & exit /b 3"""#, file.display()))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains(r"C:\board 100%\data"), "{}", String::from_utf8_lossy(&out.stdout));
}
