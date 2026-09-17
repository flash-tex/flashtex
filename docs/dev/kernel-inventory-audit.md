# Kernel inventory audit (slice 1)

## Method

`crates/compiler/supported/canonical-latex.tsv` (header: `set\tkind\tname`, where `set` is the source package and `kind` is `command` or `environment`) was filtered to its 440 `kernel` rows (410 commands, 30 environments). For each name, `grep -rnwF -e 'NAME' crates/compiler/src/` (fixed-string, whole-word, recursive) was run from the repository root to test for any implementation reference, and `grep -rnwF -e 'NAME' crates/compiler/tests/` for integration-test coverage; inline `#[cfg(test)]` modules were separated from production code by treating every `#[cfg(test)] mod …` region (verified to run to end-of-file in each file) as test code. Short names that whole-word grep over-counts (single letters colliding with Rust identifiers, SI-unit symbols, roman numerals, column specifiers) and comment-only mentions were adjudicated by reading every match in context; matches confined to `vocabulary.rs` `KNOWN_UNIMPLEMENTED_*` lists, `//` comments, or unrelated tables (SI units, roman numerals, tabular alignment) were ruled *spurious*, not implementation. Cross-checks: `crates/compiler/src/supported.rs` (the inventory-as-data; `EXPANSION_COMMANDS` entries are executed by the `flashtex-tex-expansion` engine in `crates/tex-expansion/`, so they leave no string trace in `crates/compiler/src/`) and `crates/compiler/tests/supported_latex.rs::every_inventory_entry_compiles_without_an_unsupported_diagnostic` (run 2026-09-14: 1 passed, 0 failed — every inventory entry compiles without a "not supported" diagnostic in at least one context). Table 1A rows reproduce as empty output; Table 1B/2 rows give the discriminating evidence found.

## Revision (2026-09-15)

This audit was originally written at commit `06007472` (2026-09-14T08:39:56Z) and went stale within about a day: this repo merged roughly 247 commits from parallel agent lanes in the interim, several of which fixed or tested rows this audit had just flagged (`#320`/PR-labelled-"#319" for `\hrulefill`/`\dotfill`; `#428`/`#430` for all 41 Table 2 rows; separate slices for `\includeonly`, the kernel text accents, `\eqnarray`, and several `KNOWN_UNIMPLEMENTED_COMMANDS`-listed commands that turned out to already have real dispatch arms sitting ahead of that stale list in the match order).

**Every row was re-checked against `dbe3cac8` (current `main` as of this revision)** using the same method (Section "Method" above), not just the four rows an independent review had already caught. Net changes:

| | original | current (`dbe3cac8`) | current (this revision) | resolved | moved | still valid |
| --- | --- | --- | --- | --- | --- | --- |
| Table 1A (unimplemented) | 153 | 125 | 103 | 22 | 6 → Table 2 | 125 |
| Table 1B (falsely-flagged) | 61 | 44 | 74 | 16 | 1 → Table 2 | 44 |
| Table 2 (untested) | 41 | 7 | 6 | 41 | — | 0 carried over; 7 new |
| **unimplemented total (1A+1B)** | **214** | **169** | **177** | | | |

86 of the original 255 rows (about a third) changed status in roughly 24 hours. The four rows the independent review flagged (`\hrulefill`, `\dotfill`, `\includeonly` — all Table 1A → resolved; `\displaystyle` — Table 2 → resolved) are among the 79 fully resolved rows, not the whole story: 7 more Table 1A/1B rows are now implemented but still untested (moved into the new Table 2), and dozens of others besides the four named ones flipped.

**This document is a point-in-time snapshot, not a living one, and it cannot be hand-maintained at this repo's commit rate.** A grep-based check like this is entirely mechanical (see Method) and belongs in `crates/compiler/scripts/` as a script that runs the same greps against `canonical-latex.tsv` and prints current Table 1A/1B/2 membership on demand — not as a markdown file someone re-derives by hand after the fact. Recommendation: either (a) land a `kernel_inventory_audit.py`-style generator alongside `render_supported_latex.sh` and regenerate this doc from it, or (b) close this PR once its useful findings (the resolved rows already fixed, and the 7 new Table 2 gaps) are turned into tracked follow-up issues, since the document itself will be wrong again within days.

## Revision 2 (independent review, 2026-09-17)

The prior revision's own Method (engine primitive/prelude macro in `flashtex-tex-expansion` counts as implementation) was stated but not fully applied, and four canonical rows were checked but never actually entered into any table. Fixed:

