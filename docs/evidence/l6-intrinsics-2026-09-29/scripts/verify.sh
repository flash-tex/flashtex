#!/bin/bash
# verify.sh ENGINE TEXFILE [MODE] [VAR=VALUE...]: compile TEXFILE (a path) twice with the
# engine in /tmp/l6/ENGINE and FLASHTEX_INTRINSICS=MODE (default verify: both paths for
# every replayable call, the full state change diffed), in a scratch directory, and print
# one line: calls, replays, verified calls, differences (and the first details, if any).
set -e
E=$1; F=$2; MODE=${3:-verify}; shift 3 || shift $#
D=$(mktemp -d /tmp/l6/verify.XXXXXX)
cp "$F" "$D/"; B=$(basename "$F" .tex)
for x in "$(dirname "$F")"/*.bib "$(dirname "$F")"/*.sty "$(dirname "$F")"/*.cls "$(dirname "$F")"/*.png "$(dirname "$F")"/*.pdf "$(dirname "$F")"/*.eps; do
  [ -e "$x" ] && [ "$(basename "$x")" != "$B.pdf" ] && cp "$x" "$D/" 2>/dev/null || true
done
cd "$D"
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC
export FLASHTEX_POOL=/tmp/l6/$E/pdftex.pool FLASHTEX_FORMATS=/tmp/l6/fmt-$E
for kv in "$@"; do export "$kv"; done
FLASHTEX_INTRINSICS=$MODE /tmp/l6/$E/flashtex-initex -fmt=pdflatex -interaction=batchmode "$B.tex" >/dev/null 2>&1 || true
FLASHTEX_INTRINSICS=$MODE FLASHTEX_INTRINSICS_STATS=$D/stats.txt /tmp/l6/$E/flashtex-initex -fmt=pdflatex -interaction=batchmode "$B.tex" >/dev/null 2>&1 || true
c=$(awk '/^    calls:/{gsub(",","");print $2}' stats.txt)
r=$(awk '/^    replays:/{gsub(",","");print $2}' stats.txt)
v=$(awk '/^    verified:/{gsub(",","");print $2}' stats.txt)
x=$(awk '/^    verify_differences:/{gsub(",","");print $2}' stats.txt)
echo "$B mode=$MODE calls=$c replays=$r verified=$v differences=$x"
if [ "${x:-0}" != 0 ]; then grep -A2 verify_details stats.txt | tail -2 | cut -c1-400; fi
rm -rf "$D"
