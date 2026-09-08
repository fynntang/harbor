# Implementation and limits

English | [简体中文](zh-CN/ARCHITECTURE.md) · [Documentation](README.md)

This page describes the checked-in implementation as reviewed on 2026-09-08. Code is authoritative when descriptions diverge. It is not an upstream compatibility promise or security certification.

## Components

| Component | Responsibility |
|---|---|
| [SwiftUI GUI](../apps/Harbor/Sources/Harbor/) | Views, operation ordering, image conversion, menu/window lifecycle. |
| [Native helper](../apps/Harbor/Sources/HarborNative/) | NSRunningApplication quit requests and FileManager Trash operations. |
| [Rust CLI](../crates/harbor-cli/src/main.rs) | Clap commands, text diagnostics and JSON transport. |
| [Core](../crates/harbor-core/src/) | Path/identity policy, registry, clone, launch, icons, stop/removal. |

The GUI invokes the bundled CLI rather than reimplementing core policy. The native helper is found beside the CLI executable. A standalone Rust install does not contain it. There is no server or always-running supervisor.

## Registration and paths

[AppInfo](../crates/harbor-core/src/app.rs) parses XML/binary Info.plist, requires executable name `ChatGPT` or `Codex`, a Chromium marker (`CrProductDirName` or `ElectronAsarIntegrity`), nonempty ID/version/build strings, and an executable resolving inside the bundle. This is a layout check, not proof of compatibility or vendor trust.

[Store](../crates/harbor-core/src/store.rs) defaults to `~/Library/Application Support/Harbor`. Opening it creates/permissions the registry and `profiles` directories if needed, even for nominally read-oriented commands such as `list`. Protection checks precede those writes. `--root` must be absolute, without `..`, and cannot overlap HOME/root/default account paths or their resolved aliases. Reserved default data paths are:

```text
~/.codex
~/Library/Application Support/com.openai.codex
~/Library/Application Support/Codex
~/Library/Application Support/ChatGPT
```

Comparisons conservatively reserve ASCII case variants even on case-sensitive volumes. Live paths must still resolve to their registered locations. Profile manifests use schema version `1`; unknown fields are rejected. An old manifest may omit the registered build snapshot, but normal launch then requires explicit review/override.

The manifest's `name` is the stable registry identifier. Optional `display_name` stores user-facing Unicode text; missing values fall back to `name` for compatibility. New registrations generate lowercase identifiers, preserving the legacy ASCII syntax or using an ASCII prefix plus 64 random bits. Display text never becomes a path or bundle identifier. Both identifiers and display names reserve ASCII case variants; clone Bundle IDs use `com.openai.codex.harbor.<identifier>`. Prepared apps registered with `create`/`adopt` retain their existing Bundle IDs.

A profile records app/executable/ID, short version/build, two data paths, working directory, extra environment variable names and `adopted_data`. It stores no environment values. Codex/GUI paths cannot overlap each other or the app; other profiles cannot reuse the same app path/ID or conflicting data. `adopt` requires existing data outside the `profiles` registry; `create` allocates empty data for a prepared app. Neither signs or modifies that app.

New Harbor directories/files use private permissions (normally 0700/0600; executable shortcuts 0700); existing account trees are not recursively chmodded. Registration uses a private lock and no-overwrite manifest publication. `RegistryLock` explicitly unlocks on drop before closing the handle, so a concurrently inherited handle does not prolong the lock. An incomplete `create`/`adopt` setup is left for inspection rather than recursively removed.

## Clone and signature policy

[clone.rs](../crates/harbor-core/src/clone.rs) holds the registry lock throughout preparation and publication:

