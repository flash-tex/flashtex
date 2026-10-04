# `third_party/pdftex-regex`

pdfTeX's own copy of glibc 2.5's POSIX regular expressions,
`texk/web2c/pdftexdir/regex/`, vendored **unmodified** for `\pdfmatch` on
Windows (issue #1418).

pdfTeX's `utils.c` (`matchstrings`) calls `regcomp`/`regexec` from
`<regex.h>`. On Unix that is the C library's; Windows' C runtimes have none,
so TeX Live compiles this copy into `libpdftex.a` `if MINGW32`
(`pdftexdir/am/libpdftex.am`, lines 43-48 at the pin below, with
`REGEX_INCLUDES = -I$(srcdir)/pdftexdir/regex`). The engine does the same:
`crates/flashtex-engine/build.rs` compiles `regex/regex.c` (which includes
the other three `.c` files) for any Windows target, with this directory and
`regex/` on the include path, so `csrc/flashtex_regex.c`'s `<regex.h>` is
this one. On macOS and Linux the C library's regex is used, as pdfTeX does.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` (the same pin as `third_party/pdftex`) |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` |
| Path | `texk/web2c/pdftexdir/regex/` (all 8 files) |
| Version | glibc 2.5's regex, as packaged for MinGW by Tor Lillqvist and Keith Marshall (`regex/README`), with TeX Live's 2017 change to `regex_internal.h` (Karl Berry) |
| Fetched | 2026-10-04 |
| Licence | LGPL-2.1-or-later (`regex/COPYING.LIB`; every source file's header says "version 2.1 of the License, or (at your option) any later version"). Linked statically into the GPL-2.0-or-later engine, which LGPL-2.1 §3 permits, as kpathsea is. It is on the engine side of the DESIGN.md §3 boundary; nothing MIT links it. |

All 8 files are byte-identical to that commit (`SHA256SUMS`; check with
`shasum -a 256 -c SHA256SUMS` in this directory).

## How it is compiled

As TeX Live compiles it: no `config.h` and no `HAVE_*` macro, so none of
glibc's locale, wide-character or `libintl` code (`RE_ENABLE_I18N` stays
undefined; the regex is byte-oriented, as the README says), and `bool` is
the file's own `enum`. That `enum` names `false` and `true`, which are
keywords in C23, the default of GCC 15 and later, so build.rs asks for
`-std=gnu17`, the dialect TeX Live's compilers use. Warnings are off, as for
the other vendored C.

To update: fetch `texk/web2c/pdftexdir/regex/` at the new TeX Live pin,
replace these files, regenerate `SHA256SUMS`, and check `libpdftex.am` for a
change in when and how they are compiled.
