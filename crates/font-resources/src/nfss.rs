//! NFSS text font selection: LaTeX2e's `\fontfamily`/`\fontseries`/
//! `\fontshape` + `\selectfont` for the text families the render pipeline
//! sets. The parser applies every font command to [`Selected`] as it reads
//! it (`parser::TextStyle::font`), so the state in force travels on each
//! text node, macro-expanded or not; the pipeline re-exports this module
//! for its face selection. Moved here from `crates/render-pipeline`
//! (PLAN1 slice 2), where it was applied to font commands found in the
//! source bytes. It lives in `font-resources` since PLAN3 S1, next to the
//! TFM file tables of [`crate::tfm_files`]; `flashtex_compiler::nfss`
//! re-exports it. Transcribed from MacTeX 2026 (`tex/latex/base/latex.ltx`, LaTeX
//! 2025-11-01) and the font definition files it loads:
//!
//! * `ot1cmr.fd`, `ot1cmss.fd`, `ot1cmtt.fd` (no `fontenc`),
//! * `t1cmr.fd`, `t1cmss.fd`, `t1cmtt.fd` (`\usepackage[T1]{fontenc}`),
//! * `t1lmr.fd`, `t1lmss.fd`, `t1lmtt.fd` (`lmodern`; the `ot1lm*.fd`
//!   files declare the same shapes with `rm-lm*` metrics).
//!
//! Every font command is `\fontX{<request>}\selectfont`
//! (latex.ltx `\bfseries`, `\itshape`, `\sffamily`, ...). A series or
//! shape request is first *merged* with the current value through the
//! change rules (`\DeclareFontSeriesChangeRule`, latex.ltx 12056-12076;
//! `\DeclareFontShapeChangeRule`, latex.ltx 12361-12399), which
//! `\merge@font@shape@` (latex.ltx 12451-12469) and `\merge@font@series@`
//! (12274-12292) apply: the rule's first choice when that shape is declared
//! for the current encoding/family/series, else its second choice (with a
//! warning), else the request itself (with a warning). `\selectfont` then
//! loads the font; an undeclared shape goes through `\wrong@fontshape`
//! (latex.ltx 10689-10719): the default shape `n`, then the default series
//! `m`, then the default family, with "Font shape `...' undefined using
//! `...' instead". The state after a substitution is the substituted one,
//! so `\textbf{\textsc{\textmd{x}}}` in OT1 is `m/n`, not `m/sc`.
//! Declared `sub*` entries load another shape with a warning, `ssub*`
//! silently, and leave the state unchanged.
//!
//! `\em` (latex.ltx 14048-14057, no `\DeclareEmphSequence`) is
//! `\eminnershape` (= `\upshape`) when `\fontdimen1` (slant) of the current
//! font is positive, else `\itshape`: the slanted fonts here are exactly the
//! terminal `it`/`sl`/`scit`/`scsl` shapes.
//!
//! Only `m`/`bx` are ever requested (`\bfdefault` = `bx`, `\mddefault` =
//! `m`); `b` and `sbc` appear as declared/terminal series of the tables.

/// A text family slot: `\rmfamily`, `\sffamily`, `\ttfamily`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum FamilyKind {
    #[default]
    Rm,
    Sf,
    Tt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Series {
    #[default]
    M,
    B,
    Bx,
    Sbc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Shape {
    #[default]
    N,
    It,
    Sl,
    Sc,
    Scit,
    Scsl,
    Ui,
}

/// One encoding-independent font shape: family slot, series, shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FontKey {
    pub family: FamilyKind,
    pub series: Series,
    pub shape: Shape,
}

impl FontKey {
    pub const fn new(family: FamilyKind, series: Series, shape: Shape) -> FontKey {
        FontKey { family, series, shape }
    }

    /// `\fontdimen1 > 0`: the italic, slanted and slanted small-caps designs.
    pub fn slanted(self) -> bool {
        matches!(self.shape, Shape::It | Shape::Sl | Shape::Scit | Shape::Scsl)
    }

