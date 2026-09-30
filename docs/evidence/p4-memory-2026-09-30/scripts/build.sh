#!/bin/bash
# build.sh: release and mem-stats builds of the lane's checkout on the NixOS PC (lane P4-MEMORY).
# The mem-stats build (the counting allocator, measurement only) goes to target/memstats.
cd ~/code/flashtex-p4mem || exit 1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export CARGO_BUILD_JOBS=${J:-8}
timeout 3600 cargo build --release -p flashtex-engine -p flashtex-display-list 2>&1 | tail -3
timeout 3600 cargo build --release -p flashtex-engine --features mem-stats --bin flashtex-host \
  --target-dir target/memstats 2>&1 | tail -3
echo BUILD-DONE
