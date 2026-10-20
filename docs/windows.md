# Windows

Herdr Projects v1 runs natively on 64-bit Windows with Herdr for Windows. It is a new platform; the existing [install guide](install.md) still describes macOS and Linux and is unchanged until the Windows release ships.

## Requirements

- Herdr 0.9.4 or newer for Windows.
- Git for Windows, including its `sh.exe`/Git Bash tools. Herdr Projects uses this shell for checks and other user-configured commands.
- PowerShell 7. Herdr starts pane commands through its configured default shell; PowerShell must be available. Windows PowerShell 5.1 is not tested in v1.
- An installed agent CLI. Windows v1 targets Claude Code, pi and omp. See [agents](agents.md) for agent-specific setup.

## Set Herdr's pane shell

Set `default_shell` in Herdr's Windows configuration to the PowerShell executable. For PowerShell 7, for example:

```toml
default_shell = "C:\\Program Files\\PowerShell\\7\\pwsh.exe"
```

If you use Windows PowerShell 5.1, use its actual executable path instead, commonly `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`. Restart or reload Herdr after changing its configuration. Without an explicit default shell, Herdr can fall back to `$SHELL` or `/bin/sh`, neither of which is a suitable native Windows pane shell.

## Launcher and PATH

The board places a copy of `herdr-projects.exe` under `%USERPROFILE%\\.herdr-projects\\data\\bin` when it opens. Herdr and its agents invoke that launcher by its full path, so adding it to PATH is not required for the plugin itself. To run `herdr-projects` by name in PowerShell, add this folder to your user PATH:

```powershell
$launcherDir = Join-Path $env:USERPROFILE '.herdr-projects\data\bin'
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
[Environment]::SetEnvironmentVariable('Path', "$launcherDir;$userPath", 'User')
```

Open a new PowerShell window and check it with `herdr-projects doctor`.

## Windows v1 scope

Windows v1 targets Claude Code, pi and omp. Codex is not tested. SSH machines, voice calls and plan approval are off in Windows v1; plan approval needs a read-only worktree guard, which this release does not provide. Windows does not turn that guard into a weaker promise.

Stopping programs before a task's temporary folder is removed does not work on Windows in v1. The board cannot list the programs that run from a folder there, so it stops none, and a folder one of them still has open stays until that program ends.

The default reviewer remains `codex`, but Codex is not installed in the Windows validation environment. `herdr-projects doctor` warns when the configured reviewer is missing. Choose an installed reviewer with `reviewer.agent` in the board settings (for example, Claude Code, pi or omp), or install Codex before relying on the default. A review can otherwise wait or use a configured reviewer fallback.

## Tests not run on Windows

These tests are gated with `#[cfg(unix)]`; their reasons are specific platform facilities or features outside Windows v1:

- `tests/board_still_rest.rs` and `tests/real_terminal.rs`: require a pseudo-terminal (`openpty`); ConPTY is not included in v1.
- `tests/board_size_held.rs`: requires signals and a pseudo-terminal.
- The pseudo-terminal portion of `tests/take_over_more.rs`: requires a pseudo-terminal; its other portions run on Windows.
- `tests/cleanup_retry.rs`: connects processes through UnixStream pairs.
- `tests/detached_jobs.rs`: uses `setsid` and `waitpid`; Windows has a separate named-job test.
- `tests/far/mod.rs`, `tests/remote.rs` and `tests/remote_why.rs`: exercise SSH machines, which are not available in v1.
- `tests/plan_sandbox.rs`: exercises the Linux and macOS sandbox tools.
- `tests/linking.rs`: uses Linux and macOS link flags.
- `tests/lifecycle_popups.rs`: uses Unix-only configuration links.
- `tests/settings_move.rs`: uses libc account lookup on Unix.
- `tests/board_settings_yours.rs` (two tests) and `tests/board_coordinators_inks.rs` (one test): show other machines, which are not available in v1.
- `tests/plan_steps.rs` (two tests), `tests/plan_untouched.rs` and `tests/disk_space_kept.rs` (one test each): plan approval, which needs a read-only mount that Windows v1 does not have. `tests/plan_modes.rs` tests the refusal on Windows instead.
- `tests/disk_space_sweep.rs` (one test): stops the programs that run from a task's temporary folder, which Windows v1 cannot list (see [Windows v1 scope](#windows-v1-scope)).
