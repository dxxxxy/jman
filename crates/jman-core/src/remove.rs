//! Removing JDKs: jman's own downloads, vendor installs (through their uninstaller),
//! package-manager installs and plain folders.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use serde::Serialize;

use crate::detect::{self, Jdk};
use crate::{link, paths};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Removal {
    /// Delete the JDK folder, elevated if the user can't write its parent.
    Delete { path: PathBuf, needs_admin: bool },
    /// Run the uninstaller registered in Windows "Installed apps".
    Uninstaller { name: String, command: String },
    /// Remove the package that owns the JDK: `apt`, `dnf`, `brew`, `brew-cask` or `scoop`.
    Package { manager: String, package: String, affected: Vec<String> },
}

/// A one-line description of what [`execute`] will do.
pub fn describe(removal: &Removal) -> String {
    match removal {
        Removal::Delete { path, needs_admin: false } => format!("delete {}", path.display()),
        Removal::Delete { path, needs_admin: true } => {
            format!("delete {} (needs administrator rights)", path.display())
        }
        Removal::Uninstaller { name, .. } => format!("run the {name} uninstaller"),
        Removal::Package { manager, package, affected } if affected.len() > 1 => {
            format!("remove {package} with {manager}, which also removes: {}", affected.join(", "))
        }
        Removal::Package { manager, package, .. } => format!("remove {package} with {manager}"),
    }
}

/// Decides how to remove `jdk`.
pub fn plan(jdk: &Jdk) -> anyhow::Result<Removal> {
    let home_key = detect::canonical_key(&jdk.home);
    if link::current_target().is_some_and(|t| detect::canonical_key(&t) == home_key) {
        bail!("{} is the selected JDK; switch to another one first", jdk.id);
    }
    let root = detect::install_root(&jdk.home).to_path_buf();

    if jdk.managed {
        return Ok(Removal::Delete { path: root, needs_admin: false });
    }
    #[cfg(windows)]
    if let Some((name, command)) = windows::find_uninstaller(&jdk.home) {
        return Ok(Removal::Uninstaller { name, command });
    }
    if jdk.source == "scoop" {
        let app = root.file_name().context("scoop app has no name")?.to_string_lossy().into_owned();
        return Ok(Removal::Package { manager: "scoop".into(), affected: vec![app.clone()], package: app });
    }
    #[cfg(target_os = "linux")]
    if let Some(removal) = linux::owning_package(&jdk.home) {
        return Ok(removal);
    }
    #[cfg(target_os = "macos")]
    if let Some(removal) = brew_package(&jdk.home) {
        return Ok(removal);
    }

    if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
        bail!(
            "{} is a link to {}; remove that JDK instead",
            root.display(),
            dunce::canonicalize(&root).unwrap_or_default().display()
        );
    }
    let needs_admin = !root.parent().is_some_and(is_writable);
    Ok(Removal::Delete { path: root, needs_admin })
}

/// Carries out a [`plan`]. `self_elevate` deletes a folder from an elevated copy of the
/// running program (Windows only; other platforms use `osascript` / `pkexec`).
pub fn execute(jdk: &Jdk, removal: &Removal, self_elevate: impl Fn(&Path) -> anyhow::Result<()>) -> anyhow::Result<()> {
    match removal {
        Removal::Delete { path, needs_admin: false } => {
            fs::remove_dir_all(path).with_context(|| format!("deleting {}", path.display()))?
        }
        Removal::Delete { path, needs_admin: true } => {
            if cfg!(windows) {
                self_elevate(path)?
            } else if cfg!(target_os = "macos") {
                run(Command::new("osascript").arg("-e").arg(format!(
                    "do shell script \"rm -rf \" & quoted form of \"{}\" with administrator privileges",
                    applescript_escape(&path.to_string_lossy())
                )))?
            } else {
                run(Command::new("pkexec").args(["rm", "-rf", "--"]).arg(path))?
            }
        }
        Removal::Uninstaller { command, .. } => {
            #[cfg(windows)]
            windows::run_uninstaller(command)?;
            #[cfg(not(windows))]
            bail!("cannot run Windows uninstaller {command}");
        }
        Removal::Package { manager, package, .. } => {
            let mut cmd = match manager.as_str() {
                "apt" => Command::new("pkexec"),
                "dnf" => Command::new("pkexec"),
                "brew" | "brew-cask" => Command::new("brew"),
                "scoop" => Command::new("powershell"),
                other => bail!("unknown package manager {other}"),
            };
            match manager.as_str() {
                "apt" => cmd.args(["apt-get", "remove", "-y", package]),
                "dnf" => cmd.args(["dnf", "remove", "-y", package]),
                "brew" => cmd.args(["uninstall", package]),
                "brew-cask" => cmd.args(["uninstall", "--cask", package]),
                _ => cmd.args(["-NoProfile", "-Command", &format!("scoop uninstall {package}")]),
            };
            run(&mut cmd)?
        }
    }

    if jdk.home.join("bin").join(paths::exe_name("java")).exists() {
        bail!("{} is still installed (was the uninstall cancelled?)", jdk.id);
    }
    // uninstallers sometimes leave an empty folder behind
    let root = detect::install_root(&jdk.home);
    if root.is_dir() {
        let _ = fs::remove_dir_all(root);
    }
    Ok(())
}

