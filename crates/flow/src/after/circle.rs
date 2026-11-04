//! A wait in a circle: a way from a task, through what each task waits for, back to it. The tasks are read here.
use crate::json::*;
use crate::store::tasks::{list, ListOpts};
use std::collections::HashMap;

/// What tasks wait for, and what each is called.
pub struct Graph { edges: HashMap<String, Vec<String>>, names: HashMap<String, String> }
impl Graph {
    /// This machine's tasks, with `me` waiting for `deps`.
    pub fn new(me: &str, deps: &[String]) -> Graph {
        let mut edges: HashMap<String, Vec<String>> = list(ListOpts { include_archived: true, ..Default::default() }).iter().map(|t| (t.t("id"), super::after_of(t))).collect();
        edges.insert(me.to_string(), deps.to_vec());
        Graph { edges, names: HashMap::new() }
    }
    /// A way from `me` back to `me`, when there is one.
    pub fn circle(&mut self, me: &str) -> Option<Vec<String>> {
        fn walk(at: &str, me: &str, edges: &HashMap<String, Vec<String>>, seen: &mut Vec<String>, path: &mut Vec<String>) -> bool {
            for next in edges.get(at).cloned().unwrap_or_default() {
                path.push(next.clone());
                if next == me { return true; }
                if !seen.contains(&next) { seen.push(next.clone()); if walk(&next, me, edges, seen, path) { return true; } }
                path.pop();
            }
            false
        }
        let mut path = vec![me.to_string()];
        walk(me, me, &self.edges, &mut vec![], &mut path).then_some(path)
    }
    /// A task on the way, as a person names it.
    pub fn say(&self, id: &str) -> String { self.names.get(id).cloned().unwrap_or_else(|| super::say(id)) }
}
