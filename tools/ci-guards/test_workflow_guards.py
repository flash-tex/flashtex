"""CI steps must not pass without testing or skip silently (#2116).

Line-based reads of .github/workflows (no YAML library on every runner):

- a job that runs only when a self-hosted runner variable is '1' has, in the
  same workflow, a job for the variable unset (a hosted fallback or a job
  that says nothing ran), so a run without the runners is not silently green;
- sweeps-unicode's reopen leg is red when a pull request removes or renames
  tools/incr-bench/unicode_reopen.py (its absence otherwise passes);
- unicode-m3 and unicode-m3-bundle trigger on the tools they build and run.

FLASHTEX_WORKFLOWS_DIR points the checks at another directory (to show they
fail on the workflows before the fixes).
"""

import os
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = Path(os.environ.get("FLASHTEX_WORKFLOWS_DIR", ROOT / ".github" / "workflows"))

JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
JOB_IF = re.compile(r"^    if:\s*(.*)$")


def jobs(text):
    """(name, `if` expression or '') for each job, in order."""
    out, in_jobs, cur = [], False, None
    for line in text.splitlines():
        if re.match(r"^[A-Za-z]", line):
            in_jobs = line.startswith("jobs:")
            continue
        if not in_jobs:
            continue
        m = JOB.match(line)
        if m:
            cur = [m.group(1), ""]
            out.append(cur)
            continue
        m = JOB_IF.match(line)
        if m and cur is not None:
            cur[1] = m.group(1)
    return [tuple(j) for j in out]


def needs_selfhosted(cond):
    return "FLASHTEX_SELFHOSTED" in cond and "== '1'" in cond and "!" not in cond


def runs_without_selfhosted(cond):
    return "FLASHTEX_SELFHOSTED" in cond and ("!= '1'" in cond or "!(" in cond)


def paths_blocks(text):
    """The `paths:` lists, one per trigger, as lists of entries."""
    blocks, cur = [], None
    for line in text.splitlines():
        if re.match(r"^    paths:\s*$", line):
            cur = []
            blocks.append(cur)
            continue
        if cur is not None:
            m = re.match(r'^      - "?([^"]+)"?\s*$', line)
            if m:
                cur.append(m.group(1))
            else:
                cur = None
    return blocks


class SelfHostedJobsSayWhenTheyDoNotRun(unittest.TestCase):
    def test_every_selfhosted_only_job_has_a_job_for_the_variable_unset(self):
        missing = []
        for wf in sorted(WORKFLOWS.glob("*.yml")):
            js = jobs(wf.read_text())
            fallback = any(runs_without_selfhosted(c) for _, c in js)
            for name, cond in js:
                if needs_selfhosted(cond) and not fallback:
                    missing.append(f"{wf.name}: {name} ({cond})")
        self.assertEqual(missing, [], "skipped silently without the self-hosted runners")

    def test_the_parser_sees_the_jobs(self):
        # guard against the check passing because it read nothing
        js = jobs((WORKFLOWS / "memory-safety.yml").read_text())
        self.assertTrue(any(needs_selfhosted(c) for _, c in js), js)
        self.assertTrue(any(runs_without_selfhosted(c) for _, c in js), js)


class ReopenLegIsRedWhenItsScriptGoes(unittest.TestCase):
    def test_a_pull_request_that_removes_the_script_fails(self):
        text = (WORKFLOWS / "sweeps-unicode.yml").read_text()
        i = text.index("if [[ ! -f tools/incr-bench/unicode_reopen.py ]]")
        block = text[i : text.index("exit 0", i)]
        self.assertIn('pulls/$PR/files', block)
        self.assertIn("exit 1", block)
        self.assertIn("pull-requests: read", text)


class ReportOnlyStepsWarnWhenTheyFail(unittest.TestCase):
    """unicode-m3 is met (DESIGN §12), so its report-only tool runs do not
    gate; a failure warns and goes to the job summary (ruling on #2116)."""

    def test_no_tool_run_swallows_its_failure(self):
        for wf in ("unicode-m3.yml", "unicode-m3-bundle.yml"):
            text = (WORKFLOWS / wf).read_text()
            swallowed = [l.strip() for l in text.splitlines() if "| tee " in l and "|| true" in l]
            self.assertEqual(swallowed, [], f"{wf}: a tool's failure ends in || true")
            for l in text.splitlines():
                if "report-only.sh" in l and not l.lstrip().startswith("#"):
                    self.assertRegex(l, r'report-only\.sh" \S+ \$\? ', f"{wf}: {l.strip()}")
            self.assertIn("report-only.sh", text, f"{wf}: no report-only step read")

    def test_report_only_warns_and_writes_the_summary(self):
        import subprocess
        import tempfile

        with tempfile.NamedTemporaryFile("r", suffix=".md") as summary:
            r = subprocess.run(
                [str(ROOT / "scripts" / "ci" / "report-only.sh"), "visual", "3", "pixel counts"],
                env={**os.environ, "GITHUB_STEP_SUMMARY": summary.name},
                capture_output=True,
                text=True,
            )
            self.assertEqual(r.returncode, 0)
            self.assertIn("::warning::visual exited 3", r.stdout)
            self.assertIn("visual exited 3", summary.read())


class PathFiltersCoverWhatTheJobsRun(unittest.TestCase):
    def check(self, wf, want):
        blocks = paths_blocks((WORKFLOWS / wf).read_text())
        self.assertTrue(blocks, f"{wf}: no paths filter read")
        for b in blocks:
            for w in want:
                self.assertIn(w, b, f"{wf}: a paths filter lacks {w}")

    def test_unicode_m3(self):
        self.check(
            "unicode-m3.yml",
            ["tools/xetex-lockstep/**", "tools/xetex-pdfparity/**", "crates/display-list-v3/**"],
        )

    def test_unicode_m3_bundle(self):
        self.check("unicode-m3-bundle.yml", ["tools/bundle/texbundle.py"])


if __name__ == "__main__":
    unittest.main()
