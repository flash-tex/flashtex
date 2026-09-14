#!/bin/sh
# FlashTeX installer for macOS — the native GUI app (SwiftUI), which bundles
# the CLI at Contents/MacOS/flashtex-cli.
#
#   curl -fsSL https://flash-tex.github.io/flashtex/install.sh | sh
#
# Downloads the pinned release disk image from GitHub, refuses to continue
# unless its SHA-256 matches, and copies FlashTeX.app into /Applications
# (or ~/Applications when /Applications isn't writable). Set
# FLASHTEX_INSTALL_DIR to install somewhere else.
#
# For the engine + CLI alone (no GUI, macOS or Linux), use install-cli.sh
# instead: https://flash-tex.github.io/flashtex/install-cli.sh
set -eu

VERSION="v0.1.4"
SHA256="3f16d7b75da761d656e4c8552d528335af97f34253abfbfd85bda872f8a18a05"
URL="https://github.com/flash-tex/flashtex/releases/download/${VERSION}/FlashTeX.dmg"

say() { printf '%s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fail "FlashTeX is a macOS app; this system is $(uname -s)."
[ "$(uname -m)" = "arm64" ] || fail "this build requires Apple Silicon (arm64); this Mac is $(uname -m)."
macos_major=$(sw_vers -productVersion | cut -d. -f1)
[ "$macos_major" -ge 14 ] || fail "FlashTeX requires macOS 14 or later; this Mac runs $(sw_vers -productVersion)."
command -v curl >/dev/null 2>&1 || fail "curl is required."

dest="${FLASHTEX_INSTALL_DIR:-/Applications}"
if [ -z "${FLASHTEX_INSTALL_DIR:-}" ] && [ ! -w "$dest" ]; then
  dest="$HOME/Applications"
fi
mkdir -p "$dest" || fail "cannot create $dest."
[ -w "$dest" ] || fail "$dest is not writable."

tmp=$(mktemp -d "${TMPDIR:-/tmp}/flashtex-install.XXXXXX")
mount="$tmp/volume"
cleanup() {
  if [ -d "$mount" ]; then hdiutil detach "$mount" -quiet >/dev/null 2>&1 || true; fi
  rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

say "Downloading FlashTeX ${VERSION}..."
curl -fL --progress-bar -o "$tmp/FlashTeX.dmg" "$URL" || fail "download failed: $URL"

actual=$(shasum -a 256 "$tmp/FlashTeX.dmg" | awk '{print $1}')
[ "$actual" = "$SHA256" ] || fail "checksum mismatch (expected $SHA256, got $actual). Nothing was installed."
say "Checksum verified."

mkdir -p "$mount"
hdiutil attach "$tmp/FlashTeX.dmg" -nobrowse -readonly -mountpoint "$mount" -quiet \
  || fail "could not open the disk image."
[ -d "$mount/FlashTeX.app" ] || fail "FlashTeX.app not found in the disk image."

app="$dest/FlashTeX.app"
if [ -e "$app" ]; then
  say "Replacing existing $app"
  rm -rf "$app"
fi
ditto "$mount/FlashTeX.app" "$app" || fail "could not copy FlashTeX.app into $dest."
xattr -dr com.apple.quarantine "$app" 2>/dev/null || true

say ""
say "FlashTeX ${VERSION} installed to $app"
say "Open it with:  open \"$app\""
