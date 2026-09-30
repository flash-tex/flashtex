#!/bin/bash
# host.sh ENGINE DIR CMD [host opts] -- DOC : run flashtex-host CMD in DIR on DOC.tex
E=$1; DIR=$2; shift 2
cd $DIR
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/p4l5/$E/pdftex.pool FLASHTEX_FORMATS=/tmp/p4l5/fmt-$E
args=()
while [ "$1" != "--" ]; do args+=("$1"); shift; done
shift
DOC=$1; shift
exec ${PROFILER} /tmp/p4l5/$E/flashtex-host "${args[@]}" -- -fmt=pdflatex -interaction=batchmode "$@" $DOC.tex
