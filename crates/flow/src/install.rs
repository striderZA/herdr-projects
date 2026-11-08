//! The program installs and updates itself: `install` (re)writes the launcher in the data folder, `update` fetches this
//! platform's file of the newest release (or of the version given) and puts it in place of the downloaded program, in
//! one step. bin/run.sh downloads the same files when herdr starts a plugin command. Nothing needs Node or npm.
use crate::err::Res;
use crate::store::files::{bin, write_atomic, VERSION};
use std::path::{Path, PathBuf};

/// This fork's repo: where a plugin that was not cloned from a GitHub repo of its own gets its releases.
pub const RELEASES_REPO: &str = "striderZA/herdr-projects";

/// The release files' name for a platform, from `uname -s`/`uname -m` or Rust's own names. None: no release is made.
pub fn platform_of(os: &str, arch: &str) -> Option<(&'static str, &'static str)> {
    let os = match os.to_ascii_lowercase().as_str() { "linux" => "linux", "windows" => "windows", _ => return None };
    let arch = match arch { "x86_64" | "amd64" => "x86_64", "aarch64" | "arm64" => "aarch64", _ => return None };
    Some((os, arch))
}
pub fn platform() -> Option<(&'static str, &'static str)> { platform_of(std::env::consts::OS, std::env::consts::ARCH) }
/// `herdr-projects-linux-x86_64`: the release file, and the downloaded program's name in the plugin's bin/ folder.
/// Windows programs end in `.exe`: `herdr-projects-windows-x86_64.exe`.
pub fn asset_name((os, arch): (&str, &str)) -> String { let ext = if os == "windows" { ".exe" } else { "" }; format!("herdr-projects-{os}-{arch}{ext}") }
/// Where the releases are: HERDR_PROJECTS_RELEASES (a fork, a mirror, a folder as file:///…), else GitHub's releases of
/// the repo the plugin was cloned from (see repo_of).
pub fn releases(exe: &Path) -> String {
    std::env::var("HERDR_PROJECTS_RELEASES").ok().filter(|v| !v.is_empty()).map(|v| v.trim_end_matches('/').to_string()).unwrap_or_else(|| format!("https://github.com/{}/releases", repo_of(exe)))
}

