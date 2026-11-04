# Project coordinators

**Every row of the list has an agent you can talk to.** A task's row has its agent; a project's row has its project
coordinator; the top row is the Coordinator itself ([the Coordinator](coordinator.md)).

| Row | Its agent | Named | Lives in Herdr |
|---|---|---|---|
| The top row: the machine with all its projects | the Coordinator | `the Coordinator` | a `Coordinator` tab in the home space (`~`) |
| A project's row (a repo's heading) | its project coordinator | after the repo: `gtm's coordinator`, `~'s coordinator` for the home folder | a tab of that name in the repo's own space |
| A task's row | its agent | `GTM-14's agent` | its own tab |

**The board opens with the top row selected**, so the chat starts with the Coordinator. `Space` on a project's row
talks to its coordinator instead, and `Space` again goes back to the top row.

## A coordinator's row reads as a task's

- **The same marks as tasks**, in the marks' column left of the caret: `○` idle (also one that never ran, or was
  stopped), `◐` working, `?` it asks you (`!` it wants your OK), `✗` it could not start. None of a task's other marks (`◑ ◆ ✓ ◷ ◇`) is a
  coordinator's, and an idle coordinator is not counted in To do. Where no coordinator can run
  (`coordinator.agent none`), its row has no mark.
- **The same right side as tasks: only the time.** `3m` while it works, `1h ago` otherwise, nothing when it never ran.
  No state words, no role word.
- **The same second line as tasks**: what it does while it works, or its question while it waits for you. It goes when
  it stops: no old news. Its latest message shows in the chat box when you select its row.
- **Its name**, a home folder as `~`, with no rule to the edge. The top row says what it holds, open or folded:
  `9 projects · 30 tasks`.
- **The list reads project → task by how far in each row stands.**
- In a narrow pane the names shorten; the mark and the time stay.

```
 ○ ▾ Jaco-Desktop  2 projects · 5 tasks
     ? ▾ gtm · GTM                                                     1m ago
         Two PRs wait on CI. Want me to retry #11?
       ◐ GTM-14  Bundle skills into the template                           3m
```

**A coordinator that asks you shows under Needs you**, beside the tasks, with what it wants on its second line, and is
counted there.

**The filter Coordinators** (`Tab` from All, [the filter row](board.md#the-filter-row)) leaves only the coordinators
that run: started and not stopped (working, idle, needs you), or failed so that you can fix it. Its count
is how many there are. The top row is always there, counted only while it runs. A stopped or never started one is not
there and not counted; its row stays in the other filters, with `○`.

## The chat box names who it talks to

Its top edge is a ladder from the top row down (`Jaco-Desktop › gtm › GTM-14`), the current step lit, and after it the
kind of agent (`· Agent`, `· Reviewer`, `· Project coordinator`, `· Coordinator`). The box's edge is in the status colour
of whoever it talks to ([colours](colours.md#the-box-says-who-you-type-to)).

## The same keys on every row

- **`Space` talks to it**: the chat points at that row's agent, and follows the selection from row to row, as it does
  from task to task. `Space` again goes back to the top row's Coordinator.
- **`Alt+O` goes into its agent**: its real Herdr pane, as on a task. A coordinator that is not running is started
  first. The board keeps running in its own tab, and your key goes back to it.
- **`Ctrl+S` starts it, `Ctrl+X` stops it**, and `Ctrl+E` shows its settings. Those are the
  same keys as on a task's row.
- **`Ctrl+P` pins its row**, as it pins a task's; the project's tasks come under it in the Pinned filter.
- **`Enter`** on its row with words typed sends them to it, exactly as `Space` and `Enter` do on a task.
