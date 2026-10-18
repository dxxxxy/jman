use std::path::PathBuf;

use serde::Serialize;

use crate::detect::{self, Jdk};
use crate::setup::{self, SetupStatus};
use crate::paths;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub jman_dir: PathBuf,
    pub current: Option<Jdk>,
    pub setup: SetupStatus,
    /// The `java` this process would run.
    pub java_on_path: Option<PathBuf>,
    pub warnings: Vec<String>,
}

pub fn run() -> Report {
    let setup = setup::status();
    let current = crate::current();
    let java_on_path = which::which("java").ok();
    let mut warnings = Vec::new();

    let link = detect::canonical_key(&paths::current_link());
    match &java_on_path {
        None => warnings.push("`java` is not on this process's PATH".into()),
        Some(java) => {
            // resolving the junction/symlink gives the JDK home; compare that with the link target
            let home = java.parent().and_then(|bin| bin.parent()).map(detect::canonical_key);
            if home.as_ref() != Some(&link) {
                warnings.push(format!("`java` resolves to {} instead of jman's", java.display()));
            }
        }
    }
    let env_home = std::env::var_os("JAVA_HOME").map(|h| detect::canonical_key(&PathBuf::from(h)));
    if env_home.as_ref() != Some(&link) {
        warnings.push("JAVA_HOME in this process does not point at jman".into());
    }
    if setup.configured && !warnings.is_empty() {
        warnings.push(
            "programs started before setup keep their old environment; open a new terminal or restart the IDE once"
                .into(),
        );
    }

    Report { jman_dir: paths::jman_dir(), current, setup, java_on_path, warnings }
}
