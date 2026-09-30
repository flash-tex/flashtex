# `third_party/knuth`

Unmodified sources from Knuth's TeX distribution, used as the source of record
for the engine port (`docs/design/engine-v2/DESIGN.md` §4.1, decision D3).

## `tex.web`

| | |
|---|---|
| Version | TeX 3.141592653 (`@d banner=='This is TeX, Version 3.141592653'`, §193) |
| Source | <https://mirrors.ctan.org/systems/knuth/dist/tex/tex.web> |
| Fetched | 2026-09-29 |
| Lines | 25,010 |
| sha256 | `c62ab513ef167e93f71a23bd34f311e243210afd7c7a0f9b779614b71e398324` |

The file's own header states its licence:

> This program is copyright (C) 1982 by D. E. Knuth; all rights are reserved.
> Unlimited copying and redistribution of this file are permitted as long as
> this file is not modified. Modifications are permitted, but only if the
> resulting file is not named tex.web.

It is therefore stored here **byte-for-byte unmodified**, with the original
header intact, and is never edited. Everything the port needs to change is
expressed in `tools/web2rust` (the translator) or in
`crates/flashtex-engine/src/system.rs` (the hand-written system-dependent part
that `tex.ch` supplies in web2c). Because the derived engine crate is a
translation of this file, `crates/flashtex-engine` is GPL-2.0-or-later
(`crates/flashtex-engine/LICENSE`).

`tex.web` has no companion change file here: web2c's `tex.ch` is *not* used, per
§4.1 ("the system-dependent behaviour web2c adds ... is re-specified by us").

## `trip/`: the trip test

Knuth's trip-test files, unmodified, from
<https://mirrors.ctan.org/systems/knuth/dist/tex/> (fetched 2026-09-29), plus
one file derived from them. They are what tripman.tex calls the "master"
files; `scripts/flashtex-trip.sh` and the CI job `trip` compare against them.

| file | sha256 | role |
|---|---|---|
| `trip.tex` | `15f15c2ca1470085299056ec89dea5f51e9fe9303ef25581b2f2eaf7809ae97b` | the test input |
| `trip.pl` | `93b38cc794f0c4a462667e25ef34a83552cbcdd62a42b10f739a431166525a79` | property list of the test font |
| `tripin.log` | `ba01328756a8901d7c38162c9012014e9540322bf0963e105286f2a6ccb494cc` | master log of step 3 (Appendix D) |
| `trip.log` | `61a653523bdccab9fd3f9aa61d170d0198c322c951938327b7daef9b70f26d8b` | master log of step 4 (Appendix E) |
| `tripin.fot` | `10531728b618cd95e60552753c4d3c321373ca625ccfae1933c1ad545b71f922` | master terminal output of step 3 |
| `trip.fot` | `89e275ac12d025c06022e8dd6eb556b765954af2654b39ac2fbd451cf631b370` | master terminal output of step 4 (Appendix H) |
| `trip.typ` | `64efc62b962c592c2973f8c45a78e9e5d473f8b9da53ee53bc275a98041675cc` | DVItype of the master `trip.dvi` (Appendix F) |
| `tripos.tex` | `ea7447c7a8f2de278d2f84474f22c48c9d8a0059d7e16edd578d0bbe7077b47f` | master `\write` output of step 4 |
| `trip.tfm` | `2c94bdba9c769e885f357823a183aaa5d2267731075f040f2a03cf6442a26181` | **derived**, see below |

`trip.tfm` is generated so that CI needs no TeX binaries: `pltotf trip.pl
trip.tfm` with PLtoTF 3.6 (TeX Live 2026) on 2026-09-29. tripman.tex step 1's
own check passes: `tftopl trip.tfm tmp.pl` gives a `tmp.pl` byte-identical to
`trip.pl`.

## Verifying

```sh
cd third_party/knuth && shasum -a 256 -c SHA256SUMS
```
