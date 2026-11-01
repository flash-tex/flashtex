# flashtex-page-builder: contract (KC-106)

This crate implements TeX's vertical-mode machinery from *TeX: The Program*
(tex.web), with section numbers cited in the code, plus a model of LaTeX's
single-column output routine. It has no dependencies and uses integer scaled
points throughout.

| Module | tex.web / LaTeX source | Contents |
|---|---|---|
| `scaled` | §§102–108 | `badness`, `x_over_n`, `print_scaled`, TeX decimal parsing |
| `node` | §§133–159, §1239 | vlist nodes, glue orders, `\advance` on skips |
| `pack` | §§668–678, §§629–637 | `vpackage`, `vlist_out` positions (cumulative glue rounding) |
| `split` | §§967–979 | `prune_page_top`, `vert_break`, `vsplit` |
| `page` | §§980–1028, §1054 | page builder, insertions, `fire_up`, output hand-off, `\end` |
| `vlist` | §679, §1056, §§888–890, §§1199–1206 | interline glue, rules, paragraph penalties, displays, `\addvspace`, `\addpenalty` |
| `latex` | latex.ltx 2026 (ltspace, ltoutput) | `\pagebreak[n]`, `\nopagebreak[n]`, `\newpage`, `\clearpage`, `\enlargethispage(*)`, `\@makecol` / `\@opcol` / `\@doclearpage` |

## Status: verified vs believed

**Verified against pdflatex (TeX Live 2026).** 61 multi-page fixtures cover 338
pages and 11,718 column lines. All 61 are identical to pdflatex:

- every `\tracingpages` line (`%% goal height`, `% t= g= b= p= c=#`, `% split`)
  matches byte for byte;
- every `\outputpenalty` matches;
- every `\box255` matches node by node, with sizes in sp;
- every shipped line baseline matches, with a maximum deviation of 0.00021pt
  (target 0.1pt).

The fixtures cover:

- widows and clubs, including penalty variants;
- headings near the page bottom;
- displays at page breaks (`\[`, `equation`, amsmath);
- `\raggedbottom` vs `\flushbottom`, including twoside;
- `\topskip` with tall first lines;
- `\parskip` stretch, including fil;
- `\enlargethispage` and `\enlargethispage*`, positive and negative;
- `\pagebreak[0-4]`, `\nopagebreak[0-4]`, `\newpage`, `\clearpage`, `\samepage`;
- lists;
- `\vspace`, `\vspace*`, `\vfill`, `\linespread`, `\maxdepth`;
- footnotes, including footnotes split across pages (`% split253` traces in
  footnotes-03).

The oracle tests the page builder in isolation. Its input is the exact main
vertical list pdflatex produced, captured from a second run (see
`oracle/gen.py`), so a failure is a page-builder bug and never a
paragraph-layout difference. `pdftotext -bbox-layout` gives an independent
cross-check: text-line baselines agree with the computed ones to within 0.001pt.

**Believed (unit-tested from tex.web, not oracle-checked).**

- `VListBuilder`: `append_to_vlist` interline glue, the §890 penalty sums,
  display glue and penalties, and LaTeX `\addvspace`/`\addpenalty` glue
  arithmetic. The oracle streams already contain TeX's glue, so these
  functions were not exercised by it.
- `vsplit` remainders.
- Marks.
- `\end` handling (`its_all_over`).
- `\holdinginserts` > 0 during real output.
- Insertion classes with `\count` other than 1000.

**Not implemented.**

- LaTeX float and marginpar special outputs (`\outputpenalty` -10002 and
  -10003). `LatexOutput` counts these in `unsupported` and returns the page
  material.
- Two-column output (`\@outputdblcol`).
- The \vsplit mark tokens themselves: only mark ids are tracked.

## API sketch

```rust
use flashtex_page_builder::{latex::LatexOutput, page::*, vlist::*, node::*, scaled::pt};

let mut v = VListBuilder::new(InterlineParams { baseline_skip, line_skip, line_skip_limit });
v.addpenalty(-300, nobreak, maxdepth);            // \@secpenalty before a heading
v.addvspace(GlueSpec::new(pt(15.07), pt(4.3), pt(0.86)));
v.paragraph(lines, LinePenalties { inter_line: 10000, club: 150, final_widow: 150, broken: 100 });
v.penalty(10000);                                 // heading \nobreak
v.display(Display { hbox, pre_penalty: 10000, post_penalty: 0, above, below, short });

let mut pb = PageBuilder::new(PageParams { vsize, max_depth, top_skip, ..Default::default() });
pb.classes.insert(footins, InsertClass { count: 1000, dimen: pt(8.0 * 72.27), skip: skip_footins, contents: None });
let mut out = LatexOutput::new(colht, maxdepth, raggedbottom, kludgeins);
pb.contribute(v.drain());          // call build_page/run as often as TeX would (after each paragraph)
pb.run(&mut out);
pb.end(hsize, &mut out);
for col in &out.columns { for (bx, baseline) in &col.lines { /* bx.id = caller payload */ } }
```

Custom output routines, such as floats, implement `OutputRoutine`. They receive
a `FiredPage` (`box255`, `output_penalty`, `best_size`, `page_so_far`, held
insert count, marks) and return the vertical list to push back.

## Adoption

The adoption proposal targeted the old engine's typesetting pipeline,
which was deleted in old-engine retirement S7 (`git show f16430793:crates/page-builder/CONTRACT.md`
has it). The crate is kept as an oracle (retirement plan §2).

## Regenerating the oracle

Requires MacTeX `pdflatex` and poppler `pdftotext`; `cargo test` never runs
TeX.

    python3 crates/page-builder/oracle/gen.py [--only NAME ...] [--workdir DIR]

The script writes `oracle/fixtures/*.tex` and `oracle/expected/*.ftpb`. The line
format is documented in `src/format.rs` and `tests/oracle.rs`.
