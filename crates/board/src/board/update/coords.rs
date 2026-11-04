//! The coordinators behind the list's headings: space on the top row or a repo's heading talks to that coordinator, as
//! space on a task talks to its agent. The chat then follows the selection to the next heading or task, and space again
//! goes back to the Coordinator. What you type goes to that coordinator.
use super::heads::{is_head, label, HEAD};
use super::Effect;
use crate::board::job::Job;
use crate::board::state::{Row, State};
use crate::json::*;
use crate::shown::ink::Ink;

/// A heading's coordinator: its level as the coordinator code names it ("project:<repo>"), and its name
/// ("herdr-projects coordinator"). The top-level one has an empty level.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target { pub to: String, pub name: String }
impl Target {
    /// Its colour where it is named: the base colour (colour means status only).
    pub fn ink(&self) -> Ink { Ink::Plain }
    /// What the chat keeps your words to it under (update/chat.rs), beside the tasks' ids.
    pub fn key(&self) -> String { format!("co:\t{}", self.to) }
}
/// That coordinator as the board read it last.
pub fn look_of(s: &State, t: &Target) -> Option<crate::coordinator::look::Look> {
    if t.to.is_empty() { return Some(s.data.coord.clone()); }
    s.data.sub.as_ref().filter(|x| x.0 == t.to).map(|x| x.1.clone()).or_else(|| s.data.looks.get(&t.to).cloned())
}
/// The coordinator a heading stands for: the top row, or a repo's. A status group's has none.
pub fn of_head(s: &State, id: &str) -> Option<Target> {
    let rest = id.strip_prefix(HEAD)?;
    // No coordinator can run here (`coordinator.agent none`): the rows are headings only.
    if !s.data.can_dispatch { return None; }
    match rest {
        "machine" => Some(Target { to: String::new(), name: super::top::coordinator_of(&label(s)) }),
        // An archived project has no coordinator to talk to until it is restored.
        r if super::heads::archived_head(s, r.strip_prefix("repo\t").unwrap_or("")) => None,
        r => r.strip_prefix("repo\t").map(|repo| Target { to: format!("project:{repo}"), name: super::top::coordinator_of(&super::heads::repo_shown(repo)) }),
    }
}
/// The coordinator the chat talks to now, if it talks to one by its row: the selected row's, or where it talks to no
/// row's agent the top row's (top.rs). None for a task's agent, and for the top-level coordinator when it is that by
/// default: that one goes the Coordinator's own way (actions.rs: submit), which says where you were.
pub fn talking(s: &State) -> Option<Target> {
    if s.reply && is_head(&s.reply_id) && s.reply_id == s.sel { if let Some(t) = of_head(s, &s.sel) { return Some(t); } }
    if s.reply && s.reply_id == s.sel && s.current().is_some() { return None; }
    super::top::top(s).filter(|t| !t.to.is_empty())
}
/// The coordinator the board should read, as the coordinator code names its level ("" none): the one the chat talks
/// to, else the one whose row is selected (its panel shows it).
pub fn wanted(s: &State) -> String { talking(s).or_else(|| of_head(s, &s.sel)).map(|t| t.to).unwrap_or_default() }
/// The coordinators the board reads how they stand of, as the coordinator code names their level: the selected
/// row's (its panel), and every project's ("*"): their rows show their marks, and the filter row counts those that run.
pub fn looks_wanted(s: &State) -> Vec<String> {
    let mut out: Vec<String> = of_head(s, &s.sel).filter(|t| !t.to.is_empty()).map(|t| vec![t.to]).unwrap_or_default();
    out.push("*".into());
    out
}
/// Is this row still on the board: a task, or a heading.
pub fn exists(s: &State, id: &str) -> bool { s.items.iter().any(|r| r.s("id") == id) || s.rows.iter().any(|r| matches!(r, Row::Head { id: h, .. } if h == id)) }
/// Space on a heading: talk to its coordinator. A status group's heading has none: it says where they are.
pub fn flip(s: &mut State) -> bool {
    if !is_head(&s.sel) { return false; }
    if of_head(s, &s.sel).is_none() { s.flash(format!("Space on the top row or a repo's heading talks to its coordinator. {} groups the tasks by repo.", crate::board::keys::key_name("byRepo")), Ink::Muted); return true; }
    s.reply = true; s.reply_id = s.sel.clone(); s.panel_top = 0;
    true
}
/// What you typed, for that coordinator. It decides itself what to start.
pub fn send(s: &mut State, t: &Target, text: &str, fx: &mut Vec<Effect>) {
    if text.is_empty() { return; }
    if !s.data.can_dispatch { return s.flash(format!("No coordinator can run here, so nothing was sent. {}", crate::coordinator::who::OFF_WORDS), Ink::Yellow); }
    fx.push(Effect::Run(Job::CoordSend { cwd: s.ctx.cwd.clone(), text: text.into(), delegate: false, about: String::new(), to: t.to.clone() }));
    super::chat::note_coord(s, t, text);
    s.input.set_text("");
    fx.push(Effect::Load);
}