    pub fn bold(self) -> bool {
        matches!(self.series, Series::B | Series::Bx | Series::Sbc)
    }

    const fn with_shape(self, shape: Shape) -> FontKey {
        FontKey { shape, ..self }
    }

    const fn with_series(self, series: Series) -> FontKey {
        FontKey { series, ..self }
    }
}

/// Which font definition files the document's text fonts come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Scheme {
    /// LaTeX's defaults without `fontenc`: `ot1cmr`/`ot1cmss`/`ot1cmtt`.
    #[default]
    CmOt1,
    /// `\usepackage[T1]{fontenc}`: `t1cmr`/`t1cmss`/`t1cmtt` (EC fonts).
    CmT1,
    /// `lmodern` without `fontenc`: `ot1lmr`/`ot1lmss`/`ot1lmtt`.
    LmOt1,
    /// `lmodern` with T1: `t1lmr`/`t1lmss`/`t1lmtt`.
    LmT1,
}

impl Scheme {
    /// The scheme of a document from its packages and text encoding.
    /// Documents that switch to Times (`times`, `mathptmx`, ...) keep the
    /// Computer Modern tables for shape bookkeeping; their faces are Core 14.
    pub fn for_document(packages: &[String], t1_encoding: bool) -> Scheme {
        Scheme::of(packages.iter().any(|p| p == "lmodern"), t1_encoding)
    }

    /// The scheme with `lmodern` loaded or not, in T1 or OT1.
    pub const fn of(lmodern: bool, t1_encoding: bool) -> Scheme {
        match (lmodern, t1_encoding) {
            (true, true) => Scheme::LmT1,
            (true, false) => Scheme::LmOt1,
            (false, true) => Scheme::CmT1,
            (false, false) => Scheme::CmOt1,
        }
    }

    pub fn encoding(self) -> &'static str {
        match self {
            Scheme::CmOt1 | Scheme::LmOt1 => "OT1",
            Scheme::CmT1 | Scheme::LmT1 => "T1",
        }
    }

    fn lm(self) -> bool {
        matches!(self, Scheme::LmOt1 | Scheme::LmT1)
    }

    /// The NFSS family name of a slot (`\rmdefault` etc.).
    pub fn family_name(self, family: FamilyKind) -> &'static str {
        match (self.lm(), family) {
            (false, FamilyKind::Rm) => "cmr",
            (false, FamilyKind::Sf) => "cmss",
            (false, FamilyKind::Tt) => "cmtt",
            (true, FamilyKind::Rm) => "lmr",
            (true, FamilyKind::Sf) => "lmss",
            (true, FamilyKind::Tt) => "lmtt",
        }
    }

    /// `ENC/family/series/shape`, as LaTeX's font warnings spell it.
    pub fn describe(self, key: FontKey) -> String {
        let series = match key.series {
            Series::M => "m",
            Series::B => "b",
            Series::Bx => "bx",
            Series::Sbc => "sbc",
        };
        let shape = match key.shape {
            Shape::N => "n",
            Shape::It => "it",
            Shape::Sl => "sl",
            Shape::Sc => "sc",
            Shape::Scit => "scit",
            Shape::Scsl => "scsl",
            Shape::Ui => "ui",
        };
        format!("{}/{}/{series}/{shape}", self.encoding(), self.family_name(key.family))
    }
}

/// A `\DeclareFontShape` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// A real font (metrics file per size).
    Font,
    /// `sub*`: another shape, with a warning.
    Sub(FontKey),
    /// `ssub*`: another shape, silently.
    Ssub(FontKey),
}

