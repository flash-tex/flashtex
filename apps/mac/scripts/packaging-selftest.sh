#!/usr/bin/env bash
# Regression test for the packaging scripts.
#
# Usage: apps/mac/scripts/packaging-selftest.sh [--full] [--evidence <file>] [-- <make-app.sh args...>]
#
# Fast part (no build, seconds): every signing/notarization failure path of
# make-app.sh exits 1 before the build with a clear, non-secret message; the
# Resources plists lint; --help works for all scripts. Never needs a Developer
# ID: it asserts the ABSENCE paths (and skips a check when this machine does
# have the identity/profile it probes for, reporting that).
# --full additionally runs repro-check.sh (ad-hoc and `--sign -` hardened
# runtime), then launch-check.sh on the resulting app and on a --dmg image.
# make-app.sh arguments after -- (e.g. --helper-root <main checkout>) are
# passed through to every build in --full mode.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MAKE="$SCRIPT_DIR/make-app.sh"

FULL=0
EVIDENCE_FILE=""
MAKE_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --full) FULL=1; shift ;;
    --evidence) EVIDENCE_FILE="$2"; shift 2 ;;
    --) shift; MAKE_ARGS=("$@"); break ;;
    -h|--help) sed -n '2,15p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "packaging-selftest.sh: unknown argument: $1" >&2; exit 1 ;;
  esac
done

PASS=0; FAIL=0
LINES=()
ok()   { PASS=$((PASS + 1)); echo "  ok   $1"; LINES+=("- ok: $1"); }
bad()  { FAIL=$((FAIL + 1)); echo "  FAIL $1" >&2; LINES+=("- FAIL: $1"); }
skip() { echo "  skip $1"; LINES+=("- skip: $1"); }
section() { echo "==> $1"; LINES+=("" "## $1" ""); }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-selftest.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# Runs make-app.sh with the given args, expecting exit 1 BEFORE any build
# output (asserted by the absence of the "Building FlashTeXMac" line).
expect_preflight_failure() {
  local expect="$1"; shift
  local out="$WORK/out.txt" rc=0
  "$MAKE" "$@" >"$out" 2>&1 || rc=$?
  if [[ "$rc" -ne 1 ]]; then bad "make-app.sh $* exited $rc, expected 1"; return; fi
  if grep -q "Building FlashTeXMac" "$out"; then bad "make-app.sh $* started a build before failing"; return; fi
  if ! grep -qF -e "$expect" "$out"; then bad "make-app.sh $* did not print \"$expect\"; got: $(head -1 "$out")"; return; fi
  ok "make-app.sh $* -> exit 1, pre-build, message: $(head -c 120 "$out" | tr '\n' ' ')…"
}

section "Pre-flight failure paths (no build)"
BOGUS_ID="Developer ID Application: FlashTeX Selftest (SELFTEST00)"
IDENTITIES="$(security find-identity -v -p codesigning 2>/dev/null || true)"
if grep -qF -e "$BOGUS_ID" <<< "$IDENTITIES"; then
  skip "an identity literally named \"$BOGUS_ID\" exists here; absent-identity check skipped"
else
  expect_preflight_failure "no valid codesigning identity matching" --sign "$BOGUS_ID"
fi
expect_preflight_failure "--notarize requires --sign" --notarize flashtex-selftest-profile
expect_preflight_failure "cannot be notarized" --sign - --notarize flashtex-selftest-profile
expect_preflight_failure "entitlements file not found" --sign - --entitlements "$WORK/missing.entitlements"
printf 'not a plist' > "$WORK/bad.entitlements"
expect_preflight_failure "not a valid plist" --sign - --entitlements "$WORK/bad.entitlements"
if security find-generic-password -s com.apple.gke.notary.tool -a flashtex-selftest-profile >/dev/null 2>&1; then
  skip "a notarytool profile named flashtex-selftest-profile exists here; absent-profile check skipped"
