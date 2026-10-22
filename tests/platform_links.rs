//! The platform layer's links: a link to a folder leads to it on both platforms (a symbolic link on Unix, a junction on
//! Windows); a link to a file reads the file; two folders are two places; on Windows a plain path and its \\?\ form are one
//! place, and nothing swaps in one step.
use std::fs;
use std::path::{Path, PathBuf};
use sw::platform::{links, meta};

/// An empty folder of this test's own.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sw-platform-links-{}", std::process::id())).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Removes a link whatever it is: a symbolic link or a file on Unix; on Windows a junction, which remove_dir takes out
/// without touching what it points to.
fn unlink(at: &Path) {
    let _ = fs::remove_file(at);
    let _ = fs::remove_dir(at);
}

#[test]
fn a_link_to_a_folder_leads_to_it() {
    let target = scratch("folder");
    let at = target.parent().unwrap().join("folder-link");
    unlink(&at);
    links::make_link(&target, &at).unwrap();
    assert!(meta::is_link(&fs::symlink_metadata(&at).unwrap()), "the link is a link");
    assert!(links::same_place(&at, &target), "the link leads to the folder");
    unlink(&at);
}

#[test]
fn a_link_to_a_file_reads_the_file() {
    let dir = scratch("file");
    let file = dir.join("note.txt");
    fs::write(&file, "hello").unwrap();
    let at = dir.join("note-link.txt");
    unlink(&at);
    links::make_link(&file, &at).unwrap();
    assert_eq!(fs::read_to_string(&at).unwrap(), "hello");
    #[cfg(unix)]
    {
        assert!(meta::is_link(&fs::symlink_metadata(&at).unwrap()), "a symbolic link on Unix");
    }
    #[cfg(windows)]
    {
        assert!(!meta::is_link(&fs::symlink_metadata(&at).unwrap()), "a copy on Windows, where a file link needs rights");
    }
}

#[test]
fn two_folders_are_two_places() {
    let (a, b) = (scratch("place-a"), scratch("place-b"));
    assert!(links::same_place(&a, &a));
    assert!(!links::same_place(&a, &b));
}

#[cfg(windows)]
#[test]
fn a_verbatim_path_is_the_same_place_and_nothing_swaps_in_one_step() {
    let a = scratch("verbatim");
    let verbatim = PathBuf::from(format!(r"\\?\{}", a.display()));
    assert!(links::same_place(&a, &verbatim));
    assert!(!links::swap(&a, &scratch("verbatim-other")));
}
