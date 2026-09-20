//! `beamer.cls` frame geometry for the **default** theme (issue #944, Tier 0
//! and Tier 1): the frametitle box, the frame body's glue, the list
//! parameters, the title page's boxes and the structure colour.
//!
//! Everything here is transcribed from `beamerbaseframe.sty`,
//! `beamerouterthemedefault.sty`, `beamerinnerthemedefault.sty`,
//! `beamerbaselocalstructure.sty` and `beamerfontthemedefault.sty` (beamer
//! v3.71, TeX Live 2026) and checked against `\showbox\beamer@framebox` and
//! `\the\<dimen>` readings of pdflatex on the ladder in
//! `crates/render-pipeline/tests/beamer_default.rs`; every number a function
//! returns has a measured counterpart there. pdflatex is the oracle only.
//!
//! The frame (`beamerbaseframe.sty` lines 118-162, `beamerbaseframesize.sty`
//! 242-256) is one `\vbox to\textheight`:
//!
//! ```text
//! \vbox{}                                   % 0pt
//! <frametitle box>                          % Self::frametitle_box
//! \vskip\beamer@frametopskip                % Self::body_glue(align).above
//! \vskip-\parskip \vbox{} <body ...>        % the frame's material
//! \vskip\beamer@framebottomskip             % .below
//! <footnotes>
//! ```
//!
//! and the page's first box is that vbox, so `\topskip` never acts: the box
//! top is the paper top (`\topmargin` -1in, `\headheight` = `\headsep` = 0).

use crate::class::{BaseSize, FontMetrics, Glue};
use crate::tex::Sp;

fn len(s: &str) -> Sp {
    Sp::parse(s).unwrap_or_else(|| panic!("bad length literal {s}"))
}

/// The fonts beamer's default font theme selects (`beamerfontthemedefault.sty`
/// with `size11.clo`'s sizes, the 11pt class default): `(size, baselineskip)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontSpec {
    pub size: Sp,
    pub baselineskip: Sp,
}

impl FontSpec {
    const fn pt(size: f64, baselineskip: f64) -> FontSpec {
        FontSpec { size: Sp((size * 65536.0) as i64), baselineskip: Sp((baselineskip * 65536.0) as i64) }
    }
}

/// `\Large` at 11pt: `frametitle` and `title`.
pub const LARGE: FontSpec = FontSpec::pt(14.4, 18.0);
/// `\normalsize` at 11pt: the body, `subtitle`, `author`, `date`.
pub const NORMAL: FontSpec = FontSpec::pt(10.95, 13.6);
/// `\footnotesize` at 11pt: `framesubtitle`, footnotes.
pub const FOOTNOTE: FontSpec = FontSpec::pt(9.0, 11.0);
/// `\scriptsize` at 11pt: `institute`.
pub const SCRIPT: FontSpec = FontSpec::pt(8.0, 9.5);

/// beamer's `structure` colour (`beamercolorthemedefault.sty`:
/// `\setbeamercolor{structure}{fg=blue!35!black}`... measured
/// `\extractcolorspec` rgb 0.2,0.2,0.7): the frametitle, the title, list
/// labels and enumerate numbers.
pub const STRUCTURE_RGB: (f64, f64, f64) = (0.2, 0.2, 0.7);
/// `navigation symbols` colour (`fg=structure.fg!40!bg`): rgb 0.68,0.68,0.88.
pub const NAVIGATION_RGB: (f64, f64, f64) = (0.68, 0.68, 0.88);

/// The frametitle box of the default outer theme
/// (`beamerouterthemedefault.sty` lines 162-182), as the frame stacks it
/// (`beamerbaseframe.sty` 121-126: `\vbox{}` then
/// `{\parskip0pt\usebeamertemplate***{frametitle}\vskip0.25em}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTitleBox {
    /// The box's whole height in the frame, `0.25em` included: what the
    /// body's `\textheight` loses (`\beamer@frametextheight`).
    pub height: Sp,
    /// Baseline of the first title line below the frame (paper) top.
    pub title_baseline: Sp,
    /// Baseline of the first `\framesubtitle` line, when there is one.
    pub subtitle_baseline: Option<Sp>,
    /// Where the title text starts, measured from the paper's left edge:
    /// the `beamercolorbox`'s `sep=0.3cm`, because the box is `\textwidth +
    /// \beamer@leftmargin + \beamer@rightmargin` = `\paperwidth` wide and
    /// backed up to the paper edge.
    pub text_left: Sp,
    /// The title paragraph's measure: `\paperwidth - 2 * 0.3cm`.
    pub text_width: Sp,
}

/// The x-height (`\fontdimen5`) of `cmss12` scaled to 14.4pt: the template's
/// two `\vskip-1ex` in the frametitle font. `cmss12.tfm` xheight 0.444443 x
/// 14.4pt = 6.39998pt, the value `\showbox` prints.
pub const FRAMETITLE_EX: Sp = Sp(419_429);

/// [`FrameTitleBox`] for a title of `title_lines` lines (0 = no frametitle:
/// the frame has no box at all) and a subtitle of `subtitle_lines`.
///
/// Derivation, in pt, from the `\showbox` of a one-line title (`Title`):
///
/// ```text
/// \vbox(0+0)                          the frame's leading \vbox{}
/// \glue(\lineskip) 1.0                13.6 - 0 - 19.136 < \lineskiplimit
/// \hbox(19.136+0)                     the beamercolorbox:
///   \glue 8.5359                        sep=0.3cm
///   \hbox(0+0)                          \vbox{}
///   \glue -6.39998                      \vskip-1ex (\Large cmss12)
///   \glue(\baselineskip) 5.40005        18 - 0 - 12.6
///   \hbox(12.59995+5.40005)             \strut Title \strut (\Large strut)
///   \glue -6.39998                      \vskip-1ex
///   \glue -8.5359                       \vskip-.3cm
///   \glue 8.5359                        sep
/// \glue 2.7375                        \vskip0.25em (body font)
/// ```
///
/// so the box is `1 + (sep - 2ex + 5.4 + 18) + 2.7375 = 22.8735pt` and the
/// title baseline sits `1 + sep - ex + 5.4 + 12.6 = 21.13592pt` below the
/// paper top (measured 21.057bp). Each further title line adds `18pt`. A
/// `\framesubtitle` (`\footnotesize` strut 7.7+3.3 under `\baselineskip`
/// 11pt: `11 - 5.4 - 7.7 < 0`, so `\lineskip` 1pt) adds `1 + 11` per line
/// and its first baseline is `5.4 + 1 + 7.7 = 14.1pt` under the title's
/// (measured 35.104bp); `\beamer@frametextheight` then reads 234.27312pt,
/// i.e. a 34.8735pt box.
pub fn frametitle_box(paperwidth: Sp, title_lines: usize, subtitle_lines: usize) -> Option<FrameTitleBox> {
    if title_lines == 0 {
        return None;
    }
    let sep = len("0.3cm");
    let lineskip = Sp::pt(1);
    let title_strut_ht = LARGE.baselineskip.scaled("0.7").unwrap();
    let title_strut_dp = LARGE.baselineskip - title_strut_ht;
    let sub_strut_ht = FOOTNOTE.baselineskip.scaled("0.7").unwrap();
    let sub_strut_dp = FOOTNOTE.baselineskip - sub_strut_ht;
    // `\vskip0.25em` in the body font after the template.
    let after = crate::class::body_font(BaseSize::Pt11).em.scaled("0.25").unwrap();
    let extra_title = LARGE.baselineskip.times(title_lines as i64 - 1);
    let title_baseline = lineskip + sep - FRAMETITLE_EX + (LARGE.baselineskip - title_strut_ht) + title_strut_ht;
    let mut inner = sep - FRAMETITLE_EX + (LARGE.baselineskip - title_strut_ht) + title_strut_ht + title_strut_dp + extra_title;
    let mut subtitle_baseline = None;
    if subtitle_lines > 0 {
        subtitle_baseline = Some(title_baseline + extra_title + title_strut_dp + lineskip + sub_strut_ht);
        inner += lineskip + (sub_strut_ht + sub_strut_dp) + FOOTNOTE.baselineskip.times(subtitle_lines as i64 - 1);
    }
    inner = inner - FRAMETITLE_EX;
    // `\vskip-.3cm` and the closing `sep` cancel.
    Some(FrameTitleBox {
        height: lineskip + inner + after,
        title_baseline,
        subtitle_baseline,
        text_left: sep,
        text_width: paperwidth - sep - sep,
    })
}

/// How a frame's body sits between the frametitle box and the frame bottom
/// (`beamerbaseframe.sty` 255-278, keys `t`/`c`/`b`). `above` is the glue's
/// natural part; the `fill` weights are the `plus <n>fill` components, which
/// share the frame's free height in proportion (a finite `plus` next to a
/// `fill` never stretches).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyGlue {
    pub above: Sp,
    pub above_fill: f64,
    pub below_fill: f64,
}

/// `\begin{frame}[t|c|b]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrameAlign {
    /// `[t]`: `\beamer@frametopskip=.2cm plus .5\paperheight`,
    /// `\beamer@framebottomskip=0pt plus 1fill`.
    Top,
    /// `[c]`, the class default: `0pt plus 1fill` above, `0pt plus 1.5fill`
    /// below — the body sits at 40% of the free height.
    #[default]
    Center,
    /// `[b]`: `0pt plus 1fill` above, `0pt` below.
    Bottom,
}

pub fn body_glue(align: FrameAlign) -> BodyGlue {
    match align {
        FrameAlign::Top => BodyGlue { above: len("0.2cm"), above_fill: 0.0, below_fill: 1.0 },
        FrameAlign::Center => BodyGlue { above: Sp::ZERO, above_fill: 1.0, below_fill: 1.5 },
        FrameAlign::Bottom => BodyGlue { above: Sp::ZERO, above_fill: 1.0, below_fill: 0.0 },
    }
}

