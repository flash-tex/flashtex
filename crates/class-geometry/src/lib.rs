//! FlashTeX original model of the LaTeX2e standard classes (`article`,
//! `report`, `book`) and the `geometry` package.
//!
//! [`resolve`] turns a preamble's `\documentclass`, `geometry` and
//! `\pagestyle` into a [`ResolvedDocument`]: every page-frame length in TeX
//! scaled points (exact to the sp against pdflatex), the header/text/footer
//! placement, the sectioning specs and the page-style macros. No TeX engine
//! is used; see `README.md` for provenance and `CONTRACT.md` for the
//! proposed render-pipeline integration.

pub mod class;
pub mod frame;
pub mod geometry;
pub mod pagestyle;
pub mod sections;
pub mod tex;

pub use class::{
    body_font, class_params, letter_indentation, BaseSize, ClassKind, ClassOptions, FontMetrics,
    FontSize, Glue, PageParams, Paper,
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
            engine_default_media: letter_media(),
        }
    }

    /// Scan a LaTeX preamble (up to `\begin{document}`) for
    /// `\documentclass[..]{..}`, `\usepackage[..]{..geometry..}`,
    /// `\geometry{..}` and `\pagestyle{..}`. Returns `None` when the class
    /// is not one of the three standard classes.
    pub fn from_preamble(source: &str) -> Option<DocumentSetup> {
        let src = strip_comments(source);
        let end = src.find("\\begin{document}").unwrap_or(src.len());
        let pre = &src[..end];
        let mut setup: Option<DocumentSetup> = None;
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
                ("documentclass", Some(a)) => {
                    let kind = ClassKind::parse(&a)?;
                    setup = Some(DocumentSetup::new(kind, &opt.unwrap_or_default()));
                }
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
}

impl ResolvedDocument {
    pub fn heading(&self, name: &str) -> Option<&sections::HeadingSpec> {
        self.headings.iter().find(|h| h.name == name)
    }

    /// Header and footer lines shipped on `page`.
    pub fn head_foot(&self, page: i64) -> (Line, Line) {
        self.style_macros.for_page(self.flags.twoside, page)
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
    }
}
