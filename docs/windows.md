# Windows

herdr-projects runs natively on Windows, without WSL, as a herdr plugin. It has the same board and the same checks as
Linux and macOS. The Unix and Windows code sits in one platform layer, `crates/core/src/sys/`: `sys/unix.rs` keeps the
Unix bodies as they were, and `sys/windows.rs` gives each function its Windows body. The rest of the program calls the
same functions on both.

The plugin lists Windows only after a live run on a real Windows machine passes. [install.md](install.md) says what
Windows needs.

## What differs

- **Herdr**: 0.9.4 or newer. The board checks herdr's version when it starts and refuses an older one on Windows, naming
  the version it needs.
- **Herdr's socket** is a named pipe: `\\.\pipe\` followed by herdr's socket path. Named sessions (`HERDR_SESSION`) are
  not checked on Windows yet.
- **Pane shell**: PowerShell 7 is recommended (`default_shell` in herdr's `config.toml`). Pane commands are quoted for
  PowerShell.
- **Stopping**: there are no soft signals on Windows. Stopping a task's processes ends them and their children at once,
  so a program gets no chance to clean up.
- **Folders in use**: a folder that a stopped process still holds is removed once the process lets go. The board retries
  for up to 5 seconds, then leaves the folder for a later pass.
- **Locks** use `LockFileEx` on the lock file.
- **Launcher**: `bin/run.ps1` runs the same steps as `bin/run.sh`, with the same release file names plus `.exe`.

## Not on Windows yet

- **Voice calls** are off on Windows: a call says so in one line.
- **Remote machines** and **the web board** are not part of this program on any platform.
- **WSL** is not a target. The Windows build is native.
- **ARM Windows** is not built. The release file is `herdr-projects-windows-x86_64.exe`.
- **Links** need Developer Mode or an administrator (error 1314). Two tests that make links are gated on Windows until
  then.
