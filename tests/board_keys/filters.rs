//! The filter row on top: All · Coordinators · Needs you · Working · Ready · To do · Done · Automations · Pinned,
//! each with its count, the one shown lit in its own colour. Tab and shift+tab go round it, a click picks one, the list
//! shows only that kind of row; the top row and a project's row stand for their coordinators (space talks to one), and
//! Coordinators leaves only those. Alt+S groups by status or repo. ← → fold every heading.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Event};
use sw::json::*;
use sw::shown::ink::Ink;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn tasks_shown(s: &common::board::Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Rec(o) => Some(o.t("title")), _ => None }).collect() }
fn heads(s: &common::board::Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Head { name, .. } => Some(name.clone()), _ => None }).collect() }
/// The filter row's piece that says `name`, with its ink and whether it is reversed.
fn piece(s: &mut common::board::Board, name: &str) -> (Ink, bool) {
    let l = s.inked(140);
    let p = l[1].0.iter().find(|p| p.t.contains(name)).unwrap_or_else(|| panic!("{name}: {:?}", l[1].text()));
    (p.c, p.inv)
}

#[test]
fn tab_goes_round_the_filters_lit_in_their_own_colours_and_the_list_shows_only_that_kind() {
    let w = coord_world();
    let asks = w.add("Asks you"); sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), msg: "which one?".into(), ..Default::default() }).unwrap();
    w.add("Plain to do");
    for n in 0..7 { let id = w.add(&format!("Done {n}")); sw::store::tasks::update(&id, |t| { t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap(); }
    let mut s = open(&w.root, 40); s.s.side = false;
    let l = s.screen(120);
    assert_eq!(l[0], " herdr-projects  proj · PRJ");
    assert!(regex::Regex::new(r"^ +All 9 +Coordinators 0   Needs you 1   Working 0   Ready 0   To do 1   Done 7   Automations 0   Pinned 0$").unwrap().is_match(&l[1]), "{l:?}");
    assert!(!l.iter().any(|x| x.starts_with(" ") && x.contains("──")), "no heading has a rule: {l:?}");
    // All is lit in the plain text colour, reversed
    assert_eq!(piece(&mut s, "All 9"), (Ink::Text, true));
    // each item keeps a cell of room on each side, lit or not: lighting another moves nothing
    s.lines(); let before = s.s.show_hits.clone();
    s.press(&["tab"]); s.lines(); assert_eq!(s.s.show_hits, before, "the row does not move");
    assert_eq!(piece(&mut s, "Coordinators 0"), (Ink::Accent, true));
    assert!(s.inked(140)[1].0.iter().any(|p| p.inv && p.t == " Coordinators 0 "), "a cell of highlight on each side");
    s.press(&["shift+tab"]);
    // tab: Coordinators, lit in the accent colour; then needs you only, in red
    s.press(&["tab"]); assert_eq!(s.s.show, 1); assert_eq!(piece(&mut s, "Coordinators 0"), (Ink::Accent, true));
    assert!(tasks_shown(&s).is_empty());
    s.press(&["tab"]); assert_eq!(s.s.show, 2); assert_eq!(piece(&mut s, "Needs you 1"), (Ink::Red, true));
    assert_eq!(tasks_shown(&s), vec!["Asks you"]);
    s.press(&["tab"]); assert_eq!(piece(&mut s, "Working 0"), (Ink::Yellow, true));
    s.press(&["tab"]); assert_eq!(piece(&mut s, "Ready 0"), (Ink::Green, true));
    s.press(&["tab"]); assert_eq!(s.s.show, 5); assert_eq!(tasks_shown(&s), vec!["Plain to do"]);
    // done: every done task, not the latest five
    s.press(&["tab"]); assert_eq!(tasks_shown(&s).len(), 7, "{:?}", tasks_shown(&s));
    s.press(&["tab"]); assert_eq!(s.s.show, 7); assert_eq!(piece(&mut s, "Automations 0"), (Ink::Auto, true));
    assert!(tasks_shown(&s).is_empty());
    // Pinned, the last: nothing pinned here
    s.press(&["tab"]); assert_eq!(s.s.show, 8); assert_eq!(piece(&mut s, "Pinned 0"), (Ink::Text, true));
    assert!(tasks_shown(&s).is_empty());
    // shift+tab: back; round again
    s.press(&["shift+tab"]); assert_eq!(s.s.show, 7);
    s.press(&["tab", "tab"]); assert_eq!(s.s.show, 0);
    // a click on a filter picks it
    s.lines();
    let (y, x, _, _) = *s.s.show_hits.iter().find(|h| h.3 == 2).unwrap();
    let fx = update(&mut s.s, Event::Click { x: x as u16, y: y as u16 }); s.run(fx);
    assert_eq!(s.s.show, 2);
    // tab never puts an agent on a task any more
    assert!(sw::store::tasks::list(Default::default()).iter().all(|t| t.s("stage") != "progress"));
    let view: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(sw::store::files::data_dir().join("view.json")).unwrap()).unwrap();
    assert_eq!(view.at("show"), &json!(2));
    // narrow: shorter names, the empty ones gone (no coordinator runs), then only the one shown
    assert!(s.screen(90)[1].contains("Coords 0 ") && s.screen(76)[1].contains("Needs 1") && !s.screen(76)[1].contains("Coords"), "{:?}", s.screen(76));
    assert_eq!(s.screen(30)[1].trim(), "Needs you 1  tab next");
}

