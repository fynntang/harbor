//! Create a new local app copy and an empty profile without touching existing accounts.
use crate::{environment, fsutil, model, process, store, AppInfo, Profile, Store, SCHEMA_VERSION};
use anyhow::{bail, ensure, Context, Result};
use std::ffi::CString;
use std::fs::{self, DirBuilder};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Component, Path};
use std::process::{Command, Output};

const SOURCE_ID: &str = "com.openai.codex";
const VENDOR_REQUIREMENT: &str = "=anchor apple generic and identifier \"com.openai.codex\" and certificate leaf[subject.OU] = \"2DC432GLL2\"";
const LIBRARY_VALIDATION: &str = "com.apple.security.cs.disable-library-validation";

/// macOS-only, local ad-hoc copy. Does not launch, log in, update, or replace apps.
pub fn clone_profile(
    store: &Store,
    name: &str,
    source: &Path,
    destination: Option<&Path>,
    cwd: Option<&Path>,
    pass_env: Vec<String>,
) -> Result<Profile> {
    clone_profile_with_progress(store, name, source, destination, cwd, pass_env, |_| {})
}

/// Reports preparation stages without account contents or child-process output.
pub fn clone_profile_with_progress(
    store: &Store,
    name: &str,
    source: &Path,
    destination: Option<&Path>,
    cwd: Option<&Path>,
    pass_env: Vec<String>,
    mut progress: impl FnMut(&str),
) -> Result<Profile> {
    process::require_macos()?;
    let profile = clone_with(
        store,
        name,
        source,
        destination,
        cwd,
        pass_env,
        |original, staged, profile| prepare_bundle(original, staged, profile, &mut progress),
    )?;
    progress("complete");
    Ok(profile)
}

// The preparation seam lets tests inject copy/signing failures without trusting
// unsigned fixtures or providing a production switch to skip vendor verification.
fn clone_with(
    store: &Store,
    name: &str,
    source: &Path,
    destination: Option<&Path>,
    cwd: Option<&Path>,
    pass_env: Vec<String>,
    prepare: impl FnOnce(&AppInfo, &Path, &Profile) -> Result<()>,
) -> Result<Profile> {
    let display_name = name;
    let identifier = model::identifier_for_display_name(display_name)?;
    let name = identifier.as_str();
    for key in &pass_env {
        environment::validate_extra_key(key)?;
    }
    let _guard = fsutil::lock_registry(&store.root)?;
    let profile_dir = store.profile_dir(name)?;
    require_absent(&profile_dir)?;
    let original = AppInfo::inspect(source)?;
    ensure!(
        original.bundle_id == SOURCE_ID,
        "Clone requires the original com.openai.codex application, not another copy"
    );
    let requested = destination.map(Path::to_path_buf).unwrap_or_else(|| {
        store
            .home
            .join("Applications/Harbor")
            .join(format!("ChatGPT-{name}.app"))
    });
    ensure!(
        requested.is_absolute() && requested.extension().is_some_and(|s| s == "app"),
        "Clone destination must be an absolute .app path"
    );
    ensure!(
        !requested
            .components()
            .any(|c| matches!(c, Component::ParentDir)),
        "Clone destination must not contain '..'"
    );
    let parent = store::resolve_future_dir(requested.parent().context("Missing app parent")?)?;
    let destination = parent.join(requested.file_name().context("Missing app name")?);
    require_absent(&destination)?;
    store::reject_default_paths(&store.home, &[&destination])?;
    ensure!(
        !model::paths_overlap(&destination, &original.bundle)
            && !model::paths_overlap(&destination, &store.root),
        "Clone destination must not overlap the source app or Harbor metadata"
    );
    let profile = Profile {
        schema_version: SCHEMA_VERSION,
        name: name.into(),
        display_name: Some(display_name.into()),
        app_bundle: destination.clone(),
        executable: destination.join("Contents/MacOS").join(
            original
                .executable
                .file_name()
                .context("Missing executable name")?,
        ),
        bundle_id: format!("{SOURCE_ID}.harbor.{name}"),
        registered_app_version: original.version.clone(),
        registered_app_build_version: Some(original.build_version.clone()),
        codex_home: profile_dir.join("codex"),
        gui_home: profile_dir.join("gui"),
        working_directory: fsutil::existing_dir(cwd.unwrap_or(&store.home))?,
        pass_env,
        adopted_data: false,
    };
    profile.validate()?;
    let others = store.list()?;
    for other in &others {
        store.validate_live_paths(other)?;
        for path in [&other.app_bundle, &other.codex_home, &other.gui_home] {
            ensure!(
                !model::paths_overlap(&destination, path),
                "Clone destination overlaps profile '{}'",
                other.name
            );
        }
    }
    model::ensure_no_conflict(&profile, &others)?;

    // Do not chmod an existing application parent. All temporary directories are
    // private and owned by this operation; only those directories are auto-cleaned.
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".harbor-clone-")
        .tempdir_in(&parent)?;
    let staged_app = staging.path().join("Prepared.app");
    prepare(&original, &staged_app, &profile)?;
    let prepared = AppInfo::inspect(&staged_app)?;
    ensure!(
        prepared.bundle_id == profile.bundle_id
            && prepared.version == profile.registered_app_version
            && Some(prepared.build_version.as_str())
                == profile.registered_app_build_version.as_deref()
            && prepared.executable.file_name() == profile.executable.file_name(),
        "Prepared copy does not match the planned profile"
    );

    let metadata = tempfile::Builder::new()
        .prefix(".harbor-profile-")
        .tempdir_in(&store.root)?;
    fsutil::private_dir(&metadata.path().join("codex"))?;
    fsutil::private_dir(&metadata.path().join("gui"))?;
    fsutil::write_new_json(&metadata.path().join("profile.json"), &profile)?;
    rename_new(&staged_app, &destination)?;
    if let Err(error) = rename_new(metadata.path(), &profile_dir) {
        // We published this new, never-launched app. Do not delete any existing
        // profile or data if a concurrent external writer occupied the name.
        if let Err(cleanup) = fs::remove_dir_all(&destination) {
            bail!(
                "{error:#}; could not remove the newly created copy at {}: {cleanup}",
                destination.display()
            );
        }
        return Err(error)
            .context("Profile registration failed; the newly created copy was removed");
    }
    Ok(profile)
}

