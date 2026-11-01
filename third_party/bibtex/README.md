# `third_party/bibtex`

Unmodified sources from TeX Live 2026: BibTeX, the source of record for the
BibTeX port `crates/bibtex` (lane RUST-TOOLS: no external programs at user
time; DESIGN.md §4.5).

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` (the pin of `third_party/pdftex`) |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` |
| Fetched | 2026-10-09 |

## Files

| path | upstream path | notes |
|---|---|---|
| `bibtex.web` | `texk/web2c/bibtex.web` | BibTeX 0.99e (Oren Patashnik); 11,671 lines |
| `bibtex.ch` | `texk/web2c/bibtex.ch` | TeX Live's change file for web2c (dynamic arrays, kpathsea, command line); 1,818 lines |

Checksums: `shasum -a 256 -c SHA256SUMS`.

## Licence

`bibtex.web`:

> This program is copyright (C) 1985, 1988, 2010, 2025 by Oren Patashnik;
> all rights are reserved.
>
> This program, BibTeX, is available under the same terms as
> Donald Knuth's TeX program.
>
> (Request to implementors: The WEB system provides for alterations via
> an auxiliary file; the master file should stay intact.)

So it is kept byte-for-byte unmodified, and every change is a WEB change file:
TeX Live's `bibtex.ch` (public domain, like TeX Live's other web2c change
files), then `crates/bibtex/changes/flashtex.ch`, both applied by
`tools/web2rust` as TANGLE applies them. `crates/bibtex/LICENSE` records what
covers the translation.

MacTeX/TeX Live's `bibtex` is an oracle only (the comparison in
`docs/evidence/rusttools-2026-10-09/bibtex/`); it is never in the product path.
