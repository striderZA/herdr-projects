//! The gh calls of a clone are for its origin's GitHub repo and no other: a task's pull request is opened in it, a pull
//! request link of another repo is not acted on, and a clone whose origin is not GitHub gets no gh call at all (gh would
//! take its upstream first). The command that runs gh is checked here, and the finish line through it.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use sw::actions::finish_line;
use sw::finish::already;
use sw::flow::git::gh_command;
use sw::json::*;
use sw::store::tasks::{update, Change};

const FORK: &str = "striderZA/herdr-projects";
const FORK_URL: &str = "git@github.com:striderZA/herdr-projects.git";
const ORIGINAL_URL: &str = "git@github.com:eliasstravik/shepherdr.git";
const ORIGINAL_PR: &str = "https://github.com/eliasstravik/shepherdr/pull/7";

fn pr_world() -> World {
    world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() })
}
/// The test's bare repo, which every world's origin answers from.
fn bare(w: &World) -> String { w.base.join("origin.git").display().to_string().replace('\\', "/") }
/// The clone's origin becomes the fork (its URL, as the real one), answered from the bare repo, and `upstream` the original.
fn name_fork(w: &World) {
    git(&w.root, &["config", "remote.origin.url", FORK_URL]);
    reach(&w.root, &bare(w));
    git(&w.root, &["remote", "add", "upstream", ORIGINAL_URL]);
}
/// The first call of gh that starts with `prefix`, as the test logged it.
fn first_call(w: &World, prefix: &str) -> String {
    w.gh.log.borrow().iter().find(|l| l.starts_with(prefix)).cloned().unwrap_or_default()
}
/// The environment a call's command runs gh with.
fn env_of(c: &Command) -> Vec<(String, String)> {
    c.get_envs().filter_map(|(k, v)| Some((k.to_str()?.to_string(), v?.to_str()?.to_string()))).collect()
}
fn gh_repo_of(c: &Command) -> Option<String> {
    env_of(c).into_iter().find(|(k, _)| k == "GH_REPO").map(|(_, v)| v)
}
/// Why a call is refused: the board does not ask gh for it.
fn refused(args: &[&str], root: &Path) -> String {
    gh_command(args, root).err().unwrap_or_else(|| panic!("gh would be asked: {args:?}"))
}

#[test]
fn the_pull_request_is_opened_in_the_fork_its_branch_is_pushed_to_never_in_the_original() {
    let w = pr_world();
    name_fork(&w);
    let id = to_approval(&w, "In the fork");
    assert_eq!(task(&id).s("pr"), "https://github.com/striderZA/herdr-projects/pull/7");
    let create = first_call(&w, "pr create");
    assert!(create.contains(&format!("--repo {FORK}")), "{create}");
    assert!(!create.contains("eliasstravik"), "{create}");
}

#[test]
fn gh_is_asked_for_the_origin_repo_by_name_and_never_for_a_pull_request_of_another_repo() {
    let w = pr_world();
    assert_eq!(gh_repo_of(&gh_command(&["pr", "view", "7"], &w.root).unwrap()).as_deref(), Some("o/r"));
    let merge = ["pr", "merge", "https://github.com/o/r/pull/7", "--squash"];
    assert_eq!(gh_repo_of(&gh_command(&merge, &w.root).unwrap()).as_deref(), Some("o/r"));
    assert!(refused(&["pr", "merge", ORIGINAL_PR, "--squash"], &w.root).contains("is not a pull request of o/r"));
    name_fork(&w);
    assert_eq!(gh_repo_of(&gh_command(&["pr", "view", "11"], &w.root).unwrap()).as_deref(), Some(FORK));
    assert!(refused(&["pr", "view", ORIGINAL_PR], &w.root).contains(&format!("is not a pull request of {FORK}")));
}

#[test]
fn a_clone_whose_origin_is_not_github_is_asked_nothing_and_opens_no_pull_request() {
    let w = pr_world();
    git(&w.root, &["remote", "set-url", "origin", &bare(&w)]); // a folder, not a GitHub repo
    assert!(refused(&["pr", "list", "--head", "feat/1"], &w.root).contains("not a GitHub repo"));
    assert!(!already::archived(&w.root));
    assert!(!gh_log(&w).contains("repo view"), "{}", gh_log(&w));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Local origin");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.at("failure"));
    assert_eq!(t.at("failure").s("title"), "Could not open the pull request", "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("not a GitHub repo"), "{}", t.at("failure"));
    assert!(t.s("pr").is_empty() && !gh_log(&w).contains("pr create"), "{}", gh_log(&w));
}

#[test]
fn a_pull_request_link_of_another_repo_stored_on_a_task_is_not_acted_on() {
    let w = pr_world();
    let id = to_approval(&w, "Stored elsewhere");
    update(&id, |t| { t.put("pr", ORIGINAL_PR); Ok(Change::Write) }).unwrap();
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_ne!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(!gh_log(&w).contains("pr merge") && !w.gh.merged.get(), "{}", gh_log(&w));
    assert!(format!("{} {}", t.s("msg"), t.at("failure")).contains(&format!("is not a pull request of o/r")), "{} {}", t.s("msg"), t.at("failure"));
}

#[test]
fn archived_is_asked_of_the_origin_repo_by_name_and_not_asked_at_all_for_a_clone_not_on_github() {
    let w = pr_world();
    assert!(!already::archived(&w.root));
    assert_eq!(first_call(&w, "repo view"), "repo view o/r --json isArchived --jq .isArchived");
    git(&w.root, &["remote", "set-url", "origin", &bare(&w)]);
    let before = gh_log(&w);
    assert!(!already::archived(&w.root));
    assert_eq!(gh_log(&w), before, "no call was made");
}

#[test]
fn a_clone_whose_origin_is_ssh_with_a_port_is_asked_by_its_repo_name() {
    let w = pr_world();
    git(&w.root, &["remote", "set-url", "origin", "ssh://git@github.com:22/o/r.git"]);
    assert_eq!(gh_repo_of(&gh_command(&["pr", "view", "7"], &w.root).unwrap()).as_deref(), Some("o/r"));
}