- **9 Table 1A rows removed** (`closein`, `closeout`, `openin`, `openout`, `lineskip`, `lineskiplimit`, `topskip`, `pdfpageheight`, `pdfpagewidth`): the prior revision's own triage notes already found these have real `crates/tex-expansion/src/expand.rs` primitive/register entries — the same standard that correctly excludes `\day`/`\month`/`\year`/`\space` from both tables — but left them in Table 1A with a hedging "may be partially handled" note instead of actually removing them. Removed, applying the stated rule consistently.
- **`tabbing` removed from Table 1B**: it is genuinely implemented (`IMPLEMENTED_ENVIRONMENTS`, `vocabulary.rs:109`, and a real row/column-stop dispatch arm at `parser.rs:6056`/`6239`), not merely name-listed. The cited "explicit not-implemented" tests (`tests/recovery.rs:68-69`, `tests/diagnostic_codes.rs:65-66`) test `picture`, not `tabbing` — misattributed evidence, not a real finding.
- **`marginpar` removed from Table 2**: `parser.rs` already has a dedicated inline test, `marginpar_parses_to_a_margin_note_without_diagnostics` (`parser.rs:14277`), covering exactly this at the audited commit; the claim that both test greps came back empty was wrong.
- **`part`, `vbox`, `linespread` added to Table 1B**: all three are canonical `kernel` rows (`part`/`vbox` commands, confirmed in `canonical-latex.tsv`) that were checked during this revision but never entered into any table. All three have real `crates/compiler/src/` matches (so Table 1A's "zero matches" doesn't apply) that are not genuine implementation — see the rows below for the discriminating evidence.
- **`minipage` needs no row** [REVISED by Revision 3 below — the `environment == "minipage"` check turned out to be footnote-routing only, so `minipage` now has a Table 1B row]: it has a real dispatch check (`environment == "minipage"`, `parser.rs:10796`) with genuine footnote-numbering behavior, and a real test exercising it (`tests/footnote_counters.rs:134`) — implemented and tested, like `\day`/`\month`/`\year`/`\space`. (It is also stale-listed in `vocabulary.rs`'s `KNOWN_UNIMPLEMENTED_ENVIRONMENTS`, which is a pre-existing inconsistency in that list, not an audit gap — the same shape as the `hss`/hard-coded-list staleness the original revision already noted for other names.)

Net effect on the counts below: Table 1A 125 → 116 (-9), Table 1B 44 → 46 (-1 `tabbing`, +3 `part`/`vbox`/`linespread`), Table 2 7 → 6 (-1 `marginpar`). Unimplemented total (1A+1B): 169 → 162.

## Revision 3 (independent review round 2, 2026-09-17)

A second independent review (real `parser::parse` probes, checked against the audited tree) found nine canonical `kernel` rows missing from every table, and thirteen Table 1A rows sitting under a "ZERO matches" header they do not satisfy. Every row below was re-verified with this document's own Method (fixed-string, whole-word grep from the repo root, matches read in context) plus a probe of the compiler's actual diagnostic before entry:

- **8 commands added to Table 1B** (`\newline`, `\strut`, `\slash`, `\addvspace`, `\twocolumn`, `\stretch`, `\stop`, `\accent`): every `crates/compiler/src/` hit is spurious — `KNOWN_UNIMPLEMENTED_COMMANDS` list entries (`vocabulary.rs:46,48,56`), comments, the `twocolumn` *class option* (`parser.rs:4353`, not a command dispatch arm), or unrelated Rust identifiers and English prose (`stretch` glue fields, `stop` locals, the accent machinery; the engine's only lookalikes are a `verbatim` comment for `newline` and the internal `flashtex@stop` token for `stop`). Probes emit "`\NAME` is not supported by this compiler version" for all eight (`UnsupportedFeature` for the four list members, `UnknownCommand` for the other four).
- **`minipage` added to Table 1B as an environment**: this revises Revision 2's "needs no row" call above. `\begin{minipage}{3cm}b\end{minipage}` probes as "environment 'minipage' is not implemented; its body is typeset as plain text"; the only real behavior is `mpfootnote` `\alph` footnote numbering (`parser.rs:8958-8981`, routed via `environment == "minipage"`, `parser.rs:10796`) — the same "recognised, partial side behaviour, not fully implemented" shape that already places `linespread` in Table 1B, so the two are now consistent under one criterion.
- **13 rows moved Table 1A → Table 1B** (`DeclareOption`, `ExecuteOptions`, `ProcessOptions`, `arraycolsep`, and all nine `capital*` accents): each has only spurious matches — `natbib.rs`/`parser.rs` comments, `supported.rs` description strings, and the `text_builtins.rs:161-166` doc comment that lists the `capital*` names as deliberately absent (`TEXT_ACCENTS` holds only the eight lowercase names; `capitaltie`/`capitalnewtie` alias `\t`, itself unimplemented). None is genuinely implemented (no dispatch arm, no engine entry, no name-specific test), so this is a move, not a resolution.
- **Two citation fixes**: `\day`'s register entry is at `expand.rs:5819`, not `expand.rs:4998` (triage note corrected); `linespread`'s row now cites both `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:45`) and `KNOWN_ARITY_UNIMPLEMENTED` (`parser.rs:9863-9865`).

Net effect on the counts below: Table 1A 116 → 103 (-13 moved), Table 1B 46 → 68 (+8 new commands, +1 `minipage`, +13 moved), Table 2 unchanged at 6. Unimplemented total (1A+1B): 162 → 171.

## Revision 4 (independent review round 3, 2026-09-17)

A third review named 6 canonical rows still missing from every table with high confidence (`table`, `list`, `math`, `titlepage` environments; `\indent`, `\symbol` commands) and 8 more with medium confidence (page/layout lengths: `\columnwidth`, `\textwidth`, `\textheight`, `\headsep`, `\oddsidemargin`, `\paperwidth`, `\leftmargin`, `\linewidth`). Rather than repeat the review's own probes on faith, or attempt a full mechanical sweep of the remaining ~250 unlisted rows and claim completeness this document has not earned three times running, every one of the 14 named rows was probed directly against this tree with the actual context each command needs (preamble vs. body, `\setlength` target vs. value, `is_preamble_length`'s `\documentclass`-then-preamble requirement):