fn require_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!(
            "Path already exists; nothing was overwritten: {}",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Inspect {}", path.display())),
    }
}

pub(crate) fn checked(command: &mut Command, action: &str) -> Result<Output> {
    let output = command.output().with_context(|| action.to_string())?;
    ensure!(
        output.status.success(),
        "{action}: {}",
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(3000)
            .collect::<String>()
    );
    Ok(output)
}

fn verify_vendor(app: &Path) -> Result<()> {
    checked(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict", "-R", VENDOR_REQUIREMENT])
            .arg(app),
        "Verify original OpenAI application signature",
    )?;
    Ok(())
}

fn prepare_bundle(
    original: &AppInfo,
    staged: &Path,
    profile: &Profile,
    progress: &mut impl FnMut(&str),
) -> Result<()> {
    progress("verify_source");
    verify_vendor(&original.bundle)?;
    progress("copy");
    let copy = Command::new("/bin/cp")
        .arg("-cR")
        .arg(&original.bundle)
        .arg(staged)
        .output()
        .context("Copy source app")?;
    if !copy.status.success() {
        if staged.try_exists()? {
            fs::remove_dir_all(staged)?;
        }
        checked(
            Command::new("/usr/bin/ditto")
                .arg(&original.bundle)
                .arg(staged),
            "Copy source app with ditto",
        )?;
    }
    // Reject a partial/mixed copy if an updater changed the source while copying.
    progress("verify_copy");
    verify_vendor(staged)?;
    let copied = AppInfo::inspect(staged)?;
    ensure!(
        copied.version == original.version && copied.build_version == original.build_version,
        "The source version changed during copying; retry once its update is complete"
    );
    for relative in [
        "Contents",
        "Contents/MacOS",
        "Contents/Info.plist",
        "Contents/_CodeSignature",
        "Contents/_CodeSignature/CodeResources",
    ] {
        let path = staged.join(relative);
        ensure!(
            !fs::symlink_metadata(&path)?.file_type().is_symlink(),
            "Unsupported symlink in signing path: {relative}"
        );
    }
    let executable = staged.join("Contents/MacOS").join(
        copied
            .executable
            .file_name()
            .context("Missing executable name")?,
    );
    ensure!(
        !fs::symlink_metadata(&executable)?.file_type().is_symlink(),
        "Clone requires a regular main executable"
    );
    let asar = staged.join("Contents/Resources/app.asar");
    ensure!(
        asar.canonicalize()?.starts_with(staged.canonicalize()?),
        "app.asar resolves outside the copied bundle"
    );
    let bytes = fs::read(asar)?;
    for marker in [
        b"CODEX_ELECTRON_USER_DATA_PATH".as_slice(),
        b"CODEX_SPARKLE_ENABLED".as_slice(),
    ] {
        ensure!(
            bytes.windows(marker.len()).any(|w| w == marker),
            "Source version lacks the expected profile/updater controls"
        );
    }
    let output = checked(
        Command::new("/usr/bin/codesign")
            .args(["--display", "--entitlements", "-", "--xml"])
            .arg(staged),
        "Read source app entitlements",
    )?;
    let entitlements = plist::Value::from_reader_xml(output.stdout.as_slice())
        .context("Parse source entitlements")?;
    let filtered = filter_entitlements(&entitlements)?;
    let mut file =
        tempfile::NamedTempFile::new_in(staged.parent().context("Missing staging parent")?)?;
    plist::Value::Dictionary(filtered).to_writer_xml(file.as_file_mut())?;
    file.as_file().sync_all()?;
    preserve_original_icon(staged)?;
    progress("sign");
    patch_info(staged, profile)?;
    // Keep nested vendor code intact. Only the outer copy is signed locally;
    // library validation is relaxed for its vendor-signed frameworks, while the
    // remaining hardened-runtime protections and supported permissions are kept.
    checked(
        Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-", "--identifier"])
            .arg(&profile.bundle_id)
            .args(["--options", "runtime", "--timestamp=none", "--entitlements"])
            .arg(file.path())
            .arg(staged),
        "Sign local application copy",
    )?;
    progress("verify_signature");
    checked(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(staged),
        "Verify signed application copy",
    )?;
    Ok(())
}

