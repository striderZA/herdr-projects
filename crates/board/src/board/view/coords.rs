//! The chat with a heading's coordinator (update/coords.rs), drawn as the Coordinator's is but under its own name
//! ("herdr-projects coordinator") and in its own colour, as this machine reads it.
use super::chat::{coord_who, parts_text, Who};
use super::talk::{with_mine, Part};
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::update::coords::Target;
use crate::coordinator::look::Look;
use crate::shown::ink::{Ink, Inked};

/// Nothing to show of it yet: no message has started it.
fn none() -> Look { Look { state: "none".into(), ..Default::default() } }
/// Its own name where the Coordinator's would be: on who said what, and on the tab a dialog waits in.
fn renamed(mut c: Who, t: &Target) -> Who {
    let tab = format!("herdr's tab \"{}\"", crate::coordinator::place::COORDINATOR_TAB);
    for p in c.parts.iter_mut() {
        if let Some(rest) = p.by.strip_prefix("Coordinator") { p.by = format!("{}{rest}", t.name); }
        p.text = p.text.replace("The Coordinator shows", &format!("The {} shows", t.name)).replace(&tab, &format!("herdr's tab \"{}\"", t.name));
    }
    c.text = c.parts.iter().map(|p| format!("{}{}", if p.by.is_empty() { String::new() } else { format!("{}:\n", p.by) }, p.text)).collect::<Vec<_>>().join("\n\n");
    let flag = t.to.strip_prefix("project:").map(|r| format!("--project {r}")).unwrap_or_default();
    c.note = c.note.replace("coordinator new starts", &format!("coordinator new {flag} starts"));
    Who { name: t.name.clone(), tint: t.ink(), ..c }
}
fn muted(t: &Theme, target: &Target, status: String, parts: Vec<Part>) -> Who {
    let _ = t;
    let text = parts.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("\n\n");
    Who { mark: Inked::of("", Ink::Muted), name: target.name.clone(), status, text, said: !parts.is_empty(), parts, ink: Ink::Muted, tint: target.ink(), ..Default::default() }
}
/// What you just sent it, while its conversation does not show it yet: yours at once at the end of the box, and the
/// line over the box says it is on its way (update/chat.rs). Not over the words of its screen.
pub fn with_sent(s: &mut State, target: &Target, v: &Look, mut c: Who) -> Who {
    if c.raw { return c; }
    let mine = crate::board::update::chat::coord_sent(s, target, v);
    if mine.is_empty() { return c; }
    c.parts = with_mine(c.parts, &mine);
    c.text = parts_text(&c.parts);
    Who { mine: true, said: true, status: "sending…".into(), ..c }
}
/// The chat's line and message for that coordinator.
pub fn sub_who(s: &mut State, t: &Theme, target: &Target) -> Who {
    if !s.data.can_dispatch { return muted(t, target, format!("off here: {}", crate::coordinator::who::OFF_WORDS), vec![]); }
    // The main Coordinator, picked by its row: as the board reads it every time.
    let (v, screen) = if target.to.is_empty() { (s.data.coord.clone(), s.data.coord_screen.clone()) } else { s.data.sub.as_ref().filter(|x| x.0 == target.to).map(|x| (x.1.clone(), x.2.clone())).unwrap_or_else(|| (none(), String::new())) };
    let fresh = !v.unseen.is_empty() && !s.chat_open;
    let mut c = renamed(coord_who(t, &v, &screen, fresh), target);
    c.always = !v.error.is_empty() && s.err_hid != v.error;
    with_sent(s, target, &v, c)
}
