//! Replace a stopped managed app; account directories never participate in the transaction.
use crate::{clone, fsutil, icon, model, AppInfo, Profile, Store};
use anyhow::{ensure, Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn update_profile(
    store: &Store,
    name: &str,
    source: &Path,
    mut progress: impl FnMut(&str),
) -> Result<Profile> {
    crate::process::require_macos()?;
    clone::verify_vendor(source)?;
    update_with(
        store,
        name,
        source,
        |original, staged, profile| {
            clone::prepare_bundle(
                original,
                staged,
                profile,
                &mut progress,
                Some(&profile.app_bundle),
            )
        },
        stopped,
        verify,
        |temp, path| {
            temp.persist(path)
                .map_err(|e| e.error)
                .context("Publish updated profile manifest")?;
            Ok(())
        },
    )
}

fn stopped(profile: &Profile) -> Result<()> {
    ensure!(crate::auxiliary::running(profile)?.is_empty(),
        "Quit this instance and all its helpers (including browser integrations) before updating; no app was replaced");
    Ok(())
}
fn verify(app: &Path) -> Result<()> {
    clone::checked(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(app),
        "Verify update bundle signature",
    )?;
    Ok(())
}
fn newer(current: &str, source: &str) -> Result<()> {
    let current: u64 = current
        .parse()
        .context("Registered build is not numeric; inspect this instance")?;
    let source: u64 = source
        .parse()
        .context("Official build is not numeric; this version is unsupported")?;
    ensure!(
        source > current,
        "Official build must be newer; downgrades are not supported"
    );
    Ok(())
}

fn update_with(
    store: &Store,
    name: &str,
    source: &Path,
    prepare: impl FnOnce(&AppInfo, &Path, &Profile) -> Result<()>,
    stopped: impl Fn(&Profile) -> Result<()>,
    verify: impl Fn(&Path) -> Result<()>,
    commit: impl FnOnce(tempfile::NamedTempFile, &Path) -> Result<()>,
) -> Result<Profile> {
    let _lock = fsutil::lock_registry(&store.root)?;
    let old = store.load(name)?;
    store.validate_live_paths(&old)?;
    let directory = store.profile_dir(name)?;
    ensure!(
        !old.adopted_data
            && old.bundle_id == format!("com.openai.codex.harbor.{name}")
            && old.codex_home == directory.join("codex")
            && old.gui_home == directory.join("gui"),
        "Only Harbor-created copies with their own data directories can be updated"
    );
    let current = AppInfo::inspect(&old.app_bundle)?;
    ensure!(
        current.executable == old.executable
            && current.bundle_id == old.bundle_id
            && current.version == old.registered_app_version
            && Some(&current.build_version) == old.registered_app_build_version.as_ref(),
        "Existing app identity/version differs from its manifest; inspect it before updating"
    );
    stopped(&old)?;
    verify(&old.app_bundle)?;
    let original = AppInfo::inspect(source)?;
    ensure!(
        original.bundle_id == "com.openai.codex",
        "Update source must be the official original app"
    );
    for path in [&old.app_bundle, &store.root, &old.codex_home, &old.gui_home] {
        ensure!(
            !model::paths_overlap(&original.bundle, path),
            "Update source overlaps managed app/data paths"
        );
    }
    if original.version == current.version && original.build_version == current.build_version {
        return Ok(old);
    }
    newer(&current.build_version, &original.build_version)?;
    ensure!(
        original.executable.file_name() == old.executable.file_name(),
        "Official executable name changed; this layout is unsupported"
    );
    let mut updated = old.clone();
    updated.registered_app_version = original.version.clone();
    updated.registered_app_build_version = Some(original.build_version.clone());
    let staging = tempfile::Builder::new()
        .prefix(".harbor-update-")
        .tempdir_in(old.app_bundle.parent().context("Missing app parent")?)?;
    let prepared = staging.path().join("Prepared.app");
    prepare(&original, &prepared, &updated)?;
    verify(&prepared)?;
    let checked = AppInfo::inspect(&prepared)?;
    ensure!(
        checked.bundle_id == updated.bundle_id
            && checked.version == updated.registered_app_version
            && Some(&checked.build_version) == updated.registered_app_build_version.as_ref()
            && checked.executable.file_name() == old.executable.file_name(),
        "Prepared update identity/version mismatch"
    );
    let manifest = directory.join("profile.json");
    let mut pending = tempfile::NamedTempFile::new_in(&directory)?;
    serde_json::to_writer_pretty(&mut pending, &updated)?;
    pending.write_all(b"\n")?;
    pending.as_file().sync_all()?;
    store.validate_live_paths(&old)?;
    let before_publish = AppInfo::inspect(&old.app_bundle)?;
    ensure!(
        before_publish.bundle_id == current.bundle_id
            && before_publish.version == current.version
            && before_publish.build_version == current.build_version,
        "Existing app changed while preparing the update"
    );
    stopped(&old)?;
    icon::exchange(&prepared, &old.app_bundle)?;
    // No fallible mutation follows manifest publication. Before it, swapping back restores routing.
    let result = verify(&old.app_bundle).and_then(|_| commit(pending, &manifest));
    if let Err(error) = result {
        if let Err(rollback) = icon::exchange(&prepared, &old.app_bundle) {
            let backup = staging.keep();
            anyhow::bail!(
                "{error:#}; rollback failed: {rollback:#}; old app retained at {}",
                backup.display()
            );
        }
        return Err(error);
    }
    Ok(updated)
}

pub(crate) fn preserve_icons(previous: &Path, staged: &Path) -> Result<()> {
    let old_info = plist::Value::from_file(previous.join("Contents/Info.plist"))?;
    // Only icon changes made by Harbor set this override; pristine copies get the new official icon.
    if old_info
        .as_dictionary()
        .and_then(|d| d.get("CFBundleIconFile"))
        .and_then(plist::Value::as_string)
        != Some("icon-chatgpt.icns")
    {
        return Ok(());
    }
    for file in [
        "icon-chatgpt.png",
        "icon-chatgpt.icns",
        "icon-codex-light.png",
        "icon-codex-dark-color.png",
        "chatgptTemplate.png",
        "chatgptTemplate@2x.png",
    ] {
        let source = previous.join("Contents/Resources").join(file);
        let destination = staged.join("Contents/Resources").join(file);
        for path in [&source, &destination] {
            ensure!(
                fs::symlink_metadata(path)?.is_file(),
                "Unsupported icon layout or symlink: {file}"
            );
        }
        ensure!(
            fs::metadata(&source)?.len() <= 16 * 1024 * 1024,
            "Existing icon is unexpectedly large"
        );
        ensure!(
            source.canonicalize()?.starts_with(previous.canonicalize()?)
                && destination
                    .canonicalize()?
                    .starts_with(staged.canonicalize()?),
            "Icon path resolves outside its app"
        );
        fs::copy(source, destination)?;
    }
    let path = staged.join("Contents/Info.plist");
    let mut info = plist::Value::from_file(&path)?;
    icon::patch_info(
        info.as_dictionary_mut()
            .context("Invalid staged Info.plist")?,
    );
    info.to_file_xml(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pristine_official_icon_metadata_is_not_mistaken_for_a_custom_icon() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("Old.app");
        fs::create_dir_all(old.join("Contents")).unwrap();
        let mut info = plist::Dictionary::new();
        info.insert(
            "CodexAppIconBaseName".into(),
            plist::Value::String("icon-chatgpt".into()),
        );
        info.insert(
            "CFBundleIconFile".into(),
            plist::Value::String("electron.icns".into()),
        );
        plist::Value::Dictionary(info)
            .to_file_xml(old.join("Contents/Info.plist"))
            .unwrap();
        preserve_icons(&old, &temp.path().join("New.app")).unwrap();
    }

    #[test]
    fn builds_must_increase_numerically() {
        assert!(newer("9", "10").is_ok());
        for (old, new) in [("10", "9"), ("10", "10"), ("x", "11"), ("10", "x")] {
            assert!(newer(old, new).is_err());
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod transaction_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Fixture {
        _temp: tempfile::TempDir,
        store: Store,
        app: std::path::PathBuf,
        source: std::path::PathBuf,
    }
    fn app(path: &Path, id: &str, version: &str, build: &str) {
        fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
        fs::write(
            path.join("Contents/MacOS/ChatGPT"),
            format!("#!/bin/sh\n# {version}\nexit 0\n"),
        )
        .unwrap();
        fs::set_permissions(
            path.join("Contents/MacOS/ChatGPT"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let mut info = plist::Dictionary::new();
        for (k, v) in [
            ("CFBundleIdentifier", id),
            ("CFBundleExecutable", "ChatGPT"),
            ("CFBundleShortVersionString", version),
            ("CFBundleVersion", build),
            ("CrProductDirName", id),
        ] {
            info.insert(k.into(), plist::Value::String(v.into()));
        }
        plist::Value::Dictionary(info)
            .to_file_xml(path.join("Contents/Info.plist"))
            .unwrap();
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let home = temp.path().canonicalize().unwrap();
            let store = Store {
                root: home.join("registry"),
                home: home.clone(),
            };
            fsutil::private_dir(&store.root.join("profiles")).unwrap();
            let copy = home.join("Work.app");
            let source = home.join("Official.app");
            app(&copy, "com.openai.codex.harbor.work", "1.0", "1");
            app(&source, "com.openai.codex", "2.0", "2");
            let p = store.create("Work", &copy, None, vec![]).unwrap();
            fs::write(p.codex_home.join("sentinel"), "account sentinel").unwrap();
            Self {
                _temp: temp,
                store,
                app: copy,
                source,
            }
        }
        fn assert_old(&self) {
            assert_eq!(AppInfo::inspect(&self.app).unwrap().version, "1.0");
            assert_eq!(
                self.store.load("work").unwrap().registered_app_version,
                "1.0"
            );
            assert_eq!(
                fs::read_to_string(
                    self.store
                        .profile_dir("work")
                        .unwrap()
                        .join("codex/sentinel")
                )
                .unwrap(),
                "account sentinel"
            );
        }
    }
    fn prepare(original: &AppInfo, staged: &Path, p: &Profile) -> Result<()> {
        app(
            staged,
            &p.bundle_id,
            &original.version,
            &original.build_version,
        );
        Ok(())
    }
    fn commit(temp: tempfile::NamedTempFile, path: &Path) -> Result<()> {
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    #[test]
    fn update_preserves_identity_and_account_bytes() {
        let f = Fixture::new();
        let before = f.store.load("work").unwrap();
        let after = update_with(
            &f.store,
            "work",
            &f.source,
            prepare,
            |_| Ok(()),
            |_| Ok(()),
            commit,
        )
        .unwrap();
        assert_eq!(after.registered_app_version, "2.0");
        assert_eq!(
            f.store
                .load("work")
                .unwrap()
                .registered_app_build_version
                .as_deref(),
            Some("2")
        );
        assert_eq!(after.name, before.name);
        assert_eq!(after.display_name, before.display_name);
        assert_eq!(after.bundle_id, before.bundle_id);
        assert_eq!(after.codex_home, before.codex_home);
        assert_eq!(after.gui_home, before.gui_home);
        assert_eq!(
            fs::read_to_string(after.codex_home.join("sentinel")).unwrap(),
            "account sentinel"
        );
    }
    #[test]
    fn failures_leave_old_app_manifest_and_data_unchanged() {
        for failure in ["prepare", "identity", "helper", "commit", "final_verify"] {
            let f = Fixture::new();
            let old_manifest =
                fs::read(f.store.profile_dir("work").unwrap().join("profile.json")).unwrap();
            let checks = std::cell::Cell::new(0);
            let verifies = std::cell::Cell::new(0);
            let result = update_with(
                &f.store,
                "work",
                &f.source,
                |original, staged, p| {
                    prepare(original, staged, p)?;
                    if failure == "prepare" {
                        anyhow::bail!("injected preparation failure");
                    }
                    if failure == "identity" {
                        app(staged, "wrong.id", "2.0", "2");
                    }
                    Ok(())
                },
                |_| {
                    checks.set(checks.get() + 1);
                    ensure!(
                        failure != "helper" || checks.get() < 2,
                        "injected active helper"
                    );
                    Ok(())
                },
                |_| {
                    verifies.set(verifies.get() + 1);
                    ensure!(
                        failure != "final_verify" || verifies.get() < 3,
                        "injected verification failure"
                    );
                    Ok(())
                },
                |temp, path| {
                    ensure!(failure != "commit", "injected manifest failure");
                    commit(temp, path)
                },
            );
            assert!(result.is_err(), "{failure}");
            f.assert_old();
            assert_eq!(
                old_manifest,
                fs::read(f.store.profile_dir("work").unwrap().join("profile.json")).unwrap()
            );
        }
    }
    #[test]
    fn same_version_is_noop_and_downgrade_or_adoption_is_rejected() {
        let f = Fixture::new();
        app(&f.source, "com.openai.codex", "1.0", "1");
        update_with(
            &f.store,
            "work",
            &f.source,
            |_, _, _| panic!("same version must not prepare"),
            |_| Ok(()),
            |_| Ok(()),
            commit,
        )
        .unwrap();
        app(&f.source, "com.openai.codex", "0.0", "0");
        assert!(update_with(
            &f.store,
            "work",
            &f.source,
            prepare,
            |_| Ok(()),
            |_| Ok(()),
            commit
        )
        .is_err());
        let mut p = f.store.load("work").unwrap();
        p.adopted_data = true;
        fs::write(
            f.store.profile_dir("work").unwrap().join("profile.json"),
            serde_json::to_vec(&p).unwrap(),
        )
        .unwrap();
        assert!(update_with(
            &f.store,
            "work",
            &f.source,
            prepare,
            |_| Ok(()),
            |_| Ok(()),
            commit
        )
        .is_err());
    }
}
