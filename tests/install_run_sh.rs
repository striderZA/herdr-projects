//! `bin/run.sh`, the launcher herdr runs on Unix: it finds the program built in the checkout or downloads the release
//! file for this platform, once, checked against its `.sha256`, and runs it; and it asks the repo the checkout was
//! cloned from where the releases are. No network: a release folder and tools of the test's own (file://).
use crate::common;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;
use sw::install::*;

static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn set(k: &str, v: &str) { unsafe { std::env::set_var(k, v) } }
/// A scratch home of its own: HOME and HERDR_WINGMAN_HOME point into it.
fn scratch() -> (PathBuf, MutexGuard<'static, ()>) {
    let one = common::one_at_a_time(); // the environment is the whole test program's
    let base = std::env::temp_dir().join(format!("sw-run-sh-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("home")).unwrap();
    let base = sw::sys::canonicalize(&base).unwrap();
    set("HOME", &base.join("home").display().to_string());
    set("HERDR_WINGMAN_HOME", &base.join("sw").display().to_string());
    for k in ["HERDR_WINGMAN_DATA_DIR", "HERDR_WINGMAN_CONFIG_DIR", "HERDR_WINGMAN_RELEASES"] { unsafe { std::env::remove_var(k) } }
    (base, one)
}
fn write(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
fn exec(p: &Path, text: &str) { write(p, text); sw::sys::set_mode(p, 0o755).unwrap(); }
fn read(p: &Path) -> String { std::fs::read_to_string(p).unwrap_or_default() }
fn git(dir: &Path, args: &[&str]) { common::git(dir, args); }
fn here() -> (&'static str, &'static str) { platform().expect("the tests run on a platform a release is made for") }

/// A plugin folder of its own with this repo's run.sh and a Cargo.toml of `version`, and a release folder.
fn plugin(b: &Path, version: &str) -> PathBuf {
    let p = b.join("plugin");
    write(&p.join("bin/run.sh"), &read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("bin/run.sh")));
    write(&p.join("Cargo.toml"), &format!("[package]\nname = \"herdr-wingman\"\nversion = \"{version}\"\n"));
    write(&p.join("src/main.rs"), "");
    p
}
fn publish(b: &Path, version: &str, says: &str) {
    let f = b.join(format!("rel/download/v{version}")).join(asset_name(here()));
    exec(&f, &format!("#!/bin/sh\necho \"{says} $*\"\n"));
    write(&PathBuf::from(format!("{}.sha256", f.display())), &format!("{}  {}\n", sha256(&f).unwrap(), asset_name(here())));
}
fn run_sh(b: &Path, args: &[&str]) -> (i32, String, String) {
    let o = std::process::Command::new("sh").arg(b.join("plugin/bin/run.sh")).args(args).env("HERDR_WINGMAN_RELEASES", format!("file://{}/rel", b.display())).env("PATH", "/usr/bin:/bin").output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}
/// A PATH string of these folders, joined the way this platform's PATH is (`:` on Unix, `;` on Windows).
fn path_of(dirs: &[std::path::PathBuf]) -> String { std::env::join_paths(dirs).unwrap().to_string_lossy().into_owned() }

#[test]
// Runs run.sh through sh: Unix-only (a Windows account has no /bin/sh and the script is a shell script).
#[cfg(unix)]
fn real_programs_run_sh_downloads_once_and_runs_it() {
    let (b, _one) = scratch();
    if std::process::Command::new("curl").arg("--version").output().is_err() { eprintln!("no curl here: skipped"); return; }
    let p = plugin(&b, "0.3.0");
    // No release of this version yet: said plainly, nothing to run.
    let (code, out, err) = run_sh(&b, &["hook"]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(err.contains("could not download") && err.contains("cargo build --release"), "{err}");
    assert!(!p.join(format!("bin/{}.lock", asset_name(here()))).exists(), "no lock is left");
    // Not tried again on every command (each hook would wait for it), but by the build step.
    let prog = p.join("bin").join(asset_name(here()));
    let failed = PathBuf::from(format!("{}.failed", prog.display()));
    assert!(!run_sh(&b, &["hook"]).2.contains("downloading"));
    publish(&b, "0.3.0", "release 0.3.0");
    let (code, _, err) = run_sh(&b, &["--fetch"]);
    assert!(code == 0 && err.contains("downloading herdr-wingman 0.3.0") && !failed.exists(), "{err}");
    let (code, out, err) = run_sh(&b, &["hook", "x y"]);
    assert_eq!((code, out.as_str()), (0, "release 0.3.0 hook x y\n"), "{err}");
    assert_eq!(read(&version_file(&prog)), "0.3.0\n");
    // Downloaded once: the next start runs it at once, without the release.
    std::fs::remove_dir_all(b.join("rel")).unwrap();
    assert_eq!(run_sh(&b, &["board"]), (0, "release 0.3.0 board\n".into(), String::new()));
    // `update` put a newer one there: it is kept.
    write(&version_file(&prog), "0.4.0\n");
    assert_eq!(run_sh(&b, &["board"]).1, "release 0.3.0 board\n");
    // The checkout moved on to a newer version: that one is downloaded; a wrong checksum is never run.
    write(&p.join("Cargo.toml"), "[package]\nversion = \"0.5.0\"\n");
    publish(&b, "0.5.0", "release 0.5.0");
    write(&b.join(format!("rel/download/v0.5.0/{}.sha256", asset_name(here()))), &format!("{}  x\n", "0".repeat(64)));
    let (code, out, err) = run_sh(&b, &["board"]);
    assert!(err.contains("does not match its checksum") && err.contains("using the downloaded herdr-wingman 0.4.0, not 0.5.0"), "{err}");
    assert_eq!((code, out.as_str()), (0, "release 0.3.0 board\n"), "the older download still runs");
    publish(&b, "0.5.0", "release 0.5.0");
    std::fs::remove_file(&failed).unwrap(); // ten minutes later
    assert_eq!(run_sh(&b, &["board"]).1, "release 0.5.0 board\n");
    // A program built from this checkout, newer than its source, comes first; once the source changes it does not.
    exec(&p.join("target/release/herdr-wingman"), "#!/bin/sh\necho \"built $*\"\n");
    assert_eq!(run_sh(&b, &["board"]).1, "built board\n");
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(p.join("src/main.rs")).unwrap().set_modified(later).unwrap();
    assert_eq!(run_sh(&b, &["board"]).1, "release 0.5.0 board\n", "the source is newer than the build");
    // --fetch only gets the program.
    assert_eq!(run_sh(&b, &["--fetch"]), (0, String::new(), String::new()));
}

#[test]
// Runs run.sh through sh, with a fake curl and gh that log what they are asked and fail: nothing reaches the network.
// Unix-only (a Windows account has no /bin/sh and the script is a shell script).
#[cfg(unix)]
fn run_sh_asks_the_repo_its_checkout_came_from() {
    let (b, _one) = scratch();
    let plugin = b.join("plugin");
    std::fs::create_dir_all(plugin.join("bin")).unwrap();
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("bin/run.sh"), plugin.join("bin/run.sh")).unwrap();
    write(&plugin.join("Cargo.toml"), "[package]\nname = \"herdr-wingman\"\nversion = \"1.2.28\"\n");
    git(&plugin, &["init", "-q"]);
    git(&plugin, &["remote", "add", "origin", "git@github.com:striderZA/herdr-wingman.git"]);
    let log = b.join("calls.log");
    for tool in ["curl", "gh"] {
        exec(&b.join("shims").join(tool), &format!("#!/bin/sh\necho \"{tool} $*\" >> '{}'\nexit 22\n", log.display()));
    }
    let old = std::env::var("PATH").unwrap_or_default();
    set("PATH", &format!("{}:{old}", b.join("shims").display()));
    let out = std::process::Command::new("sh").arg(plugin.join("bin/run.sh")).output().unwrap();
    set("PATH", &old);
    let calls = read(&log);
    assert!(calls.contains("https://github.com/striderZA/herdr-wingman/releases/download/v1.2.28/herdr-wingman-"), "{calls}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(!calls.contains("eliasstravik"), "{calls}");
}
