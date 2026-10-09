#!/bin/sh
# Knuth's trip test (tripman.tex, "How to test TeX") for crates/flashtex-engine.
#
#   scripts/flashtex-trip.sh            # exit status 0 iff trip passes
#
# The trip test needs its own capacities (tripman.tex step 2), which are in
# crates/flashtex-engine/web2rust-trip.args. The engine is therefore generated
# with them into a scratch package under $TRIP_WORK (default: a temp dir) --
# never into the committed src/generated/ -- and built there without the
# `kpathsea` feature, so files are found in the working directory as
# tripman.tex assumes. No TeX installation is needed: the fixtures, including
# trip.tfm, are committed under third_party/knuth/trip/.
#
# Gates (tripman.tex steps 3-6):
#   tripin.log, trip.log, tripos.tex   byte-identical to Knuth's masters
#   tripin.fot, trip.fot               identical after exactly the two accepted
#                                      differences (tools/web2rust/tools/trip_fot.py)
#   trip.typ                           only if `dvitype` is on PATH: identical
#                                      except DVItype's own banner line
#
# Fails closed (#1208): step 3 must leave a non-empty trip.fmt, each engine run
# is killed with its process group past TRIP_TIMEOUT seconds (default 300) and
# then fails the test, the .fot comparison is byte-exact (a CR is a
# difference), and with TRIP_REQUIRE_DVITYPE=1 a missing dvitype or trip.dvi
# is a FAIL rather than a SKIP (CI's Linux leg sets it).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
fix=$root/third_party/knuth/trip
work=${TRIP_WORK:-$(mktemp -d)}
mkdir -p "$work"
pkg=$work/pkg
run=$work/run
rm -rf "$pkg" "$run"
mkdir -p "$pkg/src/generated" "$run"

# 1. Generate the trip configuration into the scratch package.
cargo build --release --locked -p web2rust
"${CARGO_TARGET_DIR:-$root/target}/release/web2rust" "$root/third_party/knuth/tex.web" \
    @"$root/crates/flashtex-engine/web2rust-trip.args" \
    --out-dir "$pkg/src/generated" --pool "$run/tex.pool"
for f in lib.rs main.rs system.rs resolver.rs arena.rs cli.rs; do
    cp "$root/crates/flashtex-engine/src/$f" "$pkg/src/"
done
cat >"$pkg/Cargo.toml" <<'EOF'
[package]
name = "flashtex-engine-trip"
version = "0.0.0"
edition = "2021"
license = "GPL-2.0-or-later"
publish = false

[lib]
name = "flashtex_engine"
path = "src/lib.rs"

[[bin]]
name = "flashtex-initex"
path = "src/main.rs"

[features]
# tex82: the crate's lib.rs leaves out src/pdftex/ (pdfTeX's C parts).
default = ["tex82"]
kpathsea = []
tex82 = []

# Standalone: not a member of the repository's workspace.
[workspace]
EOF
# The runtime shared with the XeTeX-derived engine (crates/flashtex-runtime:
# persist, os, busy, memstat), as the engine crate links it, in its tex82 form.
# (Cargo reads the path natively: under Git Bash/MSYS2 on Windows, `/d/a/...`
# is `D:/a/...`, which `cygpath -m` gives; elsewhere there is no cygpath.)
printf '\n[dependencies]\nflashtex-runtime = { path = "%s/crates/flashtex-runtime", features = ["tex82"] }\n' \
    "$(cygpath -m "$root" 2>/dev/null || printf '%s' "$root")" >>"$pkg/Cargo.toml"
# The generated code's warnings are known and not ours to fix by hand.
CARGO_TARGET_DIR=$work/target RUSTFLAGS=-Awarnings \
    cargo build --release --quiet --manifest-path "$pkg/Cargo.toml"
initex=$work/target/release/flashtex-initex

