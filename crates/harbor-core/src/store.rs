use crate::{
    app::AppInfo,
    fsutil,
    model::{self, Profile, SCHEMA_VERSION},
};
use anyhow::{ensure, Context, Result};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Store {
    pub root: PathBuf,
    pub home: PathBuf,
}

impl Store {
    pub fn open(root: Option<PathBuf>) -> Result<Self> {
        let home = fsutil::user_home()?;
        let root = root.unwrap_or_else(|| home.join("Library/Application Support/Harbor"));
        ensure!(root.is_absolute(), "--root must be an absolute path");
        ensure!(
            !root
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir)),
            "--root must not contain '..'"
        );
        ensure!(
            root != home && root != Path::new("/") && root != home.join(".codex"),
            "Choose a dedicated Harbor metadata directory, not HOME or CODEX_HOME"
        );
        reject_default_paths(&home, &[&root])?;
        fsutil::private_dir(&root)?;
        let root = root.canonicalize()?;
        fsutil::private_dir(&root.join("profiles"))?;
        Ok(Self { root, home })
    }

    pub fn profile_dir(&self, name: &str) -> Result<PathBuf> {
        model::validate_name(name)?;
        Ok(self.root.join("profiles").join(name))
    }

    pub fn log_path(&self, name: &str) -> Result<PathBuf> {
        Ok(self.profile_dir(name)?.join("launch.log"))
    }

    pub fn load(&self, name: &str) -> Result<Profile> {
        let dir = self.profile_dir(name)?;
        ensure!(
            !fs::symlink_metadata(&dir)?.file_type().is_symlink(),
            "Profile directory is a symlink"
        );
        let path = dir.join("profile.json");
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)
            .with_context(|| {
                format!(
                    "Profile '{name}' not found or incomplete at {}",
                    path.display()
                )
            })?;
        ensure!(
            file.metadata()?.len() <= 1024 * 1024,
            "Profile manifest is unexpectedly large"
        );
        let profile: Profile = serde_json::from_reader(file).context("Invalid profile JSON")?;
        profile.validate()?;
        ensure!(
            profile.name == name,
            "Manifest name does not match its registry directory"
        );
        Ok(profile)
    }

    pub fn list(&self) -> Result<Vec<Profile>> {
        let mut profiles = Vec::new();
        for entry in fs::read_dir(self.root.join("profiles"))? {
            let entry = entry?;
            if entry.file_name() == ".DS_Store" {
                continue;
            }
            ensure!(
                entry.file_type()?.is_dir(),
                "Unexpected registry entry: {}",
                entry.path().display()
            );
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Invalid profile directory name"))?;
            profiles.push(self.load(&name)?);
        }
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profiles)
    }

    /// New empty data directories. Does not clone, modify, sign, or update the app.
    pub fn create(
        &self,
        name: &str,
        app: &Path,
        cwd: Option<&Path>,
        pass_env: Vec<String>,
    ) -> Result<Profile> {
        self.register(name, app, None, cwd, pass_env)
    }

    /// References existing data IN PLACE: never copies databases or credentials.
    pub fn adopt(
        &self,
        name: &str,
        app: &Path,
        codex: &Path,
        gui: &Path,
        cwd: Option<&Path>,
        pass_env: Vec<String>,
    ) -> Result<Profile> {
        self.register(name, app, Some((codex, gui)), cwd, pass_env)
    }

    fn register(
        &self,
        name: &str,
        app: &Path,
        adopted: Option<(&Path, &Path)>,
        cwd: Option<&Path>,
        pass_env: Vec<String>,
    ) -> Result<Profile> {
        let display_name = name;
        let identifier = model::identifier_for_display_name(display_name)?;
        let name = identifier.as_str();
        let dir = self.profile_dir(name)?;
        let _guard = fsutil::lock_registry(&self.root)?;
        ensure!(
            !dir.try_exists()?,
            "Profile directory already exists: {}; nothing was overwritten",
            dir.display()
        );
        let info = AppInfo::inspect(app)?;
        let working_directory = fsutil::existing_dir(cwd.unwrap_or(&self.home))?;
        let adopted_data = adopted.is_some();
        let (codex_home, gui_home) = match adopted {
            Some((codex, gui)) => (fsutil::existing_dir(codex)?, fsutil::existing_dir(gui)?),
            None => (dir.join("codex"), dir.join("gui")),
        };
        let profile = Profile {
            schema_version: SCHEMA_VERSION,
            name: name.to_string(),
            display_name: Some(display_name.into()),
            app_bundle: info.bundle,
            executable: info.executable,
            bundle_id: info.bundle_id,
            registered_app_version: info.version,
            registered_app_build_version: Some(info.build_version),
            codex_home,
            gui_home,
            working_directory,
            pass_env,
            adopted_data,
        };
        profile.validate()?;
        if adopted_data {
            for data in [&profile.codex_home, &profile.gui_home] {
                ensure!(
                    !model::paths_overlap(data, &self.root.join("profiles")),
                    "Adopted data must not overlap the Harbor profile registry"
                );
            }
        }
        self.reject_default_data(&profile)?;
        let others = self.list()?;
        // Revalidate existing paths before comparison, so symlink changes cannot
        // silently bypass collision checks. Broken registrations fail closed.
        for other in &others {
            self.validate_live_paths(other)?;
        }
        model::ensure_no_conflict(&profile, &others)?;

        DirBuilder::new().mode(0o700).create(&dir)?;
        // Only Harbor's new metadata/data directory is touched below. If setup
        // fails, leave it for inspection rather than recursively deleting anything.
        if !adopted_data {
            fsutil::private_dir(&profile.codex_home)?;
            fsutil::private_dir(&profile.gui_home)?;
        }
        fsutil::write_new_json(&dir.join("profile.json"), &profile).with_context(|| {
            format!(
                "Registration incomplete in {}; existing account directories were not changed",
                dir.display()
            )
        })?;
        Ok(profile)
    }

    fn reject_default_data(&self, profile: &Profile) -> Result<()> {
        reject_default_paths(&self.home, &[&profile.codex_home, &profile.gui_home])
    }

    pub fn validate_live_paths(&self, profile: &Profile) -> Result<()> {
        profile.validate()?;
        for dir in [
            &profile.app_bundle,
            &profile.codex_home,
            &profile.gui_home,
            &profile.working_directory,
        ] {
            let actual = fsutil::existing_dir(dir)?;
            ensure!(
                &actual == dir,
                "Path now resolves elsewhere: {}; refusing to reroute a profile",
                dir.display()
            );
        }
        self.reject_default_data(profile)
    }

    /// A .command shortcut uses this installed CLI, never double-clicks the
    /// vendor bundle without its per-profile arguments. No existing file is replaced.
    pub fn shortcut(&self, name: &str, output: &Path, harbor_executable: &Path) -> Result<()> {
        self.load(name)?;
        ensure!(
            output.extension().is_some_and(|ext| ext == "command"),
            "Shortcut must end in .command"
        );
        let executable = harbor_executable.canonicalize()?;
        let executable = executable.to_str().context("Non-UTF-8 executable path")?;
        let root = self.root.to_str().context("Non-UTF-8 registry path")?;
        let content = format!("#!/bin/sh\n# Generated by Harbor. No account data is embedded.\nexec {} --root {} start {}\n",
            fsutil::shell_quote(executable), fsutil::shell_quote(root), fsutil::shell_quote(name));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(output)
            .with_context(|| {
                format!("Create shortcut without overwriting: {}", output.display())
            })?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}

