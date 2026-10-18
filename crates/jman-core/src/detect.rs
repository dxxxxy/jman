use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::Config;
use crate::{paths, version};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Jdk {
    /// Unique, human friendly identifier (usually the install folder name).
    pub id: String,
    /// The JDK home (the folder that contains `bin/java`).
    pub home: PathBuf,
    pub version: String,
    pub major: u32,
    pub vendor: Option<String>,
    pub arch: Option<String>,
    /// Installed by jman into `~/.jman/jdks`.
    pub managed: bool,
    /// Where the JDK was found, e.g. `jman`, `IntelliJ`, `Eclipse Adoptium`, `System`.
    pub source: String,
}

pub const SOURCE_JMAN: &str = "jman";
pub const SOURCE_CUSTOM: &str = "Custom folder";
pub const SOURCE_OTHER: &str = "Other";

/// Well-known folders that contain one JDK per child directory, with a label for each.
pub fn scan_roots() -> Vec<(PathBuf, String)> {
    let home = paths::home();
    let mut roots: Vec<(PathBuf, String)> = vec![
        (paths::jdks_dir(), SOURCE_JMAN.into()),
        (home.join(".jdks"), "IntelliJ".into()),
        (home.join(".gradle").join("jdks"), "Gradle".into()),
        (home.join(".sdkman").join("candidates").join("java"), "SDKMAN".into()),
    ];

    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramW6432"] {
            if let Some(pf) = std::env::var_os(var) {
                let pf = PathBuf::from(pf);
                for vendor in [
                    "Java",
                    "Eclipse Adoptium",
                    "Eclipse Foundation",
                    "Zulu",
                    "Microsoft",
                    "Amazon Corretto",
                    "BellSoft",
                    "Semeru",
                    "OpenJDK",
                ] {
                    roots.push((pf.join(vendor), vendor.into()));
                }
            }
        }
        roots.push((home.join("scoop").join("apps"), "scoop".into()));
    } else if cfg!(target_os = "macos") {
        roots.push(("/Library/Java/JavaVirtualMachines".into(), "System".into()));
        roots.push((home.join("Library/Java/JavaVirtualMachines"), "User".into()));
    } else {
        for root in ["/usr/lib/jvm", "/usr/java", "/opt/java", "/opt/jdk"] {
            roots.push((root.into(), "System".into()));
        }
    }

    let mut seen = HashSet::new();
    roots.retain(|(r, _)| seen.insert(r.clone()));
    roots
}

/// Finds every JDK in the well-known locations and in the configured extra folders,
/// sorted newest first.
pub fn detect_all() -> Vec<Jdk> {
    let config = Config::load();
    let mut found = Vec::new();
    for (root, source) in scan_roots() {
        scan_root(&root, &source, &mut found);
    }
    for dir in &config.extra_scan_dirs {
        // an extra folder may be a JDK itself or a folder of JDKs
        match probe(dir, SOURCE_CUSTOM) {
            Some(jdk) => found.push((jdk, false)),
            None => scan_root(dir, SOURCE_CUSTOM, &mut found),
        }
    }
    dedupe(found)
}

/// Pushes `(jdk, via_symlink)` for each JDK directly under `root`.
fn scan_root(root: &Path, source: &str, out: &mut Vec<(Jdk, bool)>) {
    let Ok(entries) = fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let via_symlink = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
        // macOS bundles keep the home in Contents/Home; scoop points apps/<name>/current at a version
        let candidates = [path.clone(), path.join("Contents").join("Home"), path.join("current")];
        if let Some(jdk) = candidates.iter().find_map(|c| probe(c, source)) {
            out.push((jdk, via_symlink));
        }
    }
}

/// Removes duplicates (e.g. `/usr/lib/jvm/default-java` symlinks), preferring real
/// directories, makes ids unique and sorts newest first.
fn dedupe(mut found: Vec<(Jdk, bool)>) -> Vec<Jdk> {
    found.sort_by_key(|(_, via_symlink)| *via_symlink);
    let mut seen = HashSet::new();
    let mut jdks: Vec<Jdk> = found
        .into_iter()
        .filter(|(jdk, _)| seen.insert(canonical_key(&jdk.home)))
        .map(|(jdk, _)| jdk)
        .collect();

    let mut counts: HashMap<String, u32> = HashMap::new();
    for jdk in &mut jdks {
        let n = counts.entry(jdk.id.to_lowercase()).or_default();
        *n += 1;
        if *n > 1 {
            jdk.id = format!("{}-{}", jdk.id, n);
        }
    }

    jdks.sort_by(|a, b| version::compare(&b.version, &a.version).then(a.id.cmp(&b.id)));
    jdks
}