/// `beamerbaselocalstructure.sty` lines 144-164: the list parameters beamer
/// sets for every size (`\@listi`/`\@listii`/`\@listiii`), replacing
/// `size11.clo`'s. `\leftmargin` is `2em` at every level (`\leftmargini` =
/// `ii` = `iii`), `\labelsep` `.5em`, `\partopsep` 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListLevel {
    pub leftmargin: Sp,
    pub labelsep: Sp,
    pub topsep: Glue,
    pub parsep: Glue,
    pub itemsep: Glue,
    pub partopsep: Glue,
}

/// The parameters of list nesting `depth` (1 = outermost). Measured: item
/// baselines 16.54bp apart (13.6pt + 3pt `\itemsep`), a paragraph 16.54bp
/// after the last item (3pt `\topsep`), label at x = 36.23bp, text at
/// 50.17bp (28.35 + 2em).
pub fn list_level(font: FontMetrics, depth: u8) -> ListLevel {
    let leftmargin = font.em.scaled("2").unwrap();
    let labelsep = font.em.scaled("0.5").unwrap();
    let (topsep, parsep, itemsep) = if depth <= 1 {
        (Glue::new("3pt", "2pt", "2.5pt"), Glue::fixed(Sp::ZERO), Glue::new("3pt", "2pt", "3pt"))
    } else {
        (Glue::new("2pt", "1pt", "2pt"), Glue::new("0pt", "1pt", "0pt"), Glue::new("0pt", "1pt", "0pt"))
    };
    ListLevel { leftmargin, labelsep, topsep, parsep, itemsep, partopsep: Glue::fixed(Sp::ZERO) }
}

/// The default inner theme's `title page` template
/// (`beamerinnerthemedefault.sty` lines 46-71): `\vbox{} \vfill`, then a
/// `beamercolorbox[sep=8pt,center]` for the title (with the subtitle
/// `\vskip0.25em` under it, in the title font's `em`), `\vskip1em`, one box
/// each for the author, institute and date, `\vskip0.5em`, `\vfill`. Each
/// box is taller than `\baselineskip`, so the interline glue before it is
/// `\lineskip` (1pt).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitlePageSpec {
    /// `sep=8pt` above and below every box's content.
    pub sep: Sp,
    /// `\vskip0.25em` between the title and the subtitle: `0.25` of the
    /// `\Large` cmss12 quad (14.0998pt) = 3.52495pt.
    pub subtitle_skip: Sp,
    /// `\vskip1em` after the title box (body font).
    pub after_title: Sp,
    /// `\vskip0.5em` after the date box (body font).
    pub after_date: Sp,
    /// `\lineskip`: the glue before each box.
    pub lineskip: Sp,
}

pub fn title_page() -> TitlePageSpec {
    let em = crate::class::body_font(BaseSize::Pt11).em;
    TitlePageSpec {
        sep: Sp::pt(8),
        // cmss12.tfm quad 0.979166 x 14.4pt.
        subtitle_skip: Sp(231_014),
        after_title: em,
        after_date: em.scaled("0.5").unwrap(),
        lineskip: Sp::pt(1),
    }
}

// ---------------------------------------------------------------------------
// Tier 3: blocks, columns and in-flow floats (issue #944).
// ---------------------------------------------------------------------------

/// `\large` at 11pt: `block title` (`beamerfontthemedefault.sty` 84,
/// `size11.clo` `\large` = 12pt on 14pt).
pub const BLOCK_TITLE: FontSpec = FontSpec::pt(12.0, 14.0);
/// `\small` at 11pt: `caption` (`beamerfontthemedefault.sty` 76; 10pt on 12pt).
pub const SMALL: FontSpec = FontSpec::pt(10.0, 12.0);

/// `alerted text` (`beamercolorthemedefault.sty` 19: `fg=red`) — the
/// `alertblock` title.
pub const ALERT_RGB: (f64, f64, f64) = (1.0, 0.0, 0.0);
/// `example text` (line 20: `fg=green!50!black`) — the `exampleblock` title.
pub const EXAMPLE_RGB: (f64, f64, f64) = (0.0, 0.5, 0.0);

/// The x-height of the body font, `cmss10` at 10.95pt (`\fontdimen5`:
/// 0.444444 x 10.95pt): the `\vskip-.25ex` at the top of a block body and
/// the `\vskip-1ex` of a `[T]` column both read it in the body font.
/// `\showbox` prints `\glue -4.86665` and `\glue -1.21666`.
pub const SANS_BODY_EX: Sp = Sp(318_940);

/// `\fontdimen22` of the math symbol font (`cmsy10` at 10.95pt: 0.25em =
/// 2.7375pt), the axis a `[c]` column's `\vcenter` centres on
/// (`\@iiiparbox`: `$\vcenter{...}\m@th$`).
pub const MATH_AXIS: Sp = Sp(179_405);

/// The default inner theme's `block begin`/`block end` templates
/// (`beamerinnerthemedefault.sty` 390-405) with no background colour (the
/// default colour theme leaves `block title`/`block body` bg empty, so
/// `colsep*` never applies and no box is painted):
///
/// ```text
/// \par\vskip\medskipamount                         before
/// \hbox{\vbox{ \large title, raggedright }}         title box (no strut:
///                                                   glyph heights)
/// {\parskip0pt\par}                                 (\lineskip glue follows,
///                                                   the body box is taller
///                                                   than \baselineskip)
/// \hbox{\vbox{ \vskip-.25ex \vbox{} body ... }}    body box
/// \vskip\smallskipamount                            after
/// ```
///
/// Measured (`fixtures/real-world/beamer-blocks-columns` p2): titles at
/// 84.58 / 133.98 / 183.37bp; a body's first baseline 13.33bp under a
/// depthless title (`1 + 13.6 - .25ex`), 15.66bp under "Example" (+ the
/// `p`'s 2.33pt depth); two-line bodies put the blocks 49.40bp apart
/// (`1 + 25.98 + 3 + 6 + 13.6`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockSpec {
    /// `\medskipamount`: 6pt plus 2pt minus 2pt.
    pub before: Glue,
    /// `\smallskipamount`: 3pt plus 1pt minus 1pt.
    pub after: Glue,
    /// `\vskip-.25ex` (body font) at the top of the body box.
    pub body_top: Sp,
}

pub fn block() -> BlockSpec {
    BlockSpec {
        before: Glue::new("6pt", "2pt", "2pt"),
        after: Glue::new("3pt", "1pt", "1pt"),
        body_top: Sp::ZERO - SANS_BODY_EX.scaled("0.25").unwrap(),
    }
}

/// `\topsep` of a `\trivlist` environment (`center`, and beamer's
/// `figure`/`table`, which are `center`) on a slide: `size11.clo`'s
/// `\@listI` value, 9pt plus 3pt minus 5pt. beamer redefines `\@listi`
/// (3pt, [`list_level`]) but never executes it at the top level, so the
/// register keeps the value the size file set when the class loaded; only
/// a `\list` (itemize, enumerate) runs `\@listi` and sees 3pt. Measured:
/// `\glue 9.0 plus 3.0 minus 5.0` around the `center` of the corpus deck's
/// graphic and table frames.
pub fn trivlist_topsep(size: BaseSize) -> Glue {
    match size {
        BaseSize::Pt10 => Glue::new("8pt", "2pt", "4pt"),
        BaseSize::Pt11 => Glue::new("9pt", "3pt", "5pt"),
        BaseSize::Pt12 => Glue::new("10pt", "4pt", "6pt"),
    }
}

/// `\abovecaptionskip` = `\belowcaptionskip` = 7pt
/// (`beamerbaselocalstructure.sty` 567-568), around the `\small` caption
/// line of an in-flow `figure`/`table`. Measured: caption baseline 176.82bp
/// on the corpus deck's table frame, 21.77bp under the last row
/// (`\bottomrule` 0.876 + 7 + \lineskip 1 + 6.944 + the row's 4.08 strut
/// depth + 1.95 rule gap).
pub const CAPTION_SKIP: Sp = Sp(7 * 65536);

/// How a `columns` row is set (`beamerbaseframecomponents.sty` 212-240).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnsBox {
    /// The default: `\hbox to\textwidth{\hskip-\beamer@leftmargin \hbox to
    /// (\textwidth + both margins){\hbox{}\hfill col \hfill col ... \hfill}
    /// \hskip-\beamer@rightmargin}` — the paper-wide box, a `\hfill`
    /// before, between and after the columns.
    PaperWide,
    /// `[onlytextwidth]` / `[totalwidth=<w>]`: `\hbox to <w>{col \hfill col
    /// \hfill}` whose exit code is `\unskip\egroup` — the last `\hfill`
    /// is removed, so only the gaps *between* columns stretch. Measured:
    /// two `.3\textwidth` columns under `[onlytextwidth]` start at x =
    /// 28.35 and 242.65bp (the second after all 122.92pt of free width).
    Fixed(Sp),
}

