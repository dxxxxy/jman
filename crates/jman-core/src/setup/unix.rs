//! macOS / Linux setup: a marked block in the shell profiles.
//!
//! The text helpers are platform independent so they are tested everywhere.

use std::fs;
use std::path::PathBuf;

use crate::paths;

const START: &str = "# >>> jman >>>";
const END: &str = "# <<< jman <<<";

fn quoted(path: &std::path::Path) -> String {
    format!("\"{}\"", path.display())
}

/// The block added to POSIX shell profiles.
pub fn posix_block() -> String {
    let entries: Vec<String> = paths::path_entries().iter().map(|p| p.display().to_string()).collect();
    format!(
        "{START}\n# managed by jman, do not edit\nexport JAVA_HOME={}\nexport PATH=\"{}:$PATH\"\n{END}\n",
        quoted(&paths::current_link()),
        entries.join(":"),
    )
}

pub fn fish_config() -> String {
    let entries: Vec<String> = paths::path_entries().iter().map(|p| quoted(p)).collect();
    format!(
        "# managed by jman, do not edit\nset -gx JAVA_HOME {}\nfish_add_path --global --move --path {}\n",
        quoted(&paths::current_link()),
        entries.join(" "),
    )
}

/// systemd `environment.d` config, picked up by graphical sessions on most Linux desktops.
pub fn environment_d() -> String {
    let entries: Vec<String> = paths::path_entries().iter().map(|p| p.display().to_string()).collect();
    format!(
        "# managed by jman\nJAVA_HOME={}\nPATH={}:${{PATH}}\n",
        paths::current_link().display(),
        entries.join(":"),
    )
}

/// Inserts or replaces the jman block in a profile, leaving the rest untouched.
pub fn upsert_block(content: &str, block: &str) -> String {
    let stripped = remove_block(content);
    let mut out = stripped.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(block);
    out
}

/// Removes the jman block from a profile.
pub fn remove_block(content: &str) -> String {
    let (Some(start), Some(end)) = (content.find(START), content.find(END)) else {
        return content.to_string();
    };
    if end < start {
        return content.to_string();
    }
    let mut end = end + END.len();
    if content[end..].starts_with('\n') {
        end += 1;
    }
    let before = content[..start].trim_end_matches('\n');
    let after = &content[end..];
    match (before.is_empty(), after.is_empty()) {
        (true, _) => after.to_string(),
        (false, true) => format!("{before}\n"),
        (false, false) => format!("{before}\n{after}"),
    }
}

/// Shell profiles that get the block: every common one that exists, plus the
/// platform's default shell profile even if it doesn't exist yet.
fn profile_files() -> Vec<PathBuf> {
    let home = paths::home();
    let default = if cfg!(target_os = "macos") { ".zshrc" } else { ".bashrc" };
    [".zshrc", ".bashrc", ".bash_profile", ".profile"]
        .iter()
        .map(|f| home.join(f))
        .filter(|p| p.exists() || p.ends_with(default))
        .collect()
}

fn fish_file() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("fish");
    dir.is_dir().then(|| dir.join("conf.d").join("jman.fish"))
}

fn environment_d_file() -> Option<PathBuf> {
    cfg!(target_os = "linux").then(dirs::config_dir).flatten().map(|d| d.join("environment.d").join("jman.conf"))
}

#[cfg(target_os = "macos")]
fn launch_agent_file() -> PathBuf {
    paths::home().join("Library/LaunchAgents/studio.dreamys.jman.env.plist")
}

#[cfg(target_os = "macos")]
fn launch_agent() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>studio.dreamys.jman.env</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/launchctl</string><string>setenv</string>
    <string>JAVA_HOME</string><string>{}</string>
  </array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
        paths::current_link().display()
    )
}

/// Everything that is not set up yet.
pub fn issues() -> Vec<String> {
    let mut issues = Vec::new();
    let block = posix_block();
    for file in profile_files() {
        let content = fs::read_to_string(&file).unwrap_or_default();
        if !content.contains(&block) {
            issues.push(format!("{} does not set JAVA_HOME to jman", file.display()));
        }
    }
    if let Some(file) = fish_file()
        && fs::read_to_string(&file).ok().as_deref() != Some(fish_config().as_str()) {
            issues.push(format!("{} is missing", file.display()));
        }
    issues
}

pub fn apply() -> anyhow::Result<()> {
    let block = posix_block();
    for file in profile_files() {
        let content = fs::read_to_string(&file).unwrap_or_default();
        fs::write(&file, upsert_block(&content, &block))?;
    }
    let generated = [(fish_file(), fish_config()), (environment_d_file(), environment_d())];
    for (file, content) in generated {
        if let Some(file) = file {
            fs::create_dir_all(file.parent().unwrap())?;
            fs::write(&file, content)?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        let file = launch_agent_file();
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, launch_agent())?;
        // apply now for GUI apps started from here on, without waiting for a re-login
        let _ = std::process::Command::new("launchctl")
            .args(["setenv", "JAVA_HOME"])
            .arg(paths::current_link())
            .status();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "# >>> jman >>>\nexport JAVA_HOME=x\n# <<< jman <<<\n";

    #[test]
    fn inserts_into_empty_and_existing_profiles() {
        assert_eq!(upsert_block("", BLOCK), BLOCK);
        assert_eq!(upsert_block("alias ll='ls -l'\n", BLOCK), format!("alias ll='ls -l'\n\n{BLOCK}"));
    }

    #[test]
    fn upsert_is_idempotent_and_replaces() {
        let once = upsert_block("a\n", BLOCK);
        assert_eq!(upsert_block(&once, BLOCK), once);

        let updated = BLOCK.replace("=x", "=y");
        let replaced = upsert_block(&once, &updated);
        assert_eq!(replaced, format!("a\n\n{updated}"));
        assert!(!replaced.contains("=x"));
    }

    #[test]
    fn removes_block_and_keeps_surroundings() {
        let content = format!("a\n\n{BLOCK}b\n");
        assert_eq!(remove_block(&content), "a\nb\n");
        assert_eq!(remove_block(BLOCK), "");
        assert_eq!(remove_block("no block\n"), "no block\n");
    }
}
