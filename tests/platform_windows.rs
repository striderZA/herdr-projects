//! The Windows platform layer against the real thing: a herdr connection (each request answered, a silent server let go),
//! a file that a program runs from or another process holds being replaced, and a process tree stopped with its children.
//! Windows only: each test names what Windows does.
#![cfg(windows)]
use crate::common;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::os::windows::fs::OpenOptionsExt;
/// Windows share modes: a file opened with these is read and written by others, and not deleted (so not renamed over).
const FILE_SHARE_READ: u32 = 0x1;
const FILE_SHARE_WRITE: u32 = 0x2;

/// Runs `f` on a thread and fails when it does not finish within `secs`: a hang is a failed test, not a stuck run.
fn within<T: Send + 'static>(secs: u64, what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(secs)).unwrap_or_else(|_| panic!("{what} did not finish within {secs} s"))
}

/// A pipe name only this test run uses.
fn pipe_name(tag: &str) -> String {
    format!("herdr-projects-test-{}-{tag}", std::process::id())
}

/// An empty folder only this test uses.
fn folder(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("herdr-platform-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_connection_answers_each_request_with_no_write_waiting_behind_a_read() {
    // A synchronous pipe queues every I/O on its handle: a reader left blocked in a read would hold the next write back.
    let name = pipe_name("two-way");
    let listener = common::pipe::Listener::bind(&name).unwrap();
    let server = std::thread::spawn(move || {
        let mut s = BufReader::new(listener.accept().unwrap());
        let mut line = String::new();
        s.read_line(&mut line).unwrap();
        s.get_mut().write_all(b"first\n").unwrap();
        line.clear();
        s.read_line(&mut line).unwrap();
        s.get_mut().write_all(b"second\n").unwrap();
    });
    let answers = within(10, "the two requests", move || {
        let mut c = BufReader::new(sw::sys::connect_local(&name).unwrap());
        let mut answers = vec![];
        c.get_mut().write_all(b"one\n").unwrap();
        let mut line = String::new();
        c.read_line(&mut line).unwrap();
        answers.push(line.clone());
        c.get_mut().write_all(b"two\n").unwrap();
        line.clear();
        c.read_line(&mut line).unwrap();
        answers.push(line);
        answers
    });
    assert_eq!(answers, vec!["first\n", "second\n"]);
    server.join().unwrap();
}

#[test]
fn a_silent_server_times_out_and_the_connection_is_let_go() {
    // The client gives up after its timeout and drops the stream: the server's read must end then, not wait for ever.
    let name = pipe_name("silent");
    let listener = common::pipe::Listener::bind(&name).unwrap();
    let (ended_tx, ended_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut s = listener.accept().unwrap();
        let mut buf = [0u8; 64];
        let _ = s.read(&mut buf);
        let end = s.read(&mut buf).map(|n| n as i64).unwrap_or(-1);
        let _ = ended_tx.send(end);
    });
    let timed_out = within(10, "the timed-out request", move || {
        let mut c = sw::sys::connect_local(&name).unwrap();
        c.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        c.write_all(b"ask\n").unwrap();
        let mut buf = [0u8; 16];
        c.read(&mut buf).unwrap_err().kind() == std::io::ErrorKind::TimedOut
    });
    assert!(timed_out, "the read did not time out");
    let end = ended_rx.recv_timeout(Duration::from_secs(5)).expect("the server's read did not end when the client let go");
    assert!(end <= 0, "the server read {end} more bytes after the client let go");
}

#[test]
fn an_empty_read_returns_at_once_and_waits_for_nothing() {
    // Read's contract: an empty buffer reads nothing and returns at once. The server here answers nothing, so a read that
    // waited for an answer would time out; and the empty read must leave nothing pending for the next read to take.
    let name = pipe_name("empty-read");
    let listener = common::pipe::Listener::bind(&name).unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let server = std::thread::spawn(move || {
        let mut s = listener.accept().unwrap();
        let mut buf = [0u8; 64];
        let _ = s.read(&mut buf);
        // Keeps the pipe open until the client is done with it.
        let _ = done_rx.recv();
    });
    let (empty, took, after) = within(10, "the empty read", move || {
        let mut c = sw::sys::connect_local(&name).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        c.write_all(b"ask\n").unwrap();
        let started = Instant::now();
        let empty = c.read(&mut []).map_err(|e| e.kind());
        let took = started.elapsed();
        c.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let after = c.read(&mut [0u8; 8]).map_err(|e| e.kind());
        (empty, took, after)
    });
    assert_eq!(empty, Ok(0), "an empty read returned something other than Ok(0)");
    assert!(took < Duration::from_secs(2), "the empty read waited {took:?} for the server");
    assert_eq!(after, Err(std::io::ErrorKind::TimedOut), "the empty read left data for the next read");
    done_tx.send(()).unwrap();
    server.join().unwrap();
}

