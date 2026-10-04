# `third_party/xetex`

Unmodified sources from TeX Live 2026, the source of record for the XeTeX
port (`crates/flashtex-xetex`, `docs/design/xetex/PLAN.md`), kept exactly as
`third_party/pdftex` keeps `pdftex.web` for the pdfTeX engine
(`docs/design/engine-v2/DESIGN.md` §4.1).

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1`, the same commit as `third_party/pdftex` |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` |
| Version | XeTeX 3.141592653-2.6-0.999998 (`xetex -version` of TeX Live 2026: `XeTeX 3.141592653-2.6-0.999998 (TeX Live 2026)`) |
| Fetched | 2026-10-04 |

`xetex.web` is byte-identical at the commit `ce624c27` of the mirror that the
feasibility spike of 2026-10-04 used.

## Files

| path | upstream path | notes |
|---|---|---|
| `xetex.web` | `texk/web2c/xetexdir/xetex.web` | 34,428 lines |
| `char-warning-xetex.ch` | `texk/web2c/xetexdir/char-warning-xetex.ch` | TeX Live's public-domain feature change file, applied unmodified |
| `COPYING` | `texk/web2c/xetexdir/COPYING` | XeTeX's licence |
| `tests/bug73.tex`, `tests/bug73.log`, `tests/ctrlsym.tex`, `tests/ctrlsym.log`, `tests/filedump.tex`, `tests/filedump.log` | `texk/web2c/xetexdir/tests/` | XeTeX's own tests (`xetexdir/xetex-bug73.test`, `xetex-ctrlsym.test`, `xetex-filedump.test`); each says it may be freely used, modified and distributed |
| `tests/basic.tex`, `tests/dump-basic.tex` | `texk/web2c/tests/` | the format those tests make (`basic.tex` is plain.tex without fonts or hyphenation; its notice permits copying and redistribution) |

TeX Live's other feature change files that XeTeX uses (`tracingstacklevels.ch`,
`partoken-102.ch`, `partoken.ch`, `locnull-optimize.ch`,
`unbalanced-braces.ch`, `showstream.ch`) are pdfTeX's too, and are used from
`third_party/pdftex/web2c/`, unmodified.

Checksums of every file are in `SHA256SUMS`:

```sh
cd third_party/xetex && shasum -a 256 -c SHA256SUMS
```

## Licence

`xetex.web` states its own licence: XeTeX's parts are MIT-licensed
(`COPYING`; Copyright (c) 2009-2026 Jonathan Kew, 1994-2008 SIL International,
2010-2012 Han The Thanh, 2012-2013 Khaled Hosny), and its e-TeX parts say:

> e-TeX is copyright (C) 1999-2012 by P. Breitenlohner (1994,98 by the NTS
> team); all rights are reserved. Copying of this file is authorized only if
> (1) you are P. Breitenlohner, or if (2) you make absolutely no changes to
> your copy.

So, like `pdftex.web`, it is kept **byte-for-byte unmodified** and is never
edited: every change is a WEB change file in
`crates/flashtex-xetex/changes/`, applied by `tools/web2rust` exactly as
TANGLE applies one (DESIGN.md §3, §4.1). The translation,
`crates/flashtex-xetex`, is GPL-2.0-or-later because it links the pdfTeX
engine's runtime (`crates/flashtex-engine`, GPL-2.0-or-later); XeTeX's MIT
notice is reproduced in that crate's `LICENSE`. Nothing MIT may link it.
