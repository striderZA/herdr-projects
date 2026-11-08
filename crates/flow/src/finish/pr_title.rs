//! The subject of a task's pull request and of the commits the board makes for it: conventional commit style
//! (`fix: the login page`), with the task's own key taken out, since the body's `Task:` line names it. The type is the
//! kind the branch name takes (branch_kind.rs).
use crate::store::keys::Keys;
use crate::store::settings::branch_kind::{of, KINDS};

/// The subject of a task's pull request and its commits: `type: description`. The description is the task's own title
/// with its repo's key and its id taken out, and its first word lower-cased when that is a plain capitalised word. A
/// title that is only a key gives the id.
pub fn of_task(id: &str, repo: &str, title: &str, keys: &Keys) -> String {
    let own = keys.get(repo).map(|k| k.to_uppercase());
    let text = title.split_whitespace().filter(|w| !is_key_or_id(w, own.as_deref(), id)).collect::<Vec<_>>().join(" ");
    if text.is_empty() { return id.to_string(); }
    if let Some((head, rest)) = prefixed(&text) { return format!("{head}: {}", lower_first(rest)); }
    let kind = of(&text);
    // A kind word that leads the title is the type now, so it is not said twice.
    let (first, after) = text.split_once(' ').unwrap_or((text.as_str(), ""));
    let word = first.trim_matches(|c: char| !c.is_ascii_alphanumeric()).to_lowercase();
    let rest = if word == kind && !after.is_empty() { after } else { text.as_str() };
    format!("{kind}: {}", lower_first(rest))
}
/// A word that is the task's own key with its number (`PRJ-134`, `prj-7b`), or the task's id (`proj/134`).
fn is_key_or_id(w: &str, own: Option<&str>, id: &str) -> bool {
    let core = w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '/');
    if core.eq_ignore_ascii_case(id) { return true; }
    let Some(k) = own else { return false };
    let up = core.to_uppercase();
    up.strip_prefix(k).and_then(|r| r.strip_prefix('-')).is_some_and(|n| n.starts_with(|c: char| c.is_ascii_digit()) && n.bytes().all(|b| b.is_ascii_alphanumeric()))
}
/// A conventional prefix the title already has (`fix:`, `Feat(ui)!:`): its head in lower case, and what follows it.
fn prefixed(text: &str) -> Option<(String, &str)> {
    let (head, rest) = text.split_once(':')?;
    let head = head.trim();
    if head.contains(' ') { return None; }
    let kind = head.split(['(', '!']).next()?.to_lowercase();
    KINDS.contains(&kind.as_str()).then(|| (head.to_lowercase(), rest.trim()))
}
/// The first word lower-cased when it is a plain capitalised word (`Login` to `login`, `A` to `a`). A name or an acronym
/// (`GitHub`, `PR`) stays as it is.
fn lower_first(s: &str) -> String {
    let (w, tail) = s.split_once(' ').unwrap_or((s, ""));
    let mut cs = w.chars();
    let Some(c0) = cs.next() else { return s.to_string() };
    let plain = c0.is_uppercase() && cs.clone().all(|c| !c.is_uppercase()) && (w.chars().count() > 1 || w == "A");
    if !plain { return s.to_string(); }
    let low: String = c0.to_lowercase().chain(cs).collect();
    if tail.is_empty() { low } else { format!("{low} {tail}") }
}
