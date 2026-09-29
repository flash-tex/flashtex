# `third_party/kpathsea`

TeX Live's path-searching library, vendored **unmodified** as flashtex-engine's
file resolver (DESIGN.md §4.4; decision record
[`docs/evidence/file-resolver-2026-09-29/`](../../docs/evidence/file-resolver-2026-09-29/README.md)).

| | |
|---|---|
| Version | kpathsea 6.4.2 (TeX Live 2026; `kpsewhich --version` on the reference install says the same) |
| Upstream | <https://github.com/TeX-Live/texlive-source>, `texk/kpathsea/` |
| Revision | branch `branch2026`, commit `fb6158926661cb7a7246b3a94a0cb170a9624d5a` |
| Fetched | 2026-09-29 |
| Licence | LGPL-2.1-or-later (`COPYING.LESSERv2`). Linked statically into the GPL-2.0-or-later engine, which LGPL-2.1 §3 permits. It is on the engine side of the §3 boundary; nothing MIT links it. |

All 124 files here are byte-identical to that revision (`SHA256SUMS`; check with
`shasum -a 256 -c SHA256SUMS` in this directory). Omitted, because they are only
autotools machinery or history: `ChangeLog`, `Makefile.in`, `PROJECTS`,
`aclocal.m4`, `configure`, `configure.ac`, `kpathsea.pc.in`. `Makefile.am` is
kept because it is the authority for the source list and preprocessor flags.

What configure would generate lives outside this directory, in
`crates/flashtex-engine/kpathsea-config/kpathsea/`:

- `c-auto.h` — hand-written answers for macOS and 64-bit glibc Linux; every
  macro is one from `c-auto.in`. `MAKE_TEX_*_BY_DEFAULT` are left undefined:
  the engine never runs mktextfm/mktexfmt.
- `paths.h` — generated from this `texmf.cnf` by `scripts/kpathsea-paths-h.sh`,
  the pipeline of `Makefile.am`'s `stamp-paths`. Only kpathsea's compiled-in
  fallback paths come from it; the user's own `texmf.cnf` is read at run time.
- `kpathsea.h` — the `stamp-kpathsea` header list, written out.

`crates/flashtex-engine/build.rs` compiles `libkpathsea_la_SOURCES` for a
non-Windows host with `-DMAKE_KPSE_DLL -DHAVE_CONFIG_H`, as `Makefile.am` does.

To update: fetch `texk/kpathsea/` at the new TeX Live release branch, replace
these files (keeping the omissions above), regenerate `SHA256SUMS` and
`paths.h`, check `c-auto.in` for new macros, and rerun
`docs/evidence/file-resolver-2026-09-29/run.py`.
