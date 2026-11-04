//! Starting the same program again on its own default model, once: the model the board gave it was refused at a turn's
//! end (docs/models.md). The task, the worktree and what was done so far stay; the agent is ended and started again on
//! the model its program has of its own. Not a fallback: no other program is tried.
use crate::clock::now;
use crate::delegate::{first_prompt, start_agent};
use crate::err::Res;
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::runner::{is_aborted, Guard};
use crate::store::repos::{cwd_of, git_out};
use crate::store::settings::values::agent_of;
use crate::store::tasks::{one_line, Task};

/// What the one before it left: its commits on the branch, and whether changes lie uncommitted.
fn done_so_far(t: &Task) -> String {
    if !t.b("worktree") { return String::new(); }
    let tree = cwd_of(t);
    let base = if t.b("baseSha") { t.t("baseSha") } else if t.b("from") { t.t("from") } else { "HEAD".into() };
    let commits: Vec<String> = git_out(&tree, &["log", "--oneline", "--no-decorate", "-20", &format!("{base}..HEAD")]).unwrap_or_default().lines().filter(|l| !l.is_empty()).map(String::from).collect();
    let loose = flow::git::dirty(&tree).lines().filter(|l| !l.is_empty()).count();
    [if commits.is_empty() { "It committed nothing on this branch yet.".to_string() } else { format!("Its commits on this branch so far, newest first:\n{}", commits.iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n")) },
        if loose > 0 { format!("{loose} file{} changes that are not committed yet.", if loose == 1 { " has" } else { "s have" }) } else { String::new() }]
        .into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}
/// The findings that are still open: the last review's, when it failed the gate and the work has not passed one since.
fn open_findings(t: &Task) -> String {
    let rv = t.at("review");
    if rv.is_object() && !gate::passes(rv) && gate::sendable(rv) { format!("The last review of this work failed ({}), and what it found is still open:\n\n{}", gate::result(rv, "merge"), gate::findings_text(rv, false)) } else { String::new() }
}
/// All the agent is given: the task as a first agent gets it, then that it starts again, from where it stood, what was
/// done, and what is open.
fn prompt(t: &Task, from: &str, why: &str, findings: &str) -> String {
    let place = if t.b("worktree") { format!("in the same folder, on the same branch ({})", t.s("branch")) } else { "in the same folder".into() };
    [first_prompt(t),
        format!("You are starting again on the same task after {from} could not go on: {why}. You work {place}: everything done so far is there. Look at it first{} and carry on from where it stands. Do not start over, and do not undo its work without a reason.", if t.b("worktree") { " (git log, git status, git diff)" } else { "" }),
        done_so_far(t),
        if t.b("said") { format!("Its last message was:\n{}", t.s("said")) } else { String::new() },
        if findings.is_empty() { open_findings(t) } else { findings.to_string() },
        "Finish the task as its rules say.".into(),
    ].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")
}
/// What a restart changes on the task, put back whole when the program cannot be started.
const KEPT: [&str; 20] = ["agent", "app", "on", "session", "transcript", "handled", "agentVersion", "hooked", "model", "usage", "failure", "waiting", "cutRetry", "checkFails", "reviewRounds", "kind", "watch", "review", "mergeIntent", "requireApproval"];
/// Start `kind` again on the task, on its program's own default model. True when it is on it; false when it could not
/// be started, with the task as it was.
pub fn restart_on(id: &str, g: &mut Guard, why: &str, findings: &str, kind: &str) -> Res<bool> {
    let mut t = g.check()?;
    if !t.at("link").b("pane") { return Ok(false); }
    let api = flow::task::h(&t);
    let from = if t.b("on") { t.t("on") } else { t.t("agent") };
    let pane = t.at("link").t("pane");
    if api.pane_get(&pane).is_none() { return Ok(false); }
    if api.agent_get(&pane).ok().flatten().is_some() && !api.pane_quit(&pane, 6000) { return Ok(false); }
    let before: Obj = KEPT.iter().map(|k| (k.to_string(), t.at(k).clone())).collect();
    t = g.set(|t| {
        t.put("on", kind); t.put("agent", kind); t.put("app", kind);
        t.put("session", Value::Null); t.put("transcript", Value::Null);
        let h = t.obj_mut("handled"); h.put("msg", Value::Null); h.put("pos", 0);
        t.put("model", Value::Null); t.put("usage", Value::Null);
        // The work goes on under a new model: what the one before used up is not counted against this one.
        for x in ["failure", "waiting", "cutRetry", "watch"] { t.put(x, Value::Null); }
        t.put("checkFails", 0); t.put("reviewRounds", 0); t.del("kind");
        if t.at("review").is_object() && !t.at("review").b("stale") { t.obj_mut("review").put("stale", true); }
        if t.at("mergeIntent").is_object() { if !t.at("mergeIntent").b("auto") { t.put("requireApproval", true); } t.put("mergeIntent", Value::Null); }
    })?;
    let shown_from = agent_of(&from).kind;
    if let Err(e) = start_agent(id, g, &prompt(&t, &shown_from, why, findings), None) {
        if is_aborted(&e) { return Err(e); }
        g.set(|t| { for (k, v) in before { t.insert(k, v); } })?;
        return Ok(false);
    }
    t = g.set(|t| { t.put("msg", format!("{}: started again on {}'s own default model", one_line(why, 200), kind)); t.put("since", now()); t.put("workAt", now()); })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(true)
}
