//! PowerShell payloads typed into agent panes preserve titles containing shell metacharacters.
#![cfg(windows)]
use crate::common;

#[test]
fn a_special_title_is_one_unchanged_powershell_argument() {
    let title = "double\" single' dollar$ ampersand& tick` and spaces";
    let command = sw::agents::hand_limits::typed_command("Write-Output", &[title.into()]);
    assert!(command.contains("'double\" single'' dollar$ ampersand& tick` and spaces'"), "{command}");
    assert!(!command.contains("export ") && !command.contains("exec "));
    // PATH is process-wide and other tests in this process narrow it, so PowerShell 7 is run from its install folder.
    let pwsh = std::path::Path::new(&std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into())).join("PowerShell").join("7").join("pwsh.exe");
    let output = std::process::Command::new(&pwsh)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &command])
        .output().unwrap_or_else(|e| panic!("PowerShell 7 is required for this Windows test ({}): {e}", pwsh.display()));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), title);
}
