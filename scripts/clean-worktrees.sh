#!/usr/bin/env bash
# Reclaim disk from idle agent worktrees (DESIGN.md §9.7).
#
# A linked worktree is skipped when a process holds it (its cwd or an open
# file is inside it) or anything in it was modified within ACTIVE_HOURS.
# Otherwise:
#   - unlocked: delete its regenerable build outputs (Cargo target dirs marked
#     by CACHEDIR.TAG or .rustc_info.json, SwiftPM .build, DerivedData,
#     node_modules; never a tracked path). If it lives under .claude/worktrees/
#     (agent-created) and has no uncommitted changes, remove the worktree; a
#     detached HEAD that no branch or remote contains is first kept as branch
#     archive/<worktree-name>.
#   - locked (the Claude Code harness keeps finished agents' worktrees locked):
#     delete only its build outputs, and only when its lane is finished — no
#     uncommitted changes and HEAD merged into MAIN_REF (default origin/main):
#     HEAD is an ancestor of it, or `git cherry` finds every commit's patch in
#     it (rebase merges and single-commit squash merges). A locked worktree is
#     never removed and its sources are never touched.
#
# Usage: scripts/clean-worktrees.sh [--apply] [--active-hours N]
# Without --apply it only prints what it would do and how much it would free.
set -euo pipefail

apply=0
active_hours=${ACTIVE_HOURS:-6}
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) apply=1 ;;
    --active-hours) active_hours=$2; shift ;;
    -h|--help) sed -n '2,21p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

common=$(git rev-parse --path-format=absolute --git-common-dir)
main_wt=$(dirname "$common")
self_wt=$(git rev-parse --show-toplevel)
git -C "$main_wt" worktree prune
main_ref=${MAIN_REF:-origin/main}
main_sha=$(git -C "$main_wt" rev-parse -q --verify "$main_ref^{commit}" 2>/dev/null || true)
[ -n "$main_sha" ] || echo "warning: $main_ref not found; no locked worktree counts as finished" >&2

run() { if [ "$apply" = 1 ]; then "$@"; else echo "would: $*"; fi; }

# One snapshot of every path a process has as cwd or holds open.
open_paths=$(mktemp "${TMPDIR:-/tmp}/clean-worktrees-XXXXXX")
trap 'rm -f "$open_paths"' EXIT
if [ -d /proc ] && [ -e /proc/self/cwd ]; then
  for p in /proc/[0-9]*; do
    readlink "$p/cwd"
    for f in "$p"/fd/*; do readlink "$f"; done
  done > "$open_paths" 2>/dev/null || true
elif command -v lsof >/dev/null; then
  lsof -n -P -Fn 2>/dev/null | sed -n 's/^n//p' > "$open_paths" || true
fi
held() { awk -v w="$1" '$0 == w || index($0, w "/") == 1 { f = 1; exit } END { exit !f }' "$open_paths"; }

# A file modified recently anywhere (build dirs included: a running build
# counts). Finder's .DS_Store writes do not count.
recent() { [ -n "$(find "$1" -type f ! -name .DS_Store -newermt "-${active_hours} hours" -print -quit 2>/dev/null)" ]; }

merged() {
  [ -n "$main_sha" ] || return 1
  git -C "$main_wt" merge-base --is-ancestor "$1" "$main_sha" 2>/dev/null && return 0
  local c
  c=$(git -C "$main_wt" cherry "$main_sha" "$1" 2>/dev/null) || return 1
  [ -n "$c" ] && ! printf '%s\n' "$c" | grep -q '^+'
}

kb_of() { du -sk "$1" 2>/dev/null | awk '{ print $1 + 0 }'; }

# Delete the build outputs of worktree $1; adds their size to wt_build_kb.
clean_builds() {
  local wt=$1 d rel kb
  wt_build_kb=0
  while IFS= read -r d; do
    rel=${d#"$wt"/}
    if [ "$(basename "$d")" = target ] && [ ! -f "$d/CACHEDIR.TAG" ] && [ ! -f "$d/.rustc_info.json" ]; then continue; fi
    if [ -n "$(git -C "$wt" ls-files -- "$rel" | head -1)" ]; then continue; fi
    kb=$(kb_of "$d")
    echo "build output, $((kb / 1024)) MB: $d"
    run rm -rf "$d"
    cleaned=$((cleaned + 1)) wt_build_kb=$((wt_build_kb + kb))
  done < <(find "$wt" -maxdepth 5 -type d \( -name target -o -name .build -o -name DerivedData -o -name node_modules \) -prune 2>/dev/null)
  freed_kb=$((freed_kb + wt_build_kb))
}

removed=0 cleaned=0 kept=0 freed_kb=0

while IFS= read -r line; do
  case "$line" in
    "worktree "*) wt=${line#worktree }; locked=0; head= ;;
    "HEAD "*) head=${line#HEAD } ;;
    locked*) locked=1 ;;
    "")
      [ -n "${wt:-}" ] || continue
      if [ "$wt" = "$main_wt" ] || [ "$wt" = "$self_wt" ] || [ ! -d "$wt" ]; then wt=; continue; fi
      if held "$wt"; then echo "held by a process, skipped: $wt"; kept=$((kept + 1)); wt=; continue; fi
      if recent "$wt"; then echo "modified in the last ${active_hours}h, skipped: $wt"; kept=$((kept + 1)); wt=; continue; fi
      dirty=$(git -C "$wt" status --porcelain 2>/dev/null) || dirty="git status failed"
      if [ "$locked" = 1 ]; then
        if [ -n "$dirty" ]; then
          echo "locked with uncommitted changes, skipped: $wt"; kept=$((kept + 1))
        elif [ -z "$head" ] || ! merged "$head"; then
          echo "locked, lane not merged into $main_ref, skipped: $wt"; kept=$((kept + 1))
        else
          echo "locked, lane finished, build outputs only: $wt"
          clean_builds "$wt"; kept=$((kept + 1))
        fi
        wt=; continue
      fi
      case "$wt" in
        "$main_wt"/.claude/worktrees/*) removable=$([ -z "$dirty" ] && echo 1 || echo 0) ;;
        *) removable=0 ;;
      esac
      [ "$removable" = 1 ] && wt_kb=$(kb_of "$wt")
      clean_builds "$wt"
      if [ "$removable" = 1 ]; then
        if [ -z "$(git -C "$wt" branch --show-current)" ] &&
           [ -z "$(git -C "$main_wt" branch -a --contains "$head" 2>/dev/null | head -1)" ]; then
          run git -C "$main_wt" branch "archive/$(basename "$wt")" "$head"
        fi
        run git -C "$main_wt" worktree remove --force "$wt"; removed=$((removed + 1))
        freed_kb=$((freed_kb + wt_kb - wt_build_kb))
      else
        case "$wt" in "$main_wt"/.claude/worktrees/*) echo "uncommitted changes, kept: $wt" ;; esac
        kept=$((kept + 1))
      fi
      wt= ;;
  esac
done < <(git -C "$main_wt" worktree list --porcelain; echo)

echo "worktrees removed: $removed; build dirs deleted: $cleaned; kept: $kept;" \
  "$([ "$apply" = 1 ] && echo freed || echo 'would free (dry run)'):" \
  "$(awk -v k="$freed_kb" 'BEGIN { printf "%.1f GB", k / 1048576 }')"
