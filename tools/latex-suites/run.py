#!/usr/bin/env python3
"""LaTeX suites (latex2e base/required, latex3 l3kernel) vs any engine binary.

Runs `l3build check -e pdftex` per suite dir with a recording shim `pdftex`
first on PATH resolving to the engine under test, so the SAME l3build
normalisation compares each log against the committed upstream `.tlg`.
Exit 0 iff every failure is an ordinary diff mismatch listed in
EXPECTED-FAILURES.txt (with a matching recorded diff hash, when the entry
has one); 1 on unexpected failures, crashed/unresolved tests, requested
tests that never ran, or stale entries (unless --allow-stale); 2 on infra
errors. Stdlib only.

Safety rules (all validated against the reference engine):
- Suite dirs run SEQUENTIALLY: every latex2e dir shares <repo>/build
  (maindir build root, incl. build/local installs), so any overlap
  corrupts results. There is deliberately no --jobs flag.
- l3build runs with stdin from /dev/null in its own process group:
  unpack/format steps invoke the engine WITHOUT -interaction=nonstopmode,
  so a broken engine (or broken unpack output) drops pdftex into an
  errorstopmode `Please type another input file name:` prompt on stdin.
  With an inherited open stdin that prompt blocks forever at 0% CPU
  (observed 1.5 h hang); from /dev/null it emergency-stops at once.
- Each directory gets a timeout (default 1800 s, --timeout-dir SECONDS):
  on expiry the whole process group is killed and every unfinished test
  is FAIL [UNEXPECTED] (timeout). l3build offers no per-test timeout
  flag, so the directory timeout is the enforcement point.
- A test whose engine process dies (crash, kill, exit code other than
  pdfTeX's 0/1) is FAIL, never PASS: l3build turns this into a Lua
  assertion abort with no `--> failed` line and no .diff, so run.py
  records every engine exit in a call log and marks unresolved tests.
- `l3build clean` runs per directory before check, so a stale build/
  result can never be read as this run's result.
- The default reference is pdfTeX 1.40.29 (`<engine> --version` first
  line must contain it as a whole token, so 1.40.290 is refused);
  `--allow-any-engine` bypasses with a warning.
"""

import argparse
import hashlib
import os
import queue
import re
import shlex
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import threading
import time

ROOT = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(ROOT, ".cache")
EXPECTED = os.path.join(ROOT, "EXPECTED-FAILURES.txt")

REF_ENGINE_VERSION = "1.40.29"

DEFAULT_TIMEOUT_DIR = 1800
KILL_GRACE = 5.0  # seconds between SIGTERM and SIGKILL of a timed-out group

SUITE_DIRS = {
    "base": [("latex2e", "base")],
    "required": [("latex2e", "required/" + d) for d in
                 ("cyrillic", "graphics", "tools", "amsmath", "firstaid",
                  "latex-lab")],
    "l3kernel": [("latex3", "l3kernel")],
}
SUITE_DIRS["all"] = SUITE_DIRS["base"] + SUITE_DIRS["required"] + \
    SUITE_DIRS["l3kernel"]

PROGRESS = re.compile(r"^  (\S+) \((\d+)/(\d+)\)\s*$")
DIFFLINE = re.compile(r"^\s+-\s+\S*?([^/ ]+)\.pdftex\.diff\s*$")
NEWCONFIG = re.compile(r"^Running l3build with target")
CHECKDONE = re.compile(r"^\s*(?:Check failed with|Failed tests for "
                          r"configuration)")
ALLPASSED = "All checks passed"
JOBNAME = re.compile(r"(?:^|\s)-jobname=([^\s]+)")
CALLLINE = re.compile(r"^rc=(-?\d+) argv=(.*)$")


SHA_TOKEN = re.compile(r"sha256=([0-9a-fA-F]{64})(?![0-9a-fA-F])")
REF_VERSION_RE = re.compile(r"(?<![\d.])" + re.escape(REF_ENGINE_VERSION) +
                            r"(?![\d.])")


def load_expected(path):
    """{test: (sha_or_None, reason)}; old `<name>: <reason>` lines parse
    with sha None (backwards compatible), new lines carry the reference
    diff hash as `<name>: sha256=<hex> <reason>`."""
    expected = {}
    if os.path.exists(path):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line and line[0] != "#":
                    name, _, rest = line.partition(":")
                    rest = rest.strip()
                    sha = None
                    m = SHA_TOKEN.match(rest)
                    if m:
                        sha = m.group(1).lower()
                        rest = rest[m.end():].strip()
                    expected[name.strip()] = (sha, rest)
    return expected


