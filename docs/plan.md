# A plan first

**With `plan.approve` on, the agent writes a plan before it builds, and nothing in the repo is changed until the plan
has its yes.** A misunderstanding caught in two minutes saves a wrong build caught in forty.

```
herdr-projects delegate 12 --plan.approve you             # show me the plan first
herdr-projects set plan.approve reviewer --repo my-app    # in this repo the reviewer checks every plan
```

Or tell the [Coordinator](coordinator.md): "show me the plan first".

```
task ─► the plan is written (nothing changes) ─► yes: you, or the reviewer ─► the agent builds that plan ─► check, review, merge
                     ▲                                │
                     └──── sent back with words ◄─────┘
```

## The setting

| Setting | Values | Means |
|---|---|---|
| `plan.approve` | `none` (built in), `you`, `reviewer` | `none`: no plan, the agent builds at once. `you`: the agent writes its plan and the task waits as **plan ready** for your yes. `reviewer`: the reviewer checks the plan against the task's description, and the build goes on by itself when the plan passes |

- It is a work [setting](settings.md) like every other: looked up task → your entry for the repo → you → built in; set
  with `set`; taken by `add --delegate` and `delegate` as `--plan.approve you`; held by an [automation](automations.md);
  listed by `show`.
- **A repo's own file cannot set it**, so code an agent writes cannot switch a plan's approval on or off.
- **When it is read.** It is read when the agent first starts on the task: set on a task whose agent already builds, it
  changes nothing about that run. It is read once more when the plan is written, for who says yes: if it was turned off
  meanwhile, nobody is asked and the build goes on.
- A new round of a task that landed (delegated again after its clean-up) starts with a new plan.

## Plan ready: say yes, or send it back

With `plan.approve: you`, the task waits in a state of its own, **plan ready**. It stands under **needs you**: the row
says `Plan ready · <the plan's first words>`, the State line and Herdr's sidebar say `plan ready`, and `board.notify`
fires as for anything that needs you.

The task panel shows the plan under **Plan**: where it stands, who wrote it, the plan's own words (its first lines beside
the list, all of it full screen), and **`Open plan`**, the plan's file in the task's folder. You do one of two things:

- **Yes**: the approve key (`ctrl+l`), or `herdr-projects approve <id>`. The plan is approved as it stands and the build
  starts.
- **A reply**: `space`, then your words, or `herdr-projects tell <id> <text>`. The plan goes back to its writer with
  your words and is written again. The task is plan ready again when the new plan is there.

## The reviewer's look

With `plan.approve: reviewer`, the reviewer (`reviewer.agent`, in a tab named `Plan review`) reads the plan and the code
it speaks of, and answers as it does for work: does building this plan do what the task asked, and a list of findings by
severity. [The gate](gate.md) decides by the same rule (`review.mustFix`):

- the plan passes: the build goes on, and nobody is asked;
- it fails: the plan goes back to its writer with the reviewer's points, by itself, up to `review.rounds` times;
- still failing then: the task is plan ready, with what the reviewer found in the plan's part, and the yes is yours;
- **the review cannot run** (its reviewer cannot start, or stops without a verdict): the task is plan ready all the same,
  its row saying why (`The plan's review could not run (…): yours to judge`). Your yes (`ctrl+l`) builds the plan as
  it stands; `herdr-projects retry <id>` has the reviewer look at it again.

The reviewer runs in the task's worktree, with the same read-only files the writer had. It is told not to change
anything, and whatever it alters is put back.

A look at a plan is not one of the task's review rounds: the first review of the work is still `Reviewer #1`.

## After the yes

The agent builds that plan.

- The plan's own writer hears `Your plan is approved` in its conversation: its program goes on in the same conversation,
  and the worktree is writable again first.
- The review of the finished work is told of the approved plan. It checks the work against the task **and** the plan,
  and says where the build left the plan (`Left the plan` under the review in the task panel). Leaving the plan is a
  finding only when the result is worse for it.

## Who writes the plan

The working agent. It is told to change nothing; to write what it understood, what it will change and where, in which
order, how it will be checked and what it is unsure of; and to stop.

