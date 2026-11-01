#!/usr/bin/env bash
# tests.sh TAG: soundness tests (release, the rt2 tree) and rt_end.py --verify with ~/rt/ib/TAG
set -u
. ~/rt/env.sh
TAG=$1; W=~/code/flashtex-rt2; R=~/rt/rt2/tests-$1; rm -rf $R; mkdir -p $R
tl=$HOME/texlive/2026
cd $W && ~/.local/bin/ftx-run 3600 cargo test --release -p flashtex-engine --test incremental -- a_dormant_line > $R/inc-dormant.txt 2>&1; echo "classic dormant rc=$? $(grep "test result" $R/inc-dormant.txt)"
(cd $W/crates/flashtex-xetex && FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype ~/.local/bin/ftx-run 3600 cargo test --release --test host -- a_dormant_line_is_kept_until_it_is_read) > $R/host-dormant.txt 2>&1; echo "unicode dormant rc=$? $(grep "test result" $R/host-dormant.txt)"
cd ~/rt
for c in "c1-lipsum end 4" "c2-ifnextchar end 4" "c1-lipsum {Thr 3" "plain-100 Section 3"; do
  set -- $c; d=$1; at=$2; k=$3; [ $d = plain-100 ] && at="Section 13"
  ~/.local/bin/ftx-run 3600 python3 rt_end.py ~/rt/ib/$TAG docs/$d.tex --fmt ~/rt/ib/fmt-$TAG --keys $k --at "$at" --verify > $R/v-$d-$k.txt 2>&1; echo "verify $d at=$at keys=$k rc=$? $(grep -c BAD $R/v-$d-$k.txt) BAD-lines; $(tail -1 $R/v-$d-$k.txt)"
done
echo TESTS-DONE