// Keep a signed, pristine source for repeated GUI badge edits.
fn preserve_original_icon(app: &Path) -> Result<()> {
    let resources = app.join("Contents/Resources");
    ensure!(
        fs::symlink_metadata(&resources)?.file_type().is_dir(),
        "Unsupported icon resources directory"
    );
    let original = resources.join("icon-chatgpt.png");
    ensure!(
        fs::symlink_metadata(&original)?.file_type().is_file(),
        "Unsupported original icon"
    );
    let metadata = fs::metadata(&original)?;
    ensure!(
        metadata.len() <= 16 * 1024 * 1024,
        "Original icon exceeds size limit"
    );
    let mut destination = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(resources.join("harbor-original-icon.png"))?;
    destination.write_all(&fs::read(original)?)?;
    Ok(())
}

pub(crate) fn filter_entitlements(value: &plist::Value) -> Result<plist::Dictionary> {
    let source = value
        .as_dictionary()
        .context("Entitlements must be a dictionary")?;
    let mut result = plist::Dictionary::new();
    for (key, value) in source {
        if [
            "com.apple.application-identifier",
            "com.apple.developer.",
            "com.apple.security.application-groups",
            "keychain-access-groups",
        ]
        .iter()
        .any(|prefix| key.starts_with(prefix))
        {
            continue;
        }
        ensure!(
            matches!(
                key.as_str(),
                "com.apple.security.app-sandbox"
                    | "com.apple.security.automation.apple-events"
                    | "com.apple.security.cs.allow-jit"
                    | "com.apple.security.cs.allow-unsigned-executable-memory"
                    | "com.apple.security.cs.disable-library-validation"
                    | "com.apple.security.device.audio-input"
                    | "com.apple.security.device.camera"
                    | "com.apple.security.files.user-selected.read-write"
                    | "com.apple.security.network.client"
                    | "com.apple.security.personal-information.calendars"
            ),
            "Unsupported source entitlement '{key}'; this app version needs adapter review"
        );
        let enabled = value
            .as_boolean()
            .with_context(|| format!("Invalid boolean entitlement '{key}'"))?;
        ensure!(
            key != "com.apple.security.app-sandbox" || !enabled,
            "Sandboxed source apps require a different clone adapter"
        );
        result.insert(key.clone(), value.clone());
    }
    // An ad-hoc main binary has no OpenAI team identity. This documented
    // exception permits loading unchanged vendor frameworks; it does not turn
    // off Gatekeeper or change any system-wide security policy.
    result.insert(LIBRARY_VALIDATION.into(), plist::Value::Boolean(true));
    Ok(result)
}

