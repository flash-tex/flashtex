#!/usr/bin/env bash
# Packages the FlashTeXMac SwiftPM executable as apps/mac/build/FlashTeX.app.
#
# A bare `swift build` product has no Finder/Dock identity: `open -a` fails on
# it and macOS cannot grant it per-app permissions (e.g. local network). This
# wraps the built executable in a minimal .app bundle so it can be launched
# with `open` and eventually granted such permissions.
#
# Usage: apps/mac/scripts/make-app.sh [--debug] [--version <x.y.z>] [--helper-root <repo>]
#          [--cli <path>] [--compiler <path>] [--pdf <path>] [--bridge <path>] [--ledger <path>]
#          [--render <path>] [--pdf-exact <path>] [--controller <path>] [--project-files <path>]
#          [--explain <path>] [--source-sha <key>=<sha>]
#          [--sign <identity>] [--entitlements <file>] [--notarize <keychain-profile>]
#          [--open] [--install] [--install-dir <dir>] [--dmg]
#
# Signing: without --sign the bundle is ad-hoc signed (local use only);
# `--sign -` is ad-hoc through the same hardened-runtime/entitlements path.
# --sign "<Developer ID Application: Name (TEAMID)>" signs every bundled helper
# and then the app with the hardened runtime, a secure timestamp and
# Resources/FlashTeX.entitlements, then verifies with codesign/spctl.
# --notarize <profile> (requires --sign) submits with `xcrun notarytool submit
# --wait` using a keychain profile created by `xcrun notarytool
# store-credentials <profile>`, then staples the app (and the DMG with --dmg).
# --version <x.y.z> (or the APP_VERSION environment variable) sets
# CFBundleShortVersionString; the default below is the last released version.
# CI passes the tag (.github/workflows/release.yml), so cutting a release never
# edits this script.
# --source-sha <key>=<sha> declares the source revision of a helper built
# outside a repository checkout (e.g. from an archive export): components.json
# then records it with git_sha_origin "declared" instead of "resolved".
# Helpers default to <helper-root>/crates/<crate>/target/release/<name>;
# --helper-root defaults to this repository (set it to the main checkout when
# packaging from a worktree). No credential is ever printed by this script.
#
# Rooted TeX metrics (GH36): the five official Latin Modern 2.004 TFMs and the
# rooted GUST license are staged from FLASHTEX_BUNDLE_TEXMF_ROOT (default: the
# vendored apps/mac/Fonts/texmf) into Contents/Resources/texmf/… via
# scripts/bundle-texmf.py. Every file must match the pinned manifest hash or
# packaging refuses BEFORE the build and again before signing; the host TeX
# tree is never consulted and nothing is downloaded. The resulting hashes are
# recorded in components.json ("resources") and resource-coverage.json.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$MAC_DIR/../.." && pwd)"
RESOURCES_SRC="$MAC_DIR/Resources"

DEFAULT_APP_VERSION="0.1.1"
APP_VERSION="${APP_VERSION:-$DEFAULT_APP_VERSION}"
BUNDLE_ID="tech.jay3332.flashtex.mac"

CONFIG="release"
HELPER_ROOT="$REPO_ROOT"
SIGN_IDENTITY=""
ENTITLEMENTS="$RESOURCES_SRC/FlashTeX.entitlements"
NOTARY_PROFILE=""
# Keychain file holding the notarytool profile (CI's temporary keychain);
# empty means notarytool's default, the login keychain.
NOTARY_KEYCHAIN="${FLASHTEX_NOTARY_KEYCHAIN:-}"
NOTARY_KEYCHAIN_ARGS=()
DO_OPEN=0
DO_INSTALL=0
INSTALL_DIR="$HOME/Applications"
DO_DMG=0

# Bundled helpers, in components.json order. Fields: components.json key,
# executable name, crate directory (relative to --helper-root), CLI flag.
# The bridge is additionally built on demand when missing (see below); the
# others are optional and skipped when not built or passed.
HELPER_TABLE=(
  # Staged as flashtex-cli: on a case-insensitive volume "flashtex" would
  # overwrite the app executable "FlashTeX" in the same directory.
  "cli|flashtex-cli|flashtex-cli|--cli"
  "compiler|flashtex-compiler|compiler|--compiler"
  "pdf|flashtex-pdf|pdf|--pdf"
  "bridge|flashtex-bridge|bridge|--bridge"
  "edit_ledger|flashtex-edit-ledger|edit-ledger|--ledger"
  "render|flashtex-render|render-pipeline|--render"
  "pdf_exact|flashtex-pdf-exact|pdf|--pdf-exact"
  "preview_controller|flashtex-preview-controller|preview-controller|--controller"
  "project_files|flashtex-project-files|project-files|--project-files"
  "explain|flashtex-explain|diagnostic-explanations|--explain"
)
# Explicit --<flag> <path> overrides as "key=path" (bash 3.2: no assoc arrays).
HELPER_OVERRIDES=()
# Declared source revisions as "key=sha" (--source-sha), for helpers whose
# build directory is not a repository (an archive export).
HELPER_SHA_OVERRIDES=()

helper_declared_sha_for() {
  local entry
  for entry in ${HELPER_SHA_OVERRIDES[@]+"${HELPER_SHA_OVERRIDES[@]}"}; do
    if [[ "${entry%%=*}" == "$1" ]]; then echo "${entry#*=}"; return 0; fi
  done
  return 0
}

