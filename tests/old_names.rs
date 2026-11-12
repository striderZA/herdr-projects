//! What the names from before the rename still do: the home folder moves to its new name by the first run, and a repo's
//! file under its old name is still read. Each test makes its own folders under its own scratch base (never the real home).
use crate::common;
use serde_json::{json, Value};
use sw::store::rename_home::rename_home;
use sw::store::settings::places::repo_file;

#[test]
fn the_home_folder_under_its_old_name_is_renamed_and_its_files_stay() {
    let w = common::world();
    let user = w.base.join("user");
    std::fs::create_dir_all(user.join(".herdr-projects/data")).unwrap();
    std::fs::write(user.join(".herdr-projects/data/task.json"), "{\"id\":1}").unwrap();
    let (home, line) = rename_home(&user);
    assert_eq!(home, user.join(".herdr-wingman"));
    assert_eq!(std::fs::read_to_string(home.join("data/task.json")).unwrap(), "{\"id\":1}");
    assert!(line.unwrap().starts_with("Renamed ~/.herdr-projects to ~/.herdr-wingman."));
    // the old name still reaches the same files wherever a link could be made
    let linked = std::fs::symlink_metadata(user.join(".herdr-projects")).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    if linked { assert_eq!(std::fs::read_to_string(user.join(".herdr-projects/data/task.json")).unwrap(), "{\"id\":1}"); }
}

#[test]
fn with_both_folders_there_the_old_one_stays_in_use_and_nothing_is_touched() {
    let w = common::world();
    let user = w.base.join("user");
    std::fs::create_dir_all(user.join(".herdr-projects")).unwrap();
    std::fs::create_dir_all(user.join(".herdr-wingman")).unwrap();
    std::fs::write(user.join(".herdr-projects/a.txt"), "old").unwrap();
    std::fs::write(user.join(".herdr-wingman/b.txt"), "new").unwrap();
    let (home, line) = rename_home(&user);
    assert_eq!(home, user.join(".herdr-projects"));
    assert!(line.unwrap().starts_with("Both ~/.herdr-projects and ~/.herdr-wingman are there"));
    assert_eq!(std::fs::read_to_string(user.join(".herdr-projects/a.txt")).unwrap(), "old");
    assert_eq!(std::fs::read_to_string(user.join(".herdr-wingman/b.txt")).unwrap(), "new");
}

#[test]
fn a_repos_file_under_its_old_name_is_still_read_and_the_new_name_wins_over_it() {
    let w = common::world();
    std::fs::write(w.root.join(".herdr-projects.json"), json!({ "check": { "command": "true" } }).to_string()).unwrap();
    let (f, r) = repo_file(&w.root, None);
    assert!(r.is_empty(), "{r:?}");
    assert_eq!(Value::Object(f), json!({ "check.command": "true" }));
    std::fs::write(w.root.join(".herdr-wingman.json"), json!({ "check": { "command": "false" } }).to_string()).unwrap();
    assert_eq!(Value::Object(repo_file(&w.root, None).0), json!({ "check.command": "false" }));
}
