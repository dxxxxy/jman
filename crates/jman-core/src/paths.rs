use std::path::PathBuf;

/// Root jman directory. `JMAN_HOME` overrides the default of `~/.jman`.
pub fn jman_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("JMAN_HOME").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    home().join(".jman")
}

pub fn home() -> PathBuf {
    dirs::home_dir().expect("could not determine the home directory")
}

/// The stable path that `JAVA_HOME` points at.
pub fn current_link() -> PathBuf {
    jman_dir().join("current")
}

/// Directory on `PATH` that holds the `jman` CLI.
pub fn bin_dir() -> PathBuf {
    jman_dir().join("bin")
}

/// Directory that JDKs downloaded by jman are extracted into.
pub fn jdks_dir() -> PathBuf {
    jman_dir().join("jdks")
}

pub fn config_file() -> PathBuf {
    jman_dir().join("config.json")
}

/// The `PATH` entries jman needs, in priority order.
pub fn path_entries() -> Vec<PathBuf> {
    vec![current_link().join("bin"), bin_dir()]
}

pub fn exe_name(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}