/// The elevated half of `Delete { needs_admin: true }` on Windows. Re-checks that the
/// path really is a JDK, since it arrives on the command line of an elevated process.
pub fn delete_elevated(path: &Path) -> anyhow::Result<()> {
    if detect::probe(path, detect::SOURCE_OTHER).is_none() {
        bail!("{} is not a JDK", path.display());
    }
    fs::remove_dir_all(path).with_context(|| format!("deleting {}", path.display()))
}

fn run(cmd: &mut Command) -> anyhow::Result<()> {
    let out = cmd.output().with_context(|| format!("starting {:?}", cmd.get_program()))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("{:?} failed: {}", cmd.get_program(), stderr.trim());
    }
    Ok(())
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".jman-write-test-{}", std::process::id()));
    let ok = fs::write(&probe, b"").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Homebrew formulae live in `.../Cellar/<formula>/...`, casks in `.../Caskroom/<cask>/...`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn brew_package(home: &Path) -> Option<Removal> {
    let real = dunce::canonicalize(home).ok()?;
    let parts: Vec<String> = real.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let (manager, i) = parts.iter().enumerate().find_map(|(i, p)| match p.as_str() {
        "Cellar" => Some(("brew", i)),
        "Caskroom" => Some(("brew-cask", i)),
        _ => None,
    })?;
    let package = parts.get(i + 1)?.clone();
    Some(Removal::Package { manager: manager.into(), affected: vec![package.clone()], package })
}

/// Package names from `apt-get -s remove` output (`Remv <name> [<version>]` lines).
pub fn parse_apt_simulation(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|l| l.strip_prefix("Remv "))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    fn stdout(cmd: &mut Command) -> Option<String> {
        let out = cmd.output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// The apt/dnf package that owns the JDK's `java` binary, if any.
    pub fn owning_package(home: &Path) -> Option<Removal> {
        let java = dunce::canonicalize(home.join("bin").join("java")).ok()?;
        if which::which("dpkg").is_ok() {
            // "openjdk-17-jre-headless:amd64: /usr/lib/jvm/.../bin/java"
            let line = stdout(Command::new("dpkg").arg("-S").arg(&java))?;
            let package = line.split(": ").next()?.to_string();
            let affected = stdout(Command::new("apt-get").args(["-s", "remove", &package]))
                .map(|o| parse_apt_simulation(&o))
                .filter(|a| !a.is_empty())
                .unwrap_or_else(|| vec![package.clone()]);
            return Some(Removal::Package { manager: "apt".into(), package, affected });
        }
        if which::which("rpm").is_ok() {
            let package = stdout(Command::new("rpm").args(["-qf", "--qf", "%{NAME}"]).arg(&java))?;
            return Some(Removal::Package { manager: "dnf".into(), affected: vec![package.clone()], package });
        }
        None
    }
}

#[cfg(windows)]
pub mod windows {
    use std::os::windows::process::CommandExt;

    use winreg::RegKey;
    use winreg::enums::*;

    use super::*;

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_WOW: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// An "Installed apps" entry: (InstallLocation, DisplayName, UninstallString).
    pub type Entry = (String, String, String);

    fn entries() -> Vec<Entry> {
        let hives = [(HKEY_LOCAL_MACHINE, UNINSTALL), (HKEY_LOCAL_MACHINE, UNINSTALL_WOW), (HKEY_CURRENT_USER, UNINSTALL)];
        let mut out = Vec::new();
        for (hive, path) in hives {
            let Ok(key) = RegKey::predef(hive).open_subkey(path) else { continue };
            for name in key.enum_keys().flatten() {
                let Ok(app) = key.open_subkey(&name) else { continue };
                let get = |v: &str| app.get_value::<String, _>(v).unwrap_or_default();
                let (location, cmd) = (get("InstallLocation"), get("UninstallString"));
                if !location.is_empty() && !cmd.is_empty() {
                    out.push((location, get("DisplayName"), cmd));
                }
            }
        }
        out
    }

