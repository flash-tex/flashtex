#!/bin/bash
# T0 latency and memory rows, run one after another (never in parallel).
cd "$(dirname "$0")/proto-copy"
export TBENCH="$PWD/../target/release/tbench"
R=../raw
mkdir -p $R
nice -n 10 bash ./run_seeded.sh $R/validate.jsonl 16 validate d10 d100 d300
nice -n 10 bash ./run_seeded.sh $R/seeded.jsonl 40 time d10 d100 d300 c300 d1000
nice -n 10 bash ./run_bench.sh $R/standard.jsonl 40 10 d10 d100 d300 c300
nice -n 10 bash ./run_bench.sh $R/standard-d1000.jsonl 10 10 d1000
nice -n 10 bash ./run_mem.sh $R/mem.jsonl d300 300 10 seeded
nice -n 10 bash ./run_mem.sh $R/mem-noevict.jsonl d300 100 0 seeded
echo ALLDONE
