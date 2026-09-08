//! Only known, orphaned helpers belonging to this instance receive SIGTERM after the main app exits.
use crate::{process, Profile};
use anyhow::{ensure, Result};
#[cfg(any(target_os = "macos", test))]
use std::path::Path;

pub fn running(profile: &Profile) -> Result<Vec<u32>> {
    let mut pids = process::find_bundle_processes(&profile.app_bundle)?;
    pids.extend(process::find_bundle_processes(&profile.codex_home)?);
    pids.extend(process::find_bundle_processes(&profile.gui_home)?);
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(any(target_os = "macos", test))]
fn known(profile: &Profile, executable: &Path) -> bool {
    let crash_root = profile
        .app_bundle
        .join("Contents/Frameworks/Codex Framework.framework/Versions");
    let crash = executable
        .strip_prefix(crash_root)
        .ok()
        .is_some_and(|relative| {
            let parts: Vec<_> = relative.components().collect();
            parts.len() == 3
                && parts[1].as_os_str() == "Helpers"
                && parts[2].as_os_str() == "browser_crashpad_handler"
        });
    crash
        || executable
            == profile
                .app_bundle
                .join("Contents/Resources/native/bare-modifier-monitor")
        || executable
            == profile
                .codex_home
                .join("computer-use/Codex Computer Use.app/Contents/MacOS/SkyComputerUseService")
}

#[cfg(target_os = "macos")]
pub fn stop_orphans(profile: &Profile) -> Result<()> {
    use anyhow::Context;
    use std::{ffi::CStr, mem, path::PathBuf};
    #[derive(PartialEq)]
    struct Identity {
        path: PathBuf,
        uid: u32,
        parent: u32,
        start: (u64, u64),
    }
    fn inspect(pid: i32) -> Option<Identity> {
        let mut info = mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
        let mut buffer = [0u8; 4096];
        // SAFETY: both buffers are writable and sized for the requested libproc calls.
        unsafe {
            if libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                mem::size_of::<libc::proc_bsdinfo>() as i32,
            ) != mem::size_of::<libc::proc_bsdinfo>() as i32
                || libc::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) <= 0
            {
                return None;
            }
            let info = info.assume_init();
            let path = CStr::from_bytes_until_nul(&buffer).ok()?.to_str().ok()?;
            Some(Identity {
                path: PathBuf::from(path),
                uid: info.pbi_uid,
                parent: info.pbi_ppid,
                start: (info.pbi_start_tvsec, info.pbi_start_tvusec),
            })
        }
    }
    ensure!(
        process::find_running(&profile.executable)?.is_empty(),
        "Main app is still running; helper cleanup was not attempted"
    );
    for pid in running(profile)? {
        let pid = i32::try_from(pid)?;
        let Some(identity) = inspect(pid) else {
            continue;
        };
        // SAFETY: geteuid has no preconditions.
        ensure!(identity.uid == unsafe { libc::geteuid() } && identity.parent == 1 && known(profile, &identity.path),
            "An unknown or active helper remains (PID {pid}); quit it manually before deleting this instance");
        // Recheck the current executable, owner, parent and process birth time immediately before SIGTERM.
        // This is not a saved PID file and never falls back to SIGKILL.
        if inspect(pid).as_ref() != Some(&identity) {
            continue;
        }
        // SAFETY: a positive current PID was validated above; SIGTERM asks this orphan to exit.
        if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error).context("Request helper exit");
            }
        }
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
pub fn stop_orphans(_profile: &Profile) -> Result<()> {
    ensure!(false, "Helper cleanup requires macOS");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_known_helper_locations_are_accepted() {
        let p: Profile = serde_json::from_value(serde_json::json!({
            "schema_version":1, "name":"work", "app_bundle":"/test/Work.app", "executable":"/test/Work.app/Contents/MacOS/ChatGPT",
            "bundle_id":"com.openai.codex.harbor.work", "registered_app_version":"1", "registered_app_build_version":"1",
            "codex_home":"/test/account/codex", "gui_home":"/test/account/gui", "working_directory":"/test", "pass_env":[], "adopted_data":false
        })).unwrap();
        assert!(known(&p, Path::new("/test/Work.app/Contents/Frameworks/Codex Framework.framework/Versions/152/Helpers/browser_crashpad_handler")));
        assert!(known(&p, Path::new("/test/account/codex/computer-use/Codex Computer Use.app/Contents/MacOS/SkyComputerUseService")));
        assert!(known(
            &p,
            Path::new("/test/Work.app/Contents/Resources/native/bare-modifier-monitor")
        ));
        for path in [
            "/other/browser_crashpad_handler",
            "/test/Work.app/Contents/MacOS/ChatGPT",
            "/test/account/codex/unknown",
            "/test/account/codex/computer-use/Other.app/Contents/MacOS/SkyComputerUseService",
        ] {
            assert!(!known(&p, Path::new(path)));
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn live_child_is_preserved_but_known_orphan_exits() {
        use std::{
            fs,
            process::Command,
            thread,
            time::{Duration, Instant},
        };
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let p = Profile {
            schema_version: crate::SCHEMA_VERSION,
            name: "work".into(),
            app_bundle: root.join("Work.app"),
            executable: root.join("Work.app/Contents/MacOS/ChatGPT"),
            bundle_id: "com.openai.codex.harbor.work".into(),
            registered_app_version: "1".into(),
            registered_app_build_version: Some("1".into()),
            codex_home: root.join("codex"),
            gui_home: root.join("gui"),
            working_directory: root.clone(),
            pass_env: vec![],
            adopted_data: false,
        };
        let executable = p.app_bundle.join("Contents/Frameworks/Codex Framework.framework/Versions/test/Helpers/browser_crashpad_handler");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::copy("/bin/sleep", &executable).unwrap();
        assert!(Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&executable)
            .status()
            .unwrap()
            .success());
        let mut child = Command::new(&executable).arg("60").spawn().unwrap();
        thread::sleep(Duration::from_millis(100));
        let checked = stop_orphans(&p);
        let live = child.try_wait().unwrap();
        let observed = running(&p).unwrap();
        if live.is_none() {
            child.kill().unwrap();
            child.wait().unwrap();
        }
        assert!(live.is_none(), "test child exited early: {live:?}");
        assert!(
            checked.is_err(),
            "child was not rejected; observed: {observed:?}"
        );
        let output = Command::new("/bin/sh")
            .args([
                "-c",
                "\"$1\" 60 >/dev/null 2>&1 </dev/null & echo $!",
                "harbor-test",
            ])
            .arg(&executable)
            .output()
            .unwrap();
        assert!(output.status.success());
        let pid: u32 = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !running(&p).unwrap().contains(&pid) {
            assert!(Instant::now() < deadline, "test orphan did not launch");
            thread::sleep(Duration::from_millis(20));
        }
        loop {
            if stop_orphans(&p).is_ok() {
                break;
            }
            assert!(Instant::now() < deadline, "test orphan did not detach");
            thread::sleep(Duration::from_millis(20));
        }
        while running(&p).unwrap().contains(&pid) {
            assert!(Instant::now() < deadline, "test orphan did not exit");
            thread::sleep(Duration::from_millis(20));
        }
    }
}
