//! Where the board keeps things, reading and writing files, and locks.
use crate::clock::now;
use crate::err::{Error, Res};
use crate::json::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Every stage a task can be in. The board's five states are derived from these in state.rs.
/// "plan": its plan is written and waits for your yes. Several attempts: `attempts` (under way), `pick` (more than one
/// passed the check: you choose), `tried` (an attempt that passed). "after": delegated, and waits for other tasks to be
/// done (after.rs).
pub const STAGES: &[&str] = &["todo", "progress", "queued", "starting", "setup", "working", "stopped", "nosummary", "asks", "checking",
    "pushing", "reviewing", "reviewneeds", "plan", "approval", "merging", "failed", "done", "attempts", "pick", "tried", "after"];
/// A slot is held only while something is actually running; tasks waiting for the human never block the queue.
pub const SLOT_STAGES: &[&str] = &["starting", "setup", "working", "checking", "pushing", "reviewing", "merging"];
/// Stages owned by a runner (a long job of ours), as opposed to the agent's own turn.
pub const RUNNER_STAGES: &[&str] = &["starting", "setup", "checking", "pushing", "reviewing", "merging"];
pub const KINDS: &[&str] = &["permission", "question", "auth"];
/// The version of this program: what the marks and the keeper compare.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Everything the board keeps for a user is in one folder, the same on every machine: ~/.herdr-projects, with a
// sub-folder for each kind of thing (docs/home-folder.md). HERDR_PROJECTS_HOME names another one; each sub-folder
// has a variable of its own too. The XDG_* variables only say where an older version kept things (move_home.rs).
// Under the tests none of them may be the real one (never_real.rs).
pub const HOME_NAME: &str = ".herdr-projects";
fn env(k: &str) -> Option<String> { std::env::var(k).ok().filter(|v| !v.is_empty()) }
pub fn user_home() -> PathBuf { env(crate::sys::HOME_VAR).map(PathBuf::from).unwrap_or_else(crate::sys::root_dir) }
pub fn home_dir() -> PathBuf { env("HERDR_PROJECTS_HOME").map(PathBuf::from).unwrap_or_else(|| user_home().join(HOME_NAME)) }
/// Each part of the home folder: its own variable, else its place in the home folder.
const PARTS: [(&str, &str); 3] = [("HERDR_PROJECTS_DATA_DIR", "data"), ("HERDR_PROJECTS_CONFIG_DIR", "settings"), ("HERDR_PROJECTS_REPORTS_DIR", "reports")];
fn part((k, name): (&str, &str)) -> PathBuf { super::never_real::guard(env(k).map(PathBuf::from).unwrap_or_else(|| home_dir().join(name))) }
pub fn data_dir() -> PathBuf { part(PARTS[0]) }
pub fn config_dir() -> PathBuf { part(PARTS[1]) }
/// Pages and reports an agent makes for the human that belong in no repo.
pub fn reports_dir() -> PathBuf { part(PARTS[2]) }
/// The board's own folders as resolved here, for a process started somewhere that does not inherit this
/// environment (an agent's herdr space): the home folder, and a part's own variable only for a part kept elsewhere.
/// So HERDR_PROJECTS_HOME set to a scratch folder in front of a command there moves all of it: a part's variable
/// handed on would have kept that part in the real board (HP-172).
pub fn board_env() -> Vec<(String, String)> {
    let home = absolute(&home_dir());
    let mut out = vec![("HERDR_PROJECTS_HOME".to_string(), home.display().to_string())];
    for (k, name) in PARTS {
        let p = absolute(&part((k, name)));
        if p != home.join(name) { out.push((k.to_string(), p.display().to_string())); }
    }
    out
}
pub fn tasks_dir() -> PathBuf { data_dir().join("tasks") }
pub fn has_tasks() -> bool { tasks_dir().exists() }
pub fn dir_of(id: &str) -> PathBuf { let mut p = tasks_dir(); for part in id.split('/') { p.push(part); } p }
pub fn artifacts_dir(id: &str) -> PathBuf { dir_of(id).join("artifacts") }
/// This program, by its full path: what agents and hooks are handed, and what a process starts its successors from.
/// Once an update put a new file at that path, Linux names the running one "… (deleted)", a file nothing can be started
/// from: the path is the one without it, where the new program is.
pub fn bin() -> PathBuf {
    let Ok(p) = std::env::current_exe() else { return PathBuf::from("herdr-wingman") };
    crate::sys::without_deleted(p)
}
pub fn absolute(p: &Path) -> PathBuf { if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf()) } }

