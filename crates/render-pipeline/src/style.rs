//! Document style: the `article` class geometry, sizes and spacing.
//!
//! Numbers come from the `document-style` sibling (transcribed from
//! `article.cls` / `size1x.clo` / `geometry.sty` and cross-checked against
//! pdflatex there); font-dependent quantities (`ex`, interword glue) come
//! from the TFM parameters of the face in use (`params.rs`), which is what
//! `\@startsection`'s `ex` skips evaluate to in LaTeX. All lengths are TeX
//! points (72.27/in).

use flashtex_class_geometry::{ResolvedDocument, Sp};
use flashtex_document_style::{BaseSize, Block, ClassOptions, Geometry, Paper, Stylesheet as DsStylesheet};

use crate::fonts::Family;
use crate::params;

/// A vertical skip with TeX-style stretch and shrink, in points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Skip {
    pub natural: f64,
    pub stretch: f64,
    pub shrink: f64,
}

impl Skip {
    pub const fn fixed(natural: f64) -> Skip {
        Skip {
            natural,
            stretch: 0.0,
            shrink: 0.0,
        }
    }
    pub const fn new(natural: f64, stretch: f64, shrink: f64) -> Skip {
        Skip {
            natural,
            stretch,
            shrink,
        }
    }
    pub fn glue(self) -> flashtex_paragraph_layout::Glue {
        flashtex_paragraph_layout::Glue::finite(self.natural, self.stretch, self.shrink)
    }
}

/// One heading level, resolved to absolute points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadingStyle {
    pub size_pt: f64,
    pub baselineskip_pt: f64,
    pub bold: bool,
    pub before: Skip,
    pub after: Skip,
    /// `\@startsection`'s `#5` when it is negative: the heading runs into
    /// the following paragraph and `\@xsect` puts `\hskip -#5` (this many
    /// `em` of the heading font) after it instead of vertical glue.
    /// `None` for a display heading, which uses [`Self::after`].
    /// article.cls: `\paragraph`/`\subparagraph` are `-1em`.
    pub run_in_after_em: Option<f64>,
}

/// `\usepackage[...]{microtype}` as pdfTeX sees it: the package options
/// `crates/microtype` resolves each font's codes with, and the
/// `\pdfprotrudechars` / `\pdfadjustspacing` levels it sets (2 for
/// `true`/`nocompatibility`, 1 for `compatibility`, 0 when off or `draft`).
#[derive(Debug, Clone, PartialEq)]
pub struct MicrotypeSetup {
    pub options: flashtex_microtype::Options,
    pub protrude_chars: i32,
    pub adjust_spacing: i32,
}