#[test]
fn space_on_the_top_row_and_a_projects_row_talks_to_its_coordinator_and_coordinators_leaves_only_those() {
    let w = coord_world();
    w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj is quiet." }] }));
    let host = sw::shown::words::clean(&sw::util::hostname());
    let mut s = open(&w.root, 40); s.s.side = false;
    // by repo: the project's row (its heading) is its coordinator's: space talks to it
    s.press(&["alt+s"]);
    s.s.sel = group_id(&s.s, "proj"); s.load();
    s.press(&["space"]);
    assert!(s.s.reply && regex::Regex::new(r"╭─ [^\n]*›  proj  · Project coordinator ─").unwrap().is_match(&s.text()), "{}", s.text());
    s.typed("hello"); s.press(&["enter"]); drain(); s.load();
    let t = s.text();
    // one line: idle, its coordinator has a task's idle mark and when it last spoke; what it said is the chat's
    assert!(regex::Regex::new(r"\n[ ▌]  ○ ▾ proj · PRJ +(\d+s ago)? *\n[ ▌] +○ PRJ-1").unwrap().is_match(&t) && !t.contains("\n       Proj is quiet.") && !t.contains("◈"), "{t}");
    s.press(&["space"]); assert!(!s.s.reply);
    // the top row: space talks to this machine's coordinator
    s.s.sel = machine_id(); s.load();
    s.press(&["space"]);
    assert!(s.s.reply && s.text().contains("· Coordinator ─"), "{}", s.text());
    s.press(&["space"]);
    // Coordinators: the top row and the project's row; no task
    s.press(&["tab"]); assert_eq!(s.s.show, 1);
    assert_eq!(heads(&s), vec![host.clone(), "proj".into()]);
    assert!(tasks_shown(&s).is_empty());
    // this machine's coordinator never started: a row that heads the others, not counted
    assert!(regex::Regex::new(r"Coordinators 1 ").unwrap().is_match(&s.screen(160)[1]), "{:?}", s.screen(160));
    // alt+o on the project's row goes into its coordinator
    s.s.sel = format!("head:repo\tproj"); s.load();
    assert!(s.text().contains("alt+o go into it"), "{}", s.text());
    let j = format!("{:?}", s.hold("alt+o")); assert!(j.contains("CoordOpen") && j.contains("project:proj"), "{j}");
    // ← on the top row folds its projects' rows away, → brings them back
    s.s.sel = machine_id(); s.load();
    s.press(&["left"]); assert!(s.s.folded.contains(&machine_id()) && !heads(&s).contains(&"proj".to_string()), "{}", s.text());
    // folded away, its project's coordinator still counts: the count is of coordinators, not of the rows that show
    assert!(regex::Regex::new(r"Coordinators 1 ").unwrap().is_match(&s.screen(160)[1]), "{:?}", s.screen(160));
    s.press(&["right"]); assert!(heads(&s).contains(&"proj".to_string()));
}

