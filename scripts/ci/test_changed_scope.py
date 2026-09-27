"""Tests for changed-scope.py (run: python3 -m unittest discover -s scripts/ci)."""
import importlib.util
import pathlib
import unittest

_spec = importlib.util.spec_from_file_location(
    "changed_scope", pathlib.Path(__file__).with_name("changed-scope.py"))
cs = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(cs)

FULL_STANDALONE = [{"os": o, "crate": c} for o in ("ubuntu-latest", "macos-15")
                   for c in ("render-pipeline", "flashtex-cli")]


class ScopeTests(unittest.TestCase):
    def check(self, paths, rust, mac, ios, full=False):
        s = cs.scope(paths)
        self.assertEqual((s["rust"], s["mac"], s["ios"]), (rust, mac, ios), paths)
        if full:
            self.assertEqual(s["rust_os"], ["ubuntu-latest", "macos-15"])
            self.assertEqual(s["standalone_matrix"]["include"], FULL_STANDALONE)
        else:
            self.assertEqual(s["rust_os"], ["ubuntu-latest"])
            self.assertEqual(s["standalone_matrix"]["include"], [
                {"os": "ubuntu-latest", "crate": "render-pipeline"},
                {"os": "ubuntu-latest", "crate": "flashtex-cli"},
                {"os": "macos-15", "crate": "flashtex-cli"},
            ])

    def test_non_pull_request_runs_everything(self):
        s = cs.scope(["crates/compiler/src/lib.rs"], run_all=True)
        self.assertTrue(s["rust"] and s["mac"] and s["ios"])
        self.assertEqual(s["standalone_matrix"]["include"], FULL_STANDALONE)

    def test_empty_diff_fails_open(self):
        self.check([], True, True, True, full=True)
        self.check(["", "  "], True, True, True, full=True)

    def test_rust_only_pr_skips_ipad_keeps_mac_app(self):
        self.check(["crates/compiler/src/lib.rs", "fixtures/x/main.tex"], True, True, False)

    def test_mac_swift_only_pr_skips_rust_and_ipad(self):
        self.check(["apps/mac/Sources/FlashTeXMac/Shell.swift",
                    "apps/mac/Tests/FlashTeXMacTests/ShellTests.swift"], False, True, False)

    def test_shared_swift_sources_reach_ipad(self):
        for p in ("apps/mac/Sources/FlashTeXProtocol/Messages.swift",
                  "apps/mac/Sources/FlashTeXEditorCore/Buffer.swift",
                  "apps/mac/tools/nearby-client/Sources/NearbyClient/Client.swift"):
            self.check([p], False, True, True)

    def test_ipad_only_pr_skips_rust_and_mac_app(self):
        self.check(["apps/ios/FlashTeXPad/ContentView.swift"], False, False, True)

    def test_paths_rust_tests_read_keep_rust(self):
        for p in ("apps/mac/Fonts/texmf/fonts/tfm/public/lm/x.tfm",
                  "apps/mac/scripts/sync-supported-latex.sh",
                  "apps/mac/docs/package-editing.md",
                  "apps/mac/Samples/demo.tex",
                  "docs/ci-cd.md", "Cargo.lock", "tools/parity/baseline-fixtures.json"):
            self.check([p], True, True, False)

    def test_ci_changes_run_everything(self):
        for p in cs.RUN_ALL:
            self.check([p], True, True, True, full=True)

    def test_output_format(self):
        import contextlib, io, sys
        buf = io.StringIO()
        old = sys.stdin
        sys.stdin = io.StringIO("apps/ios/a.swift\n")
        try:
            with contextlib.redirect_stdout(buf):
                cs.main(["changed-scope.py"])
        finally:
            sys.stdin = old
        lines = dict(l.split("=", 1) for l in buf.getvalue().splitlines())
        self.assertEqual(lines["rust"], "false")
        self.assertEqual(lines["ios"], "true")
        self.assertEqual(lines["rust_os"], '["ubuntu-latest"]')


if __name__ == "__main__":
    unittest.main()
