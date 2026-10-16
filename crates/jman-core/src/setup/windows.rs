//! Windows setup: system `JAVA_HOME` and `PATH` in the registry.
//!
//! Values are read and written raw as `REG_EXPAND_SZ`, so `%VAR%` references in PATH
//! survive and there is no 1024 character limit (unlike `setx`).

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use anyhow::{Context, bail};
use winreg::enums::*;
use winreg::{RegKey, RegValue};

use crate::paths;

const SYSTEM_ENV: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const USER_ENV: &str = "Environment";

pub struct EnvStatus {
    pub issues: Vec<String>,
    pub needs_elevation: bool,
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn read_string(key: &RegKey, name: &str) -> Option<String> {
    let value = key.get_raw_value(name).ok()?;
    let units: Vec<u16> = value.bytes.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    Some(String::from_utf16_lossy(&units[..end]))
}

fn write_string(key: &RegKey, name: &str, value: &str, vtype: RegType) -> io::Result<()> {
    let bytes = wide(OsStr::new(value)).iter().flat_map(|u| u.to_le_bytes()).collect();
    key.set_raw_value(name, &RegValue { bytes, vtype })
}

fn normalize(entry: &str) -> String {
    entry.trim().trim_end_matches(['\\', '/']).to_lowercase()
}

fn same_path(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

/// Moves `entries` to the front of a `;` separated PATH value, removing duplicates
/// of them further down. Every other entry is kept exactly as written.
pub fn with_entries_first(path: &str, entries: &[String]) -> String {
    let rest = path
        .split(';')
        .filter(|e| !e.trim().is_empty())
        .filter(|e| !entries.iter().any(|x| same_path(e, x)));
    entries.iter().map(String::as_str).chain(rest).collect::<Vec<_>>().join(";")
}

/// True if PATH starts with exactly `entries`.
pub fn has_entries_first(path: &str, entries: &[String]) -> bool {
    let mut parts = path.split(';').filter(|e| !e.trim().is_empty());
    entries.iter().all(|x| parts.next().is_some_and(|e| same_path(e, x)))
}

fn jman_entries() -> Vec<String> {
    paths::path_entries().iter().map(|p| p.display().to_string()).collect()
}

fn java_home_value() -> String {
    paths::current_link().display().to_string()
}

pub fn env_status() -> EnvStatus {
    let mut issues = Vec::new();
    let mut needs_elevation = false;
    let home = java_home_value();

    let system = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(SYSTEM_ENV);
    match system {
        Ok(key) => {
            if !read_string(&key, "JAVA_HOME").is_some_and(|v| same_path(&v, &home)) {
                issues.push("system JAVA_HOME does not point at jman".into());
                needs_elevation = true;
            }
            if !has_entries_first(&read_string(&key, "Path").unwrap_or_default(), &jman_entries()) {
                issues.push("jman is not at the front of the system PATH".into());
                needs_elevation = true;
            }
        }
        Err(e) => issues.push(format!("cannot read system environment: {e}")),
    }

    // a user JAVA_HOME overrides the system one
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(USER_ENV)
        && let Some(v) = read_string(&key, "JAVA_HOME")
            && !same_path(&v, &home) {
                issues.push(format!("user JAVA_HOME overrides jman: {v}"));
            }

    EnvStatus { issues, needs_elevation }
}

/// Points the user `JAVA_HOME` (which would otherwise override the system one) at jman.
pub fn apply_user() -> anyhow::Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(USER_ENV)?;
    if read_string(&key, "JAVA_HOME").is_some() {
        write_string(&key, "JAVA_HOME", &java_home_value(), REG_SZ)?;
    }
    broadcast_change();
    Ok(())
}

/// Sets the system `JAVA_HOME` and puts jman first on the system PATH. Needs admin.
pub fn apply_system() -> anyhow::Result<()> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(SYSTEM_ENV, KEY_READ | KEY_WRITE)
        .context("opening the system environment (administrator rights are required)")?;
    let path = read_string(&key, "Path").context("reading the system PATH")?;
    if path.trim().is_empty() {
        bail!("refusing to edit an empty system PATH");
    }
    write_string(&key, "JAVA_HOME", &java_home_value(), REG_SZ)?;
    write_string(&key, "Path", &with_entries_first(&path, &jman_entries()), REG_EXPAND_SZ)?;
    broadcast_change();
    Ok(())
}

/// True if this process can write the system environment.
pub fn can_write_system() -> bool {
    RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(SYSTEM_ENV, KEY_WRITE).is_ok()
}

/// Tells running programs (Explorer in particular) that the environment changed, so
/// terminals started from now on see the new variables.
pub fn broadcast_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let param = wide(OsStr::new("Environment"));
    let mut result = 0;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            param.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            3000,
            &mut result,
        );
    }
}

/// Runs `exe args` elevated (UAC prompt), waits for it and returns its exit code.
pub fn run_elevated(exe: &Path, args: &str) -> anyhow::Result<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb = wide(OsStr::new("runas"));
    let file = wide(exe.as_os_str());
    let params = wide(OsStr::new(args));
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = params.as_ptr();
        info.nShow = SW_HIDE;
        if ShellExecuteExW(&mut info) == 0 {
            let err = io::Error::last_os_error();
            // ERROR_CANCELLED: the user declined the UAC prompt
            if err.raw_os_error() == Some(1223) {
                bail!("the administrator prompt was declined");
            }
            return Err(err).context("starting elevated setup");
        }
        if info.hProcess.is_null() {
            bail!("elevated setup did not start");
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 1;
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<String> {
        vec![r"C:\Users\me\.jman\current\bin".into(), r"C:\Users\me\.jman\bin".into()]
    }

    #[test]
    fn prepends_and_preserves_variables() {
        let path = r"%SystemRoot%\system32;%SystemRoot%;C:\Program Files\Common Files\Oracle\Java\javapath;";
        let new = with_entries_first(path, &entries());
        assert_eq!(
            new,
            r"C:\Users\me\.jman\current\bin;C:\Users\me\.jman\bin;%SystemRoot%\system32;%SystemRoot%;C:\Program Files\Common Files\Oracle\Java\javapath"
        );
        assert!(has_entries_first(&new, &entries()));
        assert!(!has_entries_first(path, &entries()));
    }

    #[test]
    fn moves_existing_entries_to_front_without_duplicates() {
        let path = r"C:\a;c:\users\me\.jman\current\bin\;C:\b";
        let new = with_entries_first(path, &entries());
        assert_eq!(new, r"C:\Users\me\.jman\current\bin;C:\Users\me\.jman\bin;C:\a;C:\b");
        assert_eq!(with_entries_first(&new, &entries()), new);
    }
}
