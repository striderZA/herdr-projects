//! Replacing a file on Windows: a rename that Windows refuses for a moment is retried, and a busy target is moved aside.
use std::io;
use std::path::{Path, PathBuf};
/// Puts `from` at `to`, replacing what is there. Windows refuses that while a program runs from `to`, or while another
/// program has it open without delete sharing (an editor or a scanner may; Rust's own opens share delete, so do not block it).
/// Some of those pass in a moment, so the rename
/// is tried for a second; after that a busy `to` is moved aside (a running program may be renamed, not replaced) and
/// `from` put in its place. The copies moved aside are removed the next time that happens, once no program runs from them.
pub fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    let busy = |e: &io::Error| matches!(e.raw_os_error(), Some(5 | 32 | 33));
    let mut first = None;
    for _ in 0..50 {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if busy(&e) => { first = Some(e); std::thread::sleep(std::time::Duration::from_millis(20)); }
            Err(e) => return Err(e),
        }
    }
    let first = first.unwrap_or_else(|| io::Error::other("the file stayed busy"));
    clear_old(to);
    let aside = PathBuf::from(format!("{}.{}.old", to.display(), std::process::id()));
    if std::fs::rename(to, &aside).is_err() { return Err(first); }
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) => { let _ = std::fs::rename(&aside, to); Err(e) }
    }
}
/// Removes the copies `replace_file` moved aside for `to` earlier, where no program runs from them any more.
fn clear_old(to: &Path) {
    let (Some(dir), Some(name)) = (to.parent(), to.file_name().and_then(|n| n.to_str())) else { return };
    let prefix = format!("{name}.");
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_str().is_some_and(|f| f.starts_with(&prefix) && f.ends_with(".old")) { let _ = std::fs::remove_file(e.path()); }
    }
}
