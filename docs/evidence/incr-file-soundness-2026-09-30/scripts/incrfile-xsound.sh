#!/usr/bin/env bash
# INCR-FILE-SOUNDNESS: xtools sound. TAG LIST TRIALS [J]
T=$1; L=$2; TR=${3:-2}; J=${4:-4}
B=${B:-/tmp/ifs-dev}
cd ~/code/flashtex-incrfile || exit 1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib
rm -rf /tmp/ifs-xwork-$T
timeout 14400 python3 tools/external-tools/xtools.py sound --host $B/eng/flashtex-host --formats $B/fmt --pool $B/eng/pdftex.pool \
  --texbin $HOME/texlive/2026/bin/x86_64-linux --list $L --work /tmp/ifs-xwork-$T -j $J --trials $TR \
  --out /tmp/ifs-xsound-$T.jsonl > /tmp/ifs-xsound-$T.txt 2>&1
echo "exit $?" >> /tmp/ifs-xsound-$T.txt
