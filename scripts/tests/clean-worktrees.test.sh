#!/usr/bin/env bash
# Self-test for scripts/clean-worktrees.sh (DESIGN.md §9.7, lane J4). Builds a
# temp repo with six linked worktrees under .claude/worktrees/, each with
# sources and a Cargo-style target/, all backdated past the idle window:
#
#   locked-merged    locked, branch merged into origin/main: target/ deleted,
#                    sources and worktree kept
#   locked-squashed  locked, branch's commit landed on main as a cherry-pick
#                    (same patch-id): target/ deleted, sources kept
#   locked-unmerged  locked, branch not in origin/main: untouched
#   locked-dirty     locked, merged, uncommitted change: untouched
#   locked-in-use    locked, merged, a process has its cwd inside: untouched
#   unlocked-idle    unlocked, clean (the existing rule): worktree removed
#
# A dry run must change nothing and report what --apply would free.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$HERE/../clean-worktrees.sh"
WORK="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/flashtex-clean-wt-test-XXXXXX")" && pwd -P)"
holder=
cleanup() { [ -z "$holder" ] || { kill "$holder"; wait "$holder"; } 2>/dev/null || true; rm -rf "$WORK"; }
trap cleanup EXIT

FAILS=0
expect() { # expect <description> <command...>
  local what="$1"; shift
  if "$@"; then printf 'ok    %s\n' "$what"; else printf 'FAIL  %s\n' "$what" >&2; FAILS=$((FAILS + 1)); fi
}

R="$WORK/repo"
g() { git -C "$R" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
git init -q -b main "$R"
printf 'target\n' > "$R/.gitignore"
echo base > "$R/src.rs"
g add -A; g commit -qm base

wt() { # wt <name>: branch <name> in .claude/worktrees/<name>, one commit, a target/
  local p="$R/.claude/worktrees/$1"
  g worktree add -q -b "$1" "$p" main
  echo "$1" > "$p/$1.rs"
  git -C "$p" add -A
  git -C "$p" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm "$1"
  mkdir -p "$p/target/debug"
  : > "$p/target/CACHEDIR.TAG"
  head -c 65536 /dev/zero > "$p/target/debug/blob"
}
for n in locked-merged locked-squashed locked-unmerged locked-dirty locked-in-use unlocked-idle; do wt "$n"; done

g merge -q --no-ff -m "merge" locked-merged locked-dirty locked-in-use >/dev/null
g cherry-pick locked-squashed >/dev/null   # same patch, different commit
g update-ref refs/remotes/origin/main main
echo extra >> "$R/.claude/worktrees/locked-dirty/src.rs"
for n in locked-merged locked-squashed locked-unmerged locked-dirty locked-in-use; do
  g worktree lock --reason "claude agent agent-$n" "$R/.claude/worktrees/$n"
done
find "$WORK" -exec touch -h -t 202001010000 {} + 2>/dev/null

(cd "$R/.claude/worktrees/locked-in-use" && exec sleep 300) &
holder=$!
sleep 1

has_target() { [ -d "$R/.claude/worktrees/$1/target" ]; }
has_sources() { [ -f "$R/.claude/worktrees/$1/$1.rs" ] && [ -f "$R/.claude/worktrees/$1/src.rs" ]; }

# Dry run: nothing changes, the freed total is reported.
expect "setup: unlocked-idle exists" test -d "$R/.claude/worktrees/unlocked-idle"
out=$(cd "$R" && "$SCRIPT")
printf '%s\n' "$out" | sed 's/^/  dry: /'
for n in locked-merged locked-squashed locked-unmerged locked-dirty locked-in-use; do
  expect "dry run keeps $n/target" has_target "$n"
done
expect "dry run keeps unlocked-idle" test -d "$R/.claude/worktrees/unlocked-idle"
expect "dry run reports would-free total" grep -q 'would free (dry run)' <<<"$out"
expect "dry run lists locked-merged target" grep -q "build output.*locked-merged/target" <<<"$out"

out=$(cd "$R" && "$SCRIPT" --apply)
printf '%s\n' "$out" | sed 's/^/  apply: /'
expect "locked merged: target removed" eval '! has_target locked-merged'
expect "locked merged: sources kept" has_sources locked-merged
expect "locked squashed: target removed" eval '! has_target locked-squashed'
expect "locked squashed: sources kept" has_sources locked-squashed
expect "locked unmerged: untouched" has_target locked-unmerged
expect "locked unmerged: sources kept" has_sources locked-unmerged
expect "locked dirty: target kept" has_target locked-dirty
expect "locked dirty: uncommitted change kept" grep -q extra "$R/.claude/worktrees/locked-dirty/src.rs"
expect "locked in use: untouched" has_target locked-in-use
expect "unlocked idle: worktree removed" test ! -d "$R/.claude/worktrees/unlocked-idle"
expect "no locked worktree removed" test "$(g worktree list | wc -l | tr -d ' ')" = 6
expect "all five still locked" test "$(g worktree list --porcelain | grep -c '^locked')" = 5

if [ "$FAILS" -ne 0 ]; then echo "$FAILS check(s) failed" >&2; exit 1; fi
echo "all checks passed"
