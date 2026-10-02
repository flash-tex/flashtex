#!/usr/bin/env bash
# Self-test for scripts/clean-worktrees.sh (DESIGN.md §9.7, lane J4). Builds a
# temp repo whose linked worktrees under .claude/worktrees/ each have sources
# and a Cargo-style target/, all backdated past the idle window:
#
#   locked-merged    locked, merged into origin/main: target/ deleted, sources
#                    kept. Also holds an attack dir "ignored<NL>node_modules/
#                    node_modules": only that real dir goes, never <wt>/ignored
#                    or a cwd-relative node_modules/node_modules
#   locked-squashed  locked, its commit landed as a cherry-pick: target/ deleted
#   locked-unmerged  locked, not in origin/main: untouched
#   locked-dirty     locked, merged, uncommitted change: untouched
#   locked-in-use    locked, merged, a process has its cwd inside: untouched
#   locked-symlink   locked, merged; target -> a dir outside, and a symlink to
#                    an outside dir holding node_modules: neither followed
#   locked-tracked   locked, merged, a tracked node_modules/: kept
#   locked-outer     locked, merged, with a nested worktree (locked, unmerged,
#                    dirty) inside: outer target/ deleted, inner's kept
#   held paths       locked, merged worktrees whose paths hold a backslash
#                    sequence or a newline, each held by a process: untouched
#   unlocked-idle    unlocked, clean (main's rule): worktree removed
#   *-link           worktree paths that are symlinks (one unlocked holding a
#                    nested repo, one locked and merged): skipped, untouched
#
# Also: no origin/main means nothing counts as finished; with lsof missing or
# failing --apply is refused, and with lsof failing (or listing nothing)
# after its first run --apply stops with nothing deleted; a build dir whose
# parent is swapped for a symlink out during the run is not followed (these
# lsof cases run on macOS; Linux uses /proc); a build dir whose parent a
# background loop keeps swapping for a symlink out is never followed across
# RACE_RUNS (default 60, flip period RACE_PERIOD 1 ms) runs; a dry run changes nothing and reports what
# --apply would free.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$HERE/../clean-worktrees.sh"
WORK="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/flashtex-clean-wt-test-XXXXXX")" && pwd -P)"
holders=()
cleanup() {
  local h
  for h in ${holders[@]+"${holders[@]}"}; do { kill "$h"; wait "$h"; } 2>/dev/null || true; done
  rm -rf "$WORK"
}
trap cleanup EXIT

FAILS=0
expect() { # expect <description> <command...>
  local what="$1"; shift
  if "$@"; then printf 'ok    %s\n' "$what"; else printf 'FAIL  %s\n' "$what" >&2; FAILS=$((FAILS + 1)); fi
}
not() { ! "$@"; }

