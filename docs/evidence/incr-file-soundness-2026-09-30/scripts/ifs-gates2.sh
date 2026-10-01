#!/usr/bin/env bash
# INCR-FILE-SOUNDNESS after the #1313 review: the branch tip; at most 6 hosts at once.
set -u
cd ~/code/flashtex-incrfile || exit 1
timeout 300 git fetch -q origin agent/kabir-claude/incr-file-soundness || exit 1
git checkout -q -B agent/kabir-claude/incr-file-soundness FETCH_HEAD || exit 1
export INCR_BENCH_DIR=/tmp/ifs-ib R=/tmp/ifs-ib/raw-r2 J=6 CARGO_BUILD_JOBS=8
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
mkdir -p $R
git log --oneline -1 > $R/head.txt
bash tools/incr-bench/gates.sh build tests
timeout 7200 cargo test --release -p flashtex-engine --test host_tools --test checkpoint > $R/tests-tools.txt 2>&1; echo "tools tests exit $?" >> $R/tests-tools.txt
bash tools/incr-bench/gates.sh sound-a sound-c
rm -f /tmp/ifs-ib/trace-d.txt
FLASHTEX_FILE_TRACE=/tmp/ifs-ib/trace-d.txt bash tools/incr-bench/gates.sh sound-d
grep -c "^fixed " /tmp/ifs-ib/trace-d.txt > $R/trace-d-fixed.txt 2>&1; grep -c "^changed_outside" /tmp/ifs-ib/trace-d.txt >> $R/trace-d-fixed.txt 2>&1
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib
rm -rf /tmp/ifs-xwork-r2
timeout 28800 python3 tools/external-tools/xtools.py sound --host /tmp/ifs-ib/gates/flashtex-host --formats /tmp/ifs-ib/fmt-gates \
  --pool /tmp/ifs-ib/gates/pdftex.pool --texbin $HOME/texlive/2026/bin/x86_64-linux --list $HOME/p5x-sound.txt \
  --work /tmp/ifs-xwork-r2 -j 3 --trials 2 --out $R/xsound.jsonl > $R/xsound.txt 2>&1; echo "exit $?" >> $R/xsound.txt
unset LD_LIBRARY_PATH
bash tools/incr-bench/gates.sh parity lockstep trip etrip drift
echo ALLDONE >> $R/environment.txt
