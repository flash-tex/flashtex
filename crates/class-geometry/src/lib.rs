//! FlashTeX original model of the LaTeX2e standard classes (`article`,
//! `report`, `book`) and the `geometry` package.
//!
//! [`resolve`] turns a preamble's `\documentclass`, `geometry` and
//! `\pagestyle` into a [`ResolvedDocument`]: every page-frame length in TeX
//! scaled points (exact to the sp against pdflatex), the header/text/footer
//! placement, the sectioning specs and the page-style macros. No TeX engine
//! is used; see `README.md` for provenance and `CONTRACT.md` for the
//! proposed render-pipeline integration.

pub mod beamer;
pub mod class;
pub mod frame;
pub mod generated;
pub mod geometry;
pub mod pagestyle;
pub mod sections;
pub mod tex;

pub use class::{
    beamer_paper_size, body_font, class_params, koma_params, letter_indentation, BaseSize,
    ClassKind, ClassOptions, DivSpec, FontMetrics, FontSize, Glue, PageParams, Paper,
};
pub use frame::{Column, PageFrame, Side};
pub use geometry::{apply_geometry, GeometryInput, LayoutFlags};
pub use pagestyle::{Field, Line, MarkRule, Numbering, PageStyle, StyleMacros};
pub use sections::{ChapterSpec, HeadingSpec, PageBreak, PartSpec};
pub use tex::Sp;

/// The preamble facts that decide the page frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSetup {
    pub class: ClassKind,
    pub class_options: String,
    /// `Some` when `geometry` is loaded (even without options: geometry
    /// then applies its 0.7 default scale).
    pub geometry: Option<GeometryInput>,
    /// Last preamble `\pagestyle`, if any.
    pub pagestyle: Option<PageStyle>,
    /// beamer: the last `\usetheme{..}` (`ThemeKind::Default` when none or
    /// an unmodelled name; see [`beamer::theme`]).
    pub beamer_theme: beamer::ThemeKind,
    /// beamer: whether the `navigation symbols` template is the default
    /// strip (`true`) or was emptied by `\setbeamertemplate{navigation
    /// symbols}{}` / `\beamertemplatenavigationsymbolsempty`. Any other
    /// replacement template (`[only frame symbol]`, `[horizontal]`, ...)
    /// keeps the default strip: not modelled.
    pub beamer_navigation_symbols: bool,
    /// beamer: the last `\setbeamercovered{..}` (`invisible` when none).
    pub beamer_covered: beamer::Covered,
    /// beamer: formulas are set in the sans family (`beamer.cls` 266:
    /// `\mathfamilydefault` is `\sfdefault` and `\beamer@sansmathtrue`;
    /// `beamerbasefont.sty` 204-260 then moves `operators`, `numbers` and
    /// `pureletters` to `OT1/cmss`). `false` after `\usefonttheme{serif}`
    /// (`\mathfamilydefault` back to `cmr`, which `\beamer@font@check`
    /// takes as "suppress the replacements"), `\usefonttheme
    /// {professionalfonts}` or the `mathserif` class option; `serif`'s
    /// `[onlylarge]` keeps the sans math.
    pub beamer_sans_math: bool,
    /// PDF media size when geometry does not set it (pdftex's
    /// `pdftexconfig.tex`; US Letter 8.5in x 11in in MacTeX 2026).
    pub engine_default_media: (Sp, Sp),
}

/// US Letter, the MacTeX 2026 `pdftexconfig.tex` default page size.
pub fn letter_media() -> (Sp, Sp) {
    (Sp::parse("8.5in").unwrap(), Sp::parse("11in").unwrap())
}

impl DocumentSetup {
    pub fn new(class: ClassKind, class_options: &str) -> DocumentSetup {
        DocumentSetup {
            class,
            class_options: class_options.to_string(),
            geometry: None,
            pagestyle: None,
            beamer_theme: beamer::ThemeKind::Default,
            beamer_navigation_symbols: true,
            beamer_covered: beamer::Covered::Invisible,
            beamer_sans_math: class == ClassKind::Beamer && !class_options.split(',').any(|o| o.trim() == "mathserif"),
            engine_default_media: letter_media(),
        }
    }

