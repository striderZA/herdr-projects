//! The title of a pull request the board opens: conventional commit style, and the task's key kept out of it (the body's
//! `Task:` line names the key). The mapping is tested alone, and once through the fake gh.
use crate::common;
use common::*;
use serde_json::json;
use sw::finish::pr_title::of_task;
use sw::store::keys::Keys;

#[test]
fn the_title_takes_a_conventional_kind_and_loses_the_task_key() {
    let keys: Keys = [("proj".to_string(), "PRJ".to_string())].into_iter().collect();
    let title = |t: &str| of_task("proj/1", "proj", t, &keys);
    assert_eq!(title("Fix the login page"), "fix: the login page");
    assert_eq!(title("Add dark mode"), "feat: add dark mode");
    assert_eq!(title("PRJ-1: fix the login"), "fix: the login");
    assert_eq!(title("(prj-1) Refactor the branch naming"), "refactor: the branch naming");
    assert_eq!(title("The login crashes on save"), "fix: the login crashes on save");
    assert_eq!(title("fix(ui)!: GitHub login crashes"), "fix(ui)!: GitHub login crashes");
    assert_eq!(title("Bump ABC-7 deps"), "build: bump ABC-7 deps", "another project's key stays");
    assert_eq!(title("PRJ-1"), "proj/1", "a title that is only the key gives the id, as before");
}

#[test]
fn a_pull_request_the_board_opens_is_titled_so() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    to_approval(&w, "PRJ-1 Fix the login page");
    let log = gh_log(&w);
    let title = log.split("--title ").nth(1).and_then(|s| s.split(" --body").next()).unwrap_or_default();
    assert_eq!(title, "fix: the login page", "{log}");
}
