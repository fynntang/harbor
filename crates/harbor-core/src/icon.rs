//! Replace only a stopped local copy, using a verified staging bundle and atomic exchange.
use crate::{clone, fsutil, process, AppInfo, Profile, Store};
use anyhow::{ensure, Context, Result};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

pub fn set_icon(store: &Store, name: &str, image: &Path, tray_image: Option<&Path>) -> Result<()> {
    process::require_macos()?;
    let _lock = fsutil::lock_registry(&store.root)?;
    let profile = store.load(name)?;
    store.validate_live_paths(&profile)?;
    let app = AppInfo::inspect(&profile.app_bundle)?;
    ensure!(
        app.executable == profile.executable,
        "App executable changed; inspect this instance first"
    );
    validate_target(&profile, &app)?;
    ensure_stopped(&profile)?;
    let bytes = read_png(image)?;
    let tray_bytes = tray_image.map(read_png).transpose()?;
    let parent = profile.app_bundle.parent().context("Missing app parent")?;
    let staging = tempfile::Builder::new()
        .prefix(".harbor-icon-")
        .tempdir_in(parent)?;
    let source_png = staging.path().join("source.png");
    fs::write(&source_png, bytes)?;
    let prepared = staging.path().join("Prepared.app");
    clone::checked(
        Command::new("/bin/cp")
            .arg("-cR")
            .arg(&profile.app_bundle)
            .arg(&prepared),
        "Stage icon change",
    )?;
    verify(&prepared)?;
    let resources = prepared.join("Contents/Resources");
    for relative in [
        "Contents",
        "Contents/Info.plist",
        "Contents/MacOS",
        "Contents/_CodeSignature",
        "Contents/_CodeSignature/CodeResources",
        "Contents/Resources",
    ] {
        ensure!(
            !fs::symlink_metadata(prepared.join(relative))?
                .file_type()
                .is_symlink(),
            "Unsupported symlink in icon/signing path"
        );
    }
    let main = prepared
        .join("Contents/MacOS")
        .join(app.executable.file_name().context("Missing executable")?);
    ensure!(
        !fs::symlink_metadata(main)?.file_type().is_symlink(),
        "Unsupported executable symlink"
    );
    // This adapter covers the current production app and its runtime Dock icon choices.
    for file in [
        "icon-chatgpt.png",
        "icon-chatgpt.icns",
        "icon-codex-light.png",
        "icon-codex-dark-color.png",
    ] {
        let path = resources.join(file);
        ensure!(
            fs::symlink_metadata(&path)?.is_file(),
            "Unsupported icon layout: {file}"
        );
    }
    if let Some(bytes) = tray_bytes {
        let template = staging.path().join("template.png");
        fs::write(&template, bytes)?;
        for (file, size) in [("chatgptTemplate.png", 18), ("chatgptTemplate@2x.png", 36)] {
            ensure!(
                fs::symlink_metadata(resources.join(file))?.is_file(),
                "Unsupported menu-bar icon layout: {file}"
            );
            resize(&template, &resources.join(file), size)?;
        }
    }
    let iconset = staging.path().join("Harbor.iconset");
    fs::create_dir(&iconset)?;
    for (size, retina) in [
        (16, false),
        (16, true),
        (32, false),
        (32, true),
        (128, false),
        (128, true),
        (256, false),
        (256, true),
        (512, false),
        (512, true),
    ] {
        let pixel = if retina { size * 2 } else { size };
        let filename = format!("icon_{size}x{size}{}.png", if retina { "@2x" } else { "" });
        resize(&source_png, &iconset.join(filename), pixel)?;
    }
    let icns = staging.path().join("Harbor.icns");
    clone::checked(
        Command::new("/usr/bin/iconutil")
            .arg("-c")
            .arg("icns")
            .arg(&iconset)
            .arg("-o")
            .arg(&icns),
        "Build icon container",
    )?;
    for file in [
        "icon-chatgpt.png",
        "icon-codex-light.png",
        "icon-codex-dark-color.png",
    ] {
        resize(&source_png, &resources.join(file), 1024)?;
    }
    fs::copy(&icns, resources.join("icon-chatgpt.icns"))?;
    // Reuse an existing regular icon file: never follow a preexisting new-name symlink.
    let info_path = prepared.join("Contents/Info.plist");
    let mut info = plist::Value::from_file(&info_path)?;
    patch_info(info.as_dictionary_mut().context("Invalid Info.plist")?);
    info.to_file_xml(&info_path)?;
    let output = clone::checked(
        Command::new("/usr/bin/codesign")
            .args(["--display", "--entitlements", "-", "--xml"])
            .arg(&prepared),
        "Read local copy entitlements",
    )?;
    let entitlements =
        clone::filter_entitlements(&plist::Value::from_reader_xml(output.stdout.as_slice())?)?;
    let entitlements_path = staging.path().join("entitlements.plist");
    plist::Value::Dictionary(entitlements).to_file_xml(&entitlements_path)?;
    clone::checked(
        Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-", "--identifier"])
            .arg(&profile.bundle_id)
            .args(["--options", "runtime", "--timestamp=none", "--entitlements"])
            .arg(&entitlements_path)
            .arg(&prepared),
        "Sign icon change",
    )?;
    verify(&prepared)?;
    validate_target(&profile, &AppInfo::inspect(&prepared)?)?;
    ensure_stopped(&profile)?;
    // Metadata and both account directories stay untouched. The old app remains
    // in staging until the final verification passes and can be swapped back.
    exchange(&prepared, &profile.app_bundle)?;
    if let Err(error) = verify(&profile.app_bundle) {
        // On rollback failure retain the old app rather than auto-cleaning it.
        if let Err(rollback) = exchange(&prepared, &profile.app_bundle) {
            let backup = staging.keep();
            anyhow::bail!(
                "{error:#}; rollback failed: {rollback:#}; old app retained at {}",
                backup.display()
            );
        }
        return Err(error);
    }
    Ok(())
}

