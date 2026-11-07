# Installing, the first run, and taking it out again

**Install with one command, then open the board once.** The short version is [get started](get-started.md). This page
has the details. Something not working: [troubleshooting](troubleshooting.md).

```sh
herdr plugin install eliasstravik/herdr-projects
```

## What you need

- Linux, on x86_64 or aarch64, or Windows on x86_64 (see [Windows](#windows) below).
- Herdr 0.9.3 or newer (0.9.4 on Windows). Your **running** Herdr server must be 0.9.3 or newer too (0.9.4 on Windows): an older one refuses the plugin until
  it is restarted.
- `git`, and `curl` or `wget` to download the program.
- Optional: `gh` (pull requests).

No Node, no npm: the board is one program.

## What the install does

Herdr clones the repo's main branch into its own plugins folder (`herdr plugin list` shows where) and runs the plugin's
build step there, which downloads the program. `--ref v1.2.29` takes a given release.

To keep a checkout of your own instead:

```sh
git clone https://github.com/eliasstravik/herdr-projects && cd herdr-projects
herdr plugin link "$PWD"          # hooks, the board's tab and the actions
sh bin/run.sh install             # gets the program, writes its launcher, says how to put it on your PATH
```

On Windows, run `powershell -File bin\run.ps1 install` in the checkout instead.

## The first run

Open the board: the plugin's action "Open the work board", or type `herdr-projects` in any pane. The first time it
opens inside Herdr, it shows one screen with two things it can add to your Herdr `config.toml`. Each is your choice:

| Key | Does |
|---|---|
| `1`, `2` | switch each one on or off |
| `k` | pick another key |
| `enter` | save |
| `esc` | ask again next time |

The file is backed up first, each block sits between two marker lines, and a change Herdr rejects is rolled back.

**1. A key that opens the board.** It offers `prefix+a`, which Herdr does not use itself.

- If your config already binds `prefix+a`, it offers the next free key (`prefix+y`, then `prefix+u`, `prefix+i`,
  `prefix+m`) and says why.
- A key that Herdr binds by default, or that your config already binds, is never taken. The screen refuses it and says
  what the key does today (`prefix+w`, for one, is Herdr's own workspace picker). This holds for the offered key and for
  one you type with `k`, however your config writes a binding (one key, a list of keys, the older `[keys.indexed]`).
- A key already set up for the board is left as it is, however your config writes it (`[[keys.command]]`, or
  `command = [{…}]` under `[keys]`).
- The key opens the board in a tab of its own. To open it as a popup over Herdr's window instead (and close it again
  with the same key): `herdr-projects set board.open popup` ([the board](board.md)). The key itself stays.

**2. Sidebar rows**, so an agent's entry in Herdr's sidebar is a small copy of its task's row on the board
([Herdr's sidebar](sidebar.md)):

- the task's number and title;
- who is on it and where it stands, with the review round and pull request (`agent · working · R#1 PR#113`);
- the percent and time to go (`8% 16m`).

Without them, Herdr shows the first two, with the tab's name (`Agent`) after the title, and not the third: its default
sidebar shows no custom value. Rows an older version of the plugin wrote are updated by themselves, between the same
marker lines. Rows of your own are never touched: when your config already sets `ui.sidebar`, however it writes it
(`[ui.sidebar.agents]`, `sidebar = {…}` under `[ui]`, or a dotted key), the plugin's rows are not offered. The first
run says in one line that yours are kept and shows the plugin's rows, to merge into yours by hand (in a small window,
`↑` `↓` and `pgup` `pgdn` scroll them; any other key goes on).

Nothing else is needed: no Herdr integration, no hooks in your own agent settings.

### Adding the blocks by hand

```toml
[[keys.command]]
key = "prefix+a"
type = "plugin_action"
command = "herdr-projects.open"

[ui.sidebar.agents]
rows = [["state_icon", "machine", "workspace", { token = "tab", rules = [{ equals = "Agent", hide = true }, { starts_with = "Reviewer #", hide = true }] }], ["agent"], [{ token = "$summary", dim = true }, "$progress"]]

[ui.sidebar.spaces]
rows = [["state_icon", "workspace"], ["branch", "git_status"], [{ token = "$summary", dim = true }, "$progress"]]
```

On Windows, use `command = "herdr-projects.open-windows"` instead. `herdr-projects.open` runs on Linux only, and
Herdr refuses it on Windows.

**Careful with the key.** A `[[keys.command]]` beats Herdr's own binding of the same key, with no warning. Written by
hand on a key Herdr uses, it switches Herdr's action off. (An early version of this page showed `prefix+w` here, which
took away Herdr's workspace picker.) `herdr --default-config` lists most of the keys Herdr binds (not copy mode,
`prefix+[`, nor swapping panes, `prefix+shift+h` `j` `k` `l`). To move the board's key, change the `key` line and run
`herdr server reload-config`.

Optionally bind `herdr-projects.approve-current` too (`herdr-projects.approve-current-windows` on Windows). For the task that owns the focused pane, it opens the board on
that task with the merge question up (`y` = yes, `n` or `esc` = no). Anywhere else it does nothing.

## The command on your PATH

Agents and hooks are handed the launcher `~/.herdr-projects/data/bin/herdr-projects`, which runs the program.

- The board writes it when it opens. With `herdr plugin install`, open the board first, with the plugin's action "Open
  the work board".
- `herdr-projects install` writes it again, and says if something else named `herdr-projects` comes first on your PATH.
- **On Windows** the folder holds `herdr-projects.cmd`, which cmd and PowerShell type by name and agents and hooks run, and a shell script `herdr-projects` that Git Bash types by name. Put the folder on your PATH. A `herdr-projects.exe` an older install left there is removed, since PowerShell would run it first.
- To type `herdr-projects` in a shell, put that folder on your PATH (in `~/.profile` or your shell's start-up file).


## Updating

```sh
herdr-projects update           # the newest release
herdr-projects update 0.3.1     # a given version
```

- It puts the release in place of the downloaded program, in one step.
- An open board and the keeper see the file change and load it by themselves ([the board's size](board-size.md)).
- Pulling a newer plugin (`git pull`) does the same at the next command Herdr runs: `bin/run.sh` downloads that version.
- A program you built yourself is updated with `git pull` and `cargo build --release`, never by a download.

**The keeper updates itself.** Once the new file has stopped changing (a build still being linked is waited for), with
no board open and no agent running, the keeper starts its successor on the new file, and goes only once the successor
runs. A new file that does not start leaves the old keeper running; it tries again a little later (`keeper.log` in the
data folder says so).

## Building from source

`cargo build --release` in the checkout (Rust: https://rustup.rs). `bin/run.sh` then runs that build, and later builds
are picked up the same way.

## Coming from an older version

- **From the older Node version:** `npm unlink -g herdr-projects` takes the Node program off your PATH. The first run of
  this version keeps a copy of the data folder as it was (`~/.herdr-projects/data-0.2.0`) before it changes anything.
  The data's form is the same, so nothing is converted.
- **From a version that kept its files in `~/.local/state`, `~/.config` and `~/.local/share`:** nothing to do. The first
  run moves them and says so in one line.

Everything the board keeps for you is in one folder, `~/.herdr-projects/`: [the home folder](home-folder.md).

## Taking it out again

**Before unlinking the plugin, run `herdr-projects remove-extras`.** Herdr gives a plugin no uninstall hook, so this is
what takes the marked blocks out of `config.toml` and removes the how-to files ([roles and commands](roles-and-commands.md)).


## Windows

Windows is supported: the plugin's manifest lists it, and every release since 1.2.28 ships a Windows file. [windows.md](windows.md) says what differs there. You need:

- Windows on x86_64.
- Herdr 0.9.4 or newer, running. The board checks herdr's version when it starts and refuses an older one on Windows, naming the version it needs.
- PowerShell 7 (`pwsh.exe`) recommended as herdr's default shell, set by `default_shell` in herdr's `config.toml`. Without it herdr uses Windows PowerShell 5.1.

Install with the same command as on Linux, in PowerShell:

    herdr plugin install eliasstravik/herdr-projects

Herdr runs `bin/run.ps1` on Windows. It runs the program built from this checkout when there is one, and otherwise the release file `herdr-projects-windows-x86_64.exe`, downloaded and checked against its `.sha256`.

## Details: which program runs

The program is one file per platform: `herdr-projects-linux-x86_64`, `-linux-aarch64`, `herdr-projects-windows-x86_64.exe`.
The Linux ones run on any Linux.

On Linux, every command Herdr runs for the plugin goes through `bin/run.sh`, which runs the first of these it finds (on Windows, `bin/run.ps1` runs the same steps):

1. a program built in this checkout (`target/release/herdr-projects`) that no source file is newer than;
2. the downloaded one in `bin/`, when it is this checkout's version (`Cargo.toml`) or newer;
3. that file, downloaded now from the GitHub release of this checkout's version, in the repo the checkout was cloned from
   (its git origin; the upstream repo when there is none): its checksum checked, put in place in one step, one download at a time;
4. failing all that, an older build or an older download, with a line saying so. A download that failed is tried again
   ten minutes later, not on every command (each of Herdr's hooks would wait for it).

With none, it says what to do and stops at once, so Herdr is never held up.

- The plugin's build step (`sh bin/run.sh --fetch`, on Windows `bin/run.ps1 --fetch`) only gets the program, and builds it from source when it cannot
  download it.
- `HERDR_PROJECTS_RELEASES` names another place for the releases (a mirror, a folder as `file:///…`). Without it, the releases of the checkout's git origin are used, so a fork's installs get the fork's files.
- The release files are built for every version tag by `.github/workflows/release.yml`.
