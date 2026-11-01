#!/bin/bash
# get_next fast path: lockstep and etrip on its worktree
# run from a checkout of the get_next branch; the engine copied to /tmp/p6h/gn2
cd "$(dirname "$0")/../../../.." || exit 1
export TMPDIR=/tmp/p6g CARGO_BUILD_JOBS=4
FLASHTEX_POOL=/tmp/p6h/gn2/pdftex.pool nice python3 tools/lockstep/run.py --engine /tmp/p6h/gn2/flashtex-initex > /tmp/p6h/gn-lockstep.txt 2>&1
echo "lockstep exit $?" >> /tmp/p6h/gn-lockstep.txt
nice bash scripts/flashtex-etrip.sh > /tmp/p6h/gn-etrip.txt 2>&1
echo "etrip exit $?" >> /tmp/p6h/gn-etrip.txt