#[test]
fn a_file_a_program_runs_from_is_replaced_and_the_program_keeps_running() {
    let dir = folder("running");
    let prog = dir.join("prog.exe");
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    std::fs::copy(format!(r"{system}\System32\ping.exe"), &prog).unwrap();
    let mut running = Command::new(&prog).args(["-n", "30", "127.0.0.1"]).stdout(Stdio::null()).spawn().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let new = dir.join("new.tmp");
    std::fs::write(&new, b"the new program").unwrap();
    let replaced = sw::sys::replace_file(&new, &prog);
    let still_running = running.try_wait().unwrap().is_none();
    let content = std::fs::read(&prog).unwrap();
    let _ = running.kill();
    let _ = running.wait();
    replaced.expect("a running program's file was not replaced");
    assert_eq!(content, b"the new program");
    assert!(still_running, "the program that ran from the file was stopped by the replacement");
    let moved_aside = std::fs::read_dir(&dir).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".old"));
    assert!(moved_aside, "the running copy was not moved aside");
}

#[test]
fn a_file_another_process_holds_for_a_moment_is_replaced_once_let_go() {
    let dir = folder("held");
    let to = dir.join("state.json");
    let new = dir.join("new.tmp");
    std::fs::write(&to, b"old").unwrap();
    std::fs::write(&new, b"new").unwrap();
    // Held the way a program that does not share delete holds it (an editor or a scanner may): Windows refuses a rename over
    // it. Rust's own File::open shares delete, so a rename over that does not wait at all.
    let held = std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(&to).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        drop(held);
    });
    sw::sys::replace_file(&new, &to).expect("the file was not replaced once its reader let go");
    release.join().unwrap();
    assert_eq!(std::fs::read(&to).unwrap(), b"new");
}

#[test]
fn a_file_held_for_good_is_left_as_it_was_and_the_new_one_is_kept() {
    let dir = folder("held-for-good");
    let to = dir.join("state.json");
    let new = dir.join("new.tmp");
    std::fs::write(&to, b"old").unwrap();
    std::fs::write(&new, b"new").unwrap();
    let held = std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(&to).unwrap();
    assert!(sw::sys::replace_file(&new, &to).is_err(), "a file held for good was replaced");
    drop(held);
    assert_eq!(std::fs::read(&to).unwrap(), b"old");
    assert_eq!(std::fs::read(&new).unwrap(), b"new", "the new file was lost: the caller could not try again");
}

#[test]
fn a_process_tree_is_stopped_with_its_children() {
    let mut parent = Command::new("cmd.exe").args(["/C", "ping", "-n", "60", "127.0.0.1"]).stdout(Stdio::null()).spawn().unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let pid = parent.id() as i64;
    let children = sw::sys::children(pid).unwrap();
    assert!(!children.is_empty(), "cmd started no child to stop with it");
    assert!(sw::sys::kill_group(pid as i32, sw::sys::KILL_SIGNAL));
    assert!(!sw::sys::pid_alive(pid), "the process was not stopped");
    for c in children {
        assert!(!sw::sys::pid_alive(c), "child {c} was not stopped with its process");
    }
    let _ = parent.wait();
}

/// Run as a child by `read_input_on_a_pipe_returns_what_came_and_gives_up_after_its_wait`, with a pipe for its standard
/// input. In the suite it does nothing.
#[test]
fn read_input_child() {
    if std::env::var_os("HERDR_TEST_READ_INPUT").is_none() {
        return;
    }
    let mut buf = [0u8; 16];
    let n = sw::sys::read_input_within(&mut buf, 400);
    println!("READ {n} {}", String::from_utf8_lossy(&buf[..n]));
}

/// Runs `read_input_child` with `input` written to its standard input (None: nothing is written, the pipe stays open).
/// Returns the line it printed and how long it ran.
fn read_child(input: Option<&[u8]>) -> (String, Duration) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "platform_windows::read_input_child", "--nocapture", "--test-threads=1"])
        .env("HERDR_TEST_READ_INPUT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut s = String::new();
        stdout.read_to_string(&mut s).unwrap();
        s
    });
    if let Some(bytes) = input {
        stdin.write_all(bytes).unwrap();
    }
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            panic!("the child did not end: read_input_within did not give up at its timeout");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let took = started.elapsed();
    drop(stdin);
    let out = out.join().unwrap();
    assert!(status.success(), "{out}");
    // With --nocapture the harness writes its own "test name ... " before the child's line on the same line.
    let line = out.lines().find_map(|l| l.find("READ ").map(|at| l[at..].trim_end().to_string())).unwrap_or_else(|| panic!("no READ line in: {out}"));
    (line, took)
}

#[test]
fn read_input_on_a_pipe_returns_what_came_and_gives_up_after_its_wait() {
    let (got, _) = read_child(Some(b"ab"));
    assert_eq!(got, "READ 2 ab");
    let (gave_up, took) = read_child(None);
    assert_eq!(gave_up, "READ 0");
    assert!(took >= Duration::from_millis(300) && took < Duration::from_secs(20), "the wait took {took:?}");
}
