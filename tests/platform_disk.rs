//! The platform layer's disk facts: the temp folder has free space, and what is free is at most the whole disk; two folders
//! on one disk are one volume.
use std::fs;
use sw::platform::disk;

#[test]
fn the_temp_folder_has_free_space_within_its_disk() {
    let (available, total) = disk::space(&std::env::temp_dir()).expect("the space of the temp folder");
    assert!(available > 0, "free space {available}");
    assert!(total >= available, "the whole disk {total} is at least the free {available}");
}

#[test]
fn two_folders_on_one_disk_are_one_volume() {
    let a = std::env::temp_dir();
    let b = a.join(format!("sw-platform-disk-{}", std::process::id()));
    fs::create_dir_all(&b).unwrap();
    assert!(disk::volume(&a).is_some());
    assert_eq!(disk::volume(&a), disk::volume(&b));
    let _ = fs::remove_dir(&b);
}
