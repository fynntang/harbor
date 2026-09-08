mod json;
use anyhow::{ensure, Context, Result};
use clap::{Args, Parser, Subcommand};
use harbor_core::{fsutil, process, AppInfo, LaunchResult, Profile, Store};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "harbor",
    version,
    about = "Create and launch local ChatGPT/Codex desktop profiles"
)]
struct Cli {
    /// Metadata directory (default: ~/Library/Application Support/Harbor).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Emit a versioned JSON response (list/show/status/clone/icon/start/stop/remove/doctor).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct InstanceArgs {
    /// Lowercase profile name, e.g. work or personal.
    name: String,
    /// Existing, already-prepared .app bundle. Harbor never modifies it.
    #[arg(long)]
    app: PathBuf,
    /// Startup working directory; defaults to HOME, not the calling repository.
    #[arg(long)]
    cwd: Option<PathBuf>,
    /// Additional environment variable NAME to forward; repeat as needed.
    #[arg(long = "pass-env")]
    pass_env: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy the original app, sign the local copy, and register NEW empty data directories.
    Clone {
        name: String,
        /// Original OpenAI-signed app (for example /Applications/ChatGPT.app).
        #[arg(long)]
        source: PathBuf,
        /// New app path (default: ~/Applications/Harbor/ChatGPT-<name>.app).
        #[arg(long)]
        app: Option<PathBuf>,
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Additional environment variable NAME to forward; repeat as needed.
        #[arg(long = "pass-env")]
        pass_env: Vec<String>,
    },
    /// Change a stopped Harbor-created copy's Finder/Dock icon, then verify its local signature.
    Icon {
        name: String,
        /// Square PNG image, at most 4096 pixels and 16 MiB.
        #[arg(long)]
        image: PathBuf,
        /// Optional monochrome/transparent PNG for the macOS menu bar.
        #[arg(long)]
        tray_image: Option<PathBuf>,
    },
    /// Register an existing app with NEW empty Codex and GUI data directories.
    Create {
        #[command(flatten)]
        instance: InstanceArgs,
    },
    /// Register existing account directories IN PLACE; no copy, move, or deletion.
    Adopt {
        #[command(flatten)]
        instance: InstanceArgs,
        #[arg(long)]
        codex_home: PathBuf,
        #[arg(long)]
        gui_home: PathBuf,
    },
    /// List registered profiles, without reading any credential files.
    List,
    /// Show routing metadata (not application config, tokens, or cookies).
    Show { name: String },
    /// Launch detached from this terminal, using both profile directories.
    Start {
        name: String,
        /// Print the launch routing without launching an application.
        #[arg(long)]
        dry_run: bool,
        /// Permit a different app version, but never a different bundle ID/path.
        #[arg(long)]
        accept_version_change: bool,
    },
    /// Ask the exact registered app to quit normally; never force-kill it.
    Stop { name: String },
    /// Move a stopped Harbor copy to Trash; retain account data unless explicitly selected.
    Remove {
        name: String,
        #[arg(long)]
        delete_data: bool,
        /// Confirm the selected removal scope.
        #[arg(long)]
        yes: bool,
    },
    /// Inspect the selected app's process, not a possibly recycled saved PID.
    Status { name: String },
    /// Read-only checks: app metadata, signature, process routing, shared sources.
    Doctor { name: String },
    /// Print the last N log lines (read is capped at 1 MiB).
    Logs {
        name: String,
        #[arg(short = 'n', long, default_value_t = 60)]
        lines: usize,
    },
    /// Create a .command shortcut pointing to this installed Harbor executable.
    Shortcut {
        name: String,
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() {
    // SAFETY: this single-threaded CLI sets its file-creation mask before doing
    // any I/O. It does not change permissions on adopted account files.
    unsafe {
        libc::umask(0o077);
    }
    let cli = Cli::parse();
    if cli.json {
        json::respond(cli);
        return;
    }
    if let Err(error) = run(cli) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let store = Store::open(cli.root)?;
    match cli.command {
        Commands::Clone {
            name,
            source,
            app,
            cwd,
            pass_env,
        } => {
            let p = harbor_core::clone::clone_profile(
                &store,
                &name,
                &source,
                app.as_deref(),
                cwd.as_deref(),
                pass_env,
            )?;
            println!("Created local copy and empty profile '{}'. Source app and existing accounts were not modified.", p.name);
            show_routes(&p);
            println!("The copy is locally ad-hoc signed; it does not retain the vendor identity or notarization.\nStart with: harbor --root {} start {}\nSign in to the intended account in its new window.", fsutil::shell_quote(store.root.to_str().context("Non-UTF-8 registry path")?), fsutil::shell_quote(&p.name));
        }
        Commands::Icon {
            name,
            image,
            tray_image,
        } => {
            harbor_core::icon::set_icon(&store, &name, &image, tray_image.as_deref())?;
            println!("Updated icon for '{name}'. Local signature verified; restart that app to see it in the Dock.");
        }
        Commands::Create { instance } => {
            let p = store.create(
                &instance.name,
                &instance.app,
                instance.cwd.as_deref(),
                instance.pass_env,
            )?;
            println!("Created profile '{}'. App bundle was not modified.", p.name);
            show_routes(&p);
        }
        Commands::Adopt {
            instance,
            codex_home,
            gui_home,
        } => {
            let p = store.adopt(
                &instance.name,
                &instance.app,
                &codex_home,
                &gui_home,
                instance.cwd.as_deref(),
                instance.pass_env,
            )?;
            println!(
                "Adopted profile '{}' in place. Account files were not copied or modified.",
                p.name
            );
            show_routes(&p);
        }
        Commands::List => {
            let profiles = store.list()?;
            if profiles.is_empty() {
                println!(
                    "No registered profiles. Use 'harbor adopt --help' or 'harbor create --help'."
                );
            }
            for p in profiles {
                println!("{}\t{}\t{}", p.name, p.bundle_id, p.app_bundle.display());
            }
        }
        Commands::Show { name } => {
            println!("{}", serde_json::to_string_pretty(&store.load(&name)?)?)
        }
        Commands::Start {
            name,
            dry_run,
            accept_version_change,
        } => {
            if dry_run {
                let p = store.load(&name)?;
                store.validate_live_paths(&p)?;
                println!("DRY RUN — no process started; signature/runtime compatibility is not verified.");
                show_routes(&p);
                println!("CODEX_SPARKLE_ENABLED=false");
                println!(
                    "Argument (one argv entry): --user-data-dir={}",
                    p.gui_home.display()
                );
                println!(
                    "stdin=/dev/null; stdout/stderr={}; setsid=true",
                    store.log_path(&name)?.display()
                );
                println!(
                    "HOME/SSH/Docker remain shared. Extra env NAMES: {:?}",
                    p.pass_env
                );
            } else {
                match process::launch(&store, &name, accept_version_change)? {
                    LaunchResult::Started { pid } => println!("Started '{name}', PID {pid}. Process survived the 2-second startup check; check its window/account.\nLog: {}", store.log_path(&name)?.display()),
                    LaunchResult::AlreadyRunning { pid } => println!("'{name}' is already running with the expected GUI argument (PID {pid}). No second process was started."),
                }
            }
        }
        Commands::Stop { name } => {
            println!("{}: {}", name, harbor_core::lifecycle::stop(&store, &name)?)
        }
        Commands::Remove {
            name,
            delete_data,
            yes,
        } => {
            ensure!(yes, "Review 'harbor show {name}' and pass --yes to move this copy to Trash; --delete-data also trashes its account directories");
            let result = harbor_core::lifecycle::remove(&store, &name, delete_data)?;
            println!(
                "Copy moved to Trash. {}",
                result
                    .retained_data
                    .map(|p| format!("Account data retained at {}", p.display()))
                    .unwrap_or_else(|| "Account data and registration also moved to Trash.".into())
            );
        }
        Commands::Status { name } => print_status(&store.load(&name)?, &mut std::io::stdout())?,
        Commands::Doctor { name } => doctor(&store, &name, &mut std::io::stdout())?,
        Commands::Logs { name, lines } => {
            store.load(&name)?;
            let path = store.log_path(&name)?;
            if path.try_exists()? {
                println!("{}", fsutil::tail(&path, lines)?);
            } else {
                println!("No Harbor launch log yet: {}", path.display());
            }
        }
        Commands::Shortcut { name, output } => {
            let executable = std::env::current_exe()?;
            store.shortcut(&name, &output, &executable)?;
            println!(
                "Created {}\nTarget CLI: {}\nKeep this CLI installed at that path.",
                output.display(),
                executable.display()
            );
        }
    }
    Ok(())
}

fn show_routes(p: &Profile) {
    println!("App:        {}\nExecutable: {}\nBundle ID:  {}\nCodex home: {}\nGUI home:   {}\nWorking dir: {}",
        p.app_bundle.display(), p.executable.display(), p.bundle_id,
        p.codex_home.display(), p.gui_home.display(), p.working_directory.display());
}

fn print_status(p: &Profile, out: &mut impl std::io::Write) -> Result<()> {
    let processes = process::find_running(&p.executable)?;
    if processes.is_empty() {
        writeln!(out, "{}: stopped (no matching process found)", p.name)?;
    } else {
        for running in processes {
            let state = if process::matches_profile(&running, p) {
                "running / expected GUI argument"
            } else {
                "CONFLICT / unexpected launch arguments"
            };
            writeln!(out, "{}: PID {} — {}", p.name, running.pid, state)?;
        }
    }
    Ok(())
}

fn doctor(store: &Store, name: &str, out: &mut impl std::io::Write) -> Result<()> {
    let profile = store.load(name)?;
    store.validate_live_paths(&profile)?;
    let app = AppInfo::inspect(&profile.app_bundle)?;
    let mut failed = false;
    writeln!(
        out,
        "App:        {}\nExecutable: {}\nBundle ID:  {}\nCodex home: {}\nGUI home:   {}\nWorking dir: {}",
        profile.app_bundle.display(),
        profile.executable.display(),
        profile.bundle_id,
        profile.codex_home.display(),
        profile.gui_home.display(),
        profile.working_directory.display()
    )?;
    writeln!(out, "\nPASS: data directories exist, resolve to registered paths, and do not overlap each other.")?;
    if app.bundle_id == profile.bundle_id && app.executable == profile.executable {
        writeln!(out, "PASS: current app identity matches the registration.")?;
    } else {
        failed = true;
        writeln!(
            out,
            "FAIL: app identity changed; start will refuse to launch it."
        )?;
    }
    if profile.registered_app_build_version.is_none() {
        writeln!(out, "WARN: this profile has no registered app build number. Current app: {} build {}. Start requires --accept-version-change after review; the registration will not be modified.", app.version, app.build_version)?;
    } else if app.version != profile.registered_app_version
        || Some(app.build_version.as_str()) != profile.registered_app_build_version.as_deref()
    {
        writeln!(
            out,
            "WARN: app version changed: {} build {} -> {} build {}.",
            profile.registered_app_version,
            profile
                .registered_app_build_version
                .as_deref()
                .unwrap_or("unrecorded"),
            app.version,
            app.build_version
        )?;
    }
    if cfg!(target_os = "macos") {
        let output = Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict", "--verbose=2"])
            .arg(&profile.app_bundle)
            .output()
            .context("Run read-only codesign verification")?;
        writeln!(
            out,
            "{}: codesign verification (not a notarization or vendor-trust check).",
            if output.status.success() {
                "PASS"
            } else {
                "FAIL"
            }
        )?;
        if !output.status.success() {
            failed = true;
            let text = String::from_utf8_lossy(&output.stderr);
            writeln!(out, "{}", text.chars().take(4000).collect::<String>())?;
        }
        print_status(&profile, out)?;
    } else {
        writeln!(
            out,
            "WARN: macOS signature/process checks skipped on this platform."
        )?;
    }
    let shared_skills = store.home.join(".agents/skills");
    writeln!(out, "\nBOUNDARY: shared HOME, SSH, Docker, Keychain, native services, and repository access are not isolated.")?;
    writeln!(
        out,
        "User skills location: {} (exists={}).",
        shared_skills.display(),
        shared_skills.exists()
    )?;
    writeln!(out, "Repository/admin skills, external MCP storage and config.toml path overrides are NOT audited by this command.")?;
    writeln!(out, "GUI env/updater switches are version-specific; startup success is not a complete isolation/security test.")?;
    ensure!(
        !failed,
        "One or more app checks failed; no repairs were attempted"
    );
    Ok(())
}