fn validate_target(profile: &Profile, app: &AppInfo) -> Result<()> {
    ensure!(
        !profile.adopted_data
            && profile.bundle_id == format!("com.openai.codex.harbor.{}", profile.name),
        "Only Harbor-created local copies support icon changes"
    );
    ensure!(
        app.bundle_id == profile.bundle_id
            && app.executable.file_name() == profile.executable.file_name()
            && app.version == profile.registered_app_version
            && Some(app.build_version.as_str()) == profile.registered_app_build_version.as_deref(),
        "App identity/version changed; inspect this instance first"
    );
    Ok(())
}
fn ensure_stopped(profile: &Profile) -> Result<()> {
    ensure!(
        process::find_running(&profile.executable)?.is_empty(),
        "Quit this instance with Cmd-Q before changing its icon; Harbor will not stop it"
    );
    Ok(())
}
fn verify(app: &Path) -> Result<()> {
    clone::checked(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(app),
        "Verify icon bundle signature",
    )?;
    Ok(())
}
fn resize(source: &Path, destination: &Path, pixels: u32) -> Result<()> {
    let size = pixels.to_string();
    clone::checked(
        Command::new("/usr/bin/sips")
            .args(["-z", &size, &size])
            .arg(source)
            .arg("--out")
            .arg(destination),
        "Render icon size",
    )?;
    Ok(())
}
pub(crate) fn patch_info(info: &mut plist::Dictionary) {
    info.remove("CFBundleIconName");
    info.insert(
        "CFBundleIconFile".into(),
        plist::Value::String("icon-chatgpt.icns".into()),
    );
    info.insert(
        "CodexAppIconBaseName".into(),
        plist::Value::String("icon-chatgpt".into()),
    );
}
fn read_png(image: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(image)
        .context("Read selected icon PNG")?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    validate_png(&bytes)?;
    Ok(bytes)
}

