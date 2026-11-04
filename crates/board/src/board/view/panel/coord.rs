//! A coordinator in the task panel's form: beside the list (`Alt+P`) and full screen (`Alt+T`). `Status`: where its
//! chat stands, and its latest message. `Settings`: its program, model and effort.
use super::kit::Kit;
use super::READ;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::board::update::coords::{look_of, Target};
use crate::shown::ink::{Ink, Inked};

/// The panel's lines for the coordinator `t`.
pub fn coord_panel(s: &State, t: &Target, width: usize, full: bool) -> Vec<Inked> {
    let mut k = Kit::new(width);
    let look = look_of(s, t);
    let title: Vec<Inked> = k.wrap(&t.name).into_iter().map(|l| l.bold()).collect();
    k.part(title);
    let mut st = vec![k.head("Status", None)];
    let chat = look.as_ref().map(|l| l.state.clone()).unwrap_or_else(|| "none".into());
    let chat_words = match chat.as_str() { "none" => "not started: your first message starts it".to_string(), "exited" => "stopped: the next message starts it again".into(), x => x.to_string() };
    if ["none", "exited"].contains(&chat.as_str()) { st.extend(k.fact("State", vec![Kit::dim(chat_words)])); } else { st.extend(k.fact_text("State", &chat_words)); }
    // Its latest message, unless the chat under the list shows it already.
    let chat_shows = crate::board::update::coords::talking(s).map(|c| c == *t).unwrap_or(t.to.is_empty() && !s.reply);
    if let (Some(l), false) = (&look, chat_shows) { if !l.said.is_empty() { let said = if full { l.said.clone() } else { crate::shown::words::brief(&l.said, 300, " …") }; st.extend(k.fact_text("Last", &said)); } }
    k.part(st);
    let mut set = vec![k.head("Settings", None)];
    let who = look.as_ref().and_then(|l| l.run.clone().or_else(|| l.def.clone()));
    let agent = who.as_ref().map(|w| crate::coordinator::who::describe(Some(w))).unwrap_or_else(|| "its program, as its settings say".into());
    let stale = look.as_ref().filter(|l| l.stale).and_then(|l| l.def.as_ref()).map(|d| format!(" · from its next fresh chat: {}", crate::coordinator::who::describe(Some(d)))).unwrap_or_default();
    set.extend(k.fact_text("Agent", &format!("{agent}{stale}")));
    if full { set.extend(k.wrap(&format!("{} changes them", key_name("settings"))).into_iter().map(|l| l.recolor(Ink::Muted))); }
    k.part(set);
    let mut body = vec![Inked::new()];
    let mut n = 0;
    for p in &k.parts { if p.lines.is_empty() { continue; } if n > 0 { body.push(Inked::new()); } body.extend(p.lines.iter().cloned()); n += 1; }
    if !full { body.push(Inked::new()); body.push(Kit::dim(crate::text::cut(&format!("{} shows it whole", key_name("panel")), k.w))); }
    Kit::out(body)
}
/// The flag a command names this coordinator by on its machine.
pub fn flag(t: &Target) -> String { match t.to.strip_prefix("project:") { Some(r) => format!("--project-coordinator {r}"), None => "--main-coordinator".into() } }
/// The coordinator whole, full screen.
pub fn coord_full(s: &mut State, t: &Target, width: usize, h: usize) -> Vec<Inked> {
    let lines = coord_panel(s, t, width.min(READ), true);
    let mut out: Vec<Inked> = lines.into_iter().take(h).collect();
    while out.len() < h { out.push(Inked::new()); }
    out
}
