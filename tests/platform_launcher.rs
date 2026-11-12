//! The Windows launcher (bin/run.ps1) against a fake release folder, named by HERDR_WINGMAN_RELEASES as a file:/// URL:
//! the asset is downloaded, checked against its checksum and run with its arguments; a download that does not match is
//! not used, and a second start within ten minutes does not try again. Windows only: the launcher is PowerShell.
#![cfg(windows)]
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

const LAUNCHER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bin/run.ps1");
const ASSET: &str = "herdr-wingman-windows-x86_64.exe";

/// This test's plugin folder (its bin holds run.ps1; its Cargo.toml is version 1.2.27) and the fake release's folder.
fn folders(name: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("herdr-launcher-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let plugin = base.join("plugin");
    std::fs::create_dir_all(plugin.join("bin")).unwrap();
    std::fs::copy(LAUNCHER, plugin.join("bin").join("run.ps1")).unwrap();
    std::fs::write(plugin.join("Cargo.toml"), "[package]\nname = \"herdr-wingman\"\nversion = \"1.2.27\"\n").unwrap();
    let release = base.join("rel").join("download").join("v1.2.27");
    std::fs::create_dir_all(&release).unwrap();
    (plugin, release)
}

/// The SHA-256 of a file, as the release's .sha256 file names it (computed by PowerShell itself).
fn sha256_of(file: &Path) -> String {
    let script = format!(
        "[BitConverter]::ToString([System.Security.Cryptography.SHA256]::Create().ComputeHash([IO.File]::ReadAllBytes('{}'))).Replace('-','').ToLower()",
        file.display()
    );
    let out = Command::new("powershell.exe").args(["-NoProfile", "-Command", &script]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The fake release: a copy of `where.exe` as the asset, with its checksum (`sum`, or the real one when `None`).
fn publish(release: &Path, sum: Option<String>) {
    let asset = release.join(ASSET);
    std::fs::copy(r"C:\Windows\System32\where.exe", &asset).unwrap();
    let sum = sum.unwrap_or_else(|| sha256_of(&asset));
    std::fs::write(release.join(format!("{ASSET}.sha256")), format!("{sum}  {ASSET}\n")).unwrap();
}

/// Runs the launcher with `args`, with the fake release folder `rel` as the releases.
fn launch(plugin: &Path, rel: &Path, args: &[&str]) -> Output {
    Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(plugin.join("bin").join("run.ps1"))
        .args(args)
        .env("HERDR_WINGMAN_RELEASES", format!("file:///{}", rel.display().to_string().replace('\\', "/")))
        .output()
        .unwrap()
}

#[test]
fn the_launcher_fetches_checks_and_runs_the_program() {
    let (plugin, release) = folders("fetch");
    publish(&release, None);
    let rel = plugin.parent().unwrap().join("rel");
    let fetched = launch(&plugin, &rel, &["--fetch"]);
    assert!(fetched.status.success(), "{}", String::from_utf8_lossy(&fetched.stderr));
    let bin = plugin.join("bin");
    assert!(bin.join(ASSET).is_file());
    assert_eq!(std::fs::read_to_string(bin.join(format!("{ASSET}.version"))).unwrap().trim(), "1.2.27");
    // The program runs with the arguments it was given, and the launcher exits with its exit code.
    let ran = launch(&plugin, &rel, &["notepad.exe"]);
    assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
    assert!(String::from_utf8_lossy(&ran.stdout).to_lowercase().contains("notepad.exe"));
}

#[test]
fn a_download_that_does_not_match_its_checksum_is_not_used_and_not_tried_again_at_once() {
    let (plugin, release) = folders("mismatch");
    publish(&release, Some("0".repeat(64)));
    let rel = plugin.parent().unwrap().join("rel");
    let first = launch(&plugin, &rel, &["notepad.exe"]);
    assert_eq!(first.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&first.stderr).contains("does not match its checksum"));
    assert!(plugin.join("bin").join(format!("{ASSET}.failed")).exists());
    let second = launch(&plugin, &rel, &["notepad.exe"]);
    assert_eq!(second.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&second.stderr).contains("downloading"));
}

#[test]
fn a_newer_download_replaces_a_program_that_is_running() {
    // Windows will not write over a program that runs; the launcher moves that copy aside and puts the download in its place.
    let (plugin, release) = folders("replace");
    publish(&release, None);
    let rel = plugin.parent().unwrap().join("rel");
    assert!(launch(&plugin, &rel, &["--fetch"]).status.success());
    let bin = plugin.join("bin");
    let prog = bin.join(ASSET);
    // The copy that runs is a long ping, and its version is older than the release, so the launcher downloads it again.
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    std::fs::copy(format!(r"{system}\System32\ping.exe"), &prog).unwrap();
    std::fs::write(bin.join(format!("{ASSET}.version")), "1.2.26\n").unwrap();
    let mut running = Command::new(&prog).args(["-n", "30", "127.0.0.1"]).stdout(Stdio::null()).spawn().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let ran = launch(&plugin, &rel, &["notepad.exe"]);
    let still_running = running.try_wait().unwrap().is_none();
    let _ = running.kill();
    let _ = running.wait();
    assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
    assert!(String::from_utf8_lossy(&ran.stdout).to_lowercase().contains("notepad.exe"));
    assert_eq!(std::fs::read_to_string(bin.join(format!("{ASSET}.version"))).unwrap().trim(), "1.2.27");
    assert!(still_running, "the program that ran from the file was stopped by the download");
}
