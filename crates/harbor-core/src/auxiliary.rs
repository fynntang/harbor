//! Stop known orphans and instance-scoped browser connections after the main app exits.
use crate::{process, Profile};
use anyhow::{ensure, Result};
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

pub(crate) fn browser_host(profile: &Profile, executable: &Path) -> bool {
    executable
        .strip_prefix(
            profile
                .codex_home
                .join("plugins/cache/openai-bundled/chrome"),
        )
        .ok()
        .is_some_and(|relative| {
            let parts: Vec<_> = relative.components().collect();
            parts.len() == 5
                && matches!(parts[0], std::path::Component::Normal(_))
                && parts[1].as_os_str() == "extension-host"
                && parts[2].as_os_str() == "macos"
                && parts[3].as_os_str() == "arm64"
                && parts[4].as_os_str() == "ChatGPT for Chrome"
        })
}

#[cfg(any(target_os = "macos", test))]
fn plugin_server(profile: &Profile, executable: &Path) -> bool {
    executable == profile.codex_home.join("plugins/.plugin-appserver/codex")
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
    let mut blocked = Vec::new();
    // Disconnect native hosts first so they cannot keep restarting their server children.
    let mut pids = running(profile)?;
    pids.sort_by_key(|pid| {
        inspect(*pid as i32).is_none_or(|identity| !browser_host(profile, &identity.path))
    });
    for pid in pids {
        let pid = i32::try_from(pid)?;
        let Some(identity) = inspect(pid) else {
            continue;
        };
        let browser_connection = browser_host(profile, &identity.path);
        let server_connection = plugin_server(profile, &identity.path)
            && (identity.parent == 1
                || inspect(identity.parent as i32).is_some_and(|parent| {
                    parent.uid == identity.uid && browser_host(profile, &parent.path)
                }));
        // SAFETY: geteuid has no preconditions. An active browser itself is never targeted.
        if identity.uid != unsafe { libc::geteuid() }
            || !(browser_connection
                || server_connection
                || (identity.parent == 1 && known(profile, &identity.path)))
        {
            blocked.push(format!(
                "PID {pid}, parent PID {}, {}",
                identity.parent,
                identity
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ));
            continue;
        }
        // Recheck the current executable, owner, parent and process birth time immediately before SIGTERM.
        // This is not a saved PID file and never falls back to SIGKILL.
        if inspect(pid).as_ref() != Some(&identity) {
            continue;
        }
        // SAFETY: a positive current PID was validated above; SIGTERM requests a normal exit.
        if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error).context("Request helper exit");
            }
        }
    }
    ensure!(blocked.is_empty(),
        "Active or unrecognized helpers remain: {}. Disconnect the owning integration or quit that helper, then retry cleanup. Update and deletion remain blocked",
        blocked.join("; "));
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
    fn disconnects_only_this_profiles_browser_tree() {
        use std::{
            fs,
            io::Write,
            process::{Command, Stdio},
            thread,
            time::{Duration, Instant},
        };
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let p: Profile = serde_json::from_value(serde_json::json!({
            "schema_version":1, "name":"work", "app_bundle":root.join("Work.app"),
            "executable":root.join("Work.app/Contents/MacOS/ChatGPT"),
            "bundle_id":"com.openai.codex.harbor.work", "registered_app_version":"1", "registered_app_build_version":"1",
            "codex_home":root.join("work/codex"), "gui_home":root.join("work/gui"), "working_directory":root,
            "pass_env":[], "adopted_data":false
        })).unwrap();
        let host = p.codex_home.join("plugins/cache/openai-bundled/chrome/latest/extension-host/macos/arm64/ChatGPT for Chrome");
        let server = p.codex_home.join("plugins/.plugin-appserver/codex");
        let other = root.join("other/codex/plugins/cache/openai-bundled/chrome/latest/extension-host/macos/arm64/ChatGPT for Chrome");
        assert!(browser_host(&p, &host));
        assert!(!browser_host(&p, &other));
        assert!(!browser_host(&p, &host.with_file_name("unknown")));
        assert!(plugin_server(&p, &server));
        assert!(!plugin_server(
            &p,
            &root.join("other/codex/plugins/.plugin-appserver/codex")
        ));
        let browser = root.join("browser");
        let mut compiler = Command::new("/usr/bin/cc")
            .args(["-x", "c", "-", "-o"])
            .arg(&browser)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        compiler.stdin.take().unwrap().write_all(b"#include <unistd.h>\nint main(int argc,char **argv){alarm(15);if(argc>1 && fork()==0){execl(argv[1],argv[1],argc>2?argv[2]:(char*)0,(char*)0);_exit(2);}for(;;)pause();}\n").unwrap();
        assert!(compiler.wait().unwrap().success());
        for target in [&host, &server, &other] {
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(&browser, target).unwrap();
        }
        let mut parent = Command::new(&browser)
            .arg(&host)
            .arg(&server)
            .spawn()
            .unwrap();
        let mut other_child = Command::new(&other).spawn().unwrap();
        // This server has an unrelated live parent and must not be terminated.
        let mut unrelated_server = Command::new(&server).spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while running(&p).unwrap().len() < 3 {
            assert!(Instant::now() < deadline, "browser tree did not start");
            thread::sleep(Duration::from_millis(20));
        }
        let tree: Vec<_> = running(&p)
            .unwrap()
            .into_iter()
            .filter(|pid| *pid != unrelated_server.id())
            .collect();
        assert_eq!(tree.len(), 2);
        loop {
            assert!(
                stop_orphans(&p).is_err(),
                "unrelated server must remain blocked"
            );
            if running(&p).unwrap() == vec![unrelated_server.id()] {
                break;
            }
            assert!(Instant::now() < deadline, "owned browser tree did not exit");
            thread::sleep(Duration::from_millis(20));
        }
        for child in [&mut parent, &mut other_child, &mut unrelated_server] {
            assert!(
                child.try_wait().unwrap().is_none(),
                "unrelated process was terminated"
            );
            child.kill().unwrap();
            child.wait().unwrap();
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
            display_name: None,
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
        while running(&p).unwrap().contains(&pid) {
            let error = stop_orphans(&p).unwrap_err().to_string();
            assert!(error.contains(&format!("PID {}", child.id())));
            assert!(error.contains("parent PID"));
            assert!(
                Instant::now() < deadline,
                "test orphan did not exit beside active helper"
            );
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "active helper must survive cleanup"
        );
        child.kill().unwrap();
        child.wait().unwrap();
    }
}
