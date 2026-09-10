import importlib.util
import os
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("signing", ROOT / "scripts/signing.py")
signing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signing)


class SigningTests(unittest.TestCase):
    def test_only_the_requested_valid_developer_id_is_accepted(self):
        fingerprint = "A" * 40
        identities = f'1) {fingerprint} "Developer ID Application: Test Publisher (TESTTEAM01)"'
        self.assertEqual(signing.identity_team(fingerprint.lower(), identities), "TESTTEAM01")
        for requested, available in [
            ("", identities), ("-", identities), ("B" * 40, identities),
            (fingerprint, identities.replace("Developer ID Application", "Apple Development")),
            (fingerprint, "0 valid identities found"),
        ]:
            with self.subTest(requested=requested), self.assertRaises(ValueError):
                signing.identity_team(requested, available)

    def test_missing_configuration_never_queries_the_keychain(self):
        with patch.dict(os.environ, {"HARBOR_SIGNING_IDENTITY": ""}), patch.object(signing, "run") as run:
            with self.assertRaises(ValueError):
                signing.preflight()
            run.assert_not_called()

    def test_all_signature_requirements_are_mandatory(self):
        details = "\n".join([
            "Identifier=local.harbor.desktop",
            "TeamIdentifier=TESTTEAM01",
            "Authority=Developer ID Application: Test Publisher (TESTTEAM01)",
            "Timestamp=Sep 8, 2026 at 12:00:00",
            "CodeDirectory v=20500 size=123 flags=0x10000(runtime) hashes=10+3",
        ])
        signing.check_signature(details, "TESTTEAM01", "local.harbor.desktop")
        for line in details.splitlines():
            with self.subTest(missing=line), self.assertRaises(ValueError):
                signing.check_signature(details.replace(line, ""), "TESTTEAM01", "local.harbor.desktop")
        with self.assertRaises(ValueError):
            signing.check_signature(details, "OTHERTEAM1", "local.harbor.desktop")
        with self.assertRaises(ValueError):
            signing.check_signature(details.replace("0x10000(runtime)", "0x2(adhoc)"), "TESTTEAM01", "local.harbor.desktop")

    def test_adhoc_gate_requires_explicit_signature_and_identity(self):
        valid = "Identifier=local.harbor.desktop\nSignature=adhoc\nTeamIdentifier=not set"
        signing.check_adhoc_signature(valid, "local.harbor.desktop")
        for invalid in [valid.replace("Signature=adhoc", ""), valid.replace("local.harbor.desktop", "other"), valid + "\nAuthority=Unexpected"]:
            with self.assertRaises(ValueError):
                signing.check_adhoc_signature(invalid, "local.harbor.desktop")

    def test_versions_and_architectures_cannot_drift(self):
        info = {"CFBundleIdentifier": "local.harbor.desktop", "CFBundleShortVersionString": "0.0.1", "LSMinimumSystemVersion": "14.0"}
        signing.check_versions(info, "0.0.1", "harbor 0.0.1\n")
        for version, cli in [("0.0.2", "harbor 0.0.1"), ("0.0.1", "harbor 0.0.2")]:
            with self.assertRaises(ValueError):
                signing.check_versions(info, version, cli)
        signing.check_architectures(["arm64\n"] * 3)
        for architectures in [["arm64", "x86_64", "arm64"], ["arm64"] * 2, ["arm64 x86_64"] * 3]:
            with self.assertRaises(ValueError):
                signing.check_architectures(architectures)

    def test_adhoc_release_rejects_launch_arguments_before_build(self):
        result = subprocess.run(
            ["bash", str(ROOT / "scripts/build_and_run.sh"), "--release-adhoc", "--registry", "/tmp/test"],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("does not accept launch arguments", result.stderr)
        self.assertEqual(result.stdout, "")

    def test_real_release_entry_stops_before_build_when_unconfigured(self):
        result = subprocess.run(
            ["bash", str(ROOT / "scripts/build_and_run.sh"), "--release-only"],
            env={**os.environ, "HARBOR_SIGNING_IDENTITY": ""},
            capture_output=True, text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Harbor release stopped", result.stderr)
        self.assertIn("HARBOR_SIGNING_IDENTITY", result.stderr)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
