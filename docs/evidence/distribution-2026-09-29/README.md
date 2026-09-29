# Distribution: format cache, TeX Live discovery, bundle fallback (2026-09-29)

Lane P3-DISTRIBUTION, DESIGN.md §4.4 (files and distribution, D12), §4.5, and
§1's **reuse before building**. Code: `crates/flashtex-engine/src/formats.rs`,
`src/bundle/`, `src/resolver.rs` (discovery, `find_all`, `for_bundle_tree`),
`src/system.rs` (the read-set hook and the fallback to the cache in
`find_format`), `src/bin/flashtex-dist.rs`. Reproduce everything below with
`docs/evidence/distribution-2026-09-29/run.sh` (needs TeX Live 2026 and qpdf).

Machine: Apple M5 Pro, TeX Live 2026 (MacTeX), shared with other lanes'
builds (load average 7 to 11 during the runs), so the times are upper bounds.

## Decisions

| question | decision | why |
|---|---|---|
| How is a format built? | As `fmtutil` builds it: the merged `fmtutil.cnf` files (`kpsewhich -all`, first file wins, `#!` disables) give the line; the engine runs as `pdftex -ini -jobname=F -progname=F ARGS </dev/null` in an empty directory. | This is `fmtutil.pl`'s `rebuild_one_format` (TeX Live 2026, read from the installed script). Patterns come from whatever `language.dat`/`language.def` the ini files read: the user's. |
| What is the cache keyed by? | SHA-256 of the engine binary, the command line, every file the INITEX run opened (content hash) and every lookup's result, recorded by the engine itself (`FLASHTEX_READ_SET`, system.rs). | Lookup results catch a new file that would now shadow another (e.g. a `hyphen.cfg` added to TEXMFHOME), which content hashes of the files read cannot. |
| How is a hit validated? | `stat` each file (hash only when size/mtime/ctime/inode changed; a touched but unchanged file is rehashed once and its signature refreshed); repeat the lookups through the process's resolver. | Cheap and exact (numbers below). |
| Concurrency | `flock` per slot, re-validate after acquiring; formats and manifests written to temporary names, fsynced, renamed; one previous generation kept for readers. | Two app windows build once (tested with two processes). |
| TeX Live discovery | `FLASHTEX_TEXLIVE_BIN`, the process's PATH, the login shell's PATH as `path_helper(8)` builds it (`/etc/paths`, `/etc/paths.d/*`, where MacTeX installs `TeX`), `/Library/TeX/texbin`, `install-tl`'s `TEXDIR`s (`/usr/local/texlive/<year>`, `~/texlive/<year>`, `/opt/texlive/<year>`), Homebrew and system directories. The first directory with `kpsewhich` is TeX Live; the report says which rule found it. | A GUI app's PATH is `/usr/bin:/bin:/usr/sbin:/sbin`; `path_helper` is what gives Terminal its PATH. |
| Bundle | **Reuse Tectonic's TTBv1 container format** (spec and digest rule, so Tectonic's tools read our bundles), **not Tectonic's bundles, and not the `tectonic_bundles` crate.** Content is our selection of unmodified TeX Live 2026 files; kpathsea resolves them with TeX Live's own `texmf.cnf`. | See the evaluation below. |

## Bundle: what was evaluated

Sources: GitHub API reads of `tectonic-typesetting/tectonic` (0.17.0,
2026-07-27, pushed 2026-08-01) and `tectonic-typesetting/tectonic-texlive-bundles`
(archived; its builder now lives in the main repository as `tectonic bundle create`).

| criterion (DESIGN §1) | Tectonic's bundles | `tectonic_bundles` crate | TTBv1 format + our reader |
|---|---|---|---|
| Content unmodified (§4.4, LPPL clause 6 never needed) | **No**: both `texlive2023` and `texlive2024-0312` apply `patches/texlive/{latex.ltx,listings.sty,fontawesome.sty,fithesis-mu-base.sty}.diff`; the `latex.ltx` patch replaces the missing-file prompt with `\errmessage` | n/a | Yes: files are copied byte for byte from the user's-version TeX Live |
| Parity with pdfTeX 1.40.29 / TeX Live 2026 | **No**: TeX Live 2023 and 2024 snapshots; search order `tex/{xelatex,latex,xetex,plain,generic}//` (XeTeX first); formats (`*.fmt`) excluded and built by Tectonic from its own `tectonic-format-latex.tex` | Resolves names with the bundle's own `SEARCH` spec, not kpathsea | Lookups are kpathsea's with TeX Live 2026's `texmf.cnf` (P-T1 below) |
| Integrity | TTBv1 carries a SHA-256 per file and a bundle digest | verifies against the digest | digest pinned; index accepted only if it implies the digest; every file checked against its SHA-256 |
| Byte ranges | yes (one request per file; a learned working-set prefetch re-warms a cache) | yes, over `tectonic_geturl` (reqwest or curl) | yes: header, index, then the **core in one request**, then **a whole package per request** when it is ≤ 256 KiB compressed, else the file |
| Licence | MIT | MIT (fine in the GPL engine) | spec is MIT; our reader is GPL like the engine |
| Size / coupling | — | pulls `tectonic_io_base`, `tectonic_status_base`, `tectonic_errors`, `tectonic_geturl` (reqwest/tokio or curl), `zip`, `url`, `chrono` | ~700 lines, `sha2` plus TeX Live's zlib already linked; HTTPS through the system `curl` |
| Maintained | 2024 bundles; bundle repo archived | maintained with Tectonic | ours |

