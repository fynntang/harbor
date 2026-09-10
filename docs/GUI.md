# macOS GUI

English | [简体中文](zh-CN/GUI.md) · [Documentation](README.md) · [Project](../README.md)

## Build and run

Requirements: macOS 14+, Rust 1.89+/Cargo, Swift 6+/macOS SDK and tools, Bash and Python 3. SwiftPM has no third-party package dependencies.

Run from the repository root:

```bash
./scripts/build_and_run.sh
./scripts/build_and_run.sh --build-only
./scripts/build_and_run.sh --verify
swift test --package-path apps/Harbor --scratch-path target/swift-harbor
```

The result is `dist/Harbor.app`, containing a debug Swift GUI/native helper and release Rust CLI. It is locally ad-hoc signed, not Developer ID signed or notarized. Copy the complete app bundle if moving it. `cargo install` alone does not provide the native helper required by stop/remove.

Every build mode, including `--build-only`, checks for operations using this checkout's bundled helpers, refuses rebuilding while they run, and stops this checkout's existing Harbor GUI. It does not stop client instances. `--verify` launches Harbor and checks that its exact executable remains running after one second; it is not a UI or account test. `--debug` uses LLDB; `--logs` and `--telemetry` open system log streams and do not guarantee that every action emits log events.

Codex's Run action points to this script. An isolated registry can be selected with:

```bash
./scripts/build_and_run.sh --verify --registry /absolute/path/test-registry
```

A mode is required before `--registry`. The GUI shows the registry path and warns for `/tmp` or `/private/tmp`. The default clone destination remains `~/Applications/Harbor/ChatGPT-<identifier>.app`; changing the registry alone does not redirect application copies. Temporary data is unsuitable for persistent accounts; there is no automatic migration button.

## Controls and state

Use the toolbar language picker, **Harbor → Language** or the menu-bar **Language** submenu to choose **English** or **简体中文**. Custom windows, forms, menus, status, progress and GUI error messages update immediately; form input and operation state are preserved. The choice is saved in the app’s `UserDefaults` under `HarborGUILanguage`. On first launch, the first supported system language is used (Chinese maps to Simplified Chinese); otherwise English is used. This setting affects only Harbor, not client copies or system language.

Backend diagnostics, CLI help, technical reports and system-owned menu/dialog text retain their original/system language. The bilingual catalog is compiled into the GUI executable; no separate localization resource bundle is needed.

| Control | Behavior |
|---|---|
| 创建副本 / Create copy | Choose an official source and a new name; calls `clone`. No GUI `create`/`adopt` form. |
| 更新副本 / Update Copy | Update a stopped managed app from the selected official source, preserving identity, icons and account paths. |
| 启动 / Start | Calls `start`; disables when the inspected app has an issue or launch conflict. |
| 停止 / Stop | Available for `running` or `helpers_running`; requests normal quit. |
| 删除实例… / Delete instance | Shows scope confirmation for eligible managed copies. Always calls `stop` before `remove`. |
| 检查 / Check | Shows `doctor` results, including warnings. |
| 更换图标… / Change icon | Available for eligible copies with no issue and state `stopped`. |
| 刷新 / Refresh | Reloads registrations/state; also runs on window activation and ⌘R. |
| Folder controls | Reveal paths in Finder. The log-directory button currently reveals the parent of `codex_home`; for adopted data that may differ from Harbor's log directory. |

Mutating operations and refresh are serialized by the store's busy state. Errors are shown without assuming success. App identity or version differences are flagged as needing inspection. The GUI has no `--accept-version-change`, environment-variable editor or restore button. An invalid registry entry can fail the entire list operation; this is not a repair UI.

The top sailboat menu opens Harbor, creates a copy or quits Harbor. The main window uses a stable value to reuse it. Closing all windows retains the menu item; normal quit is blocked while an operation is busy. Launched clients remain independent. There is no login item, launchd daemon or periodic polling.

## Icons