def update_baseline_file(path, updates):
    """Refresh sha256= tokens for entries in updates; return count.

    Comment/blank/unknown lines pass through unchanged; entries with no
    observed reference diff keep their old line; names without a line are
    never invented (a new failure needs a human reason first).
    """
    with open(path, encoding="utf-8") as fh:
        text = fh.read()
    out, n = [], 0
    for line in text.splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#") and ":" in stripped:
            name, _, rest = stripped.partition(":")
            name = name.strip()
            if name in updates:
                rest = rest.strip()
                m = SHA_TOKEN.match(rest)
                if m:
                    rest = rest[m.end():].strip()
                line = "%s: sha256=%s%s" % (name, updates[name],
                                            " " + rest if rest else "")
                n += 1
        out.append(line)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(out) + "\n" if out else "")
    return n


DIFF_FILE_HEADER = re.compile(rb"^(?:\*\*\*|---|\+{3}) (?![0-9])")


def normalise_diff(data):
    """Drop volatile diff file-header lines; normalise whitespace/newlines.

    l3build writes normal-format diffs whose `*** path<TAB>date` /
    `--- path<TAB>date` headers embed the run date; hunk headers
    (`*** 4,10 ****`, `--- 4,10 ----`: digit after the marker) are real
    content and stay. Unified `--- a/..` / `+++ b/..` headers drop too.
    """
    kept = []
    for line in data.split(b"\n"):
        line = line.rstrip(b" \t\r")
        if DIFF_FILE_HEADER.match(line):
            continue
        kept.append(line)
    while kept and not kept[-1]:
        kept.pop()
    return b"\n".join(kept) + (b"\n" if kept else b"")


def hash_diff_files(paths):
    """SHA-256 over the normalised concatenation (sorted paths)."""
    h = hashlib.sha256()
    for path in sorted(paths):
        with open(path, "rb") as fh:
            h.update(normalise_diff(fh.read()))
    return h.hexdigest()


def _build_roots(workdir):
    """Candidate l3build build/ dirs: workdir, then up to 4 ancestors.

    Every latex2e dir shares <repo>/build via maindir, so the .diffs of a
    run usually sit above the workdir (e.g. required/tools -> latex2e/).
    """
    roots = []
    d = os.path.abspath(workdir)
    for _ in range(5):
        cand = os.path.join(d, "build")
        if os.path.isdir(cand):
            roots.append(cand)
        parent = os.path.dirname(d)
        if parent == d:
            break
        d = parent
    return roots


def _walk_diffs(root):
    """Sorted *.pdftex.diff paths under build/test* dirs only."""
    hits = []
    try:
        top = sorted(os.listdir(root))
    except OSError:
        return hits
    for name in top:
        if name != "test" and not name.startswith(("test-", "test_")):
            continue
        for dirpath, _dirnames, filenames in os.walk(
                os.path.join(root, name)):
            for fn in filenames:
                if fn.endswith(".pdftex.diff"):
                    hits.append(os.path.join(dirpath, fn))
    return sorted(hits)


def snapshot_diffs(workdir):
    """{abspath: (mtime_ns, size)} for every visible *.pdftex.diff."""
    snap = {}
    for root in _build_roots(workdir):
        for path in _walk_diffs(root):
            try:
                st = os.stat(path)
            except OSError:
                continue
            snap[path] = (st.st_mtime_ns, st.st_size)
    return snap


def collect_diffs(workdir, before):
    """{test: [paths]} for .diffs new-or-changed since `before`.

    A directory's `l3build clean` does NOT wipe the whole shared
    <repo>/build (other dirs' and older runs' .diffs linger), so a blind
    glob would hash stale diffs; only files created or rewritten after
    the snapshot count.
    """
    fresh = {}
    for root in _build_roots(workdir):
        for path in _walk_diffs(root):
            try:
                st = os.stat(path)
            except OSError:
                continue
            if before.get(path) == (st.st_mtime_ns, st.st_size):
                continue
            test = os.path.basename(path)[:-len(".pdftex.diff")]
            fresh.setdefault(test, []).append(path)
    return fresh


