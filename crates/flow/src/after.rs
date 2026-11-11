//! A task that waits for others to be done (`--after`, docs/after.md): what it waits for, checked when it is said
//! (each task exists, and no task ends up waiting for itself), where each of those stands, and the words a waiting task
//! says. Done is the stage the board calls done, however it got there: merged, a research task's report finished, or
//! marked done. The pass that starts a task once its wait is over is after/pass.rs; what its agent is told of the work
//! before it is after/handed.rs.
pub mod change;
pub mod circle;
pub mod handed;
pub mod pass;

use crate::err::Res;
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};
use crate::store::tasks::{find, get, Task};

/// What a delegation says about waiting: the tasks named with `--after` (None: what the task had; an empty list or
/// `none`: nothing), and `--now`: start it without waiting.
#[derive(Default, Clone, Debug)]
pub struct Wait { pub after: Option<Vec<String>>, pub now: bool }
/// The stage of a delegated task that waits for others. It holds no slot and nothing runs for it.
pub const STAGE: &str = "after";

/// The tasks a text names: separated by commas or spaces. `none` (or nothing) names none.
pub fn refs_of(text: &str) -> Vec<String> {
    let v: Vec<String> = text.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(String::from).collect();
    if v.len() == 1 && v[0].eq_ignore_ascii_case("none") { vec![] } else { v }
}
/// What a task waits for, as kept on it: task ids.
pub fn after_of(t: &Obj) -> Vec<String> { strings(t.at("after")) }
/// A task as a person names it: `HSW-12` where its repo has a short key, else its id.
pub fn say(id: &str) -> String { say_id(id, &repo_keys()) }
/// Several tasks in one phrase: "a", "a and b", "a, b and c".
pub fn names(ids: &[String]) -> String {
    let n: Vec<String> = ids.iter().map(|i| say(i)).collect();
    match n.len() { 0 => String::new(), 1 => n[0].clone(), k => format!("{} and {}", n[..k - 1].join(", "), n[k - 1]) }
}
fn listed(ids: &[String]) -> String { ids.iter().map(|i| say(i)).collect::<Vec<_>>().join(", ") }
/// A waiting task's line: "queued after HSW-4, HSW-5", naming the tasks not done yet. A task delegated (tabbed) with
/// `--after` is queued, as one that waits for a free slot is: it starts by itself once they are done.
pub fn waits_line(left: &[String]) -> String { format!("queued after {}", listed(left)) }
/// A queued task's line on its row.
pub fn row_line(r: &Obj) -> String { waits_line(&strings(r.at("afterLeft"))) }
/// A task a row waits for, as this board names it.
fn say_on(_r: &Obj, id: &str) -> String { say(id) }
/// "once that is done", "once they are done": when a queued task starts by itself.
fn once(n: usize) -> &'static str { if n == 1 { "once that is done" } else { "once they are done" } }
/// What `list` says of a task in to do (nobody delegated it yet) that is to wait for others: "after HSW-4", naming the
/// ones not done yet (nothing once all are; nothing in any other stage, whose own words `list` says). It is only a
/// condition: such a task never starts by itself, and its row says nothing of it.
pub fn todo_line(t: &Obj) -> String {
    if t.s("stage") != "todo" || after_of(t).is_empty() { return String::new(); }
    let (left, _) = standing(t);
    if left.is_empty() { String::new() } else { format!("after {}", listed(&left)) }
}

/// Where a task that is waited for stands: done, not yet, or never by itself, with the reason.
#[derive(Debug, PartialEq)]
pub enum Stand { Done, Open, Broken(String) }
pub fn stand_of(id: &str) -> Stand {
    let Some(d) = get(id) else { return Stand::Broken(format!("{} was removed", say(id))) };
    if d.s("stage") == "done" { return Stand::Done; }
    if d.b("archivedAt") { return Stand::Broken(format!("{} was archived without being done", say(id))); }
    match d.s("stage") {
        "failed" | "nosummary" => Stand::Broken(format!("{} failed", say(id))),
        "stopped" => Stand::Broken(format!("{} was stopped", say(id))),
        "todo" if d.b("stoppedWaiting") => Stand::Broken(format!("{} was stopped", say(id))), // stopped while it waited itself
        _ => Stand::Open,
    }
}
/// What a task still waits for, and the first reason it cannot go on by itself.
pub fn standing(t: &Obj) -> (Vec<String>, Option<String>) {
    let (mut left, mut broken) = (vec![], None);
    for id in after_of(t) {
        match stand_of(&id) { Stand::Done => {}, Stand::Open => left.push(id), Stand::Broken(w) => { left.push(id); broken.get_or_insert(w); } }
    }
    (left, broken)
}

