# Windows

herdr-projects runs natively on Windows, without WSL, as a herdr plugin. It has the same board and the same checks as
Linux. The Unix and Windows code sits in one platform layer, `crates/core/src/sys/`: `sys/unix.rs` keeps the
Unix bodies as they were, and `sys/windows.rs` gives each function its Windows body. The rest of the program calls the
same functions on both.

## Status

This branch lists Windows in the plugin's manifest (`herdr-plugin.toml`). Merging it to master waits for a live run on a
real Windows machine that passes. [install.md](install.md) says what Windows needs.

## What differs

- **Herdr**: 0.9.4 or newer. The board checks herdr's version when it starts and refuses an older one, naming the version
  it needs. Off Windows the plugin needs 0.9.3, the manifest's minimum, which herdr enforces on every platform.
- **Herdr's socket** is a named pipe: `\\.\pipe\` followed by herdr's socket path. Named sessions (`HERDR_SESSION`) are
  not checked on Windows yet. A request waits for its reply at most the timeout; a reply that never comes closes the pipe
  rather than leaving a thread waiting on it.
- **Pane shell**: PowerShell 7 is recommended (`default_shell` in herdr's `config.toml`). Pane commands are quoted for
  PowerShell.
- **Stopping**: there are no soft signals on Windows. Stopping a task's processes ends them and their children at once,
  children first, so a program gets no chance to clean up. A process whose parent has already ended is not in the tree
  the process table shows, so it is not stopped with it; on Unix the process group reaches it.
- **Folders in use**: a folder that a stopped process still holds is removed once the process lets go. The board retries
  for up to 5 seconds, then leaves the folder for a later pass.
- **Replacing a program**: Windows will not write over a program that is running. An update (`herdr-projects update`, and
  the launcher's download) moves the running copy aside, `<name>.<pid>.old`, and puts the new one in its place. A copy
  moved aside is removed by a later replacement, once no program runs from it.
- **Replacing a file another program has open** without delete sharing (an editor or a scanner may; Rust's own reads do not
  block it): the write retries for up to a second, then leaves the old file as it was and reports the failure. The new file is kept, so the write can be tried again.
- **Locks** use `LockFileEx` on the lock file.
- **Links**: the program makes no links on Windows. The one-time move into the home folder is not made here, so the
  folders stay where they are. Links need Developer Mode or an administrator (error 1314); the two tests that make links
  are gated on Windows until then.
- **Held panes**: Windows has no process table of terminals, so the board cannot tell which views hold a pane. That check
  is off, and a held pane is not reported.
- **Background jobs**: a job starts in its own process group, not in a console of its own. Whether it keeps running when
  the console that started it is closed (a pane closed) has not been checked on a live Windows run.
- **Launcher**: `bin/run.ps1` runs the same steps as `bin/run.sh`, with the same release file names plus `.exe`.
- **Command on PATH**: the launcher is `herdr-projects.cmd` in the data folder (cmd and PowerShell run it by name; agents and hooks get its path), with a shell script `herdr-projects` beside it for Git Bash ([installing](install.md#the-command-on-your-path)).

## Not on Windows yet

- **Voice calls** are off on Windows: a call says so in one line.
- **Remote machines** and **the web board** are not part of this program on any platform.
- **WSL** is not a target. The Windows build is native.
- **ARM Windows** is not built. The release file is `herdr-projects-windows-x86_64.exe`.
