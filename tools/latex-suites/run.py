#!/usr/bin/env python3
"""LaTeX suites (latex2e base/required, latex3 l3kernel) vs any engine binary.

Runs `l3build check -e pdftex` per suite dir with a recording shim `pdftex`
first on PATH resolving to the engine under test, so the SAME l3build
normalisation compares each log against the committed upstream `.tlg`.
Exit 0 iff failures are all in EXPECTED-FAILURES.txt; 1 on unexpected
failures or crashed/unresolved tests; 2 on infra errors. Stdlib only.

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
  line must contain it); `--allow-any-engine` bypasses with a warning.
"""

import argparse
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


def load_expected(path):
    expected = {}
    if os.path.exists(path):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line and line[0] != "#":
                    name, _, reason = line.partition(":")
                    expected[name.strip()] = reason.strip()
    return expected


def list_tests(workdir):
    testdir = os.path.join(workdir, "testfiles")
    return sorted(f[:-4] for f in os.listdir(testdir) if f.endswith(".lvt"))


def engine_version_firstline(engine):
    """First stdout line of `<engine> --version`, or None if unusable.

    Content-based: an exit code is NOT required, so a shim that runs the
    real engine and exits nonzero still passes when it prints the real
    banner; a binary that dies silently (or prints a foreign banner)
    yields None/wrong line and is refused.
    """
    try:
        proc = subprocess.run([engine, "--version"], capture_output=True,
                              text=True, timeout=60, stdin=subprocess.DEVNULL)
    except (OSError, subprocess.SubprocessError):
        return None
    if not proc.stdout:
        return None
    lines = proc.stdout.splitlines()
    return lines[0] if lines else None


def reference_check(engine, allow_any):
    """(code, message): code 0 proceeds, 2 refuses; message is warning/error."""
    vline = engine_version_firstline(engine)
    if vline is not None and REF_ENGINE_VERSION in vline:
        if allow_any:
            return (0, "warning: --allow-any-engine passed; engine reports "
                    "reference pdfTeX %s anyway (%s)"
                    % (REF_ENGINE_VERSION, vline.strip()))
        return (0, None)
    msg = ("engine %s is not the reference: `--version` first line %r does "
           "not contain %s (reference pdfTeX 1.40.29)"
           % (engine, vline, REF_ENGINE_VERSION))
    if allow_any:
        return (0, "warning: " + msg + "; proceeding anyway")
    return (2, "error: " + msg + "; pass --allow-any-engine to test a "
            "candidate engine anyway")


def make_shim(engine, calllog):
    """Dir with a recording `pdftex` wrapper around the engine under test.

    Every invocation's exit code + argv is appended to calllog, so an
    engine death l3build swallows (Lua assert, no diff) still maps to the
    test named by `-jobname=`. Exit code and stdio pass through unchanged.
    """
    shimdir = tempfile.mkdtemp(prefix="latex-suites-shim-")
    path = os.path.join(shimdir, "pdftex")
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("#!/bin/sh\nCALLLOG=%s\n%s \"$@\"\nrc=$?\n"
                 "printf 'rc=%%d argv=%%s\\n' \"$rc\" \"$*\" >> \"$CALLLOG\"\n"
                 "exit \"$rc\"\n"
                 % (shlex.quote(calllog), shlex.quote(engine)))
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
                timeout=DEFAULT_TIMEOUT_DIR, l3build_exe="l3build"):
    """Clean then `l3build check`; return (exitcode, ran, failed, notes,
    timedout) where timedout names tests failed by the directory timeout.
    """
    fd, calllog = tempfile.mkstemp(prefix="latex-suites-calls-")
    os.close(fd)
    shimdir = make_shim(engine, calllog)
    try:
        env = dict(os.environ)
        env["PATH"] = shimdir + os.pathsep + env.get("PATH", "")
        # Start pristine: committed results must not depend on earlier
        # installs (e.g. required/tools' multicol.sty lingering in
        # build/local flips github-1336, which expects it to be absent).
        # This is also the staleness guard: a shim that writes nothing
        # leaves an empty build/, never a previous run's results.
        subprocess.run([l3build_exe, "clean"], cwd=workdir, env=env,
                       capture_output=True, stdin=subprocess.DEVNULL,
                       timeout=min(timeout, 300))
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
        failed, notes = attribute(ran, completed, failed, deaths,
                                  1 if timed_out else rc, requested)
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
        return rc, ran, failed, notes, timedout
    finally:
        shutil.rmtree(shimdir, ignore_errors=True)
        try:
            os.unlink(calllog)
        except OSError:
            pass


def one_dir(pair, tests, engine, timeout=DEFAULT_TIMEOUT_DIR):
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
    rc, ran, failed, notes, timedout = run_l3build(
        workdir, names, engine,
        os.path.join(CACHE, "l3build-%s.log" % label.replace("/", "_")),
        timeout=timeout)
    return label, {"ran": ran, "failed": failed, "notes": notes, "rc": rc,
                   "timedout": timedout}


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
    args = ap.parse_args(argv)
    if args.timeout_dir <= 0:
        ap.error("--timeout-dir must be positive")
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
    code, msg = reference_check(args.engine, args.allow_any_engine)
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
    results = dict(one_dir(p, tests, args.engine, args.timeout_dir)
                   for p in dirs)

    all_failed, all_ran, all_notes, all_timedout = set(), set(), {}, set()
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
        all_ran |= set(res["ran"])
        all_failed |= failed
        all_notes.update(res.get("notes", {}))
        all_timedout |= timedout
        print("%s: PASS %d / FAIL %d / SKIP %d"
              % (label, len(set(res["ran"]) - failed),
                 len(failed), 0))
    if all_failed:
        print("failing tests:")
        for n in sorted(all_failed):
            tag = "UNEXPECTED" if (n not in expected or n in all_timedout) \
                else "expected"
            note = all_notes.get(n)
            print("  %s [%s]%s" % (n, tag, " (%s)" % note if note else ""))
    # Timeouts are always UNEXPECTED, even for tests that also sit in
    # EXPECTED-FAILURES.txt (a timeout is no evidence about the diff).
    unexpected = sorted(set(all_failed) - set(expected) | all_timedout)
    stale = sorted(n for n in expected if n in all_ran - all_failed)
    if stale:
        print("stale EXPECTED-FAILURES entries now passing: %s"
              % ", ".join(stale))
    if unexpected:
        print("UNEXPECTED failures: %s" % ", ".join(unexpected))
        return 1
    print("OK: %d ran, %d failed (all expected), %d skipped"
          % (len(all_ran), len(all_failed), 0))
    return 0


if __name__ == "__main__":
    sys.exit(main())
