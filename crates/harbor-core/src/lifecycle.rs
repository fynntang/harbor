//! Normal quit and recoverable removal. Native operations are supplied by the bundled macOS helper.
use crate::{clone, fsutil, process, AppInfo, Profile, Store};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub struct RemovalResult {
    pub retained_data: Option<PathBuf>,
    pub trash: Value,
}

fn native_helper() -> Result<PathBuf> {
    let helper = std::env::current_exe()?
        .parent()
        .context("Missing executable parent")?
        .join("harbor-native");
    let meta = fs::symlink_metadata(&helper).context(
        "This action requires the macOS helper bundled with Harbor.app; use its GUI or bundled CLI",
    )?;
    ensure!(
        meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        "Invalid Harbor native helper"
    );
    Ok(helper)
}
fn native(helper: &Path, args: &[String]) -> Result<Value> {
    let output = clone::checked(Command::new(helper).args(args), "Harbor native operation")?;
    let response: Value =
        serde_json::from_slice(&output.stdout).context("Invalid native response")?;
    ensure!(response["ok"] == true, "Native operation did not succeed");
    Ok(response)
}
fn identity(profile: &Profile) -> Result<()> {
    let app = AppInfo::inspect(&profile.app_bundle)?;
    ensure!(
        app.bundle_id == profile.bundle_id && app.executable == profile.executable,
        "App identity changed; no lifecycle action was performed"
    );
    Ok(())
}
fn no_bundle_processes(profile: &Profile) -> Result<()> {
    ensure!(crate::auxiliary::running(profile)?.is_empty(),
        "The app or one of its helpers is still running. Quit this instance and retry; nothing was deleted");
    Ok(())
}

pub fn stop(store: &Store, name: &str) -> Result<&'static str> {
    process::require_macos()?;
    let helper = native_helper()?;
    let _guard = fsutil::lock_registry(&store.root)?;
    let profile = store.load(name)?;
    store.validate_live_paths(&profile)?;
    identity(&profile)?;
    let processes = process::find_running(&profile.executable)?;
    if processes.is_empty() {
        wait_helpers(&profile)?;
        return Ok("already_stopped");
    }
    ensure!(processes.len() == 1 && process::matches_profile(&processes[0], &profile),
        "Ambiguous or unexpected launch arguments. Save tasks and quit that app manually; Harbor will not stop it");
    native(
        &helper,
        &[
            "stop".into(),
            processes[0].pid.to_string(),
            profile.app_bundle.to_string_lossy().into(),
            profile.executable.to_string_lossy().into(),
            profile.bundle_id.clone(),
        ],
    )?;
    wait_helpers(&profile)?;
    Ok("stopped")
}

