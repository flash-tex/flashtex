#!/usr/bin/env python3
"""Structure-aware differential fuzzer: candidate engine vs oracle. MIT.

Mutates seed cases (or generates fresh inputs) with tools/fuzz/gen.py,
runs each on both engines through the lockstep capture(), and records
where they disagree or where the candidate crashes. Stdlib only.
"""
import argparse
import hashlib
import importlib.util
import json
import os
import random
import re
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen

_spec = importlib.util.spec_from_file_location(
    "lockstep_run", os.path.join(HERE, "..", "lockstep", "run.py"))
lockstep_run = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(lockstep_run)

CLASSES = ("equal", "diverge", "candidate-crash", "oracle-crash",
           "both-crash", "both-fail", "both-hang", "timeout",
           "output-flood", "both-flood", "invalid",
           "reference-nondeterministic")
STORE = ("diverge", "candidate-crash", "oracle-crash", "both-crash",
         "both-hang", "timeout", "output-flood", "both-flood")

# Output cap: every entry point caps its own file writes (inherited by
# engine children) at FSIZE via FUZZ_FSIZE_LIMIT_BYTES (default 64 MiB).
# A child killed by SIGXFSZ (signal death -25, or shell-reported 152) is
# an output flood. Because one log file can never grow past FSIZE, no
# transcript read into memory exceeds the cap either, so classify() and
# first_diff() compare the FULL logs; only what is written into
# artifacts and JSON is truncated to head and tail (LOG_MAX_BYTES).
FSIZE_DEFAULT_BYTES = 64 * 1024 * 1024
LOG_MAX_BYTES = 64 * 1024 * 1024
STDERR_MAX_TOTAL = 64 * 1024 * 1024

# DESIGN §4.5: every fuzz engine run (candidate AND oracle) fully disables
# shell escape, so a mutated \write16 can never become a \write18 that runs
# a program. One shared place: FUZZ_SHELL_ESCAPE_FLAGS, applied to the
# harness argv by apply_fuzz_engine_flags() (capture()/crash_stderr read
# the harness flags at call time) and spliced into the parser run_capped
# argvs. The harness default (restricted) is untouched for other users:
# this only mutates the fuzz process's own copy.
FUZZ_SHELL_ESCAPE_FLAGS = ["-cnf-line=shell_escape=f"]


def apply_fuzz_engine_flags():
    """Disable shell escape on every engine argv in this process."""
    for flag in FUZZ_SHELL_ESCAPE_FLAGS:
        if flag not in lockstep_run.ENGINE_SHELL_FLAGS:
            lockstep_run.ENGINE_SHELL_FLAGS.append(flag)


def fsize_limit_bytes():
    try:
        return int(os.environ.get("FUZZ_FSIZE_LIMIT_BYTES",
                                  str(FSIZE_DEFAULT_BYTES)))
    except (ValueError, TypeError):
        return FSIZE_DEFAULT_BYTES


def apply_fsize_limit():
    """Cap this process's file writes; engine children inherit the cap,
    so a runaway engine is killed by SIGXFSZ instead of writing gigabytes."""
    try:
        import resource
    except ImportError:
        return
    rfs = getattr(resource, "RLIMIT_FSIZE", None)
    if rfs is None:
        return
    try:
        resource.setrlimit(rfs, (fsize_limit_bytes(),) * 2)
    except (ValueError, OSError):
        pass


def is_output_flood(returncode):
    """True when a child was killed by SIGXFSZ (rc -25, or shell 152)."""
    return returncode in (-25, 152)


def cap_bytes(data, limit=LOG_MAX_BYTES):
    """At most limit bytes: the whole input, else its head and tail."""
    if not data or len(data) <= limit:
        return data or b""
    half = limit // 2
    gap = len(data) - limit
    return (data[:half] + b"\n...[truncated %d bytes]...\n" % gap
            + data[-half:])


def cap_text(text, limit=LOG_MAX_BYTES):
    """At most limit characters: the whole log, else head and tail."""
    if not text or len(text) <= limit:
        return text or ""
    half = limit // 2
    gap = len(text) - limit
    return (text[:half] + "\n...[truncated %d chars]...\n" % gap
            + text[-half:])


def read_capped(path, limit=LOG_MAX_BYTES):
    """Up to limit bytes of a file: the whole file, else head and tail."""
    try:
        size = os.path.getsize(path)
    except OSError:
        return b""
    try:
        with open(path, "rb") as fh:
            if size <= limit:
                return fh.read()
            half = limit // 2
            head = fh.read(half)
            fh.seek(max(size - half, 0))
            tail = fh.read(half + 1)
    except OSError:
        return b""
    gap = size - len(head) - len(tail)
    return (head + b"\n...[truncated %d bytes]...\n" % gap + tail)