fn validate_png(bytes: &[u8]) -> Result<()> {
    ensure!(
        bytes.len() >= 33 && bytes.len() <= 16 * 1024 * 1024,
        "Icon PNG must be at most 16 MiB"
    );
    ensure!(
        bytes[..8] == *b"\x89PNG\r\n\x1a\n" && bytes[12..16] == *b"IHDR",
        "Select a PNG image"
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into()?);
    ensure!(
        width > 0 && width == height && width <= 4096,
        "Icon PNG must be square and at most 4096 pixels"
    );
    Ok(())
}
pub(crate) fn exchange(a: &Path, b: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let a = std::ffi::CString::new(a.as_os_str().as_bytes())?;
        let b = std::ffi::CString::new(b.as_os_str().as_bytes())?;
        // SAFETY: both valid C strings live through the atomic rename call.
        if unsafe { libc::renamex_np(a.as_ptr(), b.as_ptr(), libc::RENAME_SWAP) } != 0 {
            return Err(std::io::Error::last_os_error()).context("Publish icon change atomically");
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (a, b);
        anyhow::bail!("Icon changes require macOS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PNG: &[u8] = include_bytes!("../tests/fixtures/icon.png");

    #[test]
    fn icon_input_rejects_invalid_and_oversized_dimensions() {
        validate_png(PNG).unwrap();
        assert!(validate_png(b"not an icon").is_err());
        let mut invalid = PNG.to_vec();
        invalid[16..20].copy_from_slice(&9000u32.to_be_bytes());
        assert!(validate_png(&invalid).is_err());
        invalid[16..20].copy_from_slice(&15u32.to_be_bytes());
        assert!(validate_png(&invalid).is_err());
        invalid[16..24].fill(0);
        assert!(validate_png(&invalid).is_err());
    }

    #[test]
    fn metadata_patch_preserves_identity_version_and_routing() {
        let mut info = plist::Dictionary::new();
        for (key, value) in [
            ("CFBundleIdentifier", "com.openai.codex.harbor.work"),
            ("CFBundleVersion", "8109"),
            ("LSEnvironment", "routing-sentinel"),
            ("CFBundleIconName", "Icon"),
        ] {
            info.insert(key.into(), plist::Value::String(value.into()));
        }
        patch_info(&mut info);
        assert_eq!(
            info["CFBundleIdentifier"].as_string(),
            Some("com.openai.codex.harbor.work")
        );
        assert_eq!(info["CFBundleVersion"].as_string(), Some("8109"));
        assert_eq!(info["LSEnvironment"].as_string(), Some("routing-sentinel"));
        assert!(!info.contains_key("CFBundleIconName"));
    }

    #[cfg(target_os = "macos")]
    mod macos {
        use super::*;
        use std::os::unix::fs::{symlink, PermissionsExt};
        use std::path::PathBuf;
        use std::process::Stdio;

        struct Fixture {
            _temp: tempfile::TempDir,
            store: Store,
            profile: Profile,
            png: PathBuf,
        }
        impl Fixture {
            fn new() -> Self {
                let temp = tempfile::tempdir().unwrap();
                let home = temp.path().canonicalize().unwrap().join("home");
                let root = home.join("Harbor");
                fsutil::private_dir(&root.join("profiles")).unwrap();
                let app = home.join("Work.app");
                let resources = app.join("Contents/Resources");
                fs::create_dir_all(&resources).unwrap();
                fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
                // Use a copied local system executable, never a real account/client fixture.
                fs::copy("/bin/sleep", app.join("Contents/MacOS/ChatGPT")).unwrap();
                fs::set_permissions(
                    app.join("Contents/MacOS/ChatGPT"),
                    fs::Permissions::from_mode(0o700),
                )
                .unwrap();
                for file in [
                    "icon-chatgpt.png",
                    "icon-chatgpt.icns",
                    "icon-codex-light.png",
                    "icon-codex-dark-color.png",
                    "chatgptTemplate.png",
                    "chatgptTemplate@2x.png",
                    "harbor-original-icon.png",
                ] {
                    fs::write(resources.join(file), PNG).unwrap();
                }
                let mut info = plist::Dictionary::new();
                for (key, value) in [
                    ("CFBundleIdentifier", "com.openai.codex.harbor.work"),
                    ("CFBundleExecutable", "ChatGPT"),
                    ("CFBundleShortVersionString", "1"),
                    ("CFBundleVersion", "1"),
                    ("CrProductDirName", "com.openai.codex.harbor.work"),
                    ("CFBundleIconName", "Icon"),
                ] {
                    info.insert(key.into(), plist::Value::String(value.into()));
                }
                plist::Value::Dictionary(info)
                    .to_file_xml(app.join("Contents/Info.plist"))
                    .unwrap();
                let entitlements = home.join("entitlements.plist");
                plist::Value::Dictionary(plist::Dictionary::new())
                    .to_file_xml(&entitlements)
                    .unwrap();
                clone::checked(
                    Command::new("/usr/bin/codesign")
                        .args(["--force", "--sign", "-", "--entitlements"])
                        .arg(entitlements)
                        .arg(&app),
                    "Sign test fixture",
                )
                .unwrap();
                let store = Store { root, home };
                let profile = store.create("work", &app, None, vec![]).unwrap();
                let png = store.home.join("input.png");
                fs::write(&png, PNG).unwrap();
                Self {
                    _temp: temp,
                    store,
                    profile,
                    png,
                }
            }
        }
        #[test]
        fn real_icon_signing_preserves_profile_data_and_updates_both_surfaces() {
            let fixture = Fixture::new();
            let manifest = fixture
                .store
                .profile_dir("work")
                .unwrap()
                .join("profile.json");
            let before = fs::read(&manifest).unwrap();
            let data = fixture.profile.codex_home.join("test-sentinel");
            fs::write(&data, b"test-only").unwrap();
            set_icon(&fixture.store, "work", &fixture.png, Some(&fixture.png)).unwrap();
            verify(&fixture.profile.app_bundle).unwrap();
            assert_eq!(fs::read(manifest).unwrap(), before);
            assert_eq!(fs::read(data).unwrap(), b"test-only");
            let resources = fixture.profile.app_bundle.join("Contents/Resources");
            assert!(fs::read(resources.join("icon-chatgpt.icns"))
                .unwrap()
                .starts_with(b"icns"));
            assert_eq!(
                fs::read(resources.join("harbor-original-icon.png")).unwrap(),
                PNG
            );
            let tray = fs::read(resources.join("chatgptTemplate@2x.png")).unwrap();
            assert_eq!(u32::from_be_bytes(tray[16..20].try_into().unwrap()), 36);
            assert!(!fixture.store.home.read_dir().unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".harbor-icon-")));
        }
        #[test]
        fn running_instance_rejects_changes_without_touching_bundle() {
            let fixture = Fixture::new();
            let info = fixture.profile.app_bundle.join("Contents/Info.plist");
            let before = fs::read(&info).unwrap();
            let mut child = Command::new(&fixture.profile.executable)
                .arg("60")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let result = set_icon(&fixture.store, "work", &fixture.png, None);
            child.kill().unwrap();
            child.wait().unwrap();
            assert!(format!("{:#}", result.unwrap_err()).contains("Quit this instance"));
            assert_eq!(fs::read(info).unwrap(), before);
        }
        #[test]
        fn unsigned_bundle_and_external_resource_symlink_leave_source_untouched() {
            let fixture = Fixture::new();
            let info = fixture.profile.app_bundle.join("Contents/Info.plist");
            let before = fs::read(&info).unwrap();
            let resource = fixture
                .profile
                .app_bundle
                .join("Contents/Resources/icon-chatgpt.png");
            fs::remove_file(&resource).unwrap();
            symlink(&fixture.png, &resource).unwrap();
            assert!(set_icon(&fixture.store, "work", &fixture.png, None).is_err());
            assert_eq!(fs::read(&fixture.png).unwrap(), PNG);
            assert_eq!(fs::read(info).unwrap(), before);
        }
    }
}
