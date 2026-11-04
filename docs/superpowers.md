# The Superpowers method

**`worker.preset superpowers` tells a task's worker to use the
[Superpowers](https://github.com/obra/superpowers) skills inside the board's lifecycle.** The board keeps the worktree,
the check, the final review and the merge; Superpowers plans and executes the work inside the task.

```bash
herdr-projects set worker.preset superpowers --repo my-app
herdr-projects add "Add CSV export" --repo my-app --delegate --plan.approve you
```

It is a work setting: a task, your entry for a repo, or you. It is read when the agent starts, and it is never in the
repo's own `.herdr-projects.json`, so code an agent writes cannot switch its own method. The default is `none`: the
board's rules as they were.

## What it changes

The board adds one block to the worker's standing rules. It says:

- the board owns the worktree, branch, commits, push, pull request, review and merge;
- every stop that needs the human is a wait: `report … --state blocked` and then stop, including every question from
  `superpowers:brainstorming`;
- skip brainstorming when the task is already a complete spec; otherwise ask first;
- write the plan where the board's plan step says, in the Superpowers plan format; do not put a spec or plan in the
  repo unless the task asks for it;
- after the plan, use `superpowers:subagent-driven-development` when the work has independent tasks, otherwise
  `superpowers:executing-plans`, with `test-driven-development` and `verification-before-completion`;
- the board's reviewer is the final whole-branch review, so the worker does not run `requesting-code-review` or the SDD
  final review, and never runs `finishing-a-development-branch`.

The reviewer is told that the worker's per-task reviews are internal and that its own review is the final one. A plan
reviewer is told to check that the plan's tasks cover the task and each has a test or verification step.

## What the board still owns

The finish line is unchanged. The worker commits on its branch and stops; the board commits what is left, brings main
in, runs the repo's `check`, opens the pull request, has the reviewer write `verdict.json`, and the gate decides
([the gate](gate.md)). The worker never pushes, opens a pull request or merges. The board's reviewer is not replaced by
a Superpowers reviewer: it is the one that decides whether the work may land.

The worktree is unchanged too. The agent is already in the board's linked worktree, so `using-git-worktrees` only
detects it; no second worktree is made. SDD's workspace (`.superpowers/sdd/`) ignores itself, so the board's commit
does not carry ledgers or review packages.

## Questions and approvals

The board cannot read a question out of a last message. An unmarked stop is finished work and goes to check and review.
The preset therefore turns every Superpowers question into a wait, and the human's answer arrives as the agent's next
message. An interview, a design question and a choice of approach are all handled one answer at a time.

With `plan.approve you` or `reviewer`, the board's plan step is the plan's approval. The plan is written to the board's
`plan.md` in the task's own folder, in the Superpowers plan format; the board's plan approval replaces the
`writing-plans` request to review the plan and choose an execution method. With `plan.approve none`, multi-step work
gets a plan first with `writing-plans`, kept out of the repo unless the task asks for it, and is executed with SDD or
inline; a small change is just done.

There is no separate spec state. For work that needs a design discussion, use the Coordinator chat (or answer the
worker's brainstorming questions) and put the result in the task description. The plan file can hold the spec and the
plan together.

## Installing Superpowers

The board does not ship Superpowers; install it for the program the task runs. Claude Code and Codex have plugins;
pi, OpenCode and others have their own installers in the Superpowers README. Those programs load their own skills and
hooks when the board starts them, so the installed skills are found in the task's worktree.

omp has no native Superpowers adapter. Its skills can be found through the Claude-plugin compatibility path, but the
`using-superpowers` bootstrap is not injected automatically. The preset's rules tell an agent in that position to read
`superpowers:using-superpowers` first; for the full automatic bootstrap, use Claude Code or Codex.

## What it does not do

- It does not add a spec artifact or a separate design-approval state; the plan gate is the only approval.
- It does not change the board's review gate, check, merge, limits or safety rules.
- It does not let the worker finish the branch. `finishing-a-development-branch` is the board's job here.
- It does not set the Coordinator's method. A Coordinator can be told in its own words
  ([the Coordinator](coordinator.md)) to use `brainstorming` and `writing-plans` to shape task descriptions.
