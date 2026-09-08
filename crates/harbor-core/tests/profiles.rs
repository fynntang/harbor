use harbor_core::{fsutil, AppInfo, Store};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    app: PathBuf,
}

fn app_at(path: &Path, bundle_id: &str, binary_plist: bool) -> PathBuf {
    fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
    let executable = path.join("Contents/MacOS/ChatGPT");
    fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let mut dict = plist::Dictionary::new();
    for (key, value) in [
        ("CFBundleExecutable", "ChatGPT"),
        ("CFBundleIdentifier", bundle_id),
        ("CFBundleShortVersionString", "test-1"),
        ("CFBundleVersion", "100"),
        ("CrProductDirName", bundle_id),
    ] {
        dict.insert(key.to_string(), plist::Value::String(value.to_string()));
    }
    let value = plist::Value::Dictionary(dict);
    let output = path.join("Contents/Info.plist");
    if binary_plist {
        value.to_file_binary(output).unwrap();
    } else {
        value.to_file_xml(output).unwrap();
    }
    path.canonicalize().unwrap()
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    fs::create_dir(&home).unwrap();
    let root = home.join("Harbor");
    fsutil::private_dir(&root.join("profiles")).unwrap();
    let app = app_at(
        &base.join("ChatGPT Work.app"),
        "dev.harbor.test.work",
        false,
    );
    Fixture {
        _temp: temp,
        store: Store { root, home },
        app,
    }
}

#[test]
fn create_builds_empty_separate_directories_and_preserves_bundle() {
    let f = fixture();
    let before = fs::read(f.app.join("Contents/Info.plist")).unwrap();
    let p = f.store.create("work", &f.app, None, vec![]).unwrap();
    assert_ne!(p.codex_home, p.gui_home);
    assert!(p.codex_home.is_dir() && p.gui_home.is_dir());
    assert_eq!(fs::read_dir(&p.codex_home).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&p.gui_home).unwrap().count(), 0);
    assert_eq!(fs::read(f.app.join("Contents/Info.plist")).unwrap(), before);
    assert_eq!(
        f.store.load("work").unwrap().bundle_id,
        "dev.harbor.test.work"
    );
    assert_eq!(f.store.list().unwrap().len(), 1);
    assert_eq!(
        fs::metadata(&p.codex_home).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[test]
fn adopt_keeps_data_in_place_and_does_not_copy_auth() {
    let f = fixture();
    let codex = f.store.home.join("old-account/codex");
    let gui = f.store.home.join("old-account/gui");
    fs::create_dir_all(&codex).unwrap();
    fs::create_dir_all(&gui).unwrap();
    fs::write(codex.join("auth.json"), "not-a-real-token").unwrap();
    let p = f
        .store
        .adopt("work", &f.app, &codex, &gui, None, vec![])
        .unwrap();
    assert!(p.adopted_data);
    assert_eq!(p.codex_home, codex);
    assert_eq!(
        fs::read_to_string(codex.join("auth.json")).unwrap(),
        "not-a-real-token"
    );
    assert!(!f
        .store
        .profile_dir("work")
        .unwrap()
        .join("codex/auth.json")
        .exists());
}

#[test]
fn duplicate_name_does_not_overwrite_a_profile() {
    let f = fixture();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    let path = f.store.profile_dir("work").unwrap().join("profile.json");
    let before = fs::read(&path).unwrap();
    assert!(f.store.create("work", &f.app, None, vec![]).is_err());
    assert_eq!(before, fs::read(path).unwrap());
}

#[test]
fn same_app_cannot_be_registered_to_two_profiles() {
    let f = fixture();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    assert!(f.store.create("other", &f.app, None, vec![]).is_err());
}

#[test]
fn same_bundle_id_cannot_be_registered_twice() {
    let f = fixture();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    let other = app_at(
        &f.store.home.join("Other.app"),
        "dev.harbor.test.work",
        false,
    );
    assert!(f.store.create("other", &other, None, vec![]).is_err());
}

#[test]
fn adopt_rejects_nested_data_paths() {
    let f = fixture();
    let codex = f.store.home.join("old/codex");
    let gui = codex.join("gui");
    fs::create_dir_all(&gui).unwrap();
    assert!(f
        .store
        .adopt("work", &f.app, &codex, &gui, None, vec![])
        .is_err());
    assert!(!f.store.profile_dir("work").unwrap().exists());
}

#[test]
fn original_default_codex_is_not_adopted() {
    let f = fixture();
    let codex = f.store.home.join(".codex");
    let gui = f.store.home.join("gui");
    fs::create_dir(&codex).unwrap();
    fs::create_dir(&gui).unwrap();
    assert!(f
        .store
        .adopt("work", &f.app, &codex, &gui, None, vec![])
        .is_err());
}

#[test]
fn binary_info_plist_is_supported() {
    let f = fixture();
    let app = app_at(
        &f.store.home.join("Binary.app"),
        "dev.harbor.test.binary",
        true,
    );
    assert_eq!(
        AppInfo::inspect(&app).unwrap().bundle_id,
        "dev.harbor.test.binary"
    );
}

#[test]
fn another_profiles_data_cannot_be_reused() {
    let f = fixture();
    let first = f.store.create("work", &f.app, None, vec![]).unwrap();
    let app2 = app_at(
        &f.store.home.join("Other.app"),
        "dev.harbor.test.other",
        false,
    );
    assert!(f
        .store
        .adopt(
            "other",
            &app2,
            &first.codex_home,
            &first.gui_home,
            None,
            vec![]
        )
        .is_err());
}

#[test]
fn path_symlink_alias_cannot_bypass_data_conflict() {
    let f = fixture();
    let first = f.store.create("work", &f.app, None, vec![]).unwrap();
    let alias = f.store.home.join("alias");
    std::os::unix::fs::symlink(&first.codex_home, &alias).unwrap();
    let app2 = app_at(
        &f.store.home.join("Other.app"),
        "dev.harbor.test.other",
        false,
    );
    let gui = f.store.home.join("gui2");
    fs::create_dir(&gui).unwrap();
    assert!(f
        .store
        .adopt("other", &app2, &alias, &gui, None, vec![])
        .is_err());
}

#[test]
fn shortcut_is_new_file_only_and_keeps_correct_registry() {
    let f = fixture();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    let output = f.store.home.join("Harbor Work.command");
    f.store
        .shortcut("work", &output, Path::new("/bin/sh"))
        .unwrap();
    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("--root") && content.contains("start 'work'"));
    assert!(f
        .store
        .shortcut("work", &output, Path::new("/bin/sh"))
        .is_err());
}
