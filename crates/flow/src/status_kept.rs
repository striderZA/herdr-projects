//! A working task's line before its agent has said anything this run: the last sentence it wrote, kept and said with
//! its age ("12m ago: Running the check"), so an old sentence is never read as a new one. Never a bare "…".
use crate::json::*;
use crate::shown::words::age;

/// Whether the line is the board's own plain word: the row leaves it out.
pub fn quiet_plain(t: &Obj, role: &str) -> bool { !(role == "agent" && !t.at("lastLine").s("msg").is_empty()) }
pub fn quiet_line(t: &Obj, role: &str) -> String {
    let kept = if role == "agent" { t.at("lastLine") } else { &NULL };
    if !kept.s("msg").is_empty() { return format!("{}: {}", if kept.b("at") { format!("{} ago", age(crate::clock::now() - kept.i("at"))) } else { "Earlier".into() }, kept.s("msg")); }
    if role == "reviewer" { "Under review" } else { "Working" }.into()
}
