//! Quoting for the two shells a command is typed into: POSIX sh and PowerShell (`util::shell_word`). On Windows a
//! round trip through PowerShell checks the quoting against the shell itself.
use sw::util::{shell_word, Shell};

#[test]
fn posix_quoting_is_unchanged() {
    assert_eq!(shell_word("plain", Shell::Posix), "'plain'");
    assert_eq!(shell_word("it's", Shell::Posix), "'it'\\''s'");
    assert_eq!(shell_word("a b $x \"q\"", Shell::Posix), "'a b $x \"q\"'");
}

#[test]
fn powershell_quoting_doubles_single_quotes_and_nothing_else() {
    assert_eq!(shell_word("it's", Shell::PowerShell), "'it''s'");
    assert_eq!(shell_word("a b $x \"q\" `n", Shell::PowerShell), "'a b $x \"q\" `n'");
}

#[cfg(windows)]
#[test]
fn powershell_round_trip_keeps_spaces_quotes_and_dollars() {
    let text = "a 'b' \"c\" $d `e; ";
    let quoted = shell_word(text, Shell::PowerShell);
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &format!("Write-Output {quoted}")])
        .output()
        .expect("powershell.exe runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim_end_matches(['\r', '\n']), text);
}
