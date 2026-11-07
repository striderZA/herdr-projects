//! Where the herdr program is when this process's PATH does not have it: a plain `ssh host cmd` gets no login shell,
//! so `~/.local/bin` and the like are often missing. The usual install spots are looked in, then a login shell's PATH,
//! once per process. Used by cli.rs (`bin`, `found`).
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where installers put herdr, under the home folder or not: looked in after PATH, in this order.
#[cfg(unix)]
pub const SPOTS: [&str; 4] = ["~/.local/bin", "~/.cargo/bin", "~/bin", "/usr/local/bin"];
#[cfg(windows)]
pub const SPOTS: [&str; 4] = ["~/.cargo/bin", "~/bin", "~/.herdr/packages/standalone/releases", "%LOCALAPPDATA%/Programs/Herdr/bin"];
/// The usual spots, `~` being `home`.
pub fn spots(home: &Path) -> Vec<PathBuf> {
    SPOTS.iter().filter_map(|s| {
        if let Some(r) = s.strip_prefix("~/") { return Some(home.join(r)); }
        if let Some(r) = s.strip_prefix("%LOCALAPPDATA%/") { return std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join(r)); }
        Some(PathBuf::from(s))
    }).collect()
}

pub fn runs(p: &Path) -> bool { crate::sys::is_program(p) }
/// The file names a program `name` may have: as given, and on Windows with each program extension (herdr is herdr.exe).
pub fn names(name: &str) -> Vec<String> {
    if cfg!(windows) && !name.contains('.') { ["", ".exe", ".cmd", ".bat", ".com"].iter().map(|e| format!("{name}{e}")).collect() } else { vec![name.to_string()] }
}
/// The program `name` in one folder, if it runs there.
fn in_dir(dir: &Path, name: &str) -> Option<PathBuf> { names(name).iter().map(|n| dir.join(n)).find(|p| runs(p)) }
/// The first of `dirs` (a PATH, its separator between) that has a program `name` that runs.
pub fn in_dirs(name: &str, dirs: &str) -> Option<PathBuf> { std::env::split_paths(dirs).filter(|d| !d.as_os_str().is_empty()).find_map(|d| in_dir(&d, name)) }
/// Where `name` is: None when `path` has it (the name alone runs it), else the first of `spots` that has it, else the
/// first dir of `login_path` (asked only then) that has it; None too when nothing has it.
pub fn elsewhere(name: &str, path: &str, spots: &[PathBuf], login_path: impl FnOnce() -> String) -> Option<PathBuf> {
    if in_dirs(name, path).is_some() { return None; }
    spots.iter().find_map(|d| in_dir(d, name)).or_else(|| in_dirs(name, &login_path()))
}
/// herdr's full path when this process's PATH lacks it and it is found elsewhere; asked once per process.
pub fn herdr_elsewhere() -> Option<String> {
    static FOUND: OnceLock<Option<String>> = OnceLock::new();
    FOUND.get_or_init(|| {
        let path = std::env::var("PATH").unwrap_or_default();
        elsewhere("herdr", &path, &spots(&crate::store::files::user_home()), || crate::login_env::login_shell().map(|o| crate::login_env::path_of(&o)).unwrap_or_default()).map(|p| p.display().to_string())
    }).clone()
}
