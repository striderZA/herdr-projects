# Several boards: repos, sessions, Coordinators

**Short answer: one board holds every repo.** You don't need a board per project. Each herdr session has its own
Coordinator, but they all share the one list.

| You want | What happens |
|---|---|
| several repos | one board, one list. Every task's tag names its repo (`PW-12`, `HSW-3`), `Alt+S` groups the list by repo, and the header names the repo new tasks go to: the one you opened the board in ([task numbers and repo keys](task-numbers.md)) |
| several herdr sessions | one list, shared: every session's board shows every task. A task's agent runs in the session it was started in; a board in another session shows that task as `In session <name>` with what was last known, and asks you to open the board there to act on it. The keeper looks after the tasks of every session |
| several Coordinators | one top-level Coordinator per herdr session, never two in one session: it works on every repo, and each message carries the repo you were in. Each repo also has its own project coordinator ([the Coordinator](coordinator.md), [project coordinators](project-coordinators.md)) |
| to open the board twice | the board key never opens a second board: it goes to the one that runs ([the board](board.md)). A popup and a board in a tab can run side by side, and a board you start yourself with `herdr-wingman` in a pane works too: they all show the same tasks, each with its own selection and filter |
| two separate lists (say, work and personal) | there is no built-in way. Keep one board and use the repo groups (`Alt+S`) or find (`Ctrl+F`). `HERDR_PROJECTS_HOME` moves the board's folder ([the home folder](home-folder.md)); it is not made for running a second board beside the first: herdr's board key and its events use the folder herdr itself was started with |

## Sending new tasks to another repo

The header names the repo new tasks go to. Three ways to send one to another repo, without a second board:

- **The popup follows you.** With `board.open popup`, the board opens in the repo of the pane you are in: go to a pane of
  that repo, press the board key, type ([the board as a popup](board.md#the-board-as-a-popup)). It is the folder the
  pane is in now: a pane that `cd`'d into another repo is in that repo, also in a space that started in another one. A
  task's worktree is its repo. A board in a tab keeps the repo it opened in.
- **Tell the Coordinator which repo**: "add this to gtm-agent" works from any repo. It adds the task with `--repo`, as
  you can yourself: `herdr-wingman add "Fix the login" --repo gtm-agent`.
- **Talk to the repo's own coordinator.** Group by repo (`Alt+S`) and press `Space` on the repo's row. What you say
  goes to that repo's project coordinator, and the tasks it adds are that repo's
  ([project coordinators](project-coordinators.md)).
