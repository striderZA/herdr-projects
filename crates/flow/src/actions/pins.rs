//! Pinning one of this machine's projects, and unpinning it: kept in the data folder (store/pins.rs) and sent with this
//! machine's tasks to the board, as a task's pin is. A pin moves nothing on the board: it puts a mark on the row, and
//! the row under the Pinned filter.
use super::archive_done::repo_named;
use crate::err::Res;
use crate::shown::words::repo_label;

/// Pin one of this machine's projects (its name on the board, or its key), or unpin it: its name, and the words that
/// say what was done (`gtm: pinned`, `gtm: unpinned`).
pub fn pin_here(project: &str, on: bool) -> Res<(String, String)> {
    let word = if on { "pinned" } else { "unpinned" };
    // A pinned repo the board no longer knows can still be unpinned, by the name it was pinned by.
    let pinned = crate::store::pins::pins().repos.into_iter().find(|r| r.eq_ignore_ascii_case(project.trim()));
    let repo = match (pinned, on) { (Some(r), false) => r, _ => repo_named(project)? };
    if on && crate::store::archived_repos::is_archived(&repo) { crate::fail!("{} is archived: herdr-projects restore --repo {repo} brings it back, then it can be pinned.", repo_label(&repo)); }
    crate::store::pins::set(&repo, on)?;
    Ok((repo.clone(), format!("{}: {word}", repo_label(&repo))))
}
