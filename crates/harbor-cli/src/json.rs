//! Versioned GUI transport; stdout is one response, stderr contains only progress events.
use crate::{Cli, Commands};
use anyhow::{bail, Result};
use harbor_core::{process, AppInfo, LaunchResult, Profile, Store};
use serde_json::{json, Value};

pub fn respond(cli: Cli) {
    let result = run(cli);
    let (response, code) = match result {
        Ok(data) => (json!({"api_version": 1, "ok": true, "data": data}), 0),
        Err(error) => (
            json!({"api_version": 1, "ok": false, "error": format!("{error:#}")}),
            1,
        ),
    };
    println!("{response}");
    if code != 0 {
        std::process::exit(code);
    }
}

fn status(profile: &Profile) -> Result<Value> {
    let running = process::find_running(&profile.executable)?;
    let state = if running.is_empty() {
        if harbor_core::auxiliary::running(profile)?.is_empty() {
            "stopped"
        } else {
            "helpers_running"
        }
    } else if running.len() == 1 && process::matches_profile(&running[0], profile) {
        "running"
    } else {
        "conflict"
    };
    Ok(json!({"state": state, "pids": running.iter().map(|p| p.pid).collect::<Vec<_>>()}))
}

fn run(cli: Cli) -> Result<Value> {
    let store = Store::open(cli.root)?;
    match cli.command {
        Commands::List => {
            let mut items = Vec::new();
            for p in store.list()? {
                let inspected = store
                    .validate_live_paths(&p)
                    .and_then(|_| AppInfo::inspect(&p.app_bundle));
                let (version, build, issue) = match inspected {
                    Ok(app) => {
                        let issue =
                            if app.bundle_id != p.bundle_id || app.executable != p.executable {
                                Some("应用身份已变化，请检查后再启动".to_string())
                            } else if app.version != p.registered_app_version
                                || Some(app.build_version.as_str())
                                    != p.registered_app_build_version.as_deref()
                            {
                                Some("应用版本已变化，请检查；GUI 不会自动接受版本变化".to_string())
                            } else {
                                None
                            };
                        (Some(app.version), Some(app.build_version), issue)
                    }
                    Err(error) => (None, None, Some(format!("{error:#}"))),
                };
                let state = status(&p).unwrap_or_else(
                    |e| json!({"state":"unknown", "pids":[], "error":format!("{e:#}")}),
                );
                items.push(json!({"profile":p, "status":state, "current_version":version, "current_build":build, "issue":issue}));
            }
            Ok(json!({"profiles":items, "root":store.root}))
        }
        Commands::Icon {
            name,
            image,
            tray_image,
        } => {
            harbor_core::icon::set_icon(&store, &name, &image, tray_image.as_deref())?;
            Ok(json!({"updated":true}))
        }
        Commands::Stop { name } => {
            Ok(json!({"state":harbor_core::lifecycle::stop(&store, &name)?}))
        }
        Commands::Remove {
            name,
            delete_data,
            yes,
        } => {
            anyhow::ensure!(yes, "Removal requires --yes after reviewing its scope");
            Ok(serde_json::to_value(harbor_core::lifecycle::remove(
                &store,
                &name,
                delete_data,
            )?)?)
        }
        Commands::Show { name } => Ok(serde_json::to_value(store.load(&name)?)?),
        Commands::Status { name } => status(&store.load(&name)?),
        Commands::Clone {
            name,
            source,
            app,
            cwd,
            pass_env,
        } => {
            let profile = harbor_core::clone::clone_profile_with_progress(
                &store,
                &name,
                &source,
                app.as_deref(),
                cwd.as_deref(),
                pass_env,
                |stage| eprintln!("{}", json!({"event":"progress", "stage":stage})),
            )?;
            Ok(json!({"profile":profile}))
        }
        Commands::Start {
            name,
            dry_run,
            accept_version_change,
        } => {
            if dry_run {
                let p = store.load(&name)?;
                store.validate_live_paths(&p)?;
                return Ok(json!({"state":"dry_run", "profile":p}));
            }
            match process::launch(&store, &name, accept_version_change)? {
                LaunchResult::Started { pid } => Ok(json!({"state":"started", "pid":pid})),
                LaunchResult::AlreadyRunning { pid } => {
                    Ok(json!({"state":"already_running", "pid":pid}))
                }
            }
        }
        Commands::Doctor { name } => {
            let mut report = Vec::new();
            let checked = crate::doctor(&store, &name, &mut report);
            Ok(
                json!({"passed":checked.is_ok(), "report":String::from_utf8(report)?, "error":checked.err().map(|e| format!("{e:#}"))}),
            )
        }
        _ => bail!("JSON supports list, show, status, clone, icon, start, stop, remove and doctor"),
    }
}
