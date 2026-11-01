#!/usr/bin/env bash
# hostrun.sh DOC ENG [TAG] [env...]: one cold host compile (iserve) of DOC in a fresh dir; prints the report summary
set -u
DOC=$1; ENG=$2; TAG=${3:-x}; shift 3 || shift $#
E=/tmp/pr/ib/$ENG
W=/tmp/pr/w/$DOC-$ENG-$TAG
rm -rf $W; mkdir -p $W; cp -R /tmp/pr/src/$DOC/. $W/
case $DOC in
  deck) JOB=main ;; inf) JOB=infdesc ;; arx) JOB=p-trees ;; x2) JOB=infdesc-x2 ;; *) JOB=$DOC ;;
esac
rm -f $W/$JOB.pdf
cd $W
export TMPDIR=/tmp/pr/tmp FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=/tmp/pr/ib/fmt-$ENG TZ=UTC
echo compile | env "$@" FLASHTEX_PIN_CLOCK=1700000000.250000 /usr/bin/time -l $E/flashtex-host iserve -- -fmt=pdflatex -interaction=batchmode $JOB.tex > $W/host.json 2> $W/htime.txt
hi=$(grep 'instructions retired' $W/htime.txt | awk '{print $1}')
python3 - $W/host.json $hi $W/htime.txt <<'PY'
import json,sys,re
d=json.loads(open(sys.argv[1]).readline())
t=open(sys.argv[3]).read()
real=re.search(r'([\d.]+) real',t).group(1); rss=int(re.search(r'(\d+)\s+maximum resident',t).group(1))
print(f"instr={int(sys.argv[2])//10**6}M real={real}s rss={rss>>20}MB passes={d['passes']} modes={d['pass_modes']} pass_s={[round(x,2) for x in d['pass_s']]} total_s={d['total_s']:.2f} rerun_pages={d['rerun_pages']} pages={d['pages']} tests={d.get('tests')} l5={d.get('l5')}")
PY
md5 -q $W/$JOB.pdf