So the content and the lookup semantics had to be ours (Tectonic's are a
different TeX Live, patched, XeTeX-ordered), while the container format met
the bar and is reused as specified: `tectonicbundle` magic, 66-byte header,
gzip members, `[FILELIST]` index, and the digest = SHA-256 of `FILELIST`
(`<sha256|nohash> <path>` sorted by path components). FlashTeX adds
`[FLASHTEX:PACKAGES]` (each TeX Live package's byte range; members are stored
package by package) and `[FLASHTEX:CORE]`, which TTBv1 readers ignore.

Lookup: the bundle's files keep their TeX Live paths under
`<cache>/<digest>/tree` (`texmf-dist/…`, and `texmf-var/…` for what TeX Live
generates at install: `pdftex.map`, `language.dat`), each tree gets an `ls-R`
written from the index, and kpathsea runs with TeX Live's own
`texmf-dist/web2c/texmf.cnf` from the bundle, with only the tree variables set
(`TEXMFROOT`, `TEXMFDIST`, `TEXMFSYSVAR`; user and site trees point at empty
directories). Every search path and setting is therefore TeX Live's: the first
attempt without `texmf.cnf` failed P-T1 on `log_openout` (`\openout1 = ...`
missing from the log), which is exactly the kind of difference this removes.
kpathsea only returns an `ls-R` entry that exists on disk, so the files a
lookup could return are created as empty placeholders just before it, and the
one kpathsea picks is fetched and verified before its path is returned.

## Measurements (verified)

### TeX Live discovery

| environment | TeX Live found by | identical to `kpsewhich` |
|---|---|---|
| login shell | PATH → `/Library/TeX/texbin` → `/usr/local/texlive/2026/bin/universal-darwin` | **22/22** (14 variables incl. TEXMFHOME/TEXMFCNF/TEXINPUTS, 7 lookups, `-all fmtutil.cnf`) |
| launchd-like (`env -i`, `PATH=/usr/bin:/bin:/usr/sbin:/sbin`) | login-shell PATH (`/etc/paths.d/TeX`) | **22/22** |

kpathsea set-up plus the first lookup: 52.8–55.4 ms (once per engine process).

### Format cache

| | pdflatex | pdftex |
|---|---|---|
| first use: INITEX build, read set hashed, stored | **4.13 s** (4.1–4.3 s over 6 A/B runs) | 0.49 s |
| read set | 240 files, 255 lookups | 229 files, 232 lookups |
| hit validation, in process (median of 30) | **16.0–16.4 ms**, of which lookups 11.3 ms | 16.9–18.1 ms |
| minimal article end to end, format via `FLASHTEX_FORMATS` vs cache hit (medians of 25, alternated) | 190.8 ms vs 209.1 ms: **+18.3 ms** | |
| minimal article, first use (build + run) | 4.55 s | |

Tests: `tests/format_cache.rs` (hermetic, CI: miss, hit, touched-unchanged
file → rehash once then stat hit, changed file → rebuild and the previous
generation kept, a newly shadowing file → rebuild, back to an old key;
`#!`-disabled and non-pdftex lines refused; two processes → one build),
`tests/format_cache_texlive.rs` (a cached `pdflatex.fmt` and one built by
hand with fmtutil's command line give byte-identical `\tracingall` logs and
PDFs, and so does the next run, a hit).

### Bundle (local fixture bundle, local HTTP server; no network)

The fixture bundle (`flashtex-dist bundle-pack`): every file the 82 parity
fixtures and the pdflatex/pdftex format builds read, their whole TeX Live
packages, TeX Live's `texmf.cnf` and `fmtutil.cnf`: **10,248 files, 134
packages, 158 MB**; index 546,721 bytes gzipped (1.39 MB raw); **core 2,784,030
bytes** = the 246 files (14.2 MB) the pdflatex format build and a minimal
article read.