/// The left edge of every column, measured from the **text** left edge
/// (negative for the first column of a paper-wide row, which starts
/// `\beamer@leftmargin` to the left of the text). `widths` are the
/// columns' `minipage` widths; `text_width` is the enclosing `\textwidth`;
/// `left_margin`/`right_margin` are `\beamer@leftmargin`/`\beamer@rightmargin`
/// (1cm each in the default theme). `\hfill` glue shares the free width
/// equally; a row wider than its box gets no glue (an overfull `\hbox`).
///
/// Measured (corpus deck p3, two `.5\textwidth` columns): the columns start
/// at x = 18.90bp and 190.87bp from the paper edge, i.e. −9.45bp and
/// 162.52bp from the text edge: `(364.195 − 307.290) / 3 = 18.968pt` per
/// `\hfill`.
pub fn column_origins(kind: ColumnsBox, widths: &[Sp], text_width: Sp, left_margin: Sp, right_margin: Sp) -> Vec<Sp> {
    let total: i64 = widths.iter().map(|w| w.0).sum();
    let (box_width, start, leading) = match kind {
        ColumnsBox::PaperWide => (text_width + left_margin + right_margin, Sp::ZERO - left_margin, true),
        ColumnsBox::Fixed(w) => (w, Sp::ZERO, false),
    };
    // Paper-wide: a fill before, between and after (n + 1); fixed: between
    // only (n - 1), the trailing one `\unskip`ped.
    let fills = if leading { widths.len() as i64 + 1 } else { widths.len() as i64 - 1 };
    let free = (box_width.0 - total).max(0);
    let fill = if fills > 0 { free / fills } else { 0 };
    let mut x = start.0 + if leading { fill } else { 0 };
    let mut out = Vec::with_capacity(widths.len());
    for w in widths {
        out.push(Sp(x));
        x += w.0 + fill;
    }
    out
}

/// The `(height, depth)` of a column's box on the row, from the natural
/// extent of its content (`total` = the content's height plus its last
/// depth, `first_height` its first box's height, `last_depth` its last
/// box's depth), by the `minipage` position letter (`\@iiiparbox`):
///
/// * `[t]`: `\vtop` — height of the first box, the rest is depth;
/// * `[T]`: `\vtop` of an empty `\hbox` first (`\leavevmode` then the
///   `\vskip-1ex` ends that paragraph), so height 0 and everything, the
///   `-1ex` included, is depth;
/// * `[c]`: `$\vcenter{...}\m@th$` — centred on [`MATH_AXIS`];
/// * `[b]`: `\vbox` — the last depth is the depth.
///
/// Measured (corpus deck): `[T]` columns `\vbox(0.0+78.8667)`, `[c]` ones
/// `\vbox(5.17082+-0.30417)` for a 4.86665pt-high single line and
/// `\vbox(14.40416+8.92917)` for two lines totalling 23.33333pt.
pub fn column_box(align: ColumnAlign, total: Sp, first_height: Sp, last_depth: Sp) -> (Sp, Sp) {
    match align {
        ColumnAlign::Top => (first_height, total - first_height),
        ColumnAlign::TopBaseline => (Sp::ZERO, total),
        ColumnAlign::Center => {
            let half = Sp(total.0 / 2);
            (half + MATH_AXIS, total - half - MATH_AXIS)
        }
        ColumnAlign::Bottom => (total - last_depth, last_depth),
    }
}

/// The `beamer@col` alignment keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColumnAlign {
    #[default]
    Center,
    Top,
    /// `T`: `\vskip-1ex\nointerlineskip` before the content.
    TopBaseline,
    Bottom,
}

// ---------------------------------------------------------------------------
// Tier 4: frame options and the Madrid theme (issue #944).
// ---------------------------------------------------------------------------

/// `\tiny` at 11pt (`size11.clo`: 6pt on 7pt): the infolines footline's
/// font (`beamerfontthemedefault.sty` 19/67: `footline` parent `tiny
/// structure`, `size=\tiny`).
pub const TINY: FontSpec = FontSpec::pt(6.0, 7.0);
/// `\fontdimen5` of the `\tiny` sans font (`cmss8 at 6pt`: pdftex prints
/// 2.66666pt, `\dimen0=2.25\fontdimen5` 5.99997pt): the infolines
/// footline boxes' `ht=2.25ex dp=1ex` and `leftskip=2ex`. Measured: the
/// boxes are 8.634bp = 8.66663pt tall and `\footheight` reads 12.66663pt.
pub const TINY_EX: Sp = Sp(174_762);

/// An RGB triple in [0, 1], as `\extractcolorspec` reports it.
pub type Rgb = (f64, f64, f64);

/// `structure.fg!<pct>!black` (xcolor: `pct`% of the colour, the rest
/// black).
const fn shade(rgb: Rgb, pct: f64) -> Rgb {
    (rgb.0 * pct / 100.0, rgb.1 * pct / 100.0, rgb.2 * pct / 100.0)
}

/// `<colour>!10!white` (`orchid`: `block body` bg = `block title.bg!10!bg`
/// on the white page).
const fn tint(rgb: Rgb, pct: f64) -> Rgb {
    (
        rgb.0 * pct / 100.0 + (100.0 - pct) / 100.0,
        rgb.1 * pct / 100.0 + (100.0 - pct) / 100.0,
        rgb.2 * pct / 100.0 + (100.0 - pct) / 100.0,
    )
}

/// `beamercolorthemewhale.sty`: `palette primary` bg = `structure.fg`,
/// `secondary` = `structure.fg!75!black`, `tertiary` = `!50!black`, every
/// fg white. Measured in the corpus footline: `0.2 0.2 0.7`, `0.15 0.15
/// 0.525`, `0.09999 0.09999 0.34999 rg`.
pub const PALETTE_PRIMARY_BG: Rgb = STRUCTURE_RGB;
pub const PALETTE_SECONDARY_BG: Rgb = shade(STRUCTURE_RGB, 75.0);
pub const PALETTE_TERTIARY_BG: Rgb = shade(STRUCTURE_RGB, 50.0);
pub const WHITE: Rgb = (1.0, 1.0, 1.0);

/// Which theme the deck loaded (`\usetheme{..}`): the ones this crate
/// models. Any other name keeps the default theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeKind {
    #[default]
    Default,
    /// `beamerthemeMadrid.sty`: `whale` + `orchid` colours, `rounded`
    /// inner theme (with `shadow=true`), `infolines` outer theme with the
    /// `headline` reset to the default (empty) one, text margins 1em.
    Madrid,
}

impl ThemeKind {
    pub fn parse(name: &str) -> ThemeKind {
        match name.trim() {
            "Madrid" => ThemeKind::Madrid,
            _ => ThemeKind::Default,
        }
    }
}

/// One `beamercolorbox` of the infolines footline
/// (`beamerouterthemeinfolines.sty` 41-58): `wd=.333333\paperwidth,
/// ht=2.25ex, dp=1ex`, centred unless `leftskip`/`rightskip` are given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootBox {
    /// `.333333\paperwidth`.
    pub width: Sp,
    pub bg: Rgb,
    pub fg: Rgb,
    pub content: FootContent,
    /// `leftskip`/`rightskip` natural parts (`2ex` for the date box);
    /// the centred boxes have `0pt plus1fill` both sides.
    pub leftskip: Sp,
    pub rightskip: Sp,
    /// Whether the skips carry `plus1fill` (the `center` key).
    pub centered: bool,
}

/// What each footline box sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootContent {
    /// `\insertshortauthor\expandafter\ifblank...{~~(\insertshortinstitute)}`
    /// (`author in head/foot`, palette tertiary).
    AuthorInstitute,
    /// `\insertshorttitle` (`title in head/foot`, palette secondary).
    Title,
    /// `\hfill\insertshortdate{}\hfill` + `page number in head/foot`
    /// `[totalframenumber]` (`date in head/foot`, palette primary): the
    /// date and `\makebox[<wd of T\,/\,T>][r]{n\,/\,T}` with the two
    /// `\hfill`s sharing the width between `leftskip` and `rightskip`.
    DateFrameNumber,
}

/// The infolines footline: an `\hbox` of three boxes at the paper's
/// bottom edge (the footline is set below `\textheight`;
/// `\beamer@calculateheadfoot`: `\footheight` = ht + dp + 4pt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footline {
    /// `ht=2.25ex` in the footline font.
    pub height: Sp,
    /// `dp=1ex`: the text baseline is this far above the paper bottom.
    /// Measured: 269.469bp from the top of a 272.126bp page.
    pub depth: Sp,
    pub font: FontSpec,
    pub boxes: [FootBox; 3],
}

impl Footline {
    /// `\footheight`: the box plus 4pt. Measured `\footheight` 12.66663pt,
    /// `\textheight` 260.48pt (273.14662 − 12.66663).
    pub fn footheight(&self) -> Sp {
        self.height + self.depth + Sp::pt(4)
    }
}

/// `beamerboxesrounded` around the title page's `title` colour box
/// (`beamerinnerthemerounded.sty`: `title page` `[default][colsep=-4bp,
/// rounded=true,shadow=..]`; `beamerbaseboxes.sty` 36-252 with an empty
/// head): the colour box's own `\hbox` sits inside a `\vbox` of
/// `\vskip4bp`, an empty head `\hbox(1.5pt)`, `\vskip-1pt`, the lower
/// `minipage` (`\vskip2pt` + the box, raised 0.5pt) and `\vskip4bp`, so
/// it gains `above` over the box top and `below` under its bottom. The
/// painted rounded rectangle overhangs `\textwidth` by 4bp each side and
/// starts/ends `inset` (1bp) inside the vbox's top and bottom (head path
/// top `3bp` above the head box's top, lower path `3bp` under its
/// baseline). Measured (corpus title page): the fill spans 59.758bp to
/// 113.832bp from the page top, x 6.909bp to 355.926bp; the title
/// baseline moved up 2.23bp and the author down 4.49bp relative to the
/// default template.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundedBox {
    pub above: Sp,
    pub below: Sp,
    pub overhang: Sp,
    pub inset: Sp,
    pub bg: Rgb,
    pub fg: Rgb,
}