impl MicrotypeSetup {
    /// Whether either feature is on.
    pub fn active(&self) -> bool {
        self.protrude_chars > 0 || self.adjust_spacing > 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub family: Family,
    /// The font definition files text shapes are selected from (`nfss`);
    /// the adapter sets it from the document's packages and encoding.
    pub nfss: crate::nfss::Scheme,
    pub base: BaseSize,
    /// Paper size in TeX points (US Letter: 614.295 x 794.97).
    pub page_width_pt: f64,
    pub page_height_pt: f64,
    /// Text area (`\textwidth` x `\textheight`) with its top-left corner.
    pub text_x_pt: f64,
    pub text_y_pt: f64,
    pub text_width_pt: f64,
    pub text_height_pt: f64,
    pub body_size_pt: f64,
    pub baselineskip_pt: f64,
    pub lineskip_pt: f64,
    pub lineskiplimit_pt: f64,
    pub topskip_pt: f64,
    pub maxdepth_pt: f64,
    pub parindent_pt: f64,
    pub parskip: Skip,
    pub abovedisplayskip: Skip,
    pub abovedisplayshortskip: Skip,
    pub belowdisplayskip: Skip,
    pub belowdisplayshortskip: Skip,
    /// amsmath `leqno` (a class or package option): `\veqno` is `\leqno`,
    /// equation numbers sit at the left of the display.
    pub leqno: bool,
    /// amsmath `fleqn`: displays are set flush left, `\@mathmargin`
    /// (`\leftmargini`) in from the display's left edge.
    pub fleqn: bool,
    /// Whether family 3 (the `cmex` extension family, `largesymbols`)
    /// follows the sizes amsmath/amsfonts declare instead of the kernel's
    /// `sfixed*cmex10`; see [`cmex_designs`].
    pub cmex_designs: bool,
    pub script_size_pt: f64,
    pub scriptscript_size_pt: f64,
    pub tolerance: f64,
    pub pretolerance: f64,
    pub linepenalty: f64,
    pub adjdemerits: f64,
    /// `\raggedbottom` (standard classes: one-sided, one-column documents;
    /// otherwise the kernel's `\flushbottom`).
    pub raggedbottom: bool,
    /// `\emergencystretch` (0; `3em` under `\sloppy`, which the standard
    /// classes select for two-column documents).
    pub emergency_stretch_pt: f64,
    /// `\columnseprule` (the class default, or a preamble `\setlength`).
    pub columnseprule_pt: f64,
    /// `\topsep`, `\partopsep` and `\leftmargini` of a level-1 list
    /// (`\` of size1x.clo): the glue around and the margins of
    /// `center`/`quote`-style environments.
    pub topsep: Skip,
    pub partopsep: Skip,
    pub leftmargini_pt: f64,
    /// `\parsep` of a level-1 list (`\@listi`): `\list` sets
    /// `\parskip\parsep`, so it is the gap every `\item` paragraph adds.
    pub parsep: Skip,
    /// `\labelsep` (article: `.5em` of `\normalsize`): the gap between a
    /// list label's right edge and the item text.
    pub labelsep_pt: f64,
    headings: [HeadingStyle; 5],
    /// The resolved class + geometry frame this stylesheet was built from
    /// ([`Stylesheet::from_resolved`]); `None` for [`Stylesheet::article`].
    pub class_geometry: Option<Box<ResolvedDocument>>,
    /// Character protrusion and font expansion when the preamble loads
    /// `microtype` (`None` otherwise; lines are then broken exactly as
    /// before).
    pub microtype: Option<MicrotypeSetup>,
    /// Literal UTF-8 input checks (`crate::inputenc`): the text encoding
    /// and the preamble's own declarations, or `None` to typeset every
    /// character the fonts can draw without pdfLaTeX's input errors (a
    /// stylesheet built without a document, or a document outside the
    /// pdfLaTeX `utf8` world). The adapter sets it.
    pub input: Option<crate::inputenc::InputSetup>,
    /// `\hyphenation{...}` words as written (`man-u-script`): exceptions to
    /// the patterns for every paragraph (compiler `Parsed::hyphenation`).
    pub hyphenation: Vec<String>,
    /// `\enlargethispage{<dimen>}` (and `*`): the command, its dimen in
    /// points and whether it is starred, in document order (compiler
    /// `Parsed::parameters`). Where the command stands decides the page it
    /// enlarges; see `pagebuild::Enlarge`.
    pub enlarge_this_page: Vec<(flashtex_compiler::Span, f64, bool)>,
}

impl Stylesheet {
    /// `\documentclass[<size>pt]{article}` on US Letter with an optional
    /// `geometry` override. `size` other than 10/11/12 falls back to 10
    /// (LaTeX ignores unknown options).
    pub fn article(size: u32, family: Family, geometry: Option<Geometry>) -> Stylesheet {
        let base = match size {
            11 => BaseSize::Pt11,
            12 => BaseSize::Pt12,
            _ => BaseSize::Pt10,
        };
        let mut ds = DsStylesheet::article(ClassOptions {
            paper: Paper::Letter,
            size: base,
        });
        if let Some(g) = geometry {
            ds = ds.with_geometry(g);
        }
        let page = ds.page_layout();
        let body = ds.resolve(&[Block::Document, Block::Paragraph]);
        let body_size = body.font_size.0;
        // `ex` of the body font of the *selected family* (pdflatex evaluates
        // \section's skips in the current text font).
        let design = if size == 11 { 10 } else { size };
        let ex = params::text_params(family, false, false, design).x_height * body_size;
        let (script, scriptscript) = match base {
            BaseSize::Pt12 => (8.0, 6.0),
            BaseSize::Pt11 => (8.0, 6.0),
            BaseSize::Pt10 => (7.0, 5.0),
        };
        // Display skips from size1x.clo (\normalsize).
        let (above, above_short, below_short) = match base {
            BaseSize::Pt12 => (Skip::new(12.0, 3.0, 7.0), Skip::new(0.0, 3.0, 0.0), Skip::new(6.5, 3.5, 3.0)),
            BaseSize::Pt11 => (Skip::new(11.0, 3.0, 6.0), Skip::new(0.0, 3.0, 0.0), Skip::new(6.5, 3.5, 3.0)),
            BaseSize::Pt10 => (Skip::new(10.0, 2.0, 5.0), Skip::new(0.0, 3.0, 0.0), Skip::new(6.0, 3.0, 3.0)),
        };
        let heading = |level: u8| -> HeadingStyle {
            let h = ds.resolve(&[Block::Document, Block::Heading(level)]);
            let spec = flashtex_document_style::section_spec(level).expect("levels 1..=5");
            let before = spec.before_ex.scale(ex);
            let (after, run_in_after_em) = match spec.after {
                flashtex_document_style::SectionAfter::VerticalEx(s) => (s.scale(ex), None),
                // A run-in heading has no vertical after-skip at all: the
                // `em` becomes horizontal space on the paragraph's first line.
                flashtex_document_style::SectionAfter::RunInEm(em) => (flashtex_document_style::Skip::ZERO, Some(em)),
            };
            HeadingStyle {
                size_pt: h.font_size.0,
                baselineskip_pt: h.baselineskip.0,
                bold: h.bold,
                before: Skip::new(before.pt, before.plus, before.minus),
                after: Skip::new(after.pt, after.plus, after.minus),
                run_in_after_em,
            }
        };
        let parskip = ds.parskip();
        let list = flashtex_document_style::list_level(base, 1);
        Stylesheet {
            family,
            nfss: match family {
                Family::LatinModern => crate::nfss::Scheme::LmT1,
                Family::ComputerModern | Family::Times => crate::nfss::Scheme::CmT1,
            },
            base,
            page_width_pt: page.paper_width.0,
            page_height_pt: page.paper_height.0,
            text_x_pt: page.text_area.x.0,
            text_y_pt: page.text_area.y.0,
            text_width_pt: page.text_area.width.0,
            text_height_pt: page.text_area.height.0,
            body_size_pt: body_size,
            baselineskip_pt: body.baselineskip.0,
            lineskip_pt: 1.0,
            lineskiplimit_pt: 0.0,
            topskip_pt: page.top_skip.0,
            maxdepth_pt: page.top_skip.0 / 2.0,
            parindent_pt: body.parindent.0,
            parskip: Skip::new(parskip.pt, parskip.plus, parskip.minus),
            abovedisplayskip: above,
            abovedisplayshortskip: above_short,
            belowdisplayskip: above,
            belowdisplayshortskip: below_short,
            leqno: false,
            fleqn: false,
            cmex_designs: false,
            script_size_pt: script,
            scriptscript_size_pt: scriptscript,
            tolerance: 200.0,
            pretolerance: 100.0,
            linepenalty: 10.0,
            adjdemerits: 10000.0,
            raggedbottom: true,
            emergency_stretch_pt: 0.0,
            columnseprule_pt: 0.0,
            topsep: Skip::new(list.topsep.pt, list.topsep.plus, list.topsep.minus),
            partopsep: Skip::new(list.partopsep.pt, list.partopsep.plus, list.partopsep.minus),
            leftmargini_pt: list.leftmargin.0,
            parsep: Skip::new(list.parsep.pt, list.parsep.plus, list.parsep.minus),
            labelsep_pt: list.labelsep.0,
            headings: [heading(1), heading(2), heading(3), heading(4), heading(5)],
            class_geometry: None,
            hyphenation: Vec::new(),
            enlarge_this_page: Vec::new(),
            microtype: None,
            input: None,
        }
    }

