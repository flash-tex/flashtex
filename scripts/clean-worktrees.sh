#!/usr/bin/env bash
# Reclaim disk from idle agent worktrees (DESIGN.md §9.7).
#
# Unlocked linked worktrees that are idle (no process has its cwd inside, and
# nothing modified for ACTIVE_HOURS):
#   1. delete regenerable build outputs: Cargo target dirs (marked by
#      CACHEDIR.TAG or .rustc_info.json) and SwiftPM .build dirs, never a
#      tracked path;
#   2. if it lives under .claude/worktrees/ (agent-created), has no
#      uncommitted changes and contains no nested repository or worktree,
#      remove the worktree. A detached HEAD that no branch or remote contains
#      is first kept as branch archive/<worktree-name>.
# Locked worktrees (the Claude Code harness keeps finished agents' worktrees
# locked) are only ever cleaned, never removed, and only when the lane is
# finished: no process holds the directory (cwd or any open file, re-checked
# before each deletion), nothing modified for ACTIVE_HOURS, no uncommitted
# changes, and HEAD merged into MAIN_REF (default origin/main) — HEAD is an
# ancestor of it, or `git cherry` finds every commit's patch in it. Their build
# outputs (the above plus DerivedData and node_modules) are deleted.
# Nothing outside a worktree, inside a nested worktree, or reached through a
# symlink is deleted; a worktree whose path is a symlink is skipped. --apply is
# refused when open files cannot be listed, and stops if listing fails mid-run.
#
# Usage: scripts/clean-worktrees.sh [--apply] [--active-hours N]
# Without --apply it only prints what it would do and how much it would free.
set -euo pipefail
unset CDPATH

apply=0
active_hours=${ACTIVE_HOURS:-6}
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) apply=1 ;;
    --active-hours) active_hours=$2; shift ;;
    -h|--help) sed -n '2,26p' "$0"; exit 0 ;;
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

# realpath that keeps trailing newlines: realpath_of <dir> sets REPLY.
realpath_of() { REPLY=$(cd -- "$1" 2>/dev/null && pwd -P && echo .) || return 1; REPLY=${REPLY%$'\n.'}; }