/// The `\DeclareFontShape` entry for `key` in `scheme`, `None` when the
/// shape is not declared.
pub fn entry(scheme: Scheme, key: FontKey) -> Option<Entry> {
    use FamilyKind::{Rm, Sf, Tt};
    use Series::{Bx, Sbc, B, M};
    use Shape::{Ui, It, Sc, Scsl, Sl, N};
    let k = |family, series, shape| FontKey::new(family, series, shape);
    let font = Some(Entry::Font);
    match (scheme.lm(), matches!(scheme, Scheme::CmT1), key.family, key.series, key.shape) {
        // ot1cmr.fd: m/{n,sl,it,sc,ui}, b/n, bx/{n,sl,it}; bx/ui sub*cmr/m/ui.
        (false, false, Rm, M, N | Sl | It | Sc | Ui) | (false, false, Rm, B, N) | (false, false, Rm, Bx, N | Sl | It) => font,
        (false, false, Rm, Bx, Ui) => Some(Entry::Sub(k(Rm, M, Ui))),
        // ot1cmss.fd: m/n, m/it ssub*cmss/m/sl, m/sl, m/sc sub*cmr/m/sc,
        // m/ui sub*cmr/m/ui, sbc/n, bx/n, bx/ui sub*cmr/bx/ui.
        (false, false, Sf, M, N | Sl) | (false, false, Sf, Sbc, N) | (false, false, Sf, Bx, N) => font,
        (false, false, Sf, M, It) => Some(Entry::Ssub(k(Sf, M, Sl))),
        (false, false, Sf, M, Sc) => Some(Entry::Sub(k(Rm, M, Sc))),
        (false, false, Sf, M, Ui) => Some(Entry::Sub(k(Rm, M, Ui))),
        (false, false, Sf, Bx, Ui) => Some(Entry::Sub(k(Rm, Bx, Ui))),
        // ot1cmtt.fd: m/{n,it,sl,sc}; m/ui ssub*cmtt/m/it; bx/n ssub*m/n,
        // bx/it ssub*m/it, bx/sl ssub*m/n, bx/ui ssub*m/it.
        (false, false, Tt, M, N | It | Sl | Sc) => font,
        (false, false, Tt, M, Ui) | (false, false, Tt, Bx, It | Ui) => Some(Entry::Ssub(k(Tt, M, It))),
        (false, false, Tt, Bx, N | Sl) => Some(Entry::Ssub(k(Tt, M, N))),
        // t1cmr.fd: m/{n,sl,it,sc,ui,scsl}, bx/{n,it,sl,sc,scsl}, b/{n,scsl}.
        (false, true, Rm, M, N | Sl | It | Sc | Ui | Scsl) | (false, true, Rm, Bx, N | It | Sl | Sc | Scsl) | (false, true, Rm, B, N | Scsl) => font,
        // t1cmss.fd: m/{n,sl,it}, bx/{n,it,sl}, m/sc sub*cmr/m/sc, sbc/n.
        (false, true, Sf, M, N | Sl | It) | (false, true, Sf, Bx, N | It | Sl) | (false, true, Sf, Sbc, N) => font,
        (false, true, Sf, M, Sc) => Some(Entry::Sub(k(Rm, M, Sc))),
        // t1cmtt.fd: m/{n,sl,it,sc}; bx/n ssub*cmtt/m/n, bx/it ssub*cmtt/m/it.
        (false, true, Tt, M, N | Sl | It | Sc) => font,
        (false, true, Tt, Bx, N) => Some(Entry::Ssub(k(Tt, M, N))),
        (false, true, Tt, Bx, It) => Some(Entry::Ssub(k(Tt, M, It))),
        // t1lmr.fd: m/{n,sl,it,sc,ui,scsl}, b/{n,sl}, bx/{n,it,sl};
        // b/it sub*lmr/b/sl.
        (true, _, Rm, M, N | Sl | It | Sc | Ui | Scsl) | (true, _, Rm, B, N | Sl) | (true, _, Rm, Bx, N | It | Sl) => font,
        (true, _, Rm, B, It) => Some(Entry::Sub(k(Rm, B, Sl))),
        // t1lmss.fd: m/n, m/it ssub*lmss/m/sl, m/sl, m/sc sub*lmr/m/sc,
        // b/{n,sl,it} ssub*lmss/bx/{n,sl,it}, sbc/{n,sl}, sbc/it
        // ssub*lmss/sbc/sl, bx/{n,sl}, bx/it ssub*lmss/bx/sl.
        (true, _, Sf, M, N | Sl) | (true, _, Sf, Sbc, N | Sl) | (true, _, Sf, Bx, N | Sl) => font,
        (true, _, Sf, M, It) => Some(Entry::Ssub(k(Sf, M, Sl))),
        (true, _, Sf, M, Sc) => Some(Entry::Sub(k(Rm, M, Sc))),
        (true, _, Sf, B, s @ (N | Sl | It)) => Some(Entry::Ssub(k(Sf, Bx, s))),
        (true, _, Sf, Sbc, It) => Some(Entry::Ssub(k(Sf, Sbc, Sl))),
        (true, _, Sf, Bx, It) => Some(Entry::Ssub(k(Sf, Bx, Sl))),
        // t1lmtt.fd (default, not `lmtt@use@light@as@normal`):
        // m/{n,it,sl,sc,scsl}, b/{n,sl}; b/it and bx/it sub*lmtt/b/sl;
        // bx/n ssub*lmtt/b/n, bx/sl ssub*lmtt/b/sl.
        (true, _, Tt, M, N | It | Sl | Sc | Scsl) | (true, _, Tt, B, N | Sl) => font,
        (true, _, Tt, B | Bx, It) => Some(Entry::Sub(k(Tt, B, Sl))),
        (true, _, Tt, Bx, N) => Some(Entry::Ssub(k(Tt, B, N))),
        (true, _, Tt, Bx, Sl) => Some(Entry::Ssub(k(Tt, B, Sl))),
        _ => None,
    }
}

