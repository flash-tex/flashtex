#!/usr/bin/env bash
# INCR-FILE-SOUNDNESS final gates on the NixOS PC: the branch tip, tools/incr-bench (from PR #1269,
# untracked here) for build/parity/lockstep/trip/etrip/drift/tests/soundness A C D book/gate,
# plus host_tools/checkpoint tests and the external-tools soundness set (xtools.py sound).
set -u
cd ~/code/flashtex-incrfile || exit 1
timeout 300 git fetch -q origin agent/kabir-claude/incr-file-soundness || exit 1
git checkout -q -- tools/external-tools/xtools.py 2>/dev/null
git checkout -q -B agent/kabir-claude/incr-file-soundness FETCH_HEAD || exit 1
export INCR_BENCH_DIR=/tmp/ifs-ib R=/tmp/ifs-ib/raw-final J=${J:-8} CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-8}
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
mkdir -p $R
git log --oneline -1 > $R/head.txt
bash tools/incr-bench/gates.sh build tests
timeout 7200 cargo test --release -p flashtex-engine --test host_tools --test checkpoint > $R/tests-tools.txt 2>&1; echo "tools tests exit $?" >> $R/tests-tools.txt
bash tools/incr-bench/gates.sh sound-a sound-c sound-d sound-book
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib
rm -rf /tmp/ifs-xwork-final
timeout 21600 python3 tools/external-tools/xtools.py sound --host /tmp/ifs-ib/gates/flashtex-host --formats /tmp/ifs-ib/fmt-gates \
  --pool /tmp/ifs-ib/gates/pdftex.pool --texbin $HOME/texlive/2026/bin/x86_64-linux --list $HOME/p5x-sound.txt \
  --work /tmp/ifs-xwork-final -j 6 --trials 2 --out $R/xsound.jsonl > $R/xsound.txt 2>&1; echo "exit $?" >> $R/xsound.txt
unset LD_LIBRARY_PATH
bash tools/incr-bench/gates.sh parity lockstep trip etrip drift gate
echo ALLDONE >> $R/environment.txt
