//! The shell and the programs around herdr: the shell the user's commands run in, finding a program on PATH, running one that
//! is a batch file, whether a program is there at all, the login environment, the user's home and name, and the program's
//! own path. Unix: sh, `command -v`, $SHELL -lc, HOME, getpwuid. Windows: sh.exe from PATH or Git for Windows' sh.exe,
//! PATHEXT's programs, where.exe, .cmd and .bat through cmd /C, USERPROFILE, USERNAME, and no login environment.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The separator in PATH: `:` on Unix, `;` on Windows.
pub const PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// The shell the user's commands run in. Windows: sh.exe on PATH, else Git for Windows' sh.exe. Where neither is there,
/// the spawn fails as a missing program, and `doctor` says that Git for Windows is needed.
#[cfg(unix)]
pub fn sh() -> Command { Command::new("sh") }

#[cfg(windows)]
pub fn sh() -> Command { Command::new(resolve("sh").unwrap_or_else(|| PathBuf::from(r"C:\Program Files\Git\bin\sh.exe"))) }

/// The file a program of this name runs from, searched along PATH. Windows also tries the program extensions (exe, cmd,
/// bat, com); a name that has an extension of its own is looked for as it is.
pub fn resolve(name: &str) -> Option<PathBuf> { find_in(&std::env::var_os("PATH")?, name) }
/// The program a name runs, for a spawn: the file found for it along PATH (on Windows with its program extension, so a
/// `.cmd` shim runs as the program it is), or the name itself when none is found, so the spawn says it could not run it.
pub fn program_file(name: &str) -> PathBuf { resolve(name).unwrap_or_else(|| PathBuf::from(name)) }

/// The first program of this name along the given PATH: the search `resolve` makes.
pub fn find_in(path: &OsStr, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path).flat_map(|dir| candidates(&dir, name)).find(|p| crate::platform::perms::is_program(p))
}

#[cfg(unix)]
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> { vec![dir.join(name)] }

#[cfg(windows)]
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if Path::new(name).extension().is_some() { out.push(dir.join(name)); }
    for ext in ["exe", "cmd", "bat", "com"] { out.push(dir.join(format!("{name}.{ext}"))); }
    out
}

/// A command that runs the program of this name, found on PATH. Windows: a .cmd or .bat file runs through `cmd /C`.
#[cfg(unix)]
pub fn program(name: &str) -> Option<Command> { resolve(name).map(Command::new) }

#[cfg(windows)]
pub fn program(name: &str) -> Option<Command> {
    let path = resolve(name)?;
    let batch = matches!(path.extension().and_then(|e| e.to_str()), Some(e) if e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    if batch {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(path);
        Some(c)
    } else {
        Some(Command::new(path))
    }
}

/// Whether a program of this name can be run from here. Unix: `command -v` through sh. Windows: where.exe.
#[cfg(unix)]
pub fn has_program(name: &str) -> bool {
    use std::process::Stdio;
    Command::new("sh").args(["-c", &format!("command -v {name}")]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

#[cfg(windows)]
pub fn has_program(name: &str) -> bool {
    use std::process::Stdio;
    Command::new("where.exe").arg(name).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

/// A login shell's whole environment, as `env` prints it (what the shell prints before it is ignored). Unix: `$SHELL -lc env`.
/// Windows: none, since the PATH a Windows program has is already the user's.
#[cfg(unix)]
pub fn login_env() -> Option<String> {
    use std::process::Stdio;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let out = Command::new(shell).args(["-lc", "env"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(windows)]
pub fn login_env() -> Option<String> { None }

/// The user's home folder. Unix: HOME, else `/`. Windows: USERPROFILE, else the root of the system drive.
#[cfg(unix)]
pub fn user_home() -> PathBuf { std::env::var("HOME").ok().filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }

#[cfg(windows)]
pub fn user_home() -> PathBuf { std::env::var("USERPROFILE").ok().filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\")) }

/// The user's name. Unix: the account's name in the password file, or empty. Windows: USERNAME.
#[cfg(unix)]
pub fn user_name() -> String {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() { String::new() } else { std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned() }
    }
}

#[cfg(windows)]
pub fn user_name() -> String { std::env::var("USERNAME").unwrap_or_default() }

/// This program, by its full path. Unix: once an update put a new file at that path, Linux names the running one
/// "… (deleted)", and the path without that suffix is where the new program is.
#[cfg(unix)]
pub fn own_path() -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let Ok(p) = std::env::current_exe() else { return PathBuf::from("herdr-projects") };
    match p.as_os_str().as_bytes().strip_suffix(b" (deleted)") {
        Some(b) => PathBuf::from(OsStr::from_bytes(b)),
        None => p,
    }
}

#[cfg(windows)]
pub fn own_path() -> PathBuf {
    let p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("herdr-projects.exe"));
    // An update moved this running program aside as `X.exe.old`: the program at `X.exe` is the new one, the one to start.
    match p.to_str().and_then(|s| s.strip_suffix(".old")) { Some(s) if s.ends_with(".exe") && Path::new(s).exists() => PathBuf::from(s), _ => p }
}