pub fn declared(scheme: Scheme, key: FontKey) -> bool {
    entry(scheme, key).is_some()
}

/// A shape request: `\itshape`, `\slshape`, `\scshape`, `\upshape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeRequest {
    It,
    Sl,
    Sc,
    Up,
}

/// One font command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Family(FamilyKind),
    /// `\bfseries` (`bx`) or `\mdseries` (`m`).
    Series(Series),
    Shape(ShapeRequest),
    /// `\em`/`\emph`.
    Emph,
    /// `\normalfont`.
    Normal,
}

/// The state after a command and the shape LaTeX reported undefined on
/// the way, if any (the last one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Selected {
    pub key: FontKey,
    pub undefined: Option<FontKey>,
}

impl Selected {
    /// The document's starting state: `m/n` of the roman family, nothing
    /// reported.
    pub const NORMAL: Selected = Selected { key: FontKey::new(FamilyKind::Rm, Series::M, Shape::N), undefined: None };

    /// The state after `command`, as a running state: the shape reported
    /// undefined is the last one reported on the way here, kept until a
    /// later command reports another or `\normalfont` starts over (a
    /// group's end restores the whole state, report included).
    pub fn then(self, scheme: Scheme, command: Command) -> Selected {
        let next = apply(scheme, self.key, command);
        let kept = if command == Command::Normal { None } else { self.undefined };
        Selected { key: next.key, undefined: next.undefined.or(kept) }
    }

    /// `\fontdimen1 > 0` of the font LaTeX loads for this state: the shape
    /// after `sub*`/`ssub*` ([`terminal`]).
    pub fn slanted(self, scheme: Scheme) -> bool {
        terminal(scheme, self.key).0.slanted()
    }
}

