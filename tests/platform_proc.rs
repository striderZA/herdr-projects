//! The platform layer's process checks, on Unix and Windows: a live process is alive, an exited one is not, a start time
//! is stable, a child is ended by signal, a detached group is ended as one, and the process table and command line show
//! this process.
use std::process::{Command, Stdio};
use std::time::Duration;
use sw::platform::proc;

/// This test binary, set to run `sleeper` (below) until it is ended.
fn sleeping_child() -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "platform_proc::sleeper", "--ignored"]).stdout(Stdio::null()).stderr(Stdio::null());
    cmd
}

#[test]
fn this_process_is_alive_with_a_stable_start_time() {
    let me = std::process::id() as i64;
    assert!(proc::alive(me));
    // Linux (/proc) and Windows give a start time; macOS has no /proc, so it gives none, as the program always has.
    if cfg!(any(target_os = "linux", windows)) {
        assert!(proc::start_time(me).is_some(), "the start time of this process");
    }
    assert_eq!(proc::start_time(me), proc::start_time(me));
}

#[test]
fn a_child_that_has_exited_is_not_alive() {
    let mut child = Command::new(std::env::current_exe().unwrap()).args(["--list"]).stdout(Stdio::null()).spawn().unwrap();
    let pid = child.id() as i64;
    child.wait().unwrap();
    assert!(!proc::alive(pid));
}

#[test]
fn a_running_child_is_ended_by_signal() {
    let mut child = sleeping_child().spawn().unwrap();
    let pid = child.id() as i64;
    assert!(proc::alive(pid), "the child is running");
    assert!(proc::signal(pid, true), "the child can be ended");
    assert!(!child.wait().unwrap().success(), "ended, not finished");
}

#[test]
fn a_detached_group_is_ended_as_a_group() {
    let mut cmd = sleeping_child();
    proc::detach(&mut cmd);
    let mut child = cmd.spawn().unwrap();
    let pid = child.id() as i64;
    proc::join_group(pid);
    assert!(proc::signal_group(pid, true), "the group can be ended");
    assert!(!child.wait().unwrap().success(), "ended, not finished");
}

#[test]
fn the_process_table_and_command_line_show_this_process() {
    let me = std::process::id() as i64;
    let pid = me.to_string();
    assert!(proc::process_table().lines().any(|l| l.split_whitespace().next() == Some(pid.as_str())));
    assert!(proc::command_line(me).is_some());
}

/// What `sleeping_child` runs: it sleeps until it is ended. Ignored, so it runs only when a test starts it.
#[test]
#[ignore]
fn sleeper() {
    std::thread::sleep(Duration::from_secs(120));
}
