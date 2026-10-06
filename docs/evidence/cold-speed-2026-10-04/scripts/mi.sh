#!/bin/bash
rm -rf /tmp/cs/mi; mkdir -p /tmp/cs/mi; cp /tmp/cs/w-pdftex/*.idx /tmp/cs/mi/; cd /tmp/cs/mi
grep -o 'runsystem([^)]*)' /tmp/cs/w-pdftex/infdesc.log
for f in infdesc vocabulary notation latex; do
  /usr/bin/time -l makeindex -q $f.idx 2>&1 | grep -E 'real|instructions retired|cycles elapsed'
done