def kill_group(proc, sigkill_after=5.0):
    """SIGTERM a Popen'd process group, then SIGKILL after a grace."""
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except (OSError, ProcessLookupError):
        pass
    try:
        proc.wait(timeout=sigkill_after)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except (OSError, ProcessLookupError):
            pass
        proc.wait()


def run_capped(argv, cwd, env, timeout, split=False):
    """Run argv with output on disk (never a pipe buffer), then read back
    at most LOG_MAX_BYTES (head and tail). Returns (rc, combined), or
    (rc, stdout, stderr) with split=True. On timeout the process group is
    killed and subprocess.TimeoutExpired is raised with the capped partial
    output on exc.stdout (and exc.stderr when split)."""
    made = []
    try:
        fd, p_out = tempfile.mkstemp(prefix="fuzz-cap-")
        os.close(fd)
        made.append(p_out)
        p_err = None
        if split:
            fd, p_err = tempfile.mkstemp(prefix="fuzz-cap-")
            os.close(fd)
            made.append(p_err)
        f_out = open(p_out, "wb")
        f_err = f_out if not split else open(p_err, "wb")
        try:
            proc = subprocess.Popen(
                argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                stdout=f_out, stderr=f_err, start_new_session=True)
        except OSError:
            f_out.close()
            if split:
                f_err.close()
            raise
        try:
            proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            kill_group(proc)
            f_out.close()
            if split:
                f_err.close()
            exc = subprocess.TimeoutExpired(argv, timeout)
            exc.stdout = read_capped(p_out)
            exc.stderr = read_capped(p_err) if split else None
            raise exc
        f_out.close()
        if split:
            f_err.close()
        if split:
            return proc.returncode, read_capped(p_out), read_capped(p_err)
        return proc.returncode, read_capped(p_out)
    finally:
        for path in made:
            try:
                os.unlink(path)
            except OSError:
                pass
# Env the candidate may need; passed through to the candidate's capture()
# call only (extra_env on top of the pinned environment), never the oracle.
CANDIDATE_ENV_VARS = ("FLASHTEX_POOL", "FLASHTEX_FORMATS")


def candidate_env():
    env = {k: v for k, v in os.environ.items() if k in CANDIDATE_ENV_VARS}
    return env or None


def is_candidate_crash(returncode):
    """Candidate crash: killed by a signal (rc < 0) or Rust panic exit 101.

    Transcript text is never consulted: a document containing the words
    "panicked at" (e.g. \\message{panicked at}) must not count as a crash.
    """
    return (returncode is not None
            and (returncode < 0 or returncode == 101))


def is_oracle_crash(returncode):
    """Oracle crash: killed by a signal (rc < 0) only.

    The reference pdfTeX never exits 101, so a 101 from the oracle side is
    not a crash; transcript text is never consulted either.
    """
    return returncode is not None and returncode < 0


def is_crash(returncode, log=None):
    """Legacy alias of the candidate-side rule; log is accepted and ignored.

    Kept so single-engine callers (which only run the candidate) keep one
    spelling; the transcript text never decides a crash.
    """
    return is_candidate_crash(returncode)


# An unseeded \pdfrandomseed / \pdfuniformdeviate / \pdfnormaldeviate
# read differs between runs, so a mutant that reads one before any
# \pdfsetrandomseed (e.g. after a mutation deleted the set) would make the
# reference nondeterministic. Such mutants are invalid (rejected before
# comparing); as a second guard, a divergence whose oracle re-run disagrees
# with its first run is reference-nondeterministic, not a finding.
RANDOM_TOKEN_RE = re.compile(
    r"\\pdf(setrandomseed|randomseed|uniformdeviate|normaldeviate)"
    r"(?![A-Za-z])")
COMMENT_RE = re.compile(r"(?<!\\)%.*")


def has_unseeded_random_read(text):
    """True when text reads a random primitive before any
    \\pdfsetrandomseed (TeX % comments stripped, so a commented-out read
    does not count)."""
    seeded = False
    for line in (text or "").splitlines():
        for m in RANDOM_TOKEN_RE.finditer(COMMENT_RE.sub("", line)):
            if m.group(1) == "setrandomseed":
                seeded = True
            elif not seeded:
                return True
    return False