    /// The stylesheet of a resolved standard-class document
    /// (`flashtex-class-geometry`, exact to the sp against pdflatex): the
    /// page frame (MediaBox, text block, `\textheight`), `\topskip`,
    /// `\maxdepth` and `\parindent` come from `doc`; sizes, heading fonts
    /// and skips (evaluated in `family`'s TFM `ex`), display skips, `\parskip`
    /// and the list glue/margins (`\leftmargini`, `\labelsep`, `\topsep`, ...)
    /// stay document-style's article tables at `doc`'s class size
    /// (class-geometry CONTRACT step 6).
    ///
    /// The page is the PDF MediaBox, not the paper: without `geometry`
    /// pdfTeX keeps the engine default of `pdftexconfig.tex` (US Letter in
    /// MacTeX 2026), so `\documentclass[a4paper]{article}` alone ships a
    /// Letter page with the A4 text block placed from its top-left corner.
    /// The frame values are the `\oddsidemargin` (odd/one-sided page) text
    /// block of the first column (`text_width_pt` is `\columnwidth`); the
    /// page builder takes even-page left edges, the second column and the
    /// header/footer baselines from `class_geometry`.
    ///
    /// article/report/book end with `\if@twoside\else\raggedbottom\fi` and
    /// `\if@twocolumn \sloppy\flushbottom\fi`: two-sided or two-column
    /// documents keep `\flushbottom`, two-column ones `\sloppy`
    /// (`\tolerance 9999`, `\emergencystretch 3em`).
    pub fn from_resolved(doc: &ResolvedDocument, family: Family) -> Stylesheet {
        let size = match doc.options.size {
            flashtex_class_geometry::BaseSize::Pt10 => 10,
            flashtex_class_geometry::BaseSize::Pt11 => 11,
            flashtex_class_geometry::BaseSize::Pt12 => 12,
        };
        let mut s = Stylesheet::article(size, family, None);
        let (frame, p) = (&doc.frame, &doc.params);
        s.page_width_pt = media_pt(frame.pdf_page_width);
        s.page_height_pt = media_pt(frame.pdf_page_height);
        s.text_x_pt = frame_pt(frame.text_left(1));
        s.text_y_pt = frame_pt(frame.text_top);
        s.text_width_pt = frame_pt(frame.columns[0].width);
        s.text_height_pt = frame_pt(frame.text_height);
        s.topskip_pt = frame_pt(p.topskip);
        s.maxdepth_pt = frame_pt(p.maxdepth);
        s.parindent_pt = frame_pt(p.parindent);
        // `\parskip` is a *class* length, not a shared default: article and
        // friends set `0pt plus 1pt`, letter.cls line 91 sets `0.7em`
        // (7.66498pt rigid at 11pt), and reading it from the resolved class
        // instead of `Stylesheet::article`'s is what puts a letter's
        // paragraphs where pdflatex puts them. Before this the whole page
        // rode 7.6 bp per paragraph too high.
        s.parskip = Skip::new(
            frame_pt(p.parskip.natural),
            frame_pt(p.parskip.stretch),
            frame_pt(p.parskip.shrink),
        );
        // article/report/book guard it (`\if@twoside\else\raggedbottom\fi`);
        // letter.cls line 404 is a plain `\raggedbottom` with no guard at
        // all, so a `[twoside]` letter is ragged-bottom too.
        s.raggedbottom = doc.options.kind == flashtex_class_geometry::ClassKind::Letter
            || !(doc.flags.twoside || doc.flags.twocolumn);
        s.columnseprule_pt = frame_pt(frame.columnseprule);
        if doc.flags.twocolumn {
            s.tolerance = 9999.0;
            s.emergency_stretch_pt = 3.0 * s.body_size_pt;
        }
        s.class_geometry = Some(Box::new(doc.clone()));
        s
    }

