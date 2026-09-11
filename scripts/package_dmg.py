#!/usr/bin/env python3
"""Package an existing ad-hoc Harbor release without rebuilding or changing its app."""
import argparse
import hashlib
import os
import plistlib
import re
import subprocess
import tempfile
import time
from pathlib import Path

import signing
from installation import FIRST_LAUNCH


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def contents(bundle):
    result = {}
    for item in bundle.rglob("*"):
        if item.is_symlink():
            result[str(item.relative_to(bundle))] = ("link", os.readlink(item))
        elif item.is_file():
            result[str(item.relative_to(bundle))] = (
                item.stat().st_mode & 0o777, hashlib.sha256(item.read_bytes()).hexdigest())
    return result


TARGETS = {"aarch64-apple-darwin": "arm64", "x86_64-apple-darwin": "x86_64"}


def package(app, output, target):
    architecture = TARGETS[target]
    app = app.resolve(strict=True)
    with (app / "Contents/Info.plist").open("rb") as stream:
        version = plistlib.load(stream).get("CFBundleShortVersionString", "")
    if not isinstance(version, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Expected a numeric Harbor release version")
    signing.verify(app, version)
    output.mkdir(parents=True, exist_ok=True)
    stem = f"Harbor-{version}-{target}"
    destination = output / f"{stem}.dmg"
    with tempfile.TemporaryDirectory(prefix=".harbor-dmg-", dir=output) as temporary:
        temporary = Path(temporary)
        source = temporary / "source"
        source.mkdir()
        run("/usr/bin/ditto", app, source / "Harbor.app")
        # Work on a separate copy; keep the Universal update bundle unchanged.
        payload = source / "Harbor.app"
        binaries = [("MacOS/Harbor", "local.harbor.desktop"),
                    ("Helpers/harbor", "local.harbor.desktop.cli"),
                    ("Helpers/harbor-native", "local.harbor.desktop.native")]
        for relative, identifier in binaries:
            binary = payload / "Contents" / relative
            thin = binary.with_name(binary.name + ".thin")
            run("/usr/bin/lipo", binary, "-thin", architecture, "-output", thin)
            thin.replace(binary)
            run("/usr/bin/codesign", "--force", "--sign", "-", "--identifier", identifier, binary)
        run("/usr/bin/codesign", "--force", "--sign", "-", payload)
        signing.verify(payload, version, architectures={architecture})
        original = contents(payload)
        (source / "Applications").symlink_to("/Applications")
        (source / "Install - 安装说明.txt").write_text(
            f"Harbor — macOS 14+ / {target}\n\n"
            "将 Harbor.app 拖入 Applications，推出此磁盘，再从应用程序启动 Harbor。\n"
            "更新前请退出 Harbor。副本账号数据不会随安装包覆盖。\n"
            "请另行安装官方 ChatGPT/Codex 应用。\n\n"
            "Drag Harbor.app into Applications, eject this disk, then open Harbor from Applications.\n"
            "Quit Harbor before updating. This installer contains no account data.\n"
            "Install the official ChatGPT/Codex app separately.\n\n"
            "本地 ad-hoc 签名，未经 Apple 公证。 / Ad-hoc signed; not notarized.\n"
            + FIRST_LAUNCH,
            encoding="utf-8")
        image = temporary / destination.name
        run("/usr/bin/hdiutil", "create", "-volname", f"Harbor {version}",
            "-srcfolder", source, "-fs", "HFS+", "-format", "UDZO", image)
        run("/usr/bin/hdiutil", "verify", image)
        mount = temporary / "mounted"
        mount.mkdir()
        run("/usr/bin/hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", mount, image)
        try:
            if os.readlink(mount / "Applications") != "/Applications":
                raise ValueError("Incorrect Applications shortcut")
            signing.verify(mount / "Harbor.app", version, architectures={architecture})
            if contents(mount / "Harbor.app") != original:
                raise ValueError("Packaged app differs from the input release")
        finally:
            for attempt in range(5):
                result = subprocess.run(["/usr/bin/hdiutil", "detach", str(mount)])
                if result.returncode == 0:
                    break
                if attempt == 4:
                    result.check_returncode()
                time.sleep(1)
        image.replace(destination)
        checksums = []
        for name in [*(f"Harbor-{version}-{item}.dmg" for item in TARGETS),
                     f"Harbor-{version}-macos-universal.zip", "appcast.xml"]:
            artifact = output / name
            if artifact.is_file():
                checksums.append(f"{hashlib.sha256(artifact.read_bytes()).hexdigest()}  {artifact.name}\n")
        checksum_file = temporary / "SHA256SUMS.txt"
        checksum_file.write_text("".join(checksums), encoding="utf-8")
        checksum_file.replace(output / "SHA256SUMS.txt")
    print(f"Verified DMG: {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("dist/artifacts"))
    parser.add_argument("--target", choices=TARGETS, help="Default: package both architectures")
    args = parser.parse_args()
    for target in ([args.target] if args.target else TARGETS):
        package(args.app, args.output_dir.resolve(), target)
