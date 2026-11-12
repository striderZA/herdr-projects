//! Space on a machine's heading or a repo's heading (grouped by repo) talks to that machine's or that repo's
//! coordinator, as space on a task talks to its agent: the chat names it, what you type goes to it, and space again goes
//! back to the Coordinator.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::board::update::heads::group_id;
use sw::json::*;

fn coord_world(settings: Value) -> World {
    let mut all = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { all[k] = v; }
    let w = world_with(Opts { repo: true, settings: all, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_WINGMAN_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn prompts(w: &World, name: &str) -> Vec<String> { w.state().arr("panes").iter().find(|p| p.at("agent").s("name") == name).map(|p| p.at("agent").arr("prompts").iter().map(text).collect()).unwrap_or_default() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn space_on_a_repos_heading_talks_to_its_project_coordinator_and_space_again_goes_back() {
    let w = coord_world(json!({}));
    let id = w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj has one task open." }] }));
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let head = group_id(&s.s, "proj");
    s.s.sel = head.clone(); s.load();
    let host = sw::shown::words::clean(&sw::util::hostname());
    assert!(s.text().contains("space talk to proj's coordinator"), "{}", s.text());
    s.press(&["space"]);
    assert!(s.s.reply && s.s.reply_id == head);
    let t = s.text();
    // the box names it, from the top row down; it never ran: its box asks for a first message
    assert!(has(&t, &format!(r"\n╭─ {} ›  proj  · Project coordinator ─+╮\n│ +│\n│  ❯ Ask it something", regex::escape(&host))), "{t}");
    assert!(t.contains("❯ Ask it something, e.g. \"what's left to do here?\"") && t.contains(&format!("space back to {host}'s coordinator")), "{t}");
    s.typed("what is open here?");
    assert!(s.text().contains("enter send to") && s.text().contains("proj's coordinator"), "{}", s.text());
    s.press(&["enter"]); drain(); s.load();
    assert_eq!(s.s.value(), "");
    assert_eq!(prompts(&w, "sw-coordinator-proj"), vec!["what is open here?"]);
    assert!(sw::coordinator::state::get(&w.socket).is_none(), "the Coordinator got nothing");
    let t = s.text();
    // its reply alone under the ladder: no line naming it again, no label over the reply
    assert!(has(&t, r"╭─ [^\n]*›  proj  · Project coordinator ─") && !t.contains("Looks after") && !t.contains("│ proj's coordinator:") && t.contains("Proj has one task open."), "{t}");
    // the tab it is in says whose it is
    assert!(w.state().arr("tabs").iter().any(|x| x.s("label") == "proj coordinator"));
    // the chat follows the selection, as with tasks: to the task's agent and back to the repo's coordinator
    s.press(&["down"]); assert_eq!((s.s.sel.as_str(), s.s.reply_id.as_str()), (id.as_str(), id.as_str()));
    assert!(has(&s.text(), r"╭─ [^\n]* proj ›  PRJ-1  · Agent ─"), "{}", s.text());
    s.press(&["up"]); assert!(has(&s.text(), r"╭─ [^\n]*›  proj  · Project coordinator ─"), "{}", s.text());
    // space again: back to the top row's coordinator (the only machine's, the top-level one)
    s.press(&["space"]); assert!(!s.s.reply);
    assert!(s.text().contains(&format!("╭─  {host}  · Coordinator ─")), "{}", s.text());
    // a status group's heading has no coordinator: space says where they are
    s.press(&["alt+s"]); assert!(!s.s.by_repo); // by status
    s.s.sel = group_id(&s.s, "to do"); s.load();
    s.press(&["space"]);
    assert!(!s.s.reply && s.s.msg.contains("Space on the top row or a repo's heading talks to its coordinator"), "{}", s.s.msg);
}

#[test]
fn words_put_away_for_a_headings_coordinator_come_back_only_to_it() {
    let w = coord_world(json!({}));
    w.add("Fix the login");
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]);
    let head = group_id(&s.s, "proj");
    s.s.sel = head.clone(); s.load();
    s.press(&["space"]); s.typed("half a thought"); s.press(&["esc"]);
    assert_eq!(s.s.value(), ""); assert_eq!(s.s.draft_away.as_ref().map(|d| d.to.clone()), Some(head.clone()));
    s.press(&["space"]); assert!(!s.s.reply);
    s.press(&["ctrl+z"]);
    assert_eq!(s.s.value(), "", "not to the Coordinator"); assert!(s.s.msg.contains("Those words were for") && s.s.msg.contains("proj's coordinator. Select its row and press space"), "{}", s.s.msg);
    s.press(&["space"]); s.press(&["ctrl+z"]);
    assert_eq!(s.s.value(), "half a thought");
}