    /// The body family selected by the loaded packages.
    pub fn family_of(packages: &[String]) -> Family {
        Stylesheet::family_for(packages, false)
    }

    /// The body family selected by the loaded packages and the text font
    /// encoding. LaTeX's default `\rmdefault` is `cmr`; under
    /// `\usepackage[T1]{fontenc}` without `lmodern` that is `t1cmr.fd`,
    /// whose metrics are the EC fonts (`ecrm1095` for 11pt body text) —
    /// not the `ec-lm*` fonts `lmodern` selects, which are about 0.6%
    /// wider at 10.95pt and move line breaks. Those documents get
    /// [`Family::ComputerModern`]: EC metrics with Latin Modern outlines.
    /// OT1 documents (no `fontenc`) keep the Latin Modern metrics.
    pub fn family_for(packages: &[String], t1_encoding: bool) -> Family {
        if packages.iter().any(|p| matches!(p.as_str(), "times" | "mathptmx" | "newtxtext" | "txfonts")) {
            Family::Times
        } else if t1_encoding && !packages.iter().any(|p| p == "lmodern") {
            Family::ComputerModern
        } else {
            Family::LatinModern
        }
    }

    pub fn heading(&self, level: u8) -> HeadingStyle {
        self.headings[usize::from(level.clamp(1, 5) - 1)]
    }

