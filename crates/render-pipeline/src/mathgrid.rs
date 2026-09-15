//! `array`/`cases`/matrix grids set on the pipeline side. math-layout has no
//! array atom, so each cell is laid out as its own formula (text style, as
//! the `$##$` cells of LaTeX's `\halign` are) and the cells are placed by
//! LaTeX's `\@array` rules: `\arraycolsep` (5pt) on both sides of every
//! column, `\@arstrut` (0.7/0.3 `\baselineskip` scaled by `\arraystretch`)
//! in every row, rows at `\baselineskip` unless a cell forces `\lineskip`,
//! `\\[<dimen>]` as extra row depth, the whole `\vcenter`ed on the math
//! axis. amsmath's `matrix` family drops the outer `\arraycolsep`; `cases`
//! is `\left\{\array{@{}l@{\quad}l@{}}` with `\arraystretch 1.2`;
//! `aligned`/`gathered` set `\displaystyle` cells opened up by `\jot`. The
//! fences (`cases`, `pmatrix`, ...) are sized like `\left`/`\right`
//! (Appendix G Rule 19, `var_delimiter`).

use flashtex_compiler::Span;
use flashtex_math_layout as ml;
use ml::boxes::{BoxKind, Child};
use ml::{Glyph, MathBox, MathFontMetrics, MathParams, Style};

/// `\arraycolsep` (array.sty and LaTeX's default), in points.
pub const ARRAYCOLSEP: f64 = 5.0;
/// amsmath `\jot`.
pub const JOT: f64 = 3.0;

/// How one grid environment places its cells.
#[derive(Debug, Clone, PartialEq)]
pub struct GridSpec {
    pub env: String,
    /// Extra depth after each row (`\\[<dimen>]`), in points; may be
    /// shorter than the row count.
    pub row_skips: Vec<f64>,
    /// `\arraystretch`.
    pub stretch: f64,
    /// Glue on each side of a column, in points (`\arraycolsep`).
    pub colsep: f64,
    /// The outer `\arraycolsep`s removed (amsmath matrices, `cases`).
    pub trim_outer: bool,
    /// A fixed gap between columns instead of `2\arraycolsep` (`cases`:
    /// `\quad`; `aligned`: none inside an `rl` pair, `\quad` between).
    pub gaps: Gaps,
    /// The cells' math style.
    pub style: Style,
    /// `\openup\jot` (amsmath alignments).
    pub jot: f64,
    /// The environment's own `\baselineskip`/`\lineskip`/`\lineskiplimit`
    /// with interline glue between rows (amsmath `smallmatrix`); `None`
    /// uses the enclosing text's.
    pub pitch: Option<Pitch>,
    /// Whether every row carries `\@arstrut`/`\strut@` (`smallmatrix` has none).
    pub strut: bool,
    /// Math glue (mu) before and after the `\vcenter` (`smallmatrix`:
    /// `\null\,` ... `\,`).
    pub outer_mu: f64,
    /// The box's vertical position: `\vtop` (`t`), `\vbox` (`b`) or
    /// `\vcenter` (`c`), from `array[t]` (latex.ltx `\@array`) or amsmath's
    /// `aligned[t]`/`gathered[b]` (`\ams@start@box`).
    pub vpos: char,
}

