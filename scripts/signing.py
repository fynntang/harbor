#!/usr/bin/env python3
"""Read-only gates for Harbor's Developer ID builds; never handles private keys."""
import os
import plistlib
import re
import subprocess
import sys
from pathlib import Path


def run(*args):
    result = subprocess.run(args, capture_output=True, text=True)
    if result.returncode:
        raise ValueError(f"{Path(args[0]).name} failed while checking release prerequisites")
    return result.stdout + result.stderr


def identity_team(identity, identities):
    if not re.fullmatch(r"[0-9a-fA-F]{40}", identity):
        raise ValueError("Set HARBOR_SIGNING_IDENTITY to a Developer ID Application certificate SHA-1 fingerprint")
    for line in identities.splitlines():
        match = re.search(r'\b([0-9A-Fa-f]{40}) "Developer ID Application: .* \(([A-Z0-9]+)\)"', line)
        if match and match[1].lower() == identity.lower():
            return match[2]
    raise ValueError("The requested valid Developer ID Application identity is unavailable in the keychain")


def preflight():
    identity = os.environ.get("HARBOR_SIGNING_IDENTITY", "")
    # Reject missing/invalid settings before querying the keychain or changing build output.
    if not re.fullmatch(r"[0-9a-fA-F]{40}", identity):
        raise ValueError("Set HARBOR_SIGNING_IDENTITY to a Developer ID Application certificate SHA-1 fingerprint")
    return identity_team(identity, run("/usr/bin/security", "find-identity", "-v", "-p", "codesigning"))


def check_signature(details, team, identifier):
    lines = details.splitlines()
    if f"Identifier={identifier}" not in lines:
        raise ValueError("Unexpected code-signing identifier")
    if f"TeamIdentifier={team}" not in lines:
        raise ValueError("Release components must use the configured signing team")
    if not any(line.startswith("Authority=Developer ID Application:") for line in lines):
        raise ValueError("A Developer ID Application signature is required; ad-hoc builds cannot be released")
    if not any(line.startswith("Timestamp=") and line.removeprefix("Timestamp=").strip() not in ("", "none") for line in lines):
        raise ValueError("A secure signing timestamp is required")
    if not any(line.startswith("CodeDirectory ") and re.search(r"flags=0x[0-9a-f]+\([^)]*\bruntime\b", line) for line in lines):
        raise ValueError("Hardened Runtime is required for every release component")


def check_versions(info, version, cli_version):
    if info.get("CFBundleIdentifier") != "local.harbor.desktop":
        raise ValueError("Unexpected application Bundle ID")
    if info.get("CFBundleShortVersionString") != version or cli_version.strip() != f"harbor {version}":
        raise ValueError("GUI and bundled CLI versions must match the workspace version")
    if info.get("LSMinimumSystemVersion") != "14.0":
        raise ValueError("The release must declare its supported macOS minimum")


def check_architectures(architectures):
    if len(architectures) != 3 or any(value.strip() != "arm64" for value in architectures):
        raise ValueError("This release supports arm64 only; GUI and both helpers must all be arm64")


def verify(bundle, version, team):
    bundle = Path(bundle)
    contents = bundle / "Contents"
    paths = [bundle, contents / "Helpers/harbor", contents / "Helpers/harbor-native"]
    identifiers = ["local.harbor.desktop", "local.harbor.desktop.cli", "local.harbor.desktop.native"]
    for path, identifier in zip(paths, identifiers):
        run("/usr/bin/codesign", "--verify", "--strict", "--test-requirement", "anchor apple generic", str(path))
        check_signature(run("/usr/bin/codesign", "-dvv", str(path)), team, identifier)
    run("/usr/bin/codesign", "--verify", "--deep", "--strict", str(bundle))
    check_architectures([run("/usr/bin/lipo", "-archs", str(path)) for path in [contents / "MacOS/Harbor", *paths[1:]]])
    with (contents / "Info.plist").open("rb") as stream:
        check_versions(plistlib.load(stream), version, run(str(paths[1]), "--version"))


def main():
    if sys.argv[1:] == ["preflight"]:
        print(preflight())
    elif len(sys.argv) == 5 and sys.argv[1] == "verify":
        verify(*sys.argv[2:])
    else:
        raise ValueError("usage: signing.py preflight | verify APP VERSION TEAM")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, plistlib.InvalidFileException) as error:
        sys.exit(f"Harbor release stopped: {error}")