# Escape a path the way lsof prints it; sets REPLY.
esc() { local s=${1//\\/\\\\}; s=${s//$'\n'/\\n}; s=${s//$'\t'/\\t}; s=${s//$'\r'/\\r}; REPLY=$s; }

# Snapshot every process's cwd ("c" lines) and open files ("f" lines).
snap=$(mktemp "${TMPDIR:-/tmp}/clean-worktrees-XXXXXX")
snap_new="$snap.new"
trap 'rm -f "$snap" "$snap_new"' EXIT
lsof_cmd=${LSOF:-lsof}
# snapshot: refresh $snap and set snap_ok=1, or leave $snap as it was and set
# snap_ok=0 when listing fails or lists nothing.
snapshot() {
  snap_ok=0
  if [ -d /proc ] && [ -e /proc/self/cwd ]; then
    { find /proc/[0-9]*/cwd -maxdepth 0 -printf 'c%l\0'
      find /proc/[0-9]*/fd -mindepth 1 -maxdepth 1 -printf 'f%l\0'; } 2>/dev/null |
      while IFS= read -r -d '' p; do esc "${p:1}"; printf '%s%s\n' "${p:0:1}" "$REPLY"; done > "$snap_new" || true
  elif command -v "$lsof_cmd" >/dev/null 2>&1; then
    "$lsof_cmd" -n -P -Ffn 2>/dev/null |
      awk '/^p/ { t = "f" } /^f/ { t = ($0 == "fcwd") ? "c" : "f" } /^n/ { print t substr($0, 2) }' > "$snap_new" || return 0
  else
    return 0
  fi
  [ -s "$snap_new" ] && mv -f "$snap_new" "$snap" && snap_ok=1
  return 0
}
# held <real path> <cwd|any>: a process has its cwd (or, with any, an open
# file) inside. A path with control characters lsof would not print verbatim
# always counts as held.
held() {
  esc "$1"
  case "$REPLY" in *[[:cntrl:]]*) return 0 ;; esac
  W=$REPLY M=$2 awk 'BEGIN { w = ENVIRON["W"]; m = ENVIRON["M"] }
    m == "any" || substr($0, 1, 1) == "c" { p = substr($0, 2); if (p == w || index(p, w "/") == 1) { f = 1; exit } }
    END { exit !f }' "$snap"
}

snapshot
if [ "$snap_ok" != 1 ]; then
  if [ "$apply" = 1 ]; then
    echo "refusing --apply: cannot list processes' open files ($lsof_cmd missing or failed, no /proc)" >&2
    exit 2
  fi
  echo "warning: cannot list processes' open files; in-use checks are off in this dry run" >&2
fi

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

# Uncommitted changes, or git status failing.
dirty() { local s; s=$(git -C "$1" status --porcelain 2>/dev/null) || return 0; [ -n "$s" ]; }

kb_of() { du -sk "$1" 2>/dev/null | awk 'NR == 1 { print $1 + 0 }'; }

# unchanged <real> <parent realpath> <wt realpath>
unchanged() {
  local real=$1 parent=$2 wt_real=$3 p
  [ -d "$real" ] && [ ! -L "$real" ] && [ ! -L "$wt_real" ] || return 1
  realpath_of "${real%/*}" && [ "$REPLY" = "$parent" ] || return 1
  case "$parent/" in "$wt_real"/*) ;; *) return 1 ;; esac
  p=$real
  while [ "$p" != "$wt_real" ]; do
    [ ! -L "$p" ] || return 1
    p=${p%/*}
    [ -n "$p" ] || return 1
  done
}

# clean_builds <wt> <wt real path> <unlocked|locked>: delete its build
# outputs; sets wt_build_kb.
clean_builds() {
  local wt=$1 wt_real=$2 mode=$3 d parent real top rel kb n
  local names=(-name target -o -name .build)
  [ "$mode" = locked ] && names+=(-o -name DerivedData -o -name node_modules)
  wt_build_kb=0
  while IFS= read -r -d '' d; do
    case "$d" in "$wt"/*) ;; *) continue ;; esac
    if [ -L "$d" ] || [ ! -d "$d" ]; then continue; fi
    realpath_of "${d%/*}" || continue
    parent=$REPLY real="$REPLY/${d##*/}"
    case "$parent/" in "$wt_real"/*) ;; *) echo "outside the worktree, skipped: $d"; continue ;; esac
    top=$(git -C "$parent" rev-parse --show-toplevel 2>/dev/null && echo .) || continue
    [ "${top%$'\n.'}" = "$wt_real" ] || continue   # inside a nested worktree or repository
    if [ "${d##*/}" = target ] && [ ! -f "$real/CACHEDIR.TAG" ] && [ ! -f "$real/.rustc_info.json" ]; then continue; fi
    rel=${real#"$wt_real"/}
    if [ -n "$(git -C "$wt_real" ls-files -- ":(literal)$rel" | head -1)" ]; then continue; fi
    if [ "$mode" = locked ]; then
      snapshot
      if [ "$snap_ok" != 1 ]; then
        if [ "$apply" = 1 ]; then
          echo "stopping: listing open files failed mid-run ($lsof_cmd); nothing more is deleted" >&2
          exit 2
        fi
      elif held "$wt_real" any; then echo "now held by a process, stopped: $wt"; break
      fi
    fi
    kb=$(kb_of "$real")
    # Last checks, right before rm: the parent still resolves to the same
    # place inside the worktree, and no component up to the root is a symlink.
    if ! unchanged "$real" "$parent" "$wt_real"; then echo "changed during the run, skipped: $d"; continue; fi
    echo "build output, $((kb / 1024)) MB: $real"
    if [ "$apply" = 1 ]; then
      # Delete by a relative name from inside the verified parent, so a parent
      # swapped for a symlink after the checks cannot redirect the rm.
      n=${real##*/}
      if ! ( cd -P -- "$parent" && [ "$(pwd -P && echo .)" = "$parent"$'\n.' ] &&
             [ -d "$n" ] && [ ! -L "$n" ] && rm -rf -- "$n" ); then
        echo "changed during the run, skipped: $d"; continue
      fi
    else
      echo "would: rm -rf -- $real"
    fi
    cleaned=$((cleaned + 1)) wt_build_kb=$((wt_build_kb + kb))
  done < <(find "$wt" -maxdepth 5 -type d \( "${names[@]}" \) -prune -print0 2>/dev/null)
  freed_kb=$((freed_kb + wt_build_kb))
}

removed=0 cleaned=0 kept=0 freed_kb=0

while IFS= read -r -d '' line <&3; do
  case "$line" in
    "worktree "*) wt=${line#worktree }; locked=0; head= ;;
    "HEAD "*) head=${line#HEAD } ;;
    locked*) locked=1 ;;
    "")
      [ -n "${wt:-}" ] || continue
      if [ "$wt" = "$main_wt" ] || [ "$wt" = "$self_wt" ] || [ ! -d "$wt" ]; then wt=; continue; fi
      if [ -L "$wt" ] || ! realpath_of "$wt"; then
        echo "worktree path is a symlink or unresolvable, skipped: $wt"; kept=$((kept + 1)); wt=; continue
      fi
      wt_real=$REPLY
      if [ "$locked" = 1 ]; then
        if held "$wt_real" any; then echo "locked and held by a process, skipped: $wt"
        elif recent "$wt"; then echo "locked and modified in the last ${active_hours}h, skipped: $wt"
        elif dirty "$wt"; then
          echo "locked with uncommitted changes, skipped: $wt"
        elif [ -z "$head" ] || ! merged "$head"; then echo "locked, lane not merged into $main_ref, skipped: $wt"
        else
          echo "locked, lane finished, build outputs only: $wt"
          clean_builds "$wt" "$wt_real" locked
        fi
        kept=$((kept + 1)); wt=; continue
      fi
      if held "$wt_real" cwd || recent "$wt"; then
        echo "active, skipped: $wt"; kept=$((kept + 1)); wt=; continue
      fi
      removable=0
      case "$wt" in
        "$main_wt"/.claude/worktrees/*)
          if dirty "$wt"; then
            echo "uncommitted changes, kept: $wt"
          elif [ -n "$(find "$wt" -mindepth 2 -name .git -print -quit 2>/dev/null)" ]; then
            echo "contains a nested repository or worktree, kept: $wt"
          else
            removable=1 wt_kb=$(kb_of "$wt")
          fi ;;
      esac
      clean_builds "$wt" "$wt_real" unlocked
      if [ "$removable" = 1 ]; then
        if [ -z "$(git -C "$wt" branch --show-current)" ] &&
           [ -z "$(git -C "$main_wt" branch -a --contains "$head" 2>/dev/null | head -1)" ]; then
          run git -C "$main_wt" branch "archive/$(basename "$wt")" "$head"
        fi
        run git -C "$main_wt" worktree remove --force "$wt"; removed=$((removed + 1))
        freed_kb=$((freed_kb + wt_kb - wt_build_kb))
      else
        kept=$((kept + 1))
      fi
      wt= ;;
  esac
done 3< <(git -C "$main_wt" worktree list --porcelain -z; printf '\0')

echo "worktrees removed: $removed; build dirs deleted: $cleaned; kept: $kept;" \
  "$([ "$apply" = 1 ] && echo freed || echo 'would free (dry run)'):" \
  "$(awk -v k="$freed_kb" 'BEGIN { printf "%.1f GB", k / 1048576 }')"