Harbor's own Dock/Finder icon uses the same `sailboat` SF Symbol as its menu bar: white outline on a transparent background. The menu-bar template follows system appearance; the bundled Dock image is fixed white. `scripts/generate_app_icon.swift` produces the iconset/PNG. Packaging copies `Resources/Harbor.icns`, sets `CFBundleIconFile`, and the app explicitly loads it at startup.

For a client instance, choose PNG, JPEG or ICNS (up to 16 MiB) in **更换图标…**. The GUI fits the image into a transparent 1024-pixel square. The menu-bar image can be the instance's first letter or an image silhouette. Template colors are ignored; an opaque photo produces an opaque silhouette. The client must enable its own menu-bar entry for that icon to appear; Harbor does not toggle that preference.

The CLI accepts square PNGs up to 4096 pixels per side / 16 MiB and an optional `--tray-image`. Eligibility is `adopted_data=false`, the Harbor-style Bundle ID, matching identity/version and the expected resource/signature layout. A prepared app registered with `create` can meet those checks; there is no separate cryptographic record proving it was made by `clone`.

The GUI blocks icon changes while residual helpers are reported. The core `icon` command checks only for the matching main executable, before staging and before swapping; it does not use the broader stop/remove helper scan. Prefer **停止** before changing icons. Icon staging uses `cp -cR` without the clone command's `ditto` fallback.

The operation replaces supported Dock/Finder resources and optional 18/36-pixel menu templates, re-signs the outer copy, verifies it, and swaps it atomically. A failed final verification attempts rollback; a failed rollback retains the old app's staging path. It does not alter ASAR, nested programs or account directories. Restart the client to use the new resources. There is no built-in restore-original-icon action.

<a id="removal-and-recovery"></a>
## Removal and recovery

The native helper has a 15-second budget for macOS registration/readiness, sending a normal quit request and observing exit. Rust then waits up to five seconds for helpers under the app, `codex_home` and `gui_home`. Known orphan Crashpad, Computer Use and modifier-monitor executables may receive SIGTERM after owner, parent and birth-time checks. Instance-scoped Chrome native hosts and their plugin-server children can also receive SIGTERM to disconnect that instance without quitting its browser. Unknown or unrelated active helpers prevent completion; there is no SIGKILL fallback. Failure or timeout stops GUI deletion.

Deletion defaults to retaining the entire Profile directory under `<root>/retained/<identifier>-<random>/profile` and moving only the app to system Trash. The registration disappears from the list and the retained path is shown. Selecting **同时将账号数据移到废纸篓** also trashes the whole Profile directory, including metadata, logs and data. Harbor never empties Trash. Removal and error notices have a dismiss button; dismissal only clears the notice and does not delete files. If a retained directory is removed outside Harbor, refreshing or returning to the window clears its stale removal notice. These notices are kept only in memory and also disappear when Harbor quits.

Deletion requires `adopted_data=false`, the expected Harbor Bundle ID and data paths exactly equal to the profile's own `codex`/`gui`. It refuses running processes, default-data overlap, registry overlap and overlap with other instances. Current paths (including the working directory) and app identity must still validate; a missing app/directory can prevent removal. It is not a general broken-registration cleanup tool. A version difference alone does not block lifecycle actions as it blocks start/icon.

CLI example, from the repository root after packaging:

```bash
dist/Harbor.app/Contents/Helpers/harbor stop work
dist/Harbor.app/Contents/Helpers/harbor remove work --yes
```

`--delete-data` is optional and must be chosen deliberately. CLI `remove` does not stop anything itself. The native helper attempts to restore earlier trashed items if a later item fails, without overwriting existing paths. Default-retention failure also attempts to restore the Profile directory. Multiple paths are not one atomic transaction; interruption or rollback failure can require manual recovery from `retained` or Trash.

