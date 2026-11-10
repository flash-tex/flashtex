#!/bin/zsh
# land.sh <pr> <sha-prefix> <gates|-> <modes csv|->
#   env: AFTER=<pr> (wait until merged before auto-merge)  WREF=<branch for the workflow file, default main>
#        EXTRA="-f k=v ..."  SWEEPS="id id id" (reuse dispatched runs)  RETARGET=main  PROFILES="balanced low-memory"
#        NOAUTO=1 (stop after sweeps, report only)  MAXSW=<n> (max active sweeps before dispatching, default 8)
# Chain: head unchanged -> "CI required" green on head -> sweeps (one per mode) green -> AFTER merged -> auto-merge.
# Never re-pushes. Reruns a cancelled CI run at most once. Logs to $S/land/<pr>.log; last line = status.
set -u
pr=$1 want=$2 gates=$3 modes=$4
S=${LAND_STATE:-$HOME/.flashtex-land}; mkdir -p $S/land
R=flash-tex/flashtex
L=$S/land/$pr.log; mkdir -p $S/land
log(){ print -r -- "$(date -u +%H:%MZ) $*" >> $L; }
deadline=$(( $(date +%s) + ${HOURS:-30}*3600 ))
pv(){ gh pr view $pr -R $R --json state,headRefOid,baseRefName --jq '"\(.state) \(.headRefOid) \(.baseRefName)"' 2>/dev/null; }
read st sha base <<<"$(pv)"
[[ $sha == ${want}* ]] || { log "HEAD-MOVED want=$want have=$sha state=$st; stop"; exit 2; }
log "start sha=$sha gates=$gates modes=$modes after=${AFTER:-} wref=${WREF:-main} extra=${EXTRA:-}"
# An empty reply is a transient gh/API failure (09:55Z 11-02 stopped 6 chains falsely): retry, then skip this check.
headok(){ local out=""; for i in 1 2 3 4 5; do out=$(pv); [[ -n $out ]] && break; sleep 30; done
  [[ -z $out ]] && { log "gh unreachable (5 tries); continuing"; return 0; }
  read st h base <<<"$out"; [[ $st == OPEN && $h == $sha ]] || { log "HEAD-MOVED/STATE $st $h; stop"; exit 2; }; }
tick(){ (( $(date +%s) > deadline )) && { log "TIMEOUT; stop"; exit 9; }; sleep ${POLL:-600}; }

# 1. CI required on the head
reran=0
while :; do
  headok
  # Judge by the pull_request run only: the push run on the same SHA skips the mac job and can be
  # green before the PR run's mac job finishes red (11-02: #2009, #2015, #2023 got auto-merge this way;
  # the merge queue does not run the mac job, so nothing caught it).
  prid=$(gh run list -R $R -c $sha -w CI -e pull_request -L 1 --json databaseId --jq '.[0].databaseId // empty' 2>/dev/null)
  c=""; [[ -n $prid ]] && c=$(gh api "repos/$R/actions/runs/$prid/jobs?per_page=100" --jq '[.jobs[]|select(.name=="CI required")]|last|"\(.status) \(.conclusion)"' 2>/dev/null)
  case $c in
    "completed success") log "CI green"; break;;
    "completed failure"|"completed timed_out") log "CI-RED ($c); stop"; exit 3;;
    *) run=$(gh run list -R $R -c $sha -w CI -e pull_request -L 1 --json databaseId,status,conclusion --jq '.[0]|"\(.databaseId) \(.status) \(.conclusion)"' 2>/dev/null)
       if [[ $run == *" completed cancelled" && $reran == 0 ]]; then gh run rerun ${run%% *} -R $R >/dev/null 2>&1 && log "reran cancelled CI ${run%% *}"; reran=1; fi;;
  esac
  tick
done

