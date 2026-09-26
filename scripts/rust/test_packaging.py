"""Isolated packaging/install tests; never build, launch or use home Applications."""

import hashlib
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
import subprocess


SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))
import install  # noqa: E402
import package  # noqa: E402

metadata_spec = importlib.util.spec_from_file_location("source_metadata", SCRIPT_DIR / "source-metadata.py")
source_metadata = importlib.util.module_from_spec(metadata_spec)
metadata_spec.loader.exec_module(source_metadata)


REVISION = "0123456789abcdef0123456789abcdef01234567"


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="sofdevtool-package-test-")
        self.root = Path(self.temporary.name)
        self.binary = self.root / "sofdevtool"
        self.binary.write_bytes(b"synthetic executable; never launched")

    def tearDown(self):
        self.temporary.cleanup()

    def bundle(self, profile):
        return package.assemble(
            profile, self.binary, self.root / "artifacts", "0.2.0", "42", REVISION, "dirty"
        )

    def test_packaging_metadata_is_an_exact_head_with_known_worktree_state(self):
        revision, state = package.source_metadata()
        actual_head = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=package.ROOT,
            check=True, stdout=subprocess.PIPE, text=True,
        ).stdout.strip()
        self.assertEqual(revision, actual_head)
        self.assertIn(state, {"clean", "dirty"})

    def test_chmod_only_change_is_dirty_against_head_mode(self):
        fixture = self.root / "tracked-source"
        fixture.write_text("unchanged bytes")
        fixture.chmod(0o644)
        expected_blob = source_metadata.blob_id(fixture)
        self.assertTrue(source_metadata.mode_matches(fixture, "100644"))
        self.assertFalse(source_metadata.mode_matches(fixture, "100755"))
        fixture.chmod(0o755)
        self.assertEqual(source_metadata.blob_id(fixture), expected_blob)
        self.assertFalse(source_metadata.mode_matches(fixture, "100644"))
        self.assertTrue(source_metadata.mode_matches(fixture, "100755"))

    def test_identified_bundles_have_distinct_icons_and_local_notices(self):
        debug = self.bundle("debug")
        release = self.bundle("release")
        self.assertEqual(install.plist_value(debug, "CFBundleIdentifier"), "com.gerardocalia.sofdevtool.debug")
        self.assertEqual(install.plist_value(release, "CFBundleIdentifier"), "com.gerardocalia.sofdevtool")
        self.assertEqual(install.plist_value(release, "CFBundleShortVersionString"), "0.2.0")
        self.assertEqual(install.plist_value(release, "CFBundleVersion"), "42")
        self.assertEqual(install.plist_value(release, "SofDevToolRevision"), REVISION)
        self.assertEqual(install.plist_value(release, "SofDevToolSourceState"), "dirty")
        self.assertEqual(install.plist_value(debug, "SofDevToolChannel"), "debug")
        self.assertNotEqual(
            hashlib.sha256((debug / "Contents/Resources/AppIcon.icns").read_bytes()).hexdigest(),
            hashlib.sha256((release / "Contents/Resources/AppIcon.icns").read_bytes()).hexdigest(),
        )
        subprocess.run(
            ["/usr/bin/iconutil", "-c", "iconset", str(release / "Contents/Resources/AppIcon.icns"),
             "-o", str(self.root / "decoded-release.iconset")],
            check=True,
        )
        for bundle in (debug, release):
            self.assertIn("Catppuccin", (bundle / "Contents/Resources/APP_NOTICES.md").read_text())
            self.assertIn("@pierre/diffs", (bundle / "Contents/Resources/THIRD_PARTY_NOTICES.md").read_text())
        self.assertEqual(
            hashlib.sha256((package.MACOS / "icons/release.iconset/icon_512x512@2x.png").read_bytes()).hexdigest(),
            "7827889df51dd46823dd6613b467b62df9c6f7ee40f1ec45506c43387b6a7038",
        )
        self.assertEqual(
            hashlib.sha256((package.MACOS / "icons/debug.iconset/icon_512x512@2x.png").read_bytes()).hexdigest(),
            "d989c8cacec1a31ab97e8e695fe0de6cdf5a6952575067e2ef5cfda38c063c01",
        )

    def test_temporary_install_replaces_only_verified_release_bundle(self):
        release = self.bundle("release")
        debug = self.bundle("debug")
        destination_root = self.root / "Applications"
        destination_root.mkdir()
        neighbor = destination_root / "Other.app"
        neighbor.mkdir()
        (neighbor / "sentinel").write_text("untouched")
        debug_destination = destination_root / debug.name
        debug_destination.mkdir()
        (debug_destination / "sentinel").write_text("debug untouched")

        installed = install.install_bundle(release, destination_root)
        self.assertEqual(install.plist_value(installed, "SofDevToolChannel"), "release")
        (installed / "obsolete").write_text("old bundle only")
        installed = install.install_bundle(release, destination_root)
        self.assertFalse((installed / "obsolete").exists())
        self.assertEqual((neighbor / "sentinel").read_text(), "untouched")
        self.assertEqual((debug_destination / "sentinel").read_text(), "debug untouched")
        with self.assertRaises(ValueError):
            install.install_bundle(debug, destination_root)

        conflicting_root = self.root / "conflicting-Applications"
        conflicting_bundle = conflicting_root / "SofDevTool.app"
        conflicting_bundle.mkdir(parents=True)
        (conflicting_bundle / "sentinel").write_text("foreign bundle")
        with self.assertRaises(ValueError):
            install.install_bundle(release, conflicting_root)
        self.assertEqual((conflicting_bundle / "sentinel").read_text(), "foreign bundle")

    def test_failed_repackage_preserves_previous_bundle(self):
        release = self.bundle("release")
        original = (release / "Contents/Info.plist").read_bytes()
        with self.assertRaises(FileNotFoundError):
            package.assemble(
                "release", self.root / "missing-binary", self.root / "artifacts",
                "0.2.0", "43", REVISION, "dirty",
            )
        self.assertEqual((release / "Contents/Info.plist").read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
