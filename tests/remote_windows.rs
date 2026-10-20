//! SSH machines are not in v1 on Windows: the machines check says so and asks no machine. The Unix tests of the same
//! code are gated in their own files (remote.rs, remote_why.rs).
#![cfg(windows)]
use sw::json::*;

#[test]
fn the_machines_check_says_ssh_machines_are_not_available_on_windows() {
    let r = sw::remote::check::check(1000);
    assert_eq!(r.at("here").at("why").s("line"), "SSH machines are not available on Windows in v1");
    assert_eq!(r.arr("machines").len(), 0, "no machine is asked");
    assert!(sw::remote::machines().is_empty(), "none is listed");
}
