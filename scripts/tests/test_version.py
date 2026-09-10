import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from version import display, normalized, validate_display
from signing import check_versions


class CalendarVersionTests(unittest.TestCase):
    def test_examples_and_metadata_mapping(self):
        self.assertEqual(display("26.9.10000"), "26.9.10000")
        self.assertEqual(display("26.9.101156"), "26.9.101156")
        self.assertEqual(display("26.9.10800"), "26.9.10800")
        check_versions({"CFBundleIdentifier": "local.harbor.desktop", "CFBundleShortVersionString": "26.9.101046", "CFBundleVersion": "26.9.101046", "LSMinimumSystemVersion": "14.0"}, "26.9.101046", "harbor 26.9.101046")

    def test_dates_times_and_canonical_display(self):
        self.assertEqual(validate_display("28.2.290000"), "28.2.290000")
        self.assertEqual(validate_display("26.9.12359"), "26.9.12359")
        for value in ["26.2.290000", "26.13.10001", "26.9.1046", "26.09.101046", "26.9.010800", "26.9.12400", "26.9.12360", "26.9.1000000", "v26.9.101046", "26.9.101046-beta"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_display(value)

    def test_order_across_minute_hour_day_month_and_year(self):
        versions = ["26.9.10000", "26.9.10001", "26.9.10059", "26.9.10100", "26.9.12359", "26.9.20000", "26.9.92359", "26.9.100000", "26.9.101156", "26.10.10000", "27.1.10000"]
        values = [tuple(map(int, validate_display(v).split('.'))) for v in versions]
        self.assertTrue(all(a < b for a, b in zip(values, values[1:])))
