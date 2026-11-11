//! A repo whose pull requests go through bb (`repo.cli bb`: a CLI shaped like gh, for a Bitbucket Server): its calls
//! run bb, name the repo as Bitbucket does (`PROJ/repo`), never ask GitHub's own commands (`api`, `run`, `issue`), and
//! act on no pull request link of another Bitbucket repo; the finish line opens and merges its pull request through
//! bb, waits for no GitHub Actions workflow, and says bb, not gh, where it tells someone what to run.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use sw::actions::finish_line;
use sw::flow::git::{gh_command, origin_repo};
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

const BB_SSH: &str = "ssh://git@bitbucket.example.com:7999/proj/repo.git";
const BB_PR: &str = "https://bitbucket.example.com/projects/PROJ/repos/repo/pull-requests/42";

fn bare(w: &World) -> String { w.base.join("origin.git").display().to_string().replace('\\', "/") }
/// A repo whose own committed file says its pull requests go through bb, its origin a Bitbucket Server repo answered
/// from the test's bare repo, and bb printing a Bitbucket link for the pull request it opens.
fn bb_world() -> World {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), repo_settings: Some(json!({ "repo": { "cli": "bb" } })) });
    git(&w.root, &["config", "remote.origin.url", BB_SSH]);
    reach(&w.root, &bare(&w));
    w.gh_says(json!({ "url": BB_PR }));
    w
}
fn program(args: &[&str], root: &Path) -> String { gh_command(args, root).unwrap().get_program().to_string_lossy().into_owned() }
fn env_of(c: &Command) -> Vec<String> { c.get_envs().filter_map(|(k, _)| k.to_str().map(String::from)).collect() }
fn refused(args: &[&str], root: &Path) -> String { gh_command(args, root).err().unwrap_or_else(|| panic!("bb would be asked: {args:?}")) }
fn first_call(w: &World, prefix: &str) -> String { w.gh.log.borrow().iter().find(|l| l.starts_with(prefix)).cloned().unwrap_or_default() }

#[test]
fn a_repo_set_to_bb_runs_bb_and_names_its_repo_as_bitbucket_does_while_another_stays_on_gh() {
    let w = bb_world();
    assert_eq!(program(&["pr", "view", "42"], &w.root), "bb");
    assert!(!env_of(&gh_command(&["pr", "view", "42"], &w.root).unwrap()).iter().any(|k| k == "GH_REPO"), "GitHub's repo variable is not bb's");
    assert_eq!(origin_repo(&w.root).as_deref(), Some("proj/repo"));
    // An https clone of a Bitbucket Server repo has `scm/` before the project.
    git(&w.root, &["config", "remote.origin.url", "https://bitbucket.example.com/scm/PROJ/repo.git"]);
    assert_eq!(origin_repo(&w.root).as_deref(), Some("PROJ/repo"));
    // Your entry for the repo is nearer than its file: set back to gh, it is gh again (gh refuses the `scm/` form).
    git(&w.root, &["config", "remote.origin.url", BB_SSH]);
    sw::store::repos::repo_of(&w.root).unwrap();
    set_value("repo.cli", &json!("gh"), &Scope::repo("proj")).unwrap();
    assert_eq!(program(&["pr", "view", "42"], &w.root), "gh");
    // And the setting is one repo's own, one of the two programs.
    assert!(set_value("repo.cli", &json!("bb"), &Scope::you()).unwrap_err().msg.contains("one repo's own"));
    assert!(set_value("repo.cli", &json!("bitbucket"), &Scope::repo("proj")).is_err());
}

#[test]
fn github_only_commands_are_never_asked_of_bb() {
    let w = bb_world();
    for args in [&["api", "user", "--jq", ".login"][..], &["run", "view", "--job", "1", "--log-failed"], &["issue", "close", "3", "--comment", "done"]] {
        assert!(refused(args, &w.root).contains("GitHub's own"), "{args:?}: {}", refused(args, &w.root));
    }
}

#[test]
fn a_pull_request_link_of_another_bitbucket_repo_is_not_acted_on() {
    let w = bb_world();
    assert!(gh_command(&["pr", "merge", BB_PR, "--squash"], &w.root).is_ok());
    let other = "https://bitbucket.example.com/projects/OTHER/repos/repo/pull-requests/7";
    assert!(refused(&["pr", "merge", other, "--squash"], &w.root).contains("is not a pull request of proj/repo"));
    // A personal repo (`~user`) has its pull requests under /users/.
    git(&w.root, &["config", "remote.origin.url", "ssh://git@bitbucket.example.com:7999/~jaco/repo.git"]);
    assert!(gh_command(&["pr", "view", "https://bitbucket.example.com/users/jaco/repos/repo/pull-requests/3"], &w.root).is_ok());
}