/// `\DeclareFontShapeChangeRule {current}{request}{first}{second}`
/// (latex.ltx 12361-12399), restricted to the shapes and requests here.
fn shape_rule(current: Shape, request: ShapeRequest) -> Option<(Shape, Option<Shape>)> {
    use Shape::*;
    use ShapeRequest as R;
    Some(match (current, request) {
        (N, R::It) => (It, Some(Sl)),
        (N, R::Sl) => (Sl, Some(It)),
        (N, R::Up) => (N, None),
        (It, R::Sl) => (Sl, Some(It)),
        (It, R::Sc) => (Scit, Some(Scsl)),
        (It, R::Up) => (N, None),
        (Sl, R::It) => (It, Some(Sl)),
        (Sl, R::Sc) => (Scsl, Some(Scit)),
        (Sl, R::Up) => (N, None),
        (Sc, R::It) => (Scit, Some(Scsl)),
        (Sc, R::Sl) => (Scsl, Some(Scit)),
        (Sc, R::Up) => (N, None),
        (Scit, R::It) => (Scit, None),
        (Scit, R::Sl) => (Scsl, Some(Scit)),
        (Scit, R::Sc) => (Scit, None),
        (Scit, R::Up) => (Sc, None),
        (Scsl, R::It) => (Scit, Some(Scsl)),
        (Scsl, R::Sl) => (Scsl, None),
        (Scsl, R::Sc) => (Scsl, None),
        (Scsl, R::Up) => (Sc, None),
        _ => return None,
    })
}

/// `\merge@font@shape@`: the shape `\fontshape{request}` sets, and the
/// shape reported undefined when the rule's first choice was not declared.
fn merge_shape(scheme: Scheme, current: FontKey, request: ShapeRequest) -> (FontKey, Option<FontKey>) {
    let requested = match request {
        ShapeRequest::It => Shape::It,
        ShapeRequest::Sl => Shape::Sl,
        ShapeRequest::Sc => Shape::Sc,
        ShapeRequest::Up => Shape::N,
    };
    match shape_rule(current.shape, request) {
        None => (current.with_shape(requested), None),
        Some((first, second)) => {
            let a = current.with_shape(first);
            if declared(scheme, a) {
                (a, None)
            } else if let Some(b) = second.map(|s| current.with_shape(s)).filter(|b| declared(scheme, *b)) {
                (b, Some(a))
            } else {
                (current.with_shape(requested), second.map(|_| a))
            }
        }
    }
}

/// `\merge@font@series@` with the `\DeclareFontSeriesChangeRule`s for
/// `bx` (latex.ltx 12056 `{m}{bx}{bx}{b}`, 12074 `{b}{bx}{bx}{b}`, 12076
/// `{bx}{bx}{bx}{b}`); `m` has no rule from `m`/`b`/`bx`.
fn merge_series(scheme: Scheme, current: FontKey, request: Series) -> (FontKey, Option<FontKey>) {
    if request != Series::Bx || !matches!(current.series, Series::M | Series::B | Series::Bx) {
        return (current.with_series(request), None);
    }
    let a = current.with_series(Series::Bx);
    if declared(scheme, a) {
        return (a, None);
    }
    let b = current.with_series(Series::B);
    if declared(scheme, b) {
        (b, Some(a))
    } else {
        (a, Some(a))
    }
}

/// `\selectfont` on `key`: `\wrong@fontshape`'s default shape, series and
/// family for an undeclared shape.
pub fn select(scheme: Scheme, key: FontKey) -> Selected {
    if declared(scheme, key) {
        return Selected { key, undefined: None };
    }
    let mut k = key.with_shape(Shape::N);
    if !declared(scheme, k) {
        k.series = Series::M;
        if !declared(scheme, k) {
            k.family = FamilyKind::Rm;
        }
    }
    Selected { key: k, undefined: Some(key) }
}

/// The state after applying `command` to `current` (itself a selected
/// state).
pub fn apply(scheme: Scheme, current: FontKey, command: Command) -> Selected {
    let (requested, merge_undefined) = match command {
        Command::Family(family) => (FontKey { family, ..current }, None),
        Command::Series(series) => merge_series(scheme, current, series),
        Command::Shape(request) => merge_shape(scheme, current, request),
        Command::Emph => {
            let request = if terminal(scheme, current).0.slanted() { ShapeRequest::Up } else { ShapeRequest::It };
            merge_shape(scheme, current, request)
        }
        Command::Normal => (FontKey::default(), None),
    };
    let selected = select(scheme, requested);
    // A merge substitution is not reported when it lands on the shape the
    // warning would name (`\@font@shape@subst@warning` compares them).
    let merge_undefined = merge_undefined.filter(|u| *u != selected.key);
    Selected { key: selected.key, undefined: selected.undefined.or(merge_undefined) }
}

