# Testing and evidence

English | [简体中文](zh-CN/TESTING.md) · [Documentation](README.md)

## Current documentation audit: 2026-09-08

The implementation was read before rewriting/translating the docs. This audit changed documentation only. The following checks were rerun on the current local working tree, not inferred from old records:

| Check | Result |
|---|---|
| `cargo metadata --locked --no-deps --format-version 1` | Packages `harbor-cli` / `harbor-core`, Cargo version `0.0.1`; CLI binary target remains `harbor`. |
| `sh scripts/check.sh` | Exit 0: formatting, 74 Rust tests, Clippy with warnings denied, release build. |
| Rust test breakdown | 4 JSON + 13 CLI root-guard + 40 core unit + 11 profile + 6 version tests; zero failures. |
| `swift test --package-path apps/Harbor --scratch-path target/swift-harbor` | 12 passed, zero failures. |
| Documentation validation | 12 pages / 6 language pairs; 122 local links/anchors; 32 fenced blocks; paired technical identifiers and 13 CLI commands checked. |
| Local toolchain | rustc/Cargo `1.97.1`, Apple Swift `6.3.2`, macOS arm64. |

Rust and native Swift tests use temporary fixtures and subprocesses, including temporary Trash/restore operations. They do not need real account files. Process inspection and signing/native tests require the relevant macOS tools and permissions. This run used the local environment outside the restricted execution sandbox.

Documentation validation checks local links/anchors, paired pages, matching command blocks and the actual CLI help. Remote CI, external reference URLs, fresh client cloning, real account login, GUI clicks and Dock rendering were not revalidated during this documentation-only audit. That audit found independently maintained GUI/Cargo versions; the subsequent version-unification check is recorded below.

## Reproduce checks

From the repository root:

```bash
sh scripts/check.sh
swift test --package-path apps/Harbor --scratch-path target/swift-harbor
./scripts/build_and_run.sh --verify
cargo run --locked -p harbor-cli -- --help
```

The Rust script checks formatting without rewriting files, uses `--locked` for tests/Clippy/release, and exits on failure. Keep Cargo.lock with the source. `--verify` also rebuilds/restarts this checkout's Harbor GUI; it checks process survival, not visible interaction. [CI](../.github/workflows/ci.yml) runs the Rust script on macOS and Ubuntu and Swift tests/packaging on macOS. No remote CI result is asserted here. Platform-specific tests are gated, so Linux test counts need not equal the macOS count.

## Coverage and limits

| Area | Automated coverage |
|---|---|
| Storage | Names, overlapping paths/default-account guards, aliases/case variants, no-overwrite registration, lock release, symlink logs and log tails. |
| Environment/launch | Allowlist and reserved keys, exact command matching, detached session/stdio, early exit, short-version/build gates and unchanged manifests. |
| Clone | New empty profiles, conflicts, preparation failure, exclusive publication races, entitlement filtering and unsigned source rejection. |
| Icons | Synthetic PNG validation, resource sizes, local signing, unchanged data/metadata, running main-process refusal and unsafe resource layouts. |
| Lifecycle | Keep-data removal, explicit data removal, rollback, running/adopted/rerouted-data refusal, known helper paths, active child preservation and orphan exit. |
| Swift | JSON/exit/protocol validation, literal arguments, draining both pipes, state flow/retry, quit refusal preventing removal, image conversion, wrong native identity and Trash rollback. |

Unit/mock tests are not proof of all app capabilities. They use generated plists/scripts or copied system programs, not bundled vendor apps or credentials. Source inspection also found differences between main-only text status and auxiliary-aware JSON status, and between CLI and GUI icon guards; those are documented in [Architecture](ARCHITECTURE.md).

## Historical local experiments: 2026-09-08

The following summarizes earlier development records. These are historical observations, not reruns on the final documentation tree, a release certification or an assertion about current installed instances.