/// The rounded inner theme's `blocks` `[rounded]` template
/// (`beamerinnerthemedefault.sty` `block begin`/`block end` `[rounded]`:
/// a `beamerboxesrounded[upper=block title,lower=block body,shadow=..]`
/// whose head is the `\large` title): the colours the orchid theme gives
/// the title and body boxes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundedBlocks {
    pub title_bg: Rgb,
    pub title_fg: Rgb,
    pub body_bg: Rgb,
    pub alert_title_bg: Rgb,
    pub alert_body_bg: Rgb,
    pub example_title_bg: Rgb,
    pub example_body_bg: Rgb,
}

/// Everything a theme changes that the render pipeline lays out. Every
/// number is transcribed from the theme's `.sty` and checked against the
/// corpus deck `fixtures/real-world/beamer-madrid` (see the item docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub kind: ThemeKind,
    /// `\beamer@leftmargin` / `\beamer@rightmargin`
    /// (`\setbeamersize{text margin left=..}`): 1cm in the default outer
    /// theme, `1em` (10.95pt) in infolines. Measured Madrid body x =
    /// 10.909bp, `\textwidth` 342.2953pt.
    pub text_margin_left: Sp,
    pub text_margin_right: Sp,
    /// The `frametitle` beamercolor's background (`titlelike` parent
    /// `palette primary` in whale): the frametitle `beamercolorbox` is
    /// painted `\paperwidth` wide, and `\nointerlineskip` drops the
    /// `\lineskip` before it and `\vskip-.3cm` is skipped
    /// (`beamerouterthemedefault.sty` 164, 180). `None` paints nothing.
    pub frametitle_bg: Option<Rgb>,
    pub frametitle_fg: Rgb,
    pub footline: Option<Footline>,
    pub title_page: Option<RoundedBox>,
    /// `\setbeamertemplate{items}[ball]`: itemize labels are a shaded
    /// ball (a pgf radial shading, not drawn here) and enumerate labels
    /// the number in white `\tiny` over a ball.
    pub ball_items: bool,
    pub blocks: Option<RoundedBlocks>,
}

/// The theme table.
pub fn theme(kind: ThemeKind) -> Theme {
    let cm = len("1cm");
    match kind {
        ThemeKind::Default => Theme {
            kind,
            text_margin_left: cm,
            text_margin_right: cm,
            frametitle_bg: None,
            frametitle_fg: STRUCTURE_RGB,
            footline: None,
            title_page: None,
            ball_items: false,
            blocks: None,
        },
        ThemeKind::Madrid => {
            let em = crate::class::body_font(BaseSize::Pt11).em;
            let third = footline_box_width(len("128mm"));
            let two_ex = TINY_EX.times(2);
            let boxed = |bg: Rgb, content: FootContent, skips: Option<Sp>| FootBox {
                width: third,
                bg,
                fg: WHITE,
                content,
                leftskip: skips.unwrap_or(Sp::ZERO),
                rightskip: skips.unwrap_or(Sp::ZERO),
                centered: skips.is_none(),
            };
            Theme {
                kind,
                text_margin_left: em,
                text_margin_right: em,
                frametitle_bg: Some(PALETTE_PRIMARY_BG),
                frametitle_fg: WHITE,
                footline: Some(Footline {
                    height: TINY_EX.scaled("2.25").unwrap(),
                    depth: TINY_EX,
                    font: TINY,
                    boxes: [
                        boxed(PALETTE_TERTIARY_BG, FootContent::AuthorInstitute, None),
                        boxed(PALETTE_SECONDARY_BG, FootContent::Title, None),
                        boxed(PALETTE_PRIMARY_BG, FootContent::DateFrameNumber, Some(two_ex)),
                    ],
                }),
                title_page: Some(RoundedBox {
                    above: len("4bp") + len("0.5pt") + Sp::pt(2),
                    below: len("0.5pt") + len("4bp"),
                    overhang: len("4bp"),
                    inset: len("1bp"),
                    bg: PALETTE_PRIMARY_BG,
                    fg: WHITE,
                }),
                ball_items: true,
                blocks: Some(RoundedBlocks {
                    title_bg: shade(STRUCTURE_RGB, 75.0),
                    title_fg: WHITE,
                    body_bg: tint(shade(STRUCTURE_RGB, 75.0), 10.0),
                    alert_title_bg: shade(ALERT_RGB, 75.0),
                    alert_body_bg: tint(shade(ALERT_RGB, 75.0), 10.0),
                    example_title_bg: shade(EXAMPLE_RGB, 75.0),
                    example_body_bg: tint(shade(EXAMPLE_RGB, 75.0), 10.0),
                }),
            }
        }
    }
}

/// The theme's page parameters over `beamer_params`'s
/// (`beamerbaseframecomponents.sty` `\beamer@calculateheadfoot`, and
/// `\setbeamersize{text margin left/right}`): `\textwidth` = paper less
/// the two text margins (the side margin is the left one), `\footskip` =
/// `\footheight` (the footline box + 4pt) and `\textheight` = paper −
/// footheight − headheight (0: Madrid's headline is the default empty
/// one). Measured (Madrid, `\the`): `\textwidth` 342.2953pt, `\textheight`
/// 260.48pt, `\footheight` 12.66663pt; body x 10.909bp.
pub fn apply_theme(params: &mut crate::class::PageParams, theme: &Theme) {
    if theme.kind == ThemeKind::Default {
        // `beamer_params` already models the default outer theme's 1cm
        // margins (its `2cm` scanned once, as TeX's `{-2cm}` is).
        return;
    }
    let inch = len("1in");
    params.textwidth = params.paperwidth - theme.text_margin_left - theme.text_margin_right;
    params.oddsidemargin = theme.text_margin_left - inch;
    params.evensidemargin = theme.text_margin_left - inch;
    if let Some(foot) = theme.footline {
        params.footskip = foot.footheight();
        params.textheight = params.paperheight - foot.footheight();
    }
}

/// `.333333\paperwidth`: a footline box's width (the theme table is built
/// for the default 128mm paper; other aspect ratios rescale from here).
pub fn footline_box_width(paperwidth: Sp) -> Sp {
    paperwidth.scaled("0.333333").unwrap()
}

/// [`frametitle_box`] under a theme whose `frametitle` colour has a
/// background: `\nointerlineskip` before the colour box (no `\lineskip`),
/// the `\vskip-.3cm` skipped, so the painted bar is the default's inner
/// height plus `sep` and the title baseline sits 1pt higher. Measured
/// (Madrid p2): the bar is 27.569bp tall from the page top (27.672pt =
/// 19.136 + 8.5359) and `Outline` sits at baseline 20.061bp
/// (20.136pt = 8.5359 − 6.4 + 5.4 + 12.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemedFrameTitle {
    pub geometry: FrameTitleBox,
    /// The painted bar's height from the frame (paper) top, when the
    /// theme paints one.
    pub bar_height: Option<Sp>,
}

pub fn frametitle_box_themed(paperwidth: Sp, title_lines: usize, subtitle_lines: usize, theme: &Theme) -> Option<ThemedFrameTitle> {
    let plain = frametitle_box(paperwidth, title_lines, subtitle_lines)?;
    if theme.frametitle_bg.is_none() {
        return Some(ThemedFrameTitle { geometry: plain, bar_height: None });
    }
    let sep = len("0.3cm");
    let lineskip = Sp::pt(1);
    let after = crate::class::body_font(BaseSize::Pt11).em.scaled("0.25").unwrap();
    // The default's box less its `\lineskip` and trailing `0.25em`, plus
    // the `sep` that `\vskip-.3cm` no longer cancels.
    let bar = plain.height - lineskip - after + sep;
    Some(ThemedFrameTitle {
        geometry: FrameTitleBox {
            height: bar + after,
            title_baseline: plain.title_baseline - lineskip,
            subtitle_baseline: plain.subtitle_baseline.map(|b| b - lineskip),
            ..plain
        },
        bar_height: Some(bar),
    })
}

/// `[allowframebreaks]` (`beamerbaseframesize.sty` 212-243): the frame's
/// vertical list is `\vsplit` to `factor × \textheight` (0.95 by
/// default) at the last feasible break, the split part is `\unvbox`ed
/// into a `\vbox to\textheight` with the `autobreak` skips
/// (`beamerbaseframe.sty` 256-260: `[c]` gives `0pt plus .4\paperheight`
/// above and `0pt plus .6\paperheight` below — finite, so the body's own
/// stretch (`\itemsep` `plus 2pt`) takes its share), and the remainder
/// starts the next frame after `\frametitle{<title> II}` under a
/// `\splittopskip` of `\baselineskip`. Measured (corpus p4/p5): 14 items
/// on the first page with `glue set 0.04607`, 6 on the second with
/// `0.50691`; item pitch 16.63bp on p4 (16.6pt + 2pt x 0.046).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoBreak {
    /// `\beamer@autobreakfactor`.
    pub factor: f64,
    /// `\beamer@frametopskipautobreak` stretch (`.4\paperheight` for `[c]`).
    pub top_stretch: Sp,
    /// `\beamer@framebottomskipautobreak` stretch (`.6\paperheight`).
    pub bottom_stretch: Sp,
    /// The natural part of the top skip (`[t]`: `.2cm`).
    pub top: Sp,
    /// `\splittopskip` for the continuation's first box: `\baselineskip`.
    pub splittopskip: Sp,
    /// The top skip's stretch is `1fill` (`[b]`): it takes the whole free
    /// height and every finite stretch stays at its natural size (TeX
    /// §659: the highest order of infinity alone sets). Measured (probe
    /// deck `beamer-polish`, `[t,allowframebreaks]` p9/p10): items
    /// 16.538bp apart, the natural 16.6pt, on both pages.
    pub top_fill: bool,
    /// The bottom skip's stretch is `1fill` (`[t]`).
    pub bottom_fill: bool,
}

