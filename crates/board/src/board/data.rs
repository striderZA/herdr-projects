//! What the board reads, read away from the keyboard: by the loader, once a second and after anything you did, on a
//! thread of its own. The board's state takes it in as one event, so a key press never waits for a file, herdr or git.
//! A task the board looks closer at (the selected one, the one the chat talks to) has a `snap` of what only a closer
//! look gives: its settings and where each comes from, its conversation, its screen, its plan.
use crate::agents::screen_text::screen_text;
use crate::coordinator::look::{look, screen, Look};
use crate::flow::{live::board, talk::task_talk, task::is_agent_task};
use crate::herdr::cli::{current_socket, session_name};
use crate::json::*;
use crate::shown::states::as_row;
use crate::store::{automations::automations, keys::{repo_keys, Keys}, marks::tilde, repos::{branch_of, can_worktree, folder_of, known_repos, repo_of}, settings::{lookup::{in_repo, setting, Scope}, show::{problems, problems_in, rows}}};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Data {
    /// This machine's tasks, archived ones too, as rows.
    pub items: Vec<crate::board::state::Rec>,
    /// This machine's automations, each with what its row and panel show (`shown`).
    pub autos: Vec<Obj>,
    pub coord: Look, pub coord_screen: String, pub can_dispatch: bool,
    /// A repo's coordinator the chat talks to: its level ("project:<repo>"), how it stands, and its screen
    /// (update/coords.rs).
    pub sub: Option<(String, Look, String)>,
    /// How this machine's other coordinators stand, by level, as far as the board wants them (update/coords.rs:
    /// looks_wanted): the marks on projects' rows, the selected one's panel.
    pub looks: BTreeMap<String, Look>,
    /// This machine's project coordinators that were ever talked to, by level ("project:<repo>"): one folder read.
    pub coords: Vec<String>,
    /// This machine's coordinators' autopilot as each shows it, by level ("" the main one, "project:<repo>").
    pub pilots: Value,
    pub keys: Keys, pub repos: Vec<String>, pub roots: BTreeMap<String, String>, pub session: String, pub notify: String,
    /// The known repos that keep their heading by repo with no task in view (`used_lately`).
    pub lately: Vec<String>,
    /// This machine's archived projects (store/archived_repos.rs): off the board, each a heading in the archive.
    pub archived: Vec<String>,
    /// Which of this machine's projects are pinned (store/pins.rs): a mark on their rows, and the Pinned filter.
    pub pins: crate::store::pins::Pins,
    /// The repo the board is in (where it was opened, or where a draft was typed), its folder, and the tasks' file.
    pub repo: String, pub repo_root: String,
    /// This machine's name, for its row on the board.
    pub host: String,
    /// herdr's approve action asked this board to ask about a task; a line the board key left for this board.
    pub approve: String, pub heard: Option<(String, String)>,
    /// Your settings ("") or your entry for a repo, while the settings page shows them: which, and its rows and problems.
    pub mine: Option<(String, Value)>,
    /// The skills the text box offers while a draft could start one (skills.rs): whose (`Wants::skills`), and the list.
    pub skills: Option<(String, Vec<crate::agents::skills::Skill>)>,
    pub loaded: bool,
    /// Which read this is (`reads_started`), counted from the program's start.
    pub seq: u64,
}
/// What the board wants a closer look at. `mine`: your settings (`Some("")`) or your entry for a repo, for the
/// settings page. `skills`: whose skills the text box offers ("" none: update/skills.rs).
#[derive(Clone, Debug, Default)]
pub struct Wants { pub ids: Vec<String>, pub cwd: String, pub own_pane: String, pub coord: String, pub mine: Option<String>, pub looks: Vec<String>, pub skills: String }

