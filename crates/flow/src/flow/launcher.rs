//! The launcher: the command agents and hooks run, kept in the data folder with the board's environment written into it.
//! On Unix a shell script. On Windows two files: `herdr-projects.cmd`, a batch file that cmd and PowerShell run by name
//! (they run no file without an extension, and PowerShell runs a `.exe` before a `.cmd` of the same name), and
//! `herdr-projects`, a shell script beside it that Git Bash runs by name. Agents are given the batch file's path.
use crate::store::files::{bin, board_env, data_dir};
use crate::util::{shell_word, Shell};
use std::path::{Path, PathBuf};

/// Where the launcher is, without writing it (`doctor` looks at it as it is).
pub fn launcher_file() -> PathBuf {
    data_dir().join("bin").join(if cfg!(windows) { "herdr-projects.cmd" } else { "herdr-projects" })
}

/// The launcher's files as (name, text): one shell script on Unix; on Windows the batch file and the shell script. `exe`
/// is the program: the shell script names it with forward slashes, the form Git Bash runs.
pub fn launcher_files(windows: bool, env: &[(String, String)], exe: &Path) -> Vec<(&'static str, String)> {
    let exe = exe.display().to_string();
    if windows {
        vec![("herdr-projects.cmd", batch_text(env, &exe)), ("herdr-projects", sh_text(env, &exe.replace('\\', "/")))]
    } else {
        vec![("herdr-projects", sh_text(env, &exe))]
    }
}

/// The shell script: the environment, then the program with the arguments passed on.
fn sh_text(env: &[(String, String)], exe: &str) -> String {
    let mut t = "#!/bin/sh\n".to_string();
    for (k, v) in env { t.push_str(&format!("export {k}={}\n", shell_word(v, Shell::Posix))); }
    t.push_str(&format!("exec {} \"$@\"\n", shell_word(exe, Shell::Posix)));
    t
}

/// The batch file: the same environment, kept to this command (`setlocal`), then the program with the arguments (`%*`).
/// CRLF, as cmd reads it; `%` is doubled, as a batch file reads it. cmd reads it in the console's code page, so a path
/// with characters outside that page is not read right.
fn batch_text(env: &[(String, String)], exe: &str) -> String {
    let mut t = "@echo off\r\nsetlocal\r\n".to_string();
    for (k, v) in env { t.push_str(&format!("set \"{k}={}\"\r\n", v.replace('%', "%%"))); }
    t.push_str(&format!("\"{}\" %*\r\n", exe.replace('%', "%%")));
    t
}

/// The program a launcher runs: the path in its `exec '<program>' "$@"` line, or in its `"<program>" %*` line.
pub fn launcher_target(text: &str) -> Option<PathBuf> {
    let sh = text.lines().find_map(|l| l.strip_prefix("exec "))
        .and_then(|q| q.strip_suffix(" \"$@\"")).and_then(|q| q.strip_prefix('\'')?.strip_suffix('\''))
        .map(|p| p.replace("'\\''", "'"));
    let batch = text.lines().find_map(|l| l.strip_prefix('"')).and_then(|q| q.strip_suffix("\" %*")).map(|p| p.replace("%%", "%"));
    sh.or(batch).map(PathBuf::from)
}

/// The launcher's path in the data folder, so agents and hooks find it without anything on their PATH. It names its own
/// board: herdr cannot hand a worktree's space any environment. The files are written when they changed.
pub fn shim() -> PathBuf {
    if crate::herdr::cli::fake().is_none() {
        let dir = data_dir().join("bin");
        for (name, text) in launcher_files(cfg!(windows), &board_env(), &bin()) { write_if_changed(&dir.join(name), &text); }
        if cfg!(windows) { remove_stale(&dir); }
    }
    launcher_file()
}

fn write_if_changed(file: &Path, text: &str) {
    let is_link = std::fs::symlink_metadata(file).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    let have = if is_link { None } else { std::fs::read_to_string(file).ok() };
    if have.as_deref() == Some(text) { return; }
    let dir = file.parent().unwrap_or(Path::new("."));
    let _ = std::fs::create_dir_all(dir);
    let tmp = dir.join(format!("herdr-projects.{}.tmp", std::process::id()));
    let mut o = std::fs::OpenOptions::new();
    let written = crate::sys::executable_mode(o.write(true).create(true).truncate(true)).open(&tmp)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes())).is_ok();
    if written { let _ = crate::sys::replace_file(&tmp, file); } else { let _ = std::fs::remove_file(&tmp); }
}

/// The copies of the program that an older install left beside the launcher (`herdr-projects.exe`): PowerShell runs a
/// `.exe` before the batch file, so one left there takes the command.
pub fn stale_copies(dir: &Path) -> Vec<PathBuf> {
    ["herdr-projects.exe"].iter().map(|n| dir.join(n)).filter(|p| std::fs::symlink_metadata(p).is_ok()).collect()
}

/// Removes them. Best effort: a copy still running stays, and `doctor` names it.
pub fn remove_stale(dir: &Path) {
    for p in stale_copies(dir) { let _ = std::fs::remove_file(p); }
}