| Experiment | Recorded result and boundary |
|---|---|
| Lock regression | An inherited/duplicated handle could delay release. Explicit unlock on drop and a regression test were added. The then-current 15 core tests passed 100 consecutive runs (1,500 executions); this is not 100 runs of today's 74-test suite. |
| Fresh official copy | Source `/Applications/ChatGPT.app`, version `26.901.51231`, build `8109`; temporary clone passed source/copy/local-signature checks, had empty data, reached a visible sign-in page and rejected duplicate launch. Source/existing-copy executable and plist hashes were unchanged. No account login or OAuth test. |
| Persistent profile move | A temporary `work` profile was moved to persistent storage with explicit user authorization; codex/gui inode identities were retained and patched app routing/signature checked. This was a one-off maintenance operation, not a shipping migration command. |
| Instance icons | A real temporary copy accepted Dock/menu resources; signature/doctor passed, ASAR and manifest stayed unchanged, data stayed empty. It was not launched for that icon test. Synthetic icon tests also ran. |
| Stop/remove | Real temporary copies exposed registration timing and orphan-helper issues, which were fixed. Final trials stopped immediately after launch twice, rejected running removal, retained synthetic data by default, and verified explicit data Trash/removal/restoration. Temporary files were cleaned. No automatic deletion test targeted a real account. |
| GUI interaction | Computer Use connections repeatedly failed with `native pipe closed`; some later reads captured the deletion sheet/errors and the final empty list. Complete click/close/reopen acceptance was not established. An empty list then is not a statement about today's registry. |
| Harbor icon | Ten ICNS sizes, resource identity, signatures and GUI startup were checked. The blue tile was later replaced with a transparent white sailboat outline; alpha and outline colors were checked. Final Dock cache rendering was not directly verified. |
| Package/scripts cleanup | CLI package/directory became `harbor-cli`, binary stayed `harbor`; scripts were consolidated under `scripts`, including Run/CI/docs references. Relevant build/check steps passed at those stages. Earlier `harbor 0.1.0` output is historical, not the current Cargo version. |

`SOURCE_CHECKS.txt` remains the original source-generation snapshot, including its older package names and NOT RUN entries. Earlier Python-launch and supplied plist/window observations informed the adapter; they are not acceptance results for the current Rust implementation.

## Manual acceptance still needed

Use an explicitly disposable profile for destructive/recovery checks, and have the user perform actual sign-in. Keep production account data out of automated fixtures.

| Action | Observe |
|---|---|
| Fresh clone | New bundle and empty data; original and existing profiles unaffected. |
| First start and duplicate start | Correct sign-in window; no second instance from repeated start. |
| Close terminal / Harbor | Client remains independent. |
| Stop or cancel quit | Correct client exits, or refusal/timeout preserves the profile and prevents deletion. |
| Remove with/without data | Scope matches confirmation, exact retained/Trash paths are recoverable. |
| Icon and window/menu lifecycle | Dock/Finder/menu appearance, restart/cache behavior, close/reopen and busy-state controls. |
| Separate accounts across restarts | Intended accounts and conversations persist in each instance. |
| OAuth / Browser / Computer Use | Correct instance routing and actual feature availability. |
| Skills / MCP / tools | Record actual shared/instance sources, PATH, SSH and Docker behavior. |
| Version changes | Review warnings, startup blocks, one-shot override and database compatibility. |

Do not publish complete client logs or copy real credentials into tests. `doctor` checks neither all configuration overrides nor account identity, and may report version/conflict warnings without failing. For rollback of an adopted profile, continue using a previously verified launcher with the same paths; adoption itself did not migrate databases or re-sign the app. Never delete account databases, Singleton files or registry locks as a shortcut to recovery. For removal recovery, follow the [current procedure](GUI.md#removal-and-recovery).

## Version unification: 2026-09-08

CLI, core and GUI versions now use `0.0.1`; packaging reads the CLI version from Cargo metadata instead of a second hard-coded version. macOS build number remains `1`. Old version text in an application-layout error and comments was removed.

Formatting/shell syntax checks, 11 profile tests and 6 version tests passed. Packaging, local signature verification and GUI startup passed; the built app's Info.plist and bundled `harbor --version` were checked against Cargo metadata and all report `0.0.1`. The full Rust/Swift suites were not rerun for this version-only change.

## GUI language switching (2026-09-08)

- Full Swift suite: **16 tests passed**, including 4 localization tests for default language resolution, saved preferences, live message updates, argument preservation and unchanged backend diagnostics.
- Catalog audit: 114 Chinese GUI string occurrences have English translations; process and progress states and translation argument counts are covered.
- Packaged `Harbor.app` build, signature verification and process launch passed. UI checks covered immediate English/简体中文 switching, the English creation form, preserved form input and saved English selection after rebuilding/relaunching. No client instance was created or removed for these checks.
- Rust logic did not change; the prior Rust results above are historical for this GUI change. CLI/backend diagnostics and system-owned text are outside GUI translation coverage.
- Strict Swift lint still reports 11 pre-existing snake_case JSON field names in `Profile.swift`; these protocol model names were preserved. This is not a passing full-lint claim.
