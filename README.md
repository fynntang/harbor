# Harbor

English | [简体中文](README.zh-CN.md)

Harbor creates and manages ChatGPT/Codex desktop copies with separate data directories for different accounts. It provides a macOS app and a Rust CLI.

[Documentation](docs/README.md) · [GUI guide](docs/GUI.md) · [Implementation and limits](docs/ARCHITECTURE.md) · [Testing](docs/TESTING.md) · [References](docs/REFERENCES.md)

<a href="https://www.nxgntools.com/tools/harbor-1?utm_source=harbor-1" target="_blank" rel="noopener" style="display: inline-block; width: auto;">
    <img src="https://www.nxgntools.com/api/embed/harbor-1?type=FEATURED_ON" alt="Featured on NxGn Tools" style="height: 48px; width: auto;" />
</a>

## Platforms and installers

Harbor supports macOS 14+ on Apple Silicon and Intel Macs. Release builds produce a Universal app, then package separate installers:

| Mac type | DMG filename |
|---|---|
| Apple Silicon (M series) | `Harbor-<version>-aarch64-apple-darwin.dmg` |
| Intel (x64 / x86_64) | `Harbor-<version>-x86_64-apple-darwin.dmg` |

Both architectures use `Harbor-<version>-macos-universal.zip` for automatic updates. Intel copies require an official client with Intel binaries; Harbor does not convert ARM clients to x86. Validation includes Intel tests under Rosetta, but not physical Intel hardware or the official Intel client. See [testing evidence](docs/TESTING.md).

## Start with the GUI

