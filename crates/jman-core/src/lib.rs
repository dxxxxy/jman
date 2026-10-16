//! Core logic for jman.
//!
//! jman keeps `JAVA_HOME` and `PATH` pointed at one stable location (`~/.jman/current`).
//! Switching JDKs only re-points that link (a junction on Windows, a symlink elsewhere),
//! so already running terminals and IDEs pick up the new JDK on their next `java` call.

pub mod config;
pub mod detect;
pub mod doctor;
pub mod download;
pub mod link;
pub mod paths;
pub mod select;
pub mod setup;
pub mod version;

pub use detect::Jdk;

/// Lists the detected JDKs and resolves the one `~/.jman/current` points at.
pub fn current() -> Option<Jdk> {
    let target = link::current_target()?;
    let jdks = detect::detect_all();
    let key = detect::canonical_key(&target);
    jdks.into_iter()
        .find(|j| detect::canonical_key(&j.home) == key)
        .or_else(|| detect::probe(&target, false))
}

/// Switches `~/.jman/current` to the JDK matching `query` and returns it.
pub fn use_jdk(query: &str) -> anyhow::Result<Jdk> {
    let jdks = detect::detect_all();
    let jdk = select::resolve(query, &jdks)?.clone();
    link::switch(&jdk.home)?;
    Ok(jdk)
}
