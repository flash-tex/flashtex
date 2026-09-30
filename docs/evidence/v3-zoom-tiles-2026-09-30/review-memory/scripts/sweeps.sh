#!/bin/bash
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
bash $S/a38-sweep.sh full83 all TileParityTests/testEveryParityFixtureTilesExactly
FLASHTEX_V3_TILE_ONLY=min-tabular,thesis-chapter,listings-manual,lab-report,hw1,beamer-madrid,beamer-visuals,twelvept-plain,min-hrulefill \
  bash $S/a38-sweep.sh high 12,16,20 TileParityTests/testEveryParityFixtureTilesExactly
echo ALLDONE > $S/logs/a38-sweeps.done
