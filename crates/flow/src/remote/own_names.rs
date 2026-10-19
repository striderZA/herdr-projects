//! The names this machine goes by, as another machine's herdr list of machines may name it (remote/sidebar.rs): its
//! host name (whole; a `.local` one also as the bare name it is the local network's form of), and the addresses of
//! its network interfaces (a Tailscale address too: Tailscale is an interface), never a loopback one. Read once,
//! without starting a program.
use std::net::IpAddr;

/// This machine's names, lowercase: read once, the first time they are asked for.
pub fn own_names() -> Vec<String> {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| { let mut v = host_names(&super::check::hostname()); v.extend(addresses().iter().map(|a| a.to_string())); v }).clone()
}
/// A host name as names: the whole of it, and for `name.local` (the local network's form of a bare name, as a Mac
/// names itself) also `name`. No other part of a name is taken for a name: `build.prod.example` is not `build`.
pub fn host_names(host: &str) -> Vec<String> {
    let h = host.trim().trim_end_matches('.').to_lowercase();
    if h.is_empty() || h == "localhost" { return vec![]; }
    match h.strip_suffix(".local").filter(|b| !b.is_empty() && !b.contains('.')) { Some(b) => vec![h.clone(), b.to_string()], None => vec![h] }
}
/// The addresses of this machine's network interfaces, loopback ones left out.
fn addresses() -> Vec<IpAddr> { crate::platform::net::local_addrs() }
