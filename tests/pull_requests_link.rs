//! A pull request the board opened but could not read: gh said the GraphQL endpoint failed after it had made the pull
//! request, so the board recorded a failure and kept no link. The next poll looks again by branch, links it and finishes
//! the task from there; and a failure that is no longer what stops the task is not shown once its agent works again.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

const URL: &str = "https://github.com/o/r/pull/9";
const NOT_READ: &str = "GraphQL: HTTP 404 (https://api.github.com/graphql)";

fn pr_world() -> World {
    world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() })
}

fn failed_at_the_pull_request(w: &World, title: &str) -> String {
    w.gh_says(json!({ "createFail": NOT_READ, "prs": [] }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add(title);
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.at("failure"));
    assert_eq!(t.at("failure").s("stage"), "pr", "{}", t.at("failure"));
    assert!(t.s("pr").is_empty(), "{}", t.s("pr"));
    id
}

#[test]
fn a_pull_request_gh_could_not_read_is_linked_on_the_next_poll_and_the_task_finishes() {
    let w = pr_world();
    let id = failed_at_the_pull_request(&w, "Unread pull request");
    // It was opened all along: gh's read of it failed on the way.
    w.gh_says(json!({ "createFail": NOT_READ, "prs": [{ "number": 9, "url": URL, "state": "OPEN", "baseRefName": "main" }] }));
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Pr);
    let t = settle(&id);
    assert_eq!(t.s("pr"), URL);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert!(t.at("failure").is_null(), "{}", t.at("failure"));
}

#[test]
fn a_failure_is_not_shown_once_the_agent_works_on_the_task_again() {
    let w = pr_world();
    let id = failed_at_the_pull_request(&w, "Resumed after a failure");
    let pane = task(&id).at("link").t("pane");
    assert!(sw::shown::states::failure_for(&row(&id)).is_some());
    set_status(&w, &pane, "working");
    let r = row(&id);
    assert_eq!(r.s("state"), "working", "{}", r.s("note"));
    assert!(sw::shown::states::failure_for(&r).is_none(), "{}", r.at("failure"));
}

#[test]
fn a_failed_task_whose_agent_works_shows_working_not_went_wrong() {
    // The finish line stopped at a step, and the agent was taken on again in its own terminal before the hook re-armed
    // the task: herdr's live reading is what it does, not the step that failed.
    let t = obj(json!({ "id": "p/1", "stage": "failed", "agent": "omp", "app": "omp", "owner": "you",
        "failure": { "stage": "pr", "title": "Could not open the pull request", "detail": "GraphQL: HTTP 404" } }));
    let s = sw::state::show(&t, &json!({ "status": "working" }), sw::clock::now());
    assert_eq!(s.state, "working", "{}", s.msg);
    let mut r = t.clone();
    r.put("state", s.state);
    assert!(sw::shown::states::failure_for(&r).is_none(), "{}", r.at("failure"));
}
