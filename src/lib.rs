//! herdr-wingman: a work board for herdr, one native program. The board's screen (board/), its command line (cli/),
//! the keeper that runs the flow of a task, and other machines.
//! `herdr-wingman docs --where` prints which file holds what, from the comment at the top of each file.
//! This crate is the command line and the made pages, on the crates below it (sw-board, sw-flow, sw-core),
//! whose modules it passes on as its own: everything is `sw::…`, as before the split (docs/crates.md).
pub use sw_board::*;

pub mod cli;
pub mod docs;