/// The repo the releases are of: the one the plugin checkout that holds this program was cloned from (its git origin;
/// `herdr plugin install` clones it, and the program is in its bin/). The fork's repo when there is no such checkout,
/// as for a program run from anywhere else.
pub fn repo_of(exe: &Path) -> String {
    exe.parent().filter(|d| d.file_name().is_some_and(|n| n == "bin")).and_then(Path::parent).and_then(checkout_origin).and_then(|u| repo_of_remote(&u)).unwrap_or_else(|| RELEASES_REPO.to_string())
}
/// The origin's URL as git's own `remote get-url` gives it (`insteadOf` applied), or None without one: the release lookup
/// has always read it this way.
fn checkout_origin(dir: &Path) -> Option<String> { crate::store::repos::git_out(dir, &["remote", "get-url", "origin"]).ok() }
/// A remote's host, the `owner/name` it names, and whether the URL is an ssh one (its host may then be an alias of
/// ~/.ssh/config): `https://[user@]host[:port]/owner/name`, `ssh://[user@]host[:port]/owner/name`, and the scp form
/// `[user@]host:owner/name`. A `.git` suffix and a trailing slash are left out. None for a URL that names no repo on a host.
pub struct Remote { pub host: String, pub repo: String, pub ssh: bool }
pub fn remote_of(url: &str) -> Option<Remote> {
    let u = url.trim().trim_end_matches('/');
    let u = u.strip_suffix(".git").unwrap_or(u);
    // The part before the path is the host with its user and port: after `scheme://`, or before the first `:` of the scp form.
    let (ssh, authority, path) = match u.split_once("://") {
        Some((scheme, rest)) => { let (a, p) = rest.split_once('/')?; (scheme.eq_ignore_ascii_case("ssh"), a, p) }
        None => { let (a, p) = u.split_once(':')?; (true, a, p) }
    };
    let host = authority.rsplit('@').next()?.split(':').next()?.to_ascii_lowercase();
    // One letter before the colon is a Windows drive (`C:/a/b`), not a host.
    if host.len() < 2 || host.contains(['/', '\\']) { return None; }
    let mut parts = path.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    (parts.next().is_none() && !owner.is_empty() && !name.is_empty()).then(|| Remote { host, repo: format!("{owner}/{name}"), ssh })
}
/// `owner/name` of a GitHub remote URL (github.com, or ssh.github.com), or None for any other host.
pub fn repo_of_remote(url: &str) -> Option<String> {
    remote_of(url).filter(|r| matches!(r.host.as_str(), "github.com" | "ssh.github.com")).map(|r| r.repo)
}
/// The file of one release (`0.3.1` or `v0.3.1`), or of the newest one (None).
pub fn asset_url(base: &str, version: Option<&str>, asset: &str) -> String {
    match version { Some(v) => format!("{base}/download/v{}/{asset}", v.trim_start_matches('v')), None => format!("{base}/latest/download/{asset}") }
}
/// Versions compared part by part (util.rs, where the data folder's own check reads it too).
pub use crate::util::newer;
/// The hash a `.sha256` file holds (`<hex>  <name>`, as sha256sum writes it).
pub fn sum_in(text: &str) -> Option<String> {
    let h = text.split_whitespace().next()?.to_ascii_lowercase();
    (h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit())).then_some(h)
}
/// The version a program says it is: `herdr-projects 0.3.0 (/its/path)`.
pub fn version_said(out: &str) -> Option<String> {
    let v = out.strip_prefix("herdr-projects ")?.split_whitespace().next()?;
    v.chars().next().filter(|c| c.is_ascii_digit()).map(|_| v.to_string())
}
/// A program cargo built (it is in a `target` folder): updated with git pull and cargo build, never by a download.
pub fn built_from_source(exe: &Path) -> bool { exe.components().any(|c| c.as_os_str() == "target") }
/// Beside the downloaded program: the version it is, which bin/run.sh reads without running it.
pub fn version_file(prog: &Path) -> PathBuf { PathBuf::from(format!("{}.version", prog.display())) }

