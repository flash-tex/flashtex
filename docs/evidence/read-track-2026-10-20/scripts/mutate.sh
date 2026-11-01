#!/usr/bin/env bash
# mutate.sh: each READ-TRACK rule disabled in turn (a temporary edit of the PC's tree, put back
# after), and tests/incremental.rs::a_dormant_line_is_kept_until_it_is_read run against it.
# Prints, per rule, the first case that fails without it (or that none does).
set -u
. ~/rt/env.sh
cd ~/code/flashtex-readtrack
D=crates/flashtex-engine/src/dormant.rs
cp $D /tmp/rt-dormant.rs
run() {
  touch $D
  ~/.local/bin/ftx-run 3000 cargo test --release -p flashtex-engine --test incremental -- a_dormant_line 2>&1 \
    | grep -E "panicked|differs from a scratch|no jump|test result" | head -4 | cut -c1-220
}
mut() {
  echo "=== without: $1"
  cp /tmp/rt-dormant.rs $D
  perl -0pi -e "$2" $D
  if cmp -s $D /tmp/rt-dormant.rs; then echo "(the edit did not apply)"; return; fi
  run
}
mut "the return count"  's/\(d\.epoch, d\.fired, d\.opens, d\.shown, d\.level\) == \(b\.epoch, b\.fired,/(d.epoch, b.fired, d.opens, d.shown, d.level) == (b.epoch, b.fired,/'
mut "the print count"   's/        bump\(&self\.shown\);/        let _ = \&self.shown;/'
mut "the open count"    's/        bump\(&self\.opens\);/        let _ = \&self.opens;/'
mut "the words put in"  's/    g\.arena\.set_from\(old, &dj\.words\)\?;/    let _ = &dj.words;/'
mut "the stream put in" 's/                \*f = snap\.clone\(\);/                let _ = (f, snap);/'
cp /tmp/rt-dormant.rs $D
touch $D
echo "=== restored"
run