#[test]
fn a_bb_task_opens_its_pull_request_through_bb_and_merges_it_on_approval() {
    let w = bb_world();
    let id = to_approval(&w, "Through Bitbucket");
    assert_eq!(task(&id).s("pr"), BB_PR);
    let create = first_call(&w, "pr create");
    assert!(create.contains("--repo proj/repo"), "{create}");
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert!(w.gh.merged.get(), "{}", gh_log(&w));
    assert_eq!(sw::shown::words::pr_number(&t), "PR#42");
}

#[test]
fn no_github_actions_workflow_is_waited_for_on_a_bb_repo() {
    let w = bb_world();
    let flows = w.root.join(".github").join("workflows");
    std::fs::create_dir_all(&flows).unwrap();
    std::fs::write(flows.join("ci.yml"), "name: CI\non: pull_request\njobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n").unwrap();
    let id = w.add("Workflows of GitHub");
    let t = obj(json!({ "id": id, "branch": "feat/1-x" }));
    assert!(sw::land::expected::flows_for(&t, "main").is_empty());
    set_value("repo.cli", &json!("gh"), &Scope::repo("proj")).unwrap();
    assert_eq!(sw::land::expected::flows_for(&t, "main").len(), 1, "on gh the same workflow is waited for");
}

#[test]
fn a_pull_request_bb_could_not_open_says_bb_and_what_it_said() {
    let w = bb_world();
    w.gh_says(json!({ "url": BB_PR, "createFail": "HTTP 409: the branch has no changes against main" }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Not opened");
    delegate(&id);
    let t = settle(&id);
    let f = t.at("failure");
    assert_eq!(f.s("title"), "Could not open the pull request", "{f}");
    assert!(f.s("detail").contains("bb pr create:\nHTTP 409: the branch has no changes against main"), "{f}");
    assert!(f.s("next").starts_with("Inspect the bb error") && !f.s("next").contains("GitHub"), "{f}");
}

#[test]
fn failed_checks_tell_the_agent_to_look_with_the_repos_own_program() {
    let w = bb_world();
    assert!(sw::land::checks::failed_why(&w.root, BB_PR, &["build".into()]).contains("(bb pr checks)"));
}

#[test]
fn feedback_on_a_bb_pull_request_wakes_the_agent_with_bbs_own_command() {
    let w = bb_world();
    let id = to_approval(&w, "Commented on");
    let t = task(&id);
    let before = prompts(&w, t.at("link").s("pane")).len();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    // GitHub's own `api user` is not bb's: the pull request's author is taken for the human.
    w.gh_says(json!({ "url": BB_PR, "view": { "state": "OPEN", "headRefOid": t.at("prHead"), "statusCheckRollup": [], "reviews": [], "author": { "login": "jaco" },
        "comments": [{ "id": "c1", "author": { "login": "jaco" }, "body": "Please rename the flag.", "updatedAt": "2026-01-01" }] } }));
    sw::housekeeping::poll_prs();
    let p = prompts(&w, task(&id).at("link").s("pane"));
    assert_eq!(p.len(), before + 1, "{p:?}");
    assert!(p.last().unwrap().contains("(bb pr view --comments)"), "{}", p.last().unwrap());
    assert!(!gh_log(&w).lines().any(|l| l.starts_with("api ")), "{}", gh_log(&w));
}

#[test]
fn a_pull_requests_number_is_read_from_a_bitbucket_link_as_from_a_github_one() {
    let n = |pr: &str| sw::shown::words::pr_number(&obj(json!({ "pr": pr })));
    assert_eq!(n(BB_PR), "PR#42");
    assert_eq!(n(&format!("{BB_PR}/overview")), "PR#42");
    assert_eq!(n("https://github.com/o/r/pull/7"), "PR#7");
}

#[test]
fn a_bb_merge_never_says_github_closes_an_issue_its_body_names() {
    let w = bb_world();
    let id = to_approval(&w, "Closes three");
    set_value("merge.closes", &json!("3"), &Scope::task(&task(&id))).unwrap();
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    // Its body has `Closes #3`, which Bitbucket does not act on: the board closes #3 itself, and says what it could not.
    assert!(!t.s("msg").contains("GitHub closes"), "{}", t.s("msg"));
    assert!(t.s("msg").contains("Could not close #3"), "{}", t.s("msg"));
}
