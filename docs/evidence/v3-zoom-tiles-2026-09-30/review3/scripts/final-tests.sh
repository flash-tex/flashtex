#!/bin/bash
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
rm -f $S/logs/a38-final-tests.done
bash $S/a38-sweep.sh focus4 4 'TileParityTests|EngineV3PageTilesTests|EngineV3ZoomTilesTests|EngineV3OpenTests|EngineV3InstanceTests|PreviewParityTests'
FLASHTEX_V3_TILE_ONLY=beamer-default,beamer-madrid,beamer-visuals,beamer-blocks-columns,min-tabular,thesis-chapter,lab-report,hw1,twelvept-plain \
  bash $S/a38-sweep.sh highd 12,16,20 TileParityTests/testEveryParityFixtureTilesExactly
FLASHTEX_V3_TILE_ONLY=beamer-default,beamer-madrid,beamer-visuals,beamer-blocks-columns FLASHTEX_V3_TILE_SURFACES=1 \
  bash $S/a38-sweep.sh highsurfd 16,20 TileParityTests/testEveryParityFixtureTilesExactly
echo ALLDONE > $S/logs/a38-final-tests.done
