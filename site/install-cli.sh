#!/bin/sh
# FlashTeX installer for the engine + CLI only (no GUI app).
#
#   curl -fsSL https://flash-tex.github.io/flashtex/install-cli.sh | sh
#
# Detects OS/arch, downloads the matching release's
# flashtex-cli-<version>-<platform>.tar.gz from GitHub, verifies it against
# that release's SHA256SUMS, and installs `flashtex` (plus the helper
# binaries) into PREFIX/bin and the fonts/metrics into
# PREFIX/share/flashtex. PREFIX defaults to ~/.local.
#
# Flags (pass after `--` when piping through `sh -s --`):
#   --version TAG   install a specific release instead of the pinned one,
#                   e.g. --version v0.1.1
#   --prefix DIR    install root instead of ~/.local, e.g. --prefix /usr/local
#                   (a system prefix usually needs sudo)
#   --uninstall     remove a previous install from PREFIX and exit
#
#   curl -fsSL https://flash-tex.github.io/flashtex/install-cli.sh | sh -s -- --prefix /usr/local
#
# For the native macOS app (SwiftUI, which bundles this same CLI at
# Contents/MacOS/flashtex-cli), use install.sh instead:
#   curl -fsSL https://flash-tex.github.io/flashtex/install.sh | sh
set -eu

VERSION="{{TAG}}"
PREFIX="$HOME/.local"
UNINSTALL=0
REPO="flash-tex/flashtex"
DOWNLOAD_BASE="https://github.com/$REPO/releases/download"

say() { printf '%s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      [ $# -ge 2 ] || fail "--version needs a tag, e.g. v0.1.2"
      VERSION="$2"; shift 2 ;;
    --prefix)
      [ $# -ge 2 ] || fail "--prefix needs a directory"
      PREFIX="$2"; shift 2 ;;
    --uninstall) UNINSTALL=1; shift ;;
    -h|--help)
      say "Usage: install-cli.sh [--version TAG] [--prefix DIR] [--uninstall]"
      exit 0 ;;
    *) fail "unknown argument: $1 (see --help)" ;;
  esac
done

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/flashtex"

if [ "$UNINSTALL" -eq 1 ]; then
  say "Removing the FlashTeX CLI from $PREFIX..."
  for b in flashtex flashtex-render flashtex-compiler flashtex-pdf flashtex-pdf-exact; do
    rm -f "$BIN_DIR/$b"
  done
  rm -f "$BIN_DIR/Fonts" "$BIN_DIR/texmf"
  rm -rf "$SHARE_DIR"
  say "Done. (Remove any PATH line you added for $BIN_DIR from your shell profile yourself.)"
  exit 0
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) PLATFORM=macos-arm64 ;;
  Linux:x86_64|Linux:amd64) PLATFORM=linux-x86_64 ;;
  *)
    fail "FlashTeX has no prebuilt CLI for $(uname -s) $(uname -m) yet.
Build it from source instead: https://flash-tex.github.io/flashtex/download/#source" ;;
esac

command -v curl >/dev/null 2>&1 || fail "curl is required."
sha256_file() {
  if command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  else fail "need shasum or sha256sum to verify the download."
  fi
}

NAME="flashtex-cli-${VERSION#v}-$PLATFORM"
TARBALL="$NAME.tar.gz"
URL="$DOWNLOAD_BASE/$VERSION/$TARBALL"
SUMS_URL="$DOWNLOAD_BASE/$VERSION/SHA256SUMS"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/flashtex-cli-install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT INT TERM

say "Downloading FlashTeX CLI ${VERSION} (${PLATFORM})..."
curl -fL --progress-bar -o "$tmp/$TARBALL" "$URL" || fail "download failed: $URL"
curl -fsSL -o "$tmp/SHA256SUMS" "$SUMS_URL" || fail "could not fetch checksums: $SUMS_URL"

expected=$(awk -v f="$TARBALL" '$2 == f { print $1; exit }' "$tmp/SHA256SUMS")
[ -n "$expected" ] || fail "$TARBALL is not listed in ${VERSION}'s SHA256SUMS."
actual=$(sha256_file "$tmp/$TARBALL")
[ "$actual" = "$expected" ] || fail "checksum mismatch for $TARBALL (expected $expected, got $actual). Nothing was installed."
say "Checksum verified."

tar -C "$tmp" -xzf "$tmp/$TARBALL" || fail "could not extract $TARBALL."
STAGE="$tmp/$NAME"
[ -d "$STAGE/bin" ] || fail "unexpected tarball layout: no bin/ in $NAME."

mkdir -p "$BIN_DIR" "$SHARE_DIR" || fail "cannot create $BIN_DIR or $SHARE_DIR (choose a --prefix you own, or sudo for a system prefix)."
[ -w "$BIN_DIR" ] || fail "$BIN_DIR exists but is not writable (choose a --prefix you own, or sudo for a system prefix)."

cp -R "$STAGE/share/flashtex/." "$SHARE_DIR/"
for f in "$STAGE"/bin/*; do
  base=$(basename "$f")
  case "$base" in
    Fonts|texmf) continue ;;  # relative symlinks into share/flashtex; PREFIX gets its own below
  esac
  [ -f "$f" ] || continue
  cp "$f" "$BIN_DIR/$base"
  chmod +x "$BIN_DIR/$base"
done
ln -sf ../share/flashtex/Fonts "$BIN_DIR/Fonts"
ln -sf ../share/flashtex/texmf "$BIN_DIR/texmf"

[ -x "$BIN_DIR/flashtex" ] || fail "install produced no $BIN_DIR/flashtex."

say ""
say "FlashTeX CLI ${VERSION} installed to $BIN_DIR/flashtex"
case ":$PATH:" in
  *":$BIN_DIR:"*) : ;;
  *) say "Add it to your PATH:  export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac
say ""
"$BIN_DIR/flashtex" --version
"$BIN_DIR/flashtex" fonts
