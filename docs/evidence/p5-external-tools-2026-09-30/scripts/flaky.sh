#!/usr/bin/env bash
# Open 2501.07356v3 N times in parallel (load like the parity run) with a given build; count broken .toc files.
B=${B:-/tmp/p5x-dev}
N=${N:-8}
cd ~/code/flashtex-p5tools || exit 1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib
for i in $(seq 1 $N); do
  (timeout 1200 python3 tools/external-tools/debug_open.py --host $B/eng/flashtex-host --formats $B/fmt --pool $B/eng/pdftex.pool \
     --doc /home/kubar/.cache/flashtex-parity/src/arxiv/2501.07356v3:NodalMoments.tex --out /tmp/p5x-flaky-$(basename $B)-$i > /tmp/p5x-flaky-$(basename $B)-$i.txt 2>&1) &
done
wait
bad=0
for i in $(seq 1 $N); do
  if head -c 1 /tmp/p5x-flaky-$(basename $B)-$i/cand/NodalMoments.toc | od -An -c | grep -q '\\0'; then bad=$((bad+1)); echo "run $i: zeroed toc"; grep status /tmp/p5x-flaky-$(basename $B)-$i.txt | head -3; fi
done
echo "$B: $bad of $N broken"