1. Validate the new name/destination and reject existing targets, source/registry/default-data overlap and existing-profile conflicts.
2. Verify the original `com.openai.codex` signature with an Apple anchor and OpenAI Team ID `2DC432GLL2` requirement.
3. Stage under the destination's parent; attempt `cp -cR`, fall back to `ditto`. Reverify vendor signature and unchanged version/build after copying.
4. Reject unsupported signing-path symlinks. Look for the desktop data/updater control strings in `app.asar`. Marker presence is a compatibility guard, not execution proof.
5. Change the outer Info.plist identity/display/product fields and `LSEnvironment` routing. Filter unsupported vendor-bound entitlements; unknown entitlements fail closed. Sign the outer copy ad-hoc with hardened runtime and the supported entitlements plus `disable-library-validation`.
6. Verify the local signature. Publish the new app, then the staged empty Profile with exclusive renames. If profile publication fails, attempt to remove only the newly published app; cleanup failure is reported.

Nested vendor code/signatures, ASAR and URL schemes are not rewritten. Quarantine and Gatekeeper are not disabled. The result does not preserve vendor identity or notarization. Application and registry publication are not one cross-filesystem transaction. An interruption may leave staging or an unregistered app; inspect the state before using `create` to register a prepared copy.

## Launch and versions

[process.rs](../crates/harbor-core/src/process.rs) locks the registry, revalidates paths and current identity/version, then compares current process command lines. Exactly one matching executable with exactly the expected GUI argument is reused. Other matching-executable arguments are a conflict. The comparison is not an authenticated identity or an account check.

For a new process, Harbor clears/rebuilds the environment, sets the working directory, redirects stdin to `/dev/null`, appends stdout/stderr to `launch.log`, and creates a new session with `setsid` in `pre_exec`. It checks for early exit for two seconds, including exit 0. Surviving this interval does not prove a working window, login or data isolation. No supervisor remains to monitor the client.

Identity or current invalid version/build fields always block launch. Changed short version/build or a missing old build snapshot blocks launch unless `--accept-version-change` is used for that call. The exception does not update the manifest, verify that a version is newer or migrate databases. `start` does not run signature verification; `doctor` does. `start --dry-run` only validates registered paths and reports routing, without full AppInfo/version/signature/runtime checks.

<a id="environment"></a>
## Environment

[environment.rs](../crates/harbor-core/src/environment.rs) keeps true HOME and explicitly sets:

```text
HOME=<real home>
PWD=<registered working directory>
CODEX_HOME=<profile codex directory>
CODEX_ELECTRON_USER_DATA_PATH=<profile gui directory>
CODEX_SPARKLE_ENABLED=false
argv: --user-data-dir=<profile gui directory>
```

The default working directory is HOME. The allowlist includes PATH, locale, common SSH/Docker/toolchain/proxy variables. If PATH is absent, a fixed common macOS tool path is used. API keys and arbitrary MCP tokens are not forwarded by default. Repeated `--pass-env NAME` at registration adds names whose current values are read at launch.

Overrides of HOME/PWD/OLDPWD, BASH_ENV/ENV/NODE_OPTIONS and `CODEX_`, `HARBOR_`, `DYLD_`, `ELECTRON_` prefixes are rejected. This does not stop the client, tools or a login shell from reading credentials/configuration in shared HOME. Proxy/Docker values may themselves be sensitive; the diagnostic output does not print those values. Client-generated logs can contain sensitive content.

Clone also patches `LSEnvironment` for Launch Services; direct executable launches receive explicit environment instead. Finder launching a clone bypasses Harbor checks and the allowlist. Adopt/create do not update LSEnvironment. The desktop-specific data/updater switches remain version-dependent, not stable API or a guaranteed updater block.

## Stop, removal and icons

[lifecycle.rs](../crates/harbor-core/src/lifecycle.rs) revalidates paths and app identity, requires the native helper and holds the registry lock. Normal main-app quit uses NSRunningApplication identity checks and a shared 15-second readiness/request/exit deadline. Up to five additional seconds handle residual helpers.

[auxiliary.rs](../crates/harbor-core/src/auxiliary.rs) scans executable paths beneath the app and both data directories. Known Crashpad, Computer Use and modifier-monitor orphans are eligible for SIGTERM only with the current user's UID, parent PID 1 and a rechecked executable/owner/parent/birth time. There is no SIGKILL fallback. These process-table and libproc checks reduce accidental targeting; they are not atomic protection against a malicious same-user process.