- **6 high-confidence rows added to Table 1B**, all reproducing exactly as described: `table`/`list`/`math`/`titlepage` each probe as `"environment '<name>' is not implemented; its body is typeset as plain text"`; `\indent` as `"\indent is recognised but paragraph indentation is not implemented"`; `\symbol{98}` as `"\symbol is not supported by this compiler version"`.
- **1 of the 8 medium-confidence lengths confirmed and added**: `\leftmargin`. `\setlength{\leftmargin}{1cm}` inside a real preamble (`\documentclass{article}...\begin{document}`) gives `"\setlength{\leftmargin} is recognised but not implemented here"` — the generic `\setlength` fallback (`parser.rs:4661-4669`), since `\leftmargin` has no `is_preamble_length`/`is_table_length` entry. The *keyval* `leftmargin=` inside `\setlist` (`parser.rs:4735-4738`) is a same-named, unrelated, working feature — not evidence for this row either way.
- **7 of the 8 medium-confidence lengths were false alarms**, all from testing the wrong context, exactly the trap this document's Method warns about:
  - `\headsep`, `\oddsidemargin`, `\paperwidth`, `\textheight`: the review's own probe (and this document's first attempt) tested `\setlength` outside any `\documentclass`/preamble. `is_preamble_length`'s dispatch (`parser.rs:1484-1496` lists all four) only fires `if in_preamble`; probed correctly (`\documentclass{article}\setlength{\X}{1cm}\begin{document}...`), all four give **zero diagnostics**. Genuinely implemented, like their already-Table-2-listed siblings (`paperheight`, `evensidemargin`, `headheight`, `footskip`, `marginparwidth`, `columnsep`) — no row needed, same as `\day`/`\month`.
  - `\columnwidth`, `\textwidth`, `\linewidth`: real dispatch as dimension *units* inside a length expression (`text_builtins.rs:608-610`, `DimenUnit::ColumnWidth`/`TextWidth`/`LineWidth`), with a passing unit test (`text_builtins.rs:866-867`, `v(".5\\linewidth")`). The review's probe used them as a bare `\setlength`/`\hspace` argument on their own (`\hspace{\columnwidth}`), which is a different, narrower gap in the general dimension parser, not evidence the *name* is unimplemented — the document's own Method (any implementation reference counts) already treats these as fine, the same way `\pagestyle`'s "one-context smoke coverage" caveat is noted for Table 2 rather than reclassifying it.

**On the remaining ~250 unlisted rows**: a mechanical first-pass sweep (bare `\name` in body text, `\begin{name}...\end{name}`) was run for completeness-checking purposes, but the overwhelming majority of its ~80 raw hits were the same shape of false alarm just demonstrated above — math-mode commands probed in text mode, tabular commands probed outside a `tabular`, register names probed with the wrong operation. Publishing that raw sweep as findings would very likely add a fourth round of incorrect rows instead of fixing the third round's. This document's own recommendation stands: a real generator needs per-command context templates (math vs. text, preamble vs. body, environment-scoped commands), not a single universal probe — the exact thing three consecutive review rounds have now separately rediscovered by hand.

Net effect on the counts below: Table 1A unchanged at 103, Table 1B 68 → 74 (+6: `list`, `math`, `titlepage`, `indent`, `symbol`, `leftmargin`), Table 2 unchanged at 6. Unimplemented total (1A+1B): 171 → 177. (`table` was first added here too, then withdrawn: the round-4 review compiled a full document through the real CLI and `table` with `\caption`/`\label`/`\ref` works — the render pipeline's float pre-pass, `crates/render-pipeline/src/floats.rs:151`, handles it before the compiler's not-implemented list applies. A bare `parser::parse` probe is the wrong context for it, the same trap as the length rows.)

## Table 1A — kernel names with ZERO matches in `crates/compiler/src/` (103)

