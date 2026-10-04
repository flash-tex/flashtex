#!/usr/bin/env bash
# The engine host at run time, on the portability workflow's Windows runner
# (.github/workflows/portability.yml): flashtex-host on an AF_UNIX socket,
# compiling hello.tex twice through dl3-client.
#
#   live    the resident engine, restricted \write18 (cmd.exe), the pipe;
#           the socket's DACL is shown (icacls)
#   export  a child engine, flashtex-initex.exe as `--engine` (so it is
#           told its name by FLASHTEX_INVOKED_AS), full \write18; its pages
#           come back through the export channel (a second AF_UNIX socket,
#           owner-only, peer-checked); env.txt must say the \write18 child
#           did not inherit FLASHTEX_INVOKED_AS
#
# Usage: host.sh BIN_DIR WORK_DIR (BIN_DIR holds flashtex-host,
# flashtex-initex and dl3-client). Needs TeX Live first on PATH.
set -euo pipefail

BIN="$1"
WORK="$2"
if command -v cygpath >/dev/null; then
  BIN="$(cygpath -u "$BIN")"
  WORK="$(cygpath -u "$WORK")"
fi
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
exe="$(command -v cygpath >/dev/null && echo .exe || true)"
win() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else echo "$1"; fi; }
fail() { echo "::error::host.sh: $*"; exit 1; }

export FLASHTEX_POOL="$(win "$ROOT/crates/flashtex-engine/pdftex.pool")"
rm -rf "$WORK"
mkdir -p "$WORK/proj"
cp "$HERE/hello.tex" "$WORK/proj/"
PROJ="$(win "$WORK/proj")"

for mode in live export; do
  # AF_UNIX paths are at most 107 bytes: a short one.
  sock="$(win "$WORK")/$mode.sock"
  host_args=(--socket "$sock" --format pdftex --once --accept-timeout 900)
  client_args=(--root "$PROJ" --main hello.tex --format pdftex --output-dir "$PROJ/$mode")
  if [[ $mode == live ]]; then
    client_args+=(--shell-escape restricted)
  else
    host_args+=(--engine "$(win "$BIN/flashtex-initex$exe")")
    client_args+=(--shell-escape on --export)
  fi
  "$BIN/flashtex-host$exe" "${host_args[@]}" >"$WORK/host-$mode.out" 2>"$WORK/host-$mode.err" &
  pid=$!
  for _ in $(seq 1 1800); do
    grep -q "listening on" "$WORK/host-$mode.out" 2>/dev/null && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  cat "$WORK/host-$mode.out"
  grep -q "listening on" "$WORK/host-$mode.out" || { cat "$WORK/host-$mode.err"; fail "$mode: the host did not listen"; }
  if command -v icacls >/dev/null; then icacls "$sock" || true; fi
  rc=0
  "$BIN/dl3-client$exe" --socket "$sock" "${client_args[@]}" || rc=$?
  wait "$pid" || true
  echo "--- host stderr ($mode)"; cat "$WORK/host-$mode.err"
  [[ $rc -eq 0 ]] || fail "$mode: dl3-client exit $rc"
  log="$WORK/proj/$mode/hello.log"
  [[ -s "$log" && -s "$WORK/proj/$mode/hello.pdf" ]] || fail "$mode: no log or PDF"
  echo "--- $mode log"; cat "$log"
  flat="$(tr -d '\r\n' <"$log")"
  if [[ $mode == live ]]; then
    want='runsystem(kpsewhich --version)...executed safely (allowed).'
  else
    want='runsystem(kpsewhich --version)...executed.'
  fi
  [[ $flat == *"$want"* ]] || fail "$mode: the log lacks $want"
  if [[ -n $exe ]]; then
    [[ $flat == *"plain.tex]"* && $flat != *"[pipe: closed]"* ]] || fail "$mode: the single-quoted pipe did not run"
  fi
done

env_txt="$(tr -d '\r\n ' <"$WORK/proj/env.txt" 2>/dev/null || true)"
echo "env.txt: $env_txt"
[[ $env_txt == clean ]] || fail "the export's \\write18 child saw FLASHTEX_INVOKED_AS ($env_txt)"
echo "host.sh: live and export OK"
