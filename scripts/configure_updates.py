#!/usr/bin/env python3
"""Embed public updater configuration. This script never reads private keys."""
import base64
import plistlib
import sys
from pathlib import Path

FEED = "https://github.com/fynntang/harbor/releases/latest/download/appcast.xml"
PUBLIC_KEY = Path(__file__).resolve().parents[1] / "config/sparkle-public-key.txt"


def configure(plist, release, public_key):
    if len(base64.b64decode(public_key, validate=True)) != 32:
        raise ValueError("Sparkle requires a 32-byte Ed25519 public key")
    plist.update({
        "SUFeedURL": FEED,
        "SUPublicEDKey": public_key,
        "SUEnableAutomaticChecks": True,
        "SUAutomaticallyUpdate": False,
        "SUAllowsAutomaticUpdates": False,
        "SUEnableSystemProfiling": False,
        "SUScheduledCheckInterval": 86400,
        "SUVerifyUpdateBeforeExtraction": True,
        "SURequireSignedFeed": True,
        "HarborUpdatesEnabled": release,
    })
    return plist


if __name__ == "__main__":
    target = Path(sys.argv[1])
    info = plistlib.loads(target.read_bytes())
    info = configure(info, sys.argv[2] == "release", PUBLIC_KEY.read_text().strip())
    target.write_bytes(plistlib.dumps(info))