Retained `profile.json` still records the original paths. To restore, first quit relevant clients and restore the app from Trash. If the original registration path is vacant and has not been reused, the entire retained Profile can be moved back to its original location. Otherwise use `adopt` with a free name and the actual retained `codex`/`gui` paths, inspect routing and run `doctor` before starting through Harbor. Adoption does not patch stale `LSEnvironment`, so direct app launch remains inappropriate after moving data. An adopted profile is not eligible for GUI deletion/icon changes. If any path is occupied or the scope is uncertain, inspect it rather than overwrite it. There is no automatic recovery workflow.

## Bridge and JSON protocol

[HarborClient](../apps/Harbor/Sources/Harbor/Services/HarborClient.swift) invokes `Contents/Helpers/harbor` with separate argv entries, without a shell. stdout/stderr are drained concurrently off the UI thread. [HarborStore](../apps/Harbor/Sources/Harbor/Stores/HarborStore.swift) owns operation order; Rust remains responsible for filesystem and process checks. [HarborNative](../apps/Harbor/Sources/HarborNative/NativeOperations.swift) handles normal application quit and Trash.

`--json` supports `list`, `show`, `status`, `clone`, `update`, `icon`, `start`, `stop`, `remove`, `doctor`. It does not support `create`, `adopt`, `logs` or `shortcut`. [json.rs](../crates/harbor-cli/src/json.rs) defines the protocol:

- stdout: one object with `api_version: 1`, `ok`, and either `data` or `error`.
- Ordinary operation error: exit 1. Clap syntax errors: stderr and exit 2, not a JSON envelope.
- `doctor`: when store setup succeeds, even a failed diagnosis is a successful envelope/exit 0 with `data.passed=false`; inspect that field. Version/conflict warnings alone need not set it false.
- Clone/update progress: stderr JSON lines containing `event: progress` and `stage`.
- JSON status: `running`, `stopped`, `helpers_running`, `conflict`; list may report `unknown` on a process-inspection error. `pids` contains main-process IDs only, even for `helpers_running`.

