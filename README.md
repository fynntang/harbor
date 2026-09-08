# Harbor

English | [简体中文](README.zh-CN.md)

Harbor is a macOS GUI and Rust CLI for creating and launching separate local ChatGPT/Codex desktop profiles. It routes each instance to its own Codex and GUI data directories. It is **not a security sandbox**: HOME, Keychain, SSH, Git, Docker, system permissions and repository access remain shared.

The adapter checks the tested Chromium/Codex app layout. It does not support every app named ChatGPT or arbitrary AI clients. Local copies are ad-hoc signed and do not retain the vendor identity or notarization. OAuth, Browser/Computer Use and complete account isolation are not guaranteed.

[Documentation](docs/README.md) · [GUI guide](docs/GUI.md) · [Implementation and limits](docs/ARCHITECTURE.md) · [Testing](docs/TESTING.md) · [References](docs/REFERENCES.md)

## Start with the GUI

Build requirements: macOS 14+, Rust 1.89+ with Cargo, Swift 6+ with macOS SDK/tools, Bash and Python 3. The packaging script uses Python 3; the packaged app does not require a separately installed Harbor CLI or Python interpreter.

Install the official app first, then run from the repository root:

```bash
./scripts/build_and_run.sh
```

Open `dist/Harbor.app`, choose **创建副本** (Create copy), enter `work`, select the official source app and create it. Click **启动** (Start), then sign in inside the new client window. Creating a copy does not start it or sign in automatically.

The GUI supports creation, start, stop, deletion, status refresh, diagnostics, instance icons and opening paths in Finder. Closing its window keeps Harbor's menu-bar entry; quitting Harbor leaves launched clients running. There is no login item or background daemon. Choose English or 简体中文 in the toolbar language picker, Harbor menu or menu-bar entry. Changes apply immediately and persist across launches. Backend diagnostics and system-owned dialogs retain their original/system language.

The app bundles both `harbor` and `harbor-native`. Keep the entire bundle together. Codex's Run action uses the same build script.

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

`adopt` records references in place; it does not copy credentials, move data or sign the app. Both data directories must exist and must not overlap the profile registry. Its metadata/logs live under `profiles/work/`; account data stays at the supplied paths. Old experiment directories are not prerequisites.

Use `create` for an already-prepared app that needs **new empty** data directories:

```bash
harbor create another --app "/absolute/path/ChatGPT-Another.app"
```

Unlike `clone`, neither `create` nor `adopt` prepares a bundle or verifies vendor trust. They inspect its supported layout, identity and version fields, and reject conflicting registrations/data paths. Each registered app needs a distinct path and Bundle ID. The default account directories are reserved.

## Commands

| Command | Current behavior |
|---|---|
| `clone` | Create a locally signed copy and new empty profile. |
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

`start` fixes `CODEX_HOME`, `CODEX_ELECTRON_USER_DATA_PATH`, `CODEX_SPARKLE_ENABLED=false` and one `--user-data-dir` argument. `--cwd` defaults to the user's HOME, not the calling repository. The environment uses an allowlist; extra variable **names** can be supplied through repeated `--pass-env` at registration. Values are read at launch and never stored in `profile.json`. Reserved/injection-sensitive keys are rejected. See [environment behavior](docs/ARCHITECTURE.md#environment).

`start --dry-run` validates registered paths and prints routing; it does not perform the full launch/version/signature checks. `doctor` reports version changes and missing old build snapshots as warnings, which alone do not fail the command. `start` still blocks those cases by default. After reviewing a version change, the CLI supports:

```bash
harbor start work --accept-version-change
```

This is a one-run exception, not a manifest update or database migration. It does not bypass invalid current version fields or changed app identity. The GUI does not expose this exception. Normal `start` does not run `codesign`; run `doctor` when signature inspection is needed.

Launch through Harbor to retain these checks. A clone also has `LSEnvironment` values for Launch Services, but double-clicking its `.app` bypasses Harbor's validation and allowlist. `create`/`adopt` do not patch those values.

Harbor does not manage official updates, arbitrate OAuth URL callbacks, isolate all Skills/MCP storage, prevent access to other projects, or rewrite client configuration overrides. The two desktop-specific `CODEX_*` switches are version-dependent compatibility controls. `remove` may move account directories as a unit; Harbor does not parse or copy credential contents.

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

The CLI, core package and GUI app version are `0.0.1`. GUI packaging reads the CLI package version from Cargo metadata; its independent macOS build number is `1`. The package declares MIT licensing. Local ad-hoc packaging is not a notarized distribution release. [Testing](docs/TESTING.md) separates current checks from historical client experiments and outstanding account/GUI validation. `SOURCE_CHECKS.txt` is a historical snapshot, not current acceptance evidence.

For Developer ID builds, see [release builds and signing](docs/GUI.md#release-build). Development builds remain ad-hoc signed; a signed release build still requires notarization before normal distribution.

The GUI can generate instance icons from the official icon with a colored name badge and monochrome menu-bar initials. Creation previews the badge; existing stopped copies can use **Change Icon → Original with Badge**.
