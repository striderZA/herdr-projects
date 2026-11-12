# Get started

You need [Herdr](https://herdr.dev) 0.9.3+ on Linux (0.9.4+ on Windows), and Claude Code, Codex, pi or another agent herdr supports.

Coming from the herdr-projects plugin? Read [coming from an older version](install.md#coming-from-an-older-version) first; the agent steps below do the same.

## Option A: do it yourself

1. Install:

   ```sh
   herdr plugin install striderZA/herdr-wingman
   ```

2. In Herdr, run the action **Open the work board**. Say yes to the key it offers (`prefix+a`).
3. Optional: add `~/.herdr-wingman/data/bin` to your PATH so you can type `herdr-wingman`.

## Option B: let an agent do it

Paste this into Claude Code, Codex or pi, running inside Herdr:

```text
Install the herdr-wingman plugin for Herdr and set it up for me.
1. Check `herdr --version` is 0.9.3 or newer (0.9.4 on Windows), and that the running Herdr server is too; if it is older, tell me to restart Herdr. On Linux, check that git and curl (or wget) exist; on Windows, that git and PowerShell do. Stop and tell me if not.
2. If `herdr plugin list --json` has a plugin with id `herdr-projects` (this plugin's name before), remove it first, before this version runs for the first time (that run renames the old home folder): run its `remove-extras` with its launcher (`~/.herdr-projects/data/bin/herdr-projects remove-extras`; on Windows `~\.herdr-projects\data\bin\herdr-projects.cmd remove-extras`, or `herdr-projects remove-extras` if it is on PATH), then `herdr plugin uninstall herdr-projects`, then delete its launchers `herdr-projects` and `herdr-projects.cmd` from that data/bin folder. Its data stays.
3. Run `herdr plugin install striderZA/herdr-wingman`.
4. Get the plugin's folder: the `plugin_root` of the `herdr-wingman` entry in `herdr plugin list --json` (plain `herdr plugin list` does not show it). Run `sh <that folder>/bin/run.sh install` (on Windows, `powershell -File <that folder>\bin\run.ps1 install`). It writes the launcher to `~/.herdr-wingman/data/bin`.
5. Put the `herdr-wingman` launcher on my PATH: add its folder, `~/.herdr-wingman/data/bin`, so `herdr-wingman` works in new shells. On Windows, my user PATH; on Linux, my shell's start-up file (`~/.profile` or similar). Add it once.
6. Run `doctor` by the launcher's full path, since this shell's PATH does not have it yet: `~/.herdr-wingman/data/bin/herdr-wingman doctor` (on Windows, `~\.herdr-wingman\data\bin\herdr-wingman.cmd doctor`). Fix anything it reports.
7. Open the board: `herdr plugin action invoke open --plugin herdr-wingman` (on Windows, `open-windows` in place of `open`).
Then tell me in two lines: it is ready, and to say yes to the key it offers on its first screen (`prefix+a`, unless my config already uses it).
Details if you need them: https://github.com/striderZA/herdr-wingman/blob/master/docs/install.md
```

## Start it

- Press `prefix+a` in Herdr, or type `herdr-wingman` in any pane.
- Type what needs doing and press `Enter`. The Coordinator makes it a task.
- Press `Ctrl+S` instead of `Enter` to put an agent on it right away.
- Press `?` to see everything else.

Update later with `herdr-wingman update`.

More detail (building from source, uninstalling): [installing](install.md).
