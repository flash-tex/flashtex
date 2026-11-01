#!/bin/bash
cd /private/tmp/mb/c
clang -O2 -o rel rel.c || exit 1
for envv in "" MallocSpaceEfficient=1 MallocDeferredReclaim=0 MallocAggressiveMadvise=1 MallocLargeCache=0 MallocXzoneDeferSmall=0 MallocXzoneDeferLarge=0 MallocXzoneSegmentDeallocate=1; do
  echo "== ${envv:-default}"
  for s in "40000 1024" "4000 16384" "1000 65536" "250 262144" "60 1048576" "15 4194304"; do
    env $envv ./rel $s
  done
done
