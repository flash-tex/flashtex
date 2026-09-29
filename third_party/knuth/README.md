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

Verify with:

```sh
shasum -a 256 -c third_party/knuth/SHA256SUMS
```