def oracle_logs_agree(first, second):
    """True when two oracle logs match outside the accounting lines."""
    return (lockstep_run.compared_lines(first or "")
            == lockstep_run.compared_lines(second or ""))


# Differing-line snippet: a window around the first differing column, so a
# difference late in a long line still shows the differing text (a fixed
# 160-char cut showed identical snippets for those).
DIFF_WINDOW_RADIUS = 80


def _diff_window(line, col, radius=DIFF_WINDOW_RADIUS):
    lo, hi = max(0, col - radius), col + radius
    return ("%s%s%s" % ("..." if lo > 0 else "", line[lo:hi],
                        "..." if hi < len(line) else ""))


def first_diff_pair(cand_log, orc_log):
    """Raw (candidate line, oracle line) at the first compared difference.

    Same compared view as first_diff (harness normalisation only, digits
    intact). Returns None when the compared logs are identical (a
    returncode-only divergence).
    """
    a = lockstep_run.compared_lines(cand_log or "")
    b = lockstep_run.compared_lines(orc_log or "")
    n = max(len(a), len(b))
    for i in range(n):
        x = a[i] if i < len(a) else "<EOF>"
        y = b[i] if i < len(b) else "<EOF>"
        if x != y:
            return (x, y)
    return None


def first_diff(cand_rc, cand_log, orc_rc, orc_log):
    if cand_rc != orc_rc:
        return "returncode candidate=%s oracle=%s" % (cand_rc, orc_rc)
    a = lockstep_run.compared_lines(cand_log or "")
    b = lockstep_run.compared_lines(orc_log or "")
    n = max(len(a), len(b))
    for i in range(n):
        x = a[i] if i < len(a) else "<EOF>"
        y = b[i] if i < len(b) else "<EOF>"
        if x != y:
            m = min(len(x), len(y))
            j = next((k for k in range(m) if x[k] != y[k]), m)
            return "line %d col %d: %r vs %r" % (
                i + 1, j + 1, _diff_window(x, j), _diff_window(y, j))
    return None


def diverge_signature(cand_rc, cand_log, orc_rc, orc_log, diff):
    """Dedupe key from the exact normalised differing lines' hash, so two
    pairs that only differ past the snippet window still differ."""
    pair = first_diff_pair(cand_log, orc_log)
    text = "%r vs %r" % pair if pair is not None else (diff or "")
    digest = hashlib.sha256(
        re.sub(r"\d", "N", text).encode("utf-8")).hexdigest()[:16]
    return "diverge:" + digest


def classify(cand_rc, cand_log, orc_rc, orc_log, timeouts):
    # timeouts is either a bool (timed out unknown side) or a
    # (candidate, oracle) pair: both sides timing out is "both-hang".
    if isinstance(timeouts, (tuple, list)):
        if timeouts[0] and timeouts[1]:
            return "both-hang"
        if timeouts[0] or timeouts[1]:
            return "timeout"
    elif timeouts:
        return "timeout"
    cand_flood = is_output_flood(cand_rc)
    orc_flood = is_output_flood(orc_rc)
    if cand_flood and orc_flood:
        return "both-flood"
    if cand_flood or orc_flood:
        return "output-flood"
    cand_crash = is_candidate_crash(cand_rc)
    orc_crash = is_oracle_crash(orc_rc)
    if cand_crash and orc_crash:
        # pdfTeX itself crashes too: not an engine-diff.
        return "both-crash"
    if cand_crash:
        return "candidate-crash"
    if orc_crash:
        return "oracle-crash"
    if cand_rc != 0 and orc_rc != 0:
        return "both-fail"
    if first_diff(cand_rc, cand_log, orc_rc, orc_log) is not None:
        return "diverge"
    return "equal"


