//! One-time environment setup: point `JAVA_HOME` and `PATH` at `~/.jman`.

use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use crate::detect::{self, Jdk};
use crate::{link, paths};

pub mod unix;
#[cfg(windows)]
pub mod windows;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    /// Environment variables point at jman.
    pub configured: bool,
    /// What is missing or wrong, in plain words.
    pub issues: Vec<String>,
    /// Finishing setup requires an administrator prompt (Windows system variables).
    pub needs_elevation: bool,
}

pub fn status() -> SetupStatus {
    let mut issues = Vec::new();
    #[cfg(windows)]
    let needs_elevation = {
        let env = windows::env_status();
        issues.extend(env.issues);
        env.needs_elevation
    };
    #[cfg(not(windows))]
    let needs_elevation = {
        issues.extend(unix::issues());
        false
    };
    let configured = issues.is_empty();

    if !paths::bin_dir().join(paths::exe_name("jman")).is_file() {
        issues.push("the jman CLI is not installed in ~/.jman/bin".into());
    }
    match link::current_target() {
        None => issues.push("no JDK is selected yet".into()),
        Some(t) if !t.is_dir() => {
            issues.push(format!("the selected JDK no longer exists: {}", t.display()))
        }
        Some(_) => {}
    }
    SetupStatus { configured, issues, needs_elevation }
}

/// Applies everything that does not need administrator rights.
///
/// On Windows the system variables still need [`windows::apply_system`], run elevated.
pub fn apply_user() -> anyhow::Result<()> {
    fs::create_dir_all(paths::bin_dir())?;
    fs::create_dir_all(paths::jdks_dir())?;
    ensure_current_link()?;
    #[cfg(windows)]
    windows::apply_user()?;
    #[cfg(not(windows))]
    unix::apply()?;
    Ok(())
}

/// Makes sure `~/.jman/current` exists, preferring the JDK `JAVA_HOME` pointed at before jman.
pub fn ensure_current_link() -> anyhow::Result<Option<Jdk>> {
    if link::current_target().is_some_and(|t| t.is_dir()) {
        return Ok(None);
    }
    let jdks = detect::detect_all();
    let previous = std::env::var_os("JAVA_HOME")
        .map(|h| detect::canonical_key(Path::new(&h)))
        .and_then(|key| jdks.iter().find(|j| detect::canonical_key(&j.home) == key));
    let Some(jdk) = previous.or(jdks.first()) else {
        return Ok(None);
    };
    link::switch(&jdk.home)?;
    Ok(Some(jdk.clone()))
}

/// Copies the `jman` CLI binary into `~/.jman/bin`, which is on `PATH` after setup.
pub fn install_cli(source: &Path) -> anyhow::Result<()> {
    let dir = paths::bin_dir();
    fs::create_dir_all(&dir)?;
    let dest = dir.join(paths::exe_name("jman"));
    if detect::canonical_key(source) == detect::canonical_key(&dest) {
        return Ok(());
    }
    // copy next to the destination and rename over it: works even while the old
    // binary is running (Windows allows renaming a running executable)
    let tmp = dir.join(paths::exe_name("jman.new"));
    fs::copy(source, &tmp).with_context(|| format!("copying {}", source.display()))?;
    if dest.exists() {
        let old = dir.join(paths::exe_name("jman.old"));
        let _ = fs::remove_file(&old);
        let _ = fs::rename(&dest, &old);
    }
    fs::rename(&tmp, &dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
