//! Pause only browser registrations pointing at this managed instance; restore on launch.
use crate::{auxiliary, fsutil, Profile, Store};
use anyhow::{ensure, Context, Result};
use std::os::unix::fs::MetadataExt;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const HOST: &str = "com.openai.codexextension.json";
const BROWSERS: [(&str, &str); 3] = [
    ("brave", "BraveSoftware/Brave-Browser"),
    ("chrome", "Google/Chrome"),
    ("edge", "Microsoft Edge"),
];

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    // SAFETY: geteuid has no preconditions.
    ensure!(
        meta.is_file()
            && !meta.file_type().is_symlink()
            && meta.len() <= 65536
            && meta.uid() == unsafe { libc::geteuid() },
        "Unsafe browser registration: {}",
        path.display()
    );
    Ok(Some(fs::read(path)?))
}

fn belongs(bytes: &[u8], profile: &Profile) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    value["name"] == "com.openai.codexextension"
        && value["path"]
            .as_str()
            .is_some_and(|path| auxiliary::browser_host(profile, Path::new(path)))
}

fn paths(home: &Path, backups: &Path) -> Vec<(PathBuf, PathBuf)> {
    BROWSERS
        .iter()
        .map(|(name, browser)| {
            (
                home.join("Library/Application Support")
                    .join(browser)
                    .join("NativeMessagingHosts")
                    .join(HOST),
                backups.join(format!("{name}.json")),
            )
        })
        .collect()
}

fn safe_parent(path: &Path) -> Result<()> {
    let parent = path.parent().context("Missing registration parent")?;
    ensure!(
        parent.canonicalize()? == parent,
        "Browser registration parent is redirected"
    );
    Ok(())
}

fn change(home: &Path, profile: &Profile, backups: &Path, pause: bool) -> Result<()> {
    for (active, backup) in paths(home, backups) {
        if pause {
            let Some(bytes) = read(&active)? else {
                continue;
            };
            if !belongs(&bytes, profile) {
                continue;
            }
            safe_parent(&active)?;
            fsutil::private_dir(backups)?;
            safe_parent(&backup)?;
            if let Some(previous) = read(&backup)? {
                ensure!(
                    belongs(&previous, profile),
                    "Browser backup belongs to another instance"
                );
            }
            let mut staged = tempfile::NamedTempFile::new_in(backups)?;
            staged.write_all(&bytes)?;
            staged.as_file().sync_all()?;
            staged.persist(&backup)?;
            ensure!(
                read(&active)?.as_deref() == Some(bytes.as_slice()),
                "Browser registration changed during pause; retry"
            );
            fs::remove_file(&active)?;
        } else {
            let Some(bytes) = read(&backup)? else {
                continue;
            };
            ensure!(
                belongs(&bytes, profile),
                "Browser backup no longer matches this instance"
            );
            // Another instance or the client itself may already have registered this host.
            if fs::symlink_metadata(&active).is_ok() {
                continue;
            }
            safe_parent(&active)?;
            let mut staged = tempfile::NamedTempFile::new_in(active.parent().unwrap())?;
            staged.write_all(&bytes)?;
            staged.as_file().sync_all()?;
            match staged.persist_noclobber(&active) {
                Ok(_) => {
                    fs::remove_file(&backup)?;
                }
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(())
}

pub fn set_paused(store: &Store, profile: &Profile, pause: bool) -> Result<()> {
    let directory = store.profile_dir(&profile.name)?;
    if profile.adopted_data
        || profile.bundle_id != format!("com.openai.codex.harbor.{}", profile.name)
        || profile.codex_home != directory.join("codex")
    {
        return Ok(());
    }
    change(
        &store.home,
        profile,
        &directory.join("browser-registrations"),
        pause,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_restore_is_scoped_repeatable_and_never_overwrites_another_registration() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().canonicalize().unwrap();
        let profile: Profile = serde_json::from_value(serde_json::json!({
            "schema_version":1,"name":"work","app_bundle":home.join("Work.app"),"executable":home.join("Work.app/Contents/MacOS/ChatGPT"),
            "bundle_id":"com.openai.codex.harbor.work","registered_app_version":"1","registered_app_build_version":"1",
            "codex_home":home.join("work/codex"),"gui_home":home.join("work/gui"),"working_directory":home,"pass_env":[],"adopted_data":false
        })).unwrap();
        let backups = home.join("backups");
        let targets = paths(&home, &backups);
        let own = serde_json::to_vec(&serde_json::json!({"name":"com.openai.codexextension","path":profile.codex_home.join("plugins/cache/openai-bundled/chrome/latest/extension-host/macos/arm64/ChatGPT for Chrome")})).unwrap();
        let foreign = br#"{"name":"com.openai.codexextension","path":"/other/host"}"#;
        for (active, _) in &targets {
            fs::create_dir_all(active.parent().unwrap()).unwrap();
        }
        fs::write(&targets[0].0, &own).unwrap();
        fs::write(&targets[1].0, foreign).unwrap();
        change(&home, &profile, &backups, true).unwrap();
        change(&home, &profile, &backups, true).unwrap();
        assert!(!targets[0].0.exists());
        assert_eq!(fs::read(&targets[0].1).unwrap(), own);
        assert_eq!(fs::read(&targets[1].0).unwrap(), foreign);
        fs::write(&targets[0].0, foreign).unwrap();
        change(&home, &profile, &backups, false).unwrap();
        assert_eq!(fs::read(&targets[0].0).unwrap(), foreign);
        assert!(targets[0].1.exists());
        fs::remove_file(&targets[0].0).unwrap();
        change(&home, &profile, &backups, false).unwrap();
        change(&home, &profile, &backups, false).unwrap();
        assert_eq!(fs::read(&targets[0].0).unwrap(), own);
        assert!(!targets[0].1.exists());
        fs::remove_file(&targets[0].0).unwrap();
        std::os::unix::fs::symlink(&targets[1].0, &targets[0].0).unwrap();
        assert!(change(&home, &profile, &backups, true).is_err());
        assert_eq!(fs::read(&targets[1].0).unwrap(), foreign);
    }
}