helper_override_for() {
  local entry
  for entry in ${HELPER_OVERRIDES[@]+"${HELPER_OVERRIDES[@]}"}; do
    if [[ "${entry%%=*}" == "$1" ]]; then echo "${entry#*=}"; return 0; fi
  done
  return 0
}

helper_key_for_flag() {
  local row
  for row in "${HELPER_TABLE[@]}"; do
    IFS='|' read -r key _ _ flag <<< "$row"
    if [[ "$flag" == "$1" ]]; then echo "$key"; return 0; fi
  done
  return 1
}

die_early() { echo "make-app.sh: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      CONFIG="debug"
      shift
      ;;
    --helper-root)
      HELPER_ROOT="${2:?--helper-root needs a path}"
      shift 2
      ;;
    --version)
      APP_VERSION="${2:?--version needs x.y.z}"
      shift 2
      ;;
    --cli|--compiler|--pdf|--bridge|--ledger|--render|--pdf-exact|--controller|--project-files|--explain)
      key="$(helper_key_for_flag "$1")"
      HELPER_OVERRIDES+=("$key=${2:-}")
      shift 2
      ;;
    --sign)
      SIGN_IDENTITY="${2:?--sign needs a codesigning identity}"
      shift 2
      ;;
    --entitlements)
      ENTITLEMENTS="${2:?--entitlements needs a file}"
      shift 2
      ;;
    --notarize)
      NOTARY_PROFILE="${2:?--notarize needs a notarytool keychain profile name}"
      shift 2
      ;;
    --open)
      DO_OPEN=1
      shift
      ;;
    --install)
      DO_INSTALL=1
      shift
      ;;
    --install-dir)
      INSTALL_DIR="${2:?--install-dir needs a directory}"
      shift 2
      ;;
    --dmg)
      DO_DMG=1
      shift
      ;;
    --source-sha)
      [[ "${2:-}" == *=* ]] || die_early "--source-sha needs <key>=<sha>"
      [[ "${2%%=*}" =~ ^[a-z_]+$ ]] || die_early "--source-sha: component key must be a components.json key such as render"
      [[ "${2#*=}" =~ ^[0-9a-f]{7,40}$ ]] || die_early "--source-sha: <sha> must be 7-40 hex characters"
      HELPER_SHA_OVERRIDES+=("$2")
      shift 2
      ;;
    -h|--help)
      sed -n '2,30p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *)
      echo "make-app.sh: unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

die() { echo "make-app.sh: $*" >&2; exit 1; }

