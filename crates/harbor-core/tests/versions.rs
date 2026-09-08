use harbor_core::{fsutil, AppInfo, Store};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    app: PathBuf,
    marker: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("Harbor");
        fsutil::private_dir(&root.join("profiles")).unwrap();
        let app = base.join("ChatGPT Work.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        let marker = home.join("mock-started");
        let executable = app.join("Contents/MacOS/ChatGPT");
        fs::write(&executable, "#!/bin/sh\n: > mock-started\nexit 0\n").unwrap();
        fs::set_permissions(executable, fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = Self {
            _temp: temp,
            store: Store { root, home },
            app,
            marker,
        };
        fixture.write_plist("1.0", "100");
        fixture
    }

    fn write_plist(&self, version: &str, build: &str) {
        let mut dict = plist::Dictionary::new();
        for (key, value) in [
            ("CFBundleExecutable", "ChatGPT"),
            ("CFBundleIdentifier", "dev.harbor.test.work"),
            ("CFBundleShortVersionString", version),
            ("CFBundleVersion", build),
            ("CrProductDirName", "dev.harbor.test.work"),
        ] {
            dict.insert(key.into(), plist::Value::String(value.into()));
        }
        plist::Value::Dictionary(dict)
            .to_file_xml(self.plist())
            .unwrap();
    }

    fn plist(&self) -> PathBuf {
        self.app.join("Contents/Info.plist")
    }

    fn manifest(&self) -> PathBuf {
        self.store.profile_dir("work").unwrap().join("profile.json")
    }
}

#[test]
fn registration_records_short_version_and_build_number() {
    let f = Fixture::new();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    assert_eq!(json["registered_app_version"], "1.0");
    assert_eq!(json["registered_app_build_version"], "100");
    assert!(!f.marker.exists());
}

#[test]
fn missing_or_invalid_versions_are_rejected_before_registration() {
    for key in ["CFBundleShortVersionString", "CFBundleVersion"] {
        for invalid in [
            None,
            Some(plist::Value::String(String::new())),
            Some(plist::Value::String("  ".into())),
            Some(plist::Value::Integer(100.into())),
        ] {
            let f = Fixture::new();
            let mut plist = plist::Value::from_file(f.plist()).unwrap();
            let dict = plist.as_dictionary_mut().unwrap();
            match invalid {
                Some(value) => {
                    dict.insert(key.into(), value);
                }
                None => {
                    dict.remove(key);
                }
            }
            plist.to_file_xml(f.plist()).unwrap();
            let error = AppInfo::inspect(&f.app).unwrap_err().to_string();
            assert!(error.contains(key), "{error}");
            assert!(f.store.create("work", &f.app, None, vec![]).is_err());
            assert!(!f.store.profile_dir("work").unwrap().exists());
            assert!(!f.marker.exists());
        }
    }
}

#[cfg(target_os = "macos")]
fn assert_mock_started(f: &Fixture, accept_version_change: bool) {
    // An intentional early exit proves that the real launch path reached this
    // test-owned executable. A slow host may outlast launch's two-second check;
    // in that case, reap only the child that this test just launched.
    match harbor_core::process::launch(&f.store, "work", accept_version_change) {
        Err(error) => assert!(
            error.to_string().contains("App exited during startup"),
            "{error:#}"
        ),
        Ok(harbor_core::LaunchResult::Started { pid }) => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut status = 0;
            loop {
                // SAFETY: pid is this test's child, and status is writable.
                let reaped =
                    unsafe { libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG) };
                if reaped == pid as libc::pid_t {
                    assert!(libc::WIFEXITED(status));
                    assert_eq!(libc::WEXITSTATUS(status), 0);
                    break;
                }
                assert_eq!(reaped, 0, "Could not wait for the mock child");
                if std::time::Instant::now() >= deadline {
                    // SAFETY: the unreaped child is still owned by this test.
                    unsafe {
                        libc::kill(pid as libc::pid_t, libc::SIGKILL);
                        libc::waitpid(pid as libc::pid_t, &mut status, 0);
                    }
                    panic!("Mock executable did not exit");
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        Ok(harbor_core::LaunchResult::AlreadyRunning { .. }) => {
            panic!("Unexpected existing mock process")
        }
    }
    assert!(f.marker.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn matching_versions_reach_the_mock_executable() {
    let f = Fixture::new();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    assert_mock_started(&f, false);
}

#[cfg(target_os = "macos")]
#[test]
fn short_version_or_build_change_requires_explicit_acceptance() {
    for (version, build) in [("1.1", "100"), ("1.0", "101")] {
        let f = Fixture::new();
        f.store.create("work", &f.app, None, vec![]).unwrap();
        let before = fs::read(f.manifest()).unwrap();
        f.write_plist(version, build);
        let error = harbor_core::process::launch(&f.store, "work", false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("App version changed"), "{error}");
        assert!(!f.marker.exists());
        assert_mock_started(&f, true);
        assert_eq!(fs::read(f.manifest()).unwrap(), before);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn legacy_profile_loads_but_requires_explicit_version_acceptance() {
    let f = Fixture::new();
    f.store.create("work", &f.app, None, vec![]).unwrap();
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("registered_app_build_version");
    fs::write(f.manifest(), serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let before = fs::read(f.manifest()).unwrap();
    assert!(f.store.load("work").is_ok());
    let error = harbor_core::process::launch(&f.store, "work", false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("build number"), "{error}");
    assert!(!f.marker.exists());
    assert_mock_started(&f, true);
    assert_eq!(fs::read(f.manifest()).unwrap(), before);
}

#[cfg(target_os = "macos")]
#[test]
fn acceptance_does_not_bypass_invalid_current_versions_or_changed_identity() {
    for field in [
        "CFBundleVersion",
        "CFBundleShortVersionString",
        "CFBundleIdentifier",
    ] {
        let f = Fixture::new();
        f.store.create("work", &f.app, None, vec![]).unwrap();
        let mut plist = plist::Value::from_file(f.plist()).unwrap();
        let dict = plist.as_dictionary_mut().unwrap();
        if field == "CFBundleIdentifier" {
            dict.insert(
                field.into(),
                plist::Value::String("dev.harbor.test.replaced".into()),
            );
        } else {
            dict.remove(field);
        }
        plist.to_file_xml(f.plist()).unwrap();
        let error = harbor_core::process::launch(&f.store, "work", true)
            .unwrap_err()
            .to_string();
        if field == "CFBundleIdentifier" {
            assert!(error.contains("App identity changed"), "{error}");
        } else {
            assert!(error.contains(field), "{error}");
        }
        assert!(!f.marker.exists());
    }
}
