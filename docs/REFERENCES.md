# References and provenance

English | [简体中文](zh-CN/REFERENCES.md) · [Documentation](README.md)

These documents and projects informed Harbor’s implementation. See [Architecture](ARCHITECTURE.md) for Harbor’s behavior.

## Primary documentation

| Reference | Topic |
|---|---|
| [Codex environment variables](https://developers.openai.com/codex/config-file/environment-variables) | CODEX_HOME and environment/configuration boundaries. |
| [Codex Skills](https://developers.openai.com/codex/skills/) | Multiple Skills loading locations. Routing CODEX_HOME is not a claim of complete Skills isolation. |
| [Rust CommandExt](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html) | Unix pre_exec and its execution constraints. |
| [Rust File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) | File locking APIs; the repository declares Rust 1.89 as its minimum. |
| [Apple flock manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html) | Shared lock handles, duplication and process inheritance. |
| [Apple TN2206](https://developer.apple.com/library/archive/technotes/tn2206/_index.html) | Code signing, nested code and verification. |
| [Disable Library Validation entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.disable-library-validation) | The entitlement used when locally signing a copy that keeps vendor-signed frameworks. |
| [Launch Services keys](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/LaunchServicesKeys.html) | LSEnvironment and Launch Services launches. |
| [NSRunningApplication](https://developer.apple.com/documentation/appkit/nsrunningapplication) | Application identity, readiness, termination and normal quit requests. |
| [FileManager](https://developer.apple.com/documentation/foundation/filemanager) | Native Trash and file-move operations. |

The local macOS SDK headers also informed native quit handling. Harbor reads application code markers for `CODEX_ELECTRON_USER_DATA_PATH` and `CODEX_SPARKLE_ENABLED`; marker checks and previous experiments do not make these stable public APIs.

## Third-party design input

- [Doppel repository](https://github.com/thomast8/doppel): historical reference for profile-based desktop routing and updater controls.
- [Doppel engine](https://github.com/thomast8/doppel/blob/main/engine/doppel-engine.zsh): a mutable branch link, not a pinned compatibility specification.

Harbor draws on Doppel’s design and uses its own Rust/Swift implementation. It does not bundle Doppel’s engine or vendor apps, modify ASAR, or route OAuth callbacks. Local signing filters vendor-bound entitlements and preserves nested framework signatures; see [clone.rs](../crates/harbor-core/src/clone.rs).

## Local evidence and assets

User-supplied plist fields, process/window observations and earlier Python-launch experiments informed the adapter selection. They are not portable compatibility proofs. [Testing](TESTING.md) separates historical client trials from current local checks. Tests use synthetic data and temporary executables; account files and authentication material are not fixtures.

Harbor's own icon is generated from Apple's `sailboat` system symbol by [generate_app_icon.swift](../scripts/generate_app_icon.swift), matching the menu-bar graphic. It is distinct from the synthetic PNG fixture used by core icon tests. No user screenshot is used as an account-test fixture.
