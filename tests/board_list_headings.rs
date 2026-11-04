//! The list's headings: this machine's row on top and its repos (or its status groups) under it, rows that never name
//! their repo; the arrows stop on every heading, ← folds it to one line that says how many tasks it holds and → unfolds
//! it, at both levels, grouped by repo and by status; a fold outlasts a refresh and the board closing, gives way to a
//! filter and to a task the selection is taken to; the hint line and the ? menu.
//! A repo's heading is its name in plain text, bold, as loud as its tasks' titles, its key and caret muted.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::json::*;
use sw::shown::ink::Ink;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
/// A board of three tasks in `proj`.
fn three() -> (World, Board) {
    let w = world_with(Opts { repo: true, ..Default::default() });
    for n in 1..=3 { w.add(&format!("Mine {n}")); }
    let mut s = open(&w.root, 40); s.s.side = false;
    s.s.sel = top(); s.load(); // the board opens on its top row
    (w, s)
}
/// Arrow down until the selection is on this heading (or on this task).
fn go(s: &mut Board, id: &str) { for _ in 0..40 { if s.s.sel == id { return; } s.press(&["down"]); } for _ in 0..40 { if s.s.sel == id { return; } s.press(&["up"]); } panic!("{id} is not reached with the arrows"); }
fn top() -> String { format!("head:machine") }
fn repo(name: &str) -> String { format!("head:repo\t{name}") }
fn status(name: &str) -> String { format!("head:status\t{name}") }

#[test]
fn this_machines_heading_its_repos_under_it_and_rows_with_the_number_alone() {
    let (_w, mut s) = three();
    let here = sw::shown::words::clean(&sw::util::hostname());
    // By status: the top row, its status groups a step in.
    let t = s.screen(100).join("\n");
    assert!(t.starts_with(" herdr-projects") && has(&t, &format!(r"\n▌▾ {} {{2}}1 project · 3 tasks *\n   ▾ to do\n", regex::escape(&here))), "{t}");
    // By repo: the top row, its repo a step in.
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let t = s.screen(100).join("\n");
    assert!(has(&t, &format!(r"\n▌▾ {} {{2}}1 project · 3 tasks *\n   ▾ proj · PRJ *\n[ ▌] {{4}}○ PRJ-\d+ +Mine", regex::escape(&here))), "{t}");
    assert!(!t.contains("proj/") && !t.contains("PRJ-") == false, "{t}");
}

#[test]
fn left_on_a_heading_folds_it_to_one_line_with_how_many_it_holds_and_right_unfolds_it_at_both_levels() {
    let (_w, mut s) = three();
    // The arrows stop on each heading, which is selected as a row is.
    go(&mut s, &status("to do"));
    assert!(s.screen(100).iter().any(|l| has(l, r"^▌ *▾ to do")), "{}", s.text());
    assert!(s.text().contains("enter fold") && !s.text().contains("alt+o go into it"), "a status group has no agent: {}", s.text());
    s.press(&["left"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, r"\n▌  ▸ to do  3 tasks"), "{t}"); assert!(!t.contains("Mine 1"));
    assert_eq!(s.s.sel, status("to do"), "the selection stays on the heading");
    assert!(s.text().contains("enter unfold"));
    s.press(&["left"]); assert!(s.s.folded.contains(&status("to do")), "← again leaves it folded");
    s.press(&["right"]); // unfolded again, so the top row has its groups to show
    // The top row folds whole; unfolded, its groups are as they were.
    go(&mut s, &top());
    s.press(&["left"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, &format!(r"\n▌▸ {} {{2}}3 tasks *\n", regex::escape(&sw::shown::words::clean(&sw::util::hostname())))), "{t}");
    assert!(!t.contains("Mine"));
    s.press(&["right"]);
    assert!(s.text().contains("Mine 1"));
    // ← on a task's row goes to its heading.
    let task = s.s.items.iter().find(|r| std::sync::Arc::as_ref(*r).s("title") == "Mine 2").unwrap().as_ref().t("id");
    go(&mut s, &task); s.press(&["left"]); assert_eq!(s.s.sel, status("to do"));
    // By repo: a repo's heading folds the same way.
    s.press(&["alt+s"]); go(&mut s, &repo("proj"));
    s.press(&["left"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, r"\n▌  ▸ proj · PRJ  3 tasks"), "{t}"); assert!(!t.contains("Mine 1"));
    s.press(&["right"]); assert!(s.text().contains("Mine 1"));
    // The ? menu says it too.
    s.press(&["?"]); assert!(s.text().contains("fold the heading"), "{}", s.text());
}

