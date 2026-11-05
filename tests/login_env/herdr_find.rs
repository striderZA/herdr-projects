//! Finding herdr when this process's PATH lacks it (a plain `ssh host cmd` gets no login shell): the usual install
//! spots under the home folder, then a login shell's PATH; a PATH that has it keeps the bare name.
use crate::common;
use common::*;
use std::path::Path;
use sw::herdr::find::{elsewhere, spots};

fn program(dir: &Path, name: &str, mode: u32) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, "#!/bin/sh\n").unwrap();
    sw::sys::set_mode(&p, mode).unwrap();
    p
}

fn usual_bin(home: &Path) -> std::path::PathBuf {
    if cfg!(windows) { home.join(".cargo/bin") } else { home.join(".local/bin") }
}

fn path(dirs: &[&Path]) -> String {
    std::env::join_paths(dirs).unwrap().to_string_lossy().into_owned()
}

#[test]
fn herdr_in_local_bin_is_found_when_path_lacks_it_and_the_login_shell_is_not_asked() {
    let w = world();
    let home = w.base.join("home");
    let found = program(&usual_bin(&home), "herdr", 0o755);
    let asked = std::cell::Cell::new(false);
    assert_eq!(elsewhere("herdr", "", &spots(&home), || { asked.set(true); String::new() }), Some(found));
    assert!(!asked.get(), "a usual spot that has it needs no login shell");
}

#[test]
fn a_path_that_has_herdr_keeps_the_bare_name() {
    let w = world();
    let home = w.base.join("home");
    let on_path = w.base.join("tools");
    program(&on_path, "herdr", 0o755);
    program(&usual_bin(&home), "herdr", 0o755);
    assert_eq!(elsewhere("herdr", &path(&[&on_path]), &spots(&home), || panic!("not asked")), None);
}

#[test]
fn the_login_shells_path_is_asked_last_and_a_file_that_does_not_run_is_no_program() {
    let w = world();
    let home = w.base.join("home");
    // A file that does not run: on Unix its mode says so; Windows has no mode, so it is a file a shell would not run (no #! line).
    let not_run = usual_bin(&home).join("herdr");
    std::fs::create_dir_all(not_run.parent().unwrap()).unwrap();
    std::fs::write(&not_run, if cfg!(windows) { "not a program\n" } else { "#!/bin/sh\n" }).unwrap();
    sw::sys::set_mode(&not_run, 0o644).unwrap();
    let odd = w.base.join("odd/place");
    let found = program(&odd, "herdr", 0o755);
    let mine = vec![usual_bin(&home)];
    assert_eq!(elsewhere("herdr", "", &mine, || path(&[Path::new("/nowhere"), &odd])), Some(found));
    assert_eq!(elsewhere("herdr", "", &mine, || "/nowhere".into()), None, "found nowhere: the bare name, and herdr's own error");
}
