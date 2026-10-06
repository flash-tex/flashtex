# Engine portability: Windows batch build and a WASI spike (2026-10-03)

Lane ENGINE-PORTABILITY (mac-claude-a, mac-m1max-a), PR #1406. Scope: the new
engine (`crates/flashtex-engine`: `flashtex-initex`, `flashtex-host`,
`flashtex-dist`) and `crates/display-list-v3`. The old engine is frozen and
out of scope. Owner goal: the engine and CLI run on macOS, Linux, Windows and
maybe WASM. DESIGN.md §16 keeps the cross-platform *product* (app shell,
packaging) for later; this lane carries out §16's "adopted now" rules 1, 2
and 6 for the engine and measures how far its batch path is from Windows and
WASI.

Labels: **[measured]** = run on mac-m1max-a with the commands below;
**[CI]** = the proposed `portability` workflow on PR #1406 (`windows-latest`);
**[belief]** = not run.

**Summary.** On `windows-latest` the MinGW-built engine and the MSVC-built
trip/etrip packages pass everything the Unix CI runs without TeX Live:
trip (5 PASS, `trip.typ` skipped: no dvitype), etrip (18/18), pdfTeX's
regression tests (6 PASS, `wcfname.test` skipped: no kpsewhich/perl), the
engine's unit tests (72/72) and display-list-v3's tests over Windows
`AF_UNIX` [CI, run 37110451880 at 8696178e4]. macOS output is unchanged
[measured]. WASI: the batch engine runs under wasmtime and its output is
byte-identical to the native engine's [measured]; about 1–1.5
engineer-weeks to a supported WASI CLI.

## 1. Audit: what blocked each target, and where it stands

Before = `origin/main` 442ca71bb; after = this branch. "Fixed" means the
code builds for the target and, on macOS and Linux, the Unix path is the
code the call sites had before.

