use crate::{environment, fsutil, AppInfo, Profile, Store};
use anyhow::{bail, ensure, Context, Result};
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RunningProcess {
    pub pid: u32,
    /// Only processes whose executable matches the selected profile are exposed.
    /// Status output does NOT print this string, which can contain arguments.
    pub command: String,
}

#[derive(Debug)]
pub enum LaunchResult {
    Started { pid: u32 },
    AlreadyRunning { pid: u32 },
}

pub fn require_macos() -> Result<()> {
    ensure!(
        cfg!(target_os = "macos"),
        "Desktop launch/status requires macOS; other Unix systems support library tests only"
    );
    // No need to run app/profile managers as root.
    // SAFETY: geteuid has no preconditions and does not mutate memory.
    ensure!(
        unsafe { libc::geteuid() } != 0,
        "Do not launch Harbor with sudo or as root"
    );
    Ok(())
}

/// Process-table inspection, not a security identity check. Lifecycle actions must
/// revalidate identity; unexpected main-app command lines are treated as conflicts.
pub fn find_running(executable: &Path) -> Result<Vec<RunningProcess>> {
    require_macos()?;
    let output = Command::new("/bin/ps")
        .args(["-ww", "-axo", "pid=,command="])
        .output()
        .context("Read macOS process table")?;
    ensure!(output.status.success(), "ps failed");
    let expected = executable.to_str().context("Non-UTF-8 executable path")?;
    Ok(parse_ps(&String::from_utf8_lossy(&output.stdout), expected))
}

/// Includes helpers whose executable is inside this exact bundle, even after its main process exits.
pub fn find_bundle_processes(bundle: &Path) -> Result<Vec<u32>> {
    require_macos()?;
    let output = Command::new("/bin/ps")
        .args(["-ww", "-axo", "pid=,comm="])
        .output()
        .context("Inspect app and helper processes")?;
    ensure!(output.status.success(), "ps failed");
    let prefix = format!("{}/", bundle.display());
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let split = line.find(char::is_whitespace)?;
            let pid = line[..split].parse().ok()?;
            line[split..]
                .trim_start()
                .starts_with(&prefix)
                .then_some(pid)
        })
        .collect())
}

fn parse_ps(text: &str, executable: &str) -> Vec<RunningProcess> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let split = line.find(char::is_whitespace)?;
            let pid = line[..split].parse::<u32>().ok()?;
            let command = line[split..].trim_start();
            let rest = command.strip_prefix(executable)?;
            if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
                return None;
            }
            Some(RunningProcess {
                pid,
                command: command.to_string(),
            })
        })
        .collect()
}

/// Exact rendered command of the verified launch path. Extra/missing arguments
/// are a conflict, not a guess that the right profile must be running.
pub fn matches_profile(process: &RunningProcess, profile: &Profile) -> bool {
    process.command
        == format!(
            "{} --user-data-dir={}",
            profile.executable.display(),
            profile.gui_home.display()
        )
}