- **Every program plans in the task's worktree** (Claude Code, Codex, pi, Herdr's others: [agents](agents.md)) and
  saves the plan as a file in the task's own folder. The [read-only worktree](#nothing-in-the-repo-is-changed-before-the-yes)
  keeps the repo as it is, so the writer can still save notes, publish a page or write its closing note without
  stopping at its pane for a yes.
- **Claude Code's own plan mode is not used**: it asks at the pane for every write outside its plan file. A Claude Code
  that an older board started in it still has its plan taken when it presents it, until the next release.

### The plan file

One file, `plan.md`, in the task's own folder on the board
(`~/.herdr-projects/data/tasks/<repo>/<n>/artifacts/plan/`), never in the repo.

- An agent that stopped without saving one: its last message is the plan.
- Nothing said either: `No plan was written`, which goes back to the agent like any failure.
- A plan that went back and was not saved again is the old one. Then what its writer said last is the new plan; with
  nothing said, there is no new plan. An older file is never what gets the yes.

## Nothing in the repo is changed before the yes

The board does not only ask for it. Three things hold, from the moment the plan step begins until the build starts:

1. **The worktree's files are read-only.** Every file and folder in it loses its write permission, ignored files too,
   and so does the worktree's own git folder (its index, its `HEAD`). A write, a delete or a commit is refused
   (`Permission denied`) while the bits are off, whatever the program's own access. This holds for every program
   (Claude Code, Codex, pi and Herdr's others), and for the reviewer that checks a plan. The plan can still be saved in
   the board's folder, and the program keeps its own session files.

   The permission comes back when the build starts, before its agent is started (so a start that fails leaves the
   worktree writable, and `plan.locked` false), when you stop the task, and before the worktree is removed. Delegated
   again with the plan step still open, its writer starts again and the worktree is read-only again.

   Every way out of the plan step leaves the worktree writable: your yes (also to a plan whose step failed after it was
   written), a stop, and a delegate with the plan taken off (`set plan.approve none --task <id>`), which builds the plan
   that was written as it stands, or starts the work when none was.
2. **The board looks for itself**, when the plan's turn ends and once more right before the build starts. If something
   changed all the same (somebody took the read-only bit off), it undoes it in the task's own worktree: the branch goes
   back to the commit it stood on, added files go, and so do ignored files that were not there when the step began.
   What was undone is kept as `changed-too-early.patch` beside the plan, and the task says how much was put back
   (`Put back  2 changes made in the repo before the yes`).
3. **Nothing is built on a change the board may not or could not undo.** The plan is not shown and the build does not
   start: the task says `The repo was changed before the plan was approved`, with what changed. Sent to the agent, it
   undoes it. A yes already given stands, and the start is tried again once the change is gone.

And while the plan step is open, no turn's end counts as finished work: nothing is committed, checked, pushed, opened as
a pull request or reviewed.

The read-only permission is not a wall against a program that sets out to get around it (one that asks Herdr to run a
command in another pane, say, or puts the write bit back). The board's own look, and the hold on the build, are for the
rest.

### A task in your own checkout

A task with no worktree of its own differs in one way: the board never takes the write permission off your files and
never undoes anything there, because what is there may be yours.

- Rule 3 holds for whatever changes there meanwhile: it goes back to the agent (`review.rounds` times, then it waits for
  you), and nothing is built until it is gone.
- A task in a repo always gets a worktree of its own when it is first delegated.

A folder that is no repo has nothing to guard: there the plan is written and approved the same way.

## Several attempts on one task

The plan belongs to the task, not to an agent or a worktree: one file, written once, approved once. With
[several attempts](attempts.md) (`attempts.by`):

1. The task is not split when it starts. The working agent makes the plan, and it is approved once.
2. Only then is the task split. Every attempt gets that same plan in its first prompt, and builds it.
3. The worktree and tab the plan was written in become attempt A's. Each other attempt gets its own beside it.
4. From there on, all is as for any task done several times: the check, the pick, one review of the chosen attempt,
   which is checked against the task and the plan.

`attempts.by` is read for this when the build starts, so a count changed while the plan waits counts.