// ---- the network and the tools: what the tests replace -------------------------------------------------------------
/// A private repo's release is not handed out to curl: gh, logged in, fetches it. The gh command for a GitHub release
/// URL (of any repo), or None.
pub fn gh_args(url: &str, to: &Path) -> Option<Vec<String>> {
    let (repo, rest) = url.strip_prefix("https://github.com/")?.split_once("/releases/")?;
    if repo.matches('/').count() != 1 { return None; }
    let (tag, asset) = match rest.strip_prefix("latest/download/") { Some(a) => (None, a), None => { let (t, a) = rest.strip_prefix("download/")?.split_once('/')?; (Some(t), a) } };
    let mut v: Vec<String> = vec!["release".into(), "download".into()];
    v.extend(tag.map(String::from));
    v.extend(["-R", repo, "-p", asset, "-O", &to.display().to_string(), "--clobber"].map(String::from));
    Some(v)
}
/// One URL into one file, with curl or wget (or gh, for a private repo), never waiting for long.
pub fn fetch(url: &str, to: &Path) -> Res<()> {
    use std::process::{Command, Stdio};
    let to_s = to.display().to_string();
    let mut tries: Vec<(&str, Vec<String>)> = vec![("curl", ["-fsSL", "--connect-timeout", "10", "--max-time", "300", "-o", &to_s, url].map(String::from).to_vec()), ("wget", ["-q", "--tries=1", "-T", "30", "-O", &to_s, url].map(String::from).to_vec())];
    if let Some(a) = gh_args(url, to) { tries.insert(1, ("gh", a)); }
    let mut tried = false;
    for (tool, args) in tries {
        if tried && tool == "wget" { break; } // curl answered: wget would not know better
        match Command::new(tool).args(&args).stdin(Stdio::null()).stderr(Stdio::null()).status() { Ok(s) if s.success() => return Ok(()), Ok(_) => tried = true, Err(_) => {} }
    }
    let _ = std::fs::remove_file(to);
    if tried { crate::fail!("could not download {url}: no network, no such release, or a private repo (log in with gh auth login)") }
    crate::fail!("neither curl nor wget is installed here, to download {url}")
}
/// The 64 hex digits of a hash printed on a line of their own: what `certutil -hashfile <file> SHA256` writes (older
/// Windows spaces the bytes out).
pub fn hex_sum(text: &str) -> Option<String> {
    text.lines().find_map(|l| { let h: String = l.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase(); (h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())).then_some(h) })
}
/// The SHA-256 of a file, from whichever tool is here: sha256sum or openssl (Linux, and Git's own on Windows), else
/// certutil, which every Windows has in its system folder. None: no tool, or none that could hash it.
pub fn sha256(file: &Path) -> Option<String> {
    let f = file.display().to_string();
    for (tool, args) in [("sha256sum", vec![f.as_str()]), ("openssl", vec!["dgst", "-sha256", "-r", f.as_str()]), ("certutil", vec!["-hashfile", f.as_str(), "SHA256"])] {
        if let Ok(o) = std::process::Command::new(tool).args(&args).output() {
            if o.status.success() { if let Some(h) = sum_in(&String::from_utf8_lossy(&o.stdout)).or_else(|| hex_sum(&String::from_utf8_lossy(&o.stdout))) { return Some(h); } }
        }
    }
    None
}
/// What a program says its version is, run with nothing of the board's done at its start.
fn version_of(prog: &Path) -> Option<String> {
    let o = std::process::Command::new(prog).arg("version").env("HERDR_PROJECTS_INLINE", "1").stdin(std::process::Stdio::null()).output().ok()?;
    version_said(String::from_utf8_lossy(&o.stdout).trim())
}