def run_one(text, candidate, oracle, timeout, return_logs=False):
    """Run text on both engines; return (class, cand_rc, orc_rc, diff).

    With return_logs=True, append (cand_log, orc_log) to the tuple.
    A mutant that reads a random primitive before any \\pdfsetrandomseed
    is "invalid" (rejected without running any engine); a divergence whose
    oracle re-run disagrees with its first run is
    "reference-nondeterministic" instead of a finding.
    """
    apply_fuzz_engine_flags()
    if has_unseeded_random_read(text):
        result = ("invalid", None, None,
                  "invalid: unseeded random read before \\pdfsetrandomseed",
                  "", "")
        return result if return_logs else result[:4]
    workdir = tempfile.mkdtemp(prefix="fuzz-")
    try:
        shutil.copy(lockstep_run.PRELUDE, os.path.join(workdir, "prelude.tex"))
        tex_path = os.path.join(workdir, "fuzz.tex")
        with open(tex_path, "w") as fh:
            fh.write(text)
        results = []
        timeouts = []
        for binary, extra in ((candidate, candidate_env()), (oracle, None)):
            try:
                cap = lockstep_run.capture(tex_path, binary, workdir,
                                           extra_env=extra, timeout=timeout)
                results.append((cap.returncode, cap.log))
                timeouts.append(False)
            except subprocess.TimeoutExpired:
                results.append((None, ""))
                timeouts.append(True)
        (cand_rc, cand_full), (orc_rc, orc_full) = results
        # Compare the FULL logs: truncating to head+tail first would hide
        # a mid-log difference (two huge logs differing only mid-log
        # would compare equal). A log past the file-size cap never
        # reaches the comparison: its engine died with SIGXFSZ and is
        # classed output-flood/both-flood above any log content.
        timed_out = timeouts[0] or timeouts[1]
        cls = classify(cand_rc, cand_full, orc_rc, orc_full, timeouts)
        if timed_out:
            diff = "timeout: %s" % "/".join(
                s for s, t in (("candidate", timeouts[0]),
                               ("oracle", timeouts[1])) if t)
        else:
            diff = first_diff(cand_rc, cand_full, orc_rc, orc_full)
        if cls == "diverge":
            try:
                again = lockstep_run.capture(tex_path, oracle, workdir,
                                             extra_env=None,
                                             timeout=timeout)
                rerun = again.log
            except (subprocess.TimeoutExpired, OSError):
                rerun = None
            if rerun is not None and not oracle_logs_agree(orc_full, rerun):
                cls = "reference-nondeterministic"
                diff = ("reference-nondeterministic: oracle logs differ "
                        "between runs")
        # Cap only what leaves this function: artifacts and JSON carry
        # head and tail, never a huge log. (The limit is passed
        # explicitly so the artifact cap stays adjustable without
        # touching the comparison.)
        cand_log = cap_text(cand_full, LOG_MAX_BYTES)
        orc_log = cap_text(orc_full, LOG_MAX_BYTES)
        if return_logs:
            return cls, cand_rc, orc_rc, diff, cand_log, orc_log
        return cls, cand_rc, orc_rc, diff
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


STDERR_TAIL_BYTES = 64 * 1024
PANIC_LOC_RE = re.compile(r"([^\s'\",;()]+):(\d+):\d+")


def _as_text(output):
    if isinstance(output, bytes):
        return output.decode("utf-8", "replace")
    return output or ""


def signal_name(rc):
    """Signal name for a negative return code ("SIGABRT"), else None."""
    if rc is None or rc >= 0:
        return None
    try:
        return signal.Signals(-rc).name
    except ValueError:
        return "SIG%d" % (-rc,)


def panic_location(log):
    """Panic site as "<file>:<line>", or None when absent.

    Parses "panicked at <file>:<line>:<col>", dropping the column and any
    thread id ("thread 'main' (123) panicked at ..." still yields just
    the file and line), so a new panic site gets a new signature.
    """
    text = _as_text(log)
    if "panicked at" not in text:
        return None
    tail = text.split("panicked at", 1)[1]
    for scope in (tail.split("\n", 1)[0], tail):
        matches = PANIC_LOC_RE.findall(scope)
        if matches:
            path, line = matches[-1]
            return "%s:%s" % (path.strip("'\""), line)
    return None


def first_key_line(text):
    """First non-empty line with every digit replaced by N (max 200)."""
    for line in _as_text(text).splitlines():
        line = line.strip()
        if line:
            return re.sub(r"\d", "N", line[:200])
    return ""


def crash_signature(rc, log, stderr=""):
    """Dedupe signature for a crash (no class prefix).

    Panics key on the panic site ("panic:<file>:<line>"); signals key on
    the signal name plus the first stderr line ("signal:SIGABRT:<line>"),
    so different abort causes get different signatures. stderr (from a
    direct re-run, see crash_stderr) wins over the transcript log.
    """
    loc = panic_location(stderr) or panic_location(log)
    if loc is not None:
        return "panic:" + loc
    if rc is not None and rc < 0:
        first = first_key_line(stderr or log)
        if first:
            return "signal:%s:%s" % (signal_name(rc), first)
        return "signal:%s" % (signal_name(rc),)
    return "exit:%s" % (rc,)


