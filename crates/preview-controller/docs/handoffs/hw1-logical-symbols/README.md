# Remaining HW1 logical symbols: mapping inventory

Read-only review. No compiler or runtime change. Exact source occurrences and reviewed source hashes are in source-pins.json.

| Command | HW1 count | Unicode | Existing math class / CM slot | Adobe Symbol encoding | Decision |
| --- | ---: | --- | --- | --- | --- |
| vee | 4 | U+2228 | Bin / Symbol0x5F | 0xDA logicalor | Existing recognition/export-table candidate is feasible after resource review |
| Rightarrow | 3 | U+21D2 | Rel / Symbol0x29 | 0xDE arrowdblright | Same bounded route; preserve capitalized command name |
| mid | 4 | U+2223 | Rel / Symbol0x6A | No U+2223 entry in checked table | Export decision required; do not silently substitute ASCII bar U+007C |
| setminus | 2 | U+2216 | Bin / Symbol0x6E | No U+2216 entry in checked table | Export decision required; do not substitute a text backslash |
| Longrightarrow | 1 | Long arrow semantics | No U+27F9 mapping found in checked CM table | No U+27F9 entry in checked table | Requires verified construction/resource support; do not alias to the short arrow |

Primary encoding source: https://www.unicode.org/Public/MAPPINGS/VENDORS/ADOBE/symbol.txt . It explicitly maps U+2228→DA and U+21D2→DE. The local StandardSymbolsPS.afm independently lists C218 logicalor/WX603 and C222 arrowdblright/WX987. Symbol slot0x7C maps U+007C vertical line; absence of U+2223 is not authority to conflate semantic scalars. CM slots are metric slots, not PDF encoding bytes or OpenType GIDs.

Recommended next scope is vee+Rightarrow through the existing bounded tables, after the renderer checks original cmap/resource support. Keep mid/setminus/Longrightarrow diagnostics explicit until a real export policy/implementation exists. Existing math-layout already distinguishes binary and relation spacing for the two supported scalars; no duplicate parser or fabricated spacing is needed. Compiler v1 source ranges and downstream whole-math source ranges retain their distinct scopes.
