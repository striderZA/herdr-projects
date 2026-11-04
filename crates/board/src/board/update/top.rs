//! The list's top row: this machine, whose coordinator is the top-level one. The board opens on it, and the chat talks
//! to its coordinator where it talks to no other row's agent. Also how each agent is named: `GTM-14's agent`.
use super::coords::Target;
use super::heads::machine_id;
use crate::board::state::State;

/// The top row's id.
pub fn top_id() -> String { machine_id() }
/// `herdr-projects coordinator`.
pub fn coordinator_of(name: &str) -> String { format!("{} coordinator", possessive(name)) }
/// A name at the start of a sentence: `The Coordinator`, `gtm's coordinator` (a name keeps its case).
pub fn sentence(name: &str) -> String { match name.strip_prefix("the ") { Some(r) => format!("The {r}"), None => name.to_string() } }
pub fn possessive(name: &str) -> String { if name.ends_with('s') { format!("{name}'") } else { format!("{name}'s") } }
/// The top row's coordinator: who the chat talks to where it talks to no other row's agent. None where none can run
/// (`coordinator.agent none` here): the box then adds tasks.
pub fn top(s: &State) -> Option<Target> {
    if s.archive_view { return None; }
    let t = super::coords::of_head(s, &top_id())?;
    if !s.data.can_dispatch { return None; }
    Some(t)
}