/// The font shape actually loaded for a selected `key`: `sub*`/`ssub*`
/// entries followed to a real font. The second value is the first `sub*`
/// hop (`from`, `to`), which LaTeX reports.
pub fn terminal(scheme: Scheme, key: FontKey) -> (FontKey, Option<(FontKey, FontKey)>) {
    let mut k = key;
    let mut warned = None;
    for _ in 0..4 {
        match entry(scheme, k) {
            Some(Entry::Sub(to)) => {
                warned.get_or_insert((k, to));
                k = to;
            }
            Some(Entry::Ssub(to)) => k = to,
            Some(Entry::Font) | None => break,
        }
    }
    (k, warned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use FamilyKind::{Rm, Sf, Tt};
    use Series::{Bx, B, M};
    use Shape::{It, Sc, Scsl, Sl, N};

    fn run(scheme: Scheme, commands: &[Command]) -> Selected {
        let mut s = Selected { key: FontKey::default(), undefined: None };
        for c in commands {
            let next = apply(scheme, s.key, *c);
            s = Selected { key: next.key, undefined: next.undefined.or(s.undefined) };
        }
        s
    }

    const BF: Command = Command::Series(Bx);
    const MD: Command = Command::Series(M);
    const IT: Command = Command::Shape(ShapeRequest::It);
    const SL: Command = Command::Shape(ShapeRequest::Sl);
    const SC: Command = Command::Shape(ShapeRequest::Sc);
    const UP: Command = Command::Shape(ShapeRequest::Up);
    const SF: Command = Command::Family(Sf);
    const TT: Command = Command::Family(Tt);

    #[test]
    fn bold_small_caps_substitute_like_pdflatex() {
        // pdflatex (OT1): "Font shape `OT1/cmr/bx/sc' undefined using
        // `OT1/cmr/bx/n' instead".
        let s = run(Scheme::CmOt1, &[BF, SC]);
        assert_eq!(s.key, FontKey::new(Rm, Bx, N));
        assert_eq!(Scheme::CmOt1.describe(s.undefined.unwrap()), "OT1/cmr/bx/sc");
        // t1cmr.fd declares bx/sc (ecxc).
        assert_eq!(run(Scheme::CmT1, &[SC, BF]), Selected { key: FontKey::new(Rm, Bx, Sc), undefined: None });
        // t1lmr.fd does not.
        assert_eq!(run(Scheme::LmT1, &[SC, BF]).key, FontKey::new(Rm, Bx, N));
        // The state after the substitution is bx/n: \textmd gives m/n.
        assert_eq!(run(Scheme::CmOt1, &[BF, SC, MD]).key, FontKey::new(Rm, M, N));
    }

    #[test]
    fn emph_inside_small_caps_follows_the_sc_it_rule() {
        // pdflatex (OT1): "Font shape `OT1/cmr/m/scit' undefined using
        // `OT1/cmr/m/it' instead".
        let s = run(Scheme::CmOt1, &[SC, Command::Emph]);
        assert_eq!(s.key, FontKey::new(Rm, M, It));
        assert_eq!(Scheme::CmOt1.describe(s.undefined.unwrap()), "OT1/cmr/m/scit");
        // T1 and lmodern declare scsl, the rule's second choice.
        for scheme in [Scheme::CmT1, Scheme::LmT1] {
            let s = run(scheme, &[SC, Command::Emph]);
            assert_eq!(s.key, FontKey::new(Rm, M, Scsl), "{scheme:?}");
            assert_eq!(scheme.describe(s.undefined.unwrap()), format!("T1/{}/m/scit", scheme.family_name(Rm)));
            // \upshape from scsl returns to sc.
            assert_eq!(run(scheme, &[SC, Command::Emph, UP]).key, FontKey::new(Rm, M, Sc));
        }
    }

    #[test]
    fn emph_toggles_on_the_slant_of_the_loaded_font() {
        for scheme in [Scheme::CmOt1, Scheme::CmT1, Scheme::LmOt1, Scheme::LmT1] {
            assert_eq!(run(scheme, &[Command::Emph]).key, FontKey::new(Rm, M, It));
            assert_eq!(run(scheme, &[Command::Emph, Command::Emph]).key, FontKey::new(Rm, M, N));
            assert_eq!(run(scheme, &[SL, Command::Emph]).key, FontKey::new(Rm, M, N));
            assert_eq!(run(scheme, &[BF, Command::Emph]).key, FontKey::new(Rm, Bx, It));
            // cmss/lmss m/it is ssub*m/sl: slanted, so \emph goes upright.
            assert_eq!(run(scheme, &[SF, Command::Emph, Command::Emph]).key, FontKey::new(Sf, M, N));
        }
        // OT1 cmss has no bx/it: \selectfont substitutes bx/n (upright), so
        // an \emph inside it asks for italic again.
        let s = run(Scheme::CmOt1, &[SF, BF, IT]);
        assert_eq!(s.key, FontKey::new(Sf, Bx, N));
        assert_eq!(run(Scheme::CmOt1, &[SF, BF, IT, Command::Emph]).key, FontKey::new(Sf, Bx, N));
    }

    #[test]
    fn families_keep_series_and_shape_and_normalfont_resets() {
        assert_eq!(run(Scheme::CmT1, &[BF, IT, SF]).key, FontKey::new(Sf, Bx, It));
        assert_eq!(run(Scheme::CmT1, &[BF, IT, SF, Command::Normal]).key, FontKey::default());
        // Small caps in a sans family: declared as sub*cmr/m/sc.
        let s = run(Scheme::LmT1, &[SF, SC]);
        assert_eq!(s.key, FontKey::new(Sf, M, Sc));
        assert_eq!(s.undefined, None);
        assert_eq!(terminal(Scheme::LmT1, s.key), (FontKey::new(Rm, M, Sc), Some((FontKey::new(Sf, M, Sc), FontKey::new(Rm, M, Sc)))));
        // Typewriter bold: ssub to medium in cm, to lmtt/b in lm.
        assert_eq!(terminal(Scheme::CmT1, run(Scheme::CmT1, &[TT, BF]).key).0, FontKey::new(Tt, M, N));
        assert_eq!(terminal(Scheme::LmT1, run(Scheme::LmT1, &[TT, BF]).key).0, FontKey::new(Tt, B, N));
    }

    #[test]
    fn sans_italic_is_the_slanted_design() {
        assert_eq!(terminal(Scheme::CmOt1, FontKey::new(Sf, M, It)).0, FontKey::new(Sf, M, Sl));
        assert_eq!(terminal(Scheme::LmT1, FontKey::new(Sf, Bx, It)).0, FontKey::new(Sf, Bx, Sl));
        // t1cmss.fd declares m/it itself (ecsi).
        assert_eq!(terminal(Scheme::CmT1, FontKey::new(Sf, M, It)).0, FontKey::new(Sf, M, It));
    }

    #[test]
    fn slanted_shapes_merge_with_italic() {
        assert_eq!(run(Scheme::CmOt1, &[IT, SL]).key, FontKey::new(Rm, M, Sl));
        assert_eq!(run(Scheme::CmOt1, &[SL, IT]).key, FontKey::new(Rm, M, It));
        assert_eq!(run(Scheme::CmOt1, &[IT, UP]).key, FontKey::new(Rm, M, N));
        // OT1 cmr bx/sc is undeclared, so \scshape inside bold italic goes
        // through scit/scsl (both undeclared) to sc, then bx/n.
        let s = run(Scheme::CmOt1, &[BF, IT, SC]);
        assert_eq!(s.key, FontKey::new(Rm, Bx, N));
        assert_eq!(Scheme::CmOt1.describe(s.undefined.unwrap()), "OT1/cmr/bx/sc");
    }
}