#[test]
fn automations_is_a_filter_with_only_the_automations_rows_and_its_count() {
    use sw::automations::list::{self as l, Add};
    let w = coord_world();
    w.add("A task");
    l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(sw::automations::triggers::parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: Some(true) }).unwrap();
    let mut s = open(&w.root, 40); s.s.side = false;
    assert!(s.screen(140)[1].ends_with("Automations 1   Pinned 0"), "{:?}", s.screen(140));
    s.press(&["shift+tab", "shift+tab"]); assert_eq!(s.s.show, 7);
    assert!(tasks_shown(&s).is_empty() && s.s.rows.iter().any(|r| matches!(r, Row::Auto(_))), "{}", s.text());
    assert!(s.text().contains("Nightly audit") && !s.text().contains("A task"), "{}", s.text());
}

#[test]
fn alt_s_groups_by_status_by_repo_and_round_again() {
    let w = coord_world();
    w.add("One"); w.add("Two");
    let host = sw::shown::words::clean(&sw::util::hostname());
    let mut s = open(&w.root, 40); s.s.side = false;
    // the top row in every grouping
    assert_eq!(heads(&s), vec![host.clone(), "to do".into()]);
    s.press(&["alt+s"]); assert!(s.s.by_repo); assert_eq!(heads(&s), vec![host.clone(), "proj".into()]);
    s.press(&["alt+s"]); assert!(!s.s.by_repo); assert_eq!(heads(&s), vec![host.clone(), "to do".into()]);
    assert_eq!(tasks_shown(&s).len(), 2);
}

#[test]
fn the_menus_start_line_puts_an_agent_on_the_task() {
    let w = coord_world();
    let id = w.add("Start me from the menu");
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let mut s = open(&w.root, 60); s.s.side = false;
    s.s.sel = id.clone(); s.load();
    s.press(&["?"]);
    let at = sw::board::update::menu::items(&s.s).iter().position(|i| i.id == sw::board::keys::K::Delegate).expect("a start line");
    for _ in 0..=at { s.press(&["down"]); }
    s.press(&["enter"]); settle(&id); s.load();
    assert_ne!(task(&id).s("stage"), "todo", "{:?}", task(&id));
}

/// ← on a heading (and enter, and a click on it) folds it to one line that says what it holds, → unfolds it.
fn folds(s: &mut common::board::Board, id: &str, holds: &str) {
    s.s.sel = id.to_string(); s.load();
    let name = match s.s.rows.iter().find(|r| matches!(r, Row::Head { id: h, .. } if h == id)) { Some(Row::Head { name, .. }) => sw::shown::words::repo_label(name), _ => panic!("no heading {id}: {}", s.text()) };
    let fx = s.hold("enter"); assert!(s.s.folded.contains(id) && !format!("{fx:?}").contains("CoordOpen"), "enter folds {id}, and goes into nothing");
    s.press(&["enter"]); assert!(!s.s.folded.contains(id), "enter again unfolds {id}");
    s.press(&["right"]); assert!(!s.s.folded.contains(id), "→ on an open heading leaves it open");
    s.press(&["left"]);
    assert!(s.s.folded.contains(id) && regex::Regex::new(&format!(r"\n[ ▌]\s*(\S )?▸ {}( · \w+)?  {holds}", regex::escape(&name))).unwrap().is_match(&s.text()), "{id} folds to {holds}: {}", s.text());
    s.press(&["left"]); assert!(s.s.folded.contains(id), "← again leaves it folded");
    s.press(&["right"]);
    assert!(!s.s.folded.contains(id), "{id} unfolds");
    // a click does the same
    s.lines();
    let y = s.s.list_at.unwrap().0 + s.s.list_ids.iter().position(|x| x == id).unwrap();
    let fx = update(&mut s.s, Event::Click { x: 3, y: y as u16 }); s.run(fx);
    assert!(s.s.folded.contains(id), "a click folds {id}");
    let fx = update(&mut s.s, Event::Click { x: 3, y: y as u16 }); s.run(fx);
    assert!(!s.s.folded.contains(id), "a click unfolds {id}");
}