pub fn launch(store: &Store, name: &str, accept_version_change: bool) -> Result<LaunchResult> {
    require_macos()?;
    let _guard = fsutil::lock_registry(&store.root)?;
    let profile = store.load(name)?;
    store.validate_live_paths(&profile)?;
    let current = AppInfo::inspect(&profile.app_bundle)?;
    ensure!(current.executable == profile.executable && current.bundle_id == profile.bundle_id,
        "App identity changed since registration. The clone may have been replaced by an updater; do not launch it against this account yet");
    ensure!(accept_version_change || profile.registered_app_build_version.is_some(),
        "Profile has no registered app build number. Inspect it with 'harbor doctor {}'; use --accept-version-change only after reviewing the app. The old registration is not modified", name);
    ensure!(accept_version_change || (current.version == profile.registered_app_version &&
        Some(current.build_version.as_str()) == profile.registered_app_build_version.as_deref()),
        "App version changed ({} build {} -> {} build {}). Inspect it with 'harbor doctor {}'; use --accept-version-change only after reviewing the update",
        profile.registered_app_version, profile.registered_app_build_version.as_deref().unwrap_or("unrecorded"),
        current.version, current.build_version, name);

    let processes = find_running(&profile.executable)?;
    if !processes.is_empty() {
        if processes.len() == 1 && matches_profile(&processes[0], &profile) {
            return Ok(LaunchResult::AlreadyRunning {
                pid: processes[0].pid,
            });
        }
        let pids: Vec<u32> = processes.iter().map(|p| p.pid).collect();
        bail!("The selected app is already running with an unexpected or ambiguous profile (PIDs {pids:?}). Save tasks, quit THAT app with Cmd-Q, then retry. Harbor will not kill it");
    }

    let log_path = store.log_path(name)?;
    let mut log = fsutil::private_append(&log_path)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    writeln!(
        log,
        "\n===== Harbor launch: profile={} unix_time={} =====",
        profile.name, now
    )?;
    let env = environment::for_profile(
        std::env::vars_os(),
        &store.home,
        &profile.codex_home,
        &profile.gui_home,
        &profile.working_directory,
        &profile.pass_env,
    )?;
    let mut command = Command::new(&profile.executable);
    let mut user_data = OsString::from("--user-data-dir=");
    user_data.push(&profile.gui_home);
    command
        .arg(user_data)
        .env_clear()
        .envs(env)
        .current_dir(&profile.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    configure_detached(&mut command);
    let mut child = command
        .spawn()
        .with_context(|| format!("Launch {}; see {}", profile.name, log_path.display()))?;
    let pid = child.id();
    if let Some(status) = wait_for_early_exit(&mut child, Duration::from_secs(2))? {
        bail!("App exited during startup ({status}); inspect {}. Exit 0 can also mean the launch was forwarded to another instance", log_path.display());
    }
    // Dropping std::process::Child does not kill it. This is a short-lived CLI:
    // after it exits the app is reparented, just like the tested Python launcher.
    Ok(LaunchResult::Started { pid })
}

pub fn configure_detached(command: &mut Command) {
    // SAFETY: the pre-exec closure performs only async-signal-safe libc calls
    // (setsid/umask) plus construction of an OS error. No allocations, locks,
    // environment mutation, logging, or formatting are done in this closure.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            libc::umask(0o077);
            Ok(())
        });
    }
}

fn wait_for_early_exit(
    child: &mut Child,
    duration: Duration,
) -> Result<Option<std::process::ExitStatus>> {
    let deadline = Instant::now() + duration;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_matching_handles_spaces_and_rejects_suffixes() {
        let exe = "/Applications/ChatGPT Work.app/Contents/MacOS/ChatGPT";
        let table = format!("1 /sbin/launchd\n42 {exe} --user-data-dir=/a b/gui\n43 {exe}_Other\n44 /usr/bin/grep {exe}\n");
        let found = parse_ps(&table, exe);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pid, 42);
    }
    #[test]
    fn detached_mock_has_no_stdin_tty_and_can_write_log() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("mock.log");
        let log = fsutil::private_append(&log_path).unwrap();
        let mut cmd = Command::new("/bin/sh");
        cmd.args([
            "-c",
            "test ! -t 0 || exit 9; echo mock-ok; echo mock-err >&2",
        ])
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log);
        configure_detached(&mut cmd);
        assert!(cmd.spawn().unwrap().wait().unwrap().success());
        let text = std::fs::read_to_string(log_path).unwrap();
        assert!(text.contains("mock-ok") && text.contains("mock-err"));
    }
    #[test]
    fn detached_child_is_its_own_session_leader() {
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        configure_detached(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let pid = child.id();
        // SAFETY: query a process we just spawned; getsid writes no pointers.
        let session = unsafe { libc::getsid(pid as libc::pid_t) };
        // Clean up only this test-owned Child; production never kills a PID.
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(session, pid as libc::pid_t);
    }
    #[test]
    fn early_exit_is_reported() {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "exit 7"]).stdin(Stdio::null());
        configure_detached(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let status = wait_for_early_exit(&mut child, Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(status.code(), Some(7));
    }
}
