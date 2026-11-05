//! The plugin manifest's platforms: every entry names its own, and no entry mixes Windows with Linux or macOS. herdr gives an
//! entry with none the plugin's list, and that list has Windows in it, so an unnamed entry would also run on Windows.
use std::path::Path;

#[test]
fn every_manifest_entry_names_its_platforms_and_windows_stands_apart() {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("herdr-plugin.toml")).unwrap();
    let manifest: toml::Value = toml::from_str(&text).unwrap();
    let mut entries = 0;
    for section in ["build", "startup", "actions", "events", "panes"] {
        for e in manifest.get(section).and_then(|v| v.as_array()).into_iter().flatten() {
            entries += 1;
            let name = e.get("id").or(e.get("on")).and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| format!("{:?}", e.get("command")));
            let Some(list) = e.get("platforms").and_then(|v| v.as_array()) else {
                panic!("{section} entry {name} names no platforms: herdr would give it the plugin's list, Windows included");
            };
            let names: Vec<&str> = list.iter().filter_map(|p| p.as_str()).collect();
            let windows = names.contains(&"windows");
            assert!(!(windows && (names.contains(&"linux") || names.contains(&"macos"))), "{section} entry {name} mixes windows with unix: {names:?}");
        }
    }
    assert!(entries >= 20, "the manifest has {entries} entries: the test read the wrong file");
}
