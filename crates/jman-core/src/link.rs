use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::paths;

/// Where `~/.jman/current` points to, if it exists.
pub fn current_target() -> Option<PathBuf> {
    target_of(&paths::current_link())
}

pub fn target_of(link: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    let target = junction::get_target(link).or_else(|_| fs::read_link(link));
    #[cfg(not(windows))]
    let target = fs::read_link(link);
    target.ok().map(|t| dunce::simplified(&t).to_path_buf())
}

/// Points `~/.jman/current` at `target`.
pub fn switch(target: &Path) -> anyhow::Result<()> {
    switch_link(&paths::current_link(), target)
}

pub fn switch_link(link: &Path, target: &Path) -> anyhow::Result<()> {
    if !target.is_dir() {
        bail!("{} is not a directory", target.display());
    }
    let parent = link.parent().context("link has no parent directory")?;
    fs::create_dir_all(parent)?;

    if fs::symlink_metadata(link).is_ok() && target_of(link).is_none() {
        bail!("{} exists and is not a link; move it out of the way first", link.display());
    }

    replace(link, target)
        .with_context(|| format!("pointing {} at {}", link.display(), target.display()))
}

#[cfg(windows)]
fn replace(link: &Path, target: &Path) -> io::Result<()> {
    // A junction can't be replaced atomically, so remove and recreate it. Removing the
    // junction never touches the JDK it pointed at, and running JVMs keep working.
    match fs::remove_dir(link) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    junction::create(target, link)
}

#[cfg(not(windows))]
fn replace(link: &Path, target: &Path) -> io::Result<()> {
    // Create the new symlink next to the old one and rename it over: atomic.
    let tmp = link.with_extension("tmp");
    let _ = fs::remove_file(&tmp);
    std::os::unix::fs::symlink(target, &tmp)?;
    fs::rename(&tmp, link)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_back_and_forth() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("marker"), "a").unwrap();
        fs::write(b.join("marker"), "b").unwrap();
        let link = dir.path().join("jman").join("current");

        switch_link(&link, &a).unwrap();
        assert_eq!(fs::read_to_string(link.join("marker")).unwrap(), "a");
        switch_link(&link, &b).unwrap();
        assert_eq!(fs::read_to_string(link.join("marker")).unwrap(), "b");
        assert_eq!(target_of(&link).unwrap(), dunce::simplified(&b));
        // the previous target must be untouched
        assert_eq!(fs::read_to_string(a.join("marker")).unwrap(), "a");
    }

    #[test]
    fn refuses_to_replace_real_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("jdk");
        let link = dir.path().join("current");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&link).unwrap();
        assert!(switch_link(&link, &target).is_err());
        assert!(link.is_dir());
    }
}
