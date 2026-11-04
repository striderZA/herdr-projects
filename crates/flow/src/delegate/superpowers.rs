//! The Superpowers method (`worker.preset`): the board keeps the worktree, the check, the final review and the merge;
//! the worker is told to use the Superpowers skills for the spec, the plan, TDD and subagent execution inside it.
use crate::json::Obj;
use crate::store::settings::lookup::setting_text;

/// Is this task run on the Superpowers method? Read from the task's work settings.
pub fn on(t: &Obj) -> bool { setting_text("worker.preset", Some(t)) == "superpowers" }

/// The board's own words for a worker on this method, after its fixed rules and before the human's instructions.
pub fn rules(t: &Obj) -> Vec<String> {
    if !on(t) { return vec![]; }
    vec![
        "This task uses the Superpowers method. Follow its skills where they do not disagree with the board's rules; the board's rules win.".into(),
        "The board owns this worktree, branch, commits, push, pull request, review and merge. Never use superpowers:finishing-a-development-branch. Do not push, open or merge a pull request, switch branches, or make another worktree; superpowers:using-git-worktrees may only detect the worktree you are already in.".into(),
        "Every stop that needs the human is a wait, not a finish: run the report command these rules give with --state blocked --kind question --msg \"<why>\" and then stop. This includes every question from superpowers:brainstorming and every choice it asks for. A stop without that command means finished and goes to check and review.".into(),
        "If this task's description is already a complete spec, skip superpowers:brainstorming. If it is not, ask the questions it needs first, each as a wait. In a plan step, write the plan to the path these rules give. For multi-step work with no plan step, write the plan first with superpowers:writing-plans; for a small change, do it. Keep a plan out of the repo unless the task asks for it there, and do not commit a spec or plan document unless the task asks for it. The board's plan approval replaces the writing-plans \"review the plan and choose execution\" prompt.".into(),
        "After the plan is approved, or when there is no plan step, use superpowers:subagent-driven-development when the work has independent tasks, otherwise superpowers:executing-plans. Use superpowers:test-driven-development and superpowers:verification-before-completion. Your program's own subagents are yours to use. The repo's check is the final test gate.".into(),
        "When the board sends review findings back, use superpowers:receiving-code-review; fix or answer every finding. The board's reviewer is the final whole-branch review: stop after the plan's tasks, do not run superpowers:requesting-code-review or the SDD final review, and end with the short summary these rules ask for.".into(),
        "If your program has not loaded the Superpowers bootstrap, read the superpowers:using-superpowers skill before you start.".into(),
    ]
}

/// What a plan's writer is told about the plan's shape, where the board's plan step is open.
pub fn plan(t: &Obj) -> String {
    if !on(t) { return String::new(); }
    "Write it in the Superpowers plan format: the goal, the spec it builds from, the global constraints, and tasks that each name the files, the interfaces and the test or check that proves them. The board's plan approval is the only approval this task needs: do not ask separately for a design or an execution method.".into()
}

/// What the reviewer of a plan on this method is told beyond its usual brief.
pub fn plan_review(t: &Obj) -> String {
    if !on(t) { return String::new(); }
    "This plan is in the Superpowers plan format: check that its tasks cover the task's description and that each has a test or verification step.".into()
}

/// What the reviewer of finished work on this method is told beyond its usual brief.
pub fn review(t: &Obj) -> String {
    if !on(t) { return String::new(); }
    "This task was built with the Superpowers method: the worker's per-task reviews are internal. You are the board's final whole-branch review. Do not invoke superpowers:requesting-code-review or dispatch another reviewer; judge the change as a whole and write verdict.json as this brief says.".into()
}