pub struct Got { pub version: String, pub file: PathBuf }
/// One release file for this platform, beside `exe` under a spare name, its checksum checked and run once to ask its
/// version. Removed again when anything is wrong.
pub fn download(exe: &Path, version: Option<&str>, fetch: &dyn Fn(&str, &Path) -> Res<()>) -> Res<Got> {
    let Some(plat) = platform() else { crate::fail!("No release is made for {} {}: build it from source (cargo build --release).", std::env::consts::OS, std::env::consts::ARCH) };
    let (name, dir) = (asset_name(plat), exe.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")));
    let url = asset_url(&releases(exe), version, &name);
    let file = dir.join(format!(".{name}.{}.download", std::process::id()));
    let sums = PathBuf::from(format!("{}.sha256", file.display()));
    let got = (|| -> Res<Got> {
        fetch(&url, &file)?;
        fetch(&format!("{url}.sha256"), &sums)?;
        let Some(want) = std::fs::read_to_string(&sums).ok().and_then(|t| sum_in(&t)) else { crate::fail!("{url}.sha256 holds no checksum: nothing was changed") };
        let Some(have) = sha256(&file) else { crate::fail!("no sha256sum or openssl here to check the download: nothing was changed") };
        if have != want { crate::fail!("the download does not match its checksum: nothing was changed. Try again.") }
        crate::sys::set_mode(&file, 0o755)?;
        let Some(v) = version_of(&file) else { crate::fail!("the downloaded program does not run here: nothing was changed") };
        Ok(Got { version: v, file: file.clone() })
    })();
    let _ = std::fs::remove_file(&sums);
    if got.is_err() { let _ = std::fs::remove_file(&file); }
    got
}

/// `update [VERSION]`: the newest release (or that one) in place of the program that runs, in one step. An open board
/// and the keeper see the file change and load it by themselves. Never goes back to an older one unless named.
pub fn update(exe: &Path, version: Option<&str>, fetch: &dyn Fn(&str, &Path) -> Res<()>) -> Res<String> {
    if built_from_source(exe) { crate::fail!("This herdr-projects was built from source ({}): update it with git pull and cargo build --release.", exe.display()) }
    let got = download(exe, version, fetch)?;
    if version.is_none() && !newer(&got.version, VERSION) {
        let _ = std::fs::remove_file(&got.file);
        return Ok(format!("herdr-projects {VERSION} is the newest release."));
    }
    // Put in place over it, never written into: a running program keeps its old file.
    // On Windows, which will not replace a running program, replace_file moves that file aside first.
    if let Err(e) = crate::sys::replace_file(&got.file, exe) { let _ = std::fs::remove_file(&got.file); crate::fail!("could not put the new program in place of {}: {e}", exe.display()) }
    let named = platform().map(|p| exe.file_name().map(|n| n == asset_name(p).as_str()).unwrap_or(false)).unwrap_or(false);
    if named || version_file(exe).exists() { write_atomic(&version_file(exe), format!("{}\n", got.version).as_bytes())?; }
    crate::flow::send::shim();
    Ok(format!("Updated herdr-projects {VERSION} → {} ({}). An open board and the keeper load it by themselves.", got.version, exe.display()))
}

/// What is in the way of typing `herdr-projects` in a shell, as (what, what to do); None when the first one on the
/// PATH is the launcher `at` (or the shell script beside it, Git Bash's name for the same launcher, or a link to it) or this program.
pub fn path_problem(at: &Path, exe: &Path, path: &str) -> Option<(String, String)> {
    let dir = at.parent().map(Path::to_path_buf).unwrap_or_default();
    let real = |p: &Path| crate::sys::canonicalize(p).ok().map(crate::sys::plain_path);
    // As a shell looks: the first file of that name it may run.
    let runs = |f: &Path| crate::sys::is_program(f);
    match std::env::split_paths(path).flat_map(|d| crate::herdr::find::names("herdr-projects").into_iter().map(move |n| d.join(n))).find(|f| runs(f)) {
        Some(f) if f == at || (f.parent() == at.parent() && f.extension().is_none()) || real(&f).is_some() && (real(&f) == real(at) || real(&f) == real(exe)) => None,
        Some(f) => Some((format!("{} comes first on your PATH (an older install? `npm unlink -g herdr-projects` takes out the Node one)", f.display()), format!("Remove it, or put {} before it.", dir.display()))),
        None => Some(("herdr-projects is not on your PATH".into(), format!("To type herdr-projects in any shell, put its folder on your PATH (in ~/.profile or your shell's start-up file): export PATH=\"{}:$PATH\"", dir.display()))),
    }
}
/// `install`: the launcher in the data folder names this program (flow/launcher.rs writes it, the only writer), and what
/// is in the way of typing `herdr-projects` in a shell is said.
pub fn install(exe: &Path) -> String {
    let at = crate::flow::send::shim();
    let mut said = vec![format!("{} runs {}: agents and hooks use it.", at.display(), exe.display())];
    if let Some((what, fix)) = path_problem(&at, exe, &std::env::var("PATH").unwrap_or_default()) {
        said.push(if what.ends_with("not on your PATH") { fix } else { format!("{what}: {}{}", fix[..1].to_lowercase(), &fix[1..]) });
    }
    said.join("\n")
}

/// The commands: `install`, `update [VERSION]`.
pub fn main(cmd: &str, args: &[String]) -> i32 {
    let exe = bin();
    let exe = crate::sys::plain_path(crate::sys::canonicalize(&exe).unwrap_or(exe));
    let out = if cmd == "install" { Ok(install(&exe)) } else { update(&exe, args.first().map(String::as_str), &fetch) };
    match out { Ok(s) => { println!("{s}"); 0 } Err(e) => { eprintln!("{}", e.msg); 1 } }
}