def crash_stderr(text, binary, extra_env, timeout, fmt=None):
    """Re-run text directly on binary; return the last 64 KiB of stderr.

    capture() returns the transcript log and the return code but not the
    engine's stderr, so a crash signature is built from a direct re-run
    of the same input: same args and environment (pinned plus extra_env),
    its own temp dir, the same per-engine timeout, stdout discarded.
    stderr is read through a pipe in a loop keeping only the last 64 KiB,
    and the process group is killed after 64 MiB in total, so a
    stderr-flooding engine cannot hold a large buffer. Returns "" when
    the re-run fails to start or times out.
    """
    apply_fuzz_engine_flags()
    work = tempfile.mkdtemp(prefix="fuzz-stderr-")
    try:
        shutil.copy(lockstep_run.PRELUDE, os.path.join(work, "prelude.tex"))
        tex_path = os.path.join(work, "fuzz.tex")
        with open(tex_path, "w") as fh:
            fh.write(text)
        env = lockstep_run.pinned_env()
        if extra_env:
            env.update(extra_env)
        if fmt is None:
            args = (list(lockstep_run.ENGINE_ARGS)
                    + list(lockstep_run.ENGINE_SHELL_FLAGS))
        else:
            args = ([a for a in lockstep_run.ENGINE_ARGS
                     if a not in ("-ini", "-etex")]
                    + list(lockstep_run.ENGINE_SHELL_FLAGS)
                    + ["-fmt=" + fmt])
        argv0 = lockstep_run.engine_link(binary, work)
        try:
            proc = subprocess.Popen(
                [argv0] + args + [tex_path], cwd=work, env=env,
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, start_new_session=True)
        except OSError:
            return ""
        tail = bytearray()
        total = [0]
        deadline = time.monotonic() + timeout
        timed_out = [False]

        def feed(data):
            total[0] += len(data)
            tail.extend(data)
            if len(tail) > STDERR_TAIL_BYTES:
                del tail[:len(tail) - STDERR_TAIL_BYTES]
            return total[0] > STDERR_MAX_TOTAL

        fd = proc.stderr.fileno()
        try:
            os.set_blocking(fd, False)
        except OSError:
            pass
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                timed_out[0] = True
                break
            try:
                ready, _, _ = select.select([fd], [], [],
                                            min(remaining, 0.2))
            except (OSError, ValueError):
                break
            if not ready:
                if proc.poll() is None:
                    continue
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                if proc.poll() is not None:
                    break
                continue
            if feed(data):
                break
        kill_group(proc)
        try:
            proc.stderr.close()
        except OSError:
            pass
        if timed_out[0]:
            return ""
        return bytes(tail).decode("utf-8", "replace")
    finally:
        shutil.rmtree(work, ignore_errors=True)


def signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff,
              cand_stderr="", orc_stderr=""):
    """Dedupe signature for a non-equal result, or None when it has none."""
    if cls == "candidate-crash":
        return crash_signature(cand_rc, cand_log, cand_stderr)
    if cls == "oracle-crash":
        return crash_signature(orc_rc, orc_log, orc_stderr)
    if cls == "both-crash":
        # Both engines crashed: dedupe on the candidate's signature.
        return "both-crash:" + crash_signature(cand_rc, cand_log,
                                               cand_stderr)
    if cls == "both-hang":
        return "both-hang"
    if cls == "output-flood":
        return "output-flood"
    if cls == "both-flood":
        return "both-flood"
    if cls == "diverge":
        return diverge_signature(cand_rc, cand_log, orc_rc, orc_log, diff)
    if cls == "timeout":
        # The finding names which side hung (both-hang stays its own
        # constant): candidate-only and oracle-only hangs get distinct
        # signatures instead of collapsing into one "timeout".
        diff = diff or ""
        if "candidate" in diff and "oracle" in diff:
            return "timeout:both"
        if "candidate" in diff:
            return "timeout:candidate"
        if "oracle" in diff:
            return "timeout:oracle"
        return "timeout"
    return None


def normalised_diff(diff):
    """First differing line with every digit replaced by N."""
    return re.sub(r"\d", "N", diff or "")


def load_known_signatures(out_dir):
    """Signatures already stored on disk under OUT (from signatures.json
    plus per-case .json sidecars), so repeat runs do not re-store them."""
    known = set()
    try:
        with open(os.path.join(out_dir, "signatures.json")) as fh:
            data = json.load(fh)
        if isinstance(data, dict):
            known.update(data.keys())
    except (OSError, ValueError):
        pass
    try:
        for root, _dirs, files in os.walk(out_dir):
            for name in files:
                if not name.endswith(".json") or name == "signatures.json":
                    continue
                try:
                    with open(os.path.join(root, name)) as fh:
                        info = json.load(fh)
                    if isinstance(info, dict) and info.get("signature"):
                        known.add(info["signature"])
                except (OSError, ValueError):
                    continue
    except OSError:
        pass
    return known


