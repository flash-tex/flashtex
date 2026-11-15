#!/usr/bin/env bash
# Self-test for scripts/agents/slot.sh. Runs locally against a temp SLOT_ROOT; no
# ssh, no shared state. Run it with the oldest bash you care about, e.g.
#   /bin/bash scripts/agents/slot-selftest.sh        # macOS bash 3.2
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
SLOT="$HERE/slot.sh"
ROOT=$(mktemp -d "${TMPDIR:-/tmp}/slot-selftest.XXXXXX")
export SLOT_ROOT=$ROOT SLOT_POLL=0.2 SLOT_PC_HOST=local
HOST=$(uname -n)
fails=0; n=0
ok()  { n=$((n + 1)); echo "ok $n - $1"; }
bad() { n=$((n + 1)); fails=$((fails + 1)); echo "not ok $n - $1"; }
check() { if eval "$2"; then ok "$1"; else bad "$1"; fi; }
cleanup() { jobs -p | xargs kill 2>/dev/null; rm -rf "$ROOT"; }
trap cleanup EXIT
fake_owner() { # fake_owner <slotdir> <pid> <host> <start> <max>
  mkdir -p "$1"
  printf 'lane=fake\npid=%s\nhost=%s\nstart=%s\nstart_utc=x\nmax_minutes=%s\npriority=50\n' "$2" "$3" "$4" "$5" > "$1/owner"
}
dead_pid() { sh -c 'exit 0' & p=$!; wait $p; echo $p; }
now=$(date +%s)

# 1. acquire fills the pool in order and records the owner.
sleep 300 & HOLDER=$!
s1=$("$SLOT" acquire mac-build lane-a --pid $HOLDER --max-minutes 5)
s2=$("$SLOT" acquire mac-build lane-b --pid $HOLDER)
s3=$("$SLOT" acquire mac-build lane-c --pid $HOLDER --priority 7)
check "three mac-build slots" '[ "$s1 $s2 $s3" = "mac-build.lock mac-build.lock2 mac-build.lock3" ]'
check "owner file has lane, pid, host, start, max" \
  'grep -q "^lane=lane-a$" $ROOT/mac-build.lock/owner && grep -q "^pid=$HOLDER$" $ROOT/mac-build.lock/owner && grep -q "^host=$HOST$" $ROOT/mac-build.lock/owner && grep -q "^start=[0-9]" $ROOT/mac-build.lock/owner && grep -q "^max_minutes=5$" $ROOT/mac-build.lock/owner'

# 2. a full pool times out a waiter with --wait-minutes 0, and leaves no ticket.
"$SLOT" acquire mac-build lane-d --pid $HOLDER --wait-minutes 0 >/dev/null 2>&1; rc=$?
check "full pool: waiter gives up (exit 3)" '[ $rc = 3 ]'
check "gave-up waiter leaves no queue ticket" '[ -z "$(ls $ROOT/mac-build.queue)" ]'

# 3. release only removes the caller's own slot.
"$SLOT" release mac-build lane-x >/dev/null 2>&1; rc=$?
check "release by a non-owner fails" '[ $rc != 0 ] && [ -d $ROOT/mac-build.lock ] && [ -d $ROOT/mac-build.lock2 ]'
"$SLOT" release mac-build lane-b >/dev/null
check "release by the owner frees only its slot" '[ ! -d $ROOT/mac-build.lock2 ] && [ -d $ROOT/mac-build.lock ] && [ -d $ROOT/mac-build.lock3 ]'

# 4. a live local pid is never broken, even far past max+10.
fake_owner $ROOT/mac-build.lock2 $HOLDER "$HOST" $((now - 86400)) 1
"$SLOT" acquire mac-build lane-e --pid $HOLDER --wait-minutes 0 >/dev/null 2>&1; rc=$?
check "live pid past max+10 is not broken" '[ $rc = 3 ] && grep -q "^lane=fake$" $ROOT/mac-build.lock2/owner'

# 5. a dead local pid is broken at once (even inside max), and the break is logged.
DEAD=$(dead_pid)
fake_owner $ROOT/mac-build.lock2 $DEAD "$HOST" $now 60
s=$("$SLOT" acquire mac-build lane-f --pid $HOLDER --wait-minutes 0 2>/dev/null)
check "dead pid slot is broken and re-acquired" '[ "$s" = mac-build.lock2 ] && grep -q "^lane=lane-f$" $ROOT/mac-build.lock2/owner'
check "break is logged" 'grep -q "broke mac-build.lock2 by lane-f" $ROOT/slot-breaks.log'
"$SLOT" release mac-build lane-f >/dev/null

