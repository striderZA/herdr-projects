//! herdr and the plugin in it, for `doctor`: herdr found and new enough for the plugin, the plugin installed (from
//! GitHub, or linked from a folder of yours), enabled, and its folder still there.
use super::Found;
use crate::herdr::cli::{args, bin, run_raw};
use crate::install::UPSTREAM;
use crate::json::*;
use crate::store::marks::tilde;
use crate::util::newer;
use std::path::PathBuf;

/// What herdr said. `version`: None when herdr does not run here. `plugins`: its list, or why it gave none.
pub struct Herdr { pub version: Option<String>, pub plugins: Result<Vec<Value>, String> }
impl Default for Herdr { fn default() -> Self { Herdr { version: None, plugins: Ok(vec![]) } } }
/// Asked of herdr itself.
pub fn ask() -> Herdr {
    let version = run_raw(None, &args(&["--version"]), 5000, "").ok().map(|v| v.split_whitespace().last().unwrap_or("").to_string());
    if version.is_none() { return Herdr { version, plugins: Ok(vec![]) }; }
    let plugins = run_raw(None, &args(&["plugin", "list", "--json"]), 8000, "").map_err(|e| e.message).and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| e.to_string()))
        .map(|j| if j.at("result").at("plugins").is_array() { j.at("result").arr("plugins").to_vec() } else { j.arr("plugins").to_vec() });
    Herdr { version, plugins }
}
pub const ID: &str = "herdr-projects";
/// The oldest herdr the plugin works with, where herdr does not say it (herdr-plugin.toml `min_herdr_version`, which herdr
/// enforces on every platform). Windows needs more: its herdr 0.9.4 has the named pipe the board talks through.
pub const MIN_HERDR: &str = "0.9.3";
pub const MIN_HERDR_WINDOWS: &str = "0.9.4";
/// The oldest herdr this platform works with: the manifest's minimum, or the Windows one where that is newer.
pub fn floor(manifest_min: &str, windows: bool) -> String {
    if windows && newer(MIN_HERDR_WINDOWS, manifest_min) { MIN_HERDR_WINDOWS.into() } else { manifest_min.into() }
}
/// Why this herdr version is too old for the plugin on this platform, given the manifest's minimum (None: new enough).
pub fn refused_under(manifest_min: &str, version: &str, windows: bool) -> Option<String> {
    let min = floor(manifest_min, windows);
    newer(&min, version).then(|| if windows { format!("herdr {version} is older than {min}, what the plugin needs on Windows") } else { format!("herdr {version} is older than {min}, the oldest the plugin works with") })
}
/// `refused_under` with the minimum this program ships with (the board, which does not read the plugin's list).
pub fn refused(version: &str, windows: bool) -> Option<String> { refused_under(MIN_HERDR, version, windows) }
/// The herdr this program would run with, asked of it, when it is too old for Windows: the board refuses to open then.
pub fn too_old_here() -> Option<String> {
    if !cfg!(windows) { return None; }
    let v = run_raw(None, &args(&["--version"]), 5000, "").ok()?.split_whitespace().last()?.to_string();
    refused(&v, true)
}
fn ours(h: &Herdr) -> Option<&Value> { h.plugins.as_ref().ok()?.iter().find(|p| p.s("plugin_id") == ID) }
/// The folder herdr runs the plugin from.
pub fn root_of(h: &Herdr) -> Option<PathBuf> { ours(h).map(|p| PathBuf::from(p.s("plugin_root"))).filter(|r| !r.as_os_str().is_empty() && r.exists()) }

pub fn check(h: &Herdr) -> Vec<Found> { check_on(h, cfg!(windows)) }
/// `check` for the platform given: the same answers on every platform, so a test can ask for either.
pub fn check_on(h: &Herdr, windows: bool) -> Vec<Found> {
    let Some(v) = &h.version else { return vec![Found::bad(format!("herdr does not run here ({} was not found)", bin()), format!("Install Herdr {} or newer (https://herdr.dev), or put it on your PATH.", floor(MIN_HERDR, windows)))] };
    let list = match &h.plugins { Ok(l) => l, Err(why) => return vec![Found::bad(format!("herdr {v} did not list its plugins: {why}"), "Check that herdr runs: herdr workspace list")] };
    let mut out = vec![];
    let manifest = ours(h).map(|p| p.t("min_herdr_version")).filter(|m| !m.is_empty()).unwrap_or_else(|| MIN_HERDR.into());
    if let Some(why) = refused_under(&manifest, v, windows) { out.push(Found::bad(why, "Update Herdr, then restart its server.")); }
    let Some(p) = list.iter().find(|p| p.s("plugin_id") == ID) else {
        out.push(Found::bad(format!("The {ID} plugin is not installed in herdr {v}"), format!("Run: herdr plugin install {UPSTREAM}")));
        return out;
    };
    let root = p.s("plugin_root");
    let from = if p.at("source").s("kind") == "local" { format!("linked from {}", tilde(root)) } else { "installed from GitHub".into() };
    if !std::path::Path::new(root).join("herdr-plugin.toml").exists() {
        out.push(Found::bad(format!("herdr runs the plugin from {}, which is gone", tilde(root)), format!("Link a checkout again (herdr plugin link <folder>), or run: herdr plugin install {UPSTREAM}")));
        return out;
    }
    if p.at("enabled") == &json!(false) { out.push(Found::bad(format!("The plugin is in herdr, {from}, but turned off"), format!("Run: herdr plugin enable {ID}"))); return out; }
    out.push(Found::ok(format!("herdr {v} runs the plugin {}, {from}", p.s("version"))));
    out
}