    /// `\small` as `size1x.clo` declares it: `\@setfontsize\small` (the size
    /// and that size's own `\baselineskip`, from document-style's table) and
    /// the `\@listi` the command *redefines* while it is in force.
    ///
    /// `\small` only `\def`s `\@listi`; it does not execute it, so `\topsep`
    /// keeps `\normalsize`'s value ([`Stylesheet::topsep`]) until a `\list`
    /// at depth 1 runs inside the smaller size. `\partopsep` is a plain
    /// length none of the size commands touch, so it is shared with
    /// [`Stylesheet::partopsep`].
    ///
    /// size10.clo 62-67, size11.clo 58-68, size12.clo 58-68 (v1.4n,
    /// TeX Live 2025).
    pub fn small(&self) -> SmallSize {
        let fs = flashtex_document_style::font_size(self.base, flashtex_document_style::SizeName::Small);
        let (topsep, parsep) = match self.base {
            BaseSize::Pt10 => (Skip::new(4.0, 2.0, 2.0), Skip::new(2.0, 1.0, 1.0)),
            BaseSize::Pt11 => (Skip::new(6.0, 2.0, 2.0), Skip::new(3.0, 2.0, 1.0)),
            BaseSize::Pt12 => (Skip::new(9.0, 3.0, 5.0), Skip::new(4.5, 2.0, 1.0)),
        };
        SmallSize {
            size_pt: fs.size.0,
            baselineskip_pt: fs.baselineskip.0,
            topsep,
            parsep,
            partopsep: self.partopsep,
        }
    }
}

/// `\small` in the class's `size1x.clo` (see [`Stylesheet::small`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmallSize {
    pub size_pt: f64,
    pub baselineskip_pt: f64,
    /// `\topsep` of the `\@listi` `\small` defines (4pt at a 10pt base, not
    /// `\normalsize`'s 8pt).
    pub topsep: Skip,
    pub parsep: Skip,
    /// `\partopsep`, which no size command redefines.
    pub partopsep: Skip,
}

impl SmallSize {
    /// `\@topsepadd` of a level-1 `\list` opened while `\small` is in force:
    /// `\topsep` plus `\partopsep` (`\@trivlist` adds `\partopsep` when the
    /// `\begin` was read in vertical mode, which `\quotation` inside
    /// `abstract` always is). This is the glue `\endlist`'s `\@endparenv`
    /// puts after the environment.
    pub fn topsepadd(self) -> Skip {
        Skip::new(
            self.topsep.natural + self.partopsep.natural,
            self.topsep.stretch + self.partopsep.stretch,
            self.topsep.shrink + self.partopsep.shrink,
        )
    }
}

