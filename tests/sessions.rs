//! A herdr session's name, as its socket path spells it: the board keeps one file per session (a note left for the
//! board in a pane, a coordinator's state), and a name read wrong makes two sessions read each other's.
use crate::common;
use common::*;
use sw::herdr::cli::session_name;

#[test]
fn a_session_s_name_is_read_from_its_socket_on_unix_and_on_windows() {
    let _w = world();
    assert_eq!(session_name("/home/me/.config/herdr/sessions/work/herdr.sock"), "work");
    assert_eq!(session_name(r"C:\Users\me\.config\herdr\sessions\work\herdr.sock"), "work");
    assert_eq!(session_name("/home/me/.config/herdr/herdr.sock"), "default");
    assert_eq!(session_name(r"C:\Users\me\.config\herdr\herdr.sock"), "default");
}
