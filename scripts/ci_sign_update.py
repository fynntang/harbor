#!/usr/bin/env python3
"""Use the Actions signing secret only in an ephemeral macOS runner's Keychain."""
import base64
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from package_update import ACCOUNT, TOOLS


def secret_bytes(environment):
    if environment.get("GITHUB_ACTIONS") != "true":
        raise ValueError("This signing helper only runs on GitHub Actions")
    secret = environment.pop("SPARKLE_PRIVATE_KEY", "").strip()
    try:
        decoded = base64.b64decode(secret, validate=True)
    except ValueError:
        raise ValueError("Invalid update signing secret") from None
    if len(decoded) not in (32, 96):
        raise ValueError("Missing or invalid update signing secret")
    return secret.encode("ascii")


def main():
    secret = secret_bytes(os.environ)
    tool = str(TOOLS / "generate_keys")
    existing = subprocess.run([tool, "--account", ACCOUNT, "-p"], capture_output=True)
    if existing.returncode == 0:
        raise ValueError("Refusing to overwrite an existing runner signing identity")
    with tempfile.TemporaryDirectory(prefix="harbor-signing-") as temporary:
        keyfile = Path(temporary) / "key"
        descriptor = os.open(keyfile, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(secret)
        del secret
        try:
            result = subprocess.run([tool, "--account", ACCOUNT, "-f", str(keyfile)], capture_output=True)
            if result.returncode:
                raise ValueError("Could not import the update signing identity")
            keyfile.unlink()
            # Public-key matching and archive/feed verification are enforced by this script.
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("package_update.py")), *sys.argv[1:]], capture_output=True)
            if result.returncode:
                raise ValueError("Update signing failed; check the configured public key and release bundle")
            print("Signed update archive and feed verified")
        finally:
            keyfile.unlink(missing_ok=True)
            subprocess.run(["/usr/bin/security", "delete-generic-password", "-s", "https://sparkle-project.org", "-a", ACCOUNT], capture_output=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError):
        # Never include imported key material or output from a key-handling subprocess.
        raise SystemExit("CI update signing failed; verify runner, secret and public-key configuration") from None