def keep_run_diffs(dest, label, difffiles, failed):
    """Copy this run's .diffs of failed tests into dest; return count.

    Called before the next directory's `l3build clean` (all latex2e dirs
    share one build/, so a later clean deletes earlier .diffs). Same-name
    failures across configs keep their parent (config) dir as an infix.
    """
    os.makedirs(dest, exist_ok=True)
    prefix = label.replace("/", "_")
    n = 0
    for t in sorted(failed):
        paths = difffiles.get(t, ())
        for path in paths:
            name = "%s__%s" % (prefix, t)
            if len(paths) > 1:
                name += "__" + os.path.basename(os.path.dirname(path))
            try:
                shutil.copyfile(path, os.path.join(
                    dest, name + ".pdftex.diff"))
            except OSError:
                continue
            n += 1
    return n


def list_tests(workdir):
    testdir = os.path.join(workdir, "testfiles")
    return sorted(f[:-4] for f in os.listdir(testdir) if f.endswith(".lvt"))


ENGINE_ENV_KEY_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def parse_engine_env(pairs):
    """{KEY: VALUE} from repeatable `--engine-env KEY=VALUE` flags.

    Raises ValueError on a malformed pair (no `=`, an empty key, or a
    key outside `[A-Za-z_][A-Za-z0-9_]*`); main() turns that into an
    exit-2 argparse error. The key rule matters because the key is
    inserted unquoted into the shim's `export KEY=VALUE` line, where a
    `;` would run as a shell command. An empty value (`KEY=`) is
    allowed; values stay shlex-quoted in the shim so shell metacharacters
    (`;`, spaces, `$`) arrive literally.
    """
    env = {}
    for pair in pairs or []:
        key, sep, value = pair.partition("=")
        if not sep or not key:
            raise ValueError("malformed --engine-env %r: expected "
                             "KEY=VALUE with a non-empty KEY" % pair)
        if not ENGINE_ENV_KEY_RE.fullmatch(key):
            raise ValueError("invalid --engine-env key %r: must match "
                             "[A-Za-z_][A-Za-z0-9_]*" % key)
        env[key] = value
    return env


def scrub_flashtex(env):
    """Drop every `FLASHTEX_*` entry from an environment dict (in place).

    Only values given with `--engine-env` may supply `FLASHTEX_*` to the
    engine under test: a variable the parent shell happens to export
    (e.g. an unflagged `FLASHTEX_OTHER`, or the shell's
    `FLASHTEX_FORMATS` reaching the reference run) must never leak
    through run.py's environments.
    """
    for key in [k for k in env if k.startswith("FLASHTEX_")]:
        del env[key]
    return env


def engine_version_firstline(engine, extra_env=None):
    """First stdout line of `<engine> --version`, or None if unusable.

    Content-based: an exit code is NOT required, so a shim that runs the
    real engine and exits nonzero still passes when it prints the real
    banner; a binary that dies silently (or prints a foreign banner)
    yields None/wrong line and is refused. The probe environment is the
    inherited one scrubbed of every `FLASHTEX_*` (an unflagged parent
    variable must not reach even `--version`); `extra_env` (e.g. the
    --engine-env dict) is added back on top, so a candidate probe may
    see exactly the flagged values.
    """
    env = scrub_flashtex(dict(os.environ))
    if extra_env:
        env.update(extra_env)
    try:
        proc = subprocess.run([engine, "--version"], capture_output=True,
                              text=True, timeout=60, stdin=subprocess.DEVNULL,
                              env=env)
    except (OSError, subprocess.SubprocessError):
        return None
    if not proc.stdout:
        return None
    lines = proc.stdout.splitlines()
    return lines[0] if lines else None


def reference_check(engine, allow_any, extra_env=None):
    """(code, message): code 0 proceeds, 2 refuses; message is warning/error.

    The version must match as a whole token: a substring test accepts a
    fake `1.40.290` engine as the 1.40.29 reference. `extra_env` reaches
    the version probe (the gate also runs on a candidate engine, which
    may need its --engine-env variables); the reference probe runs with
    None and never sees them.
    """
    vline = engine_version_firstline(engine, extra_env)
    if vline is not None and REF_VERSION_RE.search(vline):
        if allow_any:
            return (0, "warning: --allow-any-engine passed; engine reports "
                    "reference pdfTeX %s anyway (%s)"
                    % (REF_ENGINE_VERSION, vline.strip()))
        return (0, None)
    msg = ("engine %s is not the reference: `--version` first line %r does "
           "not contain %s as a whole token (reference pdfTeX 1.40.29)"
           % (engine, vline, REF_ENGINE_VERSION))
    if allow_any:
        return (0, "warning: " + msg + "; proceeding anyway")
    return (2, "error: " + msg + "; pass --allow-any-engine to test a "
            "candidate engine anyway")