#[test]
fn left_and_right_fold_every_kind_of_heading_under_every_filter_and_grouping_and_space_talks() {
    let w = coord_world();
    w.add("Here one");
    let asks = w.add("Asks you"); sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), msg: "which one?".into(), ..Default::default() }).unwrap();
    w.script(json!({ "coordinator": [{ "said": "Proj is quiet." }] }));
    let mut s = open(&w.root, 60); s.s.side = false;
    // this machine, by status: a status group's heading
    { let g = group_id(&s.s, "to do"); folds(&mut s, &g, "1 task"); }
    // the filter Needs you: its group's heading, and no empty heading of anything else
    s.press(&["tab", "tab"]); assert_eq!(s.s.show, 2);
    { let g = group_id(&s.s, "needs you"); folds(&mut s, &g, "1 task"); }
    s.press(&["shift+tab", "shift+tab"]); assert_eq!(s.s.show, 0);
    // by repo: a repo's row, which is its project coordinator's (space talks to it)
    s.press(&["alt+s"]);
    { let g = group_id(&s.s, "proj"); folds(&mut s, &g, "2 tasks"); }
    s.s.sel = group_id(&s.s, "proj"); s.load();
    s.press(&["space"]); assert!(regex::Regex::new(r"╭─ [^\n]*›  proj  · Project coordinator ─").unwrap().is_match(&s.text()), "{}", s.text());
    s.typed("hello"); s.press(&["enter"]); drain(); s.press(&["space"]); s.load();
    { let g = group_id(&s.s, "proj"); folds(&mut s, &g, "2 tasks"); }
    // the top row says what it holds open or folded
    s.press(&["alt+s"]);
    folds(&mut s, &machine_id(), "2 tasks");
    // Coordinators: the top row folds its projects' rows
    s.press(&["tab"]); assert_eq!(s.s.show, 1);
    folds(&mut s, &machine_id(), "1 project coordinator");
}

#[test]
fn ctrl_s_starts_a_coordinator_ctrl_x_stops_it_and_ctrl_o_is_its_autopilot() {
    let w = coord_world();
    w.add("Fix the login");
    let mut s = open(&w.root, 40); s.s.side = false;
    let jobs = |fx: Vec<sw::board::update::Effect>| fx.iter().filter_map(|e| match e { sw::board::update::Effect::Run(j) => Some(format!("{j:?}")), _ => None }).collect::<Vec<_>>().join(" | ");
    s.press(&["alt+s"]);
    s.s.sel = group_id(&s.s, "proj"); s.load();
    assert!(s.text().contains("ctrl+s start · ctrl+x stop · ctrl+o autopilot"), "{}", s.text());
    // (a held job never answers: what it put under way on the row is let go before the next key)
    let j = jobs(s.hold("ctrl+s")); assert!(j.contains("CoordRun") && j.contains("project:proj") && j.contains("on: true"), "{j}"); s.s.pending.clear();
    let j = jobs(s.hold("ctrl+x")); assert!(j.contains("CoordRun") && j.contains("on: false"), "{j}"); s.s.pending.clear();
    let j = jobs(s.hold("ctrl+o")); assert!(j.contains("Pilot") && j.contains("project:proj") && j.contains("on: true"), "{j}"); s.s.pending.clear();
    // the top row's coordinator, on its row and on a status group's heading
    s.s.sel = machine_id(); s.load();
    let j = jobs(s.hold("ctrl+s")); assert!(j.contains("CoordRun") && j.contains("to: \"\""), "{j}"); s.s.pending.clear();
    s.press(&["alt+s"]); s.s.sel = group_id(&s.s, "to do"); s.load();
    let j = jobs(s.hold("ctrl+s")); assert!(j.contains("CoordRun") && j.contains("to: \"\""), "{j}");
    s.press(&["alt+s"]);
    // on a task, ctrl+o says what it is for
    s.s.sel = "proj/1".into(); s.load();
    s.press(&["ctrl+o"]); assert!(s.s.msg.contains("turns a coordinator's autopilot on and off"), "{}", s.s.msg);
}