elif grep -q 'Developer ID Application' <<< "$IDENTITIES"; then
  REAL_ID="$(grep -m1 -o '"Developer ID Application[^"]*"' <<< "$IDENTITIES" | tr -d '"')"
  expect_preflight_failure "no notarytool keychain profile named" --sign "$REAL_ID" --notarize flashtex-selftest-profile
else
  # Without any Developer ID the identity check fires first; probe the profile
  # lookup make-app.sh uses so the absent-profile path is still exercised.
  ok "no Developer ID on this machine: identity check fires before the profile check; direct probe 'security find-generic-password -s com.apple.gke.notary.tool -a flashtex-selftest-profile' exits $(security find-generic-password -s com.apple.gke.notary.tool -a flashtex-selftest-profile >/dev/null 2>&1; echo $?) (non-zero = absent)"
fi
rc=0; "$MAKE" --bogus-flag >"$WORK/out.txt" 2>&1 || rc=$?
[[ "$rc" -eq 1 ]] && grep -q "unknown argument" "$WORK/out.txt" && ok "unknown argument -> exit 1" || bad "unknown argument handling"

section "Pinned rooted TFM metrics (GH36; no build, no download, no host TeX)"
TEXMF_TOOL="$SCRIPT_DIR/bundle-texmf.py"
if python3 "$TEXMF_TOOL" check "$MAC_DIR/Fonts/texmf" >"$WORK/texmf-check.json" 2>&1; then
  ok "bundle-texmf.py check apps/mac/Fonts/texmf: $(( $(grep -c '"status": "verified"' "$WORK/texmf-check.json") - 1 )) entries verified (Commander manifest + SUPPLEMENTARY-METRICS.json pin)"
else
  bad "bundle-texmf.py check apps/mac/Fonts/texmf failed: $(grep -E '"(status|reason)"' "$WORK/texmf-check.json" | head -2 | tr -s ' \n' ' ')"
fi
# A corrupted source metric (one byte appended) is refused BEFORE the build.
cp -R "$MAC_DIR/Fonts/texmf" "$WORK/texmf-corrupt"
printf 'x' >> "$WORK/texmf-corrupt/fonts/tfm/public/lm/rm-lmr8.tfm"
FLASHTEX_BUNDLE_TEXMF_ROOT="$WORK/texmf-corrupt" expect_preflight_failure "pinned bundle metric refused" --debug
# A missing source metric is refused BEFORE the build.
cp -R "$MAC_DIR/Fonts/texmf" "$WORK/texmf-missing"
rm "$WORK/texmf-missing/fonts/tfm/public/lm/ec-lmr10.tfm"
FLASHTEX_BUNDLE_TEXMF_ROOT="$WORK/texmf-missing" expect_preflight_failure "pinned bundle metric refused" --debug
# A metric hidden behind a symlink is refused (no-follow walk).
cp -R "$MAC_DIR/Fonts/texmf" "$WORK/texmf-symlink"
rm "$WORK/texmf-symlink/fonts/tfm/public/lm/ec-lmr12.tfm"
ln -s "$MAC_DIR/Fonts/texmf/fonts/tfm/public/lm/ec-lmr12.tfm" "$WORK/texmf-symlink/fonts/tfm/public/lm/ec-lmr12.tfm"
FLASHTEX_BUNDLE_TEXMF_ROOT="$WORK/texmf-symlink" expect_preflight_failure "pinned bundle metric refused" --debug
FLASHTEX_BUNDLE_TEXMF_ROOT="$WORK/texmf-absent" expect_preflight_failure "pinned bundle metrics root not found" --debug

section "Pinned Latin Modern faces (SUPPLEMENTARY-FACES.json; no build, no download, no host TeX)"
if python3 "$TEXMF_TOOL" check "$MAC_DIR/Fonts/texmf" "$MAC_DIR/Fonts" >"$WORK/faces-check.json" 2>&1; then
  ok "bundle-texmf.py check apps/mac/Fonts/texmf apps/mac/Fonts: $(grep -c '"tier": "supplementary-face"' "$WORK/faces-check.json") supplementary faces + 3 Commander-pinned faces verified"
else
  bad "bundle-texmf.py check with apps/mac/Fonts failed: $(grep -E '"(status|reason)"' "$WORK/faces-check.json" | head -2 | tr -s ' \n' ' ')"
fi
# Every face the render pipeline can request (fonts.rs latin_modern_file at
# 9aaec57a) is vendored: 8 regular + 7 bold + 5 italic + 1 bold-italic + math,
# plus the secondary double-struck math face (NewCMMath-Regular, mathfont::BB_FONT_FILE).
FACES_MISSING=""
for f in lmroman{5,6,7,8,9,10,12,17}-regular lmroman{5,6,7,8,9,10,12}-bold lmroman{7,8,9,10,12}-italic lmroman10-bolditalic latinmodern-math NewCMMath-Regular; do
  [[ -f "$MAC_DIR/Fonts/$f.otf" ]] || FACES_MISSING="$FACES_MISSING $f.otf"
done
if [[ -z "$FACES_MISSING" ]]; then ok "all 23 producer-requestable faces (22 Latin Modern + NewCMMath-Regular for \\mathbb) present in apps/mac/Fonts"; else bad "producer-requestable faces missing:$FACES_MISSING"; fi
# Faces directory copies: a corrupted face, a missing face and an unpinned
# extra .otf are each refused BEFORE the build.
copy_faces() { mkdir -p "$1"; cp "$MAC_DIR/Fonts/"*.otf "$MAC_DIR/Fonts/"*.TXT "$MAC_DIR/Fonts/SUPPLEMENTARY-FACES.json" "$1/"; }
copy_faces "$WORK/fonts-corrupt"; printf 'x' >> "$WORK/fonts-corrupt/lmroman8-regular.otf"
FLASHTEX_BUNDLE_FONTS_DIR="$WORK/fonts-corrupt" expect_preflight_failure "pinned bundle metric refused" --debug
copy_faces "$WORK/fonts-missing"; rm "$WORK/fonts-missing/lmroman6-regular.otf"
FLASHTEX_BUNDLE_FONTS_DIR="$WORK/fonts-missing" expect_preflight_failure "pinned bundle metric refused" --debug
copy_faces "$WORK/fonts-unpinned"; cp "$MAC_DIR/Fonts/lmroman8-regular.otf" "$WORK/fonts-unpinned/lmroman8-stray.otf"
FLASHTEX_BUNDLE_FONTS_DIR="$WORK/fonts-unpinned" expect_preflight_failure "pinned bundle metric refused" --debug
copy_faces "$WORK/fonts-symlink"; rm "$WORK/fonts-symlink/lmroman10-regular.otf"
ln -s "$MAC_DIR/Fonts/lmroman10-regular.otf" "$WORK/fonts-symlink/lmroman10-regular.otf"
FLASHTEX_BUNDLE_FONTS_DIR="$WORK/fonts-symlink" expect_preflight_failure "pinned bundle metric refused" --debug
FLASHTEX_BUNDLE_FONTS_DIR="$WORK/fonts-absent" expect_preflight_failure "pinned bundle fonts directory not found" --debug

section "Resources"
for f in Info.plist.template FlashTeX.entitlements; do
  if plutil -lint "$MAC_DIR/Resources/$f" >/dev/null 2>&1; then ok "plutil -lint Resources/$f"; else bad "plutil -lint Resources/$f"; fi
done
if grep -q '@VERSION@' "$MAC_DIR/Resources/Info.plist.template" && grep -q '@GIT_SHA@' "$MAC_DIR/Resources/Info.plist.template"; then
  ok "Info.plist.template carries @VERSION@ and @GIT_SHA@ placeholders"
else
  bad "Info.plist.template placeholders missing"
fi
ENT_KEYS="$(plutil -convert json -o - "$MAC_DIR/Resources/FlashTeX.entitlements" 2>/dev/null || echo '{}')"
if [[ "$ENT_KEYS" == "{}" ]]; then ok "FlashTeX.entitlements grants no entitlements (empty dict)"; else bad "FlashTeX.entitlements is not empty: $ENT_KEYS (every key needs a documented reason)"; fi
# JetBrains Mono (#742): make-app.sh copies these by name from
# Sources/FlashTeXMac/Resources/Fonts into Contents/Resources/Fonts, and its
# own post-staging check refuses to ship without them -- this just confirms
# the source files that check depends on are still there.
JETBRAINS_MISSING=""
for f in JetBrainsMono-Regular.ttf JetBrainsMono-Bold.ttf JetBrainsMono-Italic.ttf JetBrainsMono-BoldItalic.ttf OFL.txt; do
  [[ -f "$MAC_DIR/Sources/FlashTeXMac/Resources/Fonts/$f" ]] || JETBRAINS_MISSING="$JETBRAINS_MISSING $f"
done
if [[ -z "$JETBRAINS_MISSING" ]]; then
  ok "JetBrains Mono (4 faces + OFL.txt licence) present in Sources/FlashTeXMac/Resources/Fonts"
else
  bad "JetBrains Mono source files missing:$JETBRAINS_MISSING"
fi

section "--help"
for s in make-app.sh launch-check.sh repro-check.sh texmf-acceptance.sh; do
  HELP="$("$SCRIPT_DIR/$s" --help 2>/dev/null || true)"
  if grep -q "Usage:" <<< "$HELP"; then ok "$s --help"; else bad "$s --help"; fi
done
for s in make-app.sh launch-check.sh repro-check.sh packaging-selftest.sh texmf-acceptance.sh; do
  if /bin/bash -n "$SCRIPT_DIR/$s" 2>/dev/null; then ok "$s parses under /bin/bash 3.2"; else bad "$s does not parse under /bin/bash 3.2"; fi
done

if [[ "$FULL" -eq 1 ]]; then
  section "Full: repro-check (ad-hoc)"
  if "$SCRIPT_DIR/repro-check.sh" --evidence "$WORK/repro-adhoc.md" -- ${MAKE_ARGS[@]+"${MAKE_ARGS[@]}"} >"$WORK/repro-adhoc.log" 2>&1; then
    ok "repro-check.sh (ad-hoc): $(grep '^RESULT' "$WORK/repro-adhoc.md")"
  else
    bad "repro-check.sh (ad-hoc) failed: $(grep '^RESULT' "$WORK/repro-adhoc.md" 2>/dev/null || tail -3 "$WORK/repro-adhoc.log")"
  fi
  section "Full: repro-check (--sign -, hardened runtime, SOURCE_DATE_EPOCH pinned)"
  if SOURCE_DATE_EPOCH="$(date +%s)" "$SCRIPT_DIR/repro-check.sh" --evidence "$WORK/repro-hardened.md" -- ${MAKE_ARGS[@]+"${MAKE_ARGS[@]}"} --sign - --dmg >"$WORK/repro-hardened.log" 2>&1; then
    ok "repro-check.sh (--sign - --dmg, pinned): $(grep '^RESULT' "$WORK/repro-hardened.md")"
  else
    bad "repro-check.sh (--sign -) failed: $(grep '^RESULT' "$WORK/repro-hardened.md" 2>/dev/null || tail -3 "$WORK/repro-hardened.log")"
  fi
  FLAGS="$(codesign -dvv "$MAC_DIR/build/FlashTeX.app" 2>&1 | grep '^CodeDirectory' || true)"
  if grep -q 'runtime' <<< "$FLAGS"; then ok "app hardened-runtime signed: $FLAGS"; else bad "app is not hardened-runtime signed: $FLAGS"; fi
  section "Full: launch-check (app, hardened)"
  if "$SCRIPT_DIR/launch-check.sh" --evidence "$WORK/launch-app.md" >"$WORK/launch-app.log" 2>&1 && ! grep -q '^- FAIL' "$WORK/launch-app.md"; then
    ok "launch-check.sh --app: $(grep -c '^- ' "$WORK/launch-app.md") notes, 0 FAIL"
  else
    bad "launch-check.sh --app: $(grep '^- FAIL' "$WORK/launch-app.md" 2>/dev/null | head -3 | tr '\n' ' ')$(tail -2 "$WORK/launch-app.log" | tr '\n' ' ')"
  fi
  section "Full: launch-check (--dmg)"
  if "$SCRIPT_DIR/launch-check.sh" --dmg "$MAC_DIR/build/FlashTeX.dmg" --evidence "$WORK/launch-dmg.md" >"$WORK/launch-dmg.log" 2>&1 && ! grep -q '^- FAIL' "$WORK/launch-dmg.md"; then
    ok "launch-check.sh --dmg: $(grep -c '^- ' "$WORK/launch-dmg.md") notes, 0 FAIL; $(grep -o 'image detached cleanly[^|]*' "$WORK/launch-dmg.md" | head -1)"
  else
    bad "launch-check.sh --dmg: $(grep '^- FAIL' "$WORK/launch-dmg.md" 2>/dev/null | head -3 | tr '\n' ' ')$(tail -2 "$WORK/launch-dmg.log" | tr '\n' ' ')"
  fi
  section "Full: texmf-acceptance (bundled producer, host TeX excluded)"
  if [[ -x "$MAC_DIR/build/FlashTeX.app/Contents/MacOS/flashtex-render" ]]; then
    if "$SCRIPT_DIR/texmf-acceptance.sh" --evidence "$WORK/texmf-acceptance" >"$WORK/texmf-acceptance.log" 2>&1; then
      ok "texmf-acceptance.sh: $(grep '^Result' "$WORK/texmf-acceptance/README.md")"
    else
      bad "texmf-acceptance.sh: $(grep '^- FAIL' "$WORK/texmf-acceptance/README.md" 2>/dev/null | head -3 | tr '\n' ' ')$(tail -2 "$WORK/texmf-acceptance.log" | tr '\n' ' ')"
    fi
    cp "$WORK/texmf-acceptance/README.md" "$WORK/texmf-acceptance.md"
  else
    skip "texmf-acceptance: no bundled flashtex-render (pass -- --render <path>)"
  fi
  if [[ -n "$EVIDENCE_FILE" ]]; then
    EVDIR="$(dirname "$EVIDENCE_FILE")"
    for f in repro-adhoc.md repro-hardened.md launch-app.md launch-dmg.md texmf-acceptance.md; do
      [[ -f "$WORK/$f" ]] && cp "$WORK/$f" "$EVDIR/packaging-selftest-$f"
    done
  fi
fi

echo "==> packaging-selftest: $PASS ok, $FAIL failed"
if [[ -n "$EVIDENCE_FILE" ]]; then
  {
    echo "# packaging-selftest.sh — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "Mode: $([[ "$FULL" -eq 1 ]] && echo full || echo fast); make-app.sh args: ${MAKE_ARGS[*]:-(none)}"
    printf '%s\n' "${LINES[@]}"
    echo
    echo "Result: $PASS ok, $FAIL failed"
  } > "$EVIDENCE_FILE"
  echo "==> Evidence written to $EVIDENCE_FILE"
fi
[[ "$FAIL" -eq 0 ]]