/// The repo the board is in, worked out once for each folder: a busy lock keeps what was known.
pub fn repo_here(cwd: &str) -> (String, String) {
    static HERE: std::sync::Mutex<Option<(String, (String, String))>> = std::sync::Mutex::new(None);
    let mut h = HERE.lock().unwrap();
    if let Some((c, r)) = h.as_ref() { if c == cwd && !r.0.is_empty() { return r.clone(); } }
    let r = repo_of(std::path::Path::new(cwd)).map(|(n, p)| (n, p.display().to_string())).unwrap_or_else(|_| h.as_ref().map(|x| x.1.clone()).unwrap_or_default());
    *h = Some((cwd.to_string(), r.clone()));
    r
}
static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// How many reads of the tasks have started: a read numbered higher than this began after now, so it sees what was
/// just done.
pub fn reads_started() -> u64 { READS.load(std::sync::atomic::Ordering::SeqCst) }
/// One read of everything. `wants`: the tasks to look at closer.
pub fn load(wants: &Wants) -> Data {
    let seq = READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let socket = current_socket();
    let mut items: Vec<Obj> = board(true).iter().map(as_row).collect();
    for r in items.iter_mut() { if wants.ids.iter().any(|w| w == r.s("id")) { let s = snap(r); r.put("snap", s); } }
    let now = crate::clock::now();
    let roots: BTreeMap<String, String> = known_repos().iter().map(|(k, v)| (k.clone(), text(v))).collect();
    let archived: Vec<String> = crate::store::archived_repos::archived_repos().keys().cloned().collect();
    // An archived project keeps no row: not for its last use, its coordinator or its automations.
    let lately = used_lately(&items, &roots, &socket, now).into_iter().filter(|r| !archived.contains(r)).collect();
    let items = items.into_iter().map(std::sync::Arc::new).collect();
    let autos: Vec<Obj> = automations().into_iter().filter(|a| !archived.iter().any(|r| r == a.s("repo"))).map(|a| shown_auto(a, now)).collect();
    let coord = look(&socket);
    let coord_screen = if !coord.readable && coord.status.is_some() { screen_text(&screen(&coord, &socket)) } else { String::new() };
    let sub = (!wants.coord.is_empty()).then(|| { let co = crate::coordinator::co::Co::of(&socket, &wants.coord); let v = look(&co); let sc = if !v.readable && v.status.is_some() { screen_text(&screen(&v, &co)) } else { String::new() }; (wants.coord.clone(), v, sc) });
    let here = repo_here(&wants.cwd);
    let looks = looks(&socket, &wants.looks);
    // A project's whose repo folder is gone has no row: as a repo's heading, it would lead nowhere.
    let coords = crate::coordinator::state::all(&socket).into_iter().filter(|c| !c.is_top() && c.repo().is_none_or(|r| roots.get(r).is_some_and(|f| std::path::Path::new(f).is_dir()) && !archived.iter().any(|a| a == r))).map(|c| c.to()).collect();
    let skills = crate::board::skills::read(&wants.skills, &socket, &coord, sub.as_ref());
    Data {
        items, autos, coord, coord_screen, sub, looks, coords, can_dispatch: crate::coordinator::who::ready(&socket), keys: repo_keys(), repos: roots.keys().cloned().collect(), roots, lately, archived, pins: crate::store::pins::pins(),
        session: session_name(&socket), notify: setting("board.notify", None).as_str().unwrap_or("").to_string(),
        repo: here.0, repo_root: here.1, host: crate::util::hostname(),
        approve: crate::actions::finish_line::take_approve(&socket, &wants.own_pane), heard: crate::board::run::say::heard(&wants.own_pane, &socket),
        mine: wants.mine.as_ref().map(|r| (r.clone(), mine(r))), skills, pilots: crate::autopilot::shown::views(&socket, now), loaded: true, seq,
    }
}
/// How this machine's coordinators of these levels stand ("*": every one that was ever talked to).
fn looks(socket: &str, wanted: &[String]) -> BTreeMap<String, Look> {
    let mut levels: Vec<String> = wanted.iter().filter(|w| *w != "*").cloned().collect();
    if wanted.iter().any(|w| w == "*") { levels.extend(crate::coordinator::state::all(socket).into_iter().filter(|c| !c.is_top()).map(|c| c.to())); }
    levels.sort(); levels.dedup();
    levels.into_iter().filter(|l| !l.is_empty()).map(|l| { let v = look(crate::coordinator::co::Co::of(socket, &l)); (l, v) }).collect()
}
/// How long a repo with no task in view keeps its heading by repo after its last use.
pub const LATELY_DAYS: i64 = 14;
/// The known repos of this machine whose heading by repo shows even with no task in view, so that space there still
/// talks to its project coordinator: those whose folder is still there, and that have a project coordinator (running
/// or saved), or a task (archived ones too) that changed in the last `LATELY_DAYS`. The many old ones a machine
/// collects stay off the list.
pub fn used_lately(items: &[Obj], roots: &BTreeMap<String, String>, socket: &str, now: i64) -> Vec<String> {
    let since = now - LATELY_DAYS * 24 * 3600 * 1000;
    let coordinated: Vec<String> = crate::coordinator::state::all(socket).iter().filter_map(|c| c.repo().map(String::from)).collect();
    let used = |t: &Obj| ["at", "since", "created", "archivedAt"].iter().map(|k| t.i(k)).max().unwrap_or(0) >= since;
    roots.iter().filter(|(name, folder)| std::path::Path::new(folder.as_str()).is_dir()
        && (coordinated.contains(name) || items.iter().any(|t| t.s("id").split('/').next() == Some(name.as_str()) && used(t))))
        .map(|(name, _)| name.clone()).collect()
}
/// An automation with what its row and its panel show, read here: its firings, newest first, its own settings, every
/// work setting its next task will have, whether its tasks are research only.
pub fn shown_auto(a: Obj, now: i64) -> Obj {
    use crate::automations::{list as l, show::{firings, row_of}};
    let rows = l::setting_rows(&a);
    let mut o = a.clone();
    let fired = firings(&a);
    let oldest_first: Vec<Value> = fired.iter().rev().cloned().collect();
    o.put("research", l::research_only(&a));
    o.put("chat", l::chat_only(&a));
    o.put("shown", json!({ "firings": fired.into_iter().take(20).collect::<Vec<_>>(), "own": rows.iter().filter(|x| x.own).map(|x| json!({ "name": x.name, "value": x.value, "own": true })).collect::<Vec<_>>(),
        "settings": rows.iter().map(|x| x.json()).collect::<Vec<_>>() }));
    o.put("rowNow", serde_json::to_value(row_of(&a, now, Some(&oldest_first))).unwrap_or(Value::Null));
    o
}
/// Your settings (`repo` "") or your entry for a repo, as `show` and `show --repo` read them: every row, and what is
/// not counted.
pub fn mine(repo: &str) -> Value {
    if let Some(rest) = repo.strip_prefix("coord:") { return coord_rows(rest); }
    let scope = if repo.is_empty() { Scope::you() } else { Scope::repo(repo) };
    json!({ "settings": rows(&scope).iter().map(|r| r.json()).collect::<Vec<_>>(), "problems": problems_in(&scope) })
}
/// A coordinator's settings page (`key\tthen`, update/edit_coord.rs): its own rows (`own` for its own and this time's,
/// `once` for this time's), whether autopilot is on, and the rows of the place under them (yours, your entry for a repo).
fn coord_rows(spec: &str) -> Value {
    let (key, then) = spec.split_once('\t').unwrap_or((spec, ""));
    let co = crate::coordinator::co::Co::of(&current_socket(), if key == "main" { "" } else { key });
    let scope = crate::autopilot::record::scope_of(&co);
    let own: Vec<Value> = rows(&scope).iter().map(|r| { let mut j = r.json(); j["own"] = json!(r.place == "coord" || r.place == "once"); j["once"] = json!(r.place == "once"); j }).collect();
    let under = match then { "you" => mine(""), t => t.strip_prefix("repo:").map(mine).unwrap_or(json!({ "settings": [], "problems": [] })) };
    let mut problems = problems_in(&scope); problems.extend(strings(under.at("problems")));
    json!({ "coord": own, "settings": under.at("settings"), "problems": problems, "on": crate::autopilot::record::is_on(&co) })
}
/// A closer look at one task of this machine.
pub fn snap(t: &Obj) -> Value {
    let folder = folder_of(t.s("id"));
    let tree = if t.b("worktree") && std::path::Path::new(t.s("worktree")).exists() { t.t("worktree") } else { String::new() };
    let live = is_agent_task(t) && !t.b("archivedAt");
    let talk = if live { task_talk(t, 12, 20000) } else { Value::Null };
    // A program whose conversation cannot be read: the words on its screen instead.
    let screen = if live && !talk.b("readable") { crate::board::screens::screen_of(t) } else { String::new() };
    let plan = if t.at("plan").b("at") { crate::plan::read_plan(t) } else { String::new() };
    let at = if tree.is_empty() { folder.clone() } else { tree.clone().into() };
    json!({ "talk": talk, "screen": screen, "plan": plan, "landsOn": if t.s("stage") == "approval" { json!(crate::sync::lands_on(t)) } else { Value::Null },
        "folder": tilde(&folder.display().to_string()), "root": folder.display().to_string(), "tree": tilde(&tree), "treePath": tree, "branch": branch_of(&at),
        "settings": rows(&Scope::task(t)).iter().map(|r| r.json()).collect::<Vec<_>>(), "problems": problems(Some(t)), "canWorktree": can_worktree(&folder), "repo": in_repo(t),
        "worktreeGone": t.b("worktree") && tree.is_empty() })
}
