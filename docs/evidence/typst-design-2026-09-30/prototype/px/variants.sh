#!/bin/bash
cd "$(dirname "$0")"
J=${1:-p2.json}; P=${2:-p2.pdf}; PG=${3:-2}; MB=${4:-841.8898}
for s in 1 2 3; do
  echo "scale $s default";      MBH=$MB ./pxdiff $J $P $PG $s
  echo "scale $s SUBPOS=0";     SUBPOS=0 MBH=$MB ./pxdiff $J $P $PG $s
  echo "scale $s SUBQ=0";       SUBQ=0 MBH=$MB ./pxdiff $J $P $PG $s
  echo "scale $s SUBPOS=0 SUBQ=0"; SUBPOS=0 SUBQ=0 MBH=$MB ./pxdiff $J $P $PG $s
done