pub(crate) fn reject_default_paths(home: &Path, paths: &[&Path]) -> Result<()> {
    // Harbor deliberately manages additional instances, not the primary account.
    let protected = [
        home.join(".codex"),
        home.join("Library/Application Support/com.openai.codex"),
        home.join("Library/Application Support/Codex"),
        home.join("Library/Application Support/ChatGPT"),
    ];
    for legacy in &protected {
        let actual_legacy = resolve_future_dir(legacy)?;
        for data in paths {
            let actual_data = resolve_future_dir(data)?;
            // Protect both the default pathname and its physical destination.
            for candidate in [*data, actual_data.as_path()] {
                ensure!(
                    !default_paths_overlap(candidate, legacy)
                        && !default_paths_overlap(candidate, &actual_legacy),
                    "Refusing the default account directory or its parent/child: {}. Leave the primary app unmanaged",
                    data.display()
                );
            }
        }
    }
    Ok(())
}

fn default_paths_overlap(a: &Path, b: &Path) -> bool {
    // Missing suffixes cannot be canonicalized for case aliases. Conservatively
    // reserve ASCII case variants of default account paths on every filesystem.
    a.components().zip(b.components()).all(|(a, b)| {
        a.as_os_str()
            .as_encoded_bytes()
            .eq_ignore_ascii_case(b.as_os_str().as_encoded_bytes())
    })
}

/// Resolve existing ancestors without creating missing directories. Broken
/// symlinks and other resolution errors must fail before any chmod or mkdir.
pub(crate) fn resolve_future_dir(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                let mut resolved = fsutil::existing_dir(ancestor)?;
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    ancestor
                        .file_name()
                        .context("Missing directory path component")?,
                );
                ancestor = ancestor
                    .parent()
                    .context("Missing existing directory ancestor")?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Resolve directory: {}", path.display()));
            }
        }
    }
}
