#!/usr/bin/env bash
# gate.sh: scripts/gate.sh quick in the rt2 tree with CI toolchain (rustup stable 1.98.1), fresh target dir
set -u
R=~/.cache/flashtex-actions-runner/.flashtex-actions-runner-3
export PATH=$R/cargo/bin:$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH RUSTUP_TOOLCHAIN=stable CARGO_BUILD_JOBS=8 CARGO_TARGET_DIR=$HOME/code/flashtex-rt2/target-gate
rustc --version
cd ~/code/flashtex-rt2
~/.local/bin/ftx-run 14400 scripts/gate.sh quick > ~/rt/rt2/gate.log 2>&1; echo "gate rc=$?"
sed -n "/gate.sh quick -- summary/,\$p" ~/rt/rt2/gate.log