| kind | name | reproducing grep (run from repo root) | result |
| ---- | ---- | ------------------------------------- | ------ |
| command | AtBeginDvi | `grep -rnwF -e 'AtBeginDvi' crates/compiler/src/` | (no output) |
| command | AtEndDvi | `grep -rnwF -e 'AtEndDvi' crates/compiler/src/` | (no output) |
| command | AtEndOfClass | `grep -rnwF -e 'AtEndOfClass' crates/compiler/src/` | (no output) |
| command | AtEndOfPackage | `grep -rnwF -e 'AtEndOfPackage' crates/compiler/src/` | (no output) |
| command | CheckCommand | `grep -rnwF -e 'CheckCommand' crates/compiler/src/` | (no output) |
| command | ClassError | `grep -rnwF -e 'ClassError' crates/compiler/src/` | (no output) |
| command | ClassInfo | `grep -rnwF -e 'ClassInfo' crates/compiler/src/` | (no output) |
| command | ClassWarning | `grep -rnwF -e 'ClassWarning' crates/compiler/src/` | (no output) |
| command | ClassWarningNoLine | `grep -rnwF -e 'ClassWarningNoLine' crates/compiler/src/` | (no output) |
| command | CurrentOption | `grep -rnwF -e 'CurrentOption' crates/compiler/src/` | (no output) |
| command | DeclareFontEncoding | `grep -rnwF -e 'DeclareFontEncoding' crates/compiler/src/` | (no output) |
| command | DeclareTextAccent | `grep -rnwF -e 'DeclareTextAccent' crates/compiler/src/` | (no output) |
| command | DeclareTextAccentDefault | `grep -rnwF -e 'DeclareTextAccentDefault' crates/compiler/src/` | (no output) |
| command | DeclareTextCommand | `grep -rnwF -e 'DeclareTextCommand' crates/compiler/src/` | (no output) |
| command | DeclareTextComposite | `grep -rnwF -e 'DeclareTextComposite' crates/compiler/src/` | (no output) |
| command | DeclareTextCompositeCommand | `grep -rnwF -e 'DeclareTextCompositeCommand' crates/compiler/src/` | (no output) |
| command | DeclareTextSymbol | `grep -rnwF -e 'DeclareTextSymbol' crates/compiler/src/` | (no output) |
| command | DeclareTextSymbolDefault | `grep -rnwF -e 'DeclareTextSymbolDefault' crates/compiler/src/` | (no output) |
| command | IfFileExists | `grep -rnwF -e 'IfFileExists' crates/compiler/src/` | (no output) |
| command | InputIfFileExists | `grep -rnwF -e 'InputIfFileExists' crates/compiler/src/` | (no output) |
| command | LastDeclaredEncoding | `grep -rnwF -e 'LastDeclaredEncoding' crates/compiler/src/` | (no output) |
| command | LoadClass | `grep -rnwF -e 'LoadClass' crates/compiler/src/` | (no output) |
| command | LoadClassWithOptions | `grep -rnwF -e 'LoadClassWithOptions' crates/compiler/src/` | (no output) |
| command | OptionNotUsed | `grep -rnwF -e 'OptionNotUsed' crates/compiler/src/` | (no output) |
| command | PackageError | `grep -rnwF -e 'PackageError' crates/compiler/src/` | (no output) |
| command | PackageInfo | `grep -rnwF -e 'PackageInfo' crates/compiler/src/` | (no output) |
| command | PackageWarning | `grep -rnwF -e 'PackageWarning' crates/compiler/src/` | (no output) |
| command | PackageWarningNoLine | `grep -rnwF -e 'PackageWarningNoLine' crates/compiler/src/` | (no output) |
| command | PassOptionsToClass | `grep -rnwF -e 'PassOptionsToClass' crates/compiler/src/` | (no output) |
| command | ProvideTextCommand | `grep -rnwF -e 'ProvideTextCommand' crates/compiler/src/` | (no output) |
| command | ProvideTextCommandDefault | `grep -rnwF -e 'ProvideTextCommandDefault' crates/compiler/src/` | (no output) |
| command | RequirePackageWithOptions | `grep -rnwF -e 'RequirePackageWithOptions' crates/compiler/src/` | (no output) |
| command | UseTextAccent | `grep -rnwF -e 'UseTextAccent' crates/compiler/src/` | (no output) |
| command | UseTextSymbol | `grep -rnwF -e 'UseTextSymbol' crates/compiler/src/` | (no output) |
| command | addtocontents | `grep -rnwF -e 'addtocontents' crates/compiler/src/` | (no output) |
| command | baselinestretch | `grep -rnwF -e 'baselinestretch' crates/compiler/src/` | (no output) |
| command | bigbreak | `grep -rnwF -e 'bigbreak' crates/compiler/src/` | (no output) |
| command | bottomfraction | `grep -rnwF -e 'bottomfraction' crates/compiler/src/` | (no output) |
| command | circle | `grep -rnwF -e 'circle' crates/compiler/src/` | (no output) |
| command | columnseprule | `grep -rnwF -e 'columnseprule' crates/compiler/src/` | (no output) |
| command | contentsline | `grep -rnwF -e 'contentsline' crates/compiler/src/` | (no output) |
| command | dashbox | `grep -rnwF -e 'dashbox' crates/compiler/src/` | (no output) |
| command | floatpagefraction | `grep -rnwF -e 'floatpagefraction' crates/compiler/src/` | (no output) |
| command | floatsep | `grep -rnwF -e 'floatsep' crates/compiler/src/` | (no output) |
| command | fontshape | `grep -rnwF -e 'fontshape' crates/compiler/src/` | (no output) |
| command | frenchspacing | `grep -rnwF -e 'frenchspacing' crates/compiler/src/` | (no output) |
| command | ignorespacesafterend | `grep -rnwF -e 'ignorespacesafterend' crates/compiler/src/` | (no output) |
| command | indexspace | `grep -rnwF -e 'indexspace' crates/compiler/src/` | (no output) |
| command | intextsep | `grep -rnwF -e 'intextsep' crates/compiler/src/` | (no output) |
| command | labelitemii | `grep -rnwF -e 'labelitemii' crates/compiler/src/` | (no output) |
| command | labelitemiii | `grep -rnwF -e 'labelitemiii' crates/compiler/src/` | (no output) |
| command | labelitemiv | `grep -rnwF -e 'labelitemiv' crates/compiler/src/` | (no output) |
| command | lefteqn | `grep -rnwF -e 'lefteqn' crates/compiler/src/` | (no output) |
| command | leftmarginii | `grep -rnwF -e 'leftmarginii' crates/compiler/src/` | (no output) |
| command | leftmarginiii | `grep -rnwF -e 'leftmarginiii' crates/compiler/src/` | (no output) |
| command | leftmarginv | `grep -rnwF -e 'leftmarginv' crates/compiler/src/` | (no output) |
| command | leftmarginvi | `grep -rnwF -e 'leftmarginvi' crates/compiler/src/` | (no output) |
| command | linethickness | `grep -rnwF -e 'linethickness' crates/compiler/src/` | (no output) |
| command | makeglossary | `grep -rnwF -e 'makeglossary' crates/compiler/src/` | (no output) |
| command | makeindex | `grep -rnwF -e 'makeindex' crates/compiler/src/` | (no output) |
| command | marginparpush | `grep -rnwF -e 'marginparpush' crates/compiler/src/` | (no output) |
| command | mathversion | `grep -rnwF -e 'mathversion' crates/compiler/src/` | (no output) |
| command | medbreak | `grep -rnwF -e 'medbreak' crates/compiler/src/` | (no output) |
| command | multiput | `grep -rnwF -e 'multiput' crates/compiler/src/` | (no output) |
| command | newfont | `grep -rnwF -e 'newfont' crates/compiler/src/` | (no output) |
| command | newsavebox | `grep -rnwF -e 'newsavebox' crates/compiler/src/` | (no output) |
| command | newtie | `grep -rnwF -e 'newtie' crates/compiler/src/` | (no output) |
| command | newwrite | `grep -rnwF -e 'newwrite' crates/compiler/src/` | (no output) |
| command | nocorrlist | `grep -rnwF -e 'nocorrlist' crates/compiler/src/` | (no output) |
| command | nofiles | `grep -rnwF -e 'nofiles' crates/compiler/src/` | (no output) |
| command | nonfrenchspacing | `grep -rnwF -e 'nonfrenchspacing' crates/compiler/src/` | (no output) |
| command | normalmarginpar | `grep -rnwF -e 'normalmarginpar' crates/compiler/src/` | (no output) |
| command | normalsfcodes | `grep -rnwF -e 'normalsfcodes' crates/compiler/src/` | (no output) |
| command | obeycr | `grep -rnwF -e 'obeycr' crates/compiler/src/` | (no output) |
| command | oldstylenums | `grep -rnwF -e 'oldstylenums' crates/compiler/src/` | (no output) |
| command | onecolumn | `grep -rnwF -e 'onecolumn' crates/compiler/src/` | (no output) |
| command | oval | `grep -rnwF -e 'oval' crates/compiler/src/` | (no output) |
| command | poptabs | `grep -rnwF -e 'poptabs' crates/compiler/src/` | (no output) |
| command | prevdepth | `grep -rnwF -e 'prevdepth' crates/compiler/src/` | (no output) |
| command | qbezier | `grep -rnwF -e 'qbezier' crates/compiler/src/` | (no output) |
| command | restorecr | `grep -rnwF -e 'restorecr' crates/compiler/src/` | (no output) |
| command | reversemarginpar | `grep -rnwF -e 'reversemarginpar' crates/compiler/src/` | (no output) |
| command | savebox | `grep -rnwF -e 'savebox' crates/compiler/src/` | (no output) |
| command | shortstack | `grep -rnwF -e 'shortstack' crates/compiler/src/` | (no output) |
| command | smallbreak | `grep -rnwF -e 'smallbreak' crates/compiler/src/` | (no output) |
| command | spacefactor | `grep -rnwF -e 'spacefactor' crates/compiler/src/` | (no output) |
| command | subitem | `grep -rnwF -e 'subitem' crates/compiler/src/` | (no output) |
| command | subsubitem | `grep -rnwF -e 'subsubitem' crates/compiler/src/` | (no output) |
| command | suppressfloats | `grep -rnwF -e 'suppressfloats' crates/compiler/src/` | (no output) |
| command | textfloatsep | `grep -rnwF -e 'textfloatsep' crates/compiler/src/` | (no output) |
| command | textfraction | `grep -rnwF -e 'textfraction' crates/compiler/src/` | (no output) |
| command | thicklines | `grep -rnwF -e 'thicklines' crates/compiler/src/` | (no output) |
| command | thinlines | `grep -rnwF -e 'thinlines' crates/compiler/src/` | (no output) |
| command | topfraction | `grep -rnwF -e 'topfraction' crates/compiler/src/` | (no output) |
| command | typein | `grep -rnwF -e 'typein' crates/compiler/src/` | (no output) |
| command | typeout | `grep -rnwF -e 'typeout' crates/compiler/src/` | (no output) |
| command | unboldmath | `grep -rnwF -e 'unboldmath' crates/compiler/src/` | (no output) |
| command | unitlength | `grep -rnwF -e 'unitlength' crates/compiler/src/` | (no output) |
| command | usebox | `grep -rnwF -e 'usebox' crates/compiler/src/` | (no output) |
| command | usecounter | `grep -rnwF -e 'usecounter' crates/compiler/src/` | (no output) |
| command | wlog | `grep -rnwF -e 'wlog' crates/compiler/src/` | (no output) |
| environment | filecontents* | `grep -rnwF -e 'filecontents*' crates/compiler/src/` | (no output) |
| environment | theindex | `grep -rnwF -e 'theindex' crates/compiler/src/` | (no output) |