/// Packages that redeclare `OMX/cmex/m/n` with amsfonts' size ranges, so
/// family 3 is loaded at the math size instead of `sfixed` at 10pt:
/// `amsfonts.sty` 36-41 (`<-7.5>cmex7<7.5-8.5>cmex8<8.5-9.5>cmex9<9.5->cmex10`)
/// and `amsmath.sty` 109-114, which amsmath, amssymb and every package
/// loading them inherit.
const CMEX_DESIGN_PACKAGES: [&str; 5] = ["amsmath", "amsfonts", "amssymb", "mathtools", "physics"];

/// Whether family 3 (`largesymbols`) follows the sizes amsmath/amsfonts
/// declare instead of the LaTeX kernel's `omxcmex.fd` `<->sfixed*cmex10`.
///
/// `lmodern` wins over amsmath in either load order, because it rebinds the
/// symbol font itself (`\DeclareSymbolFont{largesymbols}{OMX}{lmex}{m}{n}`)
/// rather than the `cmex` shape amsmath redeclares, and `omxlmex.fd` keeps
/// `sfixed*lmex10`. amsmath's `cmex10` option restores the kernel's
/// declaration.
///
/// Measured with `\fontname\textfont3` under pdfTeX 3.141592653-2.6-1.40.27
/// (TeX Live 2025), `article`:
///
/// | packages | 10pt | 11pt | 12pt |
/// |---|---|---|---|
/// | (none), `amsthm`, `siunitx` | `cmex10` | `cmex10` | `cmex10` |
/// | `amsmath` / `amsfonts` / `amssymb` / `mathtools` / `physics` | `cmex10` | `cmex10 at 10.95pt` | `cmex10 at 12.0pt` |
/// | any of those **+ `lmodern`** (either order) | `lmex10` | `lmex10` | `lmex10` |
/// | `[cmex10]{amsmath}` | `cmex10` | `cmex10` | `cmex10` |
///
/// `\scriptfont3`/`\scriptscriptfont3` follow the same declaration:
/// `cmex7`/`cmex7 at 5.0pt` at 10pt and `cmex8`/`cmex7 at 6.0pt` at 11 and
/// 12pt, which is why the 10pt case still differs from `sfixed` inside
/// scripts even though its text size agrees.
pub fn cmex_designs(packages: &[String], cmex10_option: bool) -> bool {
    !cmex10_option
        && !packages.iter().any(|p| p == "lmodern")
        && packages.iter().any(|p| CMEX_DESIGN_PACKAGES.contains(&p.as_str()))
}

/// A frame length in TeX points for the f64 layout. `len` is exact (sp);
/// when a 0.001 pt decimal lies within [`FRAME_SNAP_SP`] of it, that
/// decimal is used (`1in` = 4736286 sp is laid out as 72.27 pt, a letter
/// `margin=1in` `\textwidth` of 30785865 sp as 469.755 pt), so the f64
/// values equal the unit declarations the pipeline used before and its
/// display lists stay byte-identical; every value remains within 2 sp of
/// pdflatex, 30x finer than the 0.001 bp pdfTeX writes positions with.
/// Lengths further from a 0.001 pt decimal keep their exact sp value.
pub(crate) fn frame_pt(len: Sp) -> f64 {
    let exact = len.to_pt();
    let decimal = (exact * 1000.0).round() / 1000.0;
    if ((decimal - exact) * 65536.0).abs() <= FRAME_SNAP_SP {
        decimal
    } else {
        exact
    }
}

/// See [`frame_pt`].
const FRAME_SNAP_SP: f64 = 2.0;