    /// Scan a LaTeX preamble (up to `\begin{document}`) for
    /// `\documentclass[..]{..}`, `\usepackage[..]{..geometry..}`,
    /// `\geometry{..}` and `\pagestyle{..}`. Returns `None` when the class
    /// is not one of the standard classes, `letter`, `beamer`, or KOMA's
    /// `scrartcl` / `scrreprt` / `scrbook`.
    pub fn from_preamble(source: &str) -> Option<DocumentSetup> {
        Self::scan_preamble(source, None)
    }

    /// [`from_preamble`](Self::from_preamble) for a document whose
    /// `\documentclass` names a project `.cls` file: the compiler has
    /// already read that file and reports the standard class it
    /// `\LoadClass`es (or `article`) with the options it passed on, so the
    /// class line in `source` is not parsed and `class`/`class_options`
    /// stand in for it. The rest of the preamble scan (`geometry`,
    /// `\pagestyle`, beamer templates) is unchanged.
    pub fn from_preamble_with_class(source: &str, class: ClassKind, class_options: &str) -> DocumentSetup {
        Self::scan_preamble(source, Some(DocumentSetup::new(class, class_options)))
            .expect("a given class always yields a setup")
    }

    fn scan_preamble(source: &str, given: Option<DocumentSetup>) -> Option<DocumentSetup> {
        let src = strip_comments(source);
        let end = src.find("\\begin{document}").unwrap_or(src.len());
        let pre = &src[..end];
        let class_given = given.is_some();
        let mut setup: Option<DocumentSetup> = given;
        let mut i = 0;
        while let Some(off) = pre[i..].find('\\') {
            let at = i + off + 1;
            let name: String = pre[at..]
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            let mut j = at + name.len();
            let opt = read_group(pre, &mut j, '[', ']');
            let arg = read_group(pre, &mut j, '{', '}');
            match (name.as_str(), arg) {
                ("documentclass", Some(a)) if !class_given => {
                    let kind = ClassKind::parse(&a)?;
                    setup = Some(DocumentSetup::new(kind, &opt.unwrap_or_default()));
                }
                ("documentclass", Some(_)) => {}
                ("usepackage" | "RequirePackage", Some(a)) => {
                    if let Some(s) = setup.as_mut() {
                        if a.split(',').any(|p| p.trim() == "geometry") {
                            s.geometry = Some(GeometryInput {
                                package_options: opt.unwrap_or_default(),
                                calls: Vec::new(),
                            });
                        }
                    }
                }
                ("geometry", Some(a)) => {
                    if let Some(g) = setup.as_mut().and_then(|s| s.geometry.as_mut()) {
                        g.calls.push(a);
                    }
                }
                ("pagestyle", Some(a)) => {
                    if let (Some(s), Some(ps)) = (setup.as_mut(), PageStyle::parse(&a)) {
                        s.pagestyle = Some(ps);
                    }
                }
                // `\usetheme{Madrid}` (beamer): a comma list loads several
                // themes; the last name decides.
                ("usetheme", Some(a)) => {
                    if let Some(s) = setup.as_mut().filter(|s| s.class == ClassKind::Beamer) {
                        if let Some(name) = a.split(',').next_back() {
                            s.beamer_theme = beamer::ThemeKind::parse(name);
                        }
                    }
                }
                // `\setbeamertemplate{navigation symbols}[opt]{<template>}`
                // (beamer): an empty template switches the strip off.
                ("setbeamertemplate", Some(a)) if a.trim() == "navigation symbols" => {
                    let _ = read_group(pre, &mut j, '[', ']');
                    if let (Some(s), Some(template)) = (setup.as_mut().filter(|s| s.class == ClassKind::Beamer), read_group(pre, &mut j, '{', '}')) {
                        s.beamer_navigation_symbols = !template.trim().is_empty();
                    }
                }
                ("beamertemplatenavigationsymbolsempty", None) => {
                    if let Some(s) = setup.as_mut().filter(|s| s.class == ClassKind::Beamer) {
                        s.beamer_navigation_symbols = false;
                    }
                }
                ("setbeamercovered", Some(a)) => {
                    if let Some(s) = setup.as_mut().filter(|s| s.class == ClassKind::Beamer) {
                        s.beamer_covered = beamer::Covered::parse(&a);
                    }
                }
                // `\usefonttheme[opts]{serif}` / `{professionalfonts}`
                // (beamer): the math replacements are suppressed.
                ("usefonttheme", Some(a)) => {
                    if let Some(s) = setup.as_mut().filter(|s| s.class == ClassKind::Beamer) {
                        let only_large = opt.as_deref().is_some_and(|o| o.split(',').any(|k| k.trim() == "onlylarge"));
                        for name in a.split(',') {
                            match name.trim() {
                                "serif" if !only_large => s.beamer_sans_math = false,
                                "professionalfonts" => s.beamer_sans_math = false,
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
            i = if j > at { j } else { at };
        }
        setup
    }
}

fn strip_comments(s: &str) -> String {
    s.lines()
        .map(|line| {
            let b = line.as_bytes();
            let mut k = 0;
            while k < b.len() {
                if b[k] == b'\\' {
                    k += 2;
                    continue;
                }
                if b[k] == b'%' {
                    return &line[..k];
                }
                k += 1;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_group(s: &str, j: &mut usize, open: char, close: char) -> Option<String> {
    let rest = &s[*j..];
    let trimmed = rest.trim_start();
    if !trimmed.starts_with(open) {
        return None;
    }
    let start = *j + (rest.len() - trimmed.len()) + 1;
    let mut depth = 1i32;
    for (k, c) in s[start..].char_indices() {
        if c == open || (open == '[' && c == '{') {
            depth += if c == open { 1 } else { 0 };
        }
        if c == close {
            depth -= 1;
            if depth == 0 {
                *j = start + k + 1;
                return Some(s[start..start + k].to_string());
            }
        }
    }
    None
}

/// Everything the class and geometry decide about pages and headings.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDocument {
    pub options: ClassOptions,
    pub params: PageParams,
    pub flags: LayoutFlags,
    pub font: FontMetrics,
    pub frame: PageFrame,
    pub headings: Vec<sections::HeadingSpec>,
    pub chapter: Option<sections::ChapterSpec>,
    pub part: sections::PartSpec,
    pub secnumdepth: i32,
    pub tocdepth: i32,
    pub pagestyle: PageStyle,
    /// Macros in effect after the class default and the preamble style.
    pub style_macros: StyleMacros,
    pub mark_rules: Vec<MarkRule>,
    pub numbering: Numbering,
    pub warnings: Vec<String>,
    /// beamer's theme (the default one for every other class).
    pub beamer_theme: beamer::Theme,
    /// beamer: the default `navigation symbols` strip is drawn on every
    /// non-plain frame page ([`DocumentSetup::beamer_navigation_symbols`]);
    /// `false` for every other class.
    pub beamer_navigation_symbols: bool,
    /// beamer: how covered overlay material is painted
    /// ([`DocumentSetup::beamer_covered`]).
    pub beamer_covered: beamer::Covered,
    /// beamer: formulas in the sans family
    /// ([`DocumentSetup::beamer_sans_math`]); `false` for every other class.
    pub beamer_sans_math: bool,
}

impl ResolvedDocument {
    pub fn heading(&self, name: &str) -> Option<&sections::HeadingSpec> {
        self.headings.iter().find(|h| h.name == name)
    }

    /// Header and footer lines shipped on `page`.
    pub fn head_foot(&self, page: i64) -> (Line, Line) {
        self.style_macros.for_page(self.flags.twoside, page)
    }

    /// `\twocolumn` / `\onecolumn` (latex.ltx lines 20256–20275): set
    /// `\if@twocolumn`, `\col@number` and `\columnwidth` (with
    /// `\hsize`/`\linewidth`), after a `\clearpage`.
    ///
    /// **They change nothing else**, and that is the whole difference
    /// between the command and the class option. The `twocolumn` *option*
    /// is read by `size1<n>.clo` while the class file is still running, so
    /// it also doubles `\textwidth`, and sets `\parindent` to `1em`,
    /// `\marginparsep` to `10pt` and `\leftmargini` to `2em`
    /// ([`crate::class`]). The commands run long after `size1<n>.clo` has
    /// finished, so those keep the one-column values.
    ///
    /// Measured against pdflatex (TeX Live 2026, `article`, 10pt, letter):
    /// `\twocolumn` in the preamble leaves `\textwidth` at 345pt — the
    /// text block still starts at x = 133.768 bp, not the option's 72.0 —
    /// and the first line of a paragraph is still indented 15pt, not the
    /// option's 1em. The two columns are 167.5pt wide with the second at
    /// +177.5pt (measured 310.605 − 133.768 = 176.837 bp), which is
    /// exactly `columnwidth(true)` and `\columnsep` of the *one-column*
    /// `\textwidth`.
    ///
    /// `options.twocolumn` is deliberately left alone: it records what
    /// `\documentclass` asked for, which is what fixed the dimensions.
    pub fn set_twocolumn(&mut self, twocolumn: bool) {
        if self.flags.twocolumn == twocolumn {
            return;
        }
        self.flags.twocolumn = twocolumn;
        self.frame.twocolumn = twocolumn;
        self.frame.columns = frame::columns(&self.params, twocolumn);
    }
}

pub fn resolve(setup: &DocumentSetup) -> ResolvedDocument {
    let options = ClassOptions::parse(setup.class, &setup.class_options);
    let font = body_font(options.size);
    let base = class_params(&options);
    let mut flags = LayoutFlags {
        twoside: options.twoside,
        mparswitch: options.twoside,
        twocolumn: options.twocolumn,
        reversemargin: false,
    };
    let mut params = base;
    let beamer_theme = beamer::theme(if options.kind == ClassKind::Beamer { setup.beamer_theme } else { beamer::ThemeKind::Default });
    if options.kind == ClassKind::Beamer {
        beamer::apply_theme(&mut params, &beamer_theme);
    }
    let mut warnings: Vec<String> = options
        .unused
        .iter()
        .map(|u| format!("Unused global option(s): [{u}]"))
        .collect();
    if let Some(g) = &setup.geometry {
        let out = apply_geometry(&options, &base, font, g);
        params = out.params;
        flags = out.flags;
        warnings.extend(out.warnings);
    }
    let (secnumdepth, tocdepth) = sections::default_depths(setup.class);
    let headings = sections::headings(setup.class, &params, font, secnumdepth);
    let chapter = sections::chapter(setup.class, options.openright, secnumdepth);
    let part = sections::part(
        setup.class,
        font,
        options.twoside,
        options.openright,
        secnumdepth,
    );
    let is_beamer = options.kind == ClassKind::Beamer;
    let default_style = pagestyle::class_default(setup.class);
    let mut macros = StyleMacros::EMPTY.apply(default_style, options.twoside);
    let style = setup.pagestyle.unwrap_or(default_style);
    if let Some(ps) = setup.pagestyle {
        macros = macros.apply(ps, options.twoside);
    }
    ResolvedDocument {
        frame: PageFrame::new(
            &params,
            flags,
            if setup.geometry.is_some() {
                (params.paperwidth, params.paperheight)
            } else if options.kind == ClassKind::Beamer
                || (options.kind.is_koma() && options.pagesize_pdf)
            {
                // `typearea` sets `\pdfpagewidth` / `\pdfpageheight` from
                // the paper (unless `pagesize=false`); the standard classes
                // never touch them, so their media stays the engine default
                // even for `a4paper`. beamer likewise always ships
                // slide-sized pages (its paper size, from `aspectratio`),
                // never the engine default.
                (params.paperwidth, params.paperheight)
            } else {
                setup.engine_default_media
            },
        ),
        mark_rules: pagestyle::mark_rules(setup.class, style, options.twoside),
        options,
        params,
        flags,
        font,
        headings,
        chapter,
        part,
        secnumdepth,
        tocdepth,
        pagestyle: style,
        style_macros: macros,
        numbering: Numbering::Arabic,
        warnings,
        beamer_theme,
        beamer_navigation_symbols: is_beamer && setup.beamer_navigation_symbols,
        beamer_covered: if is_beamer { setup.beamer_covered } else { beamer::Covered::Invisible },
        beamer_sans_math: is_beamer && setup.beamer_sans_math,
    }
}