pub fn autobreak(align: FrameAlign, paperheight: Sp) -> AutoBreak {
    let splittopskip = NORMAL.baselineskip;
    match align {
        FrameAlign::Center => AutoBreak {
            factor: 0.95,
            top_stretch: paperheight.scaled("0.4").unwrap(),
            bottom_stretch: paperheight.scaled("0.6").unwrap(),
            top: Sp::ZERO,
            splittopskip,
            top_fill: false,
            bottom_fill: false,
        },
        // `[t]`/`[b]` copy `\beamer@frametopskip`/`bottomskip`
        // (`beamerbaseframe.sty` 263-272): `[t]`'s `.2cm plus
        // .5\paperheight` above and `0pt plus 1fill` below; `[b]` `0pt
        // plus 1fill` above and nothing below. The fill's stretch is given
        // as one far larger than any finite one for the `\vsplit`'s
        // badness (0, as for any infinite stretch); the box's glue setting
        // reads the flags.
        FrameAlign::Top => AutoBreak {
            factor: 0.95,
            top_stretch: paperheight.scaled("0.5").unwrap(),
            bottom_stretch: Sp::pt(1_000_000),
            top: len("0.2cm"),
            splittopskip,
            top_fill: false,
            bottom_fill: true,
        },
        FrameAlign::Bottom => AutoBreak {
            factor: 0.95,
            top_stretch: Sp::pt(1_000_000),
            bottom_stretch: Sp::ZERO,
            top: Sp::ZERO,
            splittopskip,
            top_fill: true,
            bottom_fill: false,
        },
    }
}

/// The continuation suffix of a broken frame's title: `frametitle
/// continuation` `[default]` = `\insertcontinuationcountroman`
/// (`\@Roman\beamer@autobreakcount`), which `\insertframetitle` appends
/// after a blank on every page of an `[allowframebreaks]` frame (the
/// first too). Measured: "A long list I", "A long list II".
pub fn continuation_suffix(count: usize) -> String {
    let mut n = count;
    let mut out = String::new();
    for (value, numeral) in [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
        while n >= value {
            out.push_str(numeral);
            n -= value;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Polish (issue #944 leftovers): navigation symbols, `items[ball]`, covered
// text modes.
// ---------------------------------------------------------------------------

/// `navigation symbols dimmed` (`beamercolorthemedefault.sty` 117:
/// `fg=structure.fg!20!bg`): rgb 0.84,0.84,0.94, the "light" half of every
/// symbol. [`NAVIGATION_RGB`] is the strong half (`!40!bg`).
pub const NAVIGATION_DIMMED_RGB: Rgb = tint(STRUCTURE_RGB, 20.0);

/// One pgf path command of a navigation symbol, in **bp** from the
/// picture's origin, y up (`beamerbasenavigationsymbols.tex` writes its
/// coordinates in bp; the few `pt` ones are converted here).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NavCmd {
    Move(f64, f64),
    Line(f64, f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    Close,
}

/// One `\pgfusepathqfill` / `\pgfusepathqstroke` of a navigation symbol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavPath {
    /// Filled (`true`) or stroked.
    pub fill: bool,
    /// `\pgfsetlinewidth` in pt (0.4 is pgf's default).
    pub line_width_pt: f64,
    /// `\pgfsetroundcap` (the back/forward arrows).
    pub round_cap: bool,
    /// Painted in the `navigation symbols dimmed` colour (the "light"
    /// object) rather than `navigation symbols`.
    pub dimmed: bool,
    pub commands: &'static [NavCmd],
}

const fn stroke(commands: &'static [NavCmd], line_width_pt: f64, dimmed: bool) -> NavPath {
    NavPath { fill: false, line_width_pt, round_cap: false, dimmed, commands }
}

const fn fill(commands: &'static [NavCmd], dimmed: bool) -> NavPath {
    NavPath { fill: true, line_width_pt: 0.4, round_cap: false, dimmed, commands }
}

/// `1pt` in bp: the few `\pgfpoint{..pt}{..pt}` coordinates of the symbols.
const BP_PER_PT: f64 = 72.0 / 72.27;

use NavCmd::{Cubic, Line, Move};

/// The two small triangles every "light" object draws.
const NAV_TRIANGLES: [NavCmd; 6] = [Move(4.0, 0.5), Line(2.0, 2.0), Line(4.0, 3.5), Move(16.0, 0.5), Line(18.0, 2.0), Line(16.0, 3.5)];

/// `\insertslidenavigationsymbol`: `beamerslidenavstrong` (a stroked
/// `8.3pt,0.8pt` + `3.4pt x 2.4pt` rectangle) then `beamerslidenavlight`
/// (the triangles, filled).
const SLIDE_NAV: [NavPath; 2] = [
    stroke(
        &[
            Move(8.3 * BP_PER_PT, 0.8 * BP_PER_PT),
            Line(8.3 * BP_PER_PT + 3.4 * BP_PER_PT, 0.8 * BP_PER_PT),
            Line(8.3 * BP_PER_PT + 3.4 * BP_PER_PT, 0.8 * BP_PER_PT + 2.4 * BP_PER_PT),
            Line(8.3 * BP_PER_PT, 0.8 * BP_PER_PT + 2.4 * BP_PER_PT),
            NavCmd::Close,
        ],
        0.4,
        false,
    ),
    fill(&NAV_TRIANGLES, true),
];

/// `\insertframenavigationsymbol`: three stacked frames stroked, then the
/// triangles.
const FRAME_NAV: [NavPath; 2] = [
    stroke(
        &[
            Move(7.0 * BP_PER_PT, 0.0),
            Line(7.0 * BP_PER_PT + 3.4 * BP_PER_PT, 0.0),
            Line(7.0 * BP_PER_PT + 3.4 * BP_PER_PT, 2.4 * BP_PER_PT),
            Line(7.0 * BP_PER_PT, 2.4 * BP_PER_PT),
            NavCmd::Close,
            Move(7.8, 2.4),
            Line(7.8, 3.2),
            Line(11.2, 3.2),
            Line(11.2, 0.8),
            Line(10.4, 0.8),
            Move(8.6, 3.2),
            Line(8.6, 4.0),
            Line(12.0, 4.0),
            Line(12.0, 1.6),
            Line(11.2, 1.6),
        ],
        0.4,
        false,
    ),
    fill(&NAV_TRIANGLES, true),
];

/// `\insertsubsectionnavigationsymbol`: one strong 0.6pt line, then the
/// triangles and four dimmed lines.
const SUBSECTION_NAV: [NavPath; 3] = [
    stroke(&[Move(9.0, 3.0), Line(12.0, 3.0)], 0.6, false),
    fill(&NAV_TRIANGLES, true),
    stroke(&[Move(8.0, 4.0), Line(11.0, 4.0), Move(9.0, 2.0), Line(12.0, 2.0), Move(8.0, 1.0), Line(11.0, 1.0), Move(9.0, 0.0), Line(12.0, 0.0)], 0.6, true),
];

/// `\insertsectionnavigationsymbol`: three strong lines, then the
/// triangles and two dimmed lines.
const SECTION_NAV: [NavPath; 3] = [
    stroke(&[Move(8.0, 4.0), Line(11.0, 4.0), Move(9.0, 3.0), Line(12.0, 3.0), Move(9.0, 2.0), Line(12.0, 2.0)], 0.6, false),
    fill(&NAV_TRIANGLES, true),
    stroke(&[Move(8.0, 1.0), Line(11.0, 1.0), Move(9.0, 0.0), Line(12.0, 0.0)], 0.6, true),
];

/// `\insertdocnavigationsymbol` without an appendix
/// (`beamerdocnavstrongsingle`): five strong lines.
const DOC_NAV: [NavPath; 1] = [stroke(
    &[Move(8.0, 4.0), Line(11.0, 4.0), Move(9.0, 3.0), Line(12.0, 3.0), Move(9.0, 2.0), Line(12.0, 2.0), Move(8.0, 1.0), Line(11.0, 1.0), Move(9.0, 0.0), Line(12.0, 0.0)],
    0.6,
    false,
)];

/// `\pgfpathcircle{\pgfpoint{9.5pt}{2.5pt}}{1.2pt}` as pgf writes it: four
/// cubics with the 0.5523 tangent factor.
const fn circle_bp(cx: f64, cy: f64, r: f64) -> [NavCmd; 6] {
    const K: f64 = 0.5522847;
    [
        Move(cx + r, cy),
        Cubic(cx + r, cy + K * r, cx + K * r, cy + r, cx, cy + r),
        Cubic(cx - K * r, cy + r, cx - r, cy + K * r, cx - r, cy),
        Cubic(cx - r, cy - K * r, cx - K * r, cy - r, cx, cy - r),
        Cubic(cx + K * r, cy - r, cx + r, cy - K * r, cx + r, cy),
        NavCmd::Close,
    ]
}

const SEARCH_CIRCLE: [NavCmd; 6] = circle_bp(9.5 * BP_PER_PT, 2.5 * BP_PER_PT, 1.2 * BP_PER_PT);

/// `\insertbackfindforwardnavigationsymbol`: the search handle (0.6pt),
/// the search circle (0.4pt), then the two round-capped arrows.
const BACK_FIND_FORWARD_NAV: [NavPath; 3] = [
    stroke(&[Move(10.4, 1.6), Line(12.0, 0.0)], 0.6, false),
    stroke(&SEARCH_CIRCLE, 0.4, false),
    NavPath {
        fill: false,
        line_width_pt: 0.4,
        round_cap: true,
        dimmed: false,
        commands: &[
            Move(4.0, 0.0),
            Cubic(5.1 * BP_PER_PT, 0.0, 6.0, 0.9, 6.0, 2.0),
            Cubic(6.0, 3.1, 5.1, 4.0, 4.0, 4.0),
            Cubic(2.9, 4.0, 2.0, 3.1, 2.0, 2.0),
            Move(3.2, 2.6),
            Line(2.0, 1.6),
            Line(0.8, 2.6),
            Move(16.0, 0.0),
            Cubic(14.9, 0.0, 14.0, 0.9, 14.0, 2.0),
            Cubic(14.0, 3.1, 14.9, 4.0, 16.0, 4.0),
            Cubic(17.1, 4.0, 18.0, 3.1, 18.0, 2.0),
            Move(19.2, 2.6),
            Line(18.0, 1.6),
            Line(16.8, 2.6),
        ],
    },
];

/// The six pictures of the default `navigation symbols` template
/// (`beamerouterthemedefault.sty` 70-80), left to right: slide, frame,
/// subsection, section, document, back/find/forward. Each is a
/// `pgfpicture{0pt}{-1.5pt}{20pt}{5.5pt}` whose paths are
/// `beamerbasenavigationsymbols.tex`'s, transcribed with the reference
/// content stream (`fixtures/real-world/beamer-default`, every page) as
/// the check: the same operators, coordinates and line widths in the
/// same order.
pub const NAVIGATION_SYMBOLS: [&[NavPath]; 6] = [&SLIDE_NAV, &FRAME_NAV, &SUBSECTION_NAV, &SECTION_NAV, &DOC_NAV, &BACK_FIND_FORWARD_NAV];

/// Where the navigation symbol strip sits on a frame page
/// (`beamerouterthemedefault.sty` 138-145, `sidebar right` `[default]`:
/// `\vfill \llap{\insertlogo\hskip0.1cm} \vskip2pt \llap{\usebeamertemplate
/// ***{navigation symbols}\hskip0.1cm} \vskip2pt` in a `\vbox to
/// \sidebarheight` whose bottom is `\footheight - 4pt` (the footline box)
/// above the paper's bottom edge). The template is an `\hbox` of six
/// `\hbox{<picture>}`es separated by interword glue of the font in force
/// there, `\Tiny` (`cmss8 at 4pt`, `\fontdimen2` 1.41663pt); each picture
/// is `20pt` wide with its baseline `1.5pt` above its bounding box.
///
/// Measured (pdflatex TeX Live 2026, `fixtures/real-world/beamer-default`
/// p2 and `beamer-madrid` p3 content streams): the pictures' origins are
/// `1 0 0 1 233.391 3.487 cm` then five `21.337`/`21.336 cm` steps
/// (Madrid: `233.391 12.121`), i.e. the strip's right edge is `0.1cm`
/// from the paper's right edge and its baseline `2pt + 1.5pt` above the
/// footline box (`3.5pt` = 3.487bp; Madrid `8.66663pt + 3.5pt` =
/// 12.121bp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationStrip {
    /// Each picture's width (`20pt`).
    pub picture_width: Sp,
    /// The glue between two pictures (`\Tiny` interword space, natural).
    pub gap: Sp,
    /// The strip's right edge from the paper's right edge (`0.1cm`).
    pub right_inset: Sp,
    /// The pictures' baseline above the paper's bottom edge.
    pub baseline_above_bottom: Sp,
    /// A picture's bounding box above (`5.5pt`) and below (`1.5pt`) its
    /// baseline.
    pub height: Sp,
    pub depth: Sp,
}

impl NavigationStrip {
    /// The whole strip's width: six pictures and five gaps.
    pub fn width(&self) -> Sp {
        self.picture_width.times(6) + self.gap.times(5)
    }
}

/// `\fontdimen2` of `cmss8 at 4pt`: the TFM's space `0.35417175` (fix_word
/// 371360) scaled to 4pt = 92840sp.
pub const TINY4_SPACE: Sp = Sp(92_840);

pub fn navigation_strip(theme: &Theme) -> NavigationStrip {
    let foot_box = theme.footline.map_or(Sp::ZERO, |f| f.height + f.depth);
    NavigationStrip {
        picture_width: Sp::pt(20),
        gap: TINY4_SPACE,
        right_inset: len("0.1cm"),
        baseline_above_bottom: foot_box + Sp::pt(2) + len("1.5pt"),
        height: len("5.5pt"),
        depth: len("1.5pt"),
    }
}

/// `\setbeamercovered{<spec>}` (`beamerbaseoverlay.sty` 360-388): how
/// covered material (`\uncover`, `\pause`, `\item<2->`, ...) is painted
/// on the slides it is covered on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Covered {
    /// `invisible` (the default): `\pgfsys@begininvisible`, nothing is
    /// painted.
    #[default]
    Invisible,
    /// `transparent[=<pct>]` (15 by default): `\opaqueness<1->{<pct>}`
    /// mixes the current colour `<pct>!bg` (`\beamer@colorhook`), so
    /// black text on the white page is painted `pct`% black. `dynamic`
    /// and `highly dynamic` are taken as their first step (10 and 15):
    /// their fade with the distance to the uncovering slide is not
    /// modelled.
    Transparent(u8),
}

impl Covered {
    /// The value of one `\setbeamercovered{..}` argument (a key list;
    /// `invisible` is always applied first, then the keys in order).
    pub fn parse(spec: &str) -> Covered {
        let mut mode = Covered::Invisible;
        for key in spec.split(',') {
            let key = key.trim();
            let (name, value) = match key.split_once('=') {
                Some((n, v)) => (n.trim(), Some(v.trim())),
                None => (key, None),
            };
            mode = match name {
                "invisible" => Covered::Invisible,
                "transparent" => Covered::Transparent(value.and_then(|v| v.parse::<u8>().ok()).unwrap_or(15).min(100)),
                "dynamic" => Covered::Transparent(10),
                "highly dynamic" => Covered::Transparent(15),
                // `still covered=`/`again covered=` take arbitrary actions:
                // not modelled.
                _ => mode,
            };
        }
        mode
    }
}

/// `items[ball]` (`beamerbaseauxtemplates.sty` 35-40, 366, 374-384): the
/// `bigsphere` radial shading is a `2 x 0.53ex` square (`\normalsize`
/// ex, 4.86665pt: 5.139bp, the XObject's `/BBox [0 0 5.139 5.139]` on
/// Madrid p3) whose colour runs from `bg!15` at the (off-centre) focus
/// through `bg!75`, `bg!70!black`, `bg!50!black` at `0.452ex` to
/// `parent.bg` (white) at `0.53ex`, `bg` being `item projected`'s
/// background = `structure.fg`. The pipeline paints a flat disc instead:
/// its radius is the midpoint of the fade band (`(0.452 + 0.53) / 2 ex`),
/// its colour the area-weighted mean of the shading inside the last
/// opaque stop (numerically, on the reference's `/ShadingType 3`:
/// `0.632 bg + 0.113 white + 0.256 black`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    /// The shading square's side (`2 x 0.53ex`): the label box's width.
    pub side: Sp,
    /// The painted disc's radius.
    pub radius: Sp,
    pub color: Rgb,
}

