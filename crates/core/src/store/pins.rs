//! Pinned projects: its repos, each pinned or not, kept in `pins.json` in the data folder (a task's pin is its own
//! `pinned`). It is sent with this machine's tasks, as they send their own pins (docs/pinned.md).
use super::files::*;
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

/// What is pinned: this machine's projects by their name on the board.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pins { pub repos: Vec<String> }
impl Pins {
    pub fn repo(&self, name: &str) -> bool { self.repos.iter().any(|r| r == name) }
    pub fn json(&self) -> Value { json!({ "repos": self.repos }) }
    /// As it was kept (nothing pinned when the file says nothing).
    pub fn read(v: &Value) -> Pins { Pins { repos: v.arr("repos").iter().map(text).filter(|r| !r.is_empty()).collect() } }
}
fn file() -> PathBuf { data_dir().join("pins.json") }
/// This machine's pins.
pub fn pins() -> Pins { Pins::read(&Value::Object(read_obj(&file()).unwrap_or_default())) }
/// Pin one of this machine's projects, or unpin it: what is pinned after, under the file's lock.
pub fn set(repo: &str, on: bool) -> Res<Pins> {
    with_lock_wait("pins", 5000, || {
        let mut p = pins();
        p.repos.retain(|x| x != repo);
        if on { p.repos.push(repo.to_string()); p.repos.sort(); }
        write_json(&file(), &p.json())?;
        Ok(p)
    })
}