    /// Finds the entry whose install location is the JDK's folder.
    pub fn match_entry(home: &Path, entries: &[Entry]) -> Option<(String, String)> {
        let targets = [detect::canonical_key(home), detect::canonical_key(detect::install_root(home))];
        entries.iter().find_map(|(location, name, cmd)| {
            let key = detect::canonical_key(Path::new(location.trim().trim_end_matches(['\\', '/'])));
            targets.contains(&key).then(|| (name.clone(), uninstall_command(cmd)))
        })
    }

    /// MSI entries often use `/I{GUID}` (opens "modify"); `/X{GUID}` uninstalls.
    fn uninstall_command(cmd: &str) -> String {
        let lower = cmd.to_lowercase();
        if lower.contains("msiexec")
            && let Some(i) = lower.find("/i{")
        {
            return format!("{}/X{}", &cmd[..i], &cmd[i + 2..]);
        }
        cmd.to_string()
    }

    pub fn find_uninstaller(home: &Path) -> Option<(String, String)> {
        match_entry(home, &entries())
    }

    /// Runs an UninstallString as Windows would (it is a command line, not a path).
    pub fn run_uninstaller(command: &str) -> anyhow::Result<()> {
        let status = Command::new("cmd")
            .arg("/C")
            .raw_arg(command)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .context("starting the uninstaller")?;
        // 1605: product not installed, 1614: already uninstalled
        match status.code() {
            Some(0 | 1605 | 1614 | 3010) => Ok(()),
            Some(1602) => bail!("the uninstall was cancelled"),
            code => bail!("the uninstaller failed (exit code {code:?})"),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn matches_install_location_and_fixes_msi_command() {
            let entries = vec![
                (r"C:\Program Files\Other".into(), "Other".into(), "other.exe".into()),
                (
                    r"C:\Program Files\Eclipse Adoptium\jdk-21.0.5.11-hotspot\".into(),
                    "Eclipse Temurin JDK 21".into(),
                    "MsiExec.exe /I{ABC-123}".into(),
                ),
            ];
            let home = Path::new(r"C:\Program Files\Eclipse Adoptium\jdk-21.0.5.11-hotspot");
            assert_eq!(
                match_entry(home, &entries),
                Some(("Eclipse Temurin JDK 21".into(), "MsiExec.exe /X{ABC-123}".into()))
            );
            assert_eq!(match_entry(Path::new(r"C:\jdks\x"), &entries), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_jdk(home: &Path, source: &str) -> Jdk {
        fs::create_dir_all(home.join("bin")).unwrap();
        fs::write(home.join("bin").join(paths::exe_name("java")), "").unwrap();
        detect::probe(home, source).unwrap()
    }

    #[test]
    fn plans_and_deletes_user_folder() {
        let dir = tempfile::tempdir().unwrap();
        let jdk = fake_jdk(&dir.path().join("corretto-17"), "Custom folder");
        let removal = plan(&jdk).unwrap();
        assert_eq!(removal, Removal::Delete { path: jdk.home.clone(), needs_admin: false });
        execute(&jdk, &removal, |_| panic!("must not elevate")).unwrap();
        assert!(!jdk.home.exists());
    }

    #[test]
    fn refuses_symlinked_jdk() {
        let dir = tempfile::tempdir().unwrap();
        let real = fake_jdk(&dir.path().join("real"), "Custom folder");
        let link_path = dir.path().join("alias");
        #[cfg(windows)]
        junction::create(&real.home, &link_path).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real.home, &link_path).unwrap();
        let alias = detect::probe(&link_path, "Custom folder").unwrap();
        assert!(plan(&alias).is_err());
        assert!(real.home.exists());
    }

    #[test]
    fn parses_apt_simulation() {
        let out = "NOTE: This is only a simulation!\nRemv openjdk-17-jdk [17.0.9+9-1]\nRemv openjdk-17-jre-headless [17.0.9+9-1]\n";
        assert_eq!(parse_apt_simulation(out), vec!["openjdk-17-jdk", "openjdk-17-jre-headless"]);
    }

    #[test]
    fn describes_plans() {
        let pkg = Removal::Package {
            manager: "apt".into(),
            package: "a".into(),
            affected: vec!["a".into(), "b".into()],
        };
        assert_eq!(describe(&pkg), "remove a with apt, which also removes: a, b");
    }
}
