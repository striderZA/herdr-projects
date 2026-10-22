//! The machine's network facts: its addresses on its interfaces (loopback and unspecified ones left out), its host name, and
//! its load. Unix: getifaddrs, gethostname and getloadavg. Windows: GetAdaptersAddresses and the computer name; no load
//! average is read there, and a caller takes none as zero, as it always did.
use std::net::IpAddr;

/// The addresses of this machine's network interfaces, each once, loopback and unspecified ones left out.
#[cfg(unix)]
pub fn local_addrs() -> Vec<IpAddr> {
    let mut out = vec![];
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut list) } != 0 { return out; }
    let mut at = list;
    while !at.is_null() {
        let a = unsafe { &*at };
        if !a.ifa_addr.is_null() {
            let family = unsafe { (*a.ifa_addr).sa_family } as i32;
            let ip = if family == libc::AF_INET { Some(IpAddr::from(u32::from_be(unsafe { (*(a.ifa_addr as *const libc::sockaddr_in)).sin_addr.s_addr }).to_be_bytes())) }
                else if family == libc::AF_INET6 { Some(IpAddr::from(unsafe { (*(a.ifa_addr as *const libc::sockaddr_in6)).sin6_addr.s6_addr })) } else { None };
            if let Some(ip) = ip.filter(|ip| !ip.is_loopback() && !ip.is_unspecified()) { if !out.contains(&ip) { out.push(ip); } }
        }
        at = a.ifa_next;
    }
    unsafe { libc::freeifaddrs(list) };
    out
}

#[cfg(windows)]
pub fn local_addrs() -> Vec<IpAddr> {
    use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH};
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6};
    let mut out = vec![];
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 15_000;
    // Eight-byte cells, so that the buffer is aligned for the structures written into it.
    let mut buf: Vec<u64> = vec![0; (size as usize).div_ceil(8)];
    let mut rc = unsafe { GetAdaptersAddresses(AF_UNSPEC as u32, flags, std::ptr::null(), buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH, &mut size) };
    if rc == ERROR_BUFFER_OVERFLOW {
        buf = vec![0; (size as usize).div_ceil(8)];
        rc = unsafe { GetAdaptersAddresses(AF_UNSPEC as u32, flags, std::ptr::null(), buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH, &mut size) };
    }
    if rc != 0 { return out; }
    let mut adapter = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    while !adapter.is_null() {
        let a = unsafe { &*adapter };
        let mut unicast = a.FirstUnicastAddress;
        while !unicast.is_null() {
            let u = unsafe { &*unicast };
            let sa: *const SOCKADDR = u.Address.lpSockaddr;
            if !sa.is_null() {
                let family = unsafe { (*sa).sa_family };
                let ip = if family == AF_INET {
                    let v4 = unsafe { &*(sa as *const SOCKADDR_IN) };
                    Some(IpAddr::from(u32::from_be(unsafe { v4.sin_addr.S_un.S_addr }).to_be_bytes()))
                } else if family == AF_INET6 {
                    let v6 = unsafe { &*(sa as *const SOCKADDR_IN6) };
                    Some(IpAddr::from(unsafe { v6.sin6_addr.u.Byte }))
                } else {
                    None
                };
                if let Some(ip) = ip.filter(|ip| !ip.is_loopback() && !ip.is_unspecified()) { if !out.contains(&ip) { out.push(ip); } }
            }
            unicast = u.Next;
        }
        adapter = a.Next;
    }
    out
}

/// This machine's host name. Unix: gethostname. Windows: the computer name.
#[cfg(unix)]
pub fn host_name() -> String {
    let mut b = [0u8; 256];
    unsafe { libc::gethostname(b.as_mut_ptr() as *mut libc::c_char, b.len()); }
    String::from_utf8_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(0)]).into_owned()
}

#[cfg(windows)]
pub fn host_name() -> String { std::env::var("COMPUTERNAME").unwrap_or_default() }

/// The load over the last minute, as getloadavg gives it. None where it cannot be read, and on Windows, which has none.
#[cfg(unix)]
pub fn load_average() -> Option<f64> {
    let mut l = [0f64; 3];
    let n = unsafe { libc::getloadavg(l.as_mut_ptr(), 3) };
    if n < 1 { None } else { Some(l[0]) }
}

#[cfg(windows)]
pub fn load_average() -> Option<f64> { None }
