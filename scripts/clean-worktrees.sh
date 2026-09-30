#!/usr/bin/env bash
# Reclaim disk from idle agent worktrees (DESIGN.md §9.7).
#
# For every linked worktree of this repository that is idle — not locked, no
# process with its cwd inside it, and nothing modified for ACTIVE_HOURS:
#   1. delete regenerable build outputs: Cargo target dirs (marked by
#      CACHEDIR.TAG or .rustc_info.json) and SwiftPM .build dirs, never a
#      tracked path;
#   2. if it lives under .claude/worktrees/ (agent-created) and has no
#      uncommitted changes, remove the worktree. A detached HEAD that no branch
#      or remote contains is first kept as branch archive/<worktree-name>.
# Worktrees with uncommitted changes, and worktrees outside .claude/worktrees/,
# are never removed — only their build outputs are.
#
# Usage: scripts/clean-worktrees.sh [--apply] [--active-hours N]
# Without --apply it only prints what it would do.
set -euo pipefail

apply=0
active_hours=${ACTIVE_HOURS:-6}
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) apply=1 ;;
    --active-hours) active_hours=$2; shift ;;
    -h|--help) sed -n '2,17p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

common=$(git rev-parse --path-format=absolute --git-common-dir)
main_wt=$(dirname "$common")
self_wt=$(git rev-parse --show-toplevel)
git -C "$main_wt" worktree prune

run() { if [ "$apply" = 1 ]; then "$@"; else echo "would: $*"; fi; }

in_use() {
  local wt=$1
  if [ -d /proc ] && [ -e /proc/self/cwd ]; then
    for c in /proc/[0-9]*/cwd; do
      case "$(readlink "$c" 2>/dev/null)" in "$wt"|"$wt"/*) return 0 ;; esac
    done
  elif command -v lsof >/dev/null; then
    lsof -d cwd -Fn 2>/dev/null | grep -q -e "^n$wt\$" -e "^n$wt/" && return 0
  fi
  # A file modified recently anywhere (build dirs included: a running build
  # counts). Finder's .DS_Store writes do not count.
  [ -n "$(find "$wt" -type f ! -name .DS_Store -newermt "-${active_hours} hours" -print -quit 2>/dev/null)" ]
}

freed_before=$(df -k "$main_wt" | awk 'NR==2 {print $4}')
removed=0 cleaned=0 kept=0

while IFS= read -r line; do
  case "$line" in
    "worktree "*) wt=${line#worktree }; locked=0 ;;
    locked*) locked=1 ;;
    "")
      [ -n "${wt:-}" ] || continue
      if [ "$wt" = "$main_wt" ] || [ "$wt" = "$self_wt" ] || [ ! -d "$wt" ]; then wt=; continue; fi
      if [ "$locked" = 1 ] || in_use "$wt"; then
        echo "active, skipped: $wt"; kept=$((kept + 1)); wt=; continue
      fi
      while IFS= read -r d; do
        rel=${d#"$wt"/}
        if [ "$(basename "$d")" = target ] && [ ! -f "$d/CACHEDIR.TAG" ] && [ ! -f "$d/.rustc_info.json" ]; then continue; fi
        if [ -n "$(git -C "$wt" ls-files -- "$rel" | head -1)" ]; then continue; fi
        run rm -rf "$d"; cleaned=$((cleaned + 1))
      done < <(find "$wt" -maxdepth 5 -type d \( -name target -o -name .build \) -prune 2>/dev/null)
      case "$wt" in
        "$main_wt"/.claude/worktrees/*)
          if [ -z "$(git -C "$wt" status --porcelain 2>/dev/null)" ]; then
            head=$(git -C "$wt" rev-parse HEAD)
            if [ -z "$(git -C "$wt" branch --show-current)" ] &&
               [ -z "$(git -C "$main_wt" branch -a --contains "$head" 2>/dev/null | head -1)" ]; then
              run git -C "$main_wt" branch "archive/$(basename "$wt")" "$head"
            fi
            run git -C "$main_wt" worktree remove --force "$wt"; removed=$((removed + 1))
          else
            echo "uncommitted changes, kept: $wt"; kept=$((kept + 1))
          fi ;;
      esac
      wt= ;;
  esac
done < <(git -C "$main_wt" worktree list --porcelain; echo)

freed_after=$(df -k "$main_wt" | awk 'NR==2 {print $4}')
echo "worktrees removed: $removed; build dirs deleted: $cleaned; kept: $kept;" \
  "freed: $(( (freed_after - freed_before) / 1048576 )) GB$([ "$apply" = 1 ] || echo ' (dry run)')"
