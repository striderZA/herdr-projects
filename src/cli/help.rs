//! What `herdr-wingman help` prints: every command, in a few words each.
pub const HELP: &str = r#"herdr-wingman: a work board for herdr

  herdr-wingman                     open the board here (in herdr: bind a key to the plugin action "open")
  herdr-wingman add <title> [--desc TEXT] [--repo NAME|PATH] [--delegate] [--after ID[,ID…]] [--<setting> VALUE ...] [--research|--no-research] [--chat|--no-chat]
                                      (--research: research only, short for --research.only true. Its agent changes
                                       nothing in the repo; the result is a report. --no-research: not research only,
                                       where a default says it is. --chat: chat only, short for --chat.only true: a
                                       conversation that changes nothing and ends only when you say: finish <id>.)
  herdr-wingman list [--archived] [--all] [--pinned] [--query TEXT] [--json]
  herdr-wingman report <id> --state idle|working|blocked|done|error [--kind question|permission|auth] [--msg TEXT] [--title TEXT]
                         [--progress 0-100] [--eta MINUTES]   (your own task, with --state working: how far, and minutes left)
  herdr-wingman describe <id> <text>          write or replace a task's description
  (a title is at most 60 characters: a longer one is shortened at a word, its full text kept in the description)
  (an <id> is repo/134; every repo has a short key, made from its name, and HSW-134 names the same task, in any
   letter case. Another key: herdr-wingman set repo.key HSW --repo NAME; the old one still names its tasks)
  herdr-wingman delegate <id> [--after ID[,ID…]|--now] [--<setting> VALUE ...] [--research|--no-research] [--chat|--no-chat]
                                      any work setting, for this task only: --reviewer.agent claude --merge.by itself
                                      (a refused option changes nothing on the task)
                                      --after: start only once those tasks (of any repo here) are done, by itself;
                                      --now: start a task that waits without waiting
  herdr-wingman after <id> [ID[,ID…] | --clear]   what a task waits for: shown, changed, or dropped (it starts then)
  herdr-wingman tell <id> <text>              send a message to the task's agent
  herdr-wingman stop <id>                     stop its agent
  herdr-wingman approve <id> [--anyway]       merge finished work that waits for approval (--anyway: also when its review
                                                or its pull request's checks on GitHub failed);
                                                on a task whose plan is ready (plan.approve you): yes to the plan, the build starts.
                                                tell <id> <text> there sends the plan back with those words
  herdr-wingman pick <id> <letter>            keep one of a task's several attempts (attempts.by); the others' tabs close
  herdr-wingman review <id>                   send the reviewer's findings back to the agent
  herdr-wingman retry <id>                    send the saved error back to the agent; on a task a limit stopped: continue
  herdr-wingman fix <id>                      send failure details to the agent (never approves a merge)
  herdr-wingman finish <id>                   finish anyway, when the agent stopped without a summary or at a limit,
                                                or when the board missed it finishing (its agent at rest, the row still "Finishing");
                                                on a chat-only task: end the conversation, as it is
  herdr-wingman details <id>                  failure details, review, next steps, and what it used against its limits
  herdr-wingman archive|restore|clear <id>
  herdr-wingman pin|unpin <id | project>
                                                pin a task or a project (or unpin it): it gets a yellow pin mark ★ and
                                                shows under the board's Pinned filter
  herdr-wingman archive --done [--repo NAME]  archive every done task of that repo (or of every repo) in one go; one
                                                that works, waits, is to do or is pinned stays, and is counted
  herdr-wingman archive|restore --repo NAME
                                                archive a whole project with its tasks (what works or waits is stopped
                                                first; its coordinator stops, its automations go off), or bring it back
                                                with its tasks as they were. Nothing on disk or GitHub is deleted.
  herdr-wingman set <name> <value> [--repo NAME | --task ID | --automation NAME | --main-coordinator | --project-coordinator REPO]
                                      change a setting: for you, for your entry for one repo, for one task, for an automation,
                                      or for one coordinator (its coordinator.* settings)
  herdr-wingman unset <name> [--repo NAME | --task ID | the same coordinator flags]
                                      back to what it would be without that answer
  herdr-wingman show [<id>] [--repo NAME]     every setting and the place it came from; without an id, your own
                                      (a program setting, such as worker.agent, takes claude, codex, pi or another
                                       program herdr can start, as agy/gemini-3-pro; one that cannot do that role is
                                       refused with the reason)
  herdr-wingman models [PROGRAM]              the model names each program takes, written as a setting writes them
                                      (codex/<model>); a name it does not take is refused when it is set
  herdr-wingman automations [add|show|change|on|off|run|firings|remove] …
                                      tasks made on a schedule or when something happens on the board; each firing adds
                                      a task and puts an agent on it. `herdr-wingman automations help` says how
  herdr-wingman coordinator                   the Coordinator (the agent behind the board's box): who it is, how it stands
  herdr-wingman coordinator tell <text> [--delegate]
                                                say something to it (it is started when it is not running); --delegate is
                                                the board's tab: what it adds from this message gets an agent at once
  herdr-wingman coordinator where             where the human was when they last wrote to it from the board (for the Coordinator itself)
  herdr-wingman coordinator restart           start its program again, in the same conversation
  herdr-wingman coordinator new [--agent KIND[/MODEL]] [--effort E]
                                                a fresh one; with options, that choice is for this one only
  herdr-wingman coordinator stop              close it; the next message starts it again where it left off (words still waiting go in then)
  herdr-wingman coordinator --project REPO …  the same for a repo's project coordinator
                                                its tasks under way, its settings
  herdr-wingman --skill                       instructions an agent can load
  herdr-wingman sweep [--dry]                 remove what ended tasks left behind now: their temporary files, agents'
                                                files for worktrees that are gone, old build output (docs/disk-space.md);
                                                --dry: only say what would go
  herdr-wingman version                       this program's version, and where it is
  herdr-wingman install                       write the launcher in the data folder (agents and hooks run it) for this program
  herdr-wingman doctor                        the health check: the program and its launcher, the plugin in herdr, the
                                                agent programs and their settings, disk space, leftovers
                                                to clean, older installs; each problem with what to do (exits 1 on one)
  herdr-wingman update [VERSION]              this program, in place, from the newest release (or that version)
  herdr-wingman remove-extras                 take out what the plugin added outside its own folders (the how-to
                                                files, its blocks in herdr's config); run it before unlinking
"#;
