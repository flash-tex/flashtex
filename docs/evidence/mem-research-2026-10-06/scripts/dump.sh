#!/usr/bin/env bash
# memr/dump.sh TAG SRC MAIN STEP...: proto host, dumps logs and pages at the steady open
export INCR_BENCH_DIR=$HOME/ib-memr PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
T=$1; S=$2; M=$3; shift 3
O=$HOME/ib-memr/dump/$T
args=(); for s in "$@"; do args+=(--step "$s"); done
FLASHTEX_DUMP_LOGS=$O.logs FLASHTEX_DUMP_PAGES=$O.pages python3 -u ~/memr/memr.py ~/ib-memr/proto $S $M $O "${args[@]}" > $O.out 2>&1