/// amsgen.sty `\compute@ex@` (lines 104-125): amsmath's `\ex@` at font size
/// `size` pt, in TeX's scaled-point arithmetic (1pt at 10pt, 1.11472pt at 12pt).
pub fn amsmath_ex_pt(size: f64) -> f64 {
    const PT: i64 = 65536;
    let size_sp = (size * PT as f64).round() as i64;
    if -size_sp < -20 * PT {
        return 1.5;
    }
    let mut d = (-size_sp + 10 * PT) * 2;
    let negative = d > 0;
    d = d.abs() - 1000;
    // `\vfuzz=.97\vfuzz`: the decimal .97 scans as 63570/65536.
    let mut vfuzz = PT;
    while d > 0 {
        vfuzz = vfuzz * 63570 / PT;
        d -= PT;
    }
    let d = PT - vfuzz;
    let ex = if negative { PT - d } else { PT + d };
    ex as f64 / PT as f64
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gaps {
    /// `\arraycolsep` on both sides of every column.
    ColSep,
    /// `n` quads between adjacent columns, none outside.
    Quads(f64),
    /// amsmath `aligned`: columns pair up (`r` then `l`) with no gap inside
    /// a pair and a quad between pairs.
    Pairs,
}

impl GridSpec {
    /// The spec for the environment whose `\begin` is at `span.start` in
    /// `src`; the row skips are read from the environment's `\\[<dimen>]`s.
    /// `size` is the class size (for `em`).
    pub fn from_source(src: &str, span: Span, size: u32) -> GridSpec {
        let (env, row_skips, vpos) = grid_env(src, span, size).unwrap_or_else(|| ("array".to_string(), Vec::new(), 'c'));
        let mut spec = GridSpec {
            env: env.clone(),
            row_skips,
            stretch: 1.0,
            colsep: ARRAYCOLSEP,
            trim_outer: false,
            gaps: Gaps::ColSep,
            style: Style::TEXT,
            jot: 0.0,
            pitch: None,
            strut: true,
            outer_mu: 0.0,
            vpos,
        };
        match env.as_str() {
            "cases" => {
                spec.stretch = 1.2;
                spec.gaps = Gaps::Quads(1.0);
                spec.trim_outer = true;
            }
            // `cases` and `rcases` do NOT share a macro path: `cases` is
            // amsmath.sty's own `\env@cases` (`\arraystretch=1.2`,
            // `\array{@{}l@{\quad}l@{}}`, amsmath.sty line 1121), while
            // `rcases` is mathtools.sty's `\newcases`/`\MT_start_cases:nnnn`
            // (`\ialign` with `\spread@equation`, mathtools.sty line 995).
            // They land on numerically equivalent spacing (1.2 stretch, a
            // `\quad` gap) by coincidence of both authors' choices, not
            // shared code, which is why this compiler models them
            // identically here.
            "rcases" => {
                spec.stretch = 1.2;
                spec.gaps = Gaps::Quads(1.0);
                spec.trim_outer = true;
            }
            // mathtools.sty `\MT_start_cases:nnnn` (lines 995-1026): `\left\lbrace
            // \vcenter{\spread@equation \ialign{\strut@$\displaystyle##$\hfil
            // &\quad\strut@$\displaystyle##$\hfil}}\right.`.
            "dcases" => {
                spec.gaps = Gaps::Quads(1.0);
                spec.trim_outer = true;
                spec.style = Style::DISPLAY;
                spec.jot = JOT;
            }
            "matrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" => spec.trim_outer = true,
            // amsmath.sty lines 1060-1066: `\null\,\vcenter{\baselineskip6\ex@
            // \lineskip1.5\ex@ \lineskiplimit\lineskip \ialign{\hfil
            // $\scriptstyle##$\hfil&&\thickspace\hfil$\scriptstyle##$\hfil}}\,`.
            "smallmatrix" => {
                let ex = amsmath_ex_pt(size as f64);
                spec.trim_outer = true;
                spec.style = Style::SCRIPT;
                spec.colsep = 0.0;
                spec.gaps = Gaps::Quads(5.0 / 18.0);
                spec.pitch = Some(Pitch {
                    baselineskip: 6.0 * ex,
                    lineskip: 1.5 * ex,
                    lineskiplimit: 1.5 * ex,
                });
                spec.strut = false;
                spec.outer_mu = 3.0;
            }
            "aligned" | "alignedat" | "split" => {
                spec.gaps = Gaps::Pairs;
                spec.style = Style::DISPLAY;
                spec.jot = JOT;
            }
            "gathered" => {
                spec.gaps = Gaps::Quads(0.0);
                spec.style = Style::DISPLAY;
                spec.jot = JOT;
            }
            _ => {}
        }
        spec
    }
}

/// The environment name at `\begin{...}`, the `\\[<dimen>]` row skips of
/// its body (depth 0, in order; a row without one gets 0) and the box
/// position letter of `array`/`aligned`/`alignedat`/`gathered` (`c` when
/// absent or unrecognised, as amsmath's `\ams@start@box` falls back to
/// `\vcenter`).
fn grid_env(src: &str, span: Span, size: u32) -> Option<(String, Vec<f64>, char)> {
    let rest = src.get(span.start..)?;
    let rest = rest.strip_prefix("\\begin")?.trim_start();
    let inner = rest.strip_prefix('{')?;
    let close = inner.find('}')?;
    let name = inner[..close].trim().to_string();
    let body = &inner[close + 1..];
    let vpos = if matches!(name.as_str(), "array" | "aligned" | "alignedat" | "gathered") {
        match body.trim_start().strip_prefix('[').and_then(|r| r.split(']').next()).map(str::trim) {
            Some("t") => 't',
            Some("b") => 'b',
            _ => 'c',
        }
    } else {
        'c'
    };
    let end = format!("\\end{{{name}}}");
    let body = body.split(&end).next().unwrap_or(body);
    let bytes = body.as_bytes();
    let mut skips = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    let mut row = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'\\' if i + 1 < bytes.len() && bytes[i + 1] == b'\\' => {
                if depth == 0 {
                    let after = body[i + 2..].trim_start_matches(['*', ' ', '\t']);
                    let skip = after.strip_prefix('[').and_then(|a| a.find(']').and_then(|c| crate::adapter::parse_dimen_pt(&a[..c], size))).unwrap_or(0.0);
                    while skips.len() <= row {
                        skips.push(0.0);
                    }
                    skips[row] = skip;
                    row += 1;
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    Some((name, skips, vpos))
}

/// Line pitch parameters of the enclosing text (`\baselineskip`,
/// `\lineskip`, `\lineskiplimit`), in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
    pub baselineskip: f64,
    pub lineskip: f64,
    pub lineskiplimit: f64,
}

/// Places laid-out cells (`rows[i][j]`) as `\@array` does and `\vcenter`s
/// the result on the axis (or sets it as a `\vtop`/`\vbox`, `spec.vpos`).
/// `columns` are the `l`/`c`/`r` letters. `p` holds the parameters of the
/// size the grid sits at (its axis, and the mu of `smallmatrix`'s outer
/// `\,`); `quad` is the text font's em that the column gaps (`\quad`,
/// `\thickspace`, written in the preamble's text mode) are measured in,
/// which does not shrink when the grid sits in a script.
pub fn layout_grid(rows: Vec<Vec<MathBox>>, columns: &str, spec: &GridSpec, pitch: Pitch, p: &MathParams, quad: f64) -> MathBox {
    let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let cols: Vec<char> = columns.chars().chain(std::iter::repeat('c')).take(ncols).collect();
    let widths: Vec<f64> = (0..ncols).map(|j| rows.iter().filter_map(|r| r.get(j)).map(|b| b.width).fold(0.0, f64::max)).collect();
    let pitch = spec.pitch.unwrap_or(pitch);
    let outer = spec.outer_mu * p.mu();
    // Column x origins and the total width.
    let mut xs = Vec::with_capacity(ncols);
    let mut x = outer;
    match spec.gaps {
        Gaps::ColSep => {
            for (j, w) in widths.iter().enumerate() {
                let lead = if j == 0 && spec.trim_outer { 0.0 } else { spec.colsep };
                x += lead;
                xs.push(x);
                x += w + if j + 1 == ncols && spec.trim_outer { 0.0 } else { spec.colsep };
            }
        }
        Gaps::Quads(n) => {
            for (j, w) in widths.iter().enumerate() {
                if j > 0 {
                    x += n * quad;
                }
                xs.push(x);
                x += w;
            }
        }
        Gaps::Pairs => {
            for (j, w) in widths.iter().enumerate() {
                if j > 0 && j % 2 == 0 {
                    x += quad;
                }
                xs.push(x);
                x += w;
            }
        }
    }
    let width = x + outer;
    // Row extents: the strut, then the cells.
    let bs = pitch.baselineskip + spec.jot;
    let strut = if spec.strut { (spec.stretch * 0.7 * pitch.baselineskip, spec.stretch * 0.3 * pitch.baselineskip) } else { (0.0, 0.0) };
    let mut extents: Vec<(f64, f64)> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let h = r.iter().map(|b| b.height).fold(strut.0, f64::max);
            let d = r.iter().map(|b| b.depth).fold(strut.1, f64::max);
            (h, d + spec.row_skips.get(i).copied().unwrap_or(0.0))
        })
        .collect();
    if extents.is_empty() {
        extents.push(strut);
    }
    // Baselines from the top. `\@array` sets `\baselineskip\z@
    // \lineskip\z@`: rows simply stack (the strut keeps them at least a
    // `\baselineskip` apart, `\\[<dimen>]` adds its depth). amsmath's
    // alignments keep interline glue at the opened-up `\baselineskip`.
    let mut baselines = Vec::with_capacity(extents.len());
    let mut y = extents[0].0;
    baselines.push(y);
    for i in 1..extents.len() {
        let (prev_d, h) = (extents[i - 1].1, extents[i].0);
        let dist = if spec.jot > 0.0 || spec.pitch.is_some() {
            // `\openup\jot` advances `\lineskip` and `\lineskiplimit` too.
            let glue = bs - prev_d - h;
            if glue < pitch.lineskiplimit + spec.jot { prev_d + h + pitch.lineskip + spec.jot } else { bs }
        } else {
            prev_d + h
        };
        y += dist;
        baselines.push(y);
    }
    let last_depth = extents.last().map_or(0.0, |e| e.1);
    let total = y + last_depth;
    let (height, depth) = match spec.vpos {
        // `\vtop` (tex.web §1087): the height of the first row box.
        't' => (extents[0].0, total - extents[0].0),
        // `\vbox`: the depth of the last row box.
        'b' => (total - last_depth, last_depth),
        // `\vcenter` (§736): centred on the axis.
        _ => (total / 2.0 + p.axis_height, total / 2.0 - p.axis_height),
    };
    let mut children = Vec::new();
    for (i, row) in rows.into_iter().enumerate() {
        for (j, cell) in row.into_iter().enumerate() {
            let slack = widths[j] - cell.width;
            let dx = xs[j]
                + match cols[j] {
                    'l' => 0.0,
                    'r' => slack,
                    _ => slack / 2.0,
                };
            children.push(Child {
                dx,
                dy: baselines[i] - height,
                content: cell,
            });
        }
    }
    MathBox {
        kind: BoxKind::HBox(children),
        width,
        height,
        depth,
        ..MathBox::empty()
    }
}

