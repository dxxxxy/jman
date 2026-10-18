use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Extra folders to scan for JDKs, on top of the well-known locations.
    pub extra_scan_dirs: Vec<PathBuf>,
}

impl Config {
    pub fn load() -> Config {
        fs::read_to_string(paths::config_file())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let file = paths::config_file();
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", file.display()))
    }
}
