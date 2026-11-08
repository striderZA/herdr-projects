//! Text on its way: an agent's inbox.
use super::files::*;
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

// ---- the inbox: text for an agent that could not be delivered yet. Never dropped: text is marked taken only once a
// ---- send was taken, so a crash in between delivers it again rather than losing it. --------------------------------
fn inbox_file(id: &str) -> PathBuf { dir_of(id).join("inbox.jsonl") }
fn offset_file(id: &str) -> PathBuf { dir_of(id).join("inbox.offset") }
fn offset(id: &str) -> usize { read_json(&offset_file(id)).map(|v| number(&v) as usize).unwrap_or(0) }
pub fn queue_text(id: &str, text: &str, extra: Obj) -> Res<()> {
    let mut e = obj(json!({ "text": text, "at": now() }));
    for (k, v) in extra { e.insert(k, v); }
    append_line(&inbox_file(id), &Value::Object(e))
}
pub fn pending_text(id: &str) -> Vec<Value> { let all = read_lines(&inbox_file(id)); let off = offset(id).min(all.len()); all[off..].to_vec() }
/// The text waiting for the agent, not yet taken: nothing is marked until the send is (mark_taken).
pub fn take_text(id: &str) -> Vec<Value> { let all = read_lines(&inbox_file(id)); let off = offset(id).min(all.len()); all[off..].to_vec() }
/// What the agent took: the next take reads from here. Called after the send, never before it.
pub fn mark_taken(id: &str, count: usize) -> Res<()> { write_json(&offset_file(id), &json!(offset(id).saturating_add(count))) }
