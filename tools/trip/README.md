# Trip / etrip test runner (DESIGN T0 gate: "trip passes")

`run.py` ports TeX Live's `texk/web2c/triptest.test` (Knuth's trip test) and
`texk/web2c/etexdir/etriptest.test` (e-TeX etrip test) to stdlib-only Python.
`fetch.sh` pins and fetches the upstream inputs at run time; nothing upstream
is committed here (MIT tooling; upstream files stay in the cache).

```sh
tools/trip/fetch.sh                                   # once; fills ~/.cache/flashtex-trip
python3 tools/trip/run.py --engine /Library/TeX/texbin/tex --kind trip
python3 tools/trip/run.py --engine /Library/TeX/texbin/etex --kind etrip
python3 tools/trip/test_trip.py                       # unit + fake-engine tests
```

`--engine <bin>` is the engine under test (`tex` for trip, an e-TeX for
etrip; `pdftex` is NOT trip-capable). `--keep` keeps the staging dir.
`--pltotf/--tftopl/--dvitype` override PATH lookup. A missing helper tool
is FAIL (exit 1) naming the binary, matching upstream's hard failure
(`triptest.test`/`etriptest.test` do `|| exit 1` on those steps); without
the helper the DVI/PL comparison cannot run, so a corrupt artifact must
never report PASS. `--allow-missing-tools` downgrades missing helpers to
SKIP, but the result is still `INCOMPLETE (n skip)` with exit 1 plus a
loud `PASS with skips is NOT a trip pass` warning — never `PASS`.
Exit 0 iff nothing FAILs and nothing SKIPped.

Staging mirrors upstream with one deliberate hardening: `TEXMFCNF`
points at a per-run scratch directory holding ONLY `texmf.cnf`
(`error_line=64`, `half_error_line=32`, `max_print_line=72`,
`mem_bot=1`, `main_memory=3000`, …) — never at the cache directory,
which also holds the expected outputs. Upstream sets `TEXMFCNF` to the
test directory itself, so a shim engine reading `$TEXMFCNF/trip.fot`
could copy the expected files and pass without doing any TeX work;
here that shim FAILs (no such file). Expected files are read from the
cache only by the runner, after the run. Inputs are copied into a temp workdir,
pass 1 is `<engine> --progname=initex --ini <.in >*.fot`, pass 2 runs the
built format; every subprocess has a timeout with process-group kill, engine
stdin comes from the `.in` file, everything else gets `/dev/null`.
Upstream never checks the engine's exit status — a trip run legitimately
ends with a nonzero status on some engines (INITEX exits 1) — and this tool
follows upstream on that point: engine exit codes are ignored and only the
artifacts/comparisons gate. Timeout (whole process-group kill), signal
death, engine stdin from the `.in` file, and `/dev/null` elsewhere are kept
as they are; a timeout, signal death, or a missing artifact is FAIL.

Comparisons gate exactly where upstream's `is_OK=false` gates: exact
`trip.pl`/`etrip.pl` round-trips and `tripos.tex`, plus the
accepted-difference filters (banner/date, memory statistics, x86 glue
rounding, DVI movements) over `tripin.log`, `trip.fot`, `trip.log`,
`trip.typ` (and the etrip `c*`/`x*`/`e*` counterparts). Raw diffs upstream
leaves ungated (e-TeX banners, memory usage, `etrip.out`) are INFO rows.
One deliberate deviation: inputs are copied, not symlinked, into the
workdir, so logs name bare `trip.tex` exactly as Knuth's expected files do.

`filter1` (extended phase, x side only, mirroring upstream's `:l`/`N`
whole-file `s///`) collapses only two known shapes: both anchors on one
line, and the real e-TeX group-trace block (`(end occurred …)` / blank
/ `### <group> entered at line N (…)` / `### bottom level`). Upstream's
`.*` spans newlines on BSD `sed`, so upstream `sed` itself would swallow
arbitrary lines between the anchors; this port is deliberately stricter
so smuggled middle content fails loudly. Output is byte-identical to
`sed -f filter` / `sed -f filter1` on the real files
(`test_tool_filters_byte_identical_to_upstream_sed`).

Upstream-faithful limit (not a bug, do not "fix"): the
accepted-difference filters delete/normalise matching lines on BOTH
sides, exactly like upstream's `/pattern/d`, so an injected line that
matches a deleted pattern (e.g. `** &trip  trip`) or a normalised one
(memory statistics, dates, glue rounding) is invisible to the comparison
— upstream has the identical blind spot. Pinned by
`test_accepted_difference_hidden_injection_matches_sed`, which checks
the tool agrees byte for byte with `sed -f` on such a probe.

Pin: see `PINS.txt` (no `texlive-2026` tag exists upstream; HEAD pinned).
