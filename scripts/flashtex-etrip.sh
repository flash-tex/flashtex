#!/bin/sh
# e-TeX's trip test (etrip) for the pdftex.web engine of crates/flashtex-engine.
#
#   scripts/flashtex-etrip.sh            # exit status 0 iff etrip passes
#   scripts/flashtex-etrip.sh --oracle   # regenerate third_party/pdftex/etrip-oracle/
#                                        # with TeX Live's own pdftex (oracle only)
#
# The test is etripman.tex's (appendices A and B), run as TeX Live's
# etexdir/etriptest.test runs it, in three parts:
#
#   1. Knuth's trip test in compatibility mode (ctripin, ctrip);
#   2. the trip test in extended mode (xtripin, xtrip: `*\input trip`, then
#      `&trip \toksdef\tokens=0 \input trip`);
#   3. the e-TeX part (etripin, etrip: `*etrip`, then `&etrip etrip`).
#
# Most capacities are compile-time constants of the translated engine, so the
# engine is generated with crates/flashtex-engine/web2rust-etrip.args (TeX
# Live's etrip/texmf.cnf) into a scratch package under $ETRIP_WORK (default: a
# temp dir) -- never into the committed src/generated/ -- and built there.
# error_line, half_error_line and max_print_line are read at run time, as in
# web2c, and set below.
# No TeX installation is needed: the fixtures, including etrip.tfm, and the
# expected output are committed under third_party/pdftex/.
#
# Expected output. etrip's masters are e-TeX's; pdfTeX differs from them for
# reasons of its own, so what this engine must reproduce is pdfTeX: the logs,
# terminal transcripts, \write files and DVI files that TeX Live 2026's pdftex
# 1.40.29 produces in the same configuration (third_party/pdftex/etrip-oracle/,
# made by --oracle). Both sides run with SOURCE_DATE_EPOCH=0 and
# FORCE_SOURCE_DATE=1, so dates agree too.
#
# Gate: every file byte-identical to pdfTeX's, after exactly these
# normalisations (the `norm` sed script below), each applied to both sides:
#   a. web2c's version string " (TeX Live 2026)" in the banner;
#   b. accounting lines (DESIGN.md section 1.1): "Memory usage before ...",
#      "N memory locations dumped; current usage is A&B", " N words of memory
#      out of M" (TeX Live's pdfTeX has SyncTeX's two extra words in several
#      kinds of node), the string-pool counts ("N strings of total length M",
#      " N strings out of M", " N string characters out of M"; web2c's pool
#      has other strings), and " N hyphenation exception(s) out of M"
#      (tex.ch keeps \hyphenation exceptions in a chained hash that counts a
#      repeated word once; in both, the latest positions of a word win).
#
# Informational (not gating): the same files against e-TeX's masters under
# TeX Live's accepted-difference filter (etriptest.test).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
fix=$root/third_party/pdftex/etrip
knuth=$root/third_party/knuth/trip
oracle=$root/third_party/pdftex/etrip-oracle

# The files each run leaves behind, in the order etriptest.test makes them.
files="ctripin.log ctripin.fot ctrip.log ctrip.fot ctripos.tex ctrip.dvi
xtripin.log xtripin.fot xtrip.log xtrip.fot xtripos.tex xtrip.dvi
etripin.log etripin.fot etrip.log etrip.fot etrip.out etrip.dvi"

# etrip_runs INI VIR DIR: the six runs of the three parts in DIR. INI and VIR
# are the commands for an INITEX and for a production run.
etrip_runs() {
    ini=$1 vir=$2 d=$3
    cp "$knuth/trip.tex" "$knuth/trip.tfm" "$fix/etrip.tex" "$fix/etrip.tfm" "$d/"
    (
        cd "$d"
        # Part 1: tripman.tex steps 3 and 4, compatibility mode.
        printf '\n\\input trip\n' | $ini >ctripin.fot 2>&1 || true
        mv trip.log ctripin.log
        printf ' &trip  trip \n' | $vir >ctrip.fot 2>&1 || true
        mv trip.log ctrip.log
        mv tripos.tex ctripos.tex
        mv trip.dvi ctrip.dvi
        # Part 2: extended mode.
        $ini <"$fix/etrip1.in" >xtripin.fot 2>&1 || true
        mv trip.log xtripin.log
        $vir <"$fix/trip2.in" >xtrip.fot 2>&1 || true
        mv trip.log xtrip.log
        mv tripos.tex xtripos.tex
        mv trip.dvi xtrip.dvi
        # Part 3: the e-TeX specific test.
        $ini <"$fix/etrip2.in" >etripin.fot 2>&1 || true
        mv etrip.log etripin.log
        $vir <"$fix/etrip3.in" >etrip.fot 2>&1 || true
    )
}