## Table 1B — names with matches but NO genuine implementation (74)

The word grep below returns hits, but every hit was read in context and is spurious: `//` comments, `vocabulary.rs` `KNOWN_UNIMPLEMENTED_*` entries (the compiler's own not-implemented list — itself stale in places, see the revision note above: several names it lists are dispatched *before* the fallback that would ever consult it), SI-unit symbol tables (`siunitx.rs`), the `roman()` numeral table (`parser.rs:5775`), tabular column-alignment words, or unrelated Rust identifiers. The "genuine-impl check" column gives the discriminating grep (empty output) proving no dispatch arm, builtin-table entry, or inventory claim exists.

| kind | name | word grep (run from repo root) | genuine-impl check (empty output) and what the word hits are |
| ---- | ---- | ------------------------------ | ------------------------------------------------------------ |
| command | t | `grep -rnwF -e 't' crates/compiler/src/` | `grep -rn -e '"t"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. test-region lines; hits are SI `tonne`, `VerticalPosition::Top`, identifiers |
| command | put | `grep -rnwF -e 'put' crates/compiler/src/` | `grep -rn -e '"put"' crates/compiler/src/parser.rs crates/compiler/src/math.rs crates/compiler/src/expansion.rs` empty; hits are a `text_builtins.rs` logo closure variable and `//` comments |
| command | hss | `grep -rnwF -e 'hss' crates/compiler/src/` | `grep -rn -e '"hss"' crates/compiler/src/parser.rs crates/compiler/src/math.rs` empty; hits are a `//` comment (`parser/lists.rs:15`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:43`) |
| command | DeclareTextCommandDefault | `grep -rnwF -e 'DeclareTextCommandDefault' crates/compiler/src/` | `grep -rn -e 'DeclareTextCommandDefault' crates/compiler/src/ --include='*.rs' | grep -v '^.*://'` empty; sole hit is a `//` comment (`text_builtins.rs:129`) |
| command | addcontentsline | `grep -rnwF -e 'addcontentsline' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`layout.rs:88`) |
| command | bigskipamount | `grep -rnwF -e 'bigskipamount' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:887`) |
| command | boldmath | `grep -rnwF -e 'boldmath' crates/compiler/src/` | non-comment grep empty; hits are `//` comments (`text_builtins.rs:262,367`) |
| command | fontencoding | `grep -rnwF -e 'fontencoding' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`text_builtins.rs:150`) |
| command | fontseries | `grep -rnwF -e 'fontseries' crates/compiler/src/` | non-comment grep empty; hits are `//` comments (`parser.rs:3270,3339`) |
| command | labelenumi | `grep -rnwF -e 'labelenumi' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:17`) |
| command | labelenumii | `grep -rnwF -e 'labelenumii' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:17`) |
| command | labelenumiii | `grep -rnwF -e 'labelenumiii' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:18`) |
| command | labelenumiv | `grep -rnwF -e 'labelenumiv' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:18`) |
| command | labelitemi | `grep -rnwF -e 'labelitemi' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:19`) |
| command | leftmargini | `grep -rnwF -e 'leftmargini' crates/compiler/src/` | non-comment grep empty; hits are `///` doc comments (`layout.rs:46,48`) |
| command | leftmarginiv | `grep -rnwF -e 'leftmarginiv' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`layout.rs:48`) |
| command | makelabel | `grep -rnwF -e 'makelabel' crates/compiler/src/` | non-comment grep empty; hits are `//`/`//!` comments (`layout.rs:1006`, `parser/lists.rs:15,22`) |
| command | medskipamount | `grep -rnwF -e 'medskipamount' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:886`) |
| command | nobreakspace | `grep -rnwF -e 'nobreakspace' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:6014`) |
| command | numberline | `grep -rnwF -e 'numberline' crates/compiler/src/` | non-comment grep empty; hits are `///` doc comments (`layout.rs:75,1030`) |
| command | refname | `grep -rnwF -e 'refname' crates/compiler/src/` | non-comment grep empty; sole hit is a `//` comment (`parser.rs:2958`) |
| command | sbox | `grep -rnwF -e 'sbox' crates/compiler/src/` | non-comment grep empty; sole hit is a `//` comment (`text_builtins.rs:338`) |
| command | shipout | `grep -rnwF -e 'shipout' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`color.rs:865`); no engine entry either |
| command | smallskipamount | `grep -rnwF -e 'smallskipamount' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:886`) |
| command | vector | `grep -rnwF -e 'vector' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`math.rs:3334`, English word "vector") |
| command | vtop | `grep -rnwF -e 'vtop' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`tabular.rs:27`); no engine entry either |
| command | appendix | `grep -rnwF -e 'appendix' crates/compiler/src/` | non-comment grep empty exc. `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:38`); no dispatch, no engine entry |
| command | fbox | `grep -rnwF -e 'fbox' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); no dispatch, no engine entry |
| command | fontfamily | `grep -rnwF -e 'fontfamily' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:55`); no dispatch, no engine entry |
| command | fontsize | `grep -rnwF -e 'fontsize' crates/compiler/src/` | hits are a `//` comment (`text_builtins.rs:339`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:54`) |
| command | framebox | `grep -rnwF -e 'framebox' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); no dispatch, no engine entry |
| command | listoffigures | `grep -rnwF -e 'listoffigures' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:39`); no dispatch, no engine entry |
| command | listoftables | `grep -rnwF -e 'listoftables' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:40`); no dispatch, no engine entry |
| command | makebox | `grep -rnwF -e 'makebox' crates/compiler/src/` | hits are `///` comments (`tabular.rs:235,249`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`) |
| command | parbox | `grep -rnwF -e 'parbox' crates/compiler/src/` | hits are a `///` comment (`tabular.rs:329`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); `tests/` hits are incidental corpus text, not `\parbox` tests |
| command | raisebox | `grep -rnwF -e 'raisebox' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); no dispatch, no engine entry |
| command | RequirePackage | `grep -rnwF -e 'RequirePackage' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:62`) and `//`/`///` comments (`math.rs:488,547,553`); no parser arm, no engine entry — `\RequirePackage{…}` falls through to `unsupported()` |
| command | PassOptionsToPackage | `grep -rnwF -e 'PassOptionsToPackage' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:63`); no dispatch, no engine entry |
| command | selectfont | `grep -rnwF -e 'selectfont' crates/compiler/src/` | hits are a `//` comment (`text_builtins.rs:339`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:54`) |
| command | usefont | `grep -rnwF -e 'usefont' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:55`); no dispatch, no engine entry |
| environment | abstract | `grep -rnwF -e 'abstract' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:110`); no dispatch, no engine entry |
| environment | filecontents | `grep -rnwF -e 'filecontents' crates/compiler/src/` | only hit is the unimplemented-environment list (`vocabulary.rs:113`); no dispatch, no engine entry |
| environment | picture | `grep -rnwF -e 'picture' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:111`); no dispatch, no engine entry |
| command | part | `grep -rnwF -e 'part' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:40`), `xref.rs:61,153`'s counter-name-formatting table (applies to any counter named "part", not a `\part` dispatch arm), and unrelated uses of the English word "part" (`tabular.rs`, `parser/lists.rs`); no sectioning dispatch arm exists |
| command | vbox | `grep -rnwF -e 'vbox' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:45`) and `//`/`///` comments describing other constructs (`tabular.rs`, `text_builtins.rs`, `math.rs`) in terms of what real `\vbox` would do; no dispatch arm |
| command | linespread | `grep -rnwF -e 'linespread' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:45`) and the `KNOWN_ARITY_UNIMPLEMENTED` table (`parser.rs:9863-9865`): the argument is deliberately skipped and a diagnostic is emitted (tested, `parser.rs:13121-13132`), but line spacing itself never changes — recognised-and-diagnosed, not implemented |
| command | newline | `grep -rnwF -e 'newline' crates/compiler/src/` | hits are the unrelated `LayoutCursor::newline` line-breaking method (`layout.rs:863` and call sites), `//`/`///` comments about source newlines (`lexer.rs:527`, `parser.rs:2330-2934,6627-6639`, `supported.rs:343`), and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:46`); `grep -rn -e '"newline"' crates/compiler/src/parser.rs` empty — no dispatch, no engine entry (the engine's only hit is a `verbatim` comment, `expand.rs:3262`) |
| command | strut | `grep -rnwF -e 'strut' crates/compiler/src/` | hits are `//` comments using the English word for other constructs' spacing (`tabular.rs:33,71,346,1087`, `layout/footnotes.rs:7,100`, `parser.rs:5697,5751,9960`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:48`); no dispatch, no engine entry |
| command | slash | `grep -rnwF -e 'slash' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:56`) and `//` comments about the math negation slash, a zero-width overprint unrelated to the `\slash` command (`math.rs:4124,4131,4374,7598,7628`, `lm_math.rs:138`); no dispatch, no engine entry |
| command | addvspace | `grep -rnwF -e 'addvspace' crates/compiler/src/` | hits are `///`/`//` comments describing the layout engine's own skip-merging in `\addvspace`-style terms (`layout.rs:93,700,1541,1893`, `parser.rs:640,2679,3163`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:48`); no dispatch, no engine entry |
| command | twocolumn | `grep -rnwF -e 'twocolumn' crates/compiler/src/` | every hit is the *class option*, not the command: the `twocolumn_option` flag and its doc comment (`parser.rs:2384,4353`) and the multicol incompatibility warning (`parser.rs:5090-5094`); no `\twocolumn` command dispatch arm, no engine entry |
| command | stretch | `grep -rnwF -e 'stretch' crates/compiler/src/` | hits are unrelated Rust identifiers and English prose: the `stretch` glue field (`expansion.rs:469`), glue stretch/shrink locals (`parser.rs:7921-8223`), and layout/math comments; `grep -rn -e '"stretch"' crates/compiler/src/` empty — no list entry, no dispatch; the engine's `stretch` hits are the same glue-struct field (`registers.rs:17`), not a `\stretch` command |
| command | stop | `grep -rnwF -e 'stop' crates/compiler/src/` | hits are unrelated Rust locals (`let (stop, recovery)`, `parser.rs:7417-7432`), tab-stop logic, and English prose; `grep -rn -e '"stop"' crates/compiler/src/` empty — no list entry, no dispatch; the engine's `flashtex@stop` (`expand.rs:368`) is an internal synthetic token, not a `\stop` command |
| command | accent | `grep -rnwF -e 'accent' crates/compiler/src/` | hits are the accent *machinery* (the `Accent` enum, `text_accent()`, `TEXT_ACCENTS`) and comments about accent glyphs — none is a `\accent` primitive dispatch; `grep -rn -e '"accent"' crates/compiler/src/` empty, no engine entry; `parser.rs:8733` itself notes `\accent` is not implemented |
| environment | minipage | `grep -rnwF -e 'minipage' crates/compiler/src/` | recognised but not implemented: `environment == "minipage"` (`parser.rs:10796`, "the environment itself is not implemented: its body is set as running text") exists only to route `\footnote`/`\footnotetext` to `mpfootnote` `\alph` numbering (`parser.rs:8958-8981`); also stale-listed in `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:116`); the only test exercises footnote numbering (`tests/footnote_counters.rs:133-134`) — same recognised-partial shape as `linespread` above |
| command | DeclareOption | `grep -rnwF -e 'DeclareOption' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`natbib.rs:81`) — moved from Table 1A (Revision 3) |
| command | ExecuteOptions | `grep -rnwF -e 'ExecuteOptions' crates/compiler/src/` | non-comment grep empty; hits are `//` comments (`natbib.rs:153,166`) and a `///` doc comment (`parser.rs:1750`) — moved from Table 1A (Revision 3) |
| command | ProcessOptions | `grep -rnwF -e 'ProcessOptions' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`natbib.rs:82`) — moved from Table 1A (Revision 3) |
| command | arraycolsep | `grep -rnwF -e 'arraycolsep' crates/compiler/src/` | non-comment code grep empty; hits are description strings in the inventory data itself (`supported.rs:882,886`), not implementation — moved from Table 1A (Revision 3) |
| command | capitalacute | `grep -rnwF -e 'capitalacute' crates/compiler/src/` | sole hit is the `text_builtins.rs:161` doc comment listing the punctuation-named accents as deliberately absent; `TEXT_ACCENTS` holds only the eight lowercase names — moved from Table 1A (Revision 3) |
| command | capitalcircumflex | `grep -rnwF -e 'capitalcircumflex' crates/compiler/src/` | sole hit is the `text_builtins.rs:162` doc comment listing it as deliberately absent (see `capitalacute` above) — moved from Table 1A (Revision 3) |
| command | capitaldieresis | `grep -rnwF -e 'capitaldieresis' crates/compiler/src/` | sole hit is the `text_builtins.rs:162` doc comment listing it as deliberately absent (see `capitalacute` above) — moved from Table 1A (Revision 3) |
| command | capitaldotaccent | `grep -rnwF -e 'capitaldotaccent' crates/compiler/src/` | sole hit is the `text_builtins.rs:163` doc comment listing it as deliberately absent (see `capitalacute` above) — moved from Table 1A (Revision 3) |
| command | capitalgrave | `grep -rnwF -e 'capitalgrave' crates/compiler/src/` | sole hit is the `text_builtins.rs:161` doc comment listing it as deliberately absent (see `capitalacute` above) — moved from Table 1A (Revision 3) |
| command | capitalmacron | `grep -rnwF -e 'capitalmacron' crates/compiler/src/` | sole hit is the `text_builtins.rs:163` doc comment listing it as deliberately absent (see `capitalacute` above) — moved from Table 1A (Revision 3) |
| command | capitalnewtie | `grep -rnwF -e 'capitalnewtie' crates/compiler/src/` | sole hit is the `text_builtins.rs:166` doc comment: it aliases `\t`, itself unimplemented (see `t` above) — moved from Table 1A (Revision 3) |
| command | capitaltie | `grep -rnwF -e 'capitaltie' crates/compiler/src/` | sole hit is the `text_builtins.rs:166` doc comment: it aliases `\t`, itself unimplemented (see `t` above) — moved from Table 1A (Revision 3) |
| command | capitaltilde | `grep -rnwF -e 'capitaltilde' crates/compiler/src/` | sole hit is the `text_builtins.rs:162` doc comment listing it as deliberately absent (see `capitalacute` above) — moved from Table 1A (Revision 3) |
| environment | list | `grep -rnwF -e 'list' crates/compiler/src/` | `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:117`); probed `\begin{list}{}{}\item x\end{list}`: "environment 'list' is not implemented; its body is typeset as plain text" — added Revision 4 |
| environment | math | `grep -rnwF -e 'math' crates/compiler/src/` | `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:117`); probed `\begin{math}x\end{math}`: "environment 'math' is not implemented; its body is typeset as plain text" — added Revision 4 |
| environment | titlepage | `grep -rnwF -e 'titlepage' crates/compiler/src/` | `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:116`); the other hits are the `\documentclass[titlepage]` *option*, a different feature (`parser.rs:2377-2379,4348`); probed `\begin{titlepage}x\end{titlepage}`: "environment 'titlepage' is not implemented; its body is typeset as plain text" — added Revision 4 |
| command | indent | `grep -rnwF -e 'indent' crates/compiler/src/` | `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:48`) and a real dispatch arm, `"indent" if in_preamble` etc. (`parser.rs:3958`); probed `\indent x`: "\indent is recognised but paragraph indentation is not implemented" — added Revision 4 |
| command | symbol | `grep -rnwF -e 'symbol' crates/compiler/src/` | only non-spurious hit is siunitx's own unrelated `"symbol"` key (`siunitx.rs:344`); probed `\symbol{98}`: "\symbol is not supported by this compiler version" — added Revision 4 |
| command | leftmargin | `grep -rnwF -e 'leftmargin' crates/compiler/src/` | the register itself (as opposed to enumitem's unrelated same-named `\setlist` key, `parser.rs:4735-4738`, which does work) has no `is_preamble_length`/`is_table_length` entry, so `\setlength{\leftmargin}{...}` in the preamble falls to the generic fallback (`parser.rs:4661-4669`): "\setlength{\leftmargin} is recognised but not implemented here" — added Revision 4 |

## Table 2 — implemented but with ZERO name matches in tests/ or inline test modules (6)

"Implemented" means a genuine dispatch arm, builtin-table entry, or `flashtex-tex-expansion` engine primitive/prelude macro (site noted per row). Both test greps below return no output for every row: `grep -rnwF -e 'NAME' crates/compiler/tests/` and the inline check (matches of the same word grep inside `#[cfg(test)] mod …` regions of `src/`). Caveat: `tests/supported_latex.rs::every_inventory_entry_compiles_without_an_unsupported_diagnostic` compiles every *inventory-listed* name generically (verified passing), so inventory-listed rows below do have one-context smoke coverage — the same coverage `\pagestyle` had when its preamble bug shipped. Name-specific tests (especially second-context tests: preamble vs body, math vs text) are what is missing. All 41 rows from the original slice-1 Table 2 now have dedicated name-specific tests (`crates/compiler/tests/kernel_untested_a.rs` + `kernel_untested_b.rs`, #428/#430) and are removed from this table; the 6 rows below are newly-discovered gaps of the same shape, found while re-deriving this audit against current main (`marginpar` was also flagged here in error — Revision 2 above — and is removed).

