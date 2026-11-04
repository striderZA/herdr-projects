//! The longer jobs of the board: a setting changed from the settings page, going into an agent's pane or a
//! coordinator's, and the first run's answers.
use super::job::{Done, Then};
use super::state::Whose;
use crate::board::keys::key_name;
use crate::herdr::Herdr;
use crate::json::*;
use crate::shown::ink::Ink;
use crate::store::{settings::{change::{set_value, unset_value}, lookup::Scope, show::rows, values::say}, settings_list::entry_of, tasks::{find, get}};

/// Going into a coordinator of this machine: herdr goes to its pane (started first when it is not running), or, outside
/// a herdr client, this terminal attaches to it.
pub fn coord_open(to: &str, name: &str, popup: bool) -> Done {
    let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), to);
    if !crate::herdr::cli::in_herdr() {
        let v = crate::coordinator::look::look(&co);
        if v.pane.is_empty() || v.status.is_none() { return Done::say(format!("{} is not running: your first message starts it.", crate::board::update::top::sentence(name)), Ink::Yellow); }
        let args = vec![format!("HERDR_SOCKET_PATH={}", co.socket), crate::herdr::cli::bin(), "agent".into(), "attach".into(), v.pane];
        return Done::default().and(Then::Attach { command: "env".into(), args, id: String::new() });
    }
    match crate::coordinator::look::go_in(&co, 30000) { Ok(_) => if popup { Done::default().and(Then::Leave) } else { Done::default() }, Err(e) => Done::fail(e.msg) }
}
/// A setting of one task (or automation) from the settings page: a value, or none to put the row back. Yours and your
/// entry for a repo are changed as `set` and `set --repo` change them, with the same checks.
pub fn set(id: &str, name: &str, value: Option<String>, whose: &Whose) -> crate::err::Res<Done> {
    use crate::automations::list as l;
    if let Whose::Coord(key, _) = whose { return set_coord(name, value, key); }
    if whose.mine() { return set_mine(name, value, whose.repo()); }
    let (value, from, later) = if *whose == Whose::Auto {
        let r = match &value { Some(v) => l::set_setting(id, name, &json!(v))?, None => l::unset_setting(id, name)? };
        let r = r.ok_or_else(|| crate::err::Error::from("that setting is not an automation's"))?;
        (r.value, r.from, String::new())
    } else {
        let t = find(id)?;
        let r = match &value { Some(v) => set_value(name, &json!(v), &Scope::task(&t))?, None => unset_value(name, &Scope::task(&t))? };
        // Research only, on a task under way, says when it counts.
        let later = get(id).and_then(|t| rows(&Scope::task(&t)).into_iter().find(|x| x.name == name)).map(|x| x.later).unwrap_or_default();
        (r.value, r.from, later)
    };
    let label = entry_of(name).map(|e| e.label.to_string()).unwrap_or_else(|| name.to_string());
    let shown = match &value { Value::Bool(true) => "on".to_string(), Value::Bool(false) => "off".to_string(), v => say(v).lines().next().unwrap_or("").to_string() };
    Ok(Done::say(format!("{}{}", crate::store::settings::show::saved_words(&label, &shown, &from), if later.is_empty() { String::new() } else { format!(" · {later}") }), Ink::Green).and(Then::Saved(name.to_string())))
}
/// One of a coordinator's own rows, as `set … --project-coordinator <repo>` sets it.
fn set_coord(name: &str, value: Option<String>, key: &str) -> crate::err::Res<Done> {
    use crate::actions::settings::{settings_command, Where};
    let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), if key == "main" { "" } else { key });
    let cmd = if value.is_some() { "set" } else { "unset" };
    let w = Where { task: None, repo: None, automation: None, coord: Some(key.to_string()) };
    let (j, _) = settings_command(cmd, &[cmd.to_string(), name.to_string()], &w, value.map(|v| json!(v)))?;
    let label = entry_of(name).map(|e| e.label.to_string()).unwrap_or_else(|| name.to_string());
    let running = crate::coordinator::state::get(&co).map(|c| c.b("run")).unwrap_or(false) && name.starts_with("coordinator.");
    Ok(Done::say(format!("{label}: {}, for the {} only{}", say(j.at("value")).lines().next().unwrap_or(""), co.name(), if running { " · from its next fresh chat" } else { "" }), Ink::Green).and(Then::Saved(name.to_string())))
}
fn set_mine(name: &str, value: Option<String>, repo: &str) -> crate::err::Res<Done> {
    use crate::actions::settings::{settings_command, Where};
    let cmd = if value.is_some() { "set" } else { "unset" };
    let w = Where { task: None, repo: (!repo.is_empty()).then(|| repo.to_string()), automation: None, coord: None };
    let (j, _) = settings_command(cmd, &[cmd.to_string(), name.to_string()], &w, value.map(|v| json!(v)))?;
    let label = entry_of(name).map(|e| e.label.to_string()).unwrap_or_else(|| name.to_string());
    let shown = match j.at("value") { Value::Bool(true) => "on".to_string(), Value::Bool(false) => "off".to_string(), v => say(v).lines().next().unwrap_or("").to_string() };
    let note = if j.s("note").is_empty() { String::new() } else { format!(" · {}", j.s("note")) };
    Ok(Done::say(format!("{label}: {shown}{note}"), Ink::Green).and(Then::Saved(name.to_string())))
}
/// Into a task's agent: its real herdr pane. Its space may have been closed (archive closes it): it is reopened from
/// the worktree first. Outside a herdr client, this terminal attaches to that one agent instead.
pub fn open_agent(id: &str, popup: bool) -> Done {
    let Some(t) = get(id) else { return Done::fail(format!("{id} is gone.")) };
    if let Err(e) = crate::flow::live::assert_live(&t) { return Done::fail(e.msg); }
    let api = Herdr::new(Some(t.at("link").t("socket")));
    let (pane, _, _) = match crate::delegate::jobs::reopen(id, false) { Ok(x) => x, Err(e) => return Done::say(e.msg, Ink::Yellow) };
    let has_agent = api.agent_get(pane.s("pane_id")).ok().flatten().is_some();
    if !crate::herdr::cli::in_herdr() {
        if !has_agent { return Done::say(format!("Its agent has exited. {} and a reply starts it again.", key_name("talk")), Ink::Yellow); }
        let args = vec![format!("HERDR_SOCKET_PATH={}", t.at("link").s("socket")), crate::herdr::cli::bin(), "agent".into(), "attach".into(), pane.t("pane_id")];
        return Done::default().and(Then::Attach { command: "env".into(), args, id: id.into() });
    }
    if let Err(e) = crate::delegate::jobs::bring_forward(&api, &pane, has_agent) { return Done::fail(e.message); } // herdr refused: the board stays as it is and says why
    Done::default().and(if popup { Then::Leave } else { Then::Opened(id.into()) })
}
/// The first run's answers, each saved as soon as it is settled, so one that herdr refuses is the only one asked again.
pub fn setup(key: Option<String>, sidebar: Option<bool>, file: &str) -> Done {
    use crate::extras::{add_block, answer};
    let mut said = vec![];
    let mut paste = String::new();
    let mut left = (key.is_some(), sidebar.is_some());
    for what in ["key", "sidebar"] {
        let on = if what == "key" { key.clone() } else { sidebar.map(|b| if b { "on".into() } else { String::new() }) };
        let Some(v) = on else { continue };
        if v.is_empty() {
            answer(if what == "key" { json!({ "key": "no" }) } else { json!({ "sidebarAnswer": "no" }) });
        } else {
            match add_block(what, if what == "key" { &v } else { "" }) {
                Err((msg, None)) => return Done::fail(msg).and(Then::SetupLeft { key: left.0, sidebar: left.1 }), // refused: the screen stays, with the reason
                Err((msg, Some(_))) => { answer(if what == "key" { json!({ "key": "paste" }) } else { json!({ "sidebarAnswer": "paste" }) }); paste = [paste, msg].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n"); }
                Ok(_) => { answer(if what == "key" { json!({ "key": v }) } else { json!({ "sidebarAnswer": "yes" }) }); said.push(if what == "key" { format!("{v} opens the board") } else { "herdr's sidebar shows each task's stage".into() }); }
            }
        }
        if what == "key" { left.0 = false; } else { left.1 = false; }
    }
    answer(json!({}));
    let mut d = if said.is_empty() { Done::say("Nothing was added.", Ink::Muted) } else { Done::say(format!("Added to {}: {}. `herdr-projects remove-extras` undoes it.", crate::store::marks::tilde(file), said.join("; ")), Ink::Green) };
    if !paste.is_empty() { d.then.push(Then::Paste(paste)); }
    d.then.push(Then::SetupLeft { key: false, sidebar: false });
    d
}
