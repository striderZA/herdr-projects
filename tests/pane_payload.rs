//! PowerShell payloads typed into agent panes preserve titles containing shell metacharacters.
#![cfg(windows)]
use crate::common;

#[test]
fn a_special_title_is_one_unchanged_powershell_argument() {
    let title = "double\" single' dollar$ ampersand& tick` and spaces";
    let command = sw::agents::hand_limits::typed_command("Write-Output", &[title.into()]);
    assert!(command.contains("'double\" single'' dollar$ ampersand& tick` and spaces'"), "{command}");
    assert!(!command.contains("export ") && !command.contains("exec "));
    let output = std::process::Command::new("pwsh")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &command])
        .output().expect("PowerShell 7 is required for this Windows test");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), title);
}