#[test]
fn a_fold_outlasts_a_refresh_and_the_board_closing_a_filter_and_a_task_taken_to_open_it() {
    let w = world();
    for n in 1..=3 { w.add(&format!("Task {n}")); }
    let mut s = open(&w.root, 30); s.s.side = false;
    let todo = status("to do");
    go(&mut s, &todo); s.press(&["left"]);
    s.load(); s.load();
    assert!(!s.text().contains("Task 1") && s.text().contains("▸ to do  3 tasks"), "a refresh keeps it folded: {}", s.text());
    let saved = sw::store::files::read_obj(&sw::store::files::data_dir().join("view.json")).unwrap();
    assert_eq!(saved.get("folded"), Some(&json!([todo])), "kept with the view");
    let mut again = open(&w.root, 30); again.s.side = false;
    assert!(again.text().contains("▸ to do  3 tasks") && !again.text().contains("Task 1"), "and after the board closed: {}", again.text());
    // A filter shows every task that matches, folded or not; the fold is back after it.
    again.press(&["ctrl+f"]); again.typed("Task 2");
    assert!(again.text().contains("Task 2"), "{}", again.text());
    again.press(&["esc"]); assert!(!again.text().contains("Task 2"));
    // A task the selection is taken to (a jump from elsewhere: herdr's approve action, the popup's task) opens its heading.
    let two = again.s.items.iter().find(|r| std::sync::Arc::as_ref(*r).s("title") == "Task 2").unwrap().as_ref().t("id");
    again.s.sel = two.clone(); again.load();
    let t = again.text();
    assert!(t.contains("Task 2") && t.contains("Task 1") && !t.contains("▸ to do") && again.s.sel == two, "{t}");
}

#[test]
fn a_click_on_a_heading_folds_it() {
    let w = world();
    for n in 1..=2 { w.add(&format!("Task {n}")); }
    let mut s = open(&w.root, 30); s.s.side = false;
    s.screen(120);
    let y = s.s.list_ids.iter().position(|id| id == &status("to do")).unwrap();
    s.click((s.s.list_at.unwrap().0 + y) as u16);
    assert!(s.text().contains("▸ to do  2 tasks"), "{}", s.text());
}

#[test]
fn a_repos_heading_is_its_name_in_plain_text_normal_weight_as_its_titles_and_its_key_muted() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    w.add("Mine 1");
    let mut s = open(&w.root, 30); s.s.side = false;
    if !s.s.by_repo { s.press(&["alt+s"]); }
    let lines = s.inked(100);
    // Each piece of the first line with `on`, with its ink and boldness: blanks left out.
    let pieces = |on: &str| { let l = lines.iter().find(|l| l.text().contains(on)).unwrap_or_else(|| panic!("no {on:?}")); l.0.iter().filter(|p| !p.t.trim().is_empty()).map(|p| (p.t.trim().to_string(), p.c, p.bold)).collect::<Vec<_>>() };
    let head = pieces("▾ proj");
    assert_eq!(head, vec![("▾".into(), Ink::Muted, false), ("proj".into(), Ink::Plain, false), ("· PRJ".into(), Ink::Muted, false)]); // names have no colour, a project's no weight
    let title = pieces("Mine 1").into_iter().find(|p| p.0.contains("Mine 1")).unwrap();
    assert_eq!(title.1, Ink::Plain, "the heading is as loud as the titles under it: {title:?}");
}