/// A `\left`/`\right` delimiter for a box of the given height and depth
/// (Rule 19 and `var_delimiter`), centred on the axis; `None` for a null
/// delimiter (`\right.`), which is `\nulldelimiterspace`. Returns the box
/// and, when no variant is tall enough and no extensible recipe exists,
/// the (wanted, used) sizes.
pub fn delimiter(metrics: &dyn MathFontMetrics, ch: Option<char>, height: f64, depth: f64, style: Style, p: &MathParams) -> (MathBox, Option<(f64, f64)>) {
    let Some(ch) = ch else {
        return (MathBox::kern(p.null_delimiter_space), None);
    };
    let a = p.axis_height;
    let delta1 = (height - a).max(depth + a);
    let wanted = (delta1 * 2.0 * p.delimiter_factor).max(2.0 * delta1 - p.delimiter_shortfall);
    let sizes = metrics.delimiter_sizes(ch, style.size_class());
    let ext = metrics.delimiter_extensible(ch, style.size_class());
    // `char_box` width includes the italic correction, set as a kern so the
    // glyph box keeps its TFM width.
    let glyph_box = |g: &Glyph| {
        if g.italic == 0.0 {
            MathBox::glyph(g)
        } else {
            MathBox::hlist(vec![MathBox::glyph(g), MathBox::kern(g.italic)])
        }
    };
    let (b, short) = if let Some(chosen) = sizes.iter().find(|g| g.total_height() >= wanted) {
        (glyph_box(chosen), None)
    } else if let Some(recipe) = ext {
        (stack_extensible(&recipe, wanted), None)
    } else if let Some(largest) = sizes.last() {
        (glyph_box(largest), Some((wanted, largest.total_height())))
    } else {
        return (MathBox::kern(p.null_delimiter_space), None);
    };
    let shift = (b.height - b.depth) / 2.0 - a;
    (b.shifted(shift), short)
}

