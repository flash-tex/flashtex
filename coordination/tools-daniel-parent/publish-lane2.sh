#!/bin/zsh
# publish-lane2.sh <lane> [<lane>...]
# Fetch a Muse lane, merge main, resolve the known conflict family, regenerate
# derived artefacts, run BOTH profiles on every touched crate, push only if green.
set -u
REPO=/Users/dqi26/flashtex
S=/tmp/claude-503/-Users-dqi26-flashtex/ad5c1298-8c05-46e3-95c6-f8f6b313317c/scratchpad
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$HOME/flashtex-wt/lane-target}
GEN=(docs/user/compiler.md crates/compiler/supported/supported-latex.json crates/compiler/supported/coverage.md apps/mac/Sources/FlashTeXMac/Resources/supported-latex.json)
git -C "$REPO" fetch -q origin
for L in "$@"; do
  W=$HOME/flashtex-wt/pub-${L}; M=$HOME/flashtex-muse-home/lanes/${L}
  echo "=== ${L}"
  [ -d "$M" ] || { echo "   no lane dir"; continue; }
  # HARD GATE: the lane's own commits must not touch an excluded crate.
  # Run BEFORE any fetch/merge, so nothing excluded ever enters the repo.
  if ! $S/audit-lane.sh "$L" > /tmp/audit_${L}.txt 2>&1; then
    echo "   *** EXCLUSION AUDIT FAILED -- NOT PUBLISHING ***"
    sed 's/^/   /' /tmp/audit_${L}.txt
    continue
  fi
  echo "   exclusion audit: clean"
  rm -rf "$W"; git -C "$REPO" worktree prune
  git -C "$REPO" fetch -q "$M" "+refs/heads/muse/${L}:refs/muse/${L}" 2>/dev/null || { echo "   fetch FAILED"; continue; }
  git -C "$REPO" branch -D "muse/${L}" 2>/dev/null
  git -C "$REPO" worktree add -q -b "muse/${L}" "$W" "refs/muse/${L}" 2>/dev/null || { echo "   worktree FAILED"; continue; }
  echo "   $(git -C $W log --oneline origin/main..HEAD | wc -l | tr -d ' ') commit(s) on top of main"
  git -C "$W" merge --no-commit --no-ff origin/main >/dev/null 2>&1
  U=$(git -C "$W" diff --name-only --diff-filter=U); SRC=()
  for f in ${(f)U}; do
    case " ${GEN[*]} " in (*" $f "*) git -C "$W" checkout --ours -- "$f" 2>/dev/null;; (*) SRC+=("$W/$f");; esac
  done
  if [ ${#SRC[@]} -gt 0 ]; then
    python3 $S/resolve_inventory.py ${SRC[@]} 2>&1 | grep -E '  (UNION|TUPLE|BRACE|SKIPPED|resolved)' | sed 's/^/   /'
  fi
  git -C "$W" add -A 2>/dev/null
  if [ -d "$W/crates/compiler" ]; then ( cd "$W/crates/compiler" && sh scripts/render_supported_latex.sh >/dev/null 2>&1 ); fi
  git -C "$W" add -A 2>/dev/null
  MK=$(git -C "$W" grep -lE '^(<{7}|={7}|>{7})( |$)' -- . 2>/dev/null | wc -l | tr -d ' ')
  echo "   markers: ${MK}"
  [ "$MK" != "0" ] && { echo "   ABORT (markers)"; git -C "$W" merge --abort 2>/dev/null; continue; }
  git -C "$W" diff --cached --quiet 2>/dev/null || git -C "$W" -c user.name=d-q222 -c user.email=279808976+d-q222@users.noreply.github.com commit -q -m "Merge origin/main into muse/${L}; resolve inventory conflicts and regenerate derived artefacts

Implementation-Agent: muse-spark-1.3-contributor
Commit-Executor: daniel-parent
Co-authored-by: Claude Opus 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012c9XLkHjePPGBuarrmE2mz"
  CR=$(git -C "$W" diff --name-only origin/main...HEAD -- 'crates/*' | cut -d/ -f2 | sort -u)
  echo "   crates: $(echo $CR|tr '\n' ' ')"
  FAIL=""
  for c in ${(f)CR}; do
    [ -f "$W/crates/${c}/Cargo.toml" ] || continue
    for mode in "" "--release"; do
      r=$( cd "$W/crates/${c}" && cargo test -q ${mode} 2>&1 | grep -E '^(test result|error\[|error:)' )
      okc=$(echo "$r"|grep -c 'result: ok'); fc=$(echo "$r"|grep -c FAILED); ec=$(echo "$r"|grep -cE '^error')
      echo "   ${c} ${mode:-debug}: ok=${okc} failed=${fc} errors=${ec}"
      [ "$fc" != "0" -o "$ec" != "0" ] && { FAIL="${FAIL} ${c}"; echo "$r"|grep -E 'FAILED|^error'|head -3|sed 's|^|      |'; }
    done
  done
  if [ -n "$FAIL" ]; then echo "   NOT PUSHING:${FAIL}"; else
    git -C "$W" push -q origin "HEAD:muse/${L}" 2>&1 | grep -viE 'dependabot|vulnerab|^remote' | head -1
    echo "   PUSHED ${L} ($(git -C $W rev-parse --short HEAD))"
  fi
done
echo PUBLISH-DONE
