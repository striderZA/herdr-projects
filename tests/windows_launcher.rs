//! The Windows plugin launcher fetches a local release and installs its executable launcher.
#![cfg(windows)]
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

static LOCK: Mutex<()> = Mutex::new(());
fn scratch() -> (PathBuf, MutexGuard<'static, ()>) {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = std::env::temp_dir().join(format!("herdr-windows-launcher-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    (root, guard)
}
fn run(script: &Path, args: &[&str], release: &Path, data: &Path) -> std::process::Output {
    Command::new("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script).args(args).env("HERDR_PROJECTS_RELEASES", release)
        .env("HERDR_PROJECTS_DATA_DIR", data).output().expect("start PowerShell")
}

#[test]
fn install_fetches_local_release_and_writes_launcher() {
    let (root, _guard) = scratch();
    let release = root.join("release");
    let version = env!("CARGO_PKG_VERSION");
    let assets = release.join("download").join(format!("v{version}"));
    std::fs::create_dir_all(&assets).unwrap();
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_herdr-projects"));
    let asset = assets.join("herdr-projects-windows-x86_64.exe");
    std::fs::copy(executable, &asset).unwrap();
    let hash = Command::new("powershell").args(["-NoProfile", "-Command"])
        .arg(format!("(Get-FileHash -LiteralPath '{}' -Algorithm SHA256).Hash.ToLowerInvariant()", asset.display()))
        .output().expect("hash release executable");
    assert!(hash.status.success(), "{}", String::from_utf8_lossy(&hash.stderr));
    let checksum = String::from_utf8_lossy(&hash.stdout).trim().to_owned();
    std::fs::write(assets.join("herdr-projects-windows-x86_64.exe.sha256"),
        format!("{checksum}  herdr-projects-windows-x86_64.exe\n")).unwrap();

    let plugin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/run.ps1");
    let data = root.join("data");
    let install = run(&plugin, &["install"], &release, &data);
    assert!(install.status.success(), "stdout: {}\nstderr: {}", String::from_utf8_lossy(&install.stdout), String::from_utf8_lossy(&install.stderr));
    let launcher = data.join("bin/herdr-projects.exe");
    assert!(launcher.is_file(), "install did not write {}", launcher.display());
    let version_output = Command::new(&launcher).arg("version").output().unwrap();
    assert!(version_output.status.success(), "{}", String::from_utf8_lossy(&version_output.stderr));
    assert!(String::from_utf8_lossy(&version_output.stdout).contains(&format!("herdr-projects {version} ")),
        "unexpected version output: {}", String::from_utf8_lossy(&version_output.stdout));
    std::fs::remove_dir_all(root).unwrap();
}
