//! The platform layer's file facts: a file written now has a modification time within seconds of now; two hard links to one
//! file share an identity and two other files do not; a regular file is neither a socket nor a link; on Unix the times and
//! the mode are the stat fields the program has always read.
use std::fs;
use std::path::PathBuf;
use sw::platform::meta;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sw-platform-meta-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn a_file_written_now_has_a_modification_time_of_now() {
    let path = scratch("now.txt");
    fs::write(&path, "x").unwrap();
    let ms = meta::mtime_ms(&fs::metadata(&path).unwrap());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64;
    assert!((now - ms).abs() < 5000.0, "modification {ms} ms, now {now} ms");
    let _ = fs::remove_file(&path);
}

#[test]
fn hard_links_share_an_identity_and_other_files_do_not() {
    let (a, b, c) = (scratch("link-a.txt"), scratch("link-b.txt"), scratch("link-c.txt"));
    let _ = fs::remove_file(&b);
    let _ = fs::remove_file(&c);
    fs::write(&a, "x").unwrap();
    fs::hard_link(&a, &b).unwrap();
    fs::write(&c, "y").unwrap();
    assert!(meta::file_id(&a).is_some());
    assert_eq!(meta::file_id(&a), meta::file_id(&b), "two links to one file");
    assert_ne!(meta::file_id(&a), meta::file_id(&c), "two different files");
    let open = fs::File::open(&a).unwrap();
    assert_eq!(meta::file_id_of(&open), meta::file_id(&a), "an open handle names its file");
    assert!(meta::same_disk(&a, &c), "two files in one folder are on one disk");
    for p in [&a, &b, &c] {
        let _ = fs::remove_file(p);
    }
}

#[test]
fn a_regular_file_is_neither_a_socket_nor_a_link() {
    let path = scratch("plain.txt");
    fs::write(&path, "x").unwrap();
    let m = fs::symlink_metadata(&path).unwrap();
    assert!(!meta::is_socket(&m));
    assert!(!meta::is_link(&m));
    let _ = fs::remove_file(&path);
}

#[cfg(unix)]
#[test]
fn on_unix_the_times_and_mode_are_the_stat_fields() {
    use std::os::unix::fs::MetadataExt;
    let path = scratch("stat.txt");
    fs::write(&path, "x").unwrap();
    let m = fs::metadata(&path).unwrap();
    assert_eq!(meta::mtime_parts(&m), (m.mtime(), m.mtime_nsec()));
    assert_eq!(meta::ctime_parts(&m), (m.ctime(), m.ctime_nsec()));
    assert_eq!(meta::mode(&m), m.mode());
    assert_eq!(meta::mtime_ms(&m), m.mtime() as f64 * 1000.0 + m.mtime_nsec() as f64 / 1e6);
    let _ = fs::remove_file(&path);
}
