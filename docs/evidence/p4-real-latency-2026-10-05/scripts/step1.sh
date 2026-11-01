# p4rl step 1: T7 (letter@middle) and keyrun (iso, i100, i150) on the same build, same window, interleaved
set -o pipefail
cd ~/p4rl/src
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export INCR_BENCH_DIR=$HOME/p4rl/ib
E=${ENG:-meas}
OUT=$HOME/p4rl/out/${ROUND:-s1}
mkdir -p $OUT
CG=/sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/flashtex.slice/cpu.stat
( while true; do echo "$(date +%s.%N) $(cut -d" " -f1 /proc/loadavg) $(grep -E "^(nr_periods|nr_throttled|throttled_usec|usage_usec)" $CG | tr "\n" " ")"; sleep 1; done ) > $OUT/cg.txt &
LP=$!
K="--kind,letter,--at,0.5"
for rep in 1 2; do
for d in ${DOCS:-full-1000 plain-1000}; do
  echo "$(date +%s.%N) BEGIN t7 $d $rep" >> $OUT/marks.txt
  python3 tools/incr-bench/t7.py --engine $E --docs $d --phases letter@middle --keys 20 --reopen 0 --out $OUT/t7-$d-$rep 2>&1 | tail -4
  echo "$(date +%s.%N) END t7 $d $rep" >> $OUT/marks.txt
  echo "$(date +%s.%N) BEGIN kr $d $rep" >> $OUT/marks.txt
  python3 docs/evidence/live-30ms-2026-10-04/scripts/keyrun.py $E $d $OUT/kr-$rep \
    iso=$K i100=$K,--interval-ms,100 i150=$K,--interval-ms,150 \
    --keys 40 --gap-ms 300 --timeout 1800 2>&1 | cut -c1-200
  echo "$(date +%s.%N) END kr $d $rep" >> $OUT/marks.txt
done
done
kill $LP
echo RUN-DONE