# 2. sweeps
ids=(${=SWEEPS:-})
if [[ $gates != - && ${#ids} == 0 ]]; then
  for prof in ${=PROFILES:-default}; do for m in ${(s:,:)modes}; do
    # Throttle: hosted ubuntu capacity (~12 jobs) is shared with the merge queue; 12 landers x 3 modes
    # flooded it on 11-02 (36 sweeps queued for hours). Dispatch only while < MAXSW sweeps are active.
    while :; do
      while ! mkdir $S/land/.lock 2>/dev/null; do sleep 3; done
      act=$(gh run list -R $R -w sweeps.yml -L 100 --json status --jq '[.[]|select(.status!="completed")]|length' 2>/dev/null)
      [[ -n $act ]] && (( act < ${MAXSW:-8} )) && break
      rmdir $S/land/.lock; headok; sleep 300
    done
    before=$(gh run list -R $R -w sweeps.yml -e workflow_dispatch -L 30 --json databaseId --jq '[.[].databaseId]|join(" ")')
    args=(-f ref=$sha -f gates=$gates)
    [[ $m != - ]] && args+=(-f boxmemo=$m)
    [[ $prof != default ]] && args+=(-f profile=$prof)
    gh workflow run sweeps.yml -R $R --ref ${WREF:-main} $args ${=EXTRA:-} >/dev/null 2>>$L
    new=""; for i in 1 2 3 4 5 6 7 8 9 10; do sleep 4
      new=$(gh run list -R $R -w sweeps.yml -e workflow_dispatch -L 30 --json databaseId --jq '[.[].databaseId]|join(" ")' | tr ' ' '\n' | grep -vxF -f <(print -r -- ${before// /$'\n'}) | head -1)
      [[ -n $new ]] && break; done
    rmdir $S/land/.lock
    [[ -n $new ]] || { log "DISPATCH-FAILED mode=$m prof=$prof; stop"; exit 6; }
    ids+=($new); log "sweeps dispatched $new boxmemo=$m profile=$prof"
  done; done
fi
if (( ${#ids} )); then
  while :; do
    headok; pend=0; res=()
    for id in $ids; do r=$(gh run view $id -R $R --json status,conclusion --jq '"\(.status) \(.conclusion)"' 2>/dev/null)
      case $r in "completed success") res+=("$id:ok");; "completed failure"|"completed timed_out") log "SWEEP-RED $id; stop"; exit 4;;
        "completed cancelled") log "SWEEP-CANCELLED $id; stop (needs a look)"; exit 5;; *) pend=1;; esac; done
    (( pend )) || { log "sweeps green: $res"; break; }
    tick
  done
fi
[[ -n ${NOAUTO:-} ]] && { log "DONE-NOAUTO (sweeps green, auto-merge left to Commander)"; exit 0; }

# 3. dependency
if [[ -n ${AFTER:-} ]]; then
  while :; do a=$(gh pr view $AFTER -R $R --json state --jq .state 2>/dev/null); [[ $a == MERGED ]] && { log "#$AFTER merged"; break; }
    [[ $a == CLOSED ]] && { log "#$AFTER CLOSED; stop"; exit 7; }; headok; tick; done
fi
if [[ -n ${RETARGET:-} ]]; then headok; [[ $base != $RETARGET ]] && gh pr edit $pr -R $R --base $RETARGET >/dev/null 2>>$L && log "retargeted to $RETARGET"; sleep 30; fi

# 4. auto-merge
headok
if gh pr merge $pr -R $R --auto --merge >/dev/null 2>>$L || gh pr merge $pr -R $R --auto >/dev/null 2>>$L; then
  log "DONE auto-merge on at $sha"
  if (( ${#ids} )); then
    body="Sweeps on \`${sha:0:9}\` passed: $(for id in $ids; do print -n "[$id](https://github.com/$R/actions/runs/$id) "; done)(gates \`$gates\`, boxmemo \`$modes\`${PROFILES:+, profiles \`$PROFILES\`}). CI required is green. Auto-merge is on.

daniel-parent (Commander), lander script

🤖 Generated with [Claude Code](https://claude.com/claude-code)"
    gh pr comment $pr -R $R -b "$body" >/dev/null 2>>$L
  fi
else log "AUTO-MERGE-FAILED; stop"; exit 8; fi
