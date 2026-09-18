# F12 — float / `tabular` boundaries: **withdrawn and re-attributed**

**GH-F12-FLOAT-TABULAR.** Lane `daniel-parent`, branch
`agent/daniel-parent/f12-float-tabular`, machine `mac-m5pro-dq222`, 2026-09-16.
Read-only audit plus one new test: **no rendering behaviour changes here.**

Finding **F12** of the corpus ranking
(`docs/evidence/corpus-fidelity-2026-09-16T1130Z/README.md`, as corrected by
#753 and re-attributed by #757) reads:

> | F12 | float and `tabular` boundaries | −0.056 / −0.086 / −0.069 bp | 3 | **20** | 0 | unaffected |

**It is not a float defect and not a `tabular` defect.** Every skip those
boundaries spend is exact at 10, 11 and 12 pt. The three witnesses are two
*other* causes wearing a float's clothes, and both of them are under the
project's own gates.

| witness | step | 20 lines | actual cause |
|---|---|---|---|
| `plain-article` p2, after `\end{tabular}` | −0.0560 bp | 8 | the **following line's** `charht`: ours is `ec-lmr*`, pdfLaTeX's is `cmr*`. The `tabular` only makes `\lineskip` apply. Same family as **#754**. |
| `thesis-chapter` p2, before `\begin{figure}` | −0.0861 bp | 9 | the `draft` `\includegraphics` box of a **missing file**: pdfTeX sizes it through `\Gscale@div`, whose quantised scale factor we compute exactly. |
| `conf-paper` p4, page origin | −0.0690 bp | 3 | the same `draft` `\includegraphics` box, at the top of a `figure*`. |

All three corpus pages carry a `fil`-order glue set, so **#757's shrunk-page
caveat does not apply to any of them** — see [Are the instances on shrunk
pages?](#are-the-instances-on-shrunk-pages) below. The measurement was
reproduced from the same producer before anything else was done.

---

## Method

`tools/visual-oracle/cumulative.py` at #757, producer `flashtex-render --v2` →
`flashtex-pdf-exact from-v2` built from `origin/main` `9c5b028d`, fonts
`apps/mac/Fonts`. References rendered now with `/Library/TeX/texbin/pdflatex`
(pdfTeX 1.40.29, TeX Live 2026), two passes, `SOURCE_DATE_EPOCH=0
FORCE_SOURCE_DATE=1`. Box dimensions read out of pdfTeX itself with
`\showbox` (`showbox.py`). **pdflatex is an oracle only, never in the product
path, and nothing here is a parity claim.**

`probes/` holds 69 minimal documents. The reference PDFs are not committed;
regenerate one with `cd probes/<id> && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
pdflatex -interaction=batchmode main.tex` twice, rename `main.pdf` to
`reference.pdf`, and point `cumulative.py --fixtures` at the directory.
`probes/inl-real*` include a graphic: compile their `pic.tex` first (two
passes) to produce the `pic.pdf` they read. No binary asset is committed.

### Are the instances on shrunk pages?

No. From this lane's own re-run of the three fixtures (identical to #757's
`cumulative.md` rows):

| fixture | page | reference glue set | order | displaces baselines? |
|---|---|---|---|---|
| `plain-article` | 2 | 0.0 | `fil` | no |
| `thesis-chapter` | 2 | 0.0 | `fil` | no |
| `conf-paper` | 4 | 0.0 | `fil` | no |

(`thesis-chapter` p3 is the fixture's shrunk page and carries no ranked step.)
Every probe page below is likewise `fil`-order, so nothing here is a shrink
artefact either.

Reproduction of the three steps with this lane's binaries, before any probe
was built:

```
| 1 | `before \begin{figure}` | cumulative | 0.0861 | 0.0861 | 1 | 1 | 9 | 0 | no |
| 2 | `after \end{tabular}`   | cumulative | 0.056  | 0.056  | 1 | 1 | 8 | 0 | no |
| 3 | `page-origin`           | page-origin| 0.069  | 0.069  | 1 | 1 | 3 | 0 | no |
```

---

## 1. The float skips and the `tabular` box are exact

Eight probes, three body sizes, all on `fil`-order pages. `dy` is
candidate − reference in bp at the first baseline after the boundary.

| probe | what it isolates | 10 pt | 11 pt | 12 pt |
|---|---|---|---|---|
| `e-figrule-nocap` | `\intextsep` on both sides of an `[h]` `figure` | **exact** | **exact** | **exact** |
| `d-figrule` | + `\abovecaptionskip` and the caption's own line | **exact** | **exact** | **exact** |
| `g-figtop` | `\textfloatsep` under a `[t]` `figure` | **exact** | **exact** | **exact** |
| `c-tabfloat-nocap` | a ruled `tabular` in a `table`, no caption | **exact** | **exact** | **exact** |
| `n4b-C` | a `[b]`-aligned `tabular` in running text | **exact** | **exact** | **exact** |
| `n4-x` | `tabular` → a line whose tallest glyph is `x` | **exact** | **exact** | **exact** |
| `n4-par` | `tabular` → a line whose tallest glyph is `(` | **exact** | **exact** | **exact** |
| `z-text` | control, no float and no `tabular` | **exact** | **exact** | **exact** |

"exact" is every baseline on the page within 0.0006 bp — three orders of
magnitude under the 0.5 bp glyph gate.

The `tabular`'s **internal** geometry is exact too. `a-tabinline-10`, every
baseline (`dy` in bp):

```
  y=  81.963 dy=-0.00036  Alpha            (body text)
  y=  93.918 dy=-0.00019  Bravo
  y= 103.681 dy=+0.00019  Problem          (tabular row 1)
  y= 116.035 dy=-0.00014  Duplicate        (row 2, one \hline above it)
  y= 127.990 dy=+0.00003  Stale            (row 3)
  y= 139.945 dy=+0.00020  Bottlenecks      (row 4)
  y= 151.845 dy=-0.05546  Charlie          <- the step
  y= 163.800 dy=-0.05529  Delta
  y= 175.755 dy=-0.05512  Echo
```

and pdfTeX's own `\showbox` of that `tabular` agrees with the model the rows
imply:

```
10 pt  \hbox(27.09999+22.09999)x234.44498     4 rows x 12 pt + 3 \hline x 0.4 pt = 49.2
11 pt  \hbox(30.53746+25.06248)x254.43698     = 4 x 13.6 + 1.2 = 55.6
12 pt  \hbox(32.59996+26.59996)x271.23497     = 4 x 14.5 + 1.2 = 59.2
```

`c-tabfloat-nocap` is the decisive one: the float `\vbox`'s height **is**
`height + depth` of that `tabular` (`\@endfloatbox`'s `\par\vskip\z@skip`
leaves the vbox depth 0), and the body text under the float is exact at all
three sizes. So the total is right, and the `\vcenter` split into height and
depth is right — `n4t` (`[t]`) and `n4b` (`[b]`) change the split and do not
change the step.

## 2. The step belongs to the *following line*, not the boundary

`\lineskip`, not `\baselineskip`, is what makes a glyph height visible.
TeX §679 puts a box's baseline at `prev + prevdepth + g + height(box)` with
`g = \baselineskip − prevdepth − height(box)`, replaced by `\lineskip` when
that falls below `\lineskiplimit`. On the normal branch `height(box)`
cancels and the baseline lands `\baselineskip` lower whatever the glyphs
are. A four-row `\vcenter`ed `tabular` is 22 pt deep, so `g` goes negative,
`\lineskip` is used, and the next baseline carries `height(box)` **directly**.

Vary the two sides independently (probe family 2, all at 10/11/12 pt):

| probe | change | step (10 / 11 / 12 pt) |
|---|---|---|
| `n4-C` | reference: 4 rows, follower `Charlie charlie…` | −0.0562 / −0.0610 / −0.0664 |
| `n2-C` | 2 rows instead of 4 | −0.0552 / −0.0610 / −0.0664 |
| `n6-C` | 6 rows | −0.0562 / −0.0600 / −0.0674 |
| `n4s15-C` | `\arraystretch{1.5}` | −0.0559 / −0.0606 / −0.0675 |
| `n4t-C` | `[t]`-aligned `tabular` | −0.0562 / −0.0610 / −0.0664 |
| `rule-C` | **no `tabular` at all** — a `\rule[-30pt]{2in}{40pt}` | −0.0556 / −0.0606 / −0.0664 |
| `n4-x` | follower `xxx xxx…` | **exact** |
| `n4-par` | follower `(xxx) (xxx)…` | **exact** |
| `n4-dig` | follower `123 456 789…` | −0.1465 / −0.1613 / −0.1755 |

Row count, `\arraystretch` and the height/depth split move it by less than
0.001 bp. Replacing the `tabular` with a raised `\rule` reproduces it
exactly. Changing the **follower's glyphs** moves it by a factor of three, or
to zero.

## 3. What the residual is: `cmr` heights against `ec-lmr` heights

A line's height is its tallest glyph's TFM `charht`. pdfLaTeX without
`lmodern` sets text from OT1 `cmr*`; this pipeline measures with T1 `ec-lmr*`
(`crates/render-pipeline/src/fonts.rs:latin_modern_tfm`). Latin Modern is
metric-compatible with Computer Modern in **widths** — which is why every
horizontal measurement in these probes is exact — and **not** in heights
(`tftopl`, TeX Live 2026, em):

| glyph | `cmr10` | `ec-lmr10` | `cmr12` | `ec-lmr12` |
|---|---|---|---|---|
| `h`, `l` (ascender) | 0.694445 | 0.688875 | 0.694444 | 0.688874 |
| `C` (cap) | 0.683332 | 0.688875 | 0.683333 | 0.688874 |
| digits | 0.644444 | 0.629724 | 0.644444 | 0.629729 |
| `x` | 0.430555 | 0.430550 | 0.430556 | 0.430556 |
| `(` | 0.750000 | 0.750000 | 0.750000 | 0.750000 |

Predicting the step from that table alone — `(cmr − ec-lmr) × body size ÷
1.00375` — against the measured step, over six independent measurements:

| document | follower | predicted (bp) | measured (bp) | difference |
|---|---|---|---|---|
| 10 pt | ascender | −0.05549 | −0.05566 | 0.00017 |
| 11 pt | ascender | −0.06076 | −0.06100 | 0.00024 |
| 12 pt | ascender | −0.06659 | −0.06640 | 0.00019 |
| 10 pt | digits | −0.14665 | −0.14676 | 0.00011 |
| 11 pt | digits | −0.16058 | −0.16130 | 0.00072 |
| 12 pt | digits | −0.17592 | −0.17550 | 0.00042 |

and `x` (5 × 10⁻⁶ em apart) and `(` (identical) predict and measure zero.

The 11 pt column is the extra confirmation that this is a *design size*
question and not a tweak: at 11 pt both sides use their **10 pt** design at
10.95 pt, so the em-ratio is the 10 pt one; at 12 pt both switch design.
Predicting 12 pt with the 10 pt ratio would be 0.4 % out for the ascender and
1.8 % out for digits, and it is not.

**This is #754's defect in the text path.** #754 established that math family
0 is `cmr*`, not `rm-lmr*`, unless `lmodern` is loaded, and measured the same
digit height (`\ht\hbox{$1$}` 6.44444 pt against 6.29724 pt at 10 pt — the
same two numbers as the table above). Text runs have the same split and it is
**not** fixed. Nothing here changes it: switching the text metric source is a
font-routing decision with a blast radius across every document, and #754 and
#758 are open on the same family.

### Where it can be seen at all

Only where `height` fails to cancel:

* `\lineskip` after a deep box — a `\vcenter`ed `tabular`, a raised rule, a
  tall math box, a `\parbox`;
* `\topskip` at the top of a page, when the first line is taller than 10 pt;
* `\lineskiplimit` decisions that flip.

Ordinary consecutive text lines are unaffected, which is why 21 of 22 corpus
fixtures never show it.

## 4. The other two witnesses: a `draft` `\includegraphics` box

`thesis-chapter` and `conf-paper` both write
`\includegraphics[draft,width=…]{<file>}` for a file that is **not in the
fixture**. `f-figdraft` reproduces it, and `d-figrule` — the same float with a
`\rule` of the same order of size instead — is exact, so the float is not the
subject:

| probe | 10 pt | 11 pt | 12 pt |
|---|---|---|---|
| `d-figrule` (a `\rule` in the float) | exact | exact | exact |
| `f-figdraft` (a `draft` graphic in the float) | −0.0890 | −0.0890 | −0.0888 |
| `dtop` (`[t]` float, `draft` graphic — `conf-paper`'s shape) | −0.0892 | — | — |

The box is where the two differ. pdfTeX, `\showbox`, 10 pt, `\linewidth`
469.75502 pt, natural size `0 0 72 72` bp (`pdftex.def`'s bounding box for a
file it cannot read):

```
\includegraphics[draft,width=0.7\linewidth]  ->  \hbox(328.91798+0.0)x328.82707
\includegraphics[draft,width=100pt]          ->  \hbox(100.00531+0.0)x100.0
\includegraphics[draft]                      ->  \hbox(72.26999+0.0)x72.26999
```

The natural box is square, so a faithful scale would give `height = width`.
pdfTeX's does not: `\Gscale@div` (graphics.sty) computes the scale factor by
doubling a `\dimen` until it exceeds 8192 pt, dividing, and rounding to sp,
and the quantised factor is then multiplied back — +0.00531 pt at a 100 pt
request, **+0.09091 pt** at 328.83 pt. This pipeline divides in `f64`
(`crates/render-pipeline/src/graphics.rs:size_box`) and lands on the
arithmetically correct height, 0.0893 bp above pdfTeX's.

At 0.089 bp this sits under the 0.1 bp rule gate, and emulating
`\Gscale@div` would change the box of **every** scaled `\includegraphics`,
not just `draft` ones. Filed, not fixed here.

## 5. Found on the way, not part of F12

An `\includegraphics` **outside a float reserves no vertical space at all**.
Probes `dw100` / `dw200` / `dw300` / `dnat` (`draft`, missing file) and
`inl-real` / `inl-realnat` (a real PDF this evidence generates), all inline in
running text:

| probe | graphic | first baseline after it, ours − pdflatex |
|---|---|---|
| `inl-real` | real PDF, `width=200pt` | **−100.628 bp** |
| `inl-realnat` | real PDF, natural 72 × 36 bp | **−36.996 bp** |
| `dw100` | `draft`, `width=100pt` | −100.628 bp |
| `dw300` | `draft`, `width=300pt` | −299.957 bp |
| `dnat` | `draft`, natural | −72.996 bp |

The whole box height is dropped; inside a float the same graphic is placed
correctly. That is three orders of magnitude over every gate and has nothing
to do with F12 — it needs its own issue and its own lane: filed as **#762**.

## The gate this leaves behind

`crates/render-pipeline/tests/float_tabular_boundary.rs`, fixtures in
`crates/render-pipeline/fixtures/float-tabular-boundary/` (24 documents, 8
probes × 3 body sizes, 216 baselines). Every baseline pdfTeX set on the page
is asserted, with a per-line expected residual: **0 within the 0.1 bp rule
gate** for every float skip, caption skip and `tabular` boundary, and the
`cmr − ec-lmr` difference *computed from the TFM table above, not from this
engine's output*, within 0.003 bp, for the two followers that expose it. The
day the text metrics are corrected the residual rows fail and say so, instead
of quietly following the engine.

## Not done

* **The text metric source is not changed.** See §3. It is #754's family and
  it needs that decision, not this lane's.
* **`\Gscale@div` is not emulated.** See §4: under the rule gate, and it
  would move every scaled graphic.
* **The inline-`\includegraphics` hole is not fixed.** See §5; filed as #762.
* **No truncation experiment** was needed: all three corpus pages are
  `fil`-order, so the steps are directly measurable.
* The `pbox-C` probe (a `\parbox` where the `tabular` stood) shows steps of
  +4.15 / +5.10 / +5.28 bp — far over the gate and a different cause
  (paragraph-break spacing round a `\parbox`), unexamined here.
