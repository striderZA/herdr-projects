//! Where an automation stands in the list: as its tasks do. By status, the automations are a group of their own after
//! the tasks' groups, folding as they do; by repo, each stands under its repo's heading after that repo's tasks. A
//! folded heading says what it holds (`1 task, 1 automation`).
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::parse_schedule;
use sw::board::update::heads::{group_id, machine_id};
use sw::json::*;

fn t(s: &mut Board, width: usize) -> String { s.screen(width).join("\n") }
fn like(text: &str, re: &str) { assert!(regex::Regex::new(re).unwrap().is_match(text), "{re}\n{text}"); }

#[test]
fn automations_stand_in_their_own_group_or_under_their_repo_and_fold_with_it() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    sw::clock::set(Some(sw::clock::now()));
    w.add("Mine");
    let mine = l::add(Add { name: "Look for old dependencies", prompt: "Check.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    let row = format!("auto:{}", mine.s("id"));
    let mut s = open(&w.root, 44); s.s.side = false;
    let here = sw::board::update::heads::label(&s.s);
    // By status: the tasks' groups, then the automations' own group.
    let text = t(&mut s, 120);
    like(&text, &format!(r"\n ▾ {}.*\n   ▾ to do\n[ ▌]  ○ PRJ-1 Mine +0s ago\n   ▾ automations\n   ⟳ on {{4}}Look for old dependencies {{2}}every 2h +next in {{2}}2h\n", regex::escape(&here)));
    // The arrows go through them where they stand: the task, its automation.
    s.s.sel = "proj/1".into(); s.load();
    s.press(&["down"]); assert_eq!(s.s.sel, group_id(&s.s, "automations"));
    s.press(&["down"]); assert_eq!(s.s.sel, row);
    // Its group folded: one line that says how many; space on the row in its group still acts once it is open again.
    let g = group_id(&s.s, "automations");
    s.s.sel = g.clone(); s.load(); s.press(&["left"]);
    let text = t(&mut s, 120);
    like(&text, r"\n▌  ▸ automations {2}1 automation\n"); assert!(!text.contains("Look for old"), "{text}");
    s.press(&["right", "down", "space"]); assert_eq!(s.s.sel, row);
    assert_eq!(sw::store::automations::automation(mine.s("id")).unwrap().at("on"), &json!(false), "space on its row turns it off");
    s.press(&["space"]);
    // The top row folded: its one line counts the tasks and the automations it hides.
    s.s.sel = machine_id(); s.load(); s.press(&["left"]);
    like(&t(&mut s, 120), &format!(r"\n▌▸ {} {{2}}1 task, 1 automation\n", regex::escape(&here)));
    s.press(&["right"]);
    // Taken to an automation in a folded group (a jump, a firing's link): its group unfolds.
    s.s.sel = g.clone(); s.load(); s.press(&["left"]); s.s.sel = row.clone(); s.load();
    assert!(t(&mut s, 120).contains("Look for old dependencies"));
    // By repo: under its repo's heading, after that repo's tasks; no heading of its own.
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let text = t(&mut s, 120);
    like(&text, r"\n   ▾ proj · PRJ\n[ ▌]    ○ PRJ-1 Mine +0s ago\n[ ▌]    ⟳ on {4}Look for old dependencies {2}every 2h +next in {2}2h\n");
    assert!(!text.contains(" automations\n"), "{text}");
    // A repo folded says what it holds too.
    s.s.sel = group_id(&s.s, "proj"); s.load(); s.press(&["left"]);
    like(&t(&mut s, 120), r"\n▌  ▸ proj · PRJ {2}1 task, 1 automation\n");
}
