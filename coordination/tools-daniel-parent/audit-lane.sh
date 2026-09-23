#!/bin/zsh
# audit-lane.sh <lane> [<lane>...]
# Check a Muse lane's OWN commits against the owner's crate exclusion list.
#
# CRITICAL: lane clones have NO `origin` remote -- the harness removes it so a
# lane can never reach GitHub. So `git log origin/main..HEAD` inside a lane does
# not fail loudly; it yields ZERO commits, and any audit built on it reports a
# clean result having examined nothing. That is how this script was wrong the
# first time. We therefore take the commit SHAs from the lane's own check-in
# front-matter and use `git show --name-only`, which needs no remote ref.
set -u
CHECKINS=$HOME/flashtex-muse-home/checkins
LANES=$HOME/flashtex-muse-home/lanes
EXCL='crates/(font-engine|font-resources|paragraph-layout|math-layout|microtype|tex-boxes|pdf)/|^apps/|vendor/'

rc=0
for L in "$@"; do
  d=$LANES/$L
  f=$(ls -1t $CHECKINS/$L/checkin-*.md 2>/dev/null | head -1)
  print -r -- "=== $L"
  if [[ ! -d $d ]]; then print -r -- "   no lane dir"; rc=1; continue; fi
  if [[ -z $f ]]; then print -r -- "   no check-in yet"; continue; fi

  # SHAs from the check-in's `commits:` block -- the lane's own claim of what it wrote
  shas=(${(f)"$(sed -n '/^commits:/,/^---/p' $f | grep -oE '^  - [0-9a-f]{7,40}' | awk '{print $2}')"})
  if (( ${#shas} == 0 )); then print -r -- "   check-in lists no commits"; continue; fi

  files=()
  for s in $shas; do
    out=$(git -C $d show --name-only --format= $s 2>/dev/null)
    if [[ -z $out ]]; then print -r -- "   WARN: $s not found in lane clone"; rc=1; continue; fi
    files+=(${(f)out})
  done
  files=(${(u)files})   # unique, drop blanks
  files=(${files:#})

  if (( ${#files} == 0 )); then
    print -r -- "   AUDIT INCONCLUSIVE -- no files resolved; do NOT treat as clean"
    rc=1; continue
  fi

  print -r -- "   ${#shas} commit(s), ${#files} file(s)"
  bad=(${(M)files:#${~:-}})
  bad=()
  for x in $files; do [[ $x =~ $EXCL ]] && bad+=($x); done
  if (( ${#bad} )); then
    print -r -- "   *** EXCLUDED CRATE TOUCHED -- HOLD, DO NOT PUBLISH ***"
    for x in $bad; do print -r -- "       $x"; done
    rc=1
  else
    print -r -- "   clean: no excluded crate touched"
    for x in $files; do print -r -- "       $x"; done
  fi
done
exit $rc
