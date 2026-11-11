# Bitbucket through bb

A repo whose pull requests live on a Bitbucket Server (or Data Center) goes through `bb`, a command that takes gh's
commands, instead of `gh`. Every other repo stays on gh, as before. The choice is one repo's own: `repo.cli`, `gh` or
`bb`.

```sh
herdr-wingman set repo.cli bb --repo <name>      # your entry for the repo
```

or in the repo's own `.herdr-projects.json`, so that everyone who works on it gets the same:

```json
{ "repo": { "cli": "bb" } }
```

Your entry for the repo wins over its file. `herdr-wingman show --repo <name>` says which one counts.

## What the board asks of bb

The same as of gh, word for word, and nothing more. bb must take these commands and answer as gh does: the same flags,
the same `--json` field names, the same states (`OPEN`, `MERGED`, `CLOSED`).

| When | The command |
|---|---|
| The finish line opens the pull request | `bb pr create --draft --head <branch> --title <title> --body <text> [--base <branch>] --repo PROJ/repo`: its link is the first `https://…` word it prints |
| An open one of the branch is looked for | `bb pr list --head <branch> --state all --limit 10 --json number,url,state,baseRefName` |
| It is watched, and looked at before it merges | `bb pr view <link> --json state,reviewDecision,author,comments,reviews,headRefOid,mergeCommit,statusCheckRollup` (also `baseRefName`, `body`) |
| It merges, on your yes | `bb pr ready <link>`, then `bb pr merge <link> --squash` (or `--merge`, `--rebase`, as `merge.method` says) `--match-head-commit <the reviewed commit>` |
| The review is posted on it | `bb pr comment <link> --body <text>` |
| Its body gets a `Closes #N` line for each issue of `merge.closes` (and the lines of an issue no longer named go) | `bb pr edit <link> --body <text>` |
| The repo is archived? | `bb repo view PROJ/repo --json isArchived --jq .isArchived` (an error counts as not archived) |

The repo is named as Bitbucket names it: `PROJ/repo` from `ssh://git@host:7999/PROJ/repo.git`, and from an https
clone, `https://host/scm/PROJ/repo.git`, the same. A personal repo is `~user/repo`. GitHub's `GH_REPO` is not set
for bb.

## What is different from gh

- **GitHub's own commands are never asked of bb**: `api`, `run` and `issue` are refused before anything runs, with
  the reason. So:
  - only your own comments wake the agent, as with gh, but "you" is the pull request's author (gh asks GitHub who is
    logged in; bb is not asked);
  - a failed check's log is not read: the agent gets the check's name and its link;
  - `merge.closes` closes no issue after the merge (Bitbucket has none), and the task's line says so: its `Closes #N`
    lines are still written on the pull request.
- **No GitHub Actions workflow is waited for.** A `.github/workflows` folder in the repo is not read: Bitbucket does
  not run it. `check.again` (the shorter check of a fix round) is never used, as it needs GitHub's checks.
- **Only the checks bb lists count.** With `merge.checks github` the merge waits for every check bb lists on the pull
  request (`statusCheckRollup`), and for those it listed before. One your CI has not reported yet is not waited for:
  GitHub can be asked whether an outside CI reports on the repo, bb cannot. Where your CI is slow to report, approve
  once its build has started. `merge.checks none` does not look at them.
- **A pull request link of another repo is not acted on.** A link the board would pass to bb must be of the origin's
  repo (`…/projects/PROJ/repos/repo/pull-requests/42`), as with gh.
- **What the board tells the agent or you to run names bb**: `bb pr checks`, `bb pr view --comments`,
  `bb pr edit <number> --base …`, `bb pr create:` and `bb pr merge:` above what bb said.
- The row's `PR#42` is read from the link's number, also from a Bitbucket link ending in `/overview`.

## When something fails

- **`bb` is not found**: it must be on the PATH the board's processes get. On Windows the board starts `bb.exe`: a
  `bb.cmd` or `bb.ps1` alone is not found.
- **"The clone's origin is not a Bitbucket repo"**: the origin's URL names no `PROJ/repo` on a host (a local folder,
  or a path the board does not read). Nothing is pushed or opened.
- **"`bb api` is not asked"**: something asked GitHub's own command of a bb repo. It is said on the task; nothing ran.
- **The pull request could not be opened**: the task shows what bb said, under `bb pr create:`. Fix it (a login, a
  branch Bitbucket refuses), then finish again: the board opens it then.

Not tried against a real Bitbucket Server yet: the board's tests answer for bb as its fake gh answers for gh
(`tests/pull_requests_bb.rs`).