pub fn ball(structure: Rgb) -> Ball {
    let ex = SANS_BODY_EX;
    Ball {
        side: ex.scaled("1.06").unwrap(),
        radius: ex.scaled("0.491").unwrap(),
        color: (structure.0 * 0.632 + 0.113, structure.1 * 0.632 + 0.113, structure.2 * 0.632 + 0.113),
    }
}

/// `[plain]` (`beamerbaseframe.sty` 244-246, 781-783): the frame keeps
/// `\vbox to\textheight` but its entry code is `\vspace*{-\headheight}` and
/// its exit code `\vspace*{-\footheight}` (a zero-height rule and the
/// negative skip inside the box, so the fills gain `\headheight +
/// \footheight` of free height), and the body's own `\vbox{}` is followed
/// by `\nointerlineskip` (line 116), so the first body line carries no
/// interline glue. Measured (default theme, `\footheight` 4pt): the
/// two-line corpus frame's first baseline is 120.80bp against 123.64 for
/// the same body on a normal frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainFrame {
    /// `-\headheight` at the top.
    pub head: Sp,
    /// `-\footheight` at the bottom.
    pub foot: Sp,
}

pub fn plain_frame(theme: &Theme) -> PlainFrame {
    let foot = theme.footline.map_or(Sp::pt(4), |f| f.footheight());
    PlainFrame { head: Sp::ZERO, foot }
}


// ---------------------------------------------------------------------------
// Polish (issue #944 leftovers): `beamerboxesrounded` corners and shadow.
// ---------------------------------------------------------------------------

/// The corner radius of a `beamerboxesrounded` head or body path
/// (`beamerbaseboxes.sty` 76-82 and 227-233): the head path is
/// `\pgfpathqmoveto{-4bp}{-1bp} \pgfpathqcurveto{-4bp}{1.2bp}{-2.2bp}{3bp}
/// {0bp}{3bp}` then the top edge to `\bmb@width`, the mirrored arc through
/// `(w+2.2bp, 3bp)`, `(w+4bp, 1.2bp)` down to `(w+4bp, -1bp)` -- a 4bp
/// quarter circle whose cubic tangents are 2.2bp (pgf's 0.55 x r). The
/// body path is the same shape mirrored below its origin. Measured (Madrid
/// probe deck, content stream): `-4.00005 -1.0 m -4.00005 1.2 -2.20001
/// 3.00003 0.0 3.00003 c 341.02087 3.00003 l 343.2209 3.00003 345.02094
/// 1.2 345.02094 -1.0 c`.
pub fn rounded_corner_radius() -> Sp {
    len("4bp")
}

/// The cubic tangent length of the corner arc (`2.2bp`).
pub fn rounded_corner_tangent() -> Sp {
    len("2.2bp")
}

