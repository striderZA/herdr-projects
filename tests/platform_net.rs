//! The platform layer's network facts: the machine has a name, and its addresses are real ones, neither loopback nor
//! unspecified.
use sw::platform::net;

#[test]
fn the_machine_has_a_name() {
    assert!(!net::host_name().is_empty());
}

#[test]
fn no_local_address_is_unspecified_or_loopback() {
    assert!(net::local_addrs().iter().all(|a| !a.is_unspecified() && !a.is_loopback()));
}
