# List headings

**The task list sits under headings, and `←` on a heading folds it, `→` unfolds it.** The headings are a repo's name
when the list is grouped by repo (`Alt+S`), or a status (`needs you`, `working`, `to do`, `done` …) when it is grouped by
status.

```
▾ Jaco-Desktop  2 projects · 4 tasks
  ▾ herdr-projects · HSW
    ◐ HSW-151 Rewrite the board in Rust on Ratatui       47%   3h
    ○ HSW-179 Remove the "To: Coordinator" label      1h ago
  ▾ open-crm · CRM
    ○ CRM-4   Explore Open CRM as the next-gen CRM     9h ago
```

**Every row sits tight**, with a filter on or off: no empty line between the top row and a project, between projects or
status groups, or between tasks. How far in a row stands, and its bold or its colour, say its level.

## The top row

**On top of every list is the top row**, this machine's name. Everything else stands under it, one step in. It says what
it holds, open or folded (`9 projects · 30 tasks`), and folds the whole list away. It is the Coordinator's row
([project coordinators](project-coordinators.md)), and the board opens on it. The archive has none.

## Repo headings

- **A repo's heading** is its name in normal weight, in the base colour of the task titles under it, with its key beside
  it, dimmed (`herdr-projects · HSW`). It is never quieter than its tasks, in any theme, so a list of several repos reads
  as several blocks. It has no rule.
- **The repo the board is in comes first.**
- A repo with nothing that matches has no heading while a filter is on, or in the archive.

## Folding

- **`←` on a heading folds it.** Its tasks hide, its caret turns to `▸`, and it says how many tasks it holds
  (`▸ open-crm  4 tasks`), and how many automations where it holds some (`▸ herdr-projects  7 tasks, 1 automation`,
  [automations](automations.md)). `→` unfolds it. `←` on a task's row goes to its heading.
- **`Enter` on a heading folds it and unfolds it too**; on a task's row it does nothing. `Alt+O` goes into a row's
  agent: on a project's row or the top row its coordinator, on a task its agent.
- **A click on a heading** folds it and unfolds it ([mouse](mouse.md)).
- The hint line says what `Enter` will do (`enter fold`, `enter unfold`), and the `?` menu lists `Enter`, `←` and `→`.
