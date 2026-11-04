//! The settings page of a coordinator's row: that coordinator's own rows first (`coordinator.*` and `autopilot.*`,
//! under `This coordinator`), then the rows the heading's page showed before (yours on this machine's heading, your
//! entry for the repo on a repo's). While autopilot is on, a value of an
//! autopilot row (but the goal) is saved for this time; while off, as the coordinator's own.
use crate::board::state::{State, Whose};
use crate::board::update::coords::of_head;
use crate::json::*;

/// The page a coordinator's row opens, if the selected row is one.
pub fn whose_of(s: &State) -> Option<Whose> {
    let t = of_head(s, &s.sel)?;
    let key = if t.to.is_empty() { "main".to_string() } else { t.to.clone() };
    let then = match t.to.strip_prefix("project:") { Some(r) if s.data.repos.iter().any(|x| x == r) => format!("repo:{r}"), _ => String::new() };
    Some(Whose::Coord(key, then))
}
/// Its rows: the coordinator's own (marked `coordRow`), then the ones its heading's page showed before.
pub fn rows_of(s: &State, w: &Whose) -> Vec<Obj> {
    let Some(v) = super::edit::mine_read(s, w) else { return vec![] };
    let objs = |x: &Value| x.as_array().map(|a| a.iter().filter_map(|y| y.as_object().cloned()).collect::<Vec<Obj>>()).unwrap_or_default();
    let mut out: Vec<Obj> = objs(v.at("coord")).into_iter().map(|mut r| { r.put("coordRow", true); r }).collect();
    out.extend(objs(v.at("settings")));
    out
}
/// Whose a value saved on this page is: the coordinator's, for its own row; else the place under it.
pub fn whose_for(s: &State, name: &str, w: &Whose) -> Whose {
    let at = s.editing.as_ref().map(|e| e.at).unwrap_or(0);
    let all = super::edit::rows(s);
    let row = all.get(at).filter(|r| r.s("name") == name).or_else(|| all.iter().find(|r| r.s("name") == name));
    if row.map(|r| r.b("coordRow")).unwrap_or(false) { w.clone() } else { w.then() }
}
/// Is autopilot on for the coordinator of this page (its values are then saved for this time)?
pub fn on(s: &State, w: &Whose) -> bool { super::edit::mine_read(s, w).map(|v| v.b("on")).unwrap_or(false) }
