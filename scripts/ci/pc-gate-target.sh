#!/usr/bin/env bash
# pc-gate-target.sh: the build directory of the merge-queue gate's jobs on the
# NixOS PC (ci.yml's build-pc, quick-pc and checks-pc). Prints
# `CARGO_TARGET_DIR=...` and `CARGO_BUILD_JOBS=...` for $GITHUB_ENV.
#
# One directory per runner instance, outside the checkout (the checkout is
# cleaned every job), so a gate rebuilds only what the group changed: the warm
# cache that the hosted jobs get from Swatinem/rust-cache. Per runner, not
# shared: cargo locks a target directory for a whole build. Cargo never
# collects old artifacts, so a directory above PC_GATE_TARGET_MAX_GB (default
# 30) is removed first and rebuilt cold.
#
# CARGO_BUILD_JOBS (default 4): three runner instances share the PC with the
# owner's work; each instance is a systemd service in flashtex.slice with its
# own MemoryHigh, and 4 rustc processes stay well inside it.
set -euo pipefail
dir="$HOME/.cache/flashtex-ci/${RUNNER_NAME:?}/target-gate"
max_gb="${PC_GATE_TARGET_MAX_GB:-30}"
mkdir -p "$dir"
used_kb=$(du -sk "$dir" | awk '{ print $1 }')
if [ "$used_kb" -gt $((max_gb * 1024 * 1024)) ]; then
  echo "pc-gate-target: $dir is $((used_kb / 1048576)) GB (> $max_gb GB): removing it" >&2
  rm -rf "$dir" && mkdir -p "$dir"
fi
echo "pc-gate-target: $dir ($((used_kb / 1048576)) GB used)" >&2
echo "CARGO_TARGET_DIR=$dir"
echo "CARGO_BUILD_JOBS=${PC_GATE_BUILD_JOBS:-4}"
