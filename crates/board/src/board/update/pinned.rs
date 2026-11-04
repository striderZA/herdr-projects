//! The Pinned filter, and pins from the board's side. A pin moves nothing: a pinned task or project keeps its place in
//! the board's list, with a pin mark after its name. Pinned, the last filter on the filter row (shows.rs), leaves only
//! what is pinned, in its places and the board's grouping, as every filter does: a pinned project with its tasks, a
//! pinned task alone (docs/pinned.md).
use super::heads::{archived_head, HEAD};
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::store::pins::Pins;

/// The pin mark after a pinned row's name: the same for a project and a task, in yellow (`ink`).
pub const MARK: &str = "★";
/// The pin mark after a name: a space, then the mark in its yellow.
pub fn ink() -> Inked { Inked::plain(" ").add(Inked::of(MARK, Ink::Yellow)) }
/// The Pinned filter shows (not in the archive, which has no filter).
pub fn on(s: &State) -> bool { s.show == super::shows::PINNED && !s.archive_view }
fn pins(s: &State) -> &Pins { &s.data.pins }
/// The project `repo` is pinned (an archived one is off the board, pinned or not).
pub fn project_pinned(s: &State, repo: &str) -> bool { pins(s).repo(repo) && !archived_head(s, repo) }
fn repo_of(r: &Obj) -> &str { r.s("id").split('/').next().unwrap_or("") }
/// A task stands under Pinned: pinned itself, or in a pinned project. Never an archived one.
pub fn keeps(s: &State, r: &Obj) -> bool { !r.b("archivedAt") && (r.b("pinned") || project_pinned(s, repo_of(r))) }
/// The project of a project's row; None for any other row.
pub fn place(id: &str) -> Option<String> { id.strip_prefix(HEAD)?.strip_prefix("repo\t").map(String::from) }
/// The heading `id` is a pinned project's row.
pub fn head_pinned(s: &State, id: &str) -> bool { place(id).is_some_and(|r| project_pinned(s, &r)) }
/// The pinned projects.
pub fn projects_of(s: &State) -> Vec<String> { pins(s).repos.iter().filter(|r| project_pinned(s, r)).cloned().collect() }
/// Under Pinned by repo, the names of the pinned projects: each has its heading, with no task too.
pub fn names(s: &State) -> Vec<String> { if !on(s) || !s.by_repo { return vec![]; } projects_of(s) }
/// Under Pinned by repo, the repo `name` has its heading with no task in view: a pinned project.
pub fn empty_repo(s: &State, name: &str) -> bool { s.by_repo && project_pinned(s, name) }
/// Under Pinned, nothing is pinned: the list says how to pin something.
pub fn nothing(s: &State) -> bool { on(s) && s.items.is_empty() && projects_of(s).is_empty() }
/// What the list says under Pinned with nothing pinned.
pub fn empty_says() -> String { format!("Nothing pinned yet. {} on a task or a project pins it here.", key_name("pin")) }
/// Pinned's count on the filter row: what is pinned, each pin once, as `list --pinned` lists them: the pinned projects
/// and the pinned tasks (`live`, archived ones left out). The other tasks of a pinned project stand under Pinned but
/// pin nothing of their own: they are not counted.
pub fn count(s: &State, live: &[&std::sync::Arc<Obj>]) -> usize { live.iter().filter(|r| r.b("pinned")).count() + projects_of(s).len() }

/// The pin key: the selected task or project's row pinned, or unpinned.
pub fn key(s: &mut State, r: Option<&Obj>, fx: &mut Vec<Effect>) {
    if let Some(r) = r { return super::actions::pin(s, r, fx); }
    let Some(repo) = place(&s.sel).filter(|_| !s.archive_view) else { return s.flash(format!("{} pins a task or a project's row: select one.", key_name("pin")), Ink::Muted) };
    let on = !head_pinned(s, &s.sel);
    // Its row says `Pinning…` while it goes, as its coordinator's row says `Starting…` (update/pending.rs).
    fx.push(Effect::Run(Job::PinPlace { repo, on }));
}
/// The pin key's line in the ? menu, for the selected row ("" a row it does nothing on).
pub fn menu_label(s: &State, r: Option<&Obj>) -> String {
    if let Some(r) = r { return if r.b("pinned") { "unpin this task".into() } else { "pin this task: a mark on its row, and under Pinned".into() }; }
    if place(&s.sel).filter(|_| !s.archive_view).is_none() { return String::new(); }
    if head_pinned(s, &s.sel) { "unpin this project".into() } else { "pin this project: a mark on its row, and under Pinned".into() }
}
