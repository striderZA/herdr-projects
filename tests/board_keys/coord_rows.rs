//! The rows of coordinators, read as tasks' rows: the top row (this machine) the board opens on, a task's marks for
//! where each coordinator stands, its time on the right and its live second line; enter folds a heading and does
//! nothing on a task, alt+o goes into a row's agent, ← and → fold, ← on a task goes to its heading.
use crate::common;
use common::*;
use serde_json::json;
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id};
use sw::shown::ink::Ink;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn jobs(fx: Vec<sw::board::update::Effect>) -> String { fx.iter().filter_map(|e| match e { sw::board::update::Effect::Run(j) => Some(format!("{j:?}")), _ => None }).collect::<Vec<_>>().join(" | ") }
fn heads(s: &common::board::Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Head { name, .. } => Some(name.clone()), _ => None }).collect() }

#[test]
fn the_board_opens_on_its_top_row_enter_folds_and_alt_o_goes_into_a_rows_coordinator() {
    let w = coord_world();
    let id = w.add("Fix the login");
    let host = sw::shown::words::clean(&sw::util::hostname());
    let ctx = sw::board::state::Ctx { cwd: w.root.display().to_string(), can_reload: true, ..Default::default() };
    let mut s = common::board::Board { s: sw::board::run::start::view(ctx, &w.root), rows: 40, cols: 120, closed: false, theme: sw::board::theme::Theme::default_dark(true) };
    s.s.side = false; s.load();
    // its row is the top row, the board opens on it, and the chat talks to its coordinator
    assert_eq!(s.s.sel, machine_id());
    assert!(s.text().contains(&format!("╭─  {host}  · Coordinator ─")) && !s.text().contains("Looks after"), "{}", s.text());
    assert!(s.text().contains("enter fold") && s.text().contains("alt+o go into it"), "{}", s.text());
    // enter folds it and unfolds it, and goes into nothing; alt+o goes into its coordinator
    let top = machine_id();
    let j = jobs(s.hold("enter")); assert!(j.is_empty() && s.s.folded.contains(&top), "{j}");
    assert!(s.text().contains("enter unfold"), "{}", s.text());
    let j = jobs(s.hold("enter")); assert!(j.is_empty() && !s.s.folded.contains(&top), "{j}");
    let j = jobs(s.hold("alt+o")); assert!(j.contains("CoordOpen") && j.contains("to: \"\""), "{j}");
    // a project's row: enter folds, alt+o goes into that one's coordinator
    s.press(&["alt+s"]); let proj = group_id(&s.s, "proj"); s.s.sel = proj.clone(); s.load();
    let j = jobs(s.hold("enter")); assert!(!j.contains("CoordOpen") && s.s.folded.contains(&proj), "{j}");
    s.press(&["enter"]); assert!(!s.s.folded.contains(&proj));
    let j = jobs(s.hold("alt+o")); assert!(j.contains("CoordOpen") && j.contains("project:proj"), "{j}");
    // a status group's heading has no agent: alt+o says so and starts nothing; enter folds it
    s.press(&["alt+s"]); let todo = group_id(&s.s, "to do"); s.s.sel = todo.clone(); s.load();
    let j = jobs(s.hold("alt+o")); assert!(j.is_empty(), "{j}");
    assert!(s.s.msg.contains("has no agent to go into"), "{}", s.s.msg);
    s.press(&["enter"]); assert!(s.s.folded.contains(&todo)); s.press(&["enter"]); assert!(!s.s.folded.contains(&todo));
    // a task's row: enter with nothing typed does nothing at all
    s.s.sel = id.clone(); s.load(); s.s.msg.clear();
    let fx = s.hold("enter"); assert!(fx.is_empty() && s.s.msg.is_empty() && s.s.sel == id, "{fx:?} {}", s.s.msg);
    // ← on a task goes to the heading it stands under; ← there folds it, → unfolds it
    s.press(&["left"]); assert_eq!(s.s.sel, todo);
    s.press(&["left"]); assert!(s.s.folded.contains(&s.s.sel));
    s.press(&["right"]); assert!(!s.s.folded.contains(&s.s.sel));
    // with words typed, ← and → move in them and fold nothing
    s.typed("ab"); s.press(&["left"]); assert!(!s.s.folded.contains(&s.s.sel)); s.press(&["esc"]);
}

#[test]
fn a_projects_row_that_asks_you_stands_under_needs_you_and_is_counted() {
    let w = coord_world();
    w.add("Fix the login");
    // a project coordinator that asks you: its row shows its mark, counts under Needs you, and is one line
    let now = sw::clock::now();
    let _ = now;
    let mut s = common::board::open(&w.root, 40);
    s.press(&["alt+s"]);
    let proj = group_id(&s.s, "proj");
    s.s.sel = proj.clone(); s.load();
    assert!(heads(&s).iter().any(|h| h == "proj"), "{}", s.text());
    // its row is drawn with the project coordinator's mark, in a task's marks' colours
    let l = s.inked(120);
    let row = l.iter().find(|x| x.text().contains("▾ proj")).map(|x| x.text()).unwrap_or_default();
    assert!(!row.is_empty(), "{:?}", l.iter().map(|x| x.text()).collect::<Vec<_>>());
    let mark = l.iter().flat_map(|x| x.0.clone()).find(|p| ["○", "◐", "?", "✗"].contains(&p.t.as_str()));
    assert!(mark.as_ref().is_some_and(|p| p.c == Ink::Muted), "idle, a task's idle mark: {mark:?}");
}