# 6. other-host pid: kept within max+10, broken past it.
fake_owner $ROOT/mac-build.lock2 12345 other-host $((now - 14 * 60)) 5
"$SLOT" acquire mac-build lane-g --pid $HOLDER --wait-minutes 0 >/dev/null 2>&1; rc=$?
check "other-host slot within max+10 is kept" '[ $rc = 3 ] && grep -q "^host=other-host$" $ROOT/mac-build.lock2/owner'
fake_owner $ROOT/mac-build.lock2 12345 other-host $((now - 16 * 60)) 5
s=$("$SLOT" acquire mac-build lane-g --pid $HOLDER --wait-minutes 0 2>/dev/null)
check "other-host slot past max+10 is broken" '[ "$s" = mac-build.lock2 ]'
"$SLOT" release mac-build lane-g >/dev/null

# 7. a hand-made lock with no owner file is kept while young.
mkdir $ROOT/mac-build.lock2
"$SLOT" acquire mac-build lane-h --pid $HOLDER --wait-minutes 0 >/dev/null 2>&1; rc=$?
check "young owner-less lock is kept" '[ $rc = 3 ] && [ -d $ROOT/mac-build.lock2 ]'
rmdir $ROOT/mac-build.lock2

# 8. priority queue: with the pool full, a later high-priority waiter beats an
#    earlier low-priority one; equal priority goes first-come first-served.
s=$("$SLOT" acquire mac-build lane-fill --pid $HOLDER)
"$SLOT" acquire mac-build low1 --pid $HOLDER --priority 10 > $ROOT/low1.out 2>/dev/null & L1=$!
sleep 1.2
"$SLOT" acquire mac-build low2 --pid $HOLDER --priority 10 > $ROOT/low2.out 2>/dev/null & L2=$!
sleep 1.2
"$SLOT" acquire mac-build high --pid $HOLDER --priority 90 > $ROOT/high.out 2>/dev/null & H=$!
sleep 1
check "three waiters queued, high first" '[ "$(ls $ROOT/mac-build.queue | wc -l | tr -d " ")" = 3 ] && [ "$(SLOT_PC_HOST=local "$SLOT" status mac-build | sed -n "/queue/{n;p;}" | awk "{print \$1}")" = high ]'
"$SLOT" release mac-build lane-fill >/dev/null
wait $H
check "high-priority waiter got the freed slot" '[ "$(cat $ROOT/high.out)" = mac-build.lock2 ] && kill -0 $L1 && kill -0 $L2'
"$SLOT" release mac-build high >/dev/null
wait $L1
check "earliest low-priority waiter is next" '[ "$(cat $ROOT/low1.out)" = mac-build.lock2 ] && kill -0 $L2'
"$SLOT" release mac-build low1 >/dev/null
wait $L2
check "last waiter gets it, queue empty" '[ "$(cat $ROOT/low2.out)" = mac-build.lock2 ] && [ -z "$(ls $ROOT/mac-build.queue)" ]'
"$SLOT" release mac-build low2 >/dev/null

# 9. a waiter that dies leaves a ticket that is skipped.
s=$("$SLOT" acquire mac-build lane-fill --pid $HOLDER)
sleep 300 & GHOST=$!
"$SLOT" acquire mac-build ghost --pid $GHOST --priority 99 >/dev/null 2>&1 & G=$!
sleep 0.8
kill $G $GHOST 2>/dev/null; wait $G $GHOST 2>/dev/null
# simulate a SIGKILLed waiter: plant a ticket for a dead pid at top priority
printf 'lane=zombie\npid=%s\nhost=%s\npriority=99\nsince=x\n' "$(dead_pid)" "$HOST" > "$ROOT/mac-build.queue/00000-0000000001-$HOST-1-1-zombie"
"$SLOT" release mac-build lane-fill >/dev/null
s=$("$SLOT" acquire mac-build lane-i --pid $HOLDER --wait-minutes 1 2>/dev/null)
check "dead waiter's ticket is skipped and removed" '[ "$s" = mac-build.lock2 ] && [ -z "$(ls $ROOT/mac-build.queue)" ]'
"$SLOT" release mac-build lane-i >/dev/null

# 10. run holds a slot for the command and releases it after, keeping the exit code.
"$SLOT" run pc lane-run -- sh -c 'cat '"$ROOT"'/pc-slot1/owner > '"$ROOT"'/run-owner; exit 7' 2>/dev/null; rc=$?
check "run: holds pc-slot1 with its own pid, returns the command's status" \
  '[ $rc = 7 ] && grep -q "^lane=lane-run$" $ROOT/run-owner && [ ! -d $ROOT/pc-slot1 ]'

# 11. pc pool has 4 slots; status works.
for i in 1 2 3 4; do "$SLOT" acquire pc pc$i --pid $HOLDER >/dev/null; done
"$SLOT" acquire pc pc5 --pid $HOLDER --wait-minutes 0 >/dev/null 2>&1; rc=$?
check "pc pool holds exactly four" '[ $rc = 3 ] && [ -d $ROOT/pc-slot4 ]'
check "status lists both pools" '"$SLOT" status | grep -q "pc-slot4 *lane=pc4" && "$SLOT" status | grep -q "mac-build.lock *lane=lane-a"'

echo "1..$n"
[ $fails = 0 ] && echo "slot-selftest: all $n passed" || echo "slot-selftest: $fails of $n FAILED"
[ $fails = 0 ]
