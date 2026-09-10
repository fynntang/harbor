import base64
import hashlib
import sys
import tempfile
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_sign_update import secret_bytes
from publish_release import assets_for


class ReleaseTests(unittest.TestCase):
    def test_secret_validation_does_not_include_input_in_errors(self):
        for value in ['', 'invalid-placeholder', base64.b64encode(bytes(8)).decode()]:
            environment = {'GITHUB_ACTIONS': 'true', 'SPARKLE_PRIVATE_KEY': value}
            with self.assertRaises(ValueError) as result:
                secret_bytes(environment)
            if value:
                self.assertNotIn(value, str(result.exception))
            self.assertNotIn('SPARKLE_PRIVATE_KEY', environment)
        with self.assertRaises(ValueError):
            secret_bytes({})
        synthetic = base64.b64encode(bytes(32)).decode()
        self.assertEqual(secret_bytes({'GITHUB_ACTIONS': 'true', 'SPARKLE_PRIVATE_KEY': synthetic}), synthetic.encode())

    def test_assets_must_be_complete_and_unchanged(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            with self.assertRaises(ValueError):
                assets_for('v26.9.101046', directory)
            names = ['Harbor-26.9.101046-macos-arm64.dmg', 'Harbor-26.9.101046-macos-arm64.zip', 'appcast.xml']
            for name in names:
                (directory / name).write_bytes(b'synthetic fixture')
            (directory / 'SHA256SUMS.txt').write_text(''.join(f'{hashlib.sha256((directory/name).read_bytes()).hexdigest()}  {name}\n' for name in names))
            self.assertEqual(len(assets_for('v26.9.101046', directory)), 4)
            (directory / names[0]).write_bytes(b'changed')
            with self.assertRaises(ValueError):
                assets_for('v26.9.101046', directory)
            with self.assertRaises(ValueError):
                assets_for('26.9.101046', directory)