fn patch_info(app: &Path, profile: &Profile) -> Result<()> {
    let path = app.join("Contents/Info.plist");
    let mut value = plist::Value::from_file(&path)?;
    let dict = value
        .as_dictionary_mut()
        .context("Info.plist must be a dictionary")?;
    for (key, text) in [
        ("CFBundleIdentifier", profile.bundle_id.clone()),
        ("CrProductDirName", profile.bundle_id.clone()),
        (
            "CFBundleName",
            format!("ChatGPT ({})", profile.display_name()),
        ),
        (
            "CFBundleDisplayName",
            format!("ChatGPT ({})", profile.display_name()),
        ),
    ] {
        dict.insert(key.into(), plist::Value::String(text));
    }
    let mut env = plist::Dictionary::new();
    for (key, text) in [
        (
            "CODEX_HOME",
            profile
                .codex_home
                .to_str()
                .context("Non-UTF-8 Codex path")?,
        ),
        (
            "CODEX_ELECTRON_USER_DATA_PATH",
            profile.gui_home.to_str().context("Non-UTF-8 GUI path")?,
        ),
        ("CODEX_SPARKLE_ENABLED", "false"),
    ] {
        env.insert(key.into(), plist::Value::String(text.into()));
    }
    dict.insert("LSEnvironment".into(), plist::Value::Dictionary(env));
    value.to_file_xml(path)?;
    Ok(())
}

