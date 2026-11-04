//! The filter row on top of the board: All · Coordinators · Needs you · Working · Ready · To do · Done · Automations
//! · Pinned, each with its count. Tab moves to the next, shift+tab back, a click picks one; the list then shows only the tasks of
//! that status. Coordinators and Automations are filters as the others are: Coordinators leaves the coordinators' own
//! rows, which are the top row's and the projects', of those that run (started and not stopped, or failed); Automations leaves the automations' rows, in the same places and grouping;
//! Pinned leaves what is pinned, in its places and the board's grouping (pinned.rs).
use super::heads::{machine_id, HEAD};
use super::list::relist;
use super::Effect;
use crate::board::keys::Press;
use crate::board::state::{Row, State};
use crate::json::*;
use crate::shown::states::group_of;

/// The filters, each with the status group it shows (None: every task, the coordinators, the automations, what is pinned).
pub const SHOWS: [(&str, Option<&str>); 9] = [("All", None), ("Coordinators", None), ("Needs you", Some("needs you")), ("Working", Some("working")), ("Ready", Some("ready to approve")), ("To do", Some("to do")), ("Done", Some("done")), ("Automations", None), ("Pinned", None)];
pub const COORDS: usize = 1;
pub const NEEDS: usize = 2;
pub const AUTOS: usize = 7;
pub const PINNED: usize = 8;

/// The filter Coordinators is on (not in the archive, which holds tasks only).
pub fn coords(s: &State) -> bool { s.show == COORDS && !s.archive_view }
fn group(s: &State) -> Option<&'static str> { if s.archive_view { None } else { SHOWS.get(s.show).and_then(|x| x.1) } }
/// A task stays in the list of this filter: every task, those of its status, or what is pinned.
pub fn keeps_task(s: &State, r: &Obj) -> bool {
    if s.archive_view { return true; }
    if s.show == PINNED { return super::pinned::keeps(s, r); }
    s.show != COORDS && s.show != AUTOS && group(s).is_none_or(|g| group_of(r) == g)
}
/// Automations stand in the list under All and under Automations.
pub fn keeps_all(s: &State) -> bool { s.show == 0 || s.show == AUTOS || s.archive_view }
/// Done shows every done task, which the other filters hide.
pub fn whole(s: &State) -> bool { group(s) == Some("done") }
/// How many each filter holds, from the tasks in view (archived ones left out), the coordinators' rows, the
/// automations' and the pinned rows its filter shows.
pub fn counts(s: &State, items: &[std::sync::Arc<Obj>]) -> Vec<usize> {
    let live: Vec<&std::sync::Arc<Obj>> = items.iter().filter(|r| !r.b("archivedAt")).collect();
    SHOWS.iter().enumerate().map(|(n, (_, g))| match g { Some(g) => live.iter().filter(|r| group_of(r) == *g).count() + if n == NEEDS { needing(s) } else { 0 },
        None if n == COORDS => coords_running(s), None if n == AUTOS => super::list::autos_shown(s, "").len(),
        None if n == PINNED => super::pinned::count(s, &live), None => live.len() }).collect()
}
/// Tab: the next filter; shift+tab the one before.
pub fn step(s: &mut State, p: &Press, fx: &mut Vec<Effect>) {
    let n = SHOWS.len();
    let to = if p.chord.starts_with("shift+") { (s.show + n - 1) % n } else { (s.show + 1) % n };
    set(s, to, fx);
}
/// Show the filter `n`.
pub fn set(s: &mut State, n: usize, fx: &mut Vec<Effect>) {
    s.show = n.min(SHOWS.len() - 1);
    fx.push(Effect::Save);
    relist(s);
}
/// Enter or space on a coordinator in the list: talk to it, by its row.
pub fn talk(s: &mut State, id: &str) {
    s.sel = id.to_string();
    super::coords::flip(s);
}
/// The repos that have a project coordinator, as far as the board knows.
fn projects(s: &State) -> Vec<String> {
    let mut out: Vec<String> = s.data.coords.iter().filter_map(|k| k.strip_prefix("project:").map(String::from)).collect();
    out.sort_by_key(|r| r.to_lowercase());
    out
}
/// The coordinator a heading stands for runs: started and not stopped (working, idle, needs you), or failed, so that
/// it can be fixed. One stopped or never started is not under Coordinators.
pub fn running(s: &State, id: &str) -> bool {
    let Some(t) = super::coords::of_head(s, id) else { return false };
    let v = super::coords::look_of(s, &t).unwrap_or_default();
    !v.error.is_empty() || ["starting", "working", "blocked", "idle"].contains(&v.state.as_str())
}
/// The coordinator a heading stands for needs you: it asks you, wants your OK, or could not start. It shows under
/// Needs you beside the tasks, and is counted there.
pub fn needs_you(s: &State, id: &str) -> bool {
    let Some(t) = super::coords::of_head(s, id) else { return false };
    let v = super::coords::look_of(s, &t).unwrap_or_default();
    !v.error.is_empty() || v.state == "blocked" || !v.asks.is_empty()
}
/// The projects' rows that stand beside the tasks, though no task of theirs does: under Needs you those whose
/// coordinators ask you, under Pinned the pinned ones (by repo, those have their own heading).
pub fn beside(s: &State) -> Vec<String> {
    if s.archive_view { return vec![]; }
    if s.show == NEEDS { projects_needing(s) } else if s.show == PINNED && !s.by_repo { super::pinned::projects_of(s) } else { vec![] }
}
/// The project rows whose coordinators need you.
pub fn projects_needing(s: &State) -> Vec<String> { projects(s).into_iter().filter(|r| needs_you(s, &format!("{HEAD}repo\t{r}"))).collect() }
/// Every coordinator in view that needs you, by its row: the top row's, each project's.
fn needing(s: &State) -> usize { usize::from(needs_you(s, &machine_id())) + projects_needing(s).len() }
/// How many coordinators in view run, folded away or not: the top row's, each project's.
fn coords_running(s: &State) -> usize { usize::from(running(s, &machine_id())) + projects(s).iter().filter(|r| running(s, &format!("{HEAD}repo\t{r}"))).count() }
/// The list under Coordinators: the top row, then the rows of the projects whose coordinators run. The top row is
/// always there, counted only while it runs.
pub fn coord_rows(s: &State) -> Vec<Row> {
    let head = |name: String, id: String, machine: bool| Row::Head { name, autos: false, id, machine, folded: 0, folded_autos: 0, empty: false };
    let id = machine_id();
    let projects: Vec<String> = projects(s).into_iter().filter(|r| running(s, &format!("{HEAD}repo\t{r}"))).collect();
    let shut = super::heads::folded(s, &id) && !projects.is_empty();
    let mut out = vec![Row::Head { name: crate::board::update::heads::label(s), autos: false, id: id.clone(), machine: true, folded: if shut { projects.len() } else { 0 }, folded_autos: 0, empty: false }];
    if shut { return out; }
    for r in projects { out.push(head(r.clone(), format!("{HEAD}repo\t{r}"), false)); }
    out
}
