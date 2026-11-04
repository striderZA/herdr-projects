//! A coordinator's settings (`coordinator.*`), looked up as a task's are, nearest first:
//!   this coordinator -> you -> built in
//! "This coordinator" is its own entry in the `coordinators` part of your config.json (`main`,
//! `projects.<repo>`). No coordinator's setting is ever read from a repo's file.
use super::lookup::Scope;
use crate::json::*;
use crate::store::settings_list::{Entry, Kind};

/// Whose coordinator: its key as config.json's `coordinators` names it ("main", "project:<repo>").
#[derive(Clone, Debug, Default)]
pub struct Coord { pub key: String }
impl Coord {
    pub fn of(key: &str) -> Coord { Coord { key: key.to_string() } }
    /// Its words, as the settings page and `show` say where a value came from.
    pub fn place(&self) -> &'static str { "this coordinator" }
    /// Its part of config.json, as the path to it: `coordinators.main`, `coordinators.projects.<repo>`.
    pub fn path(&self) -> Vec<String> {
        match self.key.strip_prefix("project:") { Some(r) => vec!["coordinators".into(), "projects".into(), r.into()], None => vec!["coordinators".into(), self.key.clone()] }
    }
}
/// The `coordinators` part of config.json, by key: "main", "project:<repo>" ("machine" is an old main).
pub fn entries_of(v: &Value) -> Vec<(String, Value)> {
    let mut out = vec![];
    let Some(m) = v.as_object() else { return out };
    for (k, o) in m {
        if k == "projects" { for (r, p) in o.as_object().cloned().unwrap_or_default() { out.push((format!("project:{r}"), p)); } } else { out.push((k.clone(), o.clone())); }
    }
    out
}
/// The words of one coordinator's entry, as a person reads where it is.
pub fn entry_words(key: &str) -> String {
    match key { "main" | "machine" => "the Coordinator's own".into(), k => format!("{} coordinator's own", k.strip_prefix("project:").unwrap_or(k)) }
}
/// Why a setting cannot be set (or unset) in this scope, as a coordinator's settings go: a coordinator's setting only
/// for you or for one coordinator, and one coordinator takes nothing else.
pub fn refusal(e: &Entry, scope: &Scope) -> Option<String> {
    let name = &e.name;
    let flags = "--main-coordinator or --project-coordinator <repo>";
    if e.kind == Kind::Coord && (scope.task.is_some() || !scope.repo.is_empty()) {
        return Some(format!("{name} is a coordinator's setting: it cannot be set {}. Set it for yourself (every coordinator), or for one coordinator with {flags}.", if scope.task.is_some() { "on a task" } else { "for a repo" }));
    }
    if e.kind != Kind::Coord && scope.coord.is_some() { return Some(format!("{name} is not a coordinator's setting: only coordinator.* is said for one coordinator.")); }
    None
}
