# File resolver: kpathsea linked, not rewritten (2026-09-29)

Lane P1-TEX82-CORE, DESIGN.md §4.4 (files) under §1's **reuse before
building** rule. Decision: **(A) link TeX Live's kpathsea** — vendored
unmodified in `third_party/kpathsea/`, built by `crates/flashtex-engine/build.rs`
with `cc`, behind the `FileResolver` trait in `crates/flashtex-engine/src/resolver.rs`.
Option (B), a Rust kpathsea-compatible resolver, was evaluated on paper and not
built, for the reasons below.

## The bar ("no compromise")

1. Identical search results to `kpsewhich` on ≥ 500 names from a real TeX Live
   2026: ls-R, TEXMFHOME/LOCAL/DIST, texmf.cnf, format-specific paths, `.tex`
   suffix rules.
2. Lookup latency no worse than kpathsea's.
3. Works when launched from the GUI, i.e. without a shell environment.
4. Works against a Tectonic-style bundle directory.
5. Licence fits its side of the §3 boundary; maintained or small enough to own.

## Measurements (A)

Reproduce: `python3 docs/evidence/file-resolver-2026-09-29/run.py` (needs TeX
Live 2026). Machine: Apple M5 Pro, TeX Live 2026, kpathsea 6.4.2. Raw output:
`results.json`; the corpus: `corpus.tsv`.

**Corpus**: 577 lookups drawn deterministically from TEXMFDIST's own ls-R —
424 `tex` (sty, cls, tex, fd, def, cfg, clo, ldf, ltx; 30 bare names that need
`.tex` added; 30 basenames present in several directories; 10 in the wrong
case), 64 `tfm` (40 with, 20 without suffix), 22 `type1 fonts`, 17 `enc files`,
12 `map`, 12 `vf`, 12 `bst`, 5 `bib`, 8 `fmt` (engine subdirectories), 1 `cnf`,
and 24 names that exist nowhere. kpsewhich finds 541 of them. The reference is
`kpsewhich -progname=pdflatex -engine=pdftex -format=F NAME`, one process per
name.

| environment of our process | identical to kpsewhich | set-up | first pass (577) | steady lookup, median / p99 |
|---|---|---|---|---|
| login shell | **577 / 577** | 50.3 ms | 18.0 ms | 24.1 µs / 80.3 µs |
| GUI-like (launchd: HOME, USER, TMPDIR, PATH=/usr/bin:/bin:/usr/sbin:/sbin) | **577 / 577** | 52.2 ms | 17.8 ms | 23.3 µs / 80.0 µs |
| empty (`env -i`) | **577 / 577** | 51.0 ms | 15.9 ms | 23.4 µs / 46.3 µs |

No stderr output in any run. In the GUI and empty environments TeX Live is
found by probing (`find_texlive_bin`: `$FLASHTEX_TEXLIVE_BIN`, then
`/Library/TeX/texbin`, `/usr/local/texlive/<newest>/bin/*`, Homebrew,
`/usr/bin`); kpathsea is handed that directory as `argv[0]`, derives
SELFAUTOLOC/SELFAUTOPARENT from it exactly as the installed binaries do, and
finds texmf.cnf with no variable set.

**kpsewhich itself**: 58.2 ms per process (median); run once per format with
all of that format's names, 161 µs per `tex` lookup up to 55 ms for the single
`cnf` one (process start-up and ls-R loading spread over the batch). Set-up in
our process is the same work — kpathsea reading texmf.cnf and every ls-R —
done once per engine process rather than once per lookup.

**Bundle**: every file kpsewhich found was copied into one flat directory (539
distinct basenames) and the corpus resolved with
`KpathseaResolver::for_bundle`, which points every kpathsea search variable at
that directory: **576 / 577** return a file with the same basename as kpsewhich
(steady 8.3 µs). The one difference is inherent to flattening: `texmf.cnf`
looked up in the `tex` format is not on pdflatex's TEXINPUTS in TeX Live, but
in a single flat directory it is. `tests/resolver_bundle.rs` checks the suffix
rules on a synthetic bundle without TeX Live, so CI runs it.

## Why not (B)

§1 says to build only if it is measurably faster on a path users notice, or if
nothing existing meets the bar. kpathsea is the reference implementation, so
(A) meets criterion 1 by construction and measures 577/577; (B) would have to
reproduce, and keep reproducing across TeX Live releases, texmf.cnf parsing
with brace expansion and `$VAR` expansion, progname- and engine-qualified
variables (`TEXINPUTS.pdflatex`, `$engine` subdirectories for formats), `!!`
(ls-R only) and `//` (recursive) elements, ls-R's directory-order tie-breaking
(30 duplicated basenames in the corpus), `texmf_casefold_search`, and the
per-format suffix and `try_std_extension_first` rules. That is the behaviour of 11,697 lines of library C (plus 4,142 of headers)
we would be shadowing to reach the same 577.

On speed, the paths that matter are a warm compile (a few hundred lookups at
~24 µs: single-digit milliseconds) and engine start-up (~50 ms once per resident
process, not per compile). Neither is noticeable against the §1.2 targets, so a
faster wheel has nothing to win. Licence: LGPL-2.1-or-later inside the
GPL-2.0-or-later engine is fine. Maintenance: upstream is maintained; our
footprint is 124 unmodified files plus three small headers and an 81-line C shim.

## Known costs and limits of (A)

- kpathsea keeps configuration in the process environment (`kpathsea_xputenv`
  is `putenv`), so there is one resolver per process. The engine is one
  document per process, so this costs nothing today.
- kpathsea calls `exit` on a few configuration errors — e.g. when it cannot
  find the directory of `argv[0]`, which bit the first bundle test. The
  resolver always passes an existing path. Hardening against the rest (a
  missing texmf.cnf is only a warning) is a P2 follow-up if it ever matters.
- mktex programs are disabled, as in a plain `kpsewhich` lookup; web2c's
  pdfTeX would run mktextfm for a missing TFM. Missing fonts are reported, not
  generated.
- `tex.web`'s `start_input` appends `.tex` before the lookup (§537), whereas
  web2c passes the name as typed; the two differ only for a file `foo` with no
  `foo.tex` beside it. That changes with pdftex.web's own change files.
