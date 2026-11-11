//! The program a repo's pull requests go through (`repo.cli`): gh for GitHub, or bb, a command with gh's commands for a
//! Bitbucket Server. bb is asked only what the two share (`pr …`, `repo view`): GitHub's own commands (`api`, `run`,
//! `issue`) are refused before anything runs, the repo is named as Bitbucket names it (`PROJ/repo`), and a pull
//! request link of another Bitbucket repo is not acted on. gh's own command is git.rs's.
use crate::json::*;
use crate::store::repos::{name_of, primary_root};
use crate::store::settings::lookup::{lookup, Scope};
use crate::store::settings::places::{repo_file, yours};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cli { Gh, Bb }
impl Cli { pub fn name(self) -> &'static str { match self { Cli::Gh => "gh", Cli::Bb => "bb" } } }

/// The program for a call made in `cwd`: `repo.cli` of the repo the folder belongs to (your entry for it, else its own
/// file; a repo the board does not know yet by its file alone), gh where nothing says bb.
pub fn of(cwd: &Path) -> Cli {
    let said = match name_of(cwd) {
        Some(n) => lookup("repo.cli", &Scope::repo(&n), &yours()).map(|f| text(&f.value)).unwrap_or_default(),
        None => repo_file(&primary_root(cwd), None).0.t("repo.cli"),
    };
    if said == "bb" { Cli::Bb } else { Cli::Gh }
}
/// `gh` or `bb`, for the words that tell someone what to run.
pub fn name(cwd: &Path) -> &'static str { of(cwd).name() }

/// A Bitbucket Server repo as bb names it (`PROJ/repo`, or `~user/repo` for a personal one), and the part of its pull
/// requests' links that names it (`/projects/proj/repos/repo/pull-requests/`, `/users/user/repos/…` for a personal one).
pub struct Target { pub repo: String, pub pulls: String }
/// The repo an origin URL names: ssh (`ssh://git@host:7999/PROJ/repo.git`) or https, where Bitbucket puts `scm/` before
/// the project (`https://host/scm/PROJ/repo.git`). None for a URL that names no repo on a host.
pub fn target_of(url: &str) -> Option<Target> {
    let u = url.trim();
    let u = match u.split_once("://") {
        Some((scheme, rest)) if !scheme.eq_ignore_ascii_case("ssh") => match rest.split_once('/') { Some((host, path)) => format!("{scheme}://{host}/{}", path.strip_prefix("scm/").unwrap_or(path)), None => u.to_string() },
        _ => u.to_string(),
    };
    let r = crate::install::remote_of(&u)?;
    let (owner, name) = r.repo.split_once('/')?;
    let space = match owner.strip_prefix('~') { Some(user) => format!("users/{user}"), None => format!("projects/{owner}") };
    Some(Target { pulls: format!("/{space}/repos/{name}/pull-requests/").to_lowercase(), repo: r.repo })
}
/// What only GitHub has: its REST API, its Actions runs, its issues.
const GITHUB_ONLY: [&str; 3] = ["api", "run", "issue"];
/// One argument that is a Bitbucket pull request's link: a single http(s) word with `/pull-requests/` in it.
fn is_link(a: &str) -> bool { (a.starts_with("https://") || a.starts_with("http://")) && a.contains("/pull-requests/") && !a.contains(char::is_whitespace) }
/// The command that runs bb for a call made in `cwd`, or why the call is not made. gh's GH_REPO is not bb's: the repo
/// is named on the commands that take it (`pr create --repo`), and a link the call names must be of the origin's repo.
pub fn bb_command(args: &[&str], cwd: &Path) -> Result<Command, String> {
    if let Some(what) = args.first().filter(|a| GITHUB_ONLY.contains(a)) {
        return Err(format!("`bb {what}` is not asked: `{what}` is GitHub's own, and this repo's pull requests go through bb (repo.cli)."));
    }
    let mut cmd = Command::new("bb");
    cmd.args(args).current_dir(cwd).env("GH_PROMPT_DISABLED", "1").env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let links: Vec<&str> = args.iter().copied().filter(|a| is_link(a)).collect();
    match super::git::origin_url(cwd) {
        Err(why) => return Err(format!("The clone's origin could not be read ({why}), so bb is not asked.")),
        Ok(None) => { if let Some(l) = links.first() { return Err(format!("{l} is a pull request, and this clone has no origin to check it against: bb is not asked.")); } }
        Ok(Some(url)) => {
            let Some(t) = target_of(&url) else {
                return Err("The clone's origin is not a Bitbucket repo (ssh://git@host:7999/PROJ/repo.git or https://host/scm/PROJ/repo.git), so bb is not asked.".into());
            };
            if let Some(l) = links.iter().find(|l| !l.to_lowercase().contains(&t.pulls)) { return Err(format!("{l} is not a pull request of {}, the clone's origin: the board does not act on it.", t.repo)); }
        }
    }
    Ok(cmd)
}