| minimal article (`\documentclass{article}` … Hello), fresh caches | requests | bytes | time |
|---|---|---|---|
| cold (header, index, core; then the format build from the bundle) | **3** | **3,330,817** | 5.9 s (bundle open ≈ 0.3 s; the rest is the 4.1–5.5 s format build) |
| warm | 0 | 0 | 119–187 ms |
| offline (`FLASHTEX_BUNDLE_OFFLINE=1`) | 0 | 0 | 116–166 ms |
| then `amsmath.sty` (package range) | +1 | +62,824 | |

Found on the way: an `fsync` per stored file (`F_FULLFSYNC` on macOS) cost
about a second of the cold start for the core's ~270 files. Bundle members
are now written atomically without `fsync` (they are a cache of verified
content); manifests, indexes and formats are still fsynced.

**A whole-TeX-Live bundle's index** (`flashtex-dist index-estimate`: all
runfiles of the 4,479 installed packages, 178,203 files): **26.0 MB raw,
8.48 MB gzipped**. The SHA-256s alone are ~5.7 MB of incompressible bytes, so
with TTBv1's single index this would dominate a cold start (3.3 MB above). See
open items.

Tests: `tests/bundle_fetch.rs` (hermetic, CI): a wrong pin is refused; offline
with an empty cache is an error, not a fetch; cold open is 3 requests; core
files need none; a small package comes in one request, a large one file by
file; kpathsea's suffix rules over placeholders; offline on a warm cache finds
what was fetched and nothing else; a corrupted member is refused; `file://`;
and the engine end to end building its format from a served bundle, cold then
warm offline with no request, with and without a `texmf.cnf` in the bundle
(its `max_print_line` visibly applies).

### Parity (P-T1/P-T2, tools/parity fixtures tier, 82 documents)

| engine files from | P-T1 | P-T2 |
|---|---|---|
| TeX Live, formats built by hand into `FLASHTEX_FORMATS` (baseline, this branch) | 75/82 | 75/82 |
| TeX Live, **format cache** (built on first use, no `FLASHTEX_FORMATS`) | **75/82** | **75/82** |
| **fixture bundle** (`file://`, fresh caches, format built from the bundle) | **75/82** | **75/82** |

Per document the three are identical; the 7 failures are the beamer fixtures,
which need `\pdfximage` (PR #1202, not merged into this branch). The bundle
row is `pt_bundle.py`: tools/parity's own `run_tex`, `oracle`, `compare_pt1`
and `compare_pt2`, with one normalisation added: the bundle's tree prefix
(`…/bundles/<digest>/tree/`) is replaced by the oracle's TEXMFROOT
(`/usr/local/texlive/2026/`) in the candidate's log, since the log names
every file it opens.

Also green on this branch: trip, etrip, `web2rust` drift test, licence
boundary.

## Beliefs, not measured

- A cold start from a real whole-TeX-Live bundle over the internet costs the
  index (8.5 MB) plus the same core (2.8 MB) plus round trips; not measured
  (no hosted bundle exists).
- The format cache's lookup check (11 ms) could be replaced by stat
  signatures of each tree's `ls-R` plus the directories of the non-`!!` path
  elements; expected to cut a hit to a few ms. Not built.

## Open items

1. **Index size for a full TeX Live bundle** (8.5 MB): a two-level index
   (basename → package map, ≈0.7 MB estimated; per-package file lists with
   their SHA-256s stored at the start of each package range) would keep the
   cold start near the core's size. It would be a FlashTeX extension beside
   TTBv1's index.
2. **Hosting and pinning a real bundle**: a CI job running `flashtex-dist
   bundle-pack` over a TeX Live 2026 install, a host with HTTP range support,
   and the pinned digest in the app. Owner decision (hosting).
3. `SOURCE_DATE_EPOCH`/`FORCE_SOURCE_DATE` pass through to format builds, as
   fmtutil's do; the format's banner date is not part of the key.
4. Bundle members are not fsynced; after a power loss a cached member could
   be damaged but keep its size. A verify pass (hash the tree against the
   index) is the fix if it matters.
5. kpathsea keeps its configuration in the process environment, so one
   resolver per process; the INITEX child inherits the parent's (same
   resolver, same values).
6. The host (lane P3-DISPLAYLIST) can warm the cache at start-up with
   `flashtex_engine::formats::ensure_format`; the engine process already
   falls back to it in `find_format`. Not wired into `src/host` here.
