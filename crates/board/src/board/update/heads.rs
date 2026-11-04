//! The list's headings, which the selection lands on as on a row: this machine's row above its repos' or its status
//! groups', its automations among them. Enter on one folds it to its one line, which says how many tasks (and
//! automations) it holds, and enter again unfolds it; what is folded is kept with the board's view (run/prefs.rs), so it
//! stays folded while the list refreshes and after the board closes. A repo used lately with no task in view has its
//! heading too, folded (`0 tasks`).
use super::list::{filter_now, relist, AUTOMATIONS};
use super::Effect;
use crate::board::state::{Rec, Row, State};
use crate::json::*;
use crate::shown::ink::Ink;

/// What every heading's id starts with: no task's or automation's id does.
pub const HEAD: &str = "head:";
pub fn is_head(id: &str) -> bool { id.starts_with(HEAD) }
/// This machine's heading: the list's top row.
pub fn machine_id() -> String { format!("{HEAD}machine") }
/// A group's heading: grouped by repo and by status each fold their own. An archived project's in the archive is a
/// repo's, whatever the grouping.
pub fn group_id(s: &State, name: &str) -> String { format!("{HEAD}{}\t{name}", if s.by_repo || archived_head(s, name) { "repo" } else { "status" }) }
/// Whether `name` is an archived project with its own heading here: in the archive, whatever the grouping
/// (update/archive_project.rs), with its tasks under it, or alone (`0 tasks`).
pub fn archived_head(s: &State, name: &str) -> bool { s.archive_view && s.data.archived.iter().any(|r| r == name) }
/// The archived projects.
pub fn archived_heads(s: &State) -> Vec<String> { s.data.archived.clone() }
/// A repo's name as its row reads: the home folder as `~`.
pub fn repo_shown(name: &str) -> String { crate::shown::words::repo_label(name) }
/// This machine's row as it reads: its own name.
pub fn label(s: &State) -> String {
    if s.data.host.is_empty() { "this machine".into() } else { crate::shown::words::clean(&s.data.host) }
}
/// Folding holds while nothing is filtered: every task that matches a filter shows.
pub fn folded(s: &State, id: &str) -> bool { filter_now(s).is_empty() && s.folded.contains(id) }
/// Whether the top row has its heading: not while a find, the filter row or the archive narrows the list.
pub fn empty_shown(s: &State) -> bool { filter_now(s).is_empty() && !s.archive_view && s.show == 0 }
/// Whether the repo `name` has its heading with nothing in view: by repo, for a repo used lately (data.rs:
/// used_lately), and not while a filter or the archive narrows the list. It shows folded (`0 tasks`), so that its
/// project coordinator is still a space away.
pub fn empty_repo(s: &State, name: &str) -> bool {
    if super::pinned::on(s) { return super::pinned::empty_repo(s, name); } // Pinned's own (pinned.rs)
    s.by_repo && empty_shown(s) && s.data.lately.iter().any(|r| r == name)
}
/// The groups under this machine's top row, and whether they are shown at all (all but in the archive: the top row is
/// not there).
pub struct Part { pub heading: bool, pub groups: Vec<(String, Vec<Rec>)> }
/// The one part of the list: this machine's groups.
pub fn parts(s: &State, groups: &[(String, Vec<Rec>)]) -> Vec<Part> { vec![Part { heading: !s.archive_view, groups: groups.to_vec() }] }
/// The automations that stand under the group `name`: the automations' own group (by status), or those of the repo
/// `name` (by repo).
pub fn autos_in(s: &State, name: &str) -> Vec<Obj> {
    if !s.by_repo && name != AUTOMATIONS { return vec![]; }
    s.autos.iter().filter(|a| !s.by_repo || a.s("repo") == name).cloned().collect()
}
/// What a folded heading says it holds: `7 tasks`, `7 tasks, 1 automation`, `1 automation`.
pub fn holds(folded: usize, autos: usize) -> String {
    let tasks = folded.saturating_sub(autos);
    let say = |n: usize, what: &str| format!("{n} {what}{}", if n == 1 { "" } else { "s" });
    [(tasks > 0 || autos == 0).then(|| say(tasks, "task")), (autos > 0).then(|| say(autos, "automation"))].into_iter().flatten().collect::<Vec<_>>().join(", ")
}
/// The selected heading, when the selection is on one.
pub fn selected(s: &State) -> Option<&Row> { if !is_head(&s.sel) { return None; } s.rows.iter().find(|r| matches!(r, Row::Head { id, .. } if *id == s.sel)) }
/// What enter does on the selected heading, in words: for the hint line ("" nothing: a status group's has no agent).
pub fn says(s: &State) -> Option<String> {
    let Some(Row::Head { id, .. }) = selected(s) else { return None };
    Some(if super::coords::of_head(s, id).is_some() { "go into it".into() } else { String::new() })
}
/// What ← or → does on the selected heading, in words ("" nothing to fold).
pub fn fold_says(s: &State) -> String {
    let Some(r @ Row::Head { empty, .. }) = selected(s) else { return String::new() };
    if *empty { String::new() } else if r.shut() { "unfold".into() } else { "fold".into() }
}
/// ← (`shut`) and →: fold the selected heading, or unfold it; ← on a task's or an automation's row goes to the heading
/// it stands under.
pub fn fold_key(s: &mut State, shut: bool, fx: &mut Vec<Effect>) {
    if !is_head(&s.sel) {
        if !shut { return; }
        let Some(at) = s.rows.iter().position(|r| super::list::step_id(r).as_deref() == Some(s.sel.as_str())) else { return };
        if let Some(Row::Head { id, .. }) = s.rows[..at].iter().rev().find(|r| matches!(r, Row::Head { .. })) { s.sel = id.clone(); }
        return;
    }
    let id = s.sel.clone();
    if s.folded.contains(&id) != shut { toggle(s, &id, fx); }
}
/// Fold the heading `id`, or unfold it; the selection stays on it (← →, a click). One with nothing in view says so.
pub fn toggle(s: &mut State, id: &str, fx: &mut Vec<Effect>) {
    if let Some(Row::Head { name, machine, empty: true, .. }) = s.rows.iter().find(|r| matches!(r, Row::Head { id: h, .. } if h == id)) {
        let talk = crate::board::keys::key_name("talk");
        let say = if s.archive_view { format!("{} was archived with no tasks. {} brings it back.", crate::shown::words::repo_label(name), crate::board::keys::key_name("archive")) }
            else if *machine { format!("No tasks on {name} yet. {talk} talks to its coordinator, which adds them there.") }
            else { format!("No tasks in {} yet. {talk} talks to its project coordinator, which adds them there.", crate::shown::words::repo_label(name)) };
        s.sel = id.to_string();
        return s.flash(say, Ink::Muted);
    }
    if !s.folded.remove(id) { s.folded.insert(id.to_string()); }
    s.sel = id.to_string();
    fx.push(Effect::Save);
    relist(s);
}
/// A task the selection was taken to (one just added, a jump, a found tag) is never folded away: its group unfolds.
pub fn reveal(s: &mut State) {
    let sel = s.sel.clone();
    let found = s.grouped.iter().find_map(|(n, recs)| recs.iter().any(|r| r.s("id") == sel).then(|| n.clone()))
        .or_else(|| s.autos.iter().find(|a| a.s("row") == sel).map(|a| if s.by_repo { a.t("repo") } else { AUTOMATIONS.to_string() }));
    let Some(name) = found else { return };
    let (m, g) = (machine_id(), group_id(s, &name));
    s.folded.remove(&m); s.folded.remove(&g);
}
