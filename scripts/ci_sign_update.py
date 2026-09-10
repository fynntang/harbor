#!/usr/bin/env python3
"""Pass the Actions signing secret through stdin, never arguments or files."""
import base64
import os
import subprocess
import sys
from pathlib import Path


def secret_bytes(environment):
    if environment.get("GITHUB_ACTIONS") != "true":
        raise ValueError("This signing helper only runs on GitHub Actions")
    secret = environment.pop("SPARKLE_PRIVATE_KEY", "").strip()
    try:
        decoded = base64.b64decode(secret, validate=True)
    except ValueError:
        raise ValueError("Invalid update signing secret") from None
    if len(decoded) != 32:
        raise ValueError("Missing or invalid update signing secret")
    return secret.encode("ascii")


def main():
    secret = secret_bytes(os.environ)
    result = subprocess.run(
        [sys.executable, str(Path(__file__).with_name("package_update.py")), *sys.argv[1:], "--ed-key-stdin"],
        input=secret, capture_output=True)
    if result.returncode:
        raise ValueError("Update signing failed; check the configured public key and release bundle")
    print("Signed update archive and feed verified")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError):
        # Never include imported key material or output from a key-handling subprocess.
        raise SystemExit("CI update signing failed; verify runner, secret and public-key configuration") from None
