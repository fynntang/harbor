#!/usr/bin/env bash
set -euo pipefail
MODE="${1:-run}"
if [ "$#" -gt 0 ]; then shift; fi
case "$MODE" in
  run|--build-only|--release-only|--release-adhoc|--debug|--logs|--telemetry|--verify) ;;
  *) echo "usage: $0 [run|--build-only|--release-only|--release-adhoc|--debug|--logs|--telemetry|--verify] [--registry PATH]" >&2; exit 2 ;;
esac
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/dist/Harbor.app"
cd "$ROOT_DIR"
SWIFT_CONFIGURATION=debug
SIGNING_ARGS=(--force --sign -)
if [ "$MODE" = --release-only ] || [ "$MODE" = --release-adhoc ]; then
    if [ "$#" -ne 0 ]; then
        echo "$MODE does not accept launch arguments" >&2
        exit 2
    fi
    APP_BUNDLE="$ROOT_DIR/dist/release/Harbor.app"
    SWIFT_CONFIGURATION=release
    if [ "$MODE" = --release-only ]; then
        RELEASE_TEAM="$(python3 scripts/signing.py preflight)"
        SIGNING_ARGS=(--force --sign "$HARBOR_SIGNING_IDENTITY" --options runtime --timestamp)
    else
        APP_BUNDLE="$ROOT_DIR/dist/release-adhoc/Harbor.app"
    fi
fi
if [ "$SWIFT_CONFIGURATION" = debug ]; then
# Stop only this checkout's GUI. Refuse while its bundled CLI has an operation in flight.
python3 - "$APP_BUNDLE" <<'PY'
import os, signal, subprocess, sys
bundle = sys.argv[1]
rows = subprocess.check_output(['/bin/ps', '-ww', '-axo', 'pid=,comm='], text=True).splitlines()
parsed = [row.strip().split(None, 1) for row in rows]
if any(len(row) == 2 and row[1] in [bundle + '/Contents/Helpers/harbor', bundle + '/Contents/Helpers/harbor-native'] for row in parsed):
    sys.exit('Harbor has an operation in progress. Wait for it to finish before rebuilding.')
for row in parsed:
    if len(row) == 2 and row[1] == bundle + '/Contents/MacOS/Harbor':
        try: os.kill(int(row[0]), signal.SIGTERM)
        except ProcessLookupError: pass
PY
fi
cargo build --release --locked -p harbor-cli
APP_BUILD_VERSION="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "harbor-cli"))')"
APP_VERSION="$(python3 scripts/version.py "$APP_BUILD_VERSION")"
swift build --configuration "$SWIFT_CONFIGURATION" --package-path apps/Harbor --scratch-path target/swift-harbor
SWIFT_BIN="$(swift build --configuration "$SWIFT_CONFIGURATION" --package-path apps/Harbor --scratch-path target/swift-harbor --show-bin-path)/Harbor"
mkdir -p "$(dirname "$APP_BUNDLE")"
STAGING="$(mktemp -d "$ROOT_DIR/dist/.harbor-build.XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT
CONTENTS="$STAGING/Harbor.app/Contents"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Helpers" "$CONTENTS/Resources"
cp apps/Harbor/Resources/Harbor.icns "$CONTENTS/Resources/Harbor.icns"
cp "$SWIFT_BIN" "$CONTENTS/MacOS/Harbor"
cp target/release/harbor "$CONTENTS/Helpers/harbor"
cp "$(dirname "$SWIFT_BIN")/harbor-native" "$CONTENTS/Helpers/harbor-native"
cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Harbor</string>
<key>CFBundleIdentifier</key><string>local.harbor.desktop</string>
<key>CFBundleName</key><string>Harbor</string>
<key>CFBundleDisplayName</key><string>Harbor</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleIconFile</key><string>Harbor.icns</string>
<key>CFBundleShortVersionString</key><string>${APP_VERSION}</string>
<key>CFBundleVersion</key><string>${APP_BUILD_VERSION}</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
# SwiftPM links Sparkle, but standalone bundles must embed and sign its nested code.
SPARKLE_FRAMEWORK="$ROOT_DIR/target/swift-harbor/artifacts/sparkle/Sparkle/Sparkle.xcframework/macos-arm64_x86_64/Sparkle.framework"
mkdir -p "$CONTENTS/Frameworks"
/usr/bin/ditto "$SPARKLE_FRAMEWORK" "$CONTENTS/Frameworks/Sparkle.framework"
cp "$ROOT_DIR/target/swift-harbor/artifacts/sparkle/Sparkle/LICENSE" "$CONTENTS/Resources/Sparkle-LICENSE"
python3 scripts/configure_updates.py "$CONTENTS/Info.plist" "$SWIFT_CONFIGURATION"
SPARKLE_VERSION="$CONTENTS/Frameworks/Sparkle.framework/Versions/B"
for component in "$SPARKLE_VERSION/XPCServices/Downloader.xpc" "$SPARKLE_VERSION/XPCServices/Installer.xpc" "$SPARKLE_VERSION/Autoupdate" "$SPARKLE_VERSION/Updater.app" "$CONTENTS/Frameworks/Sparkle.framework"; do
    /usr/bin/codesign "${SIGNING_ARGS[@]}" "$component"
done
/usr/bin/codesign "${SIGNING_ARGS[@]}" --identifier local.harbor.desktop.cli "$CONTENTS/Helpers/harbor"
/usr/bin/codesign "${SIGNING_ARGS[@]}" --identifier local.harbor.desktop.native "$CONTENTS/Helpers/harbor-native"
/usr/bin/codesign "${SIGNING_ARGS[@]}" "$STAGING/Harbor.app"
/usr/bin/codesign --verify --deep --strict "$STAGING/Harbor.app"
if [ "$MODE" = --release-only ]; then
    python3 scripts/signing.py verify "$STAGING/Harbor.app" "$APP_VERSION" "$RELEASE_TEAM"
elif [ "$MODE" = --release-adhoc ]; then
    python3 scripts/signing.py verify-adhoc "$STAGING/Harbor.app" "$APP_VERSION"
fi
# This is a generated app in dist; no client apps or profile data are stored here.
rm -rf "$APP_BUNDLE"
mv "$STAGING/Harbor.app" "$APP_BUNDLE"
case "$MODE" in
  --build-only|--release-only|--release-adhoc) echo "$APP_BUNDLE" ;;
  --debug) lldb -- "$APP_BUNDLE/Contents/MacOS/Harbor" "$@" ;;
  *)
    /usr/bin/open -n "$APP_BUNDLE" --args "$@"
    case "$MODE" in
      --verify)
        sleep 1
        python3 - "$APP_BUNDLE/Contents/MacOS/Harbor" <<'PYVERIFY'
import subprocess, sys
rows = subprocess.check_output(['/bin/ps', '-ww', '-axo', 'comm='], text=True).splitlines()
if sys.argv[1] not in rows: sys.exit('The built Harbor GUI process did not remain running.')
PYVERIFY
        ;;
      --logs) /usr/bin/log stream --info --style compact --predicate 'process == "Harbor"' ;;
      --telemetry) /usr/bin/log stream --info --style compact --predicate 'subsystem == "local.harbor.desktop"' ;;
    esac
    ;;
esac