/// The user's name, as the owner of a task.
pub fn me() -> String {
    let name = crate::sys::passwd_name().unwrap_or_default();
    let name = if name.is_empty() { env("USER").unwrap_or_default() } else { name };
    let clean: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || "_.+-".contains(*c)).take(32).collect();
    if clean.is_empty() { "you".into() } else { clean }
}

pub fn read_text(file: &Path) -> Option<String> { fs::read_to_string(file).ok() }
/// A JSON file, or None when it is missing or not JSON.
pub fn read_json(file: &Path) -> Option<Value> { read_text(file).and_then(|t| serde_json::from_str(&t).ok()) }
pub fn read_obj(file: &Path) -> Option<Obj> { match read_json(file) { Some(Value::Object(o)) => Some(o), _ => None } }
/// A JSON object file read to be written again: one that is there but does not read is refused, never taken for empty
/// (writing it back would drop everything it holds); one that is not there is a new one.
pub fn read_obj_or(file: &Path) -> Res<Obj> {
    let text = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Obj::new()),
        Err(e) => return Err(Error::new(format!("{} cannot be read ({e}). Nothing was changed.", file.display()))),
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(o)) => Ok(o),
        _ => Err(Error::new(format!("{} does not read as JSON. Nothing was changed: fix or remove it, then try again.", file.display()))),
    }
}
/// Written whole or not at all, and durable before the caller is told it was: the bytes on the disk, then (on Unix)
/// the folder's own entry, so a power cut cannot leave the new name pointing at nothing.
pub fn write_json(file: &Path, value: &Value) -> Res<()> {
    write_atomic(file, serde_json::to_string_pretty(&tidy(value.clone()))?.as_bytes())
}
pub fn write_atomic(file: &Path, bytes: &[u8]) -> Res<()> { write_atomic_as(file, bytes, false) }
/// As `write_atomic`, for a file only this user may read (a hook's token): made private when it is made.
pub fn write_private(file: &Path, bytes: &[u8]) -> Res<()> { write_atomic_as(file, bytes, true) }
fn write_atomic_as(file: &Path, bytes: &[u8], private: bool) -> Res<()> {
    if let Some(d) = file.parent() { fs::create_dir_all(d)?; }
    let tmp = PathBuf::from(format!("{}.{}.{}.tmp", file.display(), std::process::id(), crate::util::random_hex(4)));
    let done = (|| -> std::io::Result<()> {
        let mut o = OpenOptions::new();
        o.write(true).create(true).truncate(true);
        if private { crate::sys::private_mode(&mut o); }
        let mut f = o.open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        crate::sys::replace_file(&tmp, file)
    })();
    if let Err(e) = done { let _ = fs::remove_file(&tmp); return Err(e.into()); }
    if let Some(d) = file.parent() { let _ = crate::sys::sync_dir(d); }
    Ok(())
}
/// Whether an id names a task's own folder under tasks/: `<repo>/<number>`, an attempt's letter after the number too.
/// Nothing that could walk out of that folder (an empty part, `..`, the other platform's separator).
pub fn id_ok(id: &str) -> bool {
    let mut parts = id.split('/');
    let (Some(repo), Some(n), None) = (parts.next(), parts.next(), parts.next()) else { return false };
    if repo.is_empty() || repo == "." || repo == ".." || repo.chars().any(|c| c == '\\' || c == ':' || c == '\0') { return false; }
    let digits = n.strip_suffix(|c: char| "abcde".contains(c)).unwrap_or(n);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}
pub fn append_line(file: &Path, value: &Value) -> Res<()> {
    if let Some(d) = file.parent() { fs::create_dir_all(d)?; }
    let mut f = OpenOptions::new().create(true).append(true).open(file)?;
    f.write_all(format!("{}\n", serde_json::to_string(&tidy(value.clone()))?).as_bytes())?;
    Ok(f.sync_data()?)
}
/// One JSON value a line. A line that does not read as JSON is left out (a write cut off), never the whole file with it.
pub fn read_lines(file: &Path) -> Vec<Value> {
    let Some(text) = read_text(file) else { return vec![] };
    text.split('\n').filter(|l| !l.is_empty()).filter_map(|l| serde_json::from_str(l).ok()).collect()
}

