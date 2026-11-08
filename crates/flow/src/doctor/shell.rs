//! The shell a repo's own commands run in, for `doctor`: on Windows there is no `sh` unless Git's is installed (its
//! `bin` on the PATH, or Git in its usual folder), and without one a repo's setup or check runs through `cmd` instead,
//! where a command written for a shell may not work (runner.rs).
use super::Found;
use std::path::Path;

/// The line for what was found: the shell that runs them, or none and what follows.
pub fn found(sh: Option<&Path>) -> Found {
    match sh {
        Some(p) => Found::ok(format!("A shell runs a repo's own setup and check commands ({})", crate::store::marks::tilde(&p.display().to_string()))),
        None => Found::bad("No sh is found: a repo's setup and check commands run through cmd, where a command written for a shell may fail", "Install Git for Windows, or put its bin folder on your PATH, then open the board again"),
    }
}
pub fn check() -> Vec<Found> { vec![found(crate::runner::sh_exe().as_deref())] }
