#!/bin/bash
# The re-review's sweeps: all 83 fixtures at 3.25-8 px/pt, then path/form/image/fallback
# (drawn whole, kept raster) and stroked-rule (clipped) pages at 12, 16 and 20 px/pt.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
rm -f $S/logs/a38-sweeps2.done
bash $S/a38-sweep.sh full83b all TileParityTests/testEveryParityFixtureTilesExactly
FLASHTEX_V3_TILE_ONLY=beamer-default,beamer-madrid,beamer-visuals,beamer-blocks-columns,min-tabular,thesis-chapter,lab-report,hw1,twelvept-plain \
  bash $S/a38-sweep.sh highb 12,16,20 TileParityTests/testEveryParityFixtureTilesExactly
FLASHTEX_V3_TILE_ONLY=beamer-default,beamer-madrid,beamer-visuals,beamer-blocks-columns FLASHTEX_V3_TILE_SURFACES=1 \
  bash $S/a38-sweep.sh highsurf 16,20 TileParityTests/testEveryParityFixtureTilesExactly
echo ALLDONE > $S/logs/a38-sweeps2.done
