#!/usr/bin/env bash
# mac-heavy-target.sh: the build directory of the heavy self-hosted Mac jobs
# (p5-scoreboard's scoreboard and nightly.yml's engine-nightly-mac, which share
# the concurrency group `flashtex-mac-heavy`, so never build at once). Prints
# `CARGO_TARGET_DIR=...` for $GITHUB_ENV.
#
# One directory for both runner instances on a Mac, because the Macs are short
# of disk (mac-m1max-a: 52 GB free on 2026-10-03). Cargo never collects old
# artifacts, so a directory above MAC_HEAVY_TARGET_MAX_GB (default 40) is
# removed first and rebuilt. Fails, rather than filling the disk, when less
# than MAC_HEAVY_MIN_FREE_GB (default 25) is free after that.
set -euo pipefail
dir="$HOME/.cache/flashtex-ci/mac-heavy/target"
max_gb="${MAC_HEAVY_TARGET_MAX_GB:-40}"
min_free_gb="${MAC_HEAVY_MIN_FREE_GB:-25}"
mkdir -p "$dir"
used_kb=$(du -sk "$dir" | awk '{ print $1 }')
if [ "$used_kb" -gt $((max_gb * 1024 * 1024)) ]; then
  echo "mac-heavy-target: $dir is $((used_kb / 1048576)) GB (> $max_gb GB): removing it" >&2
  rm -rf "$dir" && mkdir -p "$dir"
fi
# df -k: POSIX columns on macOS and Linux (filesystem, blocks, used, available, ...)
free_kb=$(df -k "$dir" | awk 'NR == 2 { print $4 }')
if [ "$free_kb" -lt $((min_free_gb * 1024 * 1024)) ]; then
  echo "::error::only $((free_kb / 1048576)) GB free on this Mac (need $min_free_gb GB for a heavy job)" >&2
  exit 1
fi
echo "mac-heavy-target: $dir ($((used_kb / 1048576)) GB used, $((free_kb / 1048576)) GB free)" >&2
echo "CARGO_TARGET_DIR=$dir"
