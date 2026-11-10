#!/bin/zsh
# fixmerge.sh <fix-pr> <parent-pr>: squash-merge a stacked fix into its (protected) parent once the
# base-branch policy allows it, rerunning the PR's failed CI jobs once; then start the parent's lander.
set -u
fix=$1 parent=$2 R=flash-tex/flashtex
S=${LAND_STATE:-$HOME/.flashtex-land}; mkdir -p $S/land
log(){ print -r -- "$(date -u +%H:%MZ) $fix->$parent $*"; }
end=$(( $(date +%s) + ${HOURS:-10}*3600 )); reran=0
while (( $(date +%s) < end )); do
  st=$(gh pr view $fix -R $R --json state,headRefOid --jq '"\(.state) \(.headRefOid)"' 2>/dev/null) || { sleep 120; continue; }
  [[ $st == MERGED* ]] && break
  [[ $st == OPEN* ]] || { log "state $st; stop"; exit 2; }
  sha=${st#OPEN }
  run=$(gh run list -R $R -c $sha -w CI -e pull_request -L 1 --json databaseId,status,conclusion --jq '.[0]|"\(.databaseId) \(.status) \(.conclusion)"' 2>/dev/null)
  case $run in
    *" completed success") gh pr merge $fix -R $R --squash >/dev/null 2>&1 && { log "merged"; break; } || log "green but merge refused; retry";;
    *" completed failure"|*" completed cancelled")
      if (( reran == 0 )); then gh run rerun ${run%% *} -R $R --failed >/dev/null 2>&1 && { log "reran failed jobs of ${run%% *}"; reran=1; }
      else log "CI red again (${run%% *}); stop"; exit 3; fi;;
  esac
  sleep ${POLL:-480}
done
(( $(date +%s) >= end )) && { log "TIMEOUT"; exit 9; }
sleep 60
head=$(gh pr view $parent -R $R --json headRefOid --jq .headRefOid)
log "parent head ${head:0:9}; starting lander"
$S/land.sh $parent ${head:0:9} - -
log "lander exit $?: $(tail -1 $S/land/$parent.log)"
