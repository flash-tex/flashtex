Red-team verdict (2026-09-29), galley/repaginate idea:
- Geometric cut: REJECT (§1005 min-cost break, §1000 discard at top, §1001 topskip, §1017 glue set, §§1008-1010 inserts, output routine changes \vsize via \@colroom §987).
- Refined: MERIT, but unit = "segment" between consecutive outer build_page calls (§812,1026,1054,1076,1091,1094,1100,1103,1145,1200), NOT paragraph (OR fires after indent box before \everypar §1091, verified in initex; around display math §1145/§1200).
- Key: eqtb by value, input stack incl pending token lists, nest incl partial line list, page-builder state, pdfTeX obj counters, random seed, fonts, files/aux read. Replay: fresh node copies, eqtb/save changes, new cs/strings, fonts, ordered writes, log prints w/ relative line numbers.
- Unsound unless handled: page builder/OR mutate nodes in place (§1013/1017/1010) -> splice copies; memory-usage stats (§639, §1334, latex.ltx \tracingstats1) -> exclude or memo off; nondeterministic reads -> barriers.
- Hidden reads to track: \pagetotal/\pagegoal §421; \lastpenalty/\lastskip §424 (perpage); \pdflastximage (32/43 ids shift); \aftergroup tokens from OR (afterpage); absolute line numbers in tracing/warnings. \c@page is eqtb (cleveref \label reads it: 34/88 \write nodes change).
- Measured, 13 arXiv papers, +1-2 page edit: no text line changes beyond OR page/head lines; OR macro time median 0.54 ms/page = 13% of text time; \shipout ~78% of pdfTeX body time (PDF write+compress).
- Corpus (149 papers, static): floats 73%, graphics 58%, clearpage-type 24%, cleveref 15%, \thepage in body 0%, wrapfig/multicol/marginpar/\pageref 1-3%. Projected miss 5-9% (no cleveref), 25-30% (cleveref), ~9% pooled.
- Falsify: box-dump diff before/after page-shifting edits; per-segment hidden-read audit on port; memo vs from-scratch at every shipout. Kill if unexplained mismatch, median miss >30%, or OR+display-list shipout > half of text cost.
Artifacts: galley-redteam/tiny, galley-redteam/work.
