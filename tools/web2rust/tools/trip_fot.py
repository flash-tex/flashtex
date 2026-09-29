#!/usr/bin/env python3
"""Compare a trip-test terminal transcript (tripin.fot / trip.fot) with Knuth's
master, allowing exactly two differences and nothing else.

    trip_fot.py MASTER OURS TYPED_LINE...

1. Terminal echo. Knuth typed his responses at the `**` prompt, so the master
   shows each TYPED_LINE and its newline right after `**`. The trip script
   pipes them in, and nothing echoes piped input, so ours has `**` followed
   directly by TeX's next output. Each TYPED_LINE, in order, must appear as
   `**LINE\\n` in the master; that text becomes `**`.
2. The final newline. tex.web §1333 ends with `print_char(".")` and no
   `print_ln` (web2c's tex.ch adds one), so our transcript ends without the
   newline the master file has.

After those two edits the files must be identical. Exit status 0 iff they are.
"""
import sys


def main() -> int:
    master_path, ours_path, *typed = sys.argv[1:]
    master = open(master_path, encoding="latin-1").read()
    ours = open(ours_path, encoding="latin-1").read()
    name = ours_path.rsplit("/", 1)[-1]

    pos = 0
    for line in typed:
        needle = "**" + line + "\n"
        i = master.find(needle, pos)
        if i < 0:
            print(f"FAIL {name}: master has no `**{line}` prompt echo to remove")
            return 1
        master = master[:i] + "**" + master[i + len(needle):]
        pos = i + 2
    newline_dropped = False
    if master.endswith("\n") and not ours.endswith("\n"):
        master = master[:-1]
        newline_dropped = True

    if master == ours:
        notes = [f"{len(typed)} echoed input line(s) removed"]
        if newline_dropped:
            notes.append("final newline dropped")
        print(f"PASS {name}: identical after the accepted differences ({'; '.join(notes)})")
        return 0
    m, o = master.splitlines(), ours.splitlines()
    for k in range(max(len(m), len(o))):
        a = m[k] if k < len(m) else "<eof>"
        b = o[k] if k < len(o) else "<eof>"
        if a != b:
            print(f"FAIL {name}: first difference at line {k + 1}\n  master: {a!r}\n  ours:   {b!r}")
            break
    else:
        print(f"FAIL {name}: differs only in trailing whitespace")
    return 1


if __name__ == "__main__":
    sys.exit(main())
