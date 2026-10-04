#!/usr/bin/env bash
# sanitizers.sh LEG...: the engine under AddressSanitizer/LeakSanitizer and Miri (lane
# MEMORY-SAFETY, 2026-10-04; nightly.yml `engine-sanitizers`). Needs a nightly toolchain with
# rust-src (and miri for `miri`). Legs:
#
#   trip    Knuth's trip test, the engine built with ASan (leak detection on): any report fails
#   etrip   the e-TeX/pdfTeX etrip test, likewise
#   miri    Miri (strict provenance) over the arena's tests: the word space, its undo logs,
#           the slab and the parallel restores (src/arena.rs, most of the engine's `unsafe`),
#           and the host's crash note, written and read across threads (src/host/crash.rs)
#   host    flashtex-host and flashtex-initex built with ASan, the vendored C (kpathsea, zlib,
#           libpng, xpdf) included; builds the LaTeX format with LSan, then types through the
#           socket (tools/incr-bench/mem.py, `--once` so the host exits and LSan runs) for SHORT
#           and LONG keystrokes: no ASan error, no leak outside kpathsea's start-up, and no more
#           leaked bytes after LONG keystrokes than after SHORT (scripts/sanitizer_reports.py).
#           Needs TeX Live 2026.
#   leaks   macOS only: the release host through three socket sessions, Apple's `leaks` after
#           each (scripts/leaks_session.py): at most 2 MB lost by start-up and at most 1 KB a
#           keystroke after it, on plain-10 and full-120. LSan missed the three host leaks fixed
#           on 2026-10-04 (zlib per restore, kpathsea per lookup, 18 MB at start); this did not.
#           Needs TeX Live 2026; uses $CARGO_TARGET_DIR's release build (it builds it).
#   sweep   the fixture-wide incremental soundness test (host_incremental.rs
#           `every_fixture_edits_equal_scratch_compiles`) with ASan host and test (no leak
#           check: its hosts are stopped by signal). Needs TeX Live 2026; about an hour.
#
# SAN_WORK (default a temp dir) holds the builds and reports; KEYS_SHORT/KEYS_LONG (10/60) and
# SAN_DOC (plain-10) tune `host`.
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=${SAN_WORK:-$(mktemp -d)}
mkdir -p "$work"
triple=$(rustc -vV | sed -n 's/^host: //p')
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-3}
# Apple's clang marks ASan objects for its own runtime's version; Rust links LLVM's.
asan_c="-fsanitize=address -fno-omit-frame-pointer"
case "$triple" in *apple*) asan_c="$asan_c -mllvm -asan-guard-against-version-mismatch=0" ;; esac

asan_build() { # the host binaries with ASan, into $work/asan
    CARGO_TARGET_DIR=$work/asan RUSTFLAGS="-Zsanitizer=address" CFLAGS="$asan_c" CXXFLAGS="$asan_c" \
        cargo build --release --locked -Zbuild-std --target "$triple" \
        -p flashtex-engine -p flashtex-display-list --bins
}

fail=0
leg_trip() { FLASHTEX_SANITIZER=address TRIP_WORK=$work/trip TRIP_TIMEOUT=1200 "$root/scripts/flashtex-trip.sh"; }
leg_etrip() { FLASHTEX_SANITIZER=address ETRIP_WORK=$work/etrip "$root/scripts/flashtex-etrip.sh"; }
leg_miri() {
    CARGO_TARGET_DIR=$work/miri MIRIFLAGS="-Zmiri-strict-provenance" \
        cargo miri test --locked -p flashtex-engine --no-default-features --lib -- arena:: host::crash::
}
leg_host() {
    asan_build || return 1
    # mkeng.sh takes the binaries from $CARGO_TARGET_DIR/release.
    mkdir -p "$work/asan-bins"
    ln -sfn "$work/asan/$triple/release" "$work/asan-bins/release"
    export INCR_BENCH_DIR=$work/ib FLASHTEX_FORMAT_CACHE_DIR=$work/format-cache
    local rep=$work/reports r=0
    rm -rf "$rep"
    mkdir -p "$rep/format" "$rep/short" "$rep/long"
    ASAN_OPTIONS=detect_leaks=1:log_path=$rep/format/asan CARGO_TARGET_DIR=$work/asan-bins \
        bash "$root/tools/incr-bench/mkeng.sh" asan || return 1
    echo "== the LaTeX format, built by the ASan engine"
    python3 "$root/scripts/sanitizer_reports.py" "$rep/format" || r=1
    python3 "$root/tools/incr-bench/mkdocs.py" >/dev/null
    local doc=${SAN_DOC:-plain-10}
    for s in short long; do
        local k=${KEYS_SHORT:-10}
        [ $s = long ] && k=${KEYS_LONG:-60}
        ASAN_OPTIONS=detect_leaks=1:log_path=$rep/$s/asan python3 "$root/tools/incr-bench/mem.py" asan "$doc" \
            --pages 3 --keys "$k" --gap-ms 150 --host-args=--once --tag "san-$s" --timeout 3000 \
            --limit-gb 8 | tail -1 | cut -c1-200
    done
    echo "== $doc, ${KEYS_SHORT:-10} keystrokes"
    python3 "$root/scripts/sanitizer_reports.py" "$rep/short" || r=1
    echo "== $doc, ${KEYS_LONG:-60} keystrokes"
    python3 "$root/scripts/sanitizer_reports.py" --growth-against "$rep/short" "$rep/long" || r=1
    # A host that exits without a report did not run LSan (killed, or never started).
    ls "$rep/long"/asan.* >/dev/null 2>&1 || { echo "FAIL no LeakSanitizer report from the host"; r=1; }
    return $r
}
leg_leaks() {
    [ "$(uname -s)" = Darwin ] || { echo "SKIP leaks: macOS only"; return 0; }
    cargo build --release --locked -p flashtex-engine -p flashtex-display-list --bins || return 1
    INCR_BENCH_DIR=$work/ib FLASHTEX_FORMAT_CACHE_DIR=$work/format-cache \
        bash "$root/tools/incr-bench/mkeng.sh" rel >/dev/null || return 1
    INCR_BENCH_DIR=$work/ib python3 "$root/tools/incr-bench/mkdocs.py" >/dev/null
    local r=0
    for doc in plain-10 full-120; do
        echo "== leaks: $doc"
        FLASHTEX_FORMAT_CACHE_DIR=$work/format-cache \
            python3 "$root/scripts/leaks_session.py" "$work/ib" rel "$doc" || r=1
    done
    return $r
}
leg_sweep() {
    mkdir -p "$work/reports/sweep" "$work/tmp"
    CARGO_TARGET_DIR=$work/asan RUSTFLAGS="-Zsanitizer=address" CFLAGS="$asan_c" CXXFLAGS="$asan_c" \
        ASAN_OPTIONS=detect_leaks=0:log_path=$work/reports/sweep/asan \
        FLASHTEX_FORMAT_CACHE_DIR=$work/format-cache TMPDIR=$work/tmp \
        cargo test --release --locked -Zbuild-std --target "$triple" -p flashtex-engine \
        --test host_incremental -- --ignored every_fixture_edits_equal_scratch_compiles || return 1
    python3 "$root/scripts/sanitizer_reports.py" "$work/reports/sweep"
}

[ $# -gt 0 ] || set -- trip etrip miri host
for leg in "$@"; do
    echo "=== sanitizers: $leg"
    if "leg_$leg"; then echo "=== sanitizers: $leg PASS"; else echo "=== sanitizers: $leg FAIL"; fail=1; fi
done
echo "work dir: $work"
exit $fail