def load_seeds(seeds_dir):
    try:
        names = sorted(f for f in os.listdir(seeds_dir) if f.endswith(".tex"))
    except OSError:
        return []
    out = []
    for name in names:
        try:
            with open(os.path.join(seeds_dir, name)) as fh:
                out.append((name, fh.read()))
        except OSError:
            continue
    return out


def draw_input(rng, seeds):
    """Pick a seed (or fresh input) and mutate it; return (text, mutation,
    origin). Re-draws the mutation (up to 3 extra times, without running
    any engine) when the mutated text is identical to its seed."""
    if seeds and rng.random() < 0.5:
        name, base = seeds[rng.randrange(len(seeds))]
        origin = name
    else:
        base = gen.generate(rng)
        origin = "generated"
    text, mutation = gen.mutate_with_info(base, rng)
    for _ in range(3):
        if text != base:
            break
        text, mutation = gen.mutate_with_info(base, rng)
    return text, mutation, origin


def run_fuzz(candidate, oracle, seeds_dir, out_dir, iterations, seed,
             timeout):
    rng = random.Random(seed)
    seeds = load_seeds(seeds_dir)
    known = load_known_signatures(out_dir)
    seen = set()
    sig_counts = {}
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        text, mutation, origin = draw_input(rng, seeds)
        cls, cand_rc, orc_rc, diff, cand_log, orc_log = run_one(
            text, candidate, oracle, timeout, return_logs=True)
        counts[cls] += 1
        if cls in STORE:
            cand_err, orc_err, panic_loc = "", "", None
            if cls in ("candidate-crash", "both-crash"):
                cand_err = crash_stderr(text, candidate, candidate_env(),
                                        timeout)
                panic_loc = (panic_location(cand_err)
                             or panic_location(cand_log))
            if cls in ("oracle-crash", "both-crash"):
                orc_err = crash_stderr(text, oracle, None, timeout)
                if cls == "oracle-crash":
                    panic_loc = (panic_location(orc_err)
                                 or panic_location(orc_log))
            sig = signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff,
                            cand_stderr=cand_err, orc_stderr=orc_err)
            if sig is not None:
                sig_counts[sig] = sig_counts.get(sig, 0) + 1
            if sig is None or (sig not in seen and sig not in known):
                if sig is not None:
                    seen.add(sig)
                digest = hashlib.sha256(
                    text.encode("utf-8")).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".tex"), "w") as fh:
                    fh.write(text)
                with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                    json.dump({"iteration": i, "seed": origin,
                               "mutation": mutation,
                               "candidate_returncode": cand_rc,
                               "oracle_returncode": orc_rc,
                               "first_diff": diff,
                               "panic_location": panic_loc,
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("fuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    if sig_counts:
        try:
            path = os.path.join(out_dir, "signatures.json")
            merged = dict(sig_counts)
            try:
                with open(path) as fh:
                    old = json.load(fh)
                if isinstance(old, dict):
                    for key, val in old.items():
                        merged[key] = merged.get(key, 0) + val
            except (OSError, ValueError):
                pass
            os.makedirs(out_dir, exist_ok=True)
            with open(path, "w") as fh:
                json.dump(merged, fh, indent=2, sort_keys=True)
        except OSError:
            pass
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="differential TeX fuzzer")
    ap.add_argument("--candidate", required=True, help="candidate engine")
    ap.add_argument("--oracle", required=True, help="oracle engine")
    ap.add_argument("--seeds", required=True, help="seed .tex directory")
    ap.add_argument("--out", required=True, help="output directory")
    ap.add_argument("--iterations", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--timeout", type=float, required=True,
                    help="per-engine timeout in seconds")
    args = ap.parse_args(argv)
    apply_fsize_limit()
    apply_fuzz_engine_flags()
    for label, binary in (("candidate", args.candidate),
                          ("oracle", args.oracle)):
        if not (os.path.isfile(binary) or shutil.which(binary)):
            print("error: %s not found: %s" % (label, binary),
                  file=sys.stderr)
            return 2
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    counts = run_fuzz(args.candidate, args.oracle, args.seeds, args.out,
                      args.iterations, args.seed, args.timeout)
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
