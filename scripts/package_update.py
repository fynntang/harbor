#!/usr/bin/env python3
"""Prepare a signed Harbor ZIP and appcast for a stable GitHub release. No uploads."""
import argparse
import base64
import hashlib
import plistlib
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

from configure_updates import FEED, PUBLIC_KEY
from signing import verify
from version import normalized, validate_display

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "target/swift-harbor/artifacts/sparkle/Sparkle/bin"
ACCOUNT = "local.harbor.desktop"
NS = "http://www.andymatuschak.org/xml-namespaces/sparkle"
ET.register_namespace("sparkle", NS)


def checked(*args):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True)
    if result.returncode:
        # Signing tools may handle secrets internally; never echo their output on failure.
        raise ValueError(f"{Path(args[0]).name} failed; check signing key access and inputs")
    return result.stdout.strip()


def validate_info(info, public_key):
    version = info.get("CFBundleShortVersionString", "")
    validate_display(version)
    if info.get("CFBundleVersion") != normalized(version):
        raise ValueError("Bundle build and workspace version must match")
    if info.get("SUFeedURL") != FEED or info.get("SUPublicEDKey") != public_key:
        raise ValueError("Unexpected updater feed or public key")
    if not info.get("HarborUpdatesEnabled") or not info.get("SUVerifyUpdateBeforeExtraction") or not info.get("SURequireSignedFeed"):
        raise ValueError("Only release builds with signed-feed and archive verification can be published")
    return version


def appcast(version, filename, size, signature):
    if len(base64.b64decode(signature, validate=True)) != 64:
        raise ValueError("Invalid Ed25519 archive signature")
    rss = ET.Element("rss", version="2.0")
    channel = ET.SubElement(rss, "channel")
    ET.SubElement(channel, "title").text = "Harbor"
    item = ET.SubElement(channel, "item")
    ET.SubElement(item, "title").text = f"Harbor {version}"
    ET.SubElement(item, f"{{{NS}}}version").text = normalized(version)
    ET.SubElement(item, f"{{{NS}}}shortVersionString").text = version
    ET.SubElement(item, f"{{{NS}}}minimumSystemVersion").text = "14.0"
    ET.SubElement(item, "enclosure", {
        "url": f"https://github.com/fynntang/harbor/releases/download/v{version}/{filename}",
        "length": str(size), "type": "application/octet-stream",
        f"{{{NS}}}edSignature": signature,
    })
    return ET.tostring(rss, encoding="utf-8", xml_declaration=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist/artifacts")
    parser.add_argument("--team", help="Developer ID team; omit for explicit ad-hoc builds")
    args = parser.parse_args()
    info = plistlib.loads((args.app / "Contents/Info.plist").read_bytes())
    public_key = PUBLIC_KEY.read_text().strip()
    version = validate_info(info, public_key)
    verify(args.app, version, args.team)
    if checked(TOOLS / "generate_keys", "--account", ACCOUNT, "-p") != public_key:
        raise ValueError("The Keychain signing key does not match the embedded public key")
    filename = f"Harbor-{version}-macos-arm64.zip"
    args.output_dir.mkdir(parents=True, exist_ok=True)
    if (args.output_dir / filename).exists():
        raise ValueError("ZIP already exists; use an empty output directory to avoid replacing release artifacts")
    with tempfile.TemporaryDirectory(dir=args.output_dir) as temp:
        archive = Path(temp) / filename
        checked("/usr/bin/ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", args.app, archive)
        signature = checked(TOOLS / "sign_update", "--account", ACCOUNT, "-p", archive)
        checked(TOOLS / "sign_update", "--account", ACCOUNT, "--verify", archive, signature)
        feed = Path(temp) / "appcast.xml"
        feed.write_bytes(appcast(version, filename, archive.stat().st_size, signature))
        checked(TOOLS / "sign_update", "--account", ACCOUNT, feed)
        checked(TOOLS / "sign_update", "--account", ACCOUNT, "--verify", feed)
        archive.rename(args.output_dir / filename)
        feed.replace(args.output_dir / "appcast.xml")
    assets = [args.output_dir / filename, args.output_dir / "appcast.xml"]
    dmg = args.output_dir / f"Harbor-{version}-macos-arm64.dmg"
    if dmg.exists():
        assets.append(dmg)
    (args.output_dir / "SHA256SUMS.txt").write_text("".join(
        f"{hashlib.sha256(asset.read_bytes()).hexdigest()}  {asset.name}\n" for asset in assets))
    print(f"Prepared signed update {version} in {args.output_dir}; nothing uploaded")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError) as error:
        raise SystemExit(str(error))
