//! What you send shows in the chat's box, with no label, the moment you press enter, whoever the chat talks to (a task's
//! agent, a heading's coordinator, the Coordinator), with the line over the box saying it is on its way; nothing of it
//! stands over the chat meanwhile, and words that did not get there go back into the text box.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::board::job::{Done, Job};
use sw::board::update::heads::group_id;
use sw::board::update::{update, Effect, Event};

fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
fn coord_world(settings: Value) -> World {
    let mut all = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { all[k] = v; }
    let w = world_with(Opts { repo: true, settings: all, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }

#[test]
fn a_task_agents_chat_shows_your_words_the_moment_you_send_them_and_takes_them_back_when_they_did_not_get_there() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    s.typed("yes, g too");
    let fx = s.hold("enter");
    assert!(fx.iter().any(|e| matches!(e, Effect::Run(Job::Tell { .. }))), "{fx:?}");
    // the job has not even started: the box already has your words, and the line over it says they are on their way
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed, v(&["yes, g too", "", "❯ Send message to the agent"]), "{}", c.lines.join("\n"));
    assert!(c.who.ends_with("› PRJ-1"), "the box says whose it is: {}", c.who);
    assert!(!c.lines[..c.lines.len() - 6].iter().any(|l| l.contains("yes, g too")), "nothing of it over the chat: {}", c.lines.join("\n"));
    // delivered: the same box
    s.run(fx);
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed, v(&["yes, g too", "", "❯ Send message to the agent"]));
    // words that did not get there are not yours in the chat: they go back into the text box
    s.typed("and h");
    let _ = s.hold("enter");
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed[..1].to_vec(), v(&["and h"]));
    let _ = update(&mut s.s, Event::Done(Done { about: id.clone(), kind: "Tell".into(), ok: false, then: vec![sw::board::job::Then::Unsent { text: "and h".into(), to: id.clone() }], ..Done::fail("its agent is gone") }));
    assert!(!s.s.just_sent.contains_key(&id) && s.s.value() == "and h", "{:?}", s.s.just_sent);
}

#[test]
fn a_repos_coordinator_here_shows_your_words_at_once_until_its_own_conversation_has_them() {
    let w = coord_world(json!({}));
    w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj has one task open." }] }));
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]);
    s.s.sel = group_id(&s.s, "proj"); s.load();
    s.press(&["space"]);
    s.typed("what is open here?");
    let fx = s.hold("enter");
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed[..2].to_vec(), v(&["what is open here?", ""]), "{}", c.lines.join("\n"));
    assert!(c.who.ends_with("› proj"), "{}", c.who);
    s.run(fx); drain(); s.load();
    let t = s.text();
    // its answer alone, under no label: the box's edge names it
    assert!(!t.contains("│ proj's coordinator:") && t.contains("  Proj has one task open.") && !t.contains("sending…"), "{t}");
    assert!(s.s.just_sent.is_empty(), "its conversation has your words now: {:?}", s.s.just_sent);
}