fn wait_helpers(profile: &Profile) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !crate::auxiliary::running(profile)?.is_empty() {
        let cleanup = crate::auxiliary::stop_orphans(profile);
        if Instant::now() >= deadline {
            cleanup?;
        }
        ensure!(Instant::now() < deadline, "The main app quit but helper processes remain. Wait and retry; no deletion was performed");
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

pub fn remove(store: &Store, name: &str, delete_data: bool) -> Result<RemovalResult> {
    process::require_macos()?;
    let helper = native_helper()?;
    remove_with(store, name, delete_data, no_bundle_processes, |paths| {
        let mut args = vec!["trash".to_string()];
        args.extend(paths.iter().map(|p| p.to_string_lossy().into_owned()));
        Ok(native(&helper, &args)?["items"].clone())
    })
}

fn remove_with(
    store: &Store,
    name: &str,
    delete_data: bool,
    stopped: impl Fn(&Profile) -> Result<()>,
    trash: impl FnOnce(&[PathBuf]) -> Result<Value>,
) -> Result<RemovalResult> {
    let _guard = fsutil::lock_registry(&store.root)?;
    let profile = store.load(name)?;
    store.validate_live_paths(&profile)?;
    identity(&profile)?;
    let profile_dir = store.profile_dir(name)?;
    ensure!(!profile.adopted_data && profile.bundle_id == format!("com.openai.codex.harbor.{name}")
        && profile.codex_home == profile_dir.join("codex") && profile.gui_home == profile_dir.join("gui"),
        "Only Harbor-created copies with Harbor-owned data directories can be removed here; adopted apps/data are preserved");
    // A crafted registration must not turn app deletion into deletion of the registry/default account.
    crate::store::reject_default_paths(&store.home, &[&profile.app_bundle])?;
    ensure!(
        !crate::model::paths_overlap(&profile.app_bundle, &store.root),
        "App overlaps Harbor metadata"
    );
    for other in store.list()?.iter().filter(|other| other.name != name) {
        store.validate_live_paths(other)?;
        for path in [&other.app_bundle, &other.codex_home, &other.gui_home] {
            ensure!(
                !crate::model::paths_overlap(&profile.app_bundle, path)
                    && !crate::model::paths_overlap(&profile_dir, path),
                "Removal overlaps another instance"
            );
        }
    }
    stopped(&profile)?;
    if delete_data {
        let receipt = trash(&[profile.app_bundle, profile_dir])?;
        return Ok(RemovalResult {
            retained_data: None,
            trash: receipt,
        });
    }
    let retained_root = store.root.join("retained");
    fsutil::private_dir(&retained_root)?;
    // Never give a TempDir destructor ownership of account data.
    let holding = tempfile::Builder::new()
        .prefix(&format!("{name}-"))
        .tempdir_in(&retained_root)?
        .keep();
    let retained = holding.join("profile");
    clone::rename_new(&profile_dir, &retained)?;
    match trash(&[profile.app_bundle]) {
        Ok(receipt) => Ok(RemovalResult {
            retained_data: Some(retained),
            trash: receipt,
        }),
        Err(error) => {
            if let Err(rollback) = clone::rename_new(&retained, &profile_dir) {
                anyhow::bail!("{error:#}; registration restore failed: {rollback:#}; account data is retained at {}", retained.display());
            }
            let _ = fs::remove_dir(&holding); // empty only; never recursively delete account data
            Err(error)
                .context("Removal failed; original profile and account directories were restored")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        _temp: tempfile::TempDir,
        store: Store,
        profile: Profile,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let home = temp.path().canonicalize().unwrap().join("home");
            let root = home.join("Harbor");
            fsutil::private_dir(&root.join("profiles")).unwrap();
            let app = home.join("Work.app");
            fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
            fs::write(app.join("Contents/MacOS/ChatGPT"), b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(
                app.join("Contents/MacOS/ChatGPT"),
                fs::Permissions::from_mode(0o700),
            )
            .unwrap();
            let mut info = plist::Dictionary::new();
            for (key, value) in [
                ("CFBundleIdentifier", "com.openai.codex.harbor.work"),
                ("CFBundleExecutable", "ChatGPT"),
                ("CFBundleShortVersionString", "1"),
                ("CFBundleVersion", "1"),
                ("CrProductDirName", "com.openai.codex.harbor.work"),
            ] {
                info.insert(key.into(), plist::Value::String(value.into()));
            }
            plist::Value::Dictionary(info)
                .to_file_xml(app.join("Contents/Info.plist"))
                .unwrap();
            let store = Store { root, home };
            let profile = store.create("work", &app, None, vec![]).unwrap();
            fs::write(profile.codex_home.join("sentinel"), b"test account data").unwrap();
            Self {
                _temp: temp,
                store,
                profile,
            }
        }
        fn fake_trash(&self, paths: &[PathBuf]) -> Result<Value> {
            let trash = self.store.home.join("fake-trash");
            fs::create_dir(&trash)?;
            for (i, path) in paths.iter().enumerate() {
                fs::rename(path, trash.join(i.to_string()))?;
            }
            Ok(serde_json::json!([]))
        }
    }
    #[test]
    fn removal_retains_accounts_by_default_and_unregisters() {
        let f = Fixture::new();
        let original = fs::read(f.store.profile_dir("work").unwrap().join("profile.json")).unwrap();
        let result = remove_with(
            &f.store,
            "work",
            false,
            |_| Ok(()),
            |paths| {
                assert_eq!(paths.len(), 1);
                f.fake_trash(paths)
            },
        )
        .unwrap();
        let retained = result.retained_data.unwrap();
        assert_eq!(
            fs::read(retained.join("codex/sentinel")).unwrap(),
            b"test account data"
        );
        assert_eq!(fs::read(retained.join("profile.json")).unwrap(), original);
        assert!(f.store.list().unwrap().is_empty());
        assert!(!f.profile.app_bundle.exists());
    }
    #[test]
    fn explicit_account_removal_trashes_the_whole_profile() {
        let f = Fixture::new();
        let result = remove_with(
            &f.store,
            "work",
            true,
            |_| Ok(()),
            |paths| {
                assert_eq!(paths.len(), 2);
                f.fake_trash(paths)
            },
        )
        .unwrap();
        assert!(result.retained_data.is_none());
        assert!(f.store.list().unwrap().is_empty());
        assert_eq!(
            fs::read(f.store.home.join("fake-trash/1/codex/sentinel")).unwrap(),
            b"test account data"
        );
    }
    #[test]
    fn failed_trash_restores_profile_and_data() {
        let f = Fixture::new();
        let result = remove_with(
            &f.store,
            "work",
            false,
            |_| Ok(()),
            |_| anyhow::bail!("injected trash failure"),
        );
        assert!(result.is_err());
        assert_eq!(f.store.list().unwrap().len(), 1);
        assert!(f.profile.app_bundle.exists());
        assert_eq!(
            fs::read(f.profile.codex_home.join("sentinel")).unwrap(),
            b"test account data"
        );
    }
    #[test]
    fn running_instance_is_rejected_before_any_moves() {
        let f = Fixture::new();
        let result = remove_with(
            &f.store,
            "work",
            true,
            |_| anyhow::bail!("still running"),
            |_| panic!("must not trash"),
        );
        assert!(result.is_err());
        assert!(f.profile.app_bundle.exists());
        assert!(f.profile.codex_home.join("sentinel").exists());
        assert!(!f.store.root.join("retained").exists());
    }
    #[test]
    fn adopted_or_rerouted_data_cannot_be_removed() {
        let f = Fixture::new();
        let manifest = f.store.profile_dir("work").unwrap().join("profile.json");
        let mut p = f.profile.clone();
        p.adopted_data = true;
        fs::write(&manifest, serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(remove_with(
            &f.store,
            "work",
            true,
            |_| Ok(()),
            |_| panic!("must not trash")
        )
        .is_err());
        p.adopted_data = false;
        p.codex_home = f.store.home.join("external-account");
        fs::create_dir(&p.codex_home).unwrap();
        fs::write(&manifest, serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(remove_with(
            &f.store,
            "work",
            true,
            |_| Ok(()),
            |_| panic!("must not trash")
        )
        .is_err());
        assert!(p.codex_home.exists());
    }
}