| # | Site (before) | Unix-only mechanism | Windows | WASI (wasm32-wasip1) |
|---|---|---|---|---|
| 1 | display-list-v3 `widen_socket_buffers`, `client.rs`, `endpoint.rs` | `std::os::unix::net`, `std::os::fd`, Linux setsockopt constants as the non-macOS fallback | **fixed**: `transport.rs`, Winsock `AF_UNIX` (`WSASocketW`, not inherited by children) in a `TcpStream` [CI: tests pass] | compiles; sockets return `Unsupported` |
| 2 | `host/server.rs` listener, mode 0600 | `UnixListener`, `PermissionsExt` | **fixed**: `transport::Listener`; the socket file keeps its directory's ACL | n/a |
| 3 | `host/server.rs` `start_export` | socketpair + `pre_exec(dup2(fd, 3))`, `fd:3` | **fixed**: `os::ExportChannel`, a one-shot listener, `socket:PATH` [belief: not run] | n/a |
| 4 | `server.rs` ×3, `formats.rs` | `CommandExt::arg0("pdftex")` | **fixed**: `os::engine_command` → `FLASHTEX_INVOKED_AS=pdftex`, read by `os::invoked_as` [belief: not run] | spawn `Unsupported` |
| 5 | `arena.rs` word space | `mmap(MAP_ANON)`, Linux constant fallback | **fixed**: `VirtualAlloc(MEM_RESERVE\|MEM_COMMIT)` [CI] | **fixed**: zeroed allocator memory [measured] |
| 6 | `host/mod.rs` S₀ file | `mmap` of a file | **fixed**: `CreateFileMappingW`/`MapViewOfFile` | read into memory |
| 7 | `pdftex/utils.rs` dates | `localtime_r`/`gmtime_r` | **fixed**: `_localtime64_s`/`_gmtime64_s` | wasi-libc (UTC) |
| 8 | `incr.rs` `thread_cpu_s` | `clock_gettime(CLOCK_THREAD_CPUTIME_ID)` | **fixed**: `GetThreadTimes` | monotonic time |
| 9 | `system.rs` `\write18` | `/bin/sh -c`, `OsStrExt` | **fixed**: `%COMSPEC% /c`, raw command line | spawn `Unsupported` |
| 10 | restricted-mode quoting | `QUOTE = '\''` | **fixed for parity**: `"` and texmfmp.c's `--opt="…"` → `"--opt"="…"` rule under `WIN32` [CI: unit test] | as Unix |
| 11 | `program_name_from_argv0` | splits on `/` | **fixed**: also `\` and drive `:`, `.exe` in any case (kpathsea `xbasename`) [CI: unit test] | as Unix |
| 12 | stat stamps (`system.rs`, `formats.rs`) | `MetadataExt` ino/dev/ctime | **fixed**: `os::file_stat`; ino/dev 0, ctime = creation time | as Windows |
| 13 | `resident.rs` `apply_changes` | `FileExt::write_all_at` | **fixed**: `seek_write` loop | seek + write |
| 14 | `host/crash.rs` | POSIX `signal`, `open`/`write` | **fixed**: panic and exit lines only (no console-control handler yet) | n/a |
| 15 | `flashtex-dist`, 3 tests | `std::os::unix::fs::symlink` | **fixed**: `os::link_executable` (hard link, else copy) | n/a |
| 16 | `resolver.rs` TeX Live discovery | `kpsewhich`, `/usr/local/texlive`, `$HOME` | **fixed**: `kpsewhich.exe`, `%SystemDrive%\texlive\<year>\bin\<arch>`, no `\\?\` canonicalisation [belief: no TeX Live on the runner] | n/a |
| 17 | `resolver.rs` working-directory resolver | `Path::new(".").join(name)` | **fixed**: kpathsea's literal `./NAME` (it was `.\NAME`: etrip logs said `(.\trip.tex`); `;` as ENV_SEP [CI: etrip] | as Unix |
| 18 | `system.rs` `out_key` | `/` only | **fixed**: `\` is `/` (a restore was refused as "changed by another program" instead of "opened for output again") [CI] | as Unix |
| 19 | `-cnf-line`, `TEXMF_OUTPUT_DIRECTORY` | `std::env::set_var` | **fixed**: the CRT's environment is a separate copy on Windows; `os::set_env` also calls `_putenv_s` (cnfline.test failed without it); `;` kept in values, as kpathsea's cnf.c does there [CI] | as Unix |
| 20 | `build.rs` kpathsea | Unix `c-auto.h` and source list | **fixed**: Makefile.am's `WIN32` lists (MinGW: `mingw32.c`, `knj.c`, `xfseeko.c`, `xftello.c`; MSVC: `win32lib.c`, `knj.c`, getopt), `_WIN32` answers in `c-auto.h` | **blocker**: `tex-make.c` `<sys/wait.h>`, `tilde.c` `<pwd.h>` |
| 21 | `csrc/flashtex_regex.c` | `<regex.h>` | **open**: none in Windows C runtimes; `\pdfmatch` says "not available" until pdfTeX's own `pdftexdir/regex` (what TeX Live's Windows pdfTeX uses) is vendored | wasi-libc has `<regex.h>` |
| 22 | xpdf | `<pwd.h>` (`gfile.cc`) | **fixed**: `shell32`, `advapi32` linked *after* the static libraries (GNU ld order); libstdc++ static | **blocker**: `<pwd.h>` (a 9-line shim fixes it) |
| 23 | C formatting (`pdftex/cfmt.rs`) | the C library's `snprintf`/`sscanf` | MinGW must use **UCRT**: msvcrt prints `1e-005` (unit test failed on MSYS2 MINGW64); MSVC needs `legacy_stdio_definitions` (inline in the UCRT headers) [CI] | wasi-libc |
| 24 | libpng | `setjmp` | builds | needs wasm exception handling |
| 25 | threads | `std::thread` | works | **blocker for the host**: no threads in wasip1 |
| 26 | `memstat.rs` | `mincore`, `/proc`, `proc_pid_rusage` | already `None` (measurement only) | as Windows |

Already portable, unchanged: the format-cache lock (`File::lock`), the
little-endian S₀ codec, `clone_file`'s copy fallback, `cfile::os_path`,
`split_paths`, the cache-root resolver (`%LOCALAPPDATA%\FlashTeX`). The
software copy-on-write checkpoints (DESIGN.md §5.2) needed nothing.

## 2. Windows

### Cross-build on macOS [measured]

Homebrew `mingw-w64` 14.0.0 (GCC 16.2.0, UCRT), rustc 1.100.0-nightly:

```
rustup target add x86_64-pc-windows-gnu
cargo check -p flashtex-display-list --target x86_64-pc-windows-gnu --all-targets   # clean
cargo check -p flashtex-engine      --target x86_64-pc-windows-gnu --all-targets   # clean
cargo build --release -p flashtex-engine --bin flashtex-initex --bin flashtex-host \
    --target x86_64-pc-windows-gnu                                                  # links
```

Both are PE32+ x86-64 console programs of ~10.5 MB that import only system
DLLs (ADVAPI32, KERNEL32, ntdll, SHELL32, USERENV, WS2_32,
bcryptprimitives, the UCRT `api-ms-win-crt-*`); no `libstdc++-6.dll`. On
`origin/main` the same check failed in display-list-v3 (`std::os::unix`),
then kpathsea (`pwd.h`), `flashtex_regex.c` (`regex.h`), 23 Rust errors in
the host, and the link (`shell32`, `advapi32`). No Windows program could be
run on this Mac (Homebrew's `wine-stable` cask is disabled).

### On windows-latest [CI]

`.github/workflows/portability.yml` (§4). Run 37110451880 at 8696178e4,
rustc 1.99.0 stable, MSYS2 UCRT64 GCC 16.2.0:

| Step | Result |
|---|---|
| build `flashtex-initex.exe`, `flashtex-host.exe` (x86_64-pc-windows-gnu) | ok; system DLLs only |
| `flashtex-initex -version` | ok |
| display-list-v3 tests (x86_64-pc-windows-msvc; AF_UNIX client/listener round trips) | 17 passed (the socket-pair test is Unix-only) |
| engine unit tests (x86_64-pc-windows-gnu) | 72 passed, 1 ignored (as on macOS) |
| trip (scratch package, x86_64-pc-windows-msvc) | 5 PASS, `trip.typ` SKIP (no dvitype) |
| etrip (scratch package, x86_64-pc-windows-msvc) | 18/18 PASS |
| pdfTeX regression tests (the MinGW engine, kpathsea-self) | 6 PASS, `wcfname.test` SKIP (no kpsewhich/perl) |

The five runs before it found rows 17, 18, 19 and 23 and the link order of
row 22; each fix is its own commit.

### Still different or untested on Windows

*Superseded in part by #1418 (2026-10-04): the current state is
[`docs/dev/engine-windows.md`](../../dev/engine-windows.md).*

- `\pdfmatch`: no regex engine (row 21). Fix: vendor `pdftexdir/regex` with
  version, licence and sha256 (DESIGN.md §3), about 1 d.
- No inode in stat stamps (std's file index is unstable): size, mtime and
  creation time remain. `out_key` (row 18) keeps reopened-file detection.
- The word space's commit is charged at once (`VirtualAlloc` commit), not
  lazily as with `mmap`; pages are still touched lazily.
- Not run on Windows: a LaTeX document against TeX Live for Windows (P-T1),
  `flashtex-host` (socket, export channel, `FLASHTEX_INVOKED_AS`), `\write18`
  through `cmd.exe`, the format cache. Next gate: install TeX Live on the
  runner (or the bundle) and run `scripts/engine-parity.sh lockstep parity`.
- `x86_64-pc-windows-msvc` for the whole engine (C included) is unbuilt;
  the MSVC kpathsea list in build.rs is untested [belief]. Only the
  pure-Rust trip/etrip packages were built with MSVC.
- MiKTeX is not a target; TeX Live for Windows is.

## 3. WASI spike (wasm32-wasip1) [measured]

About a quarter of the lane. `wasm.sh` and `wasmrun.sh` here are the exact
commands; `wasi-shim/pwd.h` is the only shim. Homebrew llvm 23.1.2,
wasi-libc 34, wasi-runtimes 23.1.2, wasmtime 49.0.2.

1. Default features: the build stops in kpathsea's C (`tex-make.c` needs
   `<sys/wait.h>` to run mktex scripts, `tilde.c` needs `<pwd.h>`).
2. `--no-default-features` (working-directory resolver, no format cache, no
   regex): libpng's setjmp needs wasm exception handling (`-mllvm
   -wasm-enable-sjlj -mllvm -wasm-use-legacy-eh=false`, `-lsetjmp`); xpdf's
   `gfile.cc` needs `<pwd.h>` (shim); C++ from the sysroot's
   `libc++`/`libc++abi`. The Rust side compiles unchanged (`os.rs`'s WASI
   branch).
3. `flashtex-initex.wasm`: 3,763,534 bytes. Under
   `wasmtime run -W exceptions=y --dir .`, INITEX of `plain.tex` dumps
   `plain.fmt`, and a one-page document with Type 1 fonts writes a PDF.
   **All six outputs are byte-identical to the native macOS engine built with
   the same features**: `plain.fmt` (493,304 B), `plain.log`, `hello.log`,
   `hello.pdf` (39,527 B, sha256 `274eb9820abf…`), both terminals. Against
   the default-feature build the only difference is the banner's
   `kpathsea version 6.4.2`.

**Verdict:** the translated engine is portable to WASI as it is; the C
around it needs small, known work. Estimate:

| Work | Estimate |
|---|---|
| kpathsea on WASI (no `fork`/`wait`: mktex off; no passwd: `~` unexpanded) or the bundle resolver over a preopened directory | 2–3 d |
| commit the shims and flags as a wasm32-wasip1 branch of build.rs | 1 d |
| `\pdfmatch` (wasi-libc regex), `\write18` off, check that the batch path never reaches the parallel restore's threads | 1 d |
| CI: build and the plain/PDF comparison under wasmtime | 0.5 d |
| **Batch CLI on wasmtime, with TeX Live or the bundle** | **≈ 1–1.5 engineer-weeks** |
| Browser: virtual FS over the content-addressed bundle, fetch, JS host, wasm exception handling | +3–6 weeks |
| Resident host in WASM (threads or a serial restore; a message channel instead of sockets) | not estimated |

## 4. CI (proposed; Commander-owned)

`.github/workflows/portability.yml`, in its own commits on PR #1406, outside
`ci-required`, so it cannot hold the merge queue until the Commander adopts
it: `windows-latest`, MSYS2 UCRT64 MinGW-w64, the steps of the table in §2.
It runs on pull requests that touch the engine, display-list-v3,
third_party, web2rust or the three test scripts, and on demand. 7 minutes
for run 37110451880 (warm cache). Next additions, in order: TeX Live for
Windows + `engine-parity.sh lockstep parity`; a `wasm32-wasip1` job (§3).

## 5. Gates

macOS, this branch:

- trip 6/6 PASS, etrip 18/18 PASS at 498262c47 [measured]
- pdfTeX regression tests 7/7 PASS at 8696178e4 [measured]
- engine unit tests 72 passed, 1 ignored at 498262c47 [measured]
- display-list-v3 tests 19 passed [measured]
- `scripts/gate.sh pr` passed at c28e2651a (rustfmt, clippy, tests of both
  crates, licence boundary and its self-test, parity scoreboard self-tests,
  parity fixtures at their baseline, inventory; 7,484 s on a loaded machine)
  [measured]
- licence boundary: clean [measured]

Linux (the repository's CI on PR #1406): engine parity (GitHub-hosted:
lockstep, P-T1/P-T2 fixtures, engine tests with TeX Live), trip, etrip,
pdfTeX regression tests, licence boundary: green at 21c172e31 [CI]; later
pushes are rechecked by the same jobs.
