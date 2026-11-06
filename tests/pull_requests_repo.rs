//! The pull request of a task is opened in the repo its branch is pushed to, named on the command: gh alone would take the
//! clone's `upstream` remote, the fork's original, before its `origin`. A clone's gh calls are for its origin's repo, and
//! only while that is on GitHub; a local origin names no repo, as before.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::finish::already;
use sw::flow::git::origin_repo;

const FORK: &str = "striderZA/herdr-projects";
const FORK_URL: &str = "https://github.com/striderZA/herdr-projects.git";
const ORIGINAL_URL: &str = "https://github.com/eliasstravik/shepherdr.git";

/// The clone's `origin` becomes the fork on GitHub and `upstream` the original. Pushes still reach the test's bare repo:
/// `insteadOf` sends the fork's URL there, as the real fork is reached on GitHub.
fn name_fork(w: &World) {
    let stand_in = w.base.join("origin.git").display().to_string().replace('\\', "/");
    git(&w.root, &["config", "remote.origin.url", FORK_URL]);
    git(&w.root, &["config", &format!("url.{stand_in}.insteadOf"), FORK_URL]);
    git(&w.root, &["remote", "add", "upstream", ORIGINAL_URL]);
}
/// The first call of gh that starts with `prefix`, as the test logged it.
fn first_call(w: &World, prefix: &str) -> String {
    w.gh.log.borrow().iter().find(|l| l.starts_with(prefix)).cloned().unwrap_or_default()
}

#[test]
fn the_pull_request_is_opened_in_the_fork_its_branch_is_pushed_to_never_in_the_original() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    name_fork(&w);
    let id = to_approval(&w, "In the fork");
    assert_eq!(task(&id).s("pr"), "https://github.com/o/r/pull/7");
    let create = first_call(&w, "pr create");
    assert!(create.contains(&format!("--repo {FORK}")), "{create}");
    assert!(!create.contains("eliasstravik"), "{create}");
}

#[test]
fn a_clone_names_its_origin_repo_only_when_origin_is_on_github_and_archived_asks_that_repo() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    assert_eq!(origin_repo(&w.root), None, "a local origin is no GitHub repo");
    name_fork(&w);
    assert_eq!(origin_repo(&w.root).as_deref(), Some(FORK));
    assert!(!already::archived(&w.root));
    // `repo view` ignores GH_REPO, so the repo is named in the call itself.
    assert_eq!(first_call(&w, "repo view"), format!("repo view {FORK} --json isArchived --jq .isArchived"));
}