/// The tasks `refs` names, as ids, for task `me` to wait for (None: a task being added, which nothing waits for yet).
/// Refused, and nothing changed, for a task that does not exist or a wait in a circle.
pub fn checked(me: Option<&str>, refs: &[String]) -> Res<Vec<String>> {
    let mut out: Vec<String> = vec![];
    for r in refs {
        let d = find(r).map_err(|e| crate::err::Error::new(format!("{}: it cannot be waited for. Nothing was changed.", e.msg)))?;
        let id = if d.b("attemptOf") { d.t("attemptOf") } else { d.t("id") }; // an attempt is its task's
        if !out.contains(&id) { out.push(id); }
    }
    if let Some(me) = me {
        let mut g = circle::Graph::new(me, &out);
        if let Some(path) = g.circle(me) {
            let p: Vec<String> = path.iter().map(|i| g.say(i)).collect();
            crate::fail!("{} cannot wait for that: tasks would wait on each other ({}). Nothing was changed.", say(me), p.join(" → "));
        }
    }
    Ok(out)
}

/// What a delegated task that waits says it does: "is queued after a and starts by itself once that is done", or why
/// it waits for you. None for a task that does not wait.
pub fn waiting_words(t: &Obj) -> Option<String> {
    if t.s("stage") != STAGE { return None; }
    let left = strings(t.at("afterLeft"));
    Some(match t.so("afterBlocked") { Some(w) => format!("waits for you: {w}"), None => format!("is {} and starts by itself {}", waits_line(&left), once(left.len())) })
}
/// The task panel's line about waiting, from the task alone: its label ("Starts after", "Waited for", "To wait for" on
/// one not delegated yet) and each task with whether it is done.
pub fn panel_line(r: &Obj) -> Option<(&'static str, String)> {
    let all = after_of(r);
    if r.s("stage") == STAGE {
        let left = strings(r.at("afterLeft"));
        return Some(("Starts after", all.iter().map(|i| if left.contains(i) || !r.has("afterLeft") { say_on(r, i) } else { format!("{} (done)", say_on(r, i)) }).collect::<Vec<_>>().join(", ")));
    }
    let done = r.arr("afterDone");
    if !done.is_empty() { return Some(("Waited for", done.iter().map(|d| format!("{} ({})", say_on(r, d.s("id")), if d.b("done") { "done" } else { "not done" })).collect::<Vec<_>>().join(", "))); }
    (!all.is_empty()).then(|| ("To wait for", all.iter().map(|i| say_on(r, i)).collect::<Vec<_>>().join(", ")))
}
/// Said to a delegation of a task that already waits: it starts by itself, or early when asked.
pub fn still_waits(t: &Task) -> String {
    let id = t.t("id");
    let (left, broken) = standing(t);
    let now = match broken { Some(w) => format!("{} waits for you: {w}.", say(&id)), None => format!("{} is still {}: it starts by itself {}.", say(&id), waits_line(&left), once(left.len())) };
    format!("{now} To start it now anyway: herdr-wingman delegate {id} --now. To stop it waiting: herdr-wingman after {id} --clear")
}
/// What a task's wait comes to as its delegation is recorded (actions/start.rs), with `t` already queued: it waits
/// (stage "after", its line), or it goes on to a slot with what it waited for handed over.
pub fn on_delegate(t: &mut Task, now: bool) {
    for k in ["afterLeft", "afterBlocked", "afterDone", "afterEarly", "stoppedWaiting"] { t.del(k); }
    if after_of(t).is_empty() { t.del("after"); return; }
    let (left, broken) = standing(t);
    if now || left.is_empty() { handed::hand_over(t, now && !left.is_empty()); return; }
    t.put("stage", STAGE);
    t.put("msg", broken.clone().unwrap_or_else(|| waits_line(&left)));
    t.put("afterLeft", left);
    t.put("afterBlocked", broken.map(Value::from).unwrap_or(Value::Null));
}