def make_shim(engine, calllog, engine_env=None):
    """Dir with a recording `pdftex` wrapper around the engine under test.

    Every invocation's exit code + argv is appended to calllog, so an
    engine death l3build swallows (Lua assert, no diff) still maps to the
    test named by `-jobname=`. Exit code and stdio pass through unchanged.
    `engine_env` ({KEY: VALUE} from --engine-env) is exported by the shim
    just before it execs the engine under test, so ONLY the engine under
    test sees it -- never l3build, never the reference probe, never the
    reference engine (which runs without the flag). With no engine_env
    the shim script is exactly the historical one.
    """
    shimdir = tempfile.mkdtemp(prefix="latex-suites-shim-")
    path = os.path.join(shimdir, "pdftex")
    exports = "".join("export %s=%s\n" % (k, shlex.quote(v))
                      for k, v in (engine_env or {}).items())
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("#!/bin/sh\nCALLLOG=%s\n%s%s \"$@\"\nrc=$?\n"
                 "printf 'rc=%%d argv=%%s\\n' \"$rc\" \"$*\" >> \"$CALLLOG\"\n"
                 "exit \"$rc\"\n"
                 % (shlex.quote(calllog), exports, shlex.quote(engine)))
    os.chmod(path, os.stat(path).st_mode |
             stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return shimdir


def parse_l3build_log(lines):
    """Split l3build stdout into (ran, completed, failed).

    ran: every test l3build started (`  name (i/n)`), in order.
    completed: started tests with pass evidence — a later test started
      (so the previous one finished without `--> failed`) or an
      `All checks passed` trailer. A test that only ever started never
      counts as passed.
    failed: tests with a `--> failed` line or a `.pdftex.diff` entry.
    `current` resets at config boundaries, so config-level lines such as
    `Skipping unknown engine pdftex` (whole-config, not per-test: this
    l3build emits no per-test skip lines) can never attach to a test.
    """
    ran, completed, failed = [], set(), set()
    current = None
    for raw in lines:
        line = raw.rstrip("\n")
        if NEWCONFIG.match(line):
            current = None
            continue
        m = PROGRESS.match(line)
        if m:
            if current is not None and current not in failed:
                completed.add(current)
            current = m.group(1)
            ran.append(current)
            continue
        if "--> failed" in line:
            if current is not None:
                failed.add(current)
            continue
        m = DIFFLINE.match(line)
        if m:
            failed.add(m.group(1))
            continue
        if ALLPASSED in line or CHECKDONE.match(line):
            # Reached a per-config summary: every started test got its
            # verdict (each failure prints `--> failed` in the loop), so
            # the in-flight test, if not failed, passed.
            if current is not None:
                if current not in failed:
                    completed.add(current)
                current = None
            continue
    return ran, completed, failed


def engine_deaths(calllog):
    """Tests whose own typeset run died: engine exit not in (0, 1).

    pdfTeX exits 0 on success and 1 on TeX-level errors (which l3build
    judges by log comparison); any other code — crash, kill signal,
    foreign shim exit — means the process never completed that test.
    Only `-jobname=` invocations map to a test; unpack/format runs do not.
    """
    deaths = set()
    try:
        with open(calllog, encoding="utf-8") as fh:
            for raw in fh:
                m = CALLLINE.match(raw.rstrip("\n"))
                if not m or int(m.group(1)) in (0, 1):
                    continue
                j = JOBNAME.search(m.group(2))
                if j:
                    deaths.add(j.group(1))
    except OSError:
        pass
    return deaths


def attribute(ran, completed, failed, deaths, rc, requested):
    """Apply crash/staleness rules; return (failed, notes).

    - Engine deaths from the call log always fail their test.
    - Any started-but-unresolved test (no pass evidence, no verdict)
      fails: with rc != 0 l3build aborted mid-run (Lua assert on the
      missing .log when the engine wrote nothing, format-stage death).
    - rc != 0 with nothing started (engine dead in unpack/format) fails
      every requested test: none of them passed.
    """
    failed = set(failed)
    notes = {}
    for t in sorted(deaths):
        if t not in failed:
            notes[t] = "engine process died (exit not 0/1)"
        failed.add(t)
    unresolved = [t for t in ran if t not in completed and t not in failed]
    if rc != 0:
        for t in unresolved:
            notes.setdefault(t, "no verdict: l3build aborted (rc %d)" % rc)
            failed.add(t)
        if not ran:
            for t in requested:
                notes.setdefault(t, "check never ran: l3build aborted "
                                 "(rc %d)" % rc)
                failed.add(t)
    else:
        for t in unresolved:
            notes.setdefault(t, "no pass evidence though l3build exited 0")
            failed.add(t)
    return failed, notes


def _pump(pipe, q):
    """Reader-thread target: forward lines, then a None EOF sentinel."""
    try:
        for line in pipe:
            q.put(line)
    except Exception:
        pass
    finally:
        q.put(None)


def _kill_tree(proc):
    """SIGTERM the process group, then ALWAYS SIGKILL survivors; reap.

    The SIGKILL is unconditional, not only when proc.wait() times out:
    the group leader (l3build) usually exits on SIGTERM within
    KILL_GRACE, so proc.wait() succeeds while a SIGTERM-ignoring
    survivor (a hung engine holding the stdout pipe) is still alive --
    without the SIGKILL it holds the pipe open forever and run_capture
    never returns. With start_new_session=True the group id equals
    proc.pid, so killpg(proc.pid, ...) reaches survivors even after the
    leader has exited and been reaped. A setsid-detached grandchild (its
    own session) is NOT in our group: it cannot be killed here and is
    only reaped by the OS; run_capture bounds the wait for its pipe.
    """
    try:
        if hasattr(os, "killpg"):
            os.killpg(proc.pid, signal.SIGTERM)
        else:
            proc.terminate()
    except OSError:
        pass
    try:
        proc.wait(timeout=KILL_GRACE)
    except subprocess.TimeoutExpired:
        pass
    try:
        if hasattr(os, "killpg"):
            os.killpg(proc.pid, signal.SIGKILL)
        else:
            proc.kill()
    except OSError:
        pass
    proc.wait()


def _join_reader_before_close(proc, reader, grace=10):
    """Bounded reader join; close our pipe end only if the reader is done.

    Never close proc.stdout while the _pump thread can still be blocked
    in read(): the close deadlocks against it for as long as any live
    process holds the pipe open. After _kill_tree, in-group survivors
    are dead so the reader reaches EOF promptly and the close is safe;
    if the reader is still alive after `grace` seconds, a setsid-detached
    grandchild (unkillable by our process-group kill, reaped only by the
    OS) still holds the pipe -- leave our end open and return with the
    output collected so far instead of hanging.
    """
    reader.join(timeout=grace)
    if not reader.is_alive():
        try:
            proc.stdout.close()
        except OSError:
            pass


def run_capture(cmd, cwd, env, timeout, on_line=None):
    """Run cmd with stdin /dev/null in its own process group.

    stdout+stderr stream to on_line(line) live via a reader thread while
    the main thread enforces `timeout` seconds. Returns (rc, lines,
    timed_out); on timeout (or Ctrl-C) the whole process group is killed
    and reaped before returning/raising. The kill is SIGTERM, then
    unconditional SIGKILL after KILL_GRACE (a SIGTERM-ignoring engine
    cannot hang the runner), and the pipe is never closed while the
    reader thread is still blocked, so return is bounded by
    timeout + KILL_GRACE + a few seconds even when a survivor holds the
    pipe. A setsid-detached grandchild is outside the process group and
    cannot be killed here -- it is reaped only by the OS.
    """
    proc = subprocess.Popen(cmd, cwd=cwd, env=env,
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, bufsize=1,
                            start_new_session=True)
    q = queue.Queue()
    reader = threading.Thread(target=_pump, args=(proc.stdout, q),
                              daemon=True)
    lines, timed_out = [], False
    deadline = time.monotonic() + timeout
    try:
        reader.start()
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                timed_out = True
                break
            try:
                item = q.get(timeout=remaining)
            except queue.Empty:
                timed_out = True
                break
            if item is None:
                break
            lines.append(item)
            if on_line:
                on_line(item)
    except KeyboardInterrupt:
        _kill_tree(proc)
        _join_reader_before_close(proc, reader)
        raise
    if timed_out:
        _kill_tree(proc)
    else:
        try:
            proc.wait(timeout=60)
        except subprocess.TimeoutExpired:
            # Output done but process lingers (e.g. a detached grandchild
            # holding nothing): do not hang, kill and call it a timeout.
            timed_out = True
            _kill_tree(proc)
    _join_reader_before_close(proc, reader)
    # Keep late forensic output without blocking: whatever arrived.
    while True:
        try:
            item = q.get_nowait()
        except queue.Empty:
            break
        if item is None:
            break
        lines.append(item)
        if on_line:
            on_line(item)
    return proc.returncode, lines, timed_out


def run_l3build(workdir, names, engine, logpath,
                timeout=DEFAULT_TIMEOUT_DIR, l3build_exe="l3build",
                engine_env=None):
    """Clean then `l3build check`; return (exitcode, ran, failed, notes,
    timedout, info) where timedout names tests failed by the directory
    timeout and info holds {"diffhash": {test: sha256 of this run's
    normalised .diff}, "deaths": {tests whose engine died}, "neverran":
    {requested tests with no transcript entry}, "difffiles": {test:
    [.diff paths]}}.

    Explicitly requested tests (names) that never appear in the transcript
    FAIL as never-ran -- even when l3build exits 0 -- so a silent l3build
    can never report PASS 0 / FAIL 0 for requested tests. Unfiltered runs
    (names empty) are exempt: e.g. graphics legitimately runs 0 tests.
    `engine_env` reaches the engine under test ONLY, via the shim's
    exports; EVERY `FLASHTEX_*` variable is scrubbed from l3build's own
    environment (even ones the parent shell exports that were never
    flagged) so l3build, the shim, and everything they spawn except the
    engine under test never see them.
    """
    fd, calllog = tempfile.mkstemp(prefix="latex-suites-calls-")
    os.close(fd)
    shimdir = make_shim(engine, calllog, engine_env)
    try:
        env = scrub_flashtex(dict(os.environ))
        env["PATH"] = shimdir + os.pathsep + env.get("PATH", "")
        # Start pristine: committed results must not depend on earlier
        # installs (e.g. required/tools' multicol.sty lingering in
        # build/local flips github-1336, which expects it to be absent).
        # This is also the staleness guard: a shim that writes nothing
        # leaves an empty build/, never a previous run's results.
        subprocess.run([l3build_exe, "clean"], cwd=workdir, env=env,
                       capture_output=True, stdin=subprocess.DEVNULL,
                       timeout=min(timeout, 300))
        before = snapshot_diffs(workdir)
        cmd = [l3build_exe, "check", "-e", "pdftex"] + names
        out = []
        with open(logpath, "w", encoding="utf-8") as log:
            log.write("$ (cd %s) %s\n" % (workdir, " ".join(cmd)))

            def emit(line):
                log.write(line)
                out.append(line)
                m = PROGRESS.match(line.rstrip("\n"))
                if m:
                    print("  %s (%s/%s)" % (m.group(1), m.group(2),
                                            m.group(3)), flush=True)

            rc, _, timed_out = run_capture(cmd, workdir, env, timeout,
                                           on_line=emit)
            if timed_out:
                log.write("TIMEOUT after %s s: killed process group\n"
                          % timeout)
                print("  TIMEOUT after %s s: killed l3build process group"
                      % timeout, flush=True)
        deaths = engine_deaths(calllog)
        ran, completed, failed = parse_l3build_log(out)
        # Genuine verdicts before attribution: diff failures and engine
        # deaths. Everything else unfinished at a timeout is a timeout.
        decided = set(failed) | set(deaths)
        requested = names if names else list_tests(workdir)
        eff_rc = 1 if timed_out else rc
        failed, notes = attribute(ran, completed, failed, deaths,
                                  eff_rc, requested)
        timedout = set()
        if timed_out:
            # Every unfinished test ((ran | requested) with no verdict)
            # fails as a timeout, even ones attribute() leaves alone
            # (never-started tests when others ran). Tests with a real
            # verdict (pass, or fail-by-diff/death) keep it untouched.
            for t in list(ran) + [t for t in requested if t not in ran]:
                if t not in completed and t not in decided:
                    notes[t] = "timeout after %s s" % timeout
                    failed.add(t)
                    timedout.add(t)
        neverran = set()
        if eff_rc != 0 and not ran:
            # l3build aborted before any test (unpack/format death):
            # none of the requested tests passed.
            neverran |= set(requested)
        for t in sorted(set(names) - set(ran)):
            notes.setdefault(t, "requested but never ran "
                             "(no transcript entry)")
            failed.add(t)
            neverran.add(t)
        fresh = collect_diffs(workdir, before)
        diffhash = dict((t, hash_diff_files(ps))
                        for t, ps in fresh.items())
        info = {"diffhash": diffhash, "deaths": set(deaths),
                "neverran": neverran, "difffiles": fresh}
        return rc, ran, failed, notes, timedout, info
    finally:
        shutil.rmtree(shimdir, ignore_errors=True)
        try:
            os.unlink(calllog)
        except OSError:
            pass


def aggregate(results):
    """Union per-dir verdicts into (failed, ran, notes, diffhash,
    unexcusable); error dirs are skipped (main() already returned 2)."""
    failed, ran, notes, diffhash, unexcusable = set(), set(), {}, {}, set()
    for res in results.values():
        if "error" in res:
            continue
        info = res.get("info", {})
        failed |= set(res.get("failed", ()))
        ran |= set(res.get("ran", ()))
        notes.update(res.get("notes", {}))
        diffhash.update(info.get("diffhash", {}))
        unexcusable |= set(res.get("timedout", ()))
        unexcusable |= set(info.get("deaths", ()))
        unexcusable |= set(info.get("neverran", ()))
    return failed, ran, notes, diffhash, unexcusable


def gate(all_failed, expected, diffhash, unexcusable):
    """(tags, unexpected): a listed entry excuses ONLY an ordinary diff
    mismatch -- the test produced a .diff this run, with a matching
    recorded hash when the entry carries one. Unlisted tests, failures
    with no .diff, hash mismatches, and unexcusable verdicts (engine
    death, timeout, never ran) are UNEXPECTED even when listed."""
    tags, unexpected = {}, []
    for n in sorted(all_failed):
        rec = expected.get(n)
        if (n in unexcusable or rec is None or n not in diffhash or
                (rec[0] is not None and rec[0] != diffhash[n])):
            tags[n] = "UNEXPECTED"
            unexpected.append(n)
        else:
            tags[n] = "expected"
    return tags, unexpected


def summarize(results, expected, allow_stale):
    """Pure gate verdict over one_dir-style results: (rc, text) with the
    exact lines main() prints."""
    all_failed, all_ran, all_notes, diffhash, unexcusable = \
        aggregate(results)
    lines = []
    tags, unexpected = gate(all_failed, expected, diffhash, unexcusable)
    if all_failed:
        lines.append("failing tests:")
        for n in sorted(all_failed):
            note = all_notes.get(n)
            lines.append("  %s [%s]%s" % (n, tags[n],
                                          " (%s)" % note if note else ""))
    stale = sorted(n for n in expected if n in all_ran - all_failed)
    if stale:
        lines.append("stale EXPECTED-FAILURES entries now passing: %s"
                     % ", ".join(stale))
        if not allow_stale:
            lines.append("stale entries fail the gate: remove them or pass "
                         "--allow-stale")
            unexpected = sorted(set(unexpected) | set(stale))
    if unexpected:
        lines.append("UNEXPECTED failures: %s" % ", ".join(unexpected))
        return 1, "\n".join(lines)
    lines.append("OK: %d ran, %d failed (all expected), %d skipped"
                 % (len(all_ran), len(all_failed), 0))
    return 0, "\n".join(lines)


def one_dir(pair, tests, engine, timeout=DEFAULT_TIMEOUT_DIR,
            keep_diffs=None, engine_env=None):
    repo, sub = pair
    label = "%s/%s" % (repo, sub)
    workdir = os.path.join(CACHE, repo, sub)
    if not os.path.isdir(workdir):
        return label, {"error": "no checkout (run fetch.sh): %s" % workdir}
    names = [t for t in tests if t in set(list_tests(workdir))] if tests else []
    if tests and not names:
        return label, {"ran": [], "failed": set(), "notes": {}, "rc": 0,
                       "timedout": set()}
    print("== %s ==" % label, flush=True)
    rc, ran, failed, notes, timedout, info = run_l3build(
        workdir, names, engine,
        os.path.join(CACHE, "l3build-%s.log" % label.replace("/", "_")),
        timeout=timeout, engine_env=engine_env)
    if keep_diffs is not None:
        n = keep_run_diffs(keep_diffs, label, info.get("difffiles", {}),
                           failed)
        if n:
            print("  kept %d .diff file(s) in %s" % (n, keep_diffs),
                  flush=True)
    return label, {"ran": ran, "failed": failed, "notes": notes, "rc": rc,
                   "timedout": timedout, "info": info}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--engine", required=True, help="engine binary under test")
    ap.add_argument("--suite", default="all",
                    choices=("base", "required", "l3kernel", "all"))
    ap.add_argument("--tests", default="", help="comma-separated test names")
    ap.add_argument("--list", action="store_true", help="list tests and exit")
    ap.add_argument("--allow-any-engine", action="store_true",
                    help="test a candidate engine without the pdfTeX "
                    "1.40.29 reference check (prints a warning)")
    ap.add_argument("--timeout-dir", type=int, default=DEFAULT_TIMEOUT_DIR,
                    metavar="SECONDS",
                    help="kill the l3build process group if a directory "
                    "takes longer (default %d); unfinished tests FAIL "
                    "as timeouts" % DEFAULT_TIMEOUT_DIR)
    ap.add_argument("--allow-stale", action="store_true",
                    help="warn instead of failing on EXPECTED-FAILURES "
                    "entries that now pass")
    ap.add_argument("--update-baseline", action="store_true",
                    help="refresh the sha256= tokens in EXPECTED-FAILURES.txt "
                    "from this run's reference diffs (run with the "
                    "reference engine), then report the normal verdict")
    ap.add_argument("--keep-diffs", metavar="DIR",
                    help="copy each failing test's .pdftex.diff into DIR "
                    "before the next directory's clean deletes it")
    ap.add_argument("--engine-env", action="append", default=[],
                    metavar="KEY=VALUE",
                    help="repeatable: export KEY=VALUE for the engine under "
                    "test ONLY (set in the recording pdftex shim just "
                    "before it execs the engine); every other FLASHTEX_* "
                    "the parent shell exports is scrubbed, so it is never "
                    "visible to l3build, the reference engine, or the "
                    "reference version probe. KEY must match "
                    "[A-Za-z_][A-Za-z0-9_]* (anything else exits 2). "
                    "E.g. --engine-env FLASHTEX_FORMATS=/tmp/fmt "
                    "--engine-env FLASHTEX_POOL=$REPO/crates/"
                    "flashtex-engine/pdftex.pool")
    args = ap.parse_args(argv)
    if args.timeout_dir <= 0:
        ap.error("--timeout-dir must be positive")
    try:
        engine_env = parse_engine_env(args.engine_env)
    except ValueError as exc:
        ap.error(str(exc))
    dirs = SUITE_DIRS[args.suite]
    tests = [t.strip() for t in args.tests.split(",") if t.strip()]

    if args.list:
        total = 0
        for repo, sub in dirs:
            workdir = os.path.join(CACHE, repo, sub)
            names = list_tests(workdir) if os.path.isdir(workdir) else []
            if tests:
                names = [t for t in names if t in set(tests)]
            total += len(names)
            print("%s/%s: %d tests" % (repo, sub, len(names)))
            for n in names:
                print("  %s" % n)
        print("total: %d tests" % total)
        return 0

    if not (os.path.isfile(args.engine) and os.access(args.engine, os.X_OK)):
        print("error: engine not executable: %s" % args.engine,
              file=sys.stderr)
        return 2
    # The version gate also runs on a candidate engine, so it may see
    # the --engine-env variables; the reference probe (no flag) never does.
    code, msg = reference_check(args.engine, args.allow_any_engine,
                                engine_env or None)
    if msg:
        print(msg, file=sys.stderr)
    if code != 0:
        return code
    if any(not os.path.isdir(os.path.join(CACHE, r)) for r, _ in dirs):
        print("error: missing checkout; run fetch.sh first", file=sys.stderr)
        return 2

    expected = load_expected(EXPECTED)
    # Sequential across ALL dirs: every latex2e dir shares <repo>/build
    # (maindir build root + build/local installs), so overlap corrupts.
    results = dict(one_dir(p, tests, args.engine, args.timeout_dir,
                           keep_diffs=args.keep_diffs,
                           engine_env=engine_env or None)
                   for p in dirs)

    print("")
    for label, res in results.items():
        if "error" in res:
            print("%s: ERROR %s" % (label, res["error"]), file=sys.stderr)
            return 2
        timedout = set(res.get("timedout", ()))
        if res["rc"] not in (0, 1) and not timedout:
            print("%s: l3build exited %d (infra error)"
                  % (label, res["rc"]), file=sys.stderr)
            return 2
        failed = set(res["failed"])
        if not tests and not res["ran"]:
            print("%s: WARNING ran 0 tests (no applicable engine/config; "
                  "e.g. graphics targets etex/xetex only)" % label)
        print("%s: PASS %d / FAIL %d / SKIP %d"
              % (label, len(set(res["ran"]) - failed),
                 len(failed), 0))
    rc, text = summarize(results, expected, args.allow_stale)
    print(text)
    if args.update_baseline:
        _, _, _, diffhash, unexcusable = aggregate(results)
        updates = dict((n, diffhash[n]) for n, (sha, _) in expected.items()
                       if n in diffhash and n not in unexcusable and
                       diffhash[n] != sha)
        n = update_baseline_file(EXPECTED, updates) if updates else 0
        print("baseline: refreshed %d sha256 entr%s in EXPECTED-FAILURES.txt"
              % (n, "y" if n == 1 else "ies"))
    return rc


if __name__ == "__main__":
    sys.exit(main())
