//! `pin` and `unpin` on the command line: a task by its id, a project by its name or its key; and `list --pinned`,
//! what is pinned on this machine: its projects and its tasks (docs/pinned.md).
use crate::actions::{filing, pins::pin_here};
use crate::err::Res;
use crate::json::*;

/// `pin` (or `unpin`) `pos[1]`: what --json prints and what a person reads. `shown`: a task as `list` gives it.
pub fn command(cmd: &str, pos: &[String], _flags: &Obj, shown: &dyn Fn(&str) -> Value) -> Res<(Value, String)> {
    let on = cmd == "pin";
    let word = if on { "pinned" } else { "unpinned" };
    let Some(want) = pos.get(1).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string()) else { crate::fail!("{cmd} needs a task's id or a project's name. Try: herdr-wingman help") };
    if pos.len() > 2 { crate::fail!("{cmd} takes one task or project: herdr-wingman {cmd} <id | project>"); }
    // A task here.
    if let Ok(t) = crate::store::tasks::find(&want) { let t = filing::set_pin(t.s("id"), on)?; return Ok((shown(t.s("id")), format!("{}: {word}", t.s("id")))); }
    // A project here: a name the board does not know says which it does.
    let (name, msg) = pin_here(&want, on).map_err(|e| format!("\"{want}\" is no task or project the board knows. {}", e.msg))?;
    Ok((json!({ "repo": name, "pinned": on }), msg))
}
/// `list --pinned`: the pinned projects and the pinned tasks, one line each.
pub fn list(tasks: Vec<Obj>, line: &dyn Fn(&Obj) -> String, brief: &dyn Fn(&Obj) -> Value) -> (Value, String) {
    let pins = crate::store::pins::pins();
    let tasks: Vec<Obj> = tasks.into_iter().filter(|t| t.b("pinned") && !t.b("archivedAt")).collect();
    let mut text = vec![];
    for r in &pins.repos { text.push(format!("project   {}", crate::shown::words::repo_label(r))); }
    text.extend(tasks.iter().map(line));
    let v = json!({ "projects": pins.repos, "tasks": tasks.iter().map(brief).collect::<Vec<_>>() });
    (v, if text.is_empty() { "Nothing pinned.".into() } else { text.join("\n") })
}