/// tex.web §713 (math-layout's own `stack_extensible`, which is private):
/// `bot`, n×`rep`, `mid`, n×`rep`, `top` until the total reaches `wanted`.
fn stack_extensible(r: &ml::metrics::Extensible, wanted: f64) -> MathBox {
    let hpd = |g: &Option<Glyph>| g.as_ref().map(Glyph::total_height).unwrap_or(0.0);
    let u = r.rep.total_height();
    let mut w = hpd(&r.bot) + hpd(&r.mid) + hpd(&r.top);
    let mut n = 0usize;
    if u > 0.0 {
        while w < wanted {
            w += u;
            n += 1;
            if r.mid.is_some() {
                w += u;
            }
        }
    }
    let mut pieces: Vec<&Glyph> = Vec::new();
    if let Some(t) = &r.top {
        pieces.push(t);
    }
    pieces.extend(std::iter::repeat_n(&r.rep, n));
    if let Some(m) = &r.mid {
        pieces.push(m);
        pieces.extend(std::iter::repeat_n(&r.rep, n));
    }
    if let Some(b) = &r.bot {
        pieces.push(b);
    }
    let height = pieces.first().map(|g| g.height).unwrap_or(0.0);
    let mut children = Vec::with_capacity(pieces.len());
    let mut y_top = -height;
    for g in &pieces {
        children.push(Child {
            dx: 0.0,
            dy: y_top + g.height,
            content: MathBox::glyph(g),
        });
        y_top += g.total_height();
    }
    MathBox {
        width: r.rep.width + r.rep.italic,
        height,
        depth: w - height,
        kind: BoxKind::VBox(children),
        ..MathBox::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_skips_are_read_from_the_environment_body() {
        let src = "\\[\\begin{array}{ll}\\text{(a)} & x,\\\\[2pt]\\text{(b)} & {y\\\\[9pt]},\\\\ \\text{(c)} & z\\end{array}\\]";
        let at = src.find("\\begin").unwrap();
        let (name, skips, vpos) = grid_env(src, Span::in_document(Default::default(), at, at + 6), 11).unwrap();
        assert_eq!(name, "array");
        assert_eq!(skips, vec![2.0, 0.0]);
        assert_eq!(vpos, 'c');
        let pos = |s: &str| grid_env(s, Span::in_document(Default::default(), 0, 6), 12).unwrap().2;
        assert_eq!(pos("\\begin{array}[t]{cc} a & b \\end{array}"), 't');
        assert_eq!(pos("\\begin{aligned} [b] a &= b \\end{aligned}"), 'b');
        assert_eq!(pos("\\begin{gathered}[c] a \\end{gathered}"), 'c');
        assert_eq!(pos("\\begin{matrix}[t] a \\end{matrix}"), 'c', "matrix takes no position argument");
        let spec = GridSpec::from_source(src, Span::in_document(Default::default(), at, at + 6), 11);
        assert_eq!(spec.gaps, Gaps::ColSep);
        assert_eq!(spec.stretch, 1.0);
    }

    #[test]
    fn amsmath_ex_follows_compute_ex() {
        assert_eq!(amsmath_ex_pt(10.0), 1.0);
        assert!((amsmath_ex_pt(12.0) - 1.11472).abs() < 1e-4, "{}", amsmath_ex_pt(12.0));
        assert_eq!(amsmath_ex_pt(25.0), 1.5);
        assert!(amsmath_ex_pt(8.0) < 1.0);
    }

    #[test]
    fn cases_and_matrices_take_amsmath_shapes() {
        let src = "\\begin{cases} a & b \\\\ c & d \\end{cases}";
        let spec = GridSpec::from_source(src, Span::in_document(Default::default(), 0, 6), 10);
        assert_eq!(spec.stretch, 1.2);
        assert_eq!(spec.gaps, Gaps::Quads(1.0));
        let src = "\\begin{pmatrix} 1 & 2 \\end{pmatrix}";
        let spec = GridSpec::from_source(src, Span::in_document(Default::default(), 0, 6), 10);
        assert!(spec.trim_outer && spec.gaps == Gaps::ColSep);
    }

    #[test]
    fn rcases_spacing_matches_cases_not_array() {
        let spec_for = |env: &str| {
            let src = format!("\\begin{{{env}}} a & b \\\\ c & d \\end{{{env}}}");
            GridSpec::from_source(&src, Span::in_document(Default::default(), 0, 6), 10)
        };
        let cases = spec_for("cases");
        let rcases = spec_for("rcases");
        let array = spec_for("array");
        assert_eq!(rcases.stretch, cases.stretch);
        assert_eq!(rcases.gaps, cases.gaps);
        assert_eq!(rcases.trim_outer, cases.trim_outer);
        assert_eq!((rcases.stretch, rcases.gaps, rcases.trim_outer), (1.2, Gaps::Quads(1.0), true));
        assert_ne!((rcases.stretch, rcases.gaps), (array.stretch, array.gaps));
        assert_eq!((array.stretch, array.gaps, array.trim_outer), (1.0, Gaps::ColSep, false));
    }

    #[test]
    fn grid_places_columns_with_arraycolsep_and_rows_at_baselineskip() {
        let cell = |w: f64, h: f64, d: f64| MathBox::rule(w, h, d);
        let rows = vec![vec![cell(10.0, 5.0, 1.0), cell(20.0, 5.0, 1.0)], vec![cell(30.0, 5.0, 1.0), cell(5.0, 5.0, 1.0)]];
        let spec = GridSpec {
            env: "array".into(),
            row_skips: vec![2.0],
            stretch: 1.0,
            colsep: 5.0,
            trim_outer: false,
            gaps: Gaps::ColSep,
            style: Style::TEXT,
            jot: 0.0,
            pitch: None,
            strut: true,
            outer_mu: 0.0,
            vpos: 'c',
        };
        let pitch = Pitch {
            baselineskip: 12.0,
            lineskip: 1.0,
            lineskiplimit: 0.0,
        };
        let mut p = ml::CmMathMetrics::latex_10pt().params(ml::SizeClass::Text);
        p.axis_height = 2.5;
        p.quad = 10.0;
        // `\vtop`: height of the first row (strut 8.4); `\vbox`: depth of
        // the last row (strut 3.6); the total (26) is unchanged.
        for (vpos, h, d) in [('t', 8.4, 17.6), ('b', 22.4, 3.6)] {
            let spec = GridSpec { vpos, ..spec.clone() };
            let g = layout_grid(rows.clone(), "lr", &spec, pitch, &p, p.quad);
            assert!((g.height - h).abs() < 1e-9 && (g.depth - d).abs() < 1e-9, "{vpos}: {} {}", g.height, g.depth);
        }
        let g = layout_grid(rows, "lr", &spec, pitch, &p, p.quad);
        // 5 + 30 + 5 + 5 + 20 + 5.
        assert!((g.width - 70.0).abs() < 1e-9, "{}", g.width);
        // Strut 8.4/3.6: rows stack (`\baselineskip\z@`), 2pt extra depth
        // after row 1 = 8.4 + (5.6 + 8.4) + 3.6 = 26; centred on the axis 2.5.
        let total = g.height + g.depth;
        assert!((total - 26.0).abs() < 1e-9, "{total}");
        assert!((g.height - (13.0 + 2.5)).abs() < 1e-9);
        let BoxKind::HBox(children) = &g.kind else { panic!() };
        // Row 0: `l` cell at 5, `r` cell right-aligned in its column: 5+30+10 + (20-20) = 45.
        assert!((children[0].dx - 5.0).abs() < 1e-9);
        assert!((children[1].dx - 45.0).abs() < 1e-9);
        // Row 1: `r` cell of width 5 ends at the column's right edge.
        assert!((children[3].dx - 60.0).abs() < 1e-9, "{}", children[3].dx);
        assert!((children[2].dy - children[0].dy - 14.0).abs() < 1e-9);
    }
}
