use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub fn user_home() -> Result<PathBuf> {
    let path = std::env::var_os("HOME").context("HOME is not set")?;
    let path = PathBuf::from(path).canonicalize().context("Resolve HOME")?;
    ensure!(path.is_dir(), "HOME is not a directory");
    Ok(path)
}

pub fn existing_dir(path: &Path) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;
    ensure!(path.is_dir(), "Not a directory: {}", path.display());
    Ok(path)
}

/// Private directories owned by Harbor. Never use this on adopted account roots.
pub fn private_dir(path: &Path) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        ensure!(
            !meta.file_type().is_symlink(),
            "Refusing symlink at {}",
            path.display()
        );
        ensure!(meta.is_dir(), "Not a directory: {}", path.display());
    }
    DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

/// Owns the registry lock without exposing clonable file handles to callers.
#[derive(Debug)]
pub struct RegistryLock {
    file: File,
}

impl Drop for RegistryLock {
    fn drop(&mut self) {
        // A concurrent fork can inherit the file description until exec. Closing
        // only our handle would leave the lock held by that child in the meantime.
        // Explicitly unlock before File closes; Drop cannot report unlock errors.
        let _ = self.file.unlock();
    }
}

/// One registry lock serializes profile registration and launch decisions.
/// Never delete the lock file to 'fix' it: dropping its owner releases the lock.
pub fn lock_registry(root: &Path) -> Result<RegistryLock> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(root.join("registry.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(RegistryLock { file }),
        Err(std::fs::TryLockError::WouldBlock) => {
            anyhow::bail!("Another Harbor operation is in progress; retry shortly")
        }
        Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
    }
}

/// Atomic creation without overwriting an existing manifest.
pub fn write_new_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("Missing manifest parent")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    serde_json::to_writer_pretty(&mut temp, value)?;
    temp.write_all(b"\n")?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(path)
        .with_context(|| format!("Create {} without overwriting", path.display()))?;
    Ok(())
}

pub fn private_append(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| format!("Open log {}", path.display()))
}

/// Bound memory use even when an application log has grown very large.
pub fn tail(path: &Path, count: usize) -> Result<String> {
    ensure!(
        (1..=1000).contains(&count),
        "Line count must be between 1 and 1000"
    );
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    ensure!(file.metadata()?.is_file(), "Log is not a regular file");
    let len = file.metadata()?.len();
    let start = len.saturating_sub(1024 * 1024);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(1024 * 1024).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let text = if start > 0 {
        text.split_once('\n').map(|(_, rest)| rest).unwrap_or("")
    } else {
        &text
    };
    let mut lines: Vec<&str> = text.lines().rev().take(count).collect();
    lines.reverse();
    Ok(lines.join("\n"))
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("profile.json");
        write_new_json(&file, &[1]).unwrap();
        assert!(write_new_json(&file, &[2]).is_err());
        assert_eq!(
            serde_json::from_slice::<Vec<i32>>(&fs::read(file).unwrap()).unwrap(),
            vec![1]
        );
    }
    #[test]
    fn lock_excludes_another_handle_and_releases_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let guard = lock_registry(dir.path()).unwrap();
        assert!(lock_registry(dir.path()).is_err());
        drop(guard);
        let _next = lock_registry(dir.path()).expect("Dropping the owner must release the lock");
    }
    #[test]
    fn lock_drop_releases_while_inherited_handle_remains_open() {
        let dir = tempfile::tempdir().unwrap();
        let guard = lock_registry(dir.path()).unwrap();
        // dup shares the same file description and flock as fork inheritance.
        // Keep that reference alive to reproduce the fork-to-exec window without
        // relying on thread scheduling or sleeping.
        let inherited = guard.file.try_clone().unwrap();
        assert!(lock_registry(dir.path()).is_err());
        drop(guard);
        let next = lock_registry(dir.path())
            .expect("Dropping the owner must unlock even while an inherited handle remains open");
        // Closing an old inherited reference must not release the next owner's lock.
        drop(inherited);
        assert!(lock_registry(dir.path()).is_err());
        drop(next);
        assert!(lock_registry(dir.path()).is_ok());
    }
    #[test]
    fn tail_reads_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("log");
        fs::write(&file, "a\nb\nc\n").unwrap();
        assert_eq!(tail(&file, 2).unwrap(), "b\nc");
        assert!(tail(&file, 0).is_err());
    }
    #[test]
    fn quoted_shortcut_paths_handle_spaces_and_quotes() {
        assert_eq!(shell_quote("/a b/c'd"), "'/a b/c'\"'\"'d'");
    }
    #[test]
    fn symlink_log_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::write(&dest, "original").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&dest, &link).unwrap();
        assert!(private_append(&link).is_err());
        assert_eq!(fs::read_to_string(dest).unwrap(), "original");
    }
}
