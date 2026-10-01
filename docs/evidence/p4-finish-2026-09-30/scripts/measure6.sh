#!/bin/bash
# measure6.sh ENGINE: keep-warm's latency win and cost (owner decision 10A) through the socket, as
# the app types (page 1, no viewport, 100 keystrokes 300 ms apart, then 3 s of tail and 20 s idle):
# off, the default (2 s spin), and --keep-warm-pause 100 us, interleaved per document.
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
E=$1
mkdir -p /tmp/p4f/warm
wait_load() {
  for i in $(seq 1 120); do
    l=$(sysctl -n vm.loadavg | awk '{print int($2)}')
    [ "$l" -lt 8 ] && return
    sleep 10
  done
}
for doc in plain-10 full-10 plain-120 full-120 plain-1000 full-1000; do
  for cfg in "off=--keep-warm 0" "spin=" "pause100=--keep-warm-pause 100"; do
    wait_load
    $S/to.sh 1200 python3 $S/warm_cost.py $E $doc --keys 100 --idle-s 20 --tag "${cfg%%=*}" --host-args "${cfg#*=}"
  done
done