/// The 2pt colour transition between a rounded block's head and body
/// (`beamerbaseboxes.sty` 108-119, 257-258): a 6pt `pgfpicture` on the
/// vertical list right after the head box (`\nointerlineskip\vskip-1pt`
/// before it, `\nointerlineskip\vskip-0.5pt` after) holding the
/// `bmb@transition` vertical shading -- `color(0pt)=(lower.bg);
/// color(2pt)=(lower.bg); color(4pt)=(upper.bg)` -- `\pgftext[left,base]`
/// on the picture's baseline: `lower.bg` from that baseline to 2pt above
/// it, then a linear fade to `upper.bg` at 4pt. The picture's baseline is
/// `-1pt + 6pt` under the head box's baseline, so the fade spans 1pt to
/// 3pt above the head box's baseline; the head fill (painted first) ends
/// 2pt below it and the body box starts `-0.5pt` above the picture's
/// baseline. Measured (probe): the picture at `1 0 0 1 10.909 181.765 cm`
/// under a title baseline of 188.241 raised 1.5pt: 4.981bp = 5pt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    /// The picture's baseline below the head box's baseline (`5pt`).
    pub baseline_below_head: Sp,
    /// The fade's bottom edge above the picture's baseline (`2pt`).
    pub fade_bottom: Sp,
    /// The fade's top edge above the picture's baseline (`4pt`).
    pub fade_top: Sp,
    /// How many flat strips the pipeline paints the fade in: four, one
    /// per 144-dpi raster row of the 2pt fade (0.5pt = 0.996px), each at
    /// the mix of its midpoint.
    pub steps: usize,
    /// The body box's top above the picture's baseline (`0.5pt`).
    pub body_top_above: Sp,
}

pub fn transition() -> Transition {
    Transition { baseline_below_head: Sp::pt(5), fade_bottom: Sp::pt(2), fade_top: Sp::pt(4), steps: 4, body_top_above: len("0.5pt") }
}

/// The `shadow=true` drop shadow of a `beamerboxesrounded`
/// (`beamerbaseboxes.sty` 148-189, 205-221), read off the reference
/// content stream and its soft mask (`/pgfsmask` on a black rectangle
/// `0 -7bp (w+8bp) x (h+6bp)` in the body picture's coordinates, `w` =
/// `\bmb@width`, `h` = `\bmb@boxheight` = body box height + 4bp + head box
/// height):
///
/// - the mask is a `pgfpicture` (fading) of five shadings and a black
///   (transparent) rectangle over the box: `bmb@shadowvert` (`w-4bp` wide,
///   8bp tall, `pgftransparent!100` at its bottom to `!0` at its top)
///   `[left, at=(4bp,4bp)]`; `bmb@shadowhorz` (8bp wide, `h-5.5bp` tall,
///   opaque at its left edge to transparent at 8bp) `[base, at=(w+4bp,
///   7.5bp)]`; `bmb@shadowballlarge` (radius 8bp, opaque centre) at
///   `(w, 8bp)`; two `bmb@shadowball`s (radius 4bp, 50% at the centre)
///   at `(4bp, 4bp)` and `(w+4bp, h+2bp)`; the black rectangle `(4bp,
///   8.1bp)` to `(w+4bp, h+6.1bp)`;
/// - `\pgfsetfading{..}{\pgftransformshift{(0.5w+6bp, 0.5h-4bp)}}` puts
///   the fading's centre there; its bounding box is `0..w+12bp` (the
///   `\hskip4bp` after the picture) by `0..h+6.1bp`, so mask coordinates
///   map onto body-picture coordinates shifted by `(0, -7.05bp)`
///   (the reference writes `1.0 0.0 0.0 1.0 0.0 -7.05008 cm` around the
///   `gs`).
///
/// Composed, the alpha outside the box is `0.5 - d/8bp` at distance `d`
/// (0..4bp) from the box's bottom and right edges: the bottom shading is
/// opaque 0.95bp above the box bottom (`-3bp`) and transparent 7.05bp
/// below it, the right shading opaque at `w` and transparent at `w+8bp`
/// with the box edge at `w+4bp`, the large ball concentric with the
/// bottom-right corner arc (centre `(w, 1bp)`), and the two small balls
/// end the bottom band 4bp from the left edge and the right band `5.05bp`
/// under the box top (`h+2bp-7.05bp` above the body origin) as round
/// caps. So the shadow is a round-capped stroke of the box's bottom and
/// right edge, 4bp wide outside the box, fading from 50% black at the
/// edge to nothing. The display list has no gradient paint: the pipeline
/// paints [`Shadow::steps`] concentric opaque strokes on the white page
/// ([`shadow_rings`]), widest and lightest first, at 0.5bp per ring (one
/// 144-dpi raster pixel).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// The visible band outside the box (`4bp`).
    pub band: Sp,
    /// Alpha of the black shadow at the box edge (`0.5`).
    pub edge_alpha: f64,
    /// The right band's round cap centre, below the `\bmb@boxheight` top:
    /// `7.05bp - 2bp`.
    pub cap_below_box_top: Sp,
    /// The bottom band's round cap centre, from the box's left edge (`4bp`).
    pub cap_from_left: Sp,
    /// Concentric strokes the fade is painted in.
    pub steps: usize,
}

pub fn shadow() -> Shadow {
    Shadow { band: len("4bp"), edge_alpha: 0.5, cap_below_box_top: len("5.05bp"), cap_from_left: len("4bp"), steps: 8 }
}

/// `\bmb@boxheight` (`beamerbaseboxes.sty` 143-147): the body box's
/// height, `4bp`, the head box's height (`\bmb@prevheight`; `-4.5pt` for
/// an empty head). Measured (probe, two-line block): 25.737 + 4 + 9.797 =
/// 39.534bp, the mask's black rectangle `4.00005 8.1001 341.02087
/// 37.53387 re` being `h - 2bp` tall.
pub fn rounded_box_height(body: Sp, head: Sp) -> Sp {
    body + len("4bp") + head
}

/// `\bmb@prevheight` of a `beamerboxesrounded` with an empty head
/// (`beamerbaseboxes.sty` 56-59): `-4.5pt`, the head `\hbox{}` being
/// given `\ht 1.5pt`.
pub fn rounded_empty_head_height() -> Sp {
    Sp::ZERO - len("4.5pt")
}

