import copy
import hashlib
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import json
import unittest
from unittest.mock import patch

import verify_bundle_resources as verifier


class BundleResources(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.fd = os.open(self.root, os.O_RDONLY | os.O_DIRECTORY)
        self.entry = {"path": "texmf/fonts/test.tfm", "byte_length": 4,
                      "sha256": hashlib.sha256(b"TFM!").hexdigest()}
        self.path = self.root / self.entry["path"]
        self.path.parent.mkdir(parents=True)
        self.path.write_bytes(b"TFM!")

    def tearDown(self):
        os.close(self.fd)
        self.temp.cleanup()

    def check(self):
        return verifier.check_resource(self.fd, self.entry)["status"]

    def test_exact_file_read_only(self):
        before = self.path.stat()
        self.assertEqual(self.check(), "verified")
        self.assertEqual(self.path.read_bytes(), b"TFM!")
        self.assertEqual(self.path.stat().st_mtime_ns, before.st_mtime_ns)

    def test_missing_and_misrooted_refused(self):
        self.path.rename(self.root / "test.tfm")
        self.assertEqual(self.check(), "missing")

    def test_same_length_corruption_and_wrong_length(self):
        self.path.write_bytes(b"BAD!")
        self.assertEqual(self.check(), "digest_mismatch")
        self.path.write_bytes(b"BAD")
        self.assertEqual(self.check(), "length_mismatch")

    def test_file_and_directory_symlinks_refused(self):
        self.path.unlink()
        self.path.symlink_to(self.root / "elsewhere")
        self.assertEqual(self.check(), "unsafe_or_unreadable")
        self.path.unlink()
        self.path.parent.rmdir()
        self.path.parent.symlink_to(self.root, target_is_directory=True)
        self.assertEqual(self.check(), "unsafe_or_unreadable")

    def test_fifo_does_not_block(self):
        self.path.unlink()
        os.mkfifo(self.path)
        self.assertEqual(self.check(), "not_regular")

    def test_manifest_unsafe_duplicate_and_budget_paths(self):
        manifest = verifier.pinned_manifest()
        for path in ["/absolute", "../escape", "a/../b", "a//b", "a/./b", "a\\b", "a\x00b"]:
            candidate = copy.deepcopy(manifest)
            candidate["required_for_existing_fixtures"][0]["path"] = path
            with self.assertRaises(ValueError):
                verifier.validate_entries(candidate)
        for size in [True, -1, 0, verifier.MAX_FILE + 1]:
            candidate = copy.deepcopy(manifest)
            candidate["required_for_existing_fixtures"][0]["byte_length"] = size
            with self.assertRaises(ValueError):
                verifier.validate_entries(candidate)
        manifest["required_for_existing_fixtures"].append(manifest["required_for_existing_fixtures"][0])
        with self.assertRaises(ValueError):
            verifier.validate_entries(manifest)

    def test_manifest_removal_cannot_claim_pinned_coverage(self):
        path = self.root / "manifest.json"
        path.write_text('{"schema_version":1,"required_for_existing_fixtures":[]}')
        with patch.object(verifier, "MANIFEST", path):
            with self.assertRaisesRegex(ValueError, "pinned manifest mismatch"):
                verifier.verify(self.root)

    def test_cli_resource_and_setup_refusal_exit_codes(self):
        for root, code in [(self.root, 1), (self.root / "missing-root", 2)]:
            result = subprocess.run([sys.executable, verifier.__file__, str(root)],
                                    capture_output=True, timeout=5)
            self.assertEqual(result.returncode, code)
            self.assertFalse(json.loads(result.stdout)["native_discovery_executed"])

    def test_duplicate_manifest_keys_refused(self):
        with self.assertRaises(ValueError):
            verifier.unique_object([("path", "a"), ("path", "b")])


if __name__ == "__main__":
    unittest.main()