/// A MediaBox length as pdfTeX writes it (big points rounded to
/// `\pdfdecimaldigits` = 3, MacTeX `pdftexconfig.tex`), back in TeX points.
fn media_pt(len: Sp) -> f64 {
    (len.to_bp() * 1000.0).round() / 1000.0 * 72.27 / 72.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashtex_class_geometry::{resolve, DocumentSetup};
    use flashtex_document_style::Pt;

    /// HW1/HW2's preamble (`11pt` article, `margin=1in`) and body-only
    /// input (`12pt`, `margin=1in`): the class-geometry frame is within 2 sp
    /// of the document-style frame the pipeline used before, and the f64
    /// values laid out are identical (display lists byte-identical).
    #[test]
    fn resolved_frame_keeps_the_document_style_frame_for_margin_1in() {
        for (size, preamble) in [
            (11, "\\documentclass[11pt]{article}\n\\usepackage[margin=1in]{geometry}\n"),
            (12, "\\documentclass[12pt]{article}\n\\usepackage[margin=1in]{geometry}\n"),
            (10, "\\documentclass{article}\n"),
        ] {
            let doc = resolve(&DocumentSetup::from_preamble(preamble).expect("standard class"));
            let geometry = preamble.contains("geometry").then(|| Geometry::margin(Pt::inches(1.0)));
            let old = Stylesheet::article(size, Family::LatinModern, geometry);
            let new = Stylesheet::from_resolved(&doc, Family::LatinModern);
            let f = &doc.frame;
            for (what, old_pt, new_pt, exact) in [
                ("text_x", old.text_x_pt, new.text_x_pt, f.text_left(1)),
                ("text_y", old.text_y_pt, new.text_y_pt, f.text_top),
                ("text_width", old.text_width_pt, new.text_width_pt, f.columns[0].width),
                ("text_height", old.text_height_pt, new.text_height_pt, f.text_height),
                ("topskip", old.topskip_pt, new.topskip_pt, doc.params.topskip),
                ("maxdepth", old.maxdepth_pt, new.maxdepth_pt, doc.params.maxdepth),
            ] {
                if geometry.is_some() {
                    // The digest-relevant case: bit-identical f64.
                    assert_eq!(old_pt, new_pt, "{preamble}: {what}");
                } else {
                    // document-style sums floats (72.27 + 62 = 134.26999999999998);
                    // the snapped decimal is the same length far below one tick.
                    assert!((old_pt - new_pt).abs() < 1e-9, "{preamble}: {what} {old_pt} vs {new_pt}");
                }
                assert!(((new_pt * 65536.0) - exact.0 as f64).abs() <= 2.0, "{preamble}: {what} {new_pt} vs {exact:?}");
            }
            assert_eq!((old.page_width_pt, old.page_height_pt), (new.page_width_pt, new.page_height_pt), "{preamble}");
        }
    }

    #[test]
    fn a4paper_without_geometry_is_a_letter_media_box() {
        let doc = resolve(&DocumentSetup::from_preamble("\\documentclass[a4paper]{article}\n").unwrap());
        let s = Stylesheet::from_resolved(&doc, Family::LatinModern);
        assert!((s.page_width_pt - 614.295).abs() < 1e-9 && (s.page_height_pt - 794.97).abs() < 1e-9);
        assert_eq!((s.text_width_pt, s.text_height_pt), (345.0, 598.0));
        let with = resolve(&DocumentSetup::from_preamble("\\documentclass[a4paper]{article}\n\\usepackage{geometry}\n").unwrap());
        let a4 = Stylesheet::from_resolved(&with, Family::LatinModern);
        assert!((a4.page_width_pt * 72.0 / 72.27 - 595.276).abs() < 1e-9, "{}", a4.page_width_pt);
    }

    #[test]
    fn t1_cmr_documents_get_ec_metrics_and_lmodern_keeps_latin_modern() {
        let p = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(Stylesheet::family_for(&p(&["amsmath"]), true), Family::ComputerModern);
        assert_eq!(Stylesheet::family_for(&p(&["lmodern"]), true), Family::LatinModern);
        assert_eq!(Stylesheet::family_for(&p(&["amsmath"]), false), Family::LatinModern);
        assert_eq!(Stylesheet::family_for(&p(&["times"]), true), Family::Times);
        assert_eq!(Stylesheet::family_of(&p(&["amsmath"])), Family::LatinModern);
        assert!(crate::adapter::t1_encoding("\\usepackage[T1]{fontenc}"));
        assert!(crate::adapter::t1_encoding("\\usepackage[OT1, T1]{fontenc}"));
        assert!(!crate::adapter::t1_encoding("\\usepackage[T1,OT1]{fontenc}"));
        assert!(!crate::adapter::t1_encoding("\\usepackage{lmodern}"));
    }

    /// Every row is the `\fontname\textfont3` pdfTeX (TeX Live 2025)
    /// reports for that package set; see [`cmex_designs`].
    #[test]
    fn amsfonts_sizes_family_three_unless_lmodern_or_the_cmex10_option() {
        let p = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // cmex10, sfixed: the kernel's own declaration.
        assert!(!cmex_designs(&p(&[]), false));
        assert!(!cmex_designs(&p(&["amsthm"]), false));
        assert!(!cmex_designs(&p(&["siunitx"]), false));
        // cmex10 at the math size, in amsfonts' designs.
        assert!(cmex_designs(&p(&["amsmath"]), false));
        assert!(cmex_designs(&p(&["amsfonts"]), false));
        assert!(cmex_designs(&p(&["amssymb"]), false));
        assert!(cmex_designs(&p(&["mathtools"]), false));
        assert!(cmex_designs(&p(&["physics"]), false));
        assert!(cmex_designs(&p(&["amsmath", "amssymb", "amsthm"]), false));
        // lmodern rebinds `largesymbols` to `lmex`, which stays sfixed,
        // and wins in either load order.
        assert!(!cmex_designs(&p(&["lmodern", "amssymb"]), false));
        assert!(!cmex_designs(&p(&["amsfonts", "lmodern"]), false));
        // `\usepackage[cmex10]{amsmath}` restores the kernel's declaration.
        assert!(!cmex_designs(&p(&["amsmath"]), true));
    }

    #[test]
    fn oracle_geometry_matches_paragraph_layout_and_document_style() {
        let s = Stylesheet::article(12, Family::Times, Some(Geometry::margin(Pt::inches(1.0))));
        assert!((s.page_width_pt - 614.295).abs() < 1e-3);
        assert!((s.page_height_pt - 794.97).abs() < 1e-3);
        assert!((s.text_width_pt - 469.755).abs() < 1e-2, "{}", s.text_width_pt);
        assert!((s.text_x_pt - 72.27).abs() < 1e-3);
        assert_eq!(s.body_size_pt, 12.0);
        assert_eq!(s.baselineskip_pt, 14.5);
        assert_eq!(s.topskip_pt, 12.0);
        // \section in 12pt Times: 3.5ex = 18.9pt before (paragraph-layout doc).
        let h = s.heading(1);
        assert!((h.before.natural - 18.9).abs() < 1e-9, "{}", h.before.natural);
        assert!((h.after.natural - 12.42).abs() < 1e-9);
        assert_eq!(h.size_pt, 17.28);
        assert_eq!(h.baselineskip_pt, 22.0);
        // Latin Modern's ex differs (0.430556 em): 3.5ex = 18.083pt.
        let lm = Stylesheet::article(12, Family::LatinModern, Some(Geometry::margin(Pt::inches(1.0))));
        assert!((lm.heading(1).before.natural - 18.0834).abs() < 1e-3);
        // Plain article without geometry: 390pt text width at 12pt.
        let plain = Stylesheet::article(12, Family::LatinModern, None);
        assert_eq!(plain.text_width_pt, 390.0);
        assert!((plain.parindent_pt - 17.62482).abs() < 1e-5);
    }
}
