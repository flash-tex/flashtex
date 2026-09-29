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
`--pltotf/--tftopl/--dvitype` override PATH lookup; when `tftopl`/`dvitype`
are absent those comparisons report SKIP. Exit 0 iff nothing FAILs.

Staging mirrors upstream: `TEXMFCNF` points at the cached `texmf.cnf`
(`error_line=64`, `half_error_line=32`, `max_print_line=72`,
`mem_bot=1`, `main_memory=3000`, …), inputs are copied into a temp workdir,
pass 1 is `<engine> --progname=initex --ini <.in >*.fot`, pass 2 runs the
built format; every subprocess has a timeout with process-group kill, engine
stdin comes from the `.in` file, everything else gets `/dev/null`.
Engine exit codes are ignored (upstream ignores them too — INITEX exits 1);
timeout, signal death, or a missing artifact is FAIL.

Comparisons gate exactly where upstream's `is_OK=false` gates: exact
`trip.pl`/`etrip.pl` round-trips and `tripos.tex`, plus the
accepted-difference filters (banner/date, memory statistics, x86 glue
rounding, DVI movements) over `tripin.log`, `trip.fot`, `trip.log`,
`trip.typ` (and the etrip `c*`/`x*`/`e*` counterparts). Raw diffs upstream
leaves ungated (e-TeX banners, memory usage, `etrip.out`) are INFO rows.
One deliberate deviation: inputs are copied, not symlinked, into the
workdir, so logs name bare `trip.tex` exactly as Knuth's expected files do.

Pin: see `PINS.txt` (no `texlive-2026` tag exists upstream; HEAD pinned).
