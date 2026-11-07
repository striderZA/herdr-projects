//! The crate sw-flow (crates/flow), on sw-core: the flow of a task (delegating, the finish line, landing), the keeper
//! and what it runs, automations. With it, the parts of the board's
//! screen that this flow uses: the keys' table and the theme.
//! Which crate holds what, and why: docs/crates.md.
pub use sw_core::*;

pub mod actions;
pub mod after;
pub mod attempts;
pub mod automations;
pub mod coordinator;
pub mod delegate;
pub mod doctor;
pub mod extras;
pub mod finish;
pub mod flow;
pub mod hook;
pub mod housekeeping;
pub mod install;
pub mod jobs;
pub mod keeper;
pub mod land;
pub mod leftovers;
pub mod limits;
pub mod notify;
pub mod plan;
pub mod runner;
pub mod shown;
pub mod state;
pub mod status_kept;
pub mod submit;
pub mod sync;
pub mod turn;

/// What the board's screen has that the flow uses too: the keys (a message names a key) and the theme. sw-board's
/// `board` module passes them on, so they are `crate::board::keys` everywhere.
pub mod board {
    pub mod keys;
    pub mod remap;
    pub mod theme;
}