Removal requires no scanned processes, `adopted_data=false`, the Harbor-style ID and exact profile-owned data paths. It checks overlap with other instances and reserved directories. Default removal exclusively moves the Profile into a private `retained` directory before trashing the app. Explicit data removal trashes both app and Profile. Errors attempt no-overwrite restoration; interruption or rollback failure may require manual recovery. No code empties Trash. Moving a whole account directory is different from parsing/copying its credentials. See [recovery](GUI.md#removal-and-recovery).

[icon.rs](../crates/harbor-core/src/icon.rs) additionally requires the registered version/build and known icon layout, stages with `cp -cR`, signs/verifies and atomically swaps the app. It checks the main executable before staging/swap, not the broader auxiliary scan. The GUI requires JSON state `stopped`, so it also blocks known residual process states. Icon/removal eligibility is derived from manifest/path checks, not a separate proof of clone origin. Lifecycle stop/remove do not enforce version equality as icon/start do.

## Diagnostics and evidence boundaries

Text `status` and the process section of `doctor` inspect main processes only. JSON status/list can report `helpers_running`; their `pids` field still lists only main processes. None reads login identity or verifies a running process's environment. `doctor` fails on path/metadata/identity/signature errors, but version differences, absent old build snapshots and reported launch conflicts alone do not set the diagnostic failure flag. JSON doctor wraps a completed failed diagnosis with `ok=true`, exit 0 and `data.passed=false`; store setup errors still fail the envelope.

The system does not isolate HOME, Keychain, repositories, system permissions or all Skills/MCP storage. It does not implement OAuth callback routing, official updates, account/database migration or arbitrary client support. It cannot stop clients from following configuration paths outside routed data directories.

## Documentation audit, 2026-09-08

These differences were resolved in documentation, without changing program behavior:

| Earlier description | Current code and documentation |
|---|---|
| Product described mainly as a Rust launcher; GUI actions omitted | Swift GUI + CLI; stop/removal and native helper are included. |
| Program never sends kill signals | Supported orphan cleanup uses SIGTERM; main quit uses AppKit; no SIGKILL fallback. |
| All commands merely reference account data | Adoption references in place; removal may move entire directories to retained storage/Trash. |
| Dry run or doctor success implies launch readiness | Dry run is limited; doctor warnings can pass; launch enforces its own version gate. |
| Direct app launch has no profile routing | Clone has LSEnvironment, but direct launch bypasses Harbor validation/allowlist. |
| Text/JSON state and GUI/CLI icon guards are interchangeable | Main-only vs auxiliary-aware checks are documented separately. |
| Prepared/adopted apps are vendor-verified | Vendor trust verification belongs to clone; create/adopt inspect layout/metadata. |
| One current version and no Python requirement | Versions now share Cargo `0.0.1`; GUI build number remains `1`; packaging uses Python 3. |
| Old experiment paths and historical tests read as current prerequisites/results | Examples are generic; [Testing](TESTING.md) separates fresh checks and historical evidence. |

After the audit, the versions were unified: [Cargo.toml](../Cargo.toml) supplies `0.0.1`, and [packaging](../scripts/build_and_run.sh) reads the CLI version from Cargo metadata for the GUI. The independent macOS build number remains `1`. Documentation localization does not imply localized UI strings.

## GUI language state

The GUI owns one observable `GUILocalization` in `HarborStore`. `UserDefaults` stores the selected language. `GUIMessage` retains a template and separate arguments, so existing progress, success and GUI error messages can be rendered again when the language changes. Backend errors are explicitly verbatim. The compiled catalog and language picker live in `apps/Harbor/Sources/Harbor/Localization/GUILocalization.swift` and `apps/Harbor/Sources/Harbor/Views/LanguagePicker.swift`; they do not change CLI protocol or profile storage.