1. Download the DMG for your Mac from [GitHub Releases](https://github.com/fynntang/harbor/releases), open it and drag Harbor.app into Applications.
2. Install the official ChatGPT/Codex client.
3. Open Harbor, choose **Create copy**, enter a name such as `work` and select the official app.
4. Click **Start**, then sign in inside the copy's window.

Harbor is ad-hoc signed and is not notarized by Apple. If macOS blocks developer verification, see [first-launch instructions](docs/GUI.md#first-launch). The app includes its CLI and native helper; you do not need to install Rust or Python.

The GUI can create, update, start, stop and delete copies, inspect their status and change icons. Icons can use the official image with a colored name badge. After the local official client updates, Harbor flags copies that can be updated. Harbor also supports [automatic update checks](docs/GUI.md#harbor-automatic-updates), with confirmation before installation.

Closing the window keeps Harbor running in the menu bar; quitting Harbor leaves client copies running. Choose English or 简体中文 in the toolbar or menus; the choice is saved. Backend diagnostics and system dialogs retain their original or system language.

### Run from source

You need macOS 14+, Rust 1.89+ with Cargo, Swift 6+ with the macOS SDK, Bash and Python 3. Run from the repository root:

```bash
./scripts/build_and_run.sh
```

The script builds and opens `dist/Harbor.app`. Codex's Run action uses the same script. Keep the complete `.app` directory when moving it.

## Start with the CLI

For a machine with an official app at `/Applications/ChatGPT.app`:

```bash
sh scripts/check.sh
cargo install --path crates/harbor-cli --locked --force
export PATH="$HOME/.cargo/bin:$PATH"

harbor clone work --source /Applications/ChatGPT.app
harbor start work --dry-run
harbor doctor work
harbor start work
harbor status work
```

`clone` verifies the source's `com.openai.codex` identity and OpenAI Team ID `2DC432GLL2`, prepares a local copy and creates empty data directories. It refuses existing names and destinations. A custom destination must be a new absolute `.app` path, supplied with `--app`.

Default locations:

```text
~/Applications/Harbor/ChatGPT-work.app
~/Library/Application Support/Harbor/
├── registry.lock
├── profiles/
│   └── work/
│       ├── profile.json
│       ├── codex/
│       ├── gui/
│       └── launch.log
└── retained/                 # created when removal keeps data
```

The copy uses Bundle ID `com.openai.codex.harbor.work`. `launch.log` appears when launched. `--root` changes the registry/data root, not the default application destination. Display names contain 1–48 Unicode characters, including uppercase/lowercase letters, Chinese, spaces and parentheses; control characters and surrounding whitespace are rejected. Harbor generates a separate stable lowercase identifier: `Toobit` becomes `toobit`; other names use an ASCII prefix (or `profile`) plus a random suffix. The identifier is used in the Bundle ID, application filename and data directories. Display names that differ only in ASCII letter case cannot coexist. `clone`/`create`/`adopt` take the display name; subsequent CLI commands take the identifier reported by `list`/creation (quote names containing spaces). Existing registrations keep their identities and data paths.

## Register an existing copy

Save your work and quit the selected client before adopting its data. Replace these example paths with your actual prepared app and existing directories:

```bash
harbor adopt work \
  --app "/absolute/path/ChatGPT-Work.app" \
  --codex-home "/absolute/path/existing-codex" \
  --gui-home "/absolute/path/existing-gui"
```

`adopt` records references in place; it does not copy credentials, move data or sign the app. Both data directories must exist and must not overlap the profile registry. Its metadata/logs live under `profiles/work/`; account data stays at the supplied paths.

Use `create` for an already-prepared app that needs **new empty** data directories:

```bash
harbor create another --app "/absolute/path/ChatGPT-Another.app"
```

Unlike `clone`, neither `create` nor `adopt` prepares a bundle or verifies vendor trust. They inspect its supported layout, identity and version fields, and reject conflicting registrations/data paths. Each registered app needs a distinct path and Bundle ID. The default account directories are reserved.

## Commands

| Command | Current behavior |
|---|---|
| `clone` | Create a locally signed copy and new empty profile. |
| `update` | Update a stopped managed copy from the official app; retain identity, icons and account paths. |
| `create` / `adopt` | Register a prepared app with new / existing data. |
| `list` / `show` | List registrations / show routing metadata. |
| `start` | Validate identity/version, route data and launch detached; detect duplicate launches. |
| `status` | Inspect main-process command lines; JSON also reports residual helpers. |
| `doctor` | Inspect paths, identity, version warnings and local signature; does not repair anything. |
| `stop` | Request normal quit and handle supported orphan helpers; requires `harbor-native`. |
| `remove` | Trash a stopped managed copy; keep account data by default; requires `harbor-native` and `--yes`. |
| `icon` | Replace supported local-copy icons while stopped, then sign and verify. |
| `logs` | Read the tail of the client's launch log; content may be sensitive. |
| `shortcut` | Create a new `.command` file that runs the current CLI at its fixed path. |

Common invocations:

```bash
harbor list
harbor show work
harbor update work --source /Applications/ChatGPT.app
harbor logs work -n 80
harbor shortcut work --output "$HOME/Desktop/ChatGPT Work.command"
harbor icon work --image /absolute/path/icon.png --tray-image /absolute/path/tray.png
```

CLI icon inputs must be square PNGs, at most 4096 pixels per side and 16 MiB. `--tray-image` is optional. The GUI also converts PNG/JPEG/ICNS inputs and offers a letter template. See [icons and lifecycle operations](docs/GUI.md).

A Rust-only `cargo install` does **not** install `harbor-native`. Use the GUI or its bundled CLI for stop/remove:

```bash
dist/Harbor.app/Contents/Helpers/harbor stop work
dist/Harbor.app/Contents/Helpers/harbor remove work --yes
```

Review the scope first. Add `--delete-data` only to also move the whole profile directory to Trash. Otherwise it is retained under `<root>/retained/<name>-<random>/profile`, and the returned path is shown in the GUI. CLI `remove` refuses running processes; GUI deletion calls `stop` first, even if the last status appeared stopped. Quit refusal or timeout prevents removal. There is no permanent-delete or GUI restore command; [recovery details](docs/GUI.md#removal-and-recovery) explain the retained manifest's original paths.

## Routing, versions and limits

Harbor separates Codex and GUI data directories; it is not a security sandbox. HOME, Keychain, SSH, Git, Docker, system permissions and repository access remain shared.

The adapter checks the tested Chromium/Codex app layout. It does not support every app named ChatGPT or arbitrary AI clients. Local copies are ad-hoc signed and do not retain the vendor identity or notarization. OAuth, Browser/Computer Use and complete account isolation are not guaranteed.

`start` fixes `CODEX_HOME`, `CODEX_ELECTRON_USER_DATA_PATH`, `CODEX_SPARKLE_ENABLED=false` and one `--user-data-dir` argument. `--cwd` defaults to the user's HOME, not the calling repository. The environment uses an allowlist; extra variable **names** can be supplied through repeated `--pass-env` at registration. Values are read at launch and never stored in `profile.json`. Reserved/injection-sensitive keys are rejected. See [environment behavior](docs/ARCHITECTURE.md#environment).

`start --dry-run` validates registered paths and prints routing; it does not perform the full launch/version/signature checks. `doctor` reports version changes and missing old build snapshots as warnings, which alone do not fail the command. `start` still blocks those cases by default. After reviewing a version change, the CLI supports:

```bash
harbor start work --accept-version-change
```

This is a one-run exception, not a manifest update or database migration. It does not bypass invalid current version fields or changed app identity. The GUI does not expose this exception. Normal `start` does not run `codesign`; run `doctor` when signature inspection is needed.

Launch through Harbor to retain these checks. A clone also has `LSEnvironment` values for Launch Services, but double-clicking its `.app` bypasses Harbor's validation and allowlist. `create`/`adopt` do not patch those values.

Harbor does not automatically download official client updates, arbitrate OAuth URL callbacks, isolate all Skills/MCP storage, prevent access to other projects, or rewrite client configuration overrides. The two desktop-specific `CODEX_*` switches are version-dependent compatibility controls. `remove` may move account directories as a unit; Harbor does not parse or copy credential contents.

## Next milestone

The next milestone is **Windows support**, starting with x64 and covering instance management, a GUI, system tray controls, installation and updates. First verify that the official client can reliably use separate data directories, then adapt the core and GUI. Windows version requirements and ARM64 support remain to be assessed.

Harbor currently supports macOS; Windows is planned. See the [roadmap](docs/ROADMAP.md) for steps and completion criteria.

## Development and documentation

| Path | Responsibility |
|---|---|
| `apps/Harbor/` | SwiftUI GUI, native helper and icon assets. |
| `crates/harbor-cli/` | Cargo package `harbor-cli`; executable `harbor`; text/JSON interface. |
| `crates/harbor-core/` | Registration, cloning, routing, icons and lifecycle logic. |
| `scripts/check.sh` | Rust formatting, tests, Clippy and release build. |
| `scripts/build_and_run.sh` | GUI build, bundle signing and launch. |
| `scripts/generate_app_icon.swift` | Generate the transparent sailboat icon. |
| `docs/` / `docs/zh-CN/` | English / Simplified Chinese documentation. |

```bash
cargo run -p harbor-cli -- --help
swift test --package-path apps/Harbor --scratch-path target/swift-harbor
```

Harbor is MIT licensed and publishes updates through GitHub Releases with `vYY.M.DHHmm` tags. [Cargo.toml](Cargo.toml) defines the version used by the GUI, CLI, build metadata and asset names. See the [release guide](docs/GUI.md#release-build).

[Testing](docs/TESTING.md) records dated results and checks still needed. `SOURCE_CHECKS.txt` is an early development snapshot.
