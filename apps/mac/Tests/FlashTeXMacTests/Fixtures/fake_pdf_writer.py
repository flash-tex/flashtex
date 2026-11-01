#!/usr/bin/env python3
"""Test double for flashtex-pdf (NOT a PDF writer). Reads one JSON line from
stdin, writes a tiny PDF-looking file to --out, then floods stderr with
FAKE_STDERR_BYTES bytes (default 1 MiB) before exiting — more than a pipe
holds, so a parent that waits for exit before draining would deadlock.
FAKE_SLEEP seconds delays exit (for timeout tests). FAKE_EXIT sets the code."""
import os, sys, time
args = sys.argv[1:]
out = args[args.index("--out") + 1] if "--out" in args else None
line = sys.stdin.readline()
if out:
    with open(out, "wb") as f:
        f.write(b"%PDF-1.4\n%fake\n%%EOF\n")
n = int(os.environ.get("FAKE_STDERR_BYTES", str(1024 * 1024)))
chunk = b"x" * 65536
written = 0
while written < n:
    sys.stderr.buffer.write(chunk[: min(65536, n - written)])
    written += min(65536, n - written)
sys.stderr.buffer.flush()
time.sleep(float(os.environ.get("FAKE_SLEEP", "0")))
sys.stdout.write("note: fake writer done\n"); sys.stdout.flush()
sys.exit(int(os.environ.get("FAKE_EXIT", "0")))