/// The strokes that paint [`Shadow`] on a white page, widest first:
/// `(stroke width, grey level)`. Ring `k` covers the distances
/// `k..k+1` x `band/steps` from the edge (the stroke is centred on the
/// edge, its inner half under the box's fills); its grey is the white
/// page under black at the alpha of the ring's midpoint, `1 - edge_alpha
/// x (1 - (k + 0.5)/steps)`.
pub fn shadow_rings(shadow: &Shadow) -> Vec<(Sp, f64)> {
    let steps = shadow.steps.max(1);
    (0..steps)
        .rev()
        .map(|k| {
            let width = Sp(shadow.band.0 * 2 * (k as i64 + 1) / steps as i64);
            let alpha = shadow.edge_alpha * (1.0 - (k as f64 + 0.5) / steps as f64);
            (width, 1.0 - alpha)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_skips_are_the_templates() {
        let b = block();
        assert_eq!(b.before, Glue::new("6pt", "2pt", "2pt"));
        assert_eq!(b.after, Glue::new("3pt", "1pt", "1pt"));
        assert!((b.body_top.to_pt() + 1.21666).abs() < 0.00002, "{:?}", b.body_top);
        assert!((SANS_BODY_EX.to_pt() - 4.86665).abs() < 0.00002);
        assert!((MATH_AXIS.to_pt() - 2.7375).abs() < 0.00002);
        assert_eq!(trivlist_topsep(BaseSize::Pt11), Glue::new("9pt", "3pt", "5pt"));
    }

    #[test]
    fn column_origins_match_the_corpus_deck() {
        let tw = len("128mm") - len("2cm");
        let half = tw.scaled("0.5").unwrap();
        let x = column_origins(ColumnsBox::PaperWide, &[half, half], tw, len("1cm"), len("1cm"));
        // From the paper edge: 18.90 and 190.87bp.
        assert_eq!(bp(x[0] + len("1cm")), 18.898);
        assert_eq!(bp(x[1] + len("1cm")), 190.866);
        let y = column_origins(ColumnsBox::Fixed(tw), &[half, half], tw, len("1cm"), len("1cm"));
        assert_eq!(y[0], Sp::ZERO);
        assert_eq!(y[1], half);
        // `[onlytextwidth]` with two `.3\textwidth` columns: the second
        // starts at 242.645 - 28.346bp from the text edge (one `\hfill`).
        let third = tw.scaled("0.3").unwrap();
        let z = column_origins(ColumnsBox::Fixed(tw), &[third, third], tw, len("1cm"), len("1cm"));
        assert!((bp(z[1]) - 214.299).abs() < 0.0015, "{}", bp(z[1]));
        // `[totalwidth=6cm]` with 2cm + 3cm columns: 2cm + 1cm of fill.
        let t = column_origins(ColumnsBox::Fixed(len("6cm")), &[len("2cm"), len("3cm")], tw, len("1cm"), len("1cm"));
        assert_eq!(bp(t[1]), bp(len("3cm")));
    }

    #[test]
    fn column_boxes_by_alignment() {
        let one = Sp::pt(4) + Sp(56_797); // 4.86665pt
        let (h, d) = column_box(ColumnAlign::Center, one, one, Sp::ZERO);
        assert!((h.to_pt() - 5.17082).abs() < 0.0001, "{:?}", h);
        assert!((d.to_pt() + 0.30417).abs() < 0.0001, "{:?}", d);
        let (h, d) = column_box(ColumnAlign::TopBaseline, Sp::pt(78), Sp::ZERO, Sp::pt(2));
        assert_eq!((h, d), (Sp::ZERO, Sp::pt(78)));
        let (h, d) = column_box(ColumnAlign::Top, Sp::pt(30), Sp::pt(7), Sp::pt(2));
        assert_eq!((h, d), (Sp::pt(7), Sp::pt(23)));
        let (h, d) = column_box(ColumnAlign::Bottom, Sp::pt(30), Sp::pt(7), Sp::pt(2));
        assert_eq!((h, d), (Sp::pt(28), Sp::pt(2)));
    }

    fn bp(sp: Sp) -> f64 {
        (sp.to_bp() * 1000.0).round() / 1000.0
    }

    #[test]
    fn frametitle_box_matches_the_showbox() {
        let pw = len("128mm");
        let b = frametitle_box(pw, 1, 0).unwrap();
        // 269.14662 - 246.27312 (\beamer@frametextheight) = 22.8735pt.
        assert!((b.height.to_pt() - 22.8735).abs() < 0.0002, "{}", b.height);
        assert_eq!(bp(b.title_baseline), 21.057);
        assert_eq!(b.subtitle_baseline, None);
        assert_eq!(bp(b.text_left), 8.504);
        let s = frametitle_box(pw, 1, 1).unwrap();
        // 269.14662 - 234.27312 = 34.8735pt.
        assert!((s.height.to_pt() - 34.8735).abs() < 0.0002, "{}", s.height);
        assert_eq!(bp(s.subtitle_baseline.unwrap()), 35.104);
        assert!(frametitle_box(pw, 0, 0).is_none());
        let two = frametitle_box(pw, 2, 0).unwrap();
        assert_eq!(two.height - b.height, LARGE.baselineskip);
    }

    #[test]
    fn body_glue_by_key() {
        assert_eq!(body_glue(FrameAlign::Center), BodyGlue { above: Sp::ZERO, above_fill: 1.0, below_fill: 1.5 });
        assert_eq!(body_glue(FrameAlign::Top).above, len("0.2cm"));
        assert_eq!(body_glue(FrameAlign::Bottom).below_fill, 0.0);
    }


    #[test]
    fn madrid_theme_numbers() {
        let t = theme(ThemeKind::Madrid);
        assert_eq!(bp(t.text_margin_left), 10.909);
        let f = t.footline.unwrap();
        assert!((f.footheight().to_pt() - 12.66663).abs() < 0.00002, "{:?}", f.footheight());
        assert!(((f.height + f.depth).to_bp() - 8.634).abs() < 0.002);
        assert_eq!(bp(f.boxes[0].width), 120.943);
        assert_eq!(f.boxes[0].bg, (0.1, 0.1, 0.35));
        assert_eq!(f.boxes[1].bg, (0.15, 0.15, 0.525));
        assert_eq!(f.boxes[2].bg, (0.2, 0.2, 0.7));
        assert!((f.boxes[2].leftskip.to_pt() - 5.33331).abs() < 0.00002);
        let b = t.blocks.unwrap();
        assert!((b.body_bg.0 - 0.915).abs() < 1e-9 && (b.body_bg.2 - 0.9525).abs() < 1e-9);
        let r = t.title_page.unwrap();
        assert!((r.above.to_pt() - 6.51501).abs() < 0.001, "{:?}", r.above);
        assert!((r.below.to_pt() - 4.51501).abs() < 0.001, "{:?}", r.below);
        assert_eq!(ThemeKind::parse("Madrid"), ThemeKind::Madrid);
        assert_eq!(ThemeKind::parse("Berlin"), ThemeKind::Default);
    }

    #[test]
    fn madrid_frametitle_bar() {
        let pw = len("128mm");
        let t = frametitle_box_themed(pw, 1, 0, &theme(ThemeKind::Madrid)).unwrap();
        assert!((t.bar_height.unwrap().to_pt() - 27.6718).abs() < 0.0005, "{:?}", t.bar_height);
        assert_eq!(bp(t.bar_height.unwrap()), 27.569);
        assert_eq!(bp(t.geometry.title_baseline), 20.061);
        assert!((t.geometry.height.to_pt() - 30.4093).abs() < 0.0005);
        let d = frametitle_box_themed(pw, 1, 0, &theme(ThemeKind::Default)).unwrap();
        assert_eq!(d.bar_height, None);
        assert_eq!(d.geometry, frametitle_box(pw, 1, 0).unwrap());
    }

    #[test]
    fn autobreak_and_plain() {
        let a = autobreak(FrameAlign::Center, len("96mm"));
        assert!((a.top_stretch.to_pt() - 109.25697).abs() < 0.001, "{:?}", a.top_stretch);
        assert!((a.bottom_stretch.to_pt() - 163.88963).abs() < 0.002, "{:?}", a.bottom_stretch);
        assert!(!a.top_fill && !a.bottom_fill);
        let t = autobreak(FrameAlign::Top, len("96mm"));
        assert!((t.top.to_pt() - 5.69055).abs() < 0.001 && !t.top_fill && t.bottom_fill);
        let b = autobreak(FrameAlign::Bottom, len("96mm"));
        assert!(b.top_fill && !b.bottom_fill && b.bottom_stretch == Sp::ZERO);
        assert_eq!(continuation_suffix(1), "I");
        assert_eq!(continuation_suffix(2), "II");
        assert_eq!(continuation_suffix(4), "IV");
        assert_eq!(continuation_suffix(9), "IX");
        assert_eq!(plain_frame(&theme(ThemeKind::Default)).foot, Sp::pt(4));
        assert!((plain_frame(&theme(ThemeKind::Madrid)).foot.to_pt() - 12.66663).abs() < 0.00002);
    }

    #[test]
    fn navigation_strip_sits_where_the_reference_paints_it() {
        // beamer-default p2: `1 0 0 1 233.391 3.487 cm`, pitch 21.337bp,
        // paper 362.835bp wide.
        let paper = len("128mm");
        let s = navigation_strip(&theme(ThemeKind::Default));
        let left = paper - s.right_inset - s.width();
        assert_eq!(bp(left), 233.391);
        assert!((bp(s.picture_width + s.gap) - 21.337).abs() <= 0.001, "{}", bp(s.picture_width + s.gap));
        assert_eq!(bp(s.baseline_above_bottom), 3.487);
        // Madrid p3: `233.391 12.121`.
        let m = navigation_strip(&theme(ThemeKind::Madrid));
        assert_eq!(bp(paper - m.right_inset - m.width()), 233.391);
        assert_eq!(bp(m.baseline_above_bottom), 12.121);
        assert_eq!(NAVIGATION_SYMBOLS.len(), 6);
        assert!((NAVIGATION_DIMMED_RGB.0 - 0.84).abs() < 1e-9 && (NAVIGATION_DIMMED_RGB.2 - 0.94).abs() < 1e-9);
        assert!((NAVIGATION_RGB.0 - tint(STRUCTURE_RGB, 40.0).0).abs() < 1e-9);
    }

    #[test]
    fn covered_modes_parse_like_setbeamercovered() {
        assert_eq!(Covered::parse("invisible"), Covered::Invisible);
        assert_eq!(Covered::parse("transparent"), Covered::Transparent(15));
        assert_eq!(Covered::parse("transparent=30"), Covered::Transparent(30));
        assert_eq!(Covered::parse("dynamic"), Covered::Transparent(10));
        assert_eq!(Covered::parse("highly dynamic"), Covered::Transparent(15));
        assert_eq!(Covered::parse("transparent, invisible"), Covered::Invisible);
        assert_eq!(Covered::parse("still covered={\\opaqueness<1->{15}}"), Covered::Invisible);
    }

    #[test]
    fn ball_matches_the_shading_bbox() {
        // Madrid p3: `/BBox [0 0 5.139 5.139]`, colours from `0.2 0.2 0.7`.
        let b = ball(STRUCTURE_RGB);
        assert_eq!(bp(b.side), 5.139);
        assert!(b.radius < b.side.over(2) && b.radius.to_pt() > 0.45 * SANS_BODY_EX.to_pt());
        assert!((b.color.0 - 0.2394).abs() < 0.001 && (b.color.2 - 0.5554).abs() < 0.001, "{:?}", b.color);
    }

    #[test]
    fn list_level_one_is_beamers() {
        let l = list_level(crate::class::body_font(BaseSize::Pt11), 1);
        assert_eq!(l.leftmargin, len("21.90006pt"));
        assert_eq!(l.topsep.natural, Sp::pt(3));
        assert_eq!(l.itemsep.natural, Sp::pt(3));
        assert_eq!(l.parsep.natural, Sp::ZERO);
    }

    #[test]
    fn rounded_box_numbers() {
        assert_eq!(bp(rounded_corner_radius()), 4.0);
        assert_eq!(bp(rounded_corner_tangent()), 2.2);
        let t = transition();
        assert_eq!(t.baseline_below_head, Sp::pt(5));
        assert_eq!(t.fade_top - t.fade_bottom, Sp::pt(2));
        // Probe: body 25.737bp, head 9.83331pt = 9.797bp.
        let h = rounded_box_height(len("25.737bp"), len("9.83331pt"));
        assert!((bp(h) - 39.534).abs() < 0.002, "{}", bp(h));
        let rings = shadow_rings(&shadow());
        assert_eq!(rings.len(), 8);
        assert_eq!(bp(rings[0].0), 8.0);
        assert!((rings[0].1 - (1.0 - 0.03125)).abs() < 1e-9);
        assert_eq!(bp(rings[7].0), 1.0);
        assert!((rings[7].1 - (1.0 - 0.46875)).abs() < 1e-9);
        assert!((rounded_empty_head_height().to_pt() + 4.5).abs() < 1e-6);
    }
}