// Both commits must refuse existing directories atomically, including empty ones.
pub(crate) fn rename_new(source: &Path, destination: &Path) -> Result<()> {
    let from = CString::new(source.as_os_str().as_bytes())?;
    let to = CString::new(destination.as_os_str().as_bytes())?;
    #[cfg(target_os = "macos")]
    // SAFETY: both C strings remain alive; RENAME_EXCL never replaces the target.
    let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    // SAFETY: both C strings remain alive and both paths are absolute.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let result = {
        let _ = (from, to);
        bail!("Exclusive app publication is unsupported on this platform")
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("Publish without overwriting {}", destination.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;

    #[test]
    fn original_icon_is_saved_once_and_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let resources = temp.path().join("Contents/Resources");
        fs::create_dir_all(&resources).unwrap();
        fs::write(resources.join("icon-chatgpt.png"), b"original").unwrap();
        preserve_original_icon(temp.path()).unwrap();
        fs::write(resources.join("icon-chatgpt.png"), b"badged").unwrap();
        assert!(preserve_original_icon(temp.path()).is_err());
        assert_eq!(
            fs::read(resources.join("harbor-original-icon.png")).unwrap(),
            b"original"
        );
    }

    #[test]
    fn original_icon_backup_never_follows_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let resources = temp.path().join("Contents/Resources");
        fs::create_dir_all(&resources).unwrap();
        fs::write(resources.join("icon-chatgpt.png"), b"original").unwrap();
        let outside = temp.path().join("outside");
        fs::write(&outside, b"untouched").unwrap();
        std::os::unix::fs::symlink(&outside, resources.join("harbor-original-icon.png")).unwrap();
        assert!(preserve_original_icon(temp.path()).is_err());
        assert_eq!(fs::read(outside).unwrap(), b"untouched");
    }

    struct Fixture {
        _temp: tempfile::TempDir,
        store: Store,
        source: PathBuf,
        destination: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let base = temp.path().canonicalize().unwrap();
            let home = base.join("home");
            fs::create_dir(&home).unwrap();
            let root = home.join("Harbor");
            fsutil::private_dir(&root.join("profiles")).unwrap();
            let source = base.join("Original.app");
            fs::create_dir_all(source.join("Contents/MacOS")).unwrap();
            let executable = source.join("Contents/MacOS/ChatGPT");
            fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            let mut info = plist::Dictionary::new();
            for (key, value) in [
                ("CFBundleIdentifier", SOURCE_ID),
                ("CFBundleExecutable", "ChatGPT"),
                ("CFBundleShortVersionString", "1.0"),
                ("CFBundleVersion", "100"),
                ("CrProductDirName", SOURCE_ID),
                ("UntouchedMetadata", "sentinel"),
            ] {
                info.insert(key.into(), plist::Value::String(value.into()));
            }
            plist::Value::Dictionary(info)
                .to_file_binary(source.join("Contents/Info.plist"))
                .unwrap();
            Self {
                _temp: temp,
                store: Store { root, home },
                source,
                destination: base.join("New Work.app"),
            }
        }

        fn create(
            &self,
            prepare: impl FnOnce(&AppInfo, &Path, &Profile) -> Result<()>,
        ) -> Result<Profile> {
            clone_with(
                &self.store,
                "work",
                &self.source,
                Some(&self.destination),
                None,
                vec![],
                prepare,
            )
        }
    }

    fn fake_prepare(source: &AppInfo, stage: &Path, profile: &Profile) -> Result<()> {
        fs::create_dir_all(stage.join("Contents/MacOS"))?;
        fs::copy(
            source.bundle.join("Contents/Info.plist"),
            stage.join("Contents/Info.plist"),
        )?;
        fs::copy(&source.executable, stage.join("Contents/MacOS/ChatGPT"))?;
        patch_info(stage, profile)
    }

    #[test]
    fn uppercase_name_is_preserved_and_case_aliases_are_rejected() {
        let f = Fixture::new();
        let p = clone_with(
            &f.store,
            "Toobit",
            &f.source,
            None,
            None,
            vec![],
            fake_prepare,
        )
        .unwrap();
        assert_eq!(p.name, "toobit");
        assert_eq!(p.display_name(), "Toobit");
        assert_eq!(p.bundle_id, "com.openai.codex.harbor.toobit");
        assert_eq!(p.app_bundle.file_name().unwrap(), "ChatGPT-toobit.app");
        assert_eq!(f.store.load("toobit").unwrap().display_name(), "Toobit");
        assert_eq!(f.store.list().unwrap().len(), 1);
        assert!(f.store.load("Toobit").is_err());
        // Exercise the conflict check independently of filesystem case sensitivity.
        let mut alias = p.clone();
        alias.app_bundle = f.destination.clone();
        alias.codex_home = f.store.root.join("other-codex");
        alias.gui_home = f.store.root.join("other-gui");
        alias.name = "toobit".into();
        alias.bundle_id = "other.identity".into();
        assert!(model::ensure_no_conflict(&alias, std::slice::from_ref(&p)).is_err());
        alias.name = "other".into();
        alias.display_name = Some("Other".into());
        alias.bundle_id = p.bundle_id.to_ascii_uppercase();
        assert!(model::ensure_no_conflict(&alias, std::slice::from_ref(&p)).is_err());
        assert!(clone_with(
            &f.store,
            "toobit",
            &f.source,
            Some(&f.destination),
            None,
            vec![],
            |_, _, _| panic!("conflicting clone must not reach preparation"),
        )
        .is_err());
        assert!(!f.destination.exists());
        assert_eq!(f.store.list().unwrap().len(), 1);
    }

    #[test]
    fn display_name_is_separate_from_paths_and_bundle_identity() {
        let f = Fixture::new();
        let label = "工作账号（Toobit） / Team A";
        let p = clone_with(&f.store, label, &f.source, None, None, vec![], fake_prepare).unwrap();
        assert_eq!(p.display_name(), label);
        assert!(p.name.starts_with("toobit-team-a-"));
        assert_eq!(p.name, p.name.to_ascii_lowercase());
        model::validate_name(&p.name).unwrap();
        assert_eq!(p.bundle_id, format!("com.openai.codex.harbor.{}", p.name));
        assert_eq!(
            p.codex_home,
            f.store.profile_dir(&p.name).unwrap().join("codex")
        );
        let info = plist::Value::from_file(p.app_bundle.join("Contents/Info.plist")).unwrap();
        let info = info.as_dictionary().unwrap();
        assert_eq!(
            info["CFBundleDisplayName"].as_string(),
            Some(format!("ChatGPT ({label})").as_str())
        );
        assert_eq!(f.store.load(&p.name).unwrap().display_name(), label);
        assert!(clone_with(
            &f.store,
            label,
            &f.source,
            Some(&f.destination),
            None,
            vec![],
            |_, _, _| panic!("duplicate display name must not be prepared")
        )
        .is_err());
        assert!(!f.destination.exists());
        assert_eq!(f.store.list().unwrap().len(), 1);
    }

    #[test]
    fn clone_publishes_new_bundle_and_empty_private_profile() {
        let f = Fixture::new();
        let source_info = fs::read(f.source.join("Contents/Info.plist")).unwrap();
        let source_executable = fs::read(f.source.join("Contents/MacOS/ChatGPT")).unwrap();
        let p = f.create(fake_prepare).unwrap();
        assert_eq!(p.bundle_id, "com.openai.codex.harbor.work");
        assert_eq!(p.app_bundle, f.destination);
        assert_eq!(p.registered_app_build_version.as_deref(), Some("100"));
        assert!(!p.adopted_data);
        assert_eq!(f.store.load("work").unwrap().app_bundle, f.destination);
        for dir in [&p.codex_home, &p.gui_home] {
            assert_eq!(fs::read_dir(dir).unwrap().count(), 0);
            assert_eq!(
                fs::metadata(dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert_eq!(
            fs::metadata(f.store.profile_dir("work").unwrap().join("profile.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::read(f.source.join("Contents/Info.plist")).unwrap(),
            source_info
        );
        assert_eq!(
            fs::read(f.source.join("Contents/MacOS/ChatGPT")).unwrap(),
            source_executable
        );
        let value = plist::Value::from_file(f.destination.join("Contents/Info.plist")).unwrap();
        let dict = value.as_dictionary().unwrap();
        assert_eq!(dict["UntouchedMetadata"].as_string(), Some("sentinel"));
        assert_eq!(
            dict["CrProductDirName"].as_string(),
            Some(p.bundle_id.as_str())
        );
        let env = dict["LSEnvironment"].as_dictionary().unwrap();
        assert_eq!(env["CODEX_HOME"].as_string(), p.codex_home.to_str());
        assert_eq!(
            env["CODEX_ELECTRON_USER_DATA_PATH"].as_string(),
            p.gui_home.to_str()
        );
        assert_eq!(env["CODEX_SPARKLE_ENABLED"].as_string(), Some("false"));
    }

    #[test]
    fn clone_uses_default_destination_without_requiring_an_existing_app() {
        let f = Fixture::new();
        let p = clone_with(
            &f.store,
            "work",
            &f.source,
            None,
            None,
            vec![],
            fake_prepare,
        )
        .unwrap();
        assert_eq!(
            p.app_bundle,
            f.store.home.join("Applications/Harbor/ChatGPT-work.app")
        );
        assert!(p.app_bundle.is_dir());
    }

    #[test]
    fn preparation_failure_leaves_no_published_app_or_profile() {
        let f = Fixture::new();
        let result = f.create(|source, stage, profile| {
            fake_prepare(source, stage, profile)?;
            bail!("injected signature failure")
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("injected signature failure"));
        assert!(!f.destination.exists());
        assert!(!f.store.profile_dir("work").unwrap().exists());
        assert!(fs::read_dir(f.destination.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".harbor-clone-")));
    }

    #[test]
    fn existing_app_file_directory_and_symlink_are_never_overwritten() {
        for kind in 0..3 {
            let f = Fixture::new();
            match kind {
                0 => fs::write(&f.destination, "sentinel").unwrap(),
                1 => fs::create_dir(&f.destination).unwrap(),
                _ => symlink(f.destination.with_extension("missing"), &f.destination).unwrap(),
            }
            assert!(f
                .create(|_, _, _| panic!("must reject before preparation"))
                .is_err());
            assert!(!f.store.profile_dir("work").unwrap().exists());
            match kind {
                0 => assert_eq!(fs::read_to_string(&f.destination).unwrap(), "sentinel"),
                1 => assert_eq!(fs::read_dir(&f.destination).unwrap().count(), 0),
                _ => assert!(fs::symlink_metadata(&f.destination)
                    .unwrap()
                    .file_type()
                    .is_symlink()),
            }
        }
    }

    #[test]
    fn existing_profile_is_not_overwritten() {
        let f = Fixture::new();
        let directory = f.store.profile_dir("work").unwrap();
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("sentinel"), "original").unwrap();
        assert!(f
            .create(|_, _, _| panic!("must reject before preparation"))
            .is_err());
        assert_eq!(
            fs::read_to_string(directory.join("sentinel")).unwrap(),
            "original"
        );
        assert!(!f.destination.exists());
    }

    #[test]
    fn app_publication_race_preserves_the_other_destination() {
        let f = Fixture::new();
        assert!(f
            .create(|source, stage, profile| {
                fake_prepare(source, stage, profile)?;
                fs::create_dir(&f.destination)?;
                fs::write(f.destination.join("sentinel"), "other owner")?;
                Ok(())
            })
            .is_err());
        assert_eq!(
            fs::read_to_string(f.destination.join("sentinel")).unwrap(),
            "other owner"
        );
        assert!(!f.store.profile_dir("work").unwrap().exists());
    }

    #[test]
    fn registration_race_rolls_back_only_the_new_app() {
        let f = Fixture::new();
        let directory = f.store.profile_dir("work").unwrap();
        assert!(f
            .create(|source, stage, profile| {
                fake_prepare(source, stage, profile)?;
                fs::create_dir(&directory)?;
                fs::write(directory.join("sentinel"), "other owner")?;
                Ok(())
            })
            .is_err());
        assert!(!f.destination.exists());
        assert_eq!(
            fs::read_to_string(directory.join("sentinel")).unwrap(),
            "other owner"
        );
    }

    #[test]
    fn clone_rejects_source_data_and_metadata_overlap() {
        let f = Fixture::new();
        for destination in [
            f.source.join("Nested.app"),
            f.store.root.join("Nested.app"),
            f.store.home.join(".codex/Nested.app"),
        ] {
            assert!(clone_with(
                &f.store,
                "work",
                &f.source,
                Some(&destination),
                None,
                vec![],
                |_, _, _| panic!("must reject before preparation")
            )
            .is_err());
            assert!(!destination.exists());
        }
    }

    #[test]
    fn clone_rejects_unexpected_prepared_identity() {
        let f = Fixture::new();
        assert!(f
            .create(|source, stage, profile| {
                let mut wrong = profile.clone();
                wrong.bundle_id = "other.identity".into();
                fake_prepare(source, stage, &wrong)
            })
            .is_err());
        assert!(!f.destination.exists());
        assert!(!f.store.profile_dir("work").unwrap().exists());
    }

    #[test]
    fn exclusive_publish_does_not_replace_an_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("sentinel"), "source").unwrap();
        assert!(rename_new(&source, &target).is_err());
        assert_eq!(
            fs::read_to_string(source.join("sentinel")).unwrap(),
            "source"
        );
        assert_eq!(fs::read_dir(target).unwrap().count(), 0);
    }

    #[test]
    fn signing_permissions_remove_vendor_identity_and_preserve_supported_flags() {
        let mut entitlements = plist::Dictionary::new();
        entitlements.insert(
            "com.apple.security.cs.allow-jit".into(),
            plist::Value::Boolean(true),
        );
        entitlements.insert(
            "com.apple.security.device.camera".into(),
            plist::Value::Boolean(false),
        );
        for key in [
            "com.apple.application-identifier",
            "com.apple.developer.team-identifier",
            "com.apple.developer.aps-environment",
            "com.apple.security.application-groups",
            "keychain-access-groups",
        ] {
            entitlements.insert(
                key.into(),
                plist::Value::String("synthetic vendor metadata".into()),
            );
        }
        let filtered = filter_entitlements(&plist::Value::Dictionary(entitlements)).unwrap();
        assert_eq!(filtered.len(), 3);
        assert_eq!(
            filtered["com.apple.security.cs.allow-jit"].as_boolean(),
            Some(true)
        );
        assert_eq!(
            filtered["com.apple.security.device.camera"].as_boolean(),
            Some(false)
        );
        assert_eq!(filtered[LIBRARY_VALIDATION].as_boolean(), Some(true));
    }

    #[test]
    fn unsupported_or_invalid_entitlements_fail_closed() {
        for (key, value) in [
            (
                "com.apple.security.get-task-allow",
                plist::Value::Boolean(true),
            ),
            (
                "com.apple.security.cs.allow-dyld-environment-variables",
                plist::Value::Boolean(true),
            ),
            (
                "com.apple.security.app-sandbox",
                plist::Value::Boolean(true),
            ),
            (
                "com.apple.security.network.client",
                plist::Value::String("invalid".into()),
            ),
        ] {
            let mut input = plist::Dictionary::new();
            input.insert(key.into(), value);
            assert!(filter_entitlements(&plist::Value::Dictionary(input)).is_err());
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn production_clone_rejects_unsigned_source() {
        let f = Fixture::new();
        let error = clone_profile(
            &f.store,
            "work",
            &f.source,
            Some(&f.destination),
            None,
            vec![],
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Verify original OpenAI application signature"),
            "{error:#}"
        );
        assert!(!f.destination.exists());
        assert!(!f.store.profile_dir("work").unwrap().exists());
    }
}
