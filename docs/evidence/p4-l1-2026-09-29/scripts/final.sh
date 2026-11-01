#!/bin/bash
# Final P4-L1 measurement set, raw output into the evidence folder.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a864152353f81f7d3
R=$W/docs/evidence/p4-l1-2026-09-29/raw
mkdir -p $R
V=v8
mkdir -p /tmp/p4l1/$V
cp $W/target/release/flashtex-initex $W/target/release/flashtex-host /tmp/p4l1/$V/
cp $W/crates/flashtex-engine/pdftex.pool /tmp/p4l1/$V/
ln -sf /tmp/p4l1/$V/flashtex-initex /tmp/p4l1/$V/pdftex
ln -sfn /tmp/p4l1/fmt-v3 /tmp/p4l1/fmt-$V
cd /tmp/p4l1
{
  echo "host: mac-m5pro-kabir, $(sysctl -n machdep.cpu.brand_string), $(sysctl -n hw.memsize | awk '{print $1/1073741824 " GiB"}'), macOS $(sw_vers -productVersion), rustc $(rustc --version | cut -d' ' -f2)"
  echo "engine under test: $(cd $W && git rev-parse --short HEAD) (flashtex-initex, flashtex-host); base: #1198 head 3a5ba1cbf; nb: HEAD with feature bench-no-barrier; store: unconditional flag-store barrier (experiment)"
  echo "start: $(date -u +%FT%TZ) $(uptime)"
} > $R/environment.txt
python3 timeit3.py 30 min base:/tmp/p4l1/base:/tmp/p4l1/fmt-base nb:/tmp/p4l1/nb3:/tmp/p4l1/fmt-nb3 $V:/tmp/p4l1/$V:/tmp/p4l1/fmt-$V > $R/barrier-min.jsonl
echo "after min: $(uptime)" >> $R/environment.txt
python3 timeit3.py 16 long-full base:/tmp/p4l1/base:/tmp/p4l1/fmt-base nb:/tmp/p4l1/nb3:/tmp/p4l1/fmt-nb3 store:/tmp/p4l1/sb:/tmp/p4l1/fmt-sb $V:/tmp/p4l1/$V:/tmp/p4l1/fmt-$V > $R/barrier-long-full.jsonl
echo "after long-full: $(uptime)" >> $R/environment.txt
python3 timeit3.py 8 long8 base:/tmp/p4l1/base:/tmp/p4l1/fmt-base nb:/tmp/p4l1/nb3:/tmp/p4l1/fmt-nb3 $V:/tmp/p4l1/$V:/tmp/p4l1/fmt-$V > $R/barrier-long8.jsonl
echo "after long8: $(uptime)" >> $R/environment.txt
python3 bench.py /tmp/p4l1/$V /tmp/p4l1/fmt-$V min - 5 5 > $R/host-min.json
python3 bench.py /tmp/p4l1/$V /tmp/p4l1/fmt-$V long-full body.inc 5 5 > $R/host-long-full.json
python3 bench.py /tmp/p4l1/$V /tmp/p4l1/fmt-$V long8 body.inc 3 3 > $R/host-long8.json
echo "after host: $(uptime)" >> $R/environment.txt
python3 bench.py /tmp/p4l1/$V /tmp/p4l1/fmt-$V long-full body.inc 1 0 --every-shipout > $R/host-long-full-every-shipout.json
echo "end: $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
/tmp/p4l1/$V/flashtex-host layout -- x > $R/layout.json
echo done
