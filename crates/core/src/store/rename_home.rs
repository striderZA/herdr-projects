//! The one-time rename of the home folder: `~/.herdr-projects`, the name before this one, becomes `~/.herdr-wingman` by
//! the first run of this code. Where it can be renamed, the old name is left as a link to the new one, so whatever still
//! runs from it carries on. Where it cannot (on Windows a program still has a file open in it), this run works in the old
//! folder as it did, and the next run tries again. Nothing is merged, copied or deleted. docs/home-folder.md has the story.
use super::files::*;
use super::move_home::{add_line, env_now, lstat, move_plan, real};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The name of the home folder before this one: all that finds it.
pub const OLD_HOME_NAME: &str = ".herdr-projects";

/// The home folder this run works in, given the user's home, renaming the old one first when it can. The line for the
/// human is None when there was nothing to say.
pub fn rename_home(user: &Path) -> (PathBuf, Option<String>) {
    let (old, new) = (user.join(OLD_HOME_NAME), user.join(HOME_NAME));
    let real_folder = lstat(&old).map(|m| m.is_dir() && !m.file_type().is_symlink()).unwrap_or(false);
    if !real_folder { return (new, None); }
    if lstat(&new).is_some() {
        if real(&new) == real(&old) { return (new, None); }
        return (old.clone(), Some(format!("Both ~/{OLD_HOME_NAME} and ~/{HOME_NAME} are there, so the board uses ~/{OLD_HOME_NAME}. Move what is in one into the other, or remove ~/{HOME_NAME} if it is empty, and the board uses ~/{HOME_NAME} from then on.")));
    }
    match std::fs::rename(&old, &new) {
        Ok(()) => {
            let linked = crate::sys::symlink(&new, &old).is_ok();
            let link = if linked { " The old name is a link to it, so whatever still runs from it carries on." } else { "" };
            (new.clone(), Some(format!("Renamed ~/{OLD_HOME_NAME} to ~/{HOME_NAME}.{link}")))
        }
        Err(e) => (old.clone(), Some(format!("Could not rename ~/{OLD_HOME_NAME} to ~/{HOME_NAME} yet ({e}): the board uses ~/{OLD_HOME_NAME} until it can."))),
    }
}

/// At the start of every run: the rename, and its line recorded beside the home the board works in, to be said once on
/// a terminal. A folder named outright by HERDR_WINGMAN_HOME, and a test run, are left alone.
pub fn at_start(user: &Path) {
    if std::env::var_os("HERDR_WINGMAN_HOME").is_some() || std::env::var_os("HERDR_WINGMAN_TESTING").is_some() { return; }
    let (home, line) = rename_home(user);
    if home != user.join(HOME_NAME) { unsafe { std::env::set_var("HERDR_WINGMAN_HOME", &home) }; }
    let Some(line) = line else { return };
    let file = move_plan(user, &env_now()).file;
    let mut rec = read_obj(&file).unwrap_or_default();
    add_line(&mut rec, &line);
    let _ = write_json(&file, &Value::Object(rec));
}
