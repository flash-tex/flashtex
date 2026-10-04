#!/usr/bin/env bash
# TeX Live's network installer, verified, for the portability workflow:
# install-tl.zip, its install-tl.zip.sha512 and that file's signature
# install-tl.zip.sha512.asc, all three from ONE CTAN mirror (mirror.ctan.org
# picks a mirror per request, and two mirrors can be a sync apart: their
# checksums then differ though neither is wrong). The signature must be a
# good one by TeX Live's key (texlive.asc here, as tug.org publishes it at
# https://www.tug.org/texlive/files/texlive.asc), whose primary fingerprint
# is pinned below; keys.openpgp.org and keyserver.ubuntu.com give the same
# (checked 2026-10-04). Then the zip must match the signed checksum. So a
# mirror can serve an old installer, but not a forged one.
#
# Usage: fetch-install-tl.sh OUT_DIR. Writes `repo=<that mirror's tlnet>`
# to $GITHUB_OUTPUT when set, so the installation uses the same mirror.
set -euo pipefail

PIN=C78B82D8C79512F79CC0D7C80D5E5D9106BAB6BC  # TeX Live Distribution <tex-live@tug.org>
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$1"
if command -v cygpath >/dev/null; then OUT="$(cygpath -u "$OUT")"; fi
mkdir -p "$OUT"
fail() { echo "::error::fetch-install-tl.sh: $*"; exit 1; }

url="$(curl -sSfL --retry 5 --retry-all-errors -o /dev/null -w '%{url_effective}' \
  https://mirror.ctan.org/systems/texlive/tlnet/install-tl.zip.sha512)"
repo="${url%/install-tl.zip.sha512}"
[[ $repo != "$url" && $repo == https://* ]] || fail "unexpected mirror URL $url"
echo "mirror: $repo"
for f in install-tl.zip.sha512 install-tl.zip.sha512.asc install-tl.zip; do
  curl -sSfL --retry 5 --retry-all-errors -o "$OUT/$f" "$repo/$f"
done

g="$(mktemp -d)"
trap 'rm -rf "$g"' EXIT
gpg --homedir "$g" --batch --quiet --import "$HERE/texlive.asc" 2>/dev/null || true
# VALIDSIG's last field is the signing key's primary fingerprint.
fpr="$(gpg --homedir "$g" --batch --status-fd 1 --verify \
  "$OUT/install-tl.zip.sha512.asc" "$OUT/install-tl.zip.sha512" 2>/dev/null |
  awk '$2 == "VALIDSIG" { print $NF }')"
[[ $fpr == "$PIN" ]] || fail "install-tl.zip.sha512 is not signed by TeX Live's key ($PIN; got '${fpr:-no good signature}')"
echo "signature: good, by $fpr"
sha512() { if command -v sha512sum >/dev/null; then sha512sum "$@"; else shasum -a 512 "$@"; fi; }
(cd "$OUT" && sha512 -c install-tl.zip.sha512) || fail "install-tl.zip does not match its signed SHA-512"
if [[ -n ${GITHUB_OUTPUT:-} ]]; then echo "repo=$repo" >>"$GITHUB_OUTPUT"; fi
