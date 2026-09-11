import base64
import sys
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from configure_updates import configure
from package_update import appcast, validate_info, NS


class UpdateTests(unittest.TestCase):
    def setUp(self):
        self.key = base64.b64encode(bytes(32)).decode()
        self.info = configure({"CFBundleVersion": "26.9.101046", "CFBundleShortVersionString": "26.9.101046"}, True, self.key)

    def test_release_gates_and_development_exclusion(self):
        self.assertEqual(validate_info(self.info, self.key), "26.9.101046")
        for field, value in [("CFBundleVersion", "1"), ("SUPublicEDKey", "other"),
                             ("SUFeedURL", "https://example.org"), ("HarborUpdatesEnabled", False),
                             ("SURequireSignedFeed", False), ("SUVerifyUpdateBeforeExtraction", False),
                             ("CFBundleShortVersionString", "0.0.2-beta")]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_info(dict(self.info, **{field: value}), self.key)
        self.assertFalse(configure({}, False, self.key)["HarborUpdatesEnabled"])

    def test_rejects_malformed_public_key_and_signature(self):
        with self.assertRaises(ValueError):
            configure({}, True, "YWJj")
        with self.assertRaises(ValueError):
            appcast("26.9.101046", "Harbor.zip", 123, "YWJj")

    def test_appcast_pins_archive_version_signature_and_size(self):
        signature = base64.b64encode(bytes(64)).decode()
        root = ET.fromstring(appcast("26.9.101047", "Harbor-26.9.101047-macos-universal.zip", 123, signature))
        item = root.find("channel/item")
        self.assertEqual(item.find(f"{{{NS}}}version").text, "26.9.101047")
        enclosure = item.find("enclosure")
        self.assertEqual(enclosure.attrib["length"], "123")
        self.assertEqual(enclosure.attrib[f"{{{NS}}}edSignature"], signature)
        self.assertIn("/v26.9.101047/Harbor-26.9.101047-macos-universal.zip", enclosure.attrib["url"])
