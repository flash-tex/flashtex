#!/bin/bash
# Verify the vendored snapshots under crates/render-pipeline/vendor/ and report
# how far each one lags its live sibling crate.
#
#   scripts/check-vendor-pins.sh            # integrity check + drift report
#   scripts/check-vendor-pins.sh --quiet    # integrity only
#
# Why this exists: `crates/render-pipeline` builds against FROZEN copies, so a
# fix landed in `crates/compiler` or `crates/tex-expansion` does NOT reach users
# until the snapshot is re-pinned. Nothing previously detected that, and a merged
# fix could sit inert indefinitely while its issue looked closed (this is exactly
# what happened to the TAB catcode fix, #840/#839/#836).
#
# INTEGRITY (hard failure): each vendored tree must be byte-identical to
# `git archive <PIN>:crates/<name>`. A mismatch means somebody edited inside
# vendor/, which VENDORING.md forbids.
#
# DRIFT (reported, never fails): pins are legitimately behind main. The report
# makes the lag visible so a re-pin is a decision rather than an oversight.
set -uo pipefail

cd "$(git rev-parse --show-toplevel)"
VENDOR=crates/render-pipeline/vendor
QUIET=${1:-}
rc=0
drifted=0

for dir in "$VENDOR"/*/; do
  name=$(basename "$dir")
  if [ ! -f "$dir/PIN" ]; then
    echo "WARN  $name: no PIN file -- not checked at all"; rc=1; continue
  fi
  pin=$(tr -d '[:space:]' < "$dir/PIN")

  if ! git cat-file -e "${pin}^{commit}" 2>/dev/null; then
    echo "FAIL  $name: PIN $pin is not a commit in this repository"; rc=1; continue
  fi
  if ! git cat-file -e "${pin}:crates/${name}" 2>/dev/null; then
    echo "SKIP  $name: crates/$name does not exist at $pin"; continue
  fi

  # Integrity: compare the vendored tree against the archive of its PIN.
  tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
  git archive "${pin}:crates/${name}" | tar -x -C "$tmp" 2>/dev/null
  if diff -r -q --exclude=PIN "$tmp" "$dir" >/dev/null 2>&1; then
    integrity="ok"
  else
    echo "FAIL  $name: vendored tree differs from its PIN ${pin:0:12} -- someone edited inside vendor/"
    diff -r -q --exclude=PIN "$tmp" "$dir" 2>&1 | head -5 | sed 's/^/        /'
    rc=1; integrity="MISMATCH"
  fi
  rm -rf "$tmp"; trap - EXIT

  [ "$QUIET" = "--quiet" ] && continue

  # Drift: how far has the live crate moved past the pin?
  if git cat-file -e "HEAD:crates/$name" 2>/dev/null; then
    # Cargo.lock is excluded: since the root workspace (Cargo.toml) a member's
    # own lockfile is gone and unused, so deleting it changes nothing the
    # renderer runs.
    behind=$(git rev-list --count "${pin}..HEAD" -- "crates/$name" ":(exclude)crates/$name/Cargo.lock" 2>/dev/null || echo "?")
    if ! git merge-base --is-ancestor "$pin" HEAD 2>/dev/null; then
      echo "DIVERGED $name: pin ${pin:0:12} is NOT an ancestor of HEAD (pinned from an unmerged"
      echo "         branch); the $behind commit(s) below are divergence, not lag"
      drifted=$((drifted+1)); continue
    fi
    if [ "$behind" != "0" ] && [ "$behind" != "?" ]; then
      echo "DRIFT $name: $behind commit(s) to crates/$name since pin ${pin:0:12} -- NOT in the renderer"
      drifted=$((drifted+1))
    else
      echo "      $name: up to date with crates/$name ($integrity)"
    fi
  fi
done

if [ "$QUIET" != "--quiet" ] && [ "$drifted" -gt 0 ]; then
  echo
  echo "$drifted vendored crate(s) lag their live sibling. That is allowed, but any fix in"
  echo "those commits is INERT for users until re-pinned. Do not close an issue on the"
  echo "strength of a merge alone -- verify through the built CLI."
fi
exit $rc