# 2. Run tripman.tex steps 3 and 4. TeX ends both runs with errors on
# purpose, so their exit status is not the verdict; a timeout is (it is
# logged in $timeouts and fails the test below).
timeout_s=${TRIP_TIMEOUT:-300}
timeouts=$work/timeouts.txt
: >"$timeouts"
timed() { python3 "$root/scripts/run-with-timeout.py" "$timeout_s" --log "$timeouts" -- "$@"; }
cp "$fix/trip.tex" "$fix/trip.tfm" "$run/"
export FLASHTEX_POOL="$run/tex.pool" FLASHTEX_RESOLVER=cwd
# Step 3: an empty line at the first `**`, then `\input trip`.
printf '\n\\input trip\n' | (cd "$run" && timed "$initex" >tripin.fot 2>&1) || true
mv "$run/trip.log" "$run/tripin.log" || true
# Step 4: ` &trip  trip ` -- the spaces are part of the test.
printf ' &trip  trip \n' | (cd "$run" && timed "$initex" >trip.fot 2>&1) || true

# 3. Compare.
fail=0
if [ -s "$timeouts" ]; then
    echo "FAIL engine runs cut off:"
    sed 's/^/    /' "$timeouts"
    fail=1
fi
# Step 4 loads trip.fmt; an empty or missing one must not pass by accident.
if [ -s "$run/trip.fmt" ]; then
    echo "PASS trip.fmt: written by step 3 ($(wc -c <"$run/trip.fmt" | tr -d ' ') bytes)"
else
    echo "FAIL trip.fmt: step 3 left no format (or an empty one)"
    fail=1
fi
for f in tripin.log trip.log tripos.tex; do
    if cmp -s "$fix/$f" "$run/$f"; then
        echo "PASS $f: byte-identical ($(wc -l <"$fix/$f" | tr -d ' ') lines)"
    else
        echo "FAIL $f"
        diff "$fix/$f" "$run/$f" | head -20 || true
        fail=1
    fi
done
python3 "$root/tools/web2rust/tools/trip_fot.py" "$fix/tripin.fot" "$run/tripin.fot" '' '\input trip' || fail=1
python3 "$root/tools/web2rust/tools/trip_fot.py" "$fix/trip.fot" "$run/trip.fot" ' &trip  trip ' || fail=1
if command -v dvitype >/dev/null 2>&1 && [ ! -s "$run/trip.dvi" ]; then
    echo "FAIL trip.typ: step 4 wrote no trip.dvi"
    fail=1
elif command -v dvitype >/dev/null 2>&1; then
    # DVItype finds trip.tfm through kpathsea. A texmf.cnf of our own, with
    # just the TFM path, keeps it independent of the system's TeX setup: on
    # a runner with texlive-binaries but no texlive-base there is no
    # texmf.cnf at all, kpathsea warns into trip.typ and no TFM is found.
    mkdir -p "$work/dvitype-cnf"
    printf 'TFMFONTS = .\n' >"$work/dvitype-cnf/texmf.cnf"
    (cd "$run" && TEXMFCNF=$work/dvitype-cnf dvitype -output-level=2 -page-start='*.*.*.*.*.*.*.*.*.*' \
        -max-pages=1000000 -dpi=72.27 -magnification=0 trip.dvi >trip.typ 2>&1) || true
    if [ "$(tail -n +2 "$fix/trip.typ" | cksum)" = "$(tail -n +2 "$run/trip.typ" | cksum)" ]; then
        echo "PASS trip.typ: identical after DVItype's banner line"
    else
        echo "FAIL trip.typ; the first differences after the banner line:"
        tail -n +2 "$fix/trip.typ" >"$work/typ.master"
        tail -n +2 "$run/trip.typ" >"$work/typ.ours"
        diff "$work/typ.master" "$work/typ.ours" | head -20 || true
        fail=1
    fi
elif [ "${TRIP_REQUIRE_DVITYPE:-0}" = 1 ]; then
    echo "FAIL trip.typ: no dvitype on PATH, and TRIP_REQUIRE_DVITYPE=1"
    fail=1
else
    echo "SKIP trip.typ: no dvitype on PATH"
fi
echo "work dir: $work"
exit $fail
