//! Text on its way: an agent's inbox.
use super::files::*;
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

// ---- the inbox: text for an agent that could not be delivered yet. Never dropped. ----------------------------------
fn inbox_file(id: &str) -> PathBuf { dir_of(id).join("inbox.jsonl") }
fn offset_file(id: &str) -> PathBuf { dir_of(id).join("inbox.offset") }
fn offset(id: &str) -> usize { read_json(&offset_file(id)).map(|v| number(&v) as usize).unwrap_or(0) }
pub fn queue_text(id: &str, text: &str, extra: Obj) -> Res<()> {
    let mut e = obj(json!({ "text": text, "at": now() }));
    for (k, v) in extra { e.insert(k, v); }
    append_line(&inbox_file(id), &Value::Object(e))
}
pub fn pending_text(id: &str) -> Vec<Value> { let all = read_lines(&inbox_file(id)); let off = offset(id).min(all.len()); all[off..].to_vec() }
pub fn take_text(id: &str) -> Res<Vec<Value>> {
    let all = read_lines(&inbox_file(id));
    let off = offset(id).min(all.len());
    write_json(&offset_file(id), &json!(all.len()))?;
    Ok(all[off..].to_vec())
}
/// Put taken text back (the delivery failed after all).
pub fn untake_text(id: &str, count: usize) -> Res<()> { write_json(&offset_file(id), &json!(offset(id).saturating_sub(count))) }