| kind | name | implementation evidence | test greps (both empty) |
| ---- | ---- | ----------------------- | ----------------------- |
| command | paperheight | `PREAMBLE_LENGTHS` dimen list (`src/parser.rs:1485`) + preamble-assignment dispatch guard `is_preamble_length` (`src/parser.rs:1512`, arm at `src/parser.rs:3126`) | `grep -rnwF -e 'paperheight' crates/compiler/tests/` ; inline empty |
| command | evensidemargin | `PREAMBLE_LENGTHS` dimen list (`src/parser.rs:1489`) + same `is_preamble_length` dispatch as `paperheight` | `grep -rnwF -e 'evensidemargin' crates/compiler/tests/` ; inline empty |
| command | headheight | `PREAMBLE_LENGTHS` dimen list (`src/parser.rs:1491`) + same `is_preamble_length` dispatch as `paperheight` | `grep -rnwF -e 'headheight' crates/compiler/tests/` ; inline empty |
| command | footskip | `PREAMBLE_LENGTHS` dimen list (`src/parser.rs:1493`) + same `is_preamble_length` dispatch as `paperheight` | `grep -rnwF -e 'footskip' crates/compiler/tests/` ; inline empty |
| command | marginparwidth | `PREAMBLE_LENGTHS` dimen list (`src/parser.rs:1494`) + same `is_preamble_length` dispatch as `paperheight` | `grep -rnwF -e 'marginparwidth' crates/compiler/tests/` ; inline empty |
| command | columnsep | `PREAMBLE_LENGTHS` dimen list (`src/parser.rs:1496`) + same `is_preamble_length` dispatch as `paperheight` | `grep -rnwF -e 'columnsep' crates/compiler/tests/` ; inline empty |

## Triage notes for the supervisor

- `\day`, `\month`, `\year`, `\space` (kernel rows with `src/` matches) are engine-implemented (`\day` etc. are `Count` registers at `expand.rs:5819`, `\space` is prelude-defined), so they appear in neither table — same treatment as `closein`/`closeout`/`openin`/`openout`/`lineskip`/`lineskiplimit`/`topskip`/`pdfpageheight`/`pdfpagewidth` now get (Revision 2 above): confirmed engine-side, so excluded from Table 1A rather than left in it with a caveat.
- Highest-value follow-ups as of this revision: 8 of the original 9 Table 1B text accents (`\b \c \d \H \k \r \u \v`) are now implemented and tested (`TEXT_ACCENTS`, `crates/compiler/tests/text_accents.rs`); `\t` (the two-letter tie accent) is still unimplemented and is the last of the nine. `\RequirePackage` and `\PassOptionsToPackage` are still unimplemented (real documents use `\RequirePackage`; both still fall to `unsupported()`). The 6 Table 2 preamble-length registers above are the new highest-value test gap.
