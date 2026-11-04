//! (herdr status, stage) -> what the board shows. The one mapping function. The board draws five states: idle,
//! working, blocked, done, error; blocked carries a kind. herdr has no `error` state and no blocked `kind` today:
//! when it exposes them (OSC 7501), this is the only place to change.
use crate::attempts::words::{attempts_shown, pick_line};
use crate::check_progress::check_shown;
use crate::json::*;
use crate::limits::shown::limits_of;
use crate::status_kept::{quiet_line, quiet_plain};
use crate::store::files::RUNNER_STAGES;
use crate::store::retries::{wait_line, waiting_of};

pub const STATES: [&str; 5] = ["idle", "working", "blocked", "done", "error"];
fn stage_line(stage: &str) -> &'static str {
    match stage { "starting" => "Starting", "queued" => "Waiting for a free slot", "setup" => "Setting up", "checking" => "Running the check", "pushing" => "Pushing the branch", "reviewing" => "Under review", "merging" => "Merging", _ => "" }
}
/// What the board shows of a task: its state and the words of its row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Show {
    pub state: &'static str, pub kind: Option<String>, pub msg: String, pub app: String, pub progress: Option<f64>, pub eta: Option<i64>,
    pub plain: bool, pub review: bool, pub stale: bool, pub exited: bool, pub reviewing: bool, pub plan: bool, pub limits: Option<Value>,
}
/// With no live reading, show what was last seen rather than guessing.
fn last_known(t: &Obj) -> &'static str { if t.at("last").s("status") == "blocked" { "blocked" } else { "working" } }
/// Has the agent worked on what it was last asked? herdr saw it work since the turn was armed, or a turn herdr counts
/// as finished, and the finish line has not handled, ended after the turn was armed.
fn worked(t: &Obj, live: &Value) -> bool {
    if t.b("pendingText") { return false; }
    let armed = t.i("armedAt");
    let comp = live.at("agent").at("completion_seq");
    if t.i("workedAt") >= armed.max(1) { return true; }
    let last_at = if t.at("last").has("compAt") { t.at("last").i("compAt") } else { t.at("last").i("at") };
    !comp.is_null() && comp != t.at("handled").at("comp") && t.at("last").at("comp") == comp && last_at > armed
}
/// `live` describes the task's worker pane right now: { status } (herdr's agent status), { status: null } (no agent
/// in it), { gone }, { away: session }, { down }, or null (not looked up: a task with no agent).
pub fn show(t: &Obj, live: &Value, now: i64) -> Show {
    let limits = limits_of(t, now);
    let base = Show { state: "", app: if t.b("app") { t.t("app") } else { String::new() }, msg: t.t("msg"), limits: limits.clone(), ..Default::default() };
    let out = |state: &'static str, f: &dyn Fn(&mut Show)| { let mut s = base.clone(); s.state = state; f(&mut s); s };
    let agent = t.b("app") && t.s("app") != t.s("owner");
    // A turn cut off by a passing provider error, and the board waiting to try again by itself: work going on.
    if let Some(w) = waiting_of(t) { let msg = wait_line(w, now); let rv = w.s("who") == "reviewer"; return out("working", &|s| { s.msg = msg.clone(); s.reviewing = rv; }); }
    match t.s("stage") {
        // nobody started it; what it is to wait for (after.rs) is not said on its row: only once it is queued
        "todo" => return out("idle", &|_| {}),
        // delegated, and queued after other tasks (after.rs): its line says after which, or why it waits for you
        "after" => return if t.b("afterBlocked") { out("blocked", &|s| s.kind = Some("question".into())) } else { out("working", &|_| {}) },
        // your own task: the progress and the minutes left you reported
        "progress" => return out("working", &|s| { s.app = t.t("owner"); s.progress = t.at("watch").at("progress").is_number().then(|| t.at("watch").f("progress")); s.eta = t.at("watch").at("eta").is_number().then(|| t.at("watch").i("eta")); }),
        "done" => return out("done", &|_| {}),
        "failed" => return if t.at("failure").b("auth") { out("blocked", &|s| s.kind = Some("auth".into())) } else { out("error", &|_| {}) },
        "approval" => return out("blocked", &|s| { s.kind = Some("permission".into()); s.review = !t.b("mergeFailed") && !t.b("syncBlocked"); }),
        "asks" => return out("blocked", &|s| s.kind = Some(if t.b("kind") { t.t("kind") } else { "question".into() })),
        "plan" => return out("blocked", &|s| { s.kind = Some("permission".into()); s.plan = true; }),
        "nosummary" => return out("error", &|s| if s.msg.is_empty() { s.msg = "Stopped without a summary".into(); }),
        "reviewneeds" => return out("blocked", &|s| if s.msg.is_empty() { s.msg = "The reviewer needs you".into(); }),
        "stopped" => return if t.b("limitStop") { out("blocked", &|s| if s.msg.is_empty() { s.msg = "Stopped at a limit".into(); }) } else { out("idle", &|s| s.msg = "Stopped".into()) },
        "attempts" => {
            let kids: Vec<Obj> = t.arr("tries").iter().filter_map(|c| c.as_object().cloned()).collect();
            let a = attempts_shown(&kids, now);
            let st = a.state;
            return out(st, &|s| { s.kind = a.kind.map(String::from); s.msg = a.msg.clone(); s.progress = a.progress.map(|p| p as f64); s.eta = a.eta; });
        }
        "pick" => { let kids: Vec<Obj> = t.arr("tries").iter().filter_map(|c| c.as_object().cloned()).collect(); let line = pick_line(t, Some(&kids)); return out("blocked", &|s| { s.kind = Some("permission".into()); s.msg = line.clone(); }); }
        "tried" => return out("done", &|s| if s.msg.is_empty() { s.msg = "Passed".into(); }),
        "queued" | "starting" => { let line = stage_line(t.s("stage")); return out("working", &|s| if s.msg.is_empty() { s.msg = line.into(); }); }
        _ => {}
    }
    // Under review: the reviewer's own line, or the board's.
    let own_line = t.b("msg") && ![stage_line("reviewing"), "under review"].contains(&t.s("msg")) && !t.b("comparing");
    if t.s("stage") == "reviewing" && t.b("comparing") {
        return out("working", &|s| { s.msg = "The reviewer compares the attempts".into(); s.reviewing = true; });
    }
    if t.s("stage") == "reviewing" {
        let msg = if own_line { t.t("msg") } else { quiet_line(t, "reviewer") };
        let plain = !own_line && quiet_plain(t, "reviewer");
        return out("working", &|s| { s.msg = msg.clone(); s.plain = plain; s.reviewing = true; });
    }
    // The board's own check: the count its command reports and the minutes left.
    if t.s("stage") == "checking" && t.b("check") { let c = check_shown(t.at("check"), now); return out("working", &|s| { s.msg = c.msg.clone(); s.progress = c.progress.map(|p| p as f64); s.eta = c.eta; }); }
    if RUNNER_STAGES.contains(&t.s("stage")) { let line = stage_line(t.s("stage")); return out("working", &|s| { if s.msg.is_empty() { s.msg = line.into(); } }); }
    // A chat changed the code, and the board waits for you to be done before it reviews it (flow/chat.rs).
    if crate::flow::chat::held(t) && live.at("status").as_str().map(|s| s == "idle" || s == "done").unwrap_or(live.is_null()) { return out("working", &|s| s.msg = crate::flow::chat::HELD.into()); }
    // stage "working": the agent's own turn. herdr decides.
    if !agent || live.is_null() { return out("working", &|_| {}); }
    if live.b("down") { return out(last_known(t), &|s| { s.msg = "The herdr server is not running".into(); s.stale = true; }); }
    if live.b("away") { let away = live.t("away"); return out(last_known(t), &|s| { s.msg = format!("In session {away}"); s.stale = true; }); }
    if live.b("gone") { return out("error", &|s| s.msg = "The agent's terminal is gone".into()); }
    let status = live.at("status");
    if status.is_null() { return out("error", &|s| { s.msg = "The agent has exited".into(); s.exited = true; }); }
    match status.as_str().unwrap_or("") {
        "blocked" => { let pt = t.b("pendingText"); out("blocked", &|s| { s.kind = pt.then(|| "permission".into()); s.msg = if pt { "Queued: open the agent and answer its dialog first".into() } else { "Needs you".into() }; s.plain = !pt; }) }
        "working" => {
            let msg = if t.b("msg") { t.t("msg") } else { quiet_line(t, "agent") };
            let plain = !t.b("msg") && quiet_plain(t, "agent");
            out("working", &|s| { s.msg = msg.clone(); s.plain = plain; })
        }
        "unknown" => out("working", &|s| s.msg = "Cannot tell from herdr if it stopped".into()),
        // Idle or done. Text typed in a while ago that it has still not started on: say that.
        _ if t.at("sent").b("tries") => out("working", &|s| s.msg = "Has not started yet: pressing enter again".into()),
        _ if worked(t, live) => out("working", &|s| s.msg = "Finishing".into()),
        _ => { let w = if t.i("workedAt") >= t.i("startedAgentAt").max(1) { "About to continue" } else { "Starting" }; out("working", &|s| s.msg = w.into()) }
    }
}