R="$WORK/repo"
WTS="$R/.claude/worktrees"
BS='locked-back\nslash'          # a literal backslash-n, not a newline
NL=$'locked-nl\nx'
g() { git -C "$R" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
git init -q -b main "$R"
printf 'target\n.claude/worktrees\nignored*\nnode_modules\n' > "$R/.gitignore"
echo base > "$R/src.rs"
g add -A; g commit -qm base

wt() { # wt <dir> [branch]: branch in <dir>, one commit, a target/
  local p=$1 b=${2:-$(basename "$1")}
  g worktree add -q -b "$b" "$p" main
  echo "$b" > "$p/$b.rs"
  git -C "$p" add -A
  git -C "$p" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm "$b"
  mkdir -p "$p/target/debug"
  : > "$p/target/CACHEDIR.TAG"
  head -c 65536 /dev/zero > "$p/target/debug/blob"
}
for n in locked-merged locked-squashed locked-unmerged locked-dirty locked-in-use \
         locked-symlink locked-tracked locked-outer unlocked-idle; do wt "$WTS/$n"; done
wt "$WTS/unlocked-link"; wt "$WTS/locked-link"
for n in unlocked-link locked-link; do   # move the worktree out, leave a symlink
  mv "$WTS/$n" "$WORK/$n-real"; ln -s "$WORK/$n-real" "$WTS/$n"
  git init -q "$WORK/$n-real/ignored-nested"; echo canary > "$WORK/$n-real/ignored-nested/canary"
done
wt "$WTS/$BS" locked-bs
wt "$WTS/$NL" locked-nl
wt "$WTS/locked-outer/.claude/worktrees/inner" inner

# locked-merged: newline attack dir, and a decoy in the script's cwd.
M="$WTS/locked-merged"
mkdir -p "$M/ignored" "$M/ignored"$'\n'"node_modules/node_modules" "$R/node_modules/node_modules"
echo keep > "$M/ignored/keep.txt"
echo x > "$M/ignored"$'\n'"node_modules/node_modules/x"
echo canary > "$R/node_modules/node_modules/canary"
# locked-symlink: target is a symlink out; lnk points at an outside node_modules.
S="$WTS/locked-symlink"
rm -rf "$S/target"
mkdir -p "$WORK/outside/debug" "$WORK/outside2/node_modules"
: > "$WORK/outside/CACHEDIR.TAG"; echo canary > "$WORK/outside/debug/canary"
echo canary > "$WORK/outside2/node_modules/canary"
ln -s "$WORK/outside" "$S/target"
ln -s "$WORK/outside2" "$S/ignored-lnk"
# locked-tracked: node_modules is tracked.
T="$WTS/locked-tracked"
mkdir -p "$T/node_modules"; echo tracked > "$T/node_modules/keep.js"
git -C "$T" add -f node_modules/keep.js
git -C "$T" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm tracked
# inner: dirty.
I="$WTS/locked-outer/.claude/worktrees/inner"
echo extra >> "$I/src.rs"

g merge -q --no-ff -m merge locked-merged locked-dirty locked-in-use locked-symlink \
  locked-tracked locked-outer locked-bs locked-nl locked-link >/dev/null
g cherry-pick locked-squashed >/dev/null   # same patch, different commit
g update-ref refs/remotes/origin/main main
echo extra >> "$WTS/locked-dirty/src.rs"
for p in "$WTS"/locked-* "$I"; do   # the glob includes the backslash, newline and link paths
  g worktree lock --reason "claude agent" "$p"
done
find "$WORK" -exec touch -h -t 202001010000 {} + 2>/dev/null

for p in "$WTS/locked-in-use" "$WTS/$BS" "$WTS/$NL"; do
  (cd "$p" && exec sleep 300) &
  holders+=($!)
done
sleep 1

has_target() { [ -d "$1/target" ]; }
has_sources() { [ -f "$1/src.rs" ]; }
all_targets() {
  local p
  for p in "$WTS"/locked-merged "$WTS"/locked-squashed "$WTS"/locked-unmerged "$WTS"/locked-dirty \
           "$WTS"/locked-in-use "$WTS"/locked-tracked "$WTS"/locked-outer "$I" "$WTS/$BS" "$WTS/$NL"; do
    has_target "$p" || return 1
  done
}
links_intact() {
  local n
  for n in unlocked-link locked-link; do
    [ -f "$WORK/$n-real/src.rs" ] && [ -d "$WORK/$n-real/target" ] &&
      [ -f "$WORK/$n-real/ignored-nested/canary" ] && [ -d "$WORK/$n-real/ignored-nested/.git" ] || return 1
  done
}
lsof_is_used() { ! { [ -d /proc ] && [ -e /proc/self/cwd ]; }; }

# Dry run: nothing changes, the total is reported.
out=$(cd "$R" && "$SCRIPT" 2>&1) || true
printf '%s\n' "$out" | sed 's/^/  dry: /'
expect "dry run keeps every target" all_targets
expect "dry run keeps unlocked-idle" test -d "$WTS/unlocked-idle"
expect "dry run reports would-free total" grep -q 'would free (dry run)' <<<"$out"

# No origin/main: nothing is finished.
out=$(cd "$R" && MAIN_REF=refs/remotes/origin/nope "$SCRIPT" --apply 2>&1) || true
expect "no origin/main: run reaches its summary" grep -q '^worktrees removed:' <<<"$out"
expect "no origin/main: warning printed" grep -q 'refs/remotes/origin/nope not found' <<<"$out"
expect "no origin/main: no lane finished" not grep -q 'lane finished' <<<"$out"
expect "no origin/main: every locked target kept" all_targets
g worktree add -q -b unlocked-idle-2 "$WTS/unlocked-idle-2" main   # replace the one --apply removed
find "$WTS/unlocked-idle-2" -exec touch -h -t 202001010000 {} +

if lsof_is_used; then
  for l in /nonexistent/lsof false; do
    set +e; out=$(cd "$R" && LSOF=$l "$SCRIPT" --apply 2>&1); rc=$?; set -e
    expect "lsof=$l: --apply refused" test "$rc" = 2
    expect "lsof=$l: refusal explained" grep -q 'refusing --apply' <<<"$out"
    expect "lsof=$l: nothing deleted" all_targets
  done
  out=$(cd "$R" && LSOF=/nonexistent/lsof "$SCRIPT" 2>&1)
  expect "lsof missing: dry run warns" grep -q 'in-use checks are off' <<<"$out"

  # An lsof that works once, then fails or lists nothing.
  REAL_LSOF=$(command -v lsof)
  cat > "$WORK/flaky-lsof" <<'SH'
#!/usr/bin/env bash
n=$(( $(cat "$FLAKY_COUNT" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$FLAKY_COUNT"
if [ "$n" -ge 2 ]; then [ "$FLAKY_MODE" = empty ] && exit 0; exit 1; fi
exec "$REAL_LSOF" "$@"
SH
  chmod +x "$WORK/flaky-lsof"
  for mode in fail empty; do
    rm -f "$WORK/flaky-count"
    set +e
    out=$(cd "$R" && REAL_LSOF=$REAL_LSOF FLAKY_COUNT="$WORK/flaky-count" FLAKY_MODE=$mode \
      LSOF="$WORK/flaky-lsof" "$SCRIPT" --apply 2>&1); rc=$?
    set -e
    expect "lsof $mode mid-run: --apply stops" test "$rc" = 2
    expect "lsof $mode mid-run: stop explained" grep -q 'failed mid-run' <<<"$out"
    expect "lsof $mode mid-run: nothing locked deleted" all_targets
  done

  # TOCTOU: the parent of a build dir is swapped for a symlink out while the
  # script re-lists open files, just before rm.
  R2="$WORK/repo2"; WT2="$R2/.claude/worktrees/toc"
  git init -q -b main "$R2"
  printf '.claude/worktrees\nignored*\nnode_modules\n' > "$R2/.gitignore"
  git -C "$R2" add -A; git -C "$R2" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm base
  git -C "$R2" worktree add -q -b toc "$WT2" main
  git -C "$R2" update-ref refs/remotes/origin/main main
  mkdir -p "$WT2/ignored-sub/node_modules" "$WORK/outside3/node_modules"
  echo x > "$WT2/ignored-sub/node_modules/x"; echo canary > "$WORK/outside3/node_modules/canary"
  git -C "$R2" worktree lock "$WT2"
  find "$R2" "$WORK/outside3" -exec touch -h -t 202001010000 {} +
  cat > "$WORK/swap-lsof" <<'SH'
#!/usr/bin/env bash
n=$(( $(cat "$FLAKY_COUNT" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$FLAKY_COUNT"
if [ "$n" = 2 ]; then mv "$SWAP_WT/ignored-sub" "$SWAP_WT/ignored-moved"; ln -s "$SWAP_OUT" "$SWAP_WT/ignored-sub"; fi
exec "$REAL_LSOF" "$@"
SH
  chmod +x "$WORK/swap-lsof"
  rm -f "$WORK/flaky-count"
  out=$(cd "$R2" && REAL_LSOF=$REAL_LSOF FLAKY_COUNT="$WORK/flaky-count" SWAP_WT="$WT2" \
    SWAP_OUT="$WORK/outside3" LSOF="$WORK/swap-lsof" "$SCRIPT" --apply 2>&1) || true
  printf '%s\n' "$out" | sed 's/^/  toctou: /'
  expect "toctou: the swap happened" test -L "$WT2/ignored-sub"
  expect "toctou: outside canary kept" test -f "$WORK/outside3/node_modules/canary"
  expect "toctou: change reported" grep -q 'changed during the run' <<<"$out"
else
  echo "skip  lsof-missing cases (this host lists open files through /proc)"
fi

# Race: a loop flips a build dir's parent between the real dir and a symlink
# to an outside dir while the script runs; the outside canary must survive.
R3="$WORK/repo3"; WT3="$R3/.claude/worktrees/race"; OUT4="$WORK/outside4"
git init -q -b main "$R3"
printf '.claude/worktrees\nignored*\nnode_modules\n' > "$R3/.gitignore"
git -C "$R3" add -A; git -C "$R3" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm base
git -C "$R3" worktree add -q -b race "$WT3" main
git -C "$R3" update-ref refs/remotes/origin/main main
git -C "$R3" worktree lock "$WT3"
mkdir -p "$OUT4/node_modules"; echo canary > "$OUT4/node_modules/canary"
printf '#!/bin/sh\nprintf "p1\\nfcwd\\nn/nonexistent\\n"\n' > "$WORK/stub-lsof"; chmod +x "$WORK/stub-lsof"
flip() {   # tight loop: shell mv/ln are too slow to hit the rm's start-up window
  python3 -c '
import os, sys, time
wt, out, stop = sys.argv[1:4]
period = float(sys.argv[4])
sub, hold = wt + "/ignored-sub", wt + "/ignored-hold"
while not os.path.exists(stop):
    try:
        os.rename(sub, hold); os.symlink(out, sub)
    except OSError:
        pass
    time.sleep(period)
    try:
        os.unlink(sub); os.rename(hold, sub)
    except OSError:
        pass
    time.sleep(period)
' "$WT3" "$OUT4" "$WORK/flip-stop" "${RACE_PERIOD:-0.001}"
}
race_ok=1 race_deleted=0
for i in $(seq "${RACE_RUNS:-60}"); do
  mkdir -p "$WT3/ignored-sub/node_modules"; echo x > "$WT3/ignored-sub/node_modules/x"
  find "$R3" -exec touch -h -t 202001010000 {} +
  rm -f "$WORK/flip-stop"; flip & flipper=$!; holders+=($flipper)
  (cd "$R3" && LSOF="$WORK/stub-lsof" "$SCRIPT" --apply >/dev/null 2>&1) || true
  touch "$WORK/flip-stop"; wait "$flipper" 2>/dev/null || true
  if [ -L "$WT3/ignored-sub" ]; then rm -f "$WT3/ignored-sub"; fi
  if [ -d "$WT3/ignored-hold" ]; then mv "$WT3/ignored-hold" "$WT3/ignored-sub"; fi
  [ -d "$WT3/ignored-sub/node_modules" ] || race_deleted=$((race_deleted + 1))
  if [ ! -f "$OUT4/node_modules/canary" ]; then race_ok=0; break; fi
done
echo "  race: real build dir deleted in $race_deleted of ${RACE_RUNS:-60} runs"
expect "race: outside canary survives a symlink-flipping parent" test "$race_ok" = 1

set +e; out=$(cd "$R" && "$SCRIPT" --apply 2>&1); rc=$?; set -e
printf '%s\n' "$out" | sed 's/^/  apply: /'
expect "apply exits 0" test "$rc" = 0
expect "apply reaches its summary" grep -q '^worktrees removed:' <<<"$out"
expect "symlinked worktree paths: skipped, untouched" links_intact
expect "symlinked worktree paths: reported" test "$(grep -c 'is a symlink' <<<"$out")" = 2
expect "locked merged: target removed" not has_target "$M"
expect "locked merged: sources kept" has_sources "$M"
expect "locked merged: <wt>/ignored kept" test -f "$M/ignored/keep.txt"
expect "locked merged: cwd node_modules/node_modules kept" test -f "$R/node_modules/node_modules/canary"
expect "locked merged: real newline-named node_modules removed" test ! -e "$M/ignored"$'\n'"node_modules/node_modules"
expect "locked squashed: target removed" not has_target "$WTS/locked-squashed"
expect "locked squashed: sources kept" has_sources "$WTS/locked-squashed"
expect "locked unmerged: untouched" has_target "$WTS/locked-unmerged"
expect "locked dirty: target kept" has_target "$WTS/locked-dirty"
expect "locked dirty: uncommitted change kept" grep -q extra "$WTS/locked-dirty/src.rs"
expect "locked in use: untouched" has_target "$WTS/locked-in-use"
expect "symlink: outside target dir untouched" test -f "$WORK/outside/debug/canary"
expect "symlink: target symlink kept" test -L "$S/target"
expect "symlink: outside node_modules untouched" test -f "$WORK/outside2/node_modules/canary"
expect "tracked node_modules kept" test -f "$T/node_modules/keep.js"
expect "tracked: untracked target removed" not has_target "$T"
expect "nested: outer target removed" not has_target "$WTS/locked-outer"
expect "nested: inner (unmerged, dirty) target kept" has_target "$I"
expect "nested: inner change kept" grep -q extra "$I/src.rs"
expect "backslash path held: untouched" has_target "$WTS/$BS"
expect "newline path held: untouched" has_target "$WTS/$NL"
expect "unlocked idle: worktree removed" test ! -d "$WTS/unlocked-idle-2"
expect "every locked worktree kept and locked" \
  test "$(g worktree list --porcelain -z | tr '\0' '\n' | grep -c '^locked')" = 12

if [ "$FAILS" -ne 0 ]; then echo "$FAILS check(s) failed" >&2; exit 1; fi
echo "all checks passed"
