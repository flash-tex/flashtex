#!/usr/bin/env bash
# Install the FlashTeX-pinned Beads (bd) and Dolt CLIs into a user prefix.
#
# Pins (owner decision 2026-09-13): bd 1.2.2, dolt 2.3.3. Never 1.2.0/1.2.1,
# never a 1.3.0 release candidate. Upgrades follow docs/coordination/beads.md.
#
# Sources are the official GitHub release tarballs. Every tarball is verified
# against the sha256 recorded below before anything is extracted. Homebrew and
# npm are deliberately not used, because they cannot hold a version.
#
# Usage: scripts/beads/install-pinned.sh [--prefix DIR]
#   default prefix: ${FLASHTEX_BEADS_PREFIX:-$HOME/.local/share/flashtex-beads}
# Result: DIR/bin/bd and DIR/bin/dolt. Use scripts/beads/bd rather than calling
# DIR/bin/bd directly: the wrapper enforces the pins, the gate and the metrics setting.
#
# Idempotent: when the installed binaries already match the pinned tarball
# hashes and report the pinned versions, nothing is downloaded again.
set -euo pipefail

BD_VERSION=1.2.2
DOLT_VERSION=2.3.3
PREFIX="${FLASHTEX_BEADS_PREFIX:-$HOME/.local/share/flashtex-beads}"

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "install-pinned: unknown argument: $1" >&2; exit 2 ;;
  esac
done

os="$(uname -s)"; arch="$(uname -m)"
case "$os/$arch" in
  Darwin/arm64)   bd_plat=darwin_arm64; dolt_plat=darwin-arm64 ;;
  Linux/x86_64)   bd_plat=linux_amd64;  dolt_plat=linux-amd64 ;;
  # Hashes below come from the GitHub release asset digests but have NOT been
  # exercised on real FlashTeX machines:
  Darwin/x86_64)  bd_plat=darwin_amd64; dolt_plat=darwin-amd64 ;;
  Linux/aarch64|Linux/arm64) bd_plat=linux_arm64; dolt_plat=linux-arm64 ;;
  *) echo "install-pinned: unsupported platform $os/$arch (windows/freebsd: install manually from the same releases and record the sha256)" >&2; exit 1 ;;
esac

# sha256 of the official release assets. bd values match beads' checksums.txt
# (sha256 25507c2d3ac43d17a1e7dc6ea25141b0354300cebf3f3f549b7ba3d266ee4f69);
# dolt publishes no checksums file, so its values are the GitHub asset digests.
bd_sha() {
  case "$1" in
    darwin_arm64) echo 2aa1245c666419900d2d6993a05049e92c40e0e601d19579cca5b07a7bb8021d ;;
    linux_amd64)  echo 8140098a51d3b81d5548d1c5e6db1a2d9930e5d141efe2a4bff7d079c4d321e8 ;;
    darwin_amd64) echo e192dcb60f0f48d9463cd3bcf425d41fc0a632080cf9a06153c97ce12368c4bb ;;
    linux_arm64)  echo 501f38a1070d4b9b3b6261a86a3c92c4a52366869021560430a4bb0036afd83a ;;
  esac
}
dolt_sha() {
  case "$1" in
    darwin-arm64) echo 55c11d34df78d7583f1130a1adef7763340f2aade33e68ccb0046854134ad08b ;;
    linux-amd64)  echo 4acd730a4c53991996854a72fbb1add102b0a583bd07411320efb65037a43d9d ;;
    darwin-amd64) echo e33f4fa00032054e38da78b31314f8e93fa9eb950c587ac5f29ba5c6402b3f2a ;;
    linux-arm64)  echo 850a880aece6587cb9251ea0f07eb51fcc0a37450471fd89e03ac2fba1fdaed3 ;;
  esac
}

bd_tar="beads_${BD_VERSION}_${bd_plat}.tar.gz"
bd_url="https://github.com/gastownhall/beads/releases/download/v${BD_VERSION}/${bd_tar}"
dolt_tar="dolt-${dolt_plat}.tar.gz"
dolt_url="https://github.com/dolthub/dolt/releases/download/v${DOLT_VERSION}/${dolt_tar}"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

bd_dir="$PREFIX/bd-$BD_VERSION"
dolt_dir="$PREFIX/dolt-$DOLT_VERSION"
mkdir -p "$PREFIX/bin" "$bd_dir" "$dolt_dir"

fetch_verified() { # url file expected_sha -> path to verified tarball
  local url="$1" file="$2" want="$3" tmp got
  tmp="$(mktemp -d)"
  curl -fsSL --retry 3 -o "$tmp/$file" "$url"
  got="$(sha256_of "$tmp/$file")"
  if [ "$got" != "$want" ]; then
    echo "install-pinned: CHECKSUM MISMATCH for $file: got $got want $want" >&2
    rm -rf "$tmp"; exit 1
  fi
  echo "verified sha256 $got  $file" >&2
  echo "$tmp/$file"
}

want_bd="$(bd_sha "$bd_plat")"; want_dolt="$(dolt_sha "$dolt_plat")"