/// A path key that compares equal for the same directory reached through different paths.
pub fn canonical_key(path: &Path) -> PathBuf {
    let p = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if cfg!(windows) {
        PathBuf::from(p.to_string_lossy().to_lowercase())
    } else {
        p
    }
}

/// Returns a [`Jdk`] if `home` looks like a JDK home directory.
pub fn probe(home: &Path, source: &str) -> Option<Jdk> {
    if !home.join("bin").join(paths::exe_name("java")).is_file() {
        return None;
    }
    let release = fs::read_to_string(home.join("release")).unwrap_or_default();
    let props = parse_release(&release);
    let id = id_for(home);
    let version = props
        .get("JAVA_VERSION")
        .cloned()
        .or_else(|| version_from_name(&id))
        .unwrap_or_else(|| "unknown".into());

    Some(Jdk {
        major: version::major(&version),
        home: dunce::simplified(home).to_path_buf(),
        vendor: props.get("IMPLEMENTOR").cloned(),
        arch: props.get("OS_ARCH").cloned(),
        id,
        version,
        managed: source == SOURCE_JMAN,
        source: source.to_string(),
    })
}

/// The folder a JDK was installed as: the home itself, minus macOS bundle internals
/// (`X.jdk/Contents/Home`) and scoop's `current`.
pub fn install_root(home: &Path) -> &Path {
    let mut path = home;
    if path.ends_with("Contents/Home") {
        path = path.parent().and_then(Path::parent).unwrap_or(path);
    }
    if path.file_name().is_some_and(|n| n == "current") {
        path = path.parent().unwrap_or(path);
    }
    path
}

/// The folder name, skipping macOS bundle internals and scoop's `current`.
fn id_for(home: &Path) -> String {
    let path = install_root(home);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    name.strip_suffix(".jdk").unwrap_or(&name).to_string()
}

fn version_from_name(name: &str) -> Option<String> {
    let start = name.find(|c: char| c.is_ascii_digit())?;
    let rest = &name[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '_' | '+')))
        .unwrap_or(rest.len());
    Some(rest[..end].trim_end_matches(['.', '_', '+']).to_string())
}

/// Parses the `KEY="value"` lines of a JDK `release` file.
pub fn parse_release(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_file() {
        let props = parse_release(
            "IMPLEMENTOR=\"Azul Systems, Inc.\"\nJAVA_VERSION=\"1.8.0_472\"\nOS_ARCH=\"amd64\"\n",
        );
        assert_eq!(props["IMPLEMENTOR"], "Azul Systems, Inc.");
        assert_eq!(props["JAVA_VERSION"], "1.8.0_472");
        assert_eq!(props["OS_ARCH"], "amd64");
    }

    #[test]
    fn version_from_folder_name() {
        assert_eq!(version_from_name("jdk-17.0.9+9").as_deref(), Some("17.0.9+9"));
        assert_eq!(version_from_name("azul-1.8.0_472").as_deref(), Some("1.8.0_472"));
        assert_eq!(version_from_name("temurin-21.jdk").as_deref(), Some("21"));
        assert_eq!(version_from_name("openjdk"), None);
    }

    #[test]
    fn ids_skip_bundle_internals() {
        assert_eq!(
            id_for(Path::new("/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home")),
            "temurin-21"
        );
        assert_eq!(id_for(Path::new("/home/u/scoop/apps/temurin21-jdk/current")), "temurin21-jdk");
        assert_eq!(id_for(Path::new("/home/u/.jdks/azul-21.0.9")), "azul-21.0.9");
    }

    #[test]
    fn probes_fake_jdk() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("corretto-17.0.9");
        fs::create_dir_all(home.join("bin")).unwrap();
        fs::write(home.join("bin").join(paths::exe_name("java")), "").unwrap();
        fs::write(home.join("release"), "JAVA_VERSION=\"17.0.9\"\nIMPLEMENTOR=\"Amazon.com Inc.\"\n").unwrap();

        let jdk = probe(&home, "IntelliJ").unwrap();
        assert_eq!(jdk.id, "corretto-17.0.9");
        assert_eq!(jdk.source, "IntelliJ");
        assert!(!jdk.managed);
        assert!(probe(&home, SOURCE_JMAN).unwrap().managed);
        assert_eq!(jdk.major, 17);
        assert_eq!(jdk.vendor.as_deref(), Some("Amazon.com Inc."));

        assert!(probe(dir.path(), SOURCE_OTHER).is_none());
    }
}
