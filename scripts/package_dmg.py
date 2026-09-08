#!/usr/bin/env python3
"""Package an existing ad-hoc Harbor release without rebuilding or changing its app."""
import argparse
import hashlib
import os
import plistlib
import re
import subprocess
import tempfile
from pathlib import Path

import signing


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


def package(app, output):
    app = app.resolve(strict=True)
    with (app / "Contents/Info.plist").open("rb") as stream:
        version = plistlib.load(stream).get("CFBundleShortVersionString", "")
    if not isinstance(version, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Expected a numeric Harbor release version")
    signing.verify(app, version)
    original = contents(app)
    output.mkdir(parents=True, exist_ok=True)
    stem = f"Harbor-{version}-macos-arm64"
    destination = output / f"{stem}.dmg"
    with tempfile.TemporaryDirectory(prefix=".harbor-dmg-", dir=output) as temporary:
        temporary = Path(temporary)
        source = temporary / "source"
        source.mkdir()
        run("/usr/bin/ditto", app, source / "Harbor.app")
        (source / "Applications").symlink_to("/Applications")
        (source / "Install - 安装说明.txt").write_text(
            "Harbor — macOS 14+ / Apple Silicon\n\n"
            "将 Harbor.app 拖入 Applications，推出此磁盘，再从应用程序启动 Harbor。\n"
            "更新前请退出 Harbor。副本账号数据不会随安装包覆盖。\n"
            "请另行安装官方 ChatGPT/Codex 应用。\n\n"
            "Drag Harbor.app into Applications, eject this disk, then open Harbor from Applications.\n"
            "Quit Harbor before updating. This installer contains no account data.\n"
            "Install the official ChatGPT/Codex app separately.\n\n"
            "本地 ad-hoc 签名，未经 Apple 公证。 / Ad-hoc signed; not notarized.\n"
            "首次打开 / First launch: https://support.apple.com/en-us/102445\n",
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
            signing.verify(mount / "Harbor.app", version)
            if contents(mount / "Harbor.app") != original:
                raise ValueError("Packaged app differs from the input release")
        finally:
            run("/usr/bin/hdiutil", "detach", mount)
        image.replace(destination)
        checksums = []
        for extension in ("zip", "dmg"):
            artifact = output / f"{stem}.{extension}"
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
    args = parser.parse_args()
    package(args.app, args.output_dir.resolve())