if [ "$(cat "$bd_dir/.tarball-sha256" 2>/dev/null)" = "$want_bd" ] && [ -x "$bd_dir/bd" ]; then
  echo "bd $BD_VERSION already installed from verified tarball ($want_bd)"
else
  t="$(fetch_verified "$bd_url" "$bd_tar" "$want_bd")"
  tar -xzf "$t" -C "$(dirname "$t")"
  install -m 0755 "$(dirname "$t")/bd" "$bd_dir/bd"
  echo "$want_bd" > "$bd_dir/.tarball-sha256"
  rm -rf "$(dirname "$t")"
fi

if [ "$(cat "$dolt_dir/.tarball-sha256" 2>/dev/null)" = "$want_dolt" ] && [ -x "$dolt_dir/dolt" ]; then
  echo "dolt $DOLT_VERSION already installed from verified tarball ($want_dolt)"
else
  t="$(fetch_verified "$dolt_url" "$dolt_tar" "$want_dolt")"
  tar -xzf "$t" -C "$(dirname "$t")"
  install -m 0755 "$(dirname "$t")/dolt-${dolt_plat}/bin/dolt" "$dolt_dir/dolt"
  echo "$want_dolt" > "$dolt_dir/.tarball-sha256"
  rm -rf "$(dirname "$t")"
fi

ln -sfn "$dolt_dir/dolt" "$PREFIX/bin/dolt"

# bd is dynamically linked against glibc on Linux. NixOS refuses to run such
# binaries unless programs.nix-ld is enabled. Without nix-ld, launch the
# UNMODIFIED verified binary through nixpkgs' glibc loader. A GC root keeps that
# glibc alive. Metrics stay off in the shim, because the metrics flusher
# re-executes os.Executable(), and under the loader that is ld.so itself.
rm -f "$PREFIX/bin/bd"
# Preference order on NixOS:
#   1. The verified binary runs directly. This works once programs.nix-ld is
#      enabled: /lib64/ld-linux-x86-64.so.2 then points at nix-ld, not the stub.
#   2. Fallback: the glibc-loader shim below.
# FLASHTEX_BEADS_FORCE_SHIM=1 forces the fallback, for testing.
if [ "$os" = Linux ] && [ -e /etc/NIXOS ] && [ -z "${FLASHTEX_BEADS_FORCE_SHIM:-}" ] && BD_DISABLE_METRICS=1 "$bd_dir/bd" version >/dev/null 2>&1; then
  echo "NixOS: bd runs directly ($(readlink -f /lib64/ld-linux-x86-64.so.2 2>/dev/null || echo 'no /lib64 loader'); nix-ld detected), no shim"
fi
if [ "$os" = Linux ] && [ -e /etc/NIXOS ] && { [ -n "${FLASHTEX_BEADS_FORCE_SHIM:-}" ] || ! BD_DISABLE_METRICS=1 "$bd_dir/bd" version >/dev/null 2>&1; }; then
  if ! command -v nix-build >/dev/null 2>&1; then
    echo "install-pinned: NixOS without nix-build; enable programs.nix-ld (see docs/coordination/beads.md)" >&2; exit 1
  fi
  nix-build '<nixpkgs>' -A glibc --out-link "$PREFIX/nix-glibc" >/dev/null
  loader="$(ls "$PREFIX"/nix-glibc/lib/ld-linux-*.so.* | head -1)"
  cat > "$PREFIX/bin/bd" <<EOF
#!/bin/sh
# NixOS shim generated by install-pinned.sh (no nix-ld). Runs the verified bd binary unmodified.
export BD_DISABLE_METRICS=1
exec "$loader" --library-path "$PREFIX/nix-glibc/lib" "$bd_dir/bd" "\$@"
EOF
  chmod 0755 "$PREFIX/bin/bd"
  echo "NixOS: bd runs via nixpkgs glibc loader shim ($loader)"
else
  ln -sfn "$bd_dir/bd" "$PREFIX/bin/bd"
fi

bd_out="$(BD_DISABLE_METRICS=1 "$PREFIX/bin/bd" version)"
dolt_out="$("$PREFIX/bin/dolt" version | head -1)"
case "$bd_out" in "bd version $BD_VERSION "*) ;; *) echo "install-pinned: unexpected bd version: $bd_out" >&2; exit 1 ;; esac
case "$dolt_out" in "dolt version $DOLT_VERSION"*) ;; *) echo "install-pinned: unexpected dolt version: $dolt_out" >&2; exit 1 ;; esac

# Metrics off, persisted in the user-global bd config. The wrapper also exports
# BD_DISABLE_METRICS=1, so metrics stay off even if this config is lost.
BD_DISABLE_METRICS=1 "$PREFIX/bin/bd" metrics off >/dev/null

echo "$bd_out"
echo "$dolt_out"
echo "bd tarball sha256:   $want_bd ($bd_tar)"
echo "dolt tarball sha256: $want_dolt ($dolt_tar)"
echo "prefix: $PREFIX  (use scripts/beads/bd)"