# A leading "v" (a git tag such as v0.2.0) is accepted and dropped; the
# bundle version must be dotted digits (CFBundleShortVersionString rules).
APP_VERSION="${APP_VERSION#v}"
[[ "$APP_VERSION" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || die "--version/APP_VERSION must be x[.y[.z]] digits, got \"$APP_VERSION\""
echo "==> App version: $APP_VERSION"

# --- Pre-flight: fail before the (slow) build when signing inputs are absent --
# None of these checks print or touch a secret: identities are matched by name
# in `security find-identity` output, and the notarytool profile is looked up
# as a keychain item by service/account only (never read).
SIGN_MODE="adhoc"
HARDENED=0
TIMESTAMP_FLAG="--timestamp"
if [[ "$SIGN_IDENTITY" == "-" ]]; then
  # `--sign -`: ad-hoc identity through the distribution code path (hardened
  # runtime + entitlements + verification) so it can be exercised on a machine
  # without a Developer ID. Not distributable; cannot be timestamped or notarized.
  SIGN_MODE="hardened-adhoc"
  HARDENED=1
  TIMESTAMP_FLAG="--timestamp=none"
  command -v codesign >/dev/null 2>&1 || die "--sign: codesign not found (install Xcode command line tools)"
  [[ -f "$ENTITLEMENTS" ]] || die "--sign: entitlements file not found: $ENTITLEMENTS"
  plutil -lint "$ENTITLEMENTS" >/dev/null 2>&1 || die "--sign: entitlements file is not a valid plist: $ENTITLEMENTS"
  echo "==> Signing identity: ad-hoc (-) with hardened runtime and entitlements (local test of the distribution path)"
elif [[ -n "$SIGN_IDENTITY" ]]; then
  SIGN_MODE="developer-id"
  HARDENED=1
  command -v codesign >/dev/null 2>&1 || die "--sign: codesign not found (install Xcode command line tools)"
  [[ -f "$ENTITLEMENTS" ]] || die "--sign: entitlements file not found: $ENTITLEMENTS"
  plutil -lint "$ENTITLEMENTS" >/dev/null 2>&1 || die "--sign: entitlements file is not a valid plist: $ENTITLEMENTS"
  IDENTITY_COUNT="$( (security find-identity -v -p codesigning 2>/dev/null || true) | grep -cE '^ *[0-9]+\)' || true)"
  IDENTITIES="$(security find-identity -v -p codesigning 2>/dev/null || true)"
  if ! grep -qF -e "$SIGN_IDENTITY" <<< "$IDENTITIES"; then
    die "--sign: no valid codesigning identity matching \"$SIGN_IDENTITY\" in the keychain ($IDENTITY_COUNT valid identities found). Install a \"Developer ID Application\" certificate and its private key in the login keychain, or omit --sign for an ad-hoc build."
  fi
  echo "==> Signing identity found: \"$SIGN_IDENTITY\""
fi
if [[ -n "$NOTARY_PROFILE" ]]; then
  [[ -n "$SIGN_IDENTITY" && "$SIGN_IDENTITY" != "-" ]] || die "--notarize requires --sign <Developer ID Application identity>: Apple only notarizes Developer ID-signed, hardened-runtime code (ad-hoc '-' cannot be notarized)."
  xcrun --find notarytool >/dev/null 2>&1 || die "--notarize: xcrun notarytool not found (Xcode 13+ command line tools required)"
  xcrun --find stapler >/dev/null 2>&1 || die "--notarize: xcrun stapler not found"
  # notarytool store-credentials keeps the profile as a keychain item with
  # service com.apple.gke.notary.tool and the profile name as the account,
  # in the login keychain unless it was stored with --keychain <file>; CI
  # uses a temporary keychain and names it in FLASHTEX_NOTARY_KEYCHAIN.
  if [[ -n "$NOTARY_KEYCHAIN" ]]; then
    [[ -f "$NOTARY_KEYCHAIN" ]] || die "--notarize: FLASHTEX_NOTARY_KEYCHAIN is not a keychain file: $NOTARY_KEYCHAIN"
    NOTARY_KEYCHAIN_ARGS=(--keychain "$NOTARY_KEYCHAIN")
  fi
  if ! security find-generic-password -s com.apple.gke.notary.tool -a "$NOTARY_PROFILE" ${NOTARY_KEYCHAIN:+"$NOTARY_KEYCHAIN"} >/dev/null 2>&1; then
    die "--notarize: no notarytool keychain profile named \"$NOTARY_PROFILE\"${NOTARY_KEYCHAIN:+ in $NOTARY_KEYCHAIN}. Create it once with: xcrun notarytool store-credentials \"$NOTARY_PROFILE\" --apple-id <id> --team-id <TEAMID> (app-specific password prompted, never passed on the command line), or omit --notarize."
  fi
  echo "==> notarytool keychain profile found: \"$NOTARY_PROFILE\"${NOTARY_KEYCHAIN:+ (keychain $NOTARY_KEYCHAIN)}"
fi

# --- Pre-flight: the completion vocabulary is the compiler's inventory --------
# Completion.Vocabulary decodes Sources/FlashTeXMac/Resources/supported-latex.json,
# a byte-identical copy of crates/compiler/supported/supported-latex.json kept
# by scripts/sync-supported-latex.sh; a stale copy refuses the build.
SUPPORTED_LATEX_SYNC="$SCRIPT_DIR/sync-supported-latex.sh"
SUPPORTED_LATEX_JSON="$MAC_DIR/Sources/FlashTeXMac/Resources/supported-latex.json"
[[ -f "$SUPPORTED_LATEX_SYNC" ]] || die "missing $SUPPORTED_LATEX_SYNC"
"$SUPPORTED_LATEX_SYNC" --check --root "$REPO_ROOT" || die "bundled supported-latex.json is stale; run apps/mac/scripts/sync-supported-latex.sh and commit"

# --- Pre-flight: pinned rooted TFM metrics must verify before the build -------
# GH36: flashtex-render loads its required metrics only from a rooted texmf
# tree; a flat Fonts directory or the build machine's TeX cannot stand in.
BUNDLE_TEXMF_ROOT="${FLASHTEX_BUNDLE_TEXMF_ROOT:-$MAC_DIR/Fonts/texmf}"
BUNDLE_TEXMF_TOOL="$SCRIPT_DIR/bundle-texmf.py"
[[ -f "$BUNDLE_TEXMF_TOOL" ]] || die "missing $BUNDLE_TEXMF_TOOL"
[[ -d "$BUNDLE_TEXMF_ROOT" ]] || die "pinned bundle metrics root not found: $BUNDLE_TEXMF_ROOT (vendored apps/mac/Fonts/texmf, or set FLASHTEX_BUNDLE_TEXMF_ROOT to a verified official LM 2.004 texmf root)"
# The OpenType faces: the three Commander-pinned files plus every other Latin
# Modern Roman master/style the render pipeline can request, pinned in
# Fonts/SUPPLEMENTARY-FACES.json. An unpinned .otf in the directory refuses
# packaging, as does any hash/length drift.
BUNDLE_FONTS_DIR="${FLASHTEX_BUNDLE_FONTS_DIR:-$MAC_DIR/Fonts}"
[[ -d "$BUNDLE_FONTS_DIR" ]] || die "pinned bundle fonts directory not found: $BUNDLE_FONTS_DIR (vendored apps/mac/Fonts, or set FLASHTEX_BUNDLE_FONTS_DIR to a directory holding the pinned Latin Modern OTFs)"
TEXMF_PREFLIGHT="$(python3 "$BUNDLE_TEXMF_TOOL" check "$BUNDLE_TEXMF_ROOT" "$BUNDLE_FONTS_DIR" 2>&1)" || {
  printf '%s\n' "$TEXMF_PREFLIGHT" | grep -E '"(path|status|reason)"' | sed 's/^/    /' >&2
  die "pinned bundle metric refused under $BUNDLE_TEXMF_ROOT, or a Latin Modern face refused/unpinned under $BUNDLE_FONTS_DIR (hash/length mismatch, missing, symlink, or an .otf no pin lists; see above). Nothing is downloaded and the host TeX tree is never used."
}
echo "==> Pinned rooted TFM metrics and Latin Modern faces verified under $BUNDLE_TEXMF_ROOT / $BUNDLE_FONTS_DIR ($(( $(grep -c '"status": "verified"' <<< "$TEXMF_PREFLIGHT") - 1 )) entries)"

# Resolves the short git SHA of the repo that CONTAINS $1 (the resolved source
# path of a bundled binary, before it is copied into the bundle) — never the
# app repo's own SHA for a binary sourced from a different worktree.
component_git_sha() {
  local dir toplevel
  dir="$(cd "$(dirname "$1")" 2>/dev/null && pwd)" || { echo "unknown"; return; }
  toplevel="$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null)" || { echo "unknown"; return; }
  git -C "$toplevel" rev-parse --short HEAD 2>/dev/null || echo "unknown"
}

COMPONENTS_JSON_ENTRIES=()
# Appends one components.json entry. $1=name $2=bundled-path-in-app-or-empty
# $3=source-path-or-empty. Called AFTER the helper is signed, so sha256 is the
# hash of the bytes actually shipped in the bundle.
record_component() {
  local name="$1" bundled_path="$2" source_path="$3"
  if [[ -n "$bundled_path" && -f "$bundled_path" ]]; then
    local sha256 git_sha declared origin="resolved"
    sha256="$(shasum -a 256 "$bundled_path" | awk '{print $1}')"
    git_sha="$(component_git_sha "$source_path")"
    declared="$(helper_declared_sha_for "$name")"
    if [[ -n "$declared" ]]; then
      if [[ "$git_sha" != "unknown" && "$declared" != "$git_sha"* && "$git_sha" != "$declared"* ]]; then
        die "--source-sha $name=$declared contradicts the resolved revision $git_sha of $source_path"
      fi
      git_sha="$declared"; origin="declared"
    fi
    COMPONENTS_JSON_ENTRIES+=("  \"$name\": {\"bundled\": true, \"source_path\": \"$source_path\", \"git_sha\": \"$git_sha\", \"git_sha_origin\": \"$origin\", \"sha256\": \"$sha256\"}")
  else
    COMPONENTS_JSON_ENTRIES+=("  \"$name\": {\"bundled\": false, \"source_path\": null, \"git_sha\": null, \"sha256\": null}")
  fi
}

echo "==> Building FlashTeXMac (-c $CONFIG) in $MAC_DIR"
(cd "$MAC_DIR" && swift build -c "$CONFIG")
BIN_DIR="$(cd "$MAC_DIR" && swift build -c "$CONFIG" --show-bin-path)"
BUILT_EXECUTABLE="$BIN_DIR/FlashTeXMac"
if [[ ! -x "$BUILT_EXECUTABLE" ]]; then
  die "built executable not found at $BUILT_EXECUTABLE"
fi

APP_DIR="$MAC_DIR/build/FlashTeX.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
SAMPLES_DIR="$RESOURCES_DIR/Samples"

echo "==> Assembling bundle at $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR" "$SAMPLES_DIR"

cp "$BUILT_EXECUTABLE" "$MACOS_DIR/FlashTeX"
chmod +x "$MACOS_DIR/FlashTeX"

GIT_SHA="$(cd "$REPO_ROOT" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"

PLIST_TEMPLATE="$RESOURCES_SRC/Info.plist.template"
[[ -f "$PLIST_TEMPLATE" ]] || die "Info.plist template not found: $PLIST_TEMPLATE"
sed -e "s|@VERSION@|$APP_VERSION|g" -e "s|@GIT_SHA@|$GIT_SHA|g" "$PLIST_TEMPLATE" > "$CONTENTS_DIR/Info.plist"
plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null || die "generated Info.plist is invalid"
if grep -q '@[A-Z_]*@' "$CONTENTS_DIR/Info.plist"; then
  die "Info.plist template placeholder left unsubstituted: $(grep -o '@[A-Z_]*@' "$CONTENTS_DIR/Info.plist" | sort -u | tr '\n' ' ')"
fi

printf 'APPL????' > "$CONTENTS_DIR/PkgInfo"

echo "==> Copying fixtures and samples into Contents/Resources/Samples"
FIXTURES_DIR="$REPO_ROOT/protocol/fixtures"
for f in compile-result.json compile-request.json; do
  if [[ -f "$FIXTURES_DIR/$f" ]]; then
    cp "$FIXTURES_DIR/$f" "$SAMPLES_DIR/"
  else
    echo "make-app.sh: warning: missing fixture $FIXTURES_DIR/$f" >&2
  fi
done
# Latin Modern (GUST FL) so the app never depends on a TeX installation: the
# license, README and faces pin are copied here; the .otf faces themselves are
# staged only through the hash-verified path below (never a blind copy).
mkdir -p "$RESOURCES_DIR/Fonts"
cp "$BUNDLE_FONTS_DIR/"*.TXT "$RESOURCES_DIR/Fonts/"
[[ -f "$MAC_DIR/Fonts/README.md" ]] && cp "$MAC_DIR/Fonts/README.md" "$RESOURCES_DIR/Fonts/"
[[ -f "$BUNDLE_FONTS_DIR/SUPPLEMENTARY-FACES.json" ]] && cp "$BUNDLE_FONTS_DIR/SUPPLEMENTARY-FACES.json" "$RESOURCES_DIR/Fonts/"

# JetBrains Mono (SIL OFL 1.1, no Reserved Font Name; licence bundled
# alongside): the editor's default face. EditorFontRegistration.swift
# registers it per-process from Contents/Resources/Fonts first (the same
# direct-copy convention as supported-latex.json above) and only falls back
# to the SwiftPM resource bundle `FlashTeXMac_FlashTeXMac.bundle`, which is a
# `swift build`/`run`/`test` artefact next to the built executable, not part
# of a packaged .app -- so it is read here from source, not from that bundle,
# and nothing else needs to stage that bundle into Contents/MacOS.
JETBRAINS_FONTS_SRC="$MAC_DIR/Sources/FlashTeXMac/Resources/Fonts"
JETBRAINS_FACES=(JetBrainsMono-Regular.ttf JetBrainsMono-Bold.ttf JetBrainsMono-Italic.ttf JetBrainsMono-BoldItalic.ttf OFL.txt)
echo "==> Copying JetBrains Mono into Contents/Resources/Fonts"
for f in "${JETBRAINS_FACES[@]}"; do
  [[ -f "$JETBRAINS_FONTS_SRC/$f" ]] || die "missing $JETBRAINS_FONTS_SRC/$f (JetBrains Mono editor face)"
  cp "$JETBRAINS_FONTS_SRC/$f" "$RESOURCES_DIR/Fonts/$f"
done

if [[ -d "$MAC_DIR/Samples" ]]; then
  cp -R "$MAC_DIR/Samples/." "$SAMPLES_DIR/"
fi
# The compiler's command inventory the editor's completion reads at first use
# (Completion.Vocabulary looks in Contents/Resources first, then in the SwiftPM
# resource bundle that only exists in the .build tree).
cp "$SUPPORTED_LATEX_JSON" "$RESOURCES_DIR/supported-latex.json"
cmp -s "$SUPPORTED_LATEX_JSON" "$RESOURCES_DIR/supported-latex.json" || die "supported-latex.json was not copied into Contents/Resources"
echo "==> Bundled supported-latex.json ($(shasum -a 256 "$RESOURCES_DIR/supported-latex.json" | cut -c1-12)) into Contents/Resources"

# --- Pinned rooted TFM metrics + faces (GH36; before signing, no download/host TeX)
# Re-verifies each source file, copies it to Contents/Resources/texmf/… or
# Contents/Resources/Fonts/, then runs
# crates/rendering-core/tools/verify_bundle_resources.py over the whole
# Resources directory (3 OTFs + 5 TFMs + license) and re-verifies every
# supplementary copy. Refuses signing otherwise.
echo "==> Staging pinned rooted TFM metrics and faces into Contents/Resources (sources: $BUNDLE_TEXMF_ROOT, $BUNDLE_FONTS_DIR)"
RESOURCES_COMPONENT_JSON="$(python3 "$BUNDLE_TEXMF_TOOL" stage "$BUNDLE_TEXMF_ROOT" "$RESOURCES_DIR" "$RESOURCES_DIR/resource-coverage.json" "$BUNDLE_FONTS_DIR" 2>&1)" || {
  printf '%s\n' "$RESOURCES_COMPONENT_JSON" | grep -E '"(path|status|reason)"' | sed 's/^/    /' >&2
  die "pinned bundle resources failed verification; refusing to sign $APP_DIR"
}
echo "    verified $(find "$RESOURCES_DIR/texmf" -type f | wc -l | tr -d ' ') rooted metric/license files + $(ls "$RESOURCES_DIR/Fonts"/*.otf | wc -l | tr -d ' ') pinned Latin Modern faces; report at Contents/Resources/resource-coverage.json"

# --- Post-staging completeness check ------------------------------------------
# Every resource file below is something a code path silently falls back
# away from when absent (EditorFontRegistration.swift -> SF Mono,
# Completion.Vocabulary -> an empty inventory, a missing Info.plist/PkgInfo ->
# a bundle Finder/launchd won't treat as an app), so a gap here would build an
# app that looks fine and is quietly wrong, exactly like #742. This is a flat
# list rather than deriving it from Package.swift's `resources:` so it also
# catches gaps in the explicit `cp` calls above it, not only the SwiftPM ones.
REQUIRED_RESOURCES=(
  "$CONTENTS_DIR/Info.plist"
  "$CONTENTS_DIR/PkgInfo"
  "$RESOURCES_DIR/supported-latex.json"
  "$RESOURCES_DIR/Fonts/GUST-FONT-LICENSE.TXT"
  "$RESOURCES_DIR/Fonts/SUPPLEMENTARY-FACES.json"
  "$RESOURCES_DIR/Fonts/JetBrainsMono-Regular.ttf"
  "$RESOURCES_DIR/Fonts/JetBrainsMono-Bold.ttf"
  "$RESOURCES_DIR/Fonts/JetBrainsMono-Italic.ttf"
  "$RESOURCES_DIR/Fonts/JetBrainsMono-BoldItalic.ttf"
  "$RESOURCES_DIR/Fonts/OFL.txt"
)
echo "==> Verifying every expected resource landed in the bundle"
MISSING_RESOURCES=()
for path in "${REQUIRED_RESOURCES[@]}"; do
  [[ -s "$path" ]] || MISSING_RESOURCES+=("$path")
done
if [[ ${#MISSING_RESOURCES[@]} -gt 0 ]]; then
  die "$(printf 'expected resource missing or empty from %s:\n' "$APP_DIR"; printf '  - %s\n' "${MISSING_RESOURCES[@]}")
refusing to ship a silently-wrong bundle"
fi
echo "    all ${#REQUIRED_RESOURCES[@]} expected resources present"

# --- Helpers -----------------------------------------------------------------
echo "==> Locating built Rust binaries (helper root: $HELPER_ROOT)"
BUNDLED_HELPERS=()   # "key|name|source" for every helper actually copied
for row in "${HELPER_TABLE[@]}"; do
  IFS='|' read -r key name crate flag <<< "$row"
  src="$(helper_override_for "$key")"
  if [[ -z "$src" ]]; then
    default="$HELPER_ROOT/crates/$crate/target/release/$name"
    # The CLI crate builds "flashtex"; it is staged under a distinct name.
    [[ "$key" == "cli" ]] && default="$HELPER_ROOT/crates/$crate/target/release/flashtex"
    if [[ "$key" == "bridge" && ! -f "$default" && -f "$HELPER_ROOT/crates/$crate/Cargo.toml" ]]; then
      echo "    $name not built; building it (cargo build --release in crates/$crate)…"
      if (cd "$HELPER_ROOT/crates/$crate" && cargo build --release); then
        echo "    built $name"
      else
        echo "    warning: cargo build --release failed for crates/$crate" >&2
      fi
    fi
    [[ -f "$default" ]] && src="$default"
  fi
  if [[ -n "$src" && -f "$src" ]]; then
    cp "$src" "$MACOS_DIR/$name"
    chmod +x "$MACOS_DIR/$name"
    echo "    bundled $name from $src"
    BUNDLED_HELPERS+=("$key|$name|$src")
  else
    echo "    no $name found (build crates/$crate or pass $flag <path>); skipping"
    BUNDLED_HELPERS+=("$key|$name|")
  fi
done

# --- Signing (helpers first, then the app) -----------------------------------
# Nested code in Contents/MacOS is sealed into the app signature by its own
# code-directory hash, so each helper is signed before the app. components.json
# is written between the two steps: helper sha256 values are therefore the
# as-shipped bytes, while the app executable's sha256 is necessarily the
# pre-signature hash (its signature seals components.json itself).
sign_helper() {  # $1=path $2=identifier
  if [[ "$HARDENED" -eq 1 ]]; then
    codesign --force --options runtime "$TIMESTAMP_FLAG" --identifier "$2" --sign "$SIGN_IDENTITY" "$1"
  else
    codesign --force --identifier "$2" --sign - "$1"
  fi
}

if command -v codesign >/dev/null 2>&1; then
  echo "==> Signing bundled helpers ($SIGN_MODE, hardened runtime: $([[ "$HARDENED" -eq 1 ]] && echo yes || echo no))"
  for row in "${BUNDLED_HELPERS[@]}"; do
    IFS='|' read -r key name src <<< "$row"
    [[ -n "$src" ]] || continue
    if sign_helper "$MACOS_DIR/$name" "$BUNDLE_ID.$name" 2>"$MACOS_DIR/.codesign.err"; then
      echo "    signed $name ($BUNDLE_ID.$name)"
    else
      cat "$MACOS_DIR/.codesign.err" >&2
      rm -f "$MACOS_DIR/.codesign.err"
      if [[ "$HARDENED" -eq 1 ]]; then die "codesign failed for helper $name"; fi
      echo "    warning: ad-hoc codesign failed for $name" >&2
    fi
    rm -f "$MACOS_DIR/.codesign.err"
  done
else
  echo "    warning: codesign not available on this system; helpers left as built" >&2
fi

for row in "${BUNDLED_HELPERS[@]}"; do
  IFS='|' read -r key name src <<< "$row"
  if [[ -n "$src" ]]; then
    record_component "$key" "$MACOS_DIR/$name" "$src"
  else
    record_component "$key" "" ""
  fi
done

# Pinned resource hashes (fonts, rooted metrics, license) as verified above,
# so the component report carries them before the app signature seals it.
COMPONENTS_JSON_ENTRIES+=("  \"resources\": $RESOURCES_COMPONENT_JSON")

# The app's own entry uses $GIT_SHA (already resolved for the whole repo
# worktree) directly rather than record_component's file-based git lookup,
# which expects a binary path, not a directory.
APP_SHA256="$(shasum -a 256 "$MACOS_DIR/FlashTeX" | awk '{print $1}')"
COMPONENTS_JSON_ENTRIES+=("  \"app\": {\"bundled\": true, \"source_path\": \"$REPO_ROOT\", \"git_sha\": \"$GIT_SHA\", \"sha256\": \"$APP_SHA256\"}")

# Build metadata. `timestamp` honours SOURCE_DATE_EPOCH (reproducible-builds
# convention) so two builds of the same inputs can be made byte-identical; it
# is the only time-dependent value in the bundle.
if [[ -n "${SOURCE_DATE_EPOCH:-}" ]]; then
  BUILD_TIMESTAMP="$(date -u -r "$SOURCE_DATE_EPOCH" +%Y-%m-%dT%H:%M:%SZ)"
else
  BUILD_TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
fi
SIGN_IDENTITY_JSON="null"
[[ -n "$SIGN_IDENTITY" ]] && SIGN_IDENTITY_JSON="\"$SIGN_IDENTITY\""
NOTARIZED_JSON="false"
[[ -n "$NOTARY_PROFILE" ]] && NOTARIZED_JSON="true"
COMPONENTS_JSON_ENTRIES+=("  \"build\": {\"config\": \"$CONFIG\", \"signing\": \"$SIGN_MODE\", \"identity\": $SIGN_IDENTITY_JSON, \"hardened_runtime\": $([[ "$HARDENED" -eq 1 ]] && echo true || echo false), \"notarized\": $NOTARIZED_JSON, \"timestamp\": \"$BUILD_TIMESTAMP\"}")

echo "==> Writing Contents/Resources/components.json"
{
  echo "{"
  LAST_INDEX=$((${#COMPONENTS_JSON_ENTRIES[@]} - 1))
  for i in "${!COMPONENTS_JSON_ENTRIES[@]}"; do
    if [[ "$i" -eq "$LAST_INDEX" ]]; then
      echo "${COMPONENTS_JSON_ENTRIES[$i]}"
    else
      echo "${COMPONENTS_JSON_ENTRIES[$i]},"
    fi
  done
  echo "}"
} > "$RESOURCES_DIR/components.json"
if python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$RESOURCES_DIR/components.json" 2>/dev/null; then
  echo "    components.json written and valid JSON"
else
  echo "    warning: components.json failed JSON validation" >&2
fi

if command -v codesign >/dev/null 2>&1; then
  if [[ "$HARDENED" -eq 1 ]]; then
    echo "==> Codesigning app ($SIGN_MODE, hardened runtime, entitlements: $ENTITLEMENTS)"
    codesign --force --options runtime "$TIMESTAMP_FLAG" --entitlements "$ENTITLEMENTS" \
      --identifier "$BUNDLE_ID" --sign "$SIGN_IDENTITY" "$APP_DIR" \
      || die "codesign failed for $APP_DIR"
    echo "    codesign OK ($SIGN_IDENTITY)"
    echo "==> Verifying signature (codesign --verify --deep --strict)"
    codesign --verify --deep --strict --verbose=2 "$APP_DIR" || die "codesign --verify --deep --strict failed for $APP_DIR"
    echo "    verify OK"
    echo "    entitlements as signed:"
    codesign -d --entitlements - "$APP_DIR" 2>/dev/null | sed 's/^/      /' || true
    echo "==> Gatekeeper assessment (spctl --assess --type execute)"
    if spctl --assess --type execute --verbose=2 "$APP_DIR" 2>&1 | sed 's/^/    /'; then
      echo "    spctl: accepted"
    elif [[ -n "$NOTARY_PROFILE" ]]; then
      echo "    spctl: rejected before notarization (expected: Developer ID without a notarization ticket); re-assessed after stapling below"
    elif [[ "$SIGN_MODE" == "hardened-adhoc" ]]; then
      echo "    spctl: rejected (expected for an ad-hoc identity; Gatekeeper only accepts notarized Developer ID)"
    else
      echo "    warning: spctl rejected the bundle; Developer ID builds need --notarize to pass Gatekeeper on other Macs" >&2
    fi
  else
    echo "==> Ad-hoc codesigning"
    # No --deep: the helpers were signed above and are sealed into this
    # signature by their code-directory hashes; re-signing them here would
    # change the bytes components.json just hashed.
    if codesign --force --identifier "$BUNDLE_ID" --sign - "$APP_DIR"; then
      echo "    codesign OK (ad-hoc)"
      if codesign --verify --deep --strict "$APP_DIR" 2>/dev/null; then
        echo "    codesign --verify --deep --strict OK"
      else
        echo "    warning: codesign --verify --deep --strict failed on the ad-hoc bundle" >&2
      fi
    else
      echo "    warning: codesign failed; bundle is unsigned" >&2
    fi
  fi
else
  echo "    warning: codesign not available on this system; bundle is unsigned" >&2
fi

# --- Notarization ---------------------------------------------------------------
# Submits a zip of the signed app, waits for Apple's verdict, staples the ticket.
notarize_path() {  # $1=path to submit (zip/dmg) $2=what it is (for messages)
  local submit_log="$MAC_DIR/build/notarytool-$2.log"
  echo "==> Submitting $2 for notarization (xcrun notarytool submit --wait, profile \"$NOTARY_PROFILE\")"
  if ! xcrun notarytool submit "$1" --keychain-profile "$NOTARY_PROFILE" ${NOTARY_KEYCHAIN_ARGS[@]+"${NOTARY_KEYCHAIN_ARGS[@]}"} --wait 2>&1 | tee "$submit_log"; then
    die "notarytool submit failed for $2 (see $submit_log; no credential is written there)"
  fi
  if ! grep -qE '^ *status: Accepted' "$submit_log"; then
    local sid
    sid="$(grep -m1 -oE 'id: [0-9a-f-]+' "$submit_log" | awk '{print $2}' || true)"
    die "notarization of $2 was not accepted (status in $submit_log). Reasons: xcrun notarytool log ${sid:-<submission-id>} --keychain-profile \"$NOTARY_PROFILE\""
  fi
  echo "    notarization accepted for $2"
}

if [[ -n "$NOTARY_PROFILE" ]]; then
  ZIP_PATH="$MAC_DIR/build/FlashTeX.zip"
  rm -f "$ZIP_PATH"
  ditto -c -k --keepParent "$APP_DIR" "$ZIP_PATH" || die "ditto failed to zip $APP_DIR"
  notarize_path "$ZIP_PATH" "app"
  echo "==> Stapling ticket to the app"
  xcrun stapler staple "$APP_DIR" || die "stapler staple failed for $APP_DIR"
  xcrun stapler validate "$APP_DIR" || die "stapler validate failed for $APP_DIR"
  echo "==> Gatekeeper re-assessment after stapling"
  spctl --assess --type execute --verbose=2 "$APP_DIR" 2>&1 | sed 's/^/    /' || die "spctl still rejects $APP_DIR after notarization"
  echo "    spctl: accepted"
fi

if [[ "$DO_DMG" -eq 1 ]]; then
  echo "==> Building DMG"
  DMG_PATH="$MAC_DIR/build/FlashTeX.dmg"
  DMG_STAGING="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-dmg.XXXXXX")"
  # ditto preserves the code signature, extended attributes and (if stapled)
  # the notarization ticket; cp -R would drop the xattrs on some filesystems.
  ditto "$APP_DIR" "$DMG_STAGING/FlashTeX.app"
  ln -s /Applications "$DMG_STAGING/Applications"
  rm -f "$DMG_PATH"
  if hdiutil create -volname "FlashTeX" -srcfolder "$DMG_STAGING" -ov -format UDZO "$DMG_PATH" >/dev/null; then
    echo "    DMG at $DMG_PATH"
    if [[ "$HARDENED" -eq 1 ]]; then
      codesign --force "$TIMESTAMP_FLAG" --sign "$SIGN_IDENTITY" "$DMG_PATH" || die "codesign failed for $DMG_PATH"
      echo "    DMG signed"
      if [[ -n "$NOTARY_PROFILE" ]]; then
        notarize_path "$DMG_PATH" "dmg"
        xcrun stapler staple "$DMG_PATH" || die "stapler staple failed for $DMG_PATH"
        xcrun stapler validate "$DMG_PATH" || die "stapler validate failed for $DMG_PATH"
        echo "    DMG stapled"
      fi
    fi
  else
    echo "    warning: hdiutil failed; no DMG produced" >&2
  fi
  rm -rf "$DMG_STAGING"
fi

# Launches $1 with `open -n` (a new instance even if another FlashTeX is
# running) and prints the pid of THAT instance, found by diffing the set of
# FlashTeX pids before/after and confirming its executable lives under $1.
# Nothing is ever killed by process name.
launch_app_pid() {
  local app="$1" before after pid exe
  before=" $( (pgrep -x FlashTeX || true) | tr '\n' ' ') "
  open -n --env FLASHTEX_NO_ACTIVATE=1 "$app"
  for _ in $(seq 1 20); do
    after="$(pgrep -x FlashTeX || true)"
    for pid in $after; do
      [[ "$before" == *" $pid "* ]] && continue
      exe="$(ps -p "$pid" -o comm= 2>/dev/null || true)"
      if [[ "$exe" == "$app/"* ]]; then echo "$pid"; return 0; fi
    done
    sleep 0.5
  done
  return 1
}

if [[ "$DO_INSTALL" -eq 1 ]]; then
  echo "==> Installing to $INSTALL_DIR"
  DEST="$INSTALL_DIR/FlashTeX.app"
  PREVIOUS="$INSTALL_DIR/FlashTeX-previous.app"
  STAGED="$INSTALL_DIR/.FlashTeX.app.staging.$$"
  mkdir -p "$INSTALL_DIR"
  rm -rf "$STAGED"

  # Copy into a hidden staging name first, then rename (same directory, so
  # the final `mv` is a single atomic rename): $DEST never briefly points at
  # a half-copied bundle.
  ditto "$APP_DIR" "$STAGED"
  if [[ -d "$DEST" ]]; then
    rm -rf "$PREVIOUS"
    mv "$DEST" "$PREVIOUS"
    echo "    kept previous install at $PREVIOUS pending launch verification"
  fi
  mv "$STAGED" "$DEST"
  echo "    installed $DEST"

  echo "==> Verifying the installed app launches (open -n --env FLASHTEX_NO_ACTIVATE=1; only that pid is terminated)"
  if INSTALLED_PID="$(launch_app_pid "$DEST")"; then
    echo "    launch OK (FlashTeX pid $INSTALLED_PID running from $DEST)"
    kill -TERM "$INSTALLED_PID" 2>/dev/null || true
    for _ in $(seq 1 10); do kill -0 "$INSTALLED_PID" 2>/dev/null || break; sleep 0.5; done
    if [[ -d "$PREVIOUS" ]]; then
      rm -rf "$PREVIOUS"
      echo "    removed $PREVIOUS (new install verified)"
    fi
  else
    echo "    warning: installed app did not launch within 10s; rolling back" >&2
    rm -rf "$DEST"
    if [[ -d "$PREVIOUS" ]]; then
      mv "$PREVIOUS" "$DEST"
      echo "    restored previous install at $DEST" >&2
    fi
    exit 1
  fi
fi

echo "==> Done: $APP_DIR"
echo "    open \"$APP_DIR\""

if [[ "$DO_OPEN" -eq 1 ]]; then
  open "$APP_DIR"
fi
