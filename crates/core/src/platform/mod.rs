//! The platform layer: what differs between Unix and Windows, one area per file. Each file has a unix half that holds
//! today's code and a windows half. Callers use these areas, never libc or std::os::unix directly.
pub mod disk;
pub mod endpoint;
pub mod links;
pub mod lock;
pub mod meta;
pub mod net;
pub mod perms;
pub mod proc;
pub mod shell;
pub mod signal;
