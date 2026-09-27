#!/usr/bin/env bash
# Fetch the Adobe Glyph List (glyphlist.txt) that crates/font-engine's Core 14
# tables were generated from, verified against the SHA-256 recorded in
# crates/font-engine/src/generated.rs. Prints the path of the verified file.
#
#   scripts/ci/fetch-glyphlist.sh <work-dir>
#
# Sources, in order; the first whose glyphlist.txt matches the pinned digest
# wins. TLS is always verified; a candidate that fails TLS, HTTP or the digest
# check is skipped, never trusted.
#   1. Adobe's own repository at a fixed commit (5a6882d, "First GitHub
#      Release", Apache-2.0). Byte-identical to TeX Live 2026's
#      texmf-dist/fonts/map/glyphlist/glyphlist.txt (both hash to the pinned
#      digest) and immutable, served from the same network as the runners.
#   2. TeX Live's glyphlist.tar.xz via mirrors.ctan.org, three attempts. That
#      host redirects to a random mirror, and some mirrors serve an incomplete
#      certificate chain: curl exit 60 "unable to get local issuer
#      certificate" (#1101). curl --retry does not retry exit 60, so each
#      attempt is a fresh redirect instead.
set -euo pipefail

work=${1:?usage: fetch-glyphlist.sh <work-dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$work"

want=$(awk '$1 == "//" && $2 == "glyphlist.txt" { print $3 }' \
  "$root/crates/font-engine/src/generated.rs")
[[ $want =~ ^[0-9a-f]{64}$ ]] || {
  echo "fetch-glyphlist: no glyphlist.txt digest in generated.rs header" >&2
  exit 2
}

sha256() { shasum -a 256 "$1" | awk '{ print $1 }'; }

accept() { # <candidate file> <source label>
  local got
  got=$(sha256 "$1")
  if [[ $got == "$want" ]]; then
    mv "$1" "$work/glyphlist.txt"
    echo "fetch-glyphlist: $2 matches $want" >&2
    echo "$work/glyphlist.txt"
    exit 0
  fi
  echo "fetch-glyphlist: $2 has digest $got, want $want; skipping" >&2
}

fetch() { # <url> <out>; curl's own retry covers transient network errors
  curl -fsSL --proto '=https' --tlsv1.2 --retry 2 --connect-timeout 20 \
    --max-time 120 -o "$2" "$1"
}

adobe=https://raw.githubusercontent.com/adobe-type-tools/agl-aglfn/5a6882def2725e66240faa7f40a5c654c2b408fe/glyphlist.txt
if fetch "$adobe" "$work/glyphlist.adobe.txt"; then
  accept "$work/glyphlist.adobe.txt" "adobe-type-tools/agl-aglfn@5a6882d"
else
  echo "fetch-glyphlist: $adobe failed (curl exit $?)" >&2
fi

ctan=https://mirrors.ctan.org/systems/texlive/tlnet/archive/glyphlist.tar.xz
for attempt in 1 2 3; do
  rm -rf "$work/ctan"; mkdir -p "$work/ctan"
  if fetch "$ctan" "$work/ctan/glyphlist.tar.xz" &&
     tar -xJf "$work/ctan/glyphlist.tar.xz" -C "$work/ctan"; then
    found=$(find "$work/ctan" -path '*/glyphlist/glyphlist.txt' -print -quit)
    if [[ -n $found ]]; then
      accept "$found" "mirrors.ctan.org (attempt $attempt)"
    else
      echo "fetch-glyphlist: tarball has no glyphlist.txt" >&2
    fi
  else
    echo "fetch-glyphlist: mirrors.ctan.org attempt $attempt failed" >&2
  fi
  [[ $attempt == 3 ]] || sleep $((attempt * 5))
done

echo "fetch-glyphlist: no source produced a glyphlist.txt with digest $want" >&2
exit 1
