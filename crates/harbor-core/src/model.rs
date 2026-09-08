use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    /// Stable registry identifier; legacy manifests used this as the visible name too.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
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
    pub fn display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION,
            "Unsupported profile schema {}",
            self.schema_version
        );
        validate_name(&self.name)?;
        validate_display_name(self.display_name())?;
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
        bytes[0].is_ascii_alphabetic() || bytes[0].is_ascii_digit(),
        "Profile name must start with an ASCII letter or digit"
    );
    ensure!(
        bytes
            .iter()
            .all(|b| b.is_ascii_alphabetic() || b.is_ascii_digit() || *b == b'-'),
        "Use only ASCII letters, digits, and '-' for profile names"
    );
    Ok(())
}

/// Display text never participates in filesystem paths or bundle identity.
pub fn validate_display_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.chars().count() <= 48 && name.trim() == name,
        "Display name must contain 1–48 characters, without leading or trailing whitespace"
    );
    ensure!(
        !name.chars().any(char::is_control),
        "Display name must not contain control characters"
    );
    Ok(())
}

pub fn identifier_for_display_name(name: &str) -> Result<String> {
    validate_display_name(name)?;
    if validate_name(name).is_ok() {
        return Ok(name.to_ascii_lowercase());
    }
    // Read randomness directly rather than introducing a dependency for one identifier.
    use std::io::Read;
    let mut bytes = [0u8; 8];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let suffix: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let prefix = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase();
    let prefix = prefix.chars().take(24).collect::<String>();
    let prefix = prefix.trim_end_matches('-');
    Ok(format!(
        "{}-{suffix}",
        if prefix.is_empty() { "profile" } else { prefix }
    ))
}

pub fn paths_overlap(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

pub fn ensure_no_conflict(candidate: &Profile, others: &[Profile]) -> Result<()> {
    for other in others {
        if other.name.eq_ignore_ascii_case(&candidate.name)
            || other
                .display_name()
                .eq_ignore_ascii_case(candidate.display_name())
        {
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
            !candidate.bundle_id.eq_ignore_ascii_case(&other.bundle_id),
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
        for valid in [
            "work", "personal", "work-2", "0", "Work", "TOOBIT", "My-Work2",
        ] {
            validate_name(valid).unwrap();
        }
        for invalid in [
            "", "..", "../work", "/work", "-work", "工作", "a b", "a_b", "a\nb",
        ] {
            assert!(validate_name(invalid).is_err(), "{invalid:?}");
        }
    }
    #[test]
    fn display_names_generate_safe_lowercase_identifiers() {
        assert_eq!(identifier_for_display_name("Toobit").unwrap(), "toobit");
        for label in [
            "工作",
            "工作账号（Toobit）",
            "Work (Team A)",
            "../work",
            "-Work",
            "a_b",
        ] {
            let id = identifier_for_display_name(label).unwrap();
            validate_name(&id).unwrap();
            assert_eq!(id, id.to_ascii_lowercase());
            assert_ne!(id, label);
        }
        assert!(identifier_for_display_name("工作")
            .unwrap()
            .starts_with("profile-"));
        for invalid in ["", " ", " work", "work ", "a\nb", "a\0b", "a\u{85}b"] {
            assert!(validate_display_name(invalid).is_err());
        }
        assert!(validate_display_name(&"中".repeat(48)).is_ok());
        assert!(validate_display_name(&"中".repeat(49)).is_err());
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
