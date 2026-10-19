//! Process checks and ending: whether a process is alive, when it started, how it is ended, how a detached group starts
//! and ends as one, and a process's command line and the process table. The unix half is the code the program has always
//! run (kill(2), setsid, /proc, ps). The windows half uses OpenProcess, TerminateProcess and GetProcessTimes; a detached
//! group is a named job object (taskkill /T where there is none); command lines come from PowerShell's CIM query.
use std::process::Command;

/// True when a process with this pid exists. Where it exists but may not be signalled (EPERM on Unix, access denied on
/// Windows), it is alive too.
#[cfg(unix)]
pub fn alive(pid: i64) -> bool {
    if pid <= 0 { return false; }
    let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
pub fn alive(pid: i64) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    if pid <= 0 { return false; }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32);
        if h.is_null() { return GetLastError() == ERROR_ACCESS_DENIED; }
        let mut code = 0u32;
        let running = GetExitCodeProcess(h, &mut code) != 0 && code == STILL_ACTIVE as u32;
        CloseHandle(h);
        running
    }
}

/// When the process started: a string that stays the same for as long as that process lives, so a pid reused later
/// gives another one. None where it cannot be read; macOS has no /proc, so there it is always None, as before.
#[cfg(unix)]
pub fn start_time(pid: i64) -> Option<String> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = &s[s.rfind(')')? + 2..];
    rest.split(' ').nth(19).map(|x| x.to_string())
}

#[cfg(windows)]
pub fn start_time(pid: i64) -> Option<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let zero = || FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut created, mut exited, mut kernel, mut user) = (zero(), zero(), zero(), zero());
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32);
        if h.is_null() { return None; }
        let ok = GetProcessTimes(h, &mut created, &mut exited, &mut kernel, &mut user) != 0;
        CloseHandle(h);
        ok.then(|| (((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64).to_string())
    }
}

/// Ends the process: SIGKILL when `force`, else SIGTERM. Windows has no polite stop for a process, so both end it with
/// TerminateProcess at once, and the program's own clean-up does not run there.
#[cfg(unix)]
pub fn signal(pid: i64, force: bool) -> bool {
    let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
    unsafe { libc::kill(pid as i32, sig) == 0 }
}

#[cfg(windows)]
pub fn signal(pid: i64, _force: bool) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
        if h.is_null() { return false; }
        let ok = TerminateProcess(h, 1) != 0;
        CloseHandle(h);
        ok
    }
}

/// Ends a detached group and everything it started: its process group on Unix (kill(-pid)); on Windows its named job,
/// or, where no job was made for it, its process tree (taskkill /T). Windows ends at once whatever `force` says.
#[cfg(unix)]
pub fn signal_group(pid: i64, force: bool) -> bool {
    let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
    unsafe { libc::kill(-(pid as i32), sig) == 0 }
}

#[cfg(windows)]
pub fn signal_group(pid: i64, _force: bool) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{OpenJobObjectW, TerminateJobObject};
    // The job access right to end a job's processes (winnt.h). windows-sys 0.61 does not export it.
    const JOB_OBJECT_TERMINATE: u32 = 0x0008;
    let name = group_name(pid);
    let job = unsafe { OpenJobObjectW(JOB_OBJECT_TERMINATE, 0, name.as_ptr()) };
    if !job.is_null() {
        let ended = unsafe { TerminateJobObject(job, 1) } != 0;
        unsafe { CloseHandle(job) };
        if ended { return true; }
    }
    taskkill_tree(pid)
}

/// Makes the command's child start in a group and session of its own, apart from this process's terminal. Windows: a
/// new process group, and no console window of its own; `join_group` then gives the group its job.
#[cfg(unix)]
pub fn detach(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe { cmd.pre_exec(|| { libc::setsid(); Ok(()) }); }
}

#[cfg(windows)]
pub fn detach(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

/// Puts a process that `detach` started into its group's named job, so `signal_group` ends all of it. Call it right
/// after the spawn. Unix: nothing to do, setsid made the group at the spawn. Windows: a process that cannot be joined
/// is in no job, and `signal_group` then falls back to taskkill /T.
#[cfg(unix)]
pub fn join_group(_pid: i64) {}

#[cfg(windows)]
pub fn join_group(pid: i64) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};
    let name = group_name(pid);
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), name.as_ptr());
        if job.is_null() { return; }
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid as u32);
        if !process.is_null() {
            AssignProcessToJobObject(job, process);
            CloseHandle(process);
        }
        // The job lives on while its processes do; closing this handle does not end them.
        CloseHandle(job);
    }
}

/// The command line the process was started with: `ps -p <pid> -o args=`, or PowerShell's CIM record on Windows.
#[cfg(unix)]
pub fn command_line(pid: i64) -> Option<String> {
    let o = Command::new("ps").args(["-p", &pid.to_string(), "-o", "args="]).output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

#[cfg(windows)]
pub fn command_line(pid: i64) -> Option<String> {
    let script = format!("(Get-CimInstance Win32_Process -Filter 'ProcessId={pid}').CommandLine");
    let o = Command::new("powershell").args(["-NoProfile", "-Command", &script]).output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Every process as `ps -eo pid=,etime=,args=` prints it: pid, elapsed time, command line, one per line. Windows prints
/// the same three fields from CIM.
#[cfg(unix)]
pub fn process_table() -> String {
    Command::new("ps").args(["-eo", "pid=,etime=,args="]).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

#[cfg(windows)]
pub fn process_table() -> String {
    const SCRIPT: &str = "Get-CimInstance Win32_Process | ForEach-Object { if ($_.CreationDate) { $e = (Get-Date) - $_.CreationDate; if ($e.Days -gt 0) { $t = '{0}-{1:00}:{2:00}:{3:00}' -f $e.Days, $e.Hours, $e.Minutes, $e.Seconds } elseif ($e.Hours -gt 0) { $t = '{0:00}:{1:00}:{2:00}' -f $e.Hours, $e.Minutes, $e.Seconds } else { $t = '{0:00}:{1:00}' -f $e.Minutes, $e.Seconds }; '{0} {1} {2}' -f $_.ProcessId, $t, $_.CommandLine } }";
    Command::new("powershell").args(["-NoProfile", "-Command", SCRIPT]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// The name of a detached group's job, from the pid of its first process.
#[cfg(windows)]
fn group_name(pid: i64) -> Vec<u16> {
    format!("herdr-projects-group-{pid}").encode_utf16().chain(std::iter::once(0)).collect()
}

/// Ends a process and its tree with taskkill. /F: a console program cannot be asked to stop politely from here.
#[cfg(windows)]
fn taskkill_tree(pid: i64) -> bool {
    use std::process::Stdio;
    Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false)
}