export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1

if [ "${1:-}" = "--oracle" ]; then
    command -v pdftex >/dev/null 2>&1 || {
        echo "flashtex-etrip.sh --oracle: no pdftex on PATH" >&2
        exit 1
    }
    work=$(mktemp -d)
    # TeX Live's etrip configuration; pdfTeX's strings need a larger pool
    # than e-TeX's 32000 (etrip/texmf.cnf), which only changes the counts
    # normalised under b.
    TEXMFCNF=$fix pool_size=60000 max_strings=5000
    export TEXMFCNF pool_size max_strings
    etrip_runs "pdftex --progname=inipdftex --ini" "pdftex --progname=pdftex" "$work"
    rm -rf "$oracle"
    mkdir -p "$oracle"
    for f in $files; do cp "$work/$f" "$oracle/$f"; done
    pdftex --version | head -1 >"$oracle/VERSION"
    (cd "$root/third_party/pdftex" &&
        shasum -a 256 pdftex.web etrip/* etrip-oracle/*.* etrip-oracle/VERSION \
            web2c/*.ch >SHA256SUMS)
    echo "wrote $oracle ($(head -1 "$oracle/VERSION"))"
    rm -rf "$work"
    exit 0
fi

work=${ETRIP_WORK:-$(mktemp -d)}
mkdir -p "$work"
pkg=$work/pkg
run=$work/run
rm -rf "$pkg" "$run"
mkdir -p "$pkg/src/generated" "$run"

# 1. Generate the etrip configuration into the scratch package.
cargo build --release --locked -p web2rust
(cd "$root" && "$root/target/release/web2rust" third_party/pdftex/pdftex.web \
    @crates/flashtex-engine/web2rust-etrip.args \
    --out-dir "$pkg/src/generated" --pool "$run/pdftex.pool")
cp "$root"/crates/flashtex-engine/src/*.rs "$pkg/src/"
cp -R "$root/crates/flashtex-engine/src/pdftex" "$pkg/src/"
cat >"$pkg/Cargo.toml" <<'EOF'
[package]
name = "flashtex-engine-etrip"
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
kpathsea = []
tex82 = []

# Standalone: not a member of the repository's workspace.
[workspace]
EOF
# The generated code's warnings are known and not ours to fix by hand.
CARGO_TARGET_DIR=$work/target RUSTFLAGS=-Awarnings \
    cargo build --release --quiet --manifest-path "$pkg/Cargo.toml"
initex=$work/target/release/flashtex-initex

# 2. The runs. `cwd-kpse`: files come from the working directory as kpathsea
# finds them for etrip/texmf.cnf's search path `.`, i.e. as `./etrip.tex`.
export FLASHTEX_POOL="$run/pdftex.pool" FLASHTEX_RESOLVER=cwd-kpse
# etrip/texmf.cnf's three run-time values (changes/web2c-run.ch). That
# resolver reads no texmf.cnf, so they come from the environment, where
# kpathsea itself looks first.
export error_line=64 half_error_line=32 max_print_line=72
# The second run of each part is a production run, as in TeX Live: its
# default format is `pdftex' (never loaded, since the first line names one).
etrip_runs "$initex -ini" "$initex -fmt=pdftex" "$run"

# 3. Compare.
norm=$work/norm.sed
cat >"$norm" <<'EOF'
s/ (TeX Live 20[0-9][0-9])//
s/^Memory usage before: [0-9&]*; after: [0-9&]*; still untouched: [0-9]*$/Memory usage before: A\&B; after: C\&D; still untouched: E/
s/^[0-9]* memory locations dumped; current usage is [0-9]*&[0-9]*$/N memory locations dumped; current usage is A\&B/
s/^ [0-9]* words of memory out of [0-9]*$/ N words of memory out of M/
s/^[0-9]* strings of total length [0-9]*$/N strings of total length M/
s/^ [0-9]* strings out of [0-9]*$/ N strings out of M/
s/^ [0-9]* string characters out of [0-9]*$/ N string characters out of M/
s/^ [0-9]* hyphenation exceptions* out of [0-9]*$/ N hyphenation exceptions out of M/
EOF
fail=0
for f in $files; do
    case $f in
    *.dvi)
        if cmp -s "$oracle/$f" "$run/$f"; then
            echo "PASS $f: byte-identical to pdfTeX's ($(wc -c <"$oracle/$f" | tr -d ' ') bytes)"
        else
            echo "FAIL $f: differs from pdfTeX's"
            fail=1
        fi
        ;;
    *)
        if cmp -s "$oracle/$f" "$run/$f"; then
            echo "PASS $f: byte-identical to pdfTeX's ($(wc -l <"$oracle/$f" | tr -d ' ') lines)"
        elif sed -f "$norm" "$oracle/$f" >"$work/a" && sed -f "$norm" "$run/$f" >"$work/b" &&
            cmp -s "$work/a" "$work/b"; then
            n=$(diff "$oracle/$f" "$run/$f" | grep -c '^<' || true)
            echo "PASS $f: identical to pdfTeX's after the normalisations ($n line(s) normalised)"
        else
            echo "FAIL $f"
            diff "$work/a" "$work/b" | head -20 || true
            fail=1
        fi
        ;;
    esac
done

# 4. Informational: against e-TeX's masters under TeX Live's filter.
tlf=$work/tl.sed
cat >"$tlf" <<'EOF'
/^\*\* \&trip  trip/d
/^\*\*entering extended mode/d
s,^(trip\.tex ##,**(./trip.tex ##,
s,^## (\./trip\.tex,**(./trip.tex ##,
s/ (TeX Live 20[^)]*)//
s/ (Web2C 202[3-9][^)]*)//
s/(preloaded format=.*tex)/(INITEX)/
s/format=trip [^)][^)]*)/format=trip)/
s/)  [0-9A-Z: ]*$/)/
s,^(\./,(,
s/[1-9][0-9]* strings out of [1-9].*/XX strings out of YYY/
s/[1-9][0-9]* string characters out of [1-9].*/XXX string characters out of YYYY/
s/sequences out of [1-9].*/sequences out of YYYY/
s/[1-9] hyphenation exceptions* out of [1-9].*/X hyphenation exceptions out of YYY/
s/[1-9][0-9]* strings of total length [1-9].*/XXXX strings of total length YYYYY/
s/9 ops out of [1-9][0-9]*/9 ops out of YYY/
s/TeX output ....\...\...:..../TeX output YYYY.MM.DD:hhmm/
s/ 16341\.999.*fil/ 16342.0fil/
s/ 16238\.999.*fil/ 16239.0fil/
s/ 16317\.999.*fil/ 16318.0fil/
s/ 16330\.999.*fil/ 16331.0fil/
s/ 16331\.999.*fil/ 16332.0fil/
s/ 16343\.999.*fil/ 16344.0fil/
s/ 9737\.587..fil/ 9737.58789fil/
s/down4 639342.../down4 639342208/
s/y4 2039217../y4 203921760/
s/y0 2039217../y0 203921760/
s/This is .*TeX,/This is *TeX,/
s/ Version 3\.141592653[^(]*(/ Version 3.141592653* (/
s/ before: [1-9][0-9][0-9][0-9]*&[1-9][0-9][0-9][0-9]*; / before: XXX\&YYY; /
s/ after: [1-9][0-9][0-9][0-9]*&[1-9][0-9][0-9][0-9]*; / after: XXX\&YYY; /
s/ still untouched: [1-9][0-9][0-9][0-9]*/ still untouched: XXX/
EOF
for pair in "$knuth/tripin.log ctripin.log" "$knuth/trip.log ctrip.log" \
    "$knuth/tripos.tex ctripos.tex" "$fix/etripin.log etripin.log" \
    "$fix/etrip.log etrip.log" "$fix/etrip.out etrip.out" \
    "$knuth/trip.fot ctrip.fot" "$fix/etrip.fot etrip.fot"; do
    set -- $pair
    sed -f "$tlf" "$1" >"$work/a"
    sed -f "$tlf" "$run/$2" >"$work/b"
    n=$(diff "$work/a" "$work/b" | grep -c '^[<>]' || true)
    echo "info $2 vs e-TeX's master $(basename "$1") under TeX Live's filter: $n line(s) differ"
done

echo "work dir: $work"
exit $fail
