#!/usr/bin/env bash
# sweep.sh "ENGINE..." DOC... : the retention budget's cost (NixOS PC, lane P4-MEMORY). For each
# budget in BUDGETS (MB; default 1024 512 256 128), ab.sh's rounds with the host's --budget, so
# checkpoints are dropped (thin) and restarts move further from the edit. Tags: sw<MB>.
cd ~/code/flashtex-p4mem || exit 1
ENGS=$1; shift
for mb in ${BUDGETS:-1024 512 256 128}; do
  MEM_HOSTARGS="--budget $((mb * 1048576))" ROUNDS=${ROUNDS:-1} bash ~/p4mem-ab.sh "sw$mb" "$ENGS" "$@"
done
echo SWEEP-DONE
