//! The archive key on a project's row (a repo's heading, by repo): the whole project goes to the archive with its
//! tasks, after a yes when one of them works, waits or is to do; in the archive (where an archived project has its
//! heading whatever the grouping), the key on its heading brings it back (actions/archive_project.rs).
use super::ask::confirm;
use super::heads::HEAD;
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::{State, Valid};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::words::repo_label;

/// The selected heading's repo, when it is a repo's heading.
pub fn heading_repo(s: &State) -> Option<String> { s.sel.strip_prefix(HEAD)?.strip_prefix("repo\t").map(String::from) }
/// Whether the repo `name` is archived, as the board last read it.
pub fn archived(s: &State, name: &str) -> bool { s.data.archived.iter().any(|r| r == name) }
/// What the key does on the selected heading, in words for the hint line and the menu ("" nothing).
pub fn says(s: &State) -> &'static str {
    match heading_repo(s) {
        Some(name) if s.archive_view && archived(s, &name) => "restore this project",
        Some(_) if !s.archive_view => "archive",
        _ => "",
    }
}
/// The key on a heading: archive the project (asking first when a task of it works, waits or is to do), or restore
/// it in the archive; on any other heading, say where it works.
pub fn key(s: &mut State, fx: &mut Vec<Effect>) {
    let Some(repo) = heading_repo(s) else {
        let why = if s.archive_view { format!("On an archived project's heading, {} restores it.", key_name("archive")) }
            else { format!("On a project's row, {} archives the project with its tasks. {} groups the list by repo.", key_name("archive"), key_name("byRepo")) };
        return s.flash(why, Ink::Muted);
    };
    let name = repo_label(&repo);
    if s.archive_view {
        if !archived(s, &repo) { return s.flash(format!("{name} is not archived: {} on one of its tasks restores that task.", key_name("archive")), Ink::Muted); }
        return fx.push(Effect::Run(Job::RestoreProject { repo, said: String::new() }));
    }
    let said = format!(" {} shows it; {} there brings it back.", key_name("archiveView"), key_name("archive"));
    let rows: Vec<&Obj> = s.data.items.iter().map(|r| &**r).filter(|r| r.s("id").split('/').next() == Some(repo.as_str())).collect();
    let plan = crate::actions::archive_project::plan_of(&repo, rows);
    if let Some(id) = &plan.busy { return s.flash(format!("{id} is merging: {name} can be archived once that is done."), Ink::Yellow); }
    let job = || Job::ArchiveProject { repo: repo.clone(), said: said.clone() };
    if !plan.asks() { return fx.push(Effect::Run(job())); }
    let n = plan.tasks();
    let q = format!("Archive {name} and its {}?", if n == 1 { "task".to_string() } else { format!("{n} tasks") });
    let detail = format!("{} Its coordinator stops and its automations go off. Nothing on disk or GitHub is deleted.{said}", plan.says());
    confirm(s, q, job(), Valid::Always, vec![("sayYes", "archive it".into()), ("sayNo", "keep it".into())], detail, false);
}
