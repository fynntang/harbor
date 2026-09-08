use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub name: String,
    pub app_bundle: PathBuf,
    pub executable: PathBuf,
    pub bundle_id: String,
    pub registered_app_version: String,
    /// Older profiles lack a build snapshot and require explicit review at launch.
    #[serde(default)]
    pub registered_app_build_version: Option<String>,
    pub codex_home: PathBuf,
    pub gui_home: PathBuf,
    pub working_directory: PathBuf,
    /// Environment variable NAMES only. Values are never saved here.
    #[serde(default)]
    pub pass_env: Vec<String>,
    pub adopted_data: bool,
}

impl Profile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION,
            "Unsupported profile schema {}",
            self.schema_version
        );
        validate_name(&self.name)?;
        for path in [
            &self.app_bundle,
            &self.executable,
            &self.codex_home,
            &self.gui_home,
            &self.working_directory,
        ] {
            ensure!(
                path.is_absolute(),
                "Profile path must be absolute: {}",
                path.display()
            );
            ensure!(
                !path.components().any(|c| matches!(c, Component::ParentDir)),
                "Profile paths must not contain '..': {}",
                path.display()
            );
            // Profiles use JSON. Reject loss-prone paths instead of silently changing them.
            ensure!(
                path.to_str().is_some(),
                "Non-UTF-8 profile paths are not supported"
            );
        }
        ensure!(
            self.executable.starts_with(&self.app_bundle),
            "Executable is outside its app bundle"
        );
        ensure!(
            !paths_overlap(&self.codex_home, &self.gui_home),
            "CODEX_HOME and GUI home must be separate, non-nested directories"
        );
        for data in [&self.codex_home, &self.gui_home] {
            ensure!(
                !paths_overlap(data, &self.app_bundle),
                "Data directory overlaps app bundle"
            );
        }
        ensure!(!self.bundle_id.is_empty(), "Empty bundle identifier");
        ensure!(
            !self.registered_app_version.trim().is_empty(),
            "Empty registered app version"
        );
        if let Some(build) = &self.registered_app_build_version {
            ensure!(
                !build.trim().is_empty(),
                "Empty registered app build number"
            );
        }
        for key in &self.pass_env {
            crate::environment::validate_extra_key(key)?;
        }
        Ok(())
    }
}

pub fn validate_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    ensure!(
        !bytes.is_empty() && bytes.len() <= 48,
        "Profile name must contain 1–48 characters"
    );
    ensure!(
        bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit(),
        "Profile name must start with a lowercase letter or digit"
    );
    ensure!(
        bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-'),
        "Use only lowercase letters, digits, and '-' for profile names"
    );
    Ok(())
}

pub fn paths_overlap(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

pub fn ensure_no_conflict(candidate: &Profile, others: &[Profile]) -> Result<()> {
    for other in others {
        if other.name == candidate.name {
            bail!(
                "Profile '{}' already exists; existing data was not changed",
                other.name
            );
        }
        // Keep app-to-profile routing unambiguous.
        ensure!(
            candidate.app_bundle != other.app_bundle,
            "App is already assigned to '{}'; use a separate prepared app bundle",
            other.name
        );
        ensure!(
            candidate.bundle_id != other.bundle_id,
            "Bundle ID is already assigned to '{}'; prepare a distinct app identity",
            other.name
        );
        for a in [&candidate.codex_home, &candidate.gui_home] {
            for b in [&other.codex_home, &other.gui_home] {
                ensure!(
                    !paths_overlap(a, b),
                    "Data path overlaps profile '{}': {}",
                    other.name,
                    a.display()
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_safe_single_path_components() {
        for valid in ["work", "personal", "work-2", "0"] {
            validate_name(valid).unwrap();
        }
        for invalid in [
            "", "..", "../work", "/work", "-work", "Work", "a b", "a_b", "a\nb",
        ] {
            assert!(validate_name(invalid).is_err(), "{invalid:?}");
        }
    }
    #[test]
    fn path_overlap_uses_components_not_string_prefixes() {
        assert!(paths_overlap(
            Path::new("/tmp/gui"),
            Path::new("/tmp/gui/sub")
        ));
        assert!(!paths_overlap(
            Path::new("/tmp/gui"),
            Path::new("/tmp/gui-2")
        ));
    }
}