pub fn pid_alive(pid: i64) -> bool { crate::sys::pid_alive(pid) }
/// A process marker is a pid plus that process's start time, so a pid reused after a reboot is not mistaken for it.
pub fn proc_start(pid: i64) -> Option<String> { crate::sys::proc_start(pid) }
pub fn marker() -> Obj {
    let pid = std::process::id() as i64;
    obj(json!({ "pid": pid, "start": proc_start(pid), "started": now() }))
}
pub fn marker_alive(m: &Value) -> bool {
    let (pid, start) = (m.i("pid"), m.t("start"));
    pid > 0 && pid_alive(pid) && (start.is_empty() || proc_start(pid).as_deref() == Some(start.as_str()))
}

// ---- locks ---------------------------------------------------------------------------------------------------------
// One kernel lock per name (flock), on a file of its own in data/locks. The kernel releases it when the process ends or
// crashes, so there are no stale owners to repair. A thread that takes a lock it already holds just goes on.
thread_local! { static HELD: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new()); }

fn lock_file(name: &str) -> PathBuf {
    data_dir().join("locks").join(format!("{}.lock", base64url(name.as_bytes())))
}
pub fn base64url(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
        for i in 0..chunk.len() + 1 { out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char); }
    }
    out
}
fn same_file(f: &File, path: &Path) -> bool { crate::sys::same_open_file(f, path) }
fn flock(f: &File, wait: bool) -> bool { crate::sys::lock_file(f, wait) }
/// A lock taken: it is let go when this is dropped.
pub struct Held { file: Option<File>, path: PathBuf }
impl Drop for Held {
    fn drop(&mut self) { if self.file.take().is_some() { HELD.with(|h| h.borrow_mut().remove(&self.path)); } }
}
fn take_lock(name: &str, path: &Path, wait_ms: i64) -> Res<File> {
    if let Some(d) = path.parent() { fs::create_dir_all(d)?; }
    let until = now() + wait_ms.max(0);
    loop {
        let f = crate::sys::open_lock(path)?;
        let mut got = flock(&f, false);
        while !got && now() < until && same_file(&f, path) {
            std::thread::sleep(std::time::Duration::from_millis(15));
            got = flock(&f, false);
        }
        if !got {
            if !same_file(&f, path) && now() < until { continue; }
            return Err(Error::busy(format!("{name} is busy with another action. Try again in a moment.")));
        }
        // The file was opened before it was locked. If another file took its place at this path in between (the
        // one-time move puts two folders of locks together), what is held is a lock nobody else will ever ask for.
        if !same_file(&f, path) { continue; }
        return Ok(f);
    }
}
/// Runs `f` while holding the lock of this name, waiting up to two seconds for it.
pub fn with_lock<T>(name: &str, f: impl FnOnce() -> Res<T>) -> Res<T> { with_lock_wait(name, 2000, f) }
pub fn with_lock_wait<T>(name: &str, wait_ms: i64, f: impl FnOnce() -> Res<T>) -> Res<T> {
    let path = lock_file(name);
    if HELD.with(|h| h.borrow().contains(&path)) { return f(); } // nested on this thread
    let file = take_lock(name, &path, wait_ms)?;
    HELD.with(|h| h.borrow_mut().insert(path.clone()));
    let _held = Held { file: Some(file), path };
    f()
}
/// A lock held until the returned value is dropped, or the process ends (the keeper holds one for its whole life).
pub fn hold_lock(name: &str, wait_ms: i64) -> Res<Held> {
    let path = lock_file(name);
    let file = take_lock(name, &path, wait_ms)?;
    HELD.with(|h| h.borrow_mut().insert(path.clone()));
    Ok(Held { file: Some(file), path })
}
/// Whether somebody holds the lock of this name right now (a lock taken and let go at once).
pub fn lock_free(name: &str) -> bool {
    let path = lock_file(name);
    match crate::sys::open_lock(&path) { Ok(f) => flock(&f, false), Err(_) => true }
}
/// A task's lock: an attempt (`repo/12b`) shares its own id's.
pub fn task_key(id: &str) -> String { id.split('/').take(2).collect::<Vec<_>>().join("/") }
pub fn with_task_lock<T>(id: &str, wait_ms: i64, f: impl FnOnce() -> Res<T>) -> Res<T> {
    with_lock_wait(&format!("task:{}", task_key(id)), wait_ms, f)
}
/// Waits for a lock forever (the one-time moves and the keeper's start).
pub fn wait_lock(name: &str) -> Res<Held> {
    let path = lock_file(name);
    if let Some(d) = path.parent() { fs::create_dir_all(d)?; }
    let f = crate::sys::open_lock(&path)?;
    if !flock(&f, true) { return Err(Error::busy(format!("{name} is busy"))); }
    HELD.with(|h| h.borrow_mut().insert(path.clone()));
    Ok(Held { file: Some(f), path })
}
