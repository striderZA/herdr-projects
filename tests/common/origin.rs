//! The origin of a test world: a GitHub URL, as the real one is, which git reaches at a folder of the test's own by
//! `insteadOf`. The board names that repo for gh (`o/r`, the fake gh's), and a test makes the origin answer from another
//! folder, or from none, the way a remote that is down does.
use std::path::Path;

/// The origin's URL in every world (`Opts { origin: true }`): the repo `o/r`, the one the fake gh answers for.
pub const ORIGIN_URL: &str = "git@github.com:o/r.git";

/// Makes the origin of the clone at `root` answer from `path` (a folder that is not there: the remote cannot be reached).
pub fn reach(root: &Path, path: &str) { answer(root, "insteadof", path) }
/// The same for pushes only (`pushInsteadOf`): the fetches answer from where `reach` says.
pub fn reach_push(root: &Path, path: &str) { answer(root, "pushinsteadof", path) }
fn answer(root: &Path, kind: &str, path: &str) {
    let url = super::worktrees::git(root, &["config", "--get", "remote.origin.url"]).unwrap_or_else(|e| panic!("origin: {e}"));
    let listed = super::worktrees::git(root, &["config", "--get-regexp", &format!(r"^url\..*\.{kind}$")]).unwrap_or_default();
    for line in listed.lines() {
        // `key value`: an earlier answer for this origin's URL goes, so the one that is left is the one git uses.
        if let Some((key, value)) = line.split_once(' ') {
            if value == url { let _ = super::worktrees::git(root, &["config", "--unset-all", key]); }
        }
    }
    super::worktrees::git(root, &["config", &format!("url.{}.{kind}", path.replace('\\', "/")), &url]).unwrap_or_else(|e| panic!("reach {path}: {e}"));
}
