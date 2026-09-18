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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn list_level_one_is_beamers() {
        let l = list_level(crate::class::body_font(BaseSize::Pt11), 1);
        assert_eq!(l.leftmargin, len("21.90006pt"));
        assert_eq!(l.topsep.natural, Sp::pt(3));
        assert_eq!(l.itemsep.natural, Sp::pt(3));
        assert_eq!(l.parsep.natural, Sp::ZERO);
    }
}
