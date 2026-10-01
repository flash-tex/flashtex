#!/bin/bash
E=/Users/dqi26/flashtex-wt/d1/$1
export PATH=/Users/dqi26/flashtex-wt/d1-bin:$PATH SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
python3 /Users/dqi26/flashtex-wt/d1/d1pair.py --host $E/flashtex-host --formats $E/fmt --pool $E/pdftex.pool \
  --texbin /Users/dqi26/flashtex-wt/d1-bin --env FLASHTEX_TEXLIVE_BIN=/Users/dqi26/flashtex-wt/d1-bin \
  --doc $2 --kind $3 --out /Users/dqi26/flashtex-wt/s/pair-$1 2>&1 | cut -c1-300 | tail -25
