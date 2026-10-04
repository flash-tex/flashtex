# The engine on Windows

How flashtex-engine (`flashtex-initex`, `flashtex-host`) behaves on Windows
where it differs from macOS and Linux, and what CI runs there. Owner goal:
the engine and CLI run on macOS, Linux, Windows and maybe WASM (DESIGN.md
§16). History: the port is PR #1406 (evidence in
[`docs/evidence/portability-2026-10-03/`](../evidence/portability-2026-10-03/README.md));
the hardening below is issue #1418 (lane WIN-HARDENING).

Every OS call is in `crates/flashtex-engine/src/os.rs` (DESIGN.md §16 rule 2).

## `\write18` and pipes: `cmd.exe`

web2c runs shell escapes through the C library's `system()` and `popen()`,
which on Windows means `%COMSPEC% /c CMD` (`cmd.exe`). The engine does the
same (`os::shell_command`), with these TeX Live behaviours, all deliberate:

- **Restricted quoting uses `"`** (texmfmp.c `QUOTE`): `kpsewhich
  --format="other text files" config` runs as `kpsewhich "--format"="other
  text files" "config"`. A `'` in a restricted `\write18` is a quotation
  error, as on Unix.
- **A pipe's `'` becomes `"`.** texmfmp.c's `runpopen` turns every `'` of a
  pipe's command (`\input|"cmd"`, `\openin`/`\openout` to `|cmd`) into `"`
  before the restricted check, on WIN32 only (TeX Live 2026,
  `texk/web2c/lib/texmfmp.c` lines 678-684 at commit `6a300188`). So
  `\input|"kpsewhich 'a b'"` runs on Windows and is a quotation error
  elsewhere. `system.rs` `popen_command`; the `.fls` records the rewritten
  command, as texmfmp.c's `open_out_or_pipe` does.
- **`%VAR%` expands inside quotes.** The restricted quoting protects spaces
  and `cmd.exe`'s `&|<>()^`, but `cmd.exe` expands `%VAR%` before it
  parses quotes, so `\write18{kpsewhich "%TEMP%"}` passes the expanded
  path. TeX Live's pdftex.exe does exactly this (texmfmp.c's
  `char_needs_quote` escaping is `#if 0`).
- **The working directory is searched first.** `cmd.exe` looks a command
  up in the current directory before `PATH`, so a project containing
  `kpsewhich.exe` (or `.bat`, `.cmd`, ...) runs that, even under restricted
  shell escape. TeX Live's pdftex.exe does the same (texmfmp.c's
  `SELFAUTOLOC` prefixing is `#if 0`). Untrusted projects compile with
  shell escape off until trusted (DESIGN.md §4.5, decision 9), which is
  the protection here as on every OS.

## The engine host

- **Socket files are owner-only.** `os::restrict_to_owner` replaces the
  socket's inherited ACL with a protected DACL holding one entry, full
  access for the user the host runs as (`%TEMP%`'s inherited ACL also
  admits Administrators and SYSTEM). An `AF_UNIX` `connect` needs write
  access to the socket file, so no other account can connect. The host's
  socket and each export channel's get it; a failure is reported on the
  host's socket and fatal on an export channel's.
- **The export channel checks its peer.** Windows has no `socketpair` and
  no numbered-descriptor inheritance, so an export child connects to a
  listener at `FLASHTEX_DISPLAY_LIST=socket:PATH`. The host reads only a
  connection whose peer process (`SIO_AF_UNIX_GETPEERPID`, Windows 10 1803
  and later) is the child it spawned, and closes any other. Where Windows
  cannot name the peer, the owner-only DACL is the only check. (Unix needs
  neither: the channel is a socket pair the child inherits.)
- **`FLASHTEX_INVOKED_AS`.** Windows cannot set a child's `argv[0]` apart
  from its program, so `os::engine_command` tells an engine child its name
  (`pdftex`) through this variable. Both `flashtex-host` and
  `flashtex-initex` (a host's `--engine`) take it out of both of Windows'
  environments (the process's and the C runtime's) at the start of `main`,
  so `\write18` children never inherit it.

## `\pdfmatch`

pdfTeX's own regex, `pdftexdir/regex` (glibc 2.5's, LGPL-2.1-or-later), as
TeX Live compiles it for MinGW: [`third_party/pdftex-regex`](../../third_party/pdftex-regex/README.md).
Its messages are glibc's (`Unmatched ( or \(`), as pdftex.exe's are.

## Memory

The word space is `VirtualAlloc(MEM_RESERVE | MEM_COMMIT)`: pages take no
RAM until touched, but the whole length counts against the commit limit at
once. Lazy commit (reserve, then commit on first touch, like `mmap` on
Unix) was reviewed for #1418 and not done: it needs a vectored exception
handler, and that does not cover the kernel. A `WriteFile` or `send` whose
buffer reaches an uncommitted page fails with `ERROR_NOACCESS` instead of
faulting, and the persisted S₀ and checkpoint files are written straight
from the word space. See `os::alloc_zeroed`.

## CI: `.github/workflows/portability.yml`

GitHub-hosted `windows-latest`, path-filtered, `contents: read`, no
secrets; outside `ci-required`. In order:

1. Build `flashtex-initex`, `flashtex-host`, `dl3-client` for
   `x86_64-pc-windows-gnu` (MSYS2 UCRT64 MinGW-w64).
2. display-list-v3 tests; the engine's unit tests, with the Windows-only
   ones: owner-only DACL on a file and on an `AF_UNIX` socket, the export
   channel refusing another process, `FLASHTEX_INVOKED_AS` removal, the
   pipe's quote rewrite and the restricted quoting through `cmd.exe`, `%VAR%`
   inside quotes, `\pdfmatch` with the vendored regex.
3. trip, etrip, pdfTeX's regression tests.
4. TeX Live for Windows, `scheme-minimal`, from CTAN (not pinned; oracle
   and engine read the same installation). Then, against its `pdftex.exe`
   (`.github/portability/`):
   - `compare.py write18`: INITEX with restricted `\write18` and a
     single-quoted pipe; the logs after the banner must be equal; then a
     full `\write18` with a redirection;
   - `host.sh`: `flashtex-host` on an `AF_UNIX` socket, `hello.tex` compiled
     live (resident engine, restricted) and as an export
     (`--engine flashtex-initex.exe`, full shell escape, frames through the
     export channel), with the socket's DACL shown by `icacls`;
   - `compare.py pt1`: P-T1 (DESIGN.md §1.1) on `pt1.tex`, both formats made
     as fmtutil makes `pdftex.fmt`, with `tools/parity/capture.py`'s traced
     pass and normalisation.

First green run: [37178610440](https://github.com/flash-tex/flashtex/actions/runs/37178610440)
(PR #1478, 8.6 min; TeX Live's install 63 s): the Windows-only unit tests
all pass; `write18` logs EQUAL to pdftex.exe's (pdfTeX 1.40.29, TeX Live
2026); live and export compiles 1 page each, both sockets `icacls`
`runneradmin:(F)` only, `env.txt` `clean`; P-T1 PASS (2 shipouts, 24,895
log bytes each). The first runs found one bug, fixed in the same PR: the
format cache synced the built format through a read-only handle, which
`FlushFileBuffers` refuses ("Access is denied"), so no format could be
built on Windows.

Not yet on Windows: the LaTeX parity fixtures (`engine-parity.sh`, which
needs a LaTeX installation and qpdf), the CLI (`flashtex build`), an
`x86_64-pc-windows-msvc` build of the C parts, the MinGW C build in
trip/etrip (they are pure Rust), and MiKTeX (not a target).