Finder-launched Harbor does not source terminal shell configuration. Environment values exported only in a terminal will not automatically be available to its child instances. See [routing and environment](ARCHITECTURE.md#environment).

<a id="release-build"></a>
## Release builds and signing

Development commands above still produce `dist/Harbor.app` with a debug Swift GUI/native helper, a release Rust CLI and ad-hoc signatures. The separate release command compiles all three programs in release mode and produces `dist/release/Harbor.app`; it neither stops nor replaces the development app. The first release supports macOS 14+ on arm64 only.

For free preview distribution without a Developer ID certificate, use the explicit ad-hoc release mode. It compiles all three programs in release mode, checks their signatures, identifiers, architectures and versions, and outputs `dist/release-adhoc/Harbor.app`. It does not query signing identities, stop the GUI or replace the Developer ID output. It is **not notarized** and does not identify a trusted publisher.

```bash
./scripts/build_and_run.sh --release-adhoc
mkdir -p dist/artifacts
ditto -c -k --sequesterRsrc --keepParent dist/release-adhoc/Harbor.app dist/artifacts/Harbor-0.0.1-macos-arm64.zip
python3 scripts/package_dmg.py dist/release-adhoc/Harbor.app
```

Download the DMG and checksum from the project's GitHub Release. Verify the DMG checksum, open it and drag Harbor.app into the Applications shortcut, then eject the disk and launch Harbor from Applications. Quit Harbor before replacing an installed version. ZIP remains available as an alternative. The packaging script builds a compressed read-only DMG, mounts it to verify signatures and compare every app file with the input release, then updates `SHA256SUMS.txt` for the ZIP (if present) and DMG. It neither rebuilds nor re-signs the app. If you downloaded only one format, compare its hash with the corresponding line in the checksum file. Install the official ChatGPT/Codex app separately. If macOS blocks first launch because the publisher cannot be verified, follow [Apple's instructions](https://support.apple.com/en-us/102445) only after confirming the download's source. No global Gatekeeper changes are required. Installation on another Mac has not yet been verified.

For Developer ID signing, follow the separate path below.

Install a valid **Developer ID Application** certificate and its matching private key in the build machine’s keychain. Set `HARBOR_SIGNING_IDENTITY` to that certificate’s 40-character SHA-1 fingerprint; the fingerprint is an identifier, not a private key. Never put the private key or its password in source, command examples or release assets.

```bash
HARBOR_SIGNING_IDENTITY=YOUR_CERTIFICATE_SHA1 ./scripts/build_and_run.sh --release-only
```

Replace the placeholder with your certificate fingerprint. Missing, malformed, unavailable or wrong-type identities stop the command before any build or app replacement. There is no ad-hoc fallback. The helpers use stable signing identifiers `local.harbor.desktop.cli` and `local.harbor.desktop.native`; the GUI keeps `local.harbor.desktop` so its existing preferences remain accessible.

The script signs the helpers first, then Harbor.app, using the selected identity, Hardened Runtime and a secure timestamp. Before publishing the local output, `scripts/signing.py` verifies the Apple signing anchor, signatures, signing team, identifiers, runtime flags, timestamps, matching arm64 architectures and matching GUI/CLI/workspace versions. A failed check prevents replacing the previous release app. No additional entitlements are added. These signing settings apply only to Harbor and its own helpers; user-created client copies retain their existing local signing flow and never receive the publisher’s certificate/private key.

**This command creates a signed app, not a notarized distribution.** Apple notarization, ticket stapling, ZIP/checksum generation, installation testing on another Mac and GitHub Release publishing remain subsequent steps. Do not label this intermediate output as notarized or Gatekeeper-accepted. Release signing is not performed by the current CI: CI tests the gates and builds/verifies the ad-hoc release without accessing signing credentials.

## Name badges on instance icons

The creation form enables **Use a Name Badge** by default. Preview the official icon with a colored badge at large and small sizes, plus the monochrome menu-bar initials. Leave badge text empty for an automatic abbreviation (`work` → `WORK`, `toobit` → `TOO`); names up to four characters are kept, longer names use the first three. A custom label may contain at most four characters and no control characters. Choose one of six colors or use the stable name-based automatic choice. Orange/yellow badges use black text; other colors use white. The menu-bar icon uses up to the first two badge characters on a transparent background.

For an existing stopped instance, choose **Change Icon… → Original with Badge**. **Custom Image** retains the previous image-upload workflow. Applied badge settings are saved locally by app path; cancelling an edit does not save it.

New clones preserve `Contents/Resources/harbor-original-icon.png` before their local signature is created. Subsequent icon changes keep that file intact and always render from it. For older copies without that snapshot, the GUI reads `/Applications/ChatGPT.app/Contents/Resources/icon-chatgpt.png`, never the copy’s already-customized icon. If the original cannot be read, badge mode is unavailable; creation can disable the badge or select the correct official source, and existing copies can use Custom Image.

GUI creation performs `clone` followed by `icon`; these are separate operations. If icon application fails after cloning, the created instance remains in the list, the creation sheet closes, and an error is shown. Retry through Change Icon rather than cloning again. The CLI preserves the original image when cloning but does not generate badges automatically. No account data is modified by a badge change.

## Display names and identifiers

The creation form accepts 1–48 Unicode characters, including Chinese, uppercase/lowercase letters, spaces and parentheses, without control characters or surrounding whitespace. The full display name appears in the list, details, confirmation dialogs and generated badges. The details show the separate instance identifier and Bundle ID. ASCII names matching the old letter/digit/hyphen syntax become lowercase identifiers; other names receive an ASCII prefix (or `profile`) and a random suffix. The identifier is fixed at creation and used for app filenames, data paths and CLI operations. Existing profiles fall back to their original name; their paths and Bundle IDs are not rewritten. Renaming existing instances is not part of this feature.

## Update a copy from the official app

After updating the official app, quit the copy and all helpers, including browser integrations. Select **Update Copy…** in the app details, check the source path/version and confirm. The source defaults to `/Applications/ChatGPT.app`; a different official app may be selected. Harbor does not quit browsers or start the updated copy automatically. If helpers remain, use Clean Up Helpers to disconnect only that instance’s browser integration, then refresh. For managed copies, cleanup temporarily saves and removes Brave, Chrome and Edge native-host registrations pointing to that instance; launching through Harbor restores them if no other instance has registered there.

Harbor checks the local official app at `/Applications/ChatGPT.app` on refresh and every 60 seconds while Harbor runs (including with its main window closed). Managed copies whose readable current numeric build is older show an update badge in the sidebar, a menu-bar menu entry and a version notice in details. Stop the copy or clean up helpers before using the existing update action. Missing/invalid source metadata clears the notice; equal or older builds do not trigger it. This checks the installed official app, not online releases or custom source locations. Signature validation still occurs during the actual update. No copy is automatically updated.

```bash
harbor update work --source /Applications/ChatGPT.app
```

Only managed, non-adopted copies with matching registered identity/version and Harbor-owned data paths are eligible. A higher numeric build is required; identical short/build versions are a no-op. Downgrades and executable-layout changes are rejected. Source/copy signatures and compatibility markers are checked. The existing name, Bundle ID, app path, environment choices and data paths are preserved. Custom icon assets are copied; pristine copies get the new official icon. The original-icon snapshot is refreshed from the new official source.

Account directories are not copied, deleted or migrated by this operation. The new client may migrate databases on first launch; this is not a database backup or automatic rollback facility. Ordinary preparation/verification/manifest-publication errors restore the old app. A crash or failed rollback can leave a `.harbor-update-*` staging directory beside the app: preserve it and inspect both app and manifest versions before recovery. Do not bypass a version mismatch with `--accept-version-change` as an update procedure.

### Helpers after quitting the copy directly

Quitting the copy with Cmd-Q can leave helper processes running. Returning to Harbor or refreshing detects them as **Helpers still running**; **Clean Up Helpers** uses the same safe cleanup as `harbor stop <id>`. Known orphaned helpers receive SIGTERM even when another helper cannot be cleaned up. Cleanup also disconnects Chrome native hosts at the recognized plugin path inside this instance’s data directory, followed by their plugin-server children (or orphaned plugin servers). The browser, other instances and unrelated active/unknown helpers are preserved. Remaining processes are reported with their name, PID and parent PID. For managed copies, cleanup also pauses matching Brave, Chrome and Edge native-host registrations to prevent reconnection; launching through Harbor restores them without overwriting another registration. Cleanup failures refresh the displayed state; updates and deletion remain blocked while helpers exist. Harbor does not automatically terminate helpers on refresh.

## Harbor automatic updates

Release builds embed Sparkle 2.9.6. **Harbor → Check for Harbor Updates…** and the menu-bar menu offer manual checks and an **Automatically Check for Harbor Updates** toggle. Automatic checks default to once a day while Harbor runs; installation requires confirmation. Sparkle downloads, verifies, replaces Harbor and relaunches it. Checks are refused during a Harbor operation, relaunch is postponed until idle, and the normal termination guard remains active. Managed client copies and account directories are not updated by this mechanism. Debug builds disable the updater. Harbor menu labels follow the selected language; Sparkle's own standard dialogs follow the system's supported language.

The stable feed is `https://github.com/fynntang/harbor/releases/latest/download/appcast.xml`. Drafts and prereleases are not this channel. Both the feed and ZIP must carry valid Ed25519 signatures using the public key in `config/sparkle-public-key.txt`. Invalid signatures block installation; verification happens before extraction. Failed network checks do not replace the app. Updates require a greater workspace version; both macOS version fields use the same Cargo version. The currently published 0.0.1 has no updater: users must install an updater-enabled release manually once. Until a stable release includes the feed, manual checks can report that the feed is unavailable.

### Prepare and publish an update

The signing key is stored in the macOS login Keychain under Sparkle's service and account `local.harbor.desktop`. Only the public key belongs in Git. Keep a secure backup of this signing identity before changing machines; losing it prevents existing ad-hoc installations from trusting future updates. Do not generate a new key for each release. The ordinary CI workflow has no signing secret. The separate release workflow requires the `SPARKLE_PRIVATE_KEY` Actions secret.

1. Increase the calendar version in Cargo metadata and update Cargo.lock (for example, `26.9.101046` → `26.9.101047`). Keep the already published tags/assets immutable.
2. Build a release, then package the DMG and signed update in a fresh directory:

```sh
./scripts/build_and_run.sh --release-adhoc
python3 scripts/package_dmg.py dist/release-adhoc/Harbor.app --output-dir dist/next-release
python3 scripts/package_update.py dist/release-adhoc/Harbor.app --output-dir dist/next-release
```

For Developer ID builds, use `--release-only` and pass the signing team through `package_update.py --team TEAM_ID`; notarize/staple the final app before packaging. Ed25519 update signing does not replace Developer ID signing or notarization. The current DMG packager validates ad-hoc builds only.

3. Validate the artifacts and install/relaunch on a test Mac, then attach the generated ZIP, DMG, `appcast.xml`, and `SHA256SUMS.txt` to the matching `v<version>` GitHub Release. Publish as a stable release and mark it latest. Upload all assets while the release is still a draft; publish only after they are complete. The stable URL then resolves to the new signed feed. Never modify signed XML or ZIP bytes after signing.

`package_update.py` checks the final bundle, embedded key/feed/version and Keychain key match, signs and verifies both ZIP and feed, then writes checksums. It does not upload, change Git tags or export private keys. See [Sparkle documentation](https://sparkle-project.org/documentation/) for updater behavior and signing-key recovery limitations.

### Calendar release labels

Tags use `vYY.M.DHHmm`, based on the release's Asia/Shanghai date and 24-hour time. Month and day have no leading zero; the final time always has four digits. September 1 at midnight is `v26.9.10000`; September 10 at 11:56 is `v26.9.101156`. `0000` is a valid midnight time. No digit-sum encoding is used. Cargo is the source of truth, and GUI, CLI, build version, appcast and asset names use the same numeric version (only tags have `v`). The current configured version is `26.9.101853` (2026-09-10 18:53). `scripts/version.py` validates dates, leap years and hours/minutes. Numeric component comparisons preserve minute/hour/day/month/year order; raw lexicographic string sorting is not suitable. Only one release per minute can have a unique label: wait for the next minute rather than reusing a tag. Supported years are 2000–2099. Set the version at release preparation time and update Cargo.lock; builds do not change it automatically.

### GitHub Actions release workflow

`.github/workflows/release.yml` runs on future `v*` tag pushes whose commit contains the workflow. It checks that the tag matches Cargo, runs Rust/Swift/Python checks, builds on an arm64 macOS runner, verifies the DMG, and signs ZIP/appcast before publishing a complete stable Release as latest. It requires repository Actions secret `SPARKLE_PRIVATE_KEY` containing the existing Sparkle signing key's base64 export. Use the same key as the installed public key; never create a new identity for CI or place the secret in source. Only provision it through a secure secret input, never a command-line argument or log.

`ci_sign_update.py` removes the secret from its environment and passes it only through stdin to `package_update.py`. CryptoKit derives and checks its public key before Sparkle signs and verifies the archive/feed via stdin. No CI Keychain import or private-key file is needed. Build/test steps do not receive the key. `publish_release.py` refuses an existing release or an older stable version, requires all four assets and matching checksums, uploads to a draft, checks uploaded sizes, then publishes. A failure after draft creation leaves the draft for manual inspection; it is not overwritten on retry. Publication is serialized. This workflow currently produces ad-hoc builds, not Developer ID/notarized builds. A missing secret fails before building. The old `v26.9.101046` tag predates this workflow and is published locally; do not move it to trigger automation.

The workflow also supports **Run workflow** (`workflow_dispatch`) on main as a validation-only run: it builds and signs with the configured Secret and verifies the complete asset set, but never creates or edits a Release. Actual publication only happens on a `v*` tag push. The repository's signing Secret has been configured with authorization; maintainers should keep its access limited to trusted release code.
