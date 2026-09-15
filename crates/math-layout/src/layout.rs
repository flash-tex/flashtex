//! The layout engine: TeXbook Appendix G, rules 5–20, and `tex.web`
//! §§ 720–767 (`mlist_to_hlist`) as the reference for the arithmetic.
//!
//! Every rule is implemented against [`MathFontMetrics`] parameters, so the
//! numbers below are TeX's when the Computer Modern adapter is used.

use crate::boxes::{BoxKind, Child, Flex, MathBox};
use crate::mathlist::{
    Atom, AtomClass, BigSizing, LeftScripts, Limits, MathList, Nucleus, TextPiece, TextStyle,
};
use crate::metrics::{Extensible, Glyph, MathFontMetrics, MathParams};
use crate::metrics::{MathChar, OrdPair};
use crate::source::SourceTag;
use crate::spacing::{Space, between};
use crate::style::Style;

use crate::metrics::{OrdLigature, SizeClass};
use std::borrow::Cow;

/// Something the engine could not do exactly; the layout still completes with
/// an explicit fallback so the caller can report rather than guess.
#[derive(Debug, Clone, PartialEq)]
pub enum Limitation {
    /// The metrics provider has no glyph for this symbol; an empty box was used.
    MissingGlyph(char),
    /// No delimiter in the size list reached the wanted size; the largest one
    /// was used (extensible delimiters are not built yet).
    DelimiterTooSmall { ch: char, wanted: f64, used: f64 },
    /// Same for the radical sign.
    RadicalTooSmall { wanted: f64, used: f64 },
    /// The accent symbol is unknown; the base was laid out without it.
    MissingAccent(char),
}

/// A finished layout: the root box plus any limitations encountered.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub root: MathBox,
    pub limitations: Vec<Limitation>,
}

/// Lays out `list` in `style`; the root box is an hbox on the text baseline.
pub fn layout(list: &MathList, style: Style, metrics: &dyn MathFontMetrics) -> MathBox {
    layout_with_report(list, style, metrics).root
}

/// [`layout`] plus the list of limitations hit along the way.
pub fn layout_with_report(list: &MathList, style: Style, metrics: &dyn MathFontMetrics) -> Layout {
    let mut engine = Engine {
        m: metrics,
        limitations: Vec::new(),
    };
    let root = engine.list(list, style);
    Layout {
        root,
        limitations: engine.limitations,
    }
}

struct Engine<'a> {
    m: &'a dyn MathFontMetrics,
    limitations: Vec<Limitation>,
}

/// Rules 5 and 6: Bin atoms that cannot be binary become Ord.
pub fn effective_classes(atoms: &[Atom]) -> Vec<AtomClass> {
    use AtomClass::*;
    let mut out: Vec<AtomClass> = Vec::with_capacity(atoms.len());
    // Index in `out` of the last noad; glue is skipped like TeX's `r_type`.
    let mut last: Option<usize> = None;
    for atom in atoms {
        let mut class = atom.class;
        if is_glue(atom) {
            out.push(class);
            continue;
        }
        match class {
            Bin => {
                let prev = last.map(|i| out[i]);
                if matches!(prev, None | Some(Bin | Op | Rel | Open | Punct)) {
                    class = Ord;
                }
            }
            Rel | Close | Punct => {
                if let Some(i) = last
                    && out[i] == Bin
                {
                    out[i] = Ord;
                }
            }
            _ => {}
        }
        out.push(class);
        last = Some(out.len() - 1);
    }
    if let Some(i) = last
        && out[i] == Bin
    {
        out[i] = Ord;
    }
    out
}

/// A bare glue atom (no scripts), which takes no part in atom spacing.
fn is_glue(atom: &Atom) -> bool {
    matches!(atom.nucleus, Nucleus::Glue { .. })
        && atom.superscript.is_none()
        && atom.subscript.is_none()
        && atom.left_scripts.is_none()
}

/// TeX §1186: when a math group closes holding exactly one ordinary atom
/// without scripts, the group's nucleus is replaced by that atom's nucleus,
/// so `{x}^2` and `\mathrm{K}^{-1}` place their scripts on a character
/// (Rule 18a, `shift_up` starts at 0) and `\mathop{x}` centres a character
/// on the axis (`make_op`), instead of treating a boxed sub-list. Glue is not
/// a noad and blocks the replacement.
///
/// Also returns the source tag of the innermost unpacked atom (`SourceTag::
/// NONE` when no unpacking happened), so the caller can apply it to the
/// resulting box before the enclosing atom's own tag only fills in what that
/// leaves missing (`SourceTag::inherit`'s innermost-wins rule) -- otherwise
/// an unpacked atom's own span/attribute would be lost in favour of the
/// outer (unpacked-away) atom's.
fn unpacked(nucleus: &Nucleus) -> (&Nucleus, SourceTag) {
    let mut n = nucleus;
    let mut tag = SourceTag::NONE;
    while let Nucleus::List(list) = n {
        match list.atoms.as_slice() {
            [a] if a.class == AtomClass::Ord
                && a.superscript.is_none()
                && a.subscript.is_none()
                && a.left_scripts.is_none()
                && !matches!(a.nucleus, Nucleus::Glue { .. }) =>
            {
                n = &a.nucleus;
                tag = a.tag;
            }
            _ => break,
        }
    }
    (n, tag)
}

/// The character in an atom's nucleus after §1186 unpacking, if it is one.
fn nucleus_char(atom: &Atom) -> Option<MathChar> {
    match unpacked(&atom.nucleus).0 {
        Nucleus::Symbol(ch) => Some(MathChar::Symbol(*ch)),
        Nucleus::TextChar(ch) => Some(MathChar::Text(*ch)),
        _ => None,
    }
}

/// A character nucleus for a ligature character.
fn char_nucleus(ch: MathChar) -> Nucleus {
    match ch {
        MathChar::Symbol(ch) => Nucleus::Symbol(ch),
        MathChar::Text(ch) => Nucleus::TextChar(ch),
    }
}

/// Replaces an atom's character nucleus (after §1186 unpacking) by `ch`,
/// keeping the unpacked atom's source tag.
fn set_nucleus_char(atom: &mut Atom, ch: MathChar) {
    let mut inner = unpacked(&atom.nucleus).1;
    atom.nucleus = char_nucleus(ch);
    if !inner.is_none() {
        inner.inherit(atom.tag);
        atom.tag = inner;
    }
}

/// Most ligature steps one `make_ord` (or one text run) takes.
const LIGATURE_LIMIT: usize = 256;

impl Engine<'_> {
    fn params(&self, style: Style) -> MathParams {
        self.m.params(style.size_class())
    }

    /// `mlist_to_hlist`: box every atom, then insert spacing (Rule 20).
    fn list(&mut self, list: &MathList, style: Style) -> MathBox {
        let (atoms, pairs) = self.make_ords(&list.atoms, style);
        let classes = effective_classes(&atoms);
        let mu = self.params(style).mu();
        let mut items: Vec<MathBox> = Vec::with_capacity(atoms.len() * 2);
        let mut prev: Option<AtomClass> = None;
        for ((atom, &class), &pair) in atoms.iter().zip(&classes).zip(&pairs) {
            if is_glue(atom) {
                if let Nucleus::Glue {
                    mu: g,
                    pt,
                    stretch,
                    shrink,
                } = atom.nucleus
                {
                    items.push(MathBox::glue_flex(
                        g * mu + pt,
                        g,
                        stretch.resolve(mu),
                        shrink.resolve(mu),
                    ));
                }
                continue;
            }
            let mut b = self.atom(atom, class, style, pair.is_some_and(|p| p.text_font));
            // Leaves no inner atom claimed belong to this atom.
            b.inherit_tag(atom.tag);
            if let Some(p) = prev {
                let space = between(p, class, style);
                if space != Space::None {
                    // Rule 20 with plain.tex's muskips, stretch and shrink
                    // included (§716 `math_glue` scales all three by mu).
                    items.push(MathBox::glue_flex(
                        space.mu() * mu,
                        space.mu(),
                        Flex::pt(space.stretch_mu() * mu),
                        Flex::pt(space.shrink_mu() * mu),
                    ));
                }
            }
            items.push(b);
            // The font kern sits right after the character, before any
            // inter-atom glue Rule 20 puts ahead of the next atom.
            if let Some(p) = pair.filter(|p| p.kern != 0.0) {
                items.push(MathBox::kern(p.kern));
            }
            prev = Some(class);
        }
        MathBox::hlist(items)
    }

    /// The first pass of `mlist_to_hlist` as far as `make_ord` (tex.web
    /// §752) goes: the list after the ligatures its font programs form, and
    /// for every atom of that list what `make_ord` found after it (`None`
    /// for glue and for atoms `make_ord` leaves alone). The list is only
    /// copied when a ligature is formed.
    ///
    /// TeX calls `make_ord` from its first pass only for noads that are Ord
    /// at that moment: an Ord, or a Bin that Rule 5 turns into an Ord because
    /// of the noad before it (`r_type`). A Bin that Rule 6 or the end of the
    /// list demotes later has already been passed, so it is not kerned.
    fn make_ords<'l>(
        &self,
        atoms: &'l [Atom],
        style: Style,
    ) -> (Cow<'l, [Atom]>, Vec<Option<OrdPair>>) {
        use AtomClass::*;
        let size = style.size_class();
        let mut atoms = Cow::Borrowed(atoms);
        let mut pairs = Vec::with_capacity(atoms.len());
        // TeX's `r_type`: the class of the last noad after Rule 5 (glue is
        // not a noad).
        let mut r_type: Option<AtomClass> = None;
        // A `|=:|>>` ligature's inserted character is a `math_text_char`:
        // `make_ord` skips it, and it loses its italic correction in a text
        // font like any character followed by one of its family.
        let mut inserted: Option<OrdPair> = None;
        let mut i = 0;
        while i < atoms.len() {
            if is_glue(&atoms[i]) {
                pairs.push(None);
                i += 1;
                continue;
            }
            let class = atoms[i].class;
            let ord = class == Ord
                || (class == Bin && matches!(r_type, None | Some(Bin | Op | Rel | Open | Punct)));
            let pair = match inserted.take() {
                Some(pair) => Some(pair),
                None if ord => {
                    let (pair, no_combine) = self.make_ord(&mut atoms, i, size);
                    inserted = no_combine.then(|| pair.expect("a ligature pair"));
                    pair
                }
                None => None,
            };
            pairs.push(pair);
            r_type = Some(if ord { Ord } else { class });
            i += 1;
        }
        (atoms, pairs)
    }

    /// `make_ord` (tex.web §752-§753) for the Ord atom at `i`: when it has
    /// no scripts, its nucleus is a character, and the next atom (no glue
    /// between) is an Ord..Punct atom whose nucleus is a character of the
    /// same family, the family font's program for the pair applies.
    ///
    /// * A kern is appended after the left character (the returned pair's
    ///   `kern`), and the character loses its italic correction when the
    ///   font is a text font (§755).
    /// * A ligature rewrites the list: `=:` replaces both characters by the
    ///   ligature, which takes over the right atom's scripts; `=:|` and
    ///   `|=:` replace one of them; `|=:|` inserts the ligature character
    ///   between them. The pair is then tried again from the left atom,
    ///   unless the op is one of the `>` forms, which stop there. The second
    ///   value is true when a `|=:|>>` inserted a character that must not
    ///   combine further.
    ///
    /// With no instruction for the pair the kern is 0 and the italic
    /// correction still goes in a text font.
    fn make_ord(
        &self,
        atoms: &mut Cow<'_, [Atom]>,
        i: usize,
        size: SizeClass,
    ) -> (Option<OrdPair>, bool) {
        // TeX loops on a cyclic ligature program until interrupted
        // (`check_interrupt`); a malformed font must not hang the layout.
        for _ in 0..LIGATURE_LIMIT {
            let q = &atoms[i];
            if q.superscript.is_some() || q.subscript.is_some() {
                return (None, false);
            }
            // Glue between the two is not a noad, so it blocks the pair: its
            // nucleus is no character.
            let Some(p) = atoms.get(i + 1).filter(|p| p.class != AtomClass::Inner) else {
                return (None, false);
            };
            let (Some(left), Some(right)) = (nucleus_char(q), nucleus_char(p)) else {
                return (None, false);
            };
            let Some(pair) = self.m.ord_pair(left, right, size) else {
                return (None, false);
            };
            let Some(OrdLigature { op, ch }) = pair.ligature else {
                return (Some(pair), false);
            };
            let atoms = atoms.to_mut();
            match op {
                1 | 5 => set_nucleus_char(&mut atoms[i], ch),
                2 | 6 => set_nucleus_char(&mut atoms[i + 1], ch),
                3 | 7 | 11 => {
                    let r = Atom::new(AtomClass::Ord, char_nucleus(ch)).with_tag(atoms[i].tag);
                    atoms.insert(i + 1, r);
                }
                _ => {
                    let p = atoms.remove(i + 1);
                    set_nucleus_char(&mut atoms[i], ch);
                    atoms[i].superscript = p.superscript;
                    atoms[i].subscript = p.subscript;
                }
            }
            if op > 3 {
                let pair = OrdPair {
                    kern: 0.0,
                    ligature: None,
                    ..pair
                };
                return (Some(pair), op == 11);
            }
        }
        (None, false)
    }

    /// `clean_box`: a subformula as a single box.
    fn clean_box(&mut self, list: &MathList, style: Style) -> MathBox {
        self.list(list, style)
    }

    fn glyph(&mut self, ch: char, style: Style) -> Option<Glyph> {
        let g = self.m.glyph(ch, style.size_class());
        if g.is_none() {
            self.limitations.push(Limitation::MissingGlyph(ch));
        }
        g
    }

    /// A character of the upright text family (`\fam0`) at this style's size.
    fn text_char(&mut self, ch: char, style: Style) -> Option<Glyph> {
        let g = self.m.text_glyph(ch, style.size_class());
        if g.is_none() {
            self.limitations.push(Limitation::MissingGlyph(ch));
        }
        g
    }

    /// `text_font_pair`: `make_ord` found the next character in the same
    /// text-font family, so the italic correction is dropped (§755).
    fn atom(
        &mut self,
        atom: &Atom,
        class: AtomClass,
        style: Style,
        text_font_pair: bool,
    ) -> MathBox {
        if let Some(left) = &atom.left_scripts {
            return self.make_sideset(atom, left);
        }
        if class == AtomClass::Op {
            return self.make_op(atom, style);
        }
        // The nucleus, TeX's `delta` (italic correction still to be applied),
        // and whether the nucleus is a bare character (Rule 18a).
        let (unpacked_nucleus, unpacked_tag) = unpacked(&atom.nucleus);
        let (nucleus, delta, is_char) = match unpacked_nucleus {
            Nucleus::Symbol(ch) => match self.glyph(*ch, style) {
                Some(g) => {
                    let b = MathBox::glyph(&g);
                    let italic = if text_font_pair { 0.0 } else { g.italic };
                    if atom.subscript.is_none() && italic != 0.0 {
                        (MathBox::hlist(vec![b, MathBox::kern(italic)]), 0.0, true)
                    } else {
                        (b, italic, true)
                    }
                }
                None => (MathBox::empty(), 0.0, false),
            },
            Nucleus::TextChar(ch) => match self.text_char(*ch, style) {
                Some(g) => {
                    let b = MathBox::glyph(&g);
                    let italic = if text_font_pair { 0.0 } else { g.italic };
                    if atom.subscript.is_none() && italic != 0.0 {
                        (MathBox::hlist(vec![b, MathBox::kern(italic)]), 0.0, true)
                    } else {
                        (b, italic, true)
                    }
                }
                None => (MathBox::empty(), 0.0, false),
            },
            Nucleus::List(list) => (self.clean_box(list, style), 0.0, false),
            Nucleus::Empty => (MathBox::empty(), 0.0, false),
            Nucleus::Fraction {
                numerator,
                denominator,
                thickness,
                left,
                right,
            } => {
                let mut b =
                    self.make_fraction(numerator, denominator, *thickness, (*left, *right), style);
                tag_delimiters(&mut b, atom.delimiter_tags);
                (b, 0.0, false)
            }
            Nucleus::BigDelimiter { delim, sizing } => {
                (self.make_big_delimiter(*delim, *sizing), 0.0, false)
            }
            Nucleus::Phantom {
                body,
                horizontal,
                vertical,
            } => (
                self.make_phantom(body, *horizontal, *vertical, style),
                0.0,
                false,
            ),
            Nucleus::SubArray { rows, align } => {
                (self.make_subarray(rows, *align, style), 0.0, false)
            }
            Nucleus::ExtArrow {
                left,
                fill,
                right,
                kerns,
                above,
                below,
            } => (
                self.make_ext_arrow([*left, *fill, *right], *kerns, above, below, style),
                0.0,
                false,
            ),
            // Scripted glue (not a TeX construct): a kern carrying the scripts.
            Nucleus::Glue { mu, pt, .. } => {
                (MathBox::kern(mu * self.params(style).mu() + pt), 0.0, false)
            }
            Nucleus::Radical { radicand, degree } => (
                self.make_radical(radicand, degree.as_ref(), style),
                0.0,
                false,
            ),
            Nucleus::Accent { accent, base } => {
                // TeX moves scripts of an accented single character under
                // the accent (`make_math_accent`); the accent then clears the
                // scripted box. Otherwise scripts follow the accented base.
                if let Some(b) = self.accent_over_scripted_char(*accent, base, atom, style) {
                    return b;
                }
                (self.make_accent(*accent, base, None, style), 0.0, false)
            }
            Nucleus::Delimited { left, right, body } => {
                let mut b = self.make_left_right(*left, *right, body, style);
                tag_delimiters(&mut b, atom.delimiter_tags);
                (b, 0.0, false)
            }
            Nucleus::Brace { body, under } => (self.make_brace(body, *under), 0.0, false),
            Nucleus::OverArrow {
                left,
                fill,
                right,
                body,
                under,
                gap,
            } => (
                self.make_over_arrow([*left, *fill, *right], body, *under, *gap, style),
                0.0,
                false,
            ),
            Nucleus::MeasuredAccent {
                narrow,
                wide,
                threshold,
                base,
            } => {
                // `\@mathmeasure\z@\textstyle{#1}`: a trial box whose glyph
                // lookups are not reported a second time.
                let reported = self.limitations.len();
                let measured = self.clean_box(base, Style::TEXT).width;
                self.limitations.truncate(reported);
                let accent = if measured > *threshold {
                    *wide
                } else {
                    *narrow
                };
                let inner = Atom {
                    nucleus: Nucleus::Accent {
                        accent,
                        base: base.clone(),
                    },
                    ..atom.clone()
                };
                return self.atom(&inner, class, style, false);
            }
            Nucleus::Text(text) => (self.make_text(text, style), 0.0, false),
            Nucleus::TextRun(pieces) => (self.make_text_run(pieces, style), 0.0, false),
            Nucleus::Overline(body) => (self.make_over(body, style), 0.0, false),
            Nucleus::Underline(body) => (self.make_under(body, style), 0.0, false),
            Nucleus::Styled { style: inner, body } => (self.clean_box(body, *inner), 0.0, false),
        };
        let mut nucleus = nucleus;
        nucleus.inherit_tag(unpacked_tag);
        self.make_scripts(nucleus, delta, is_char, atom, style)
    }

    /// `make_math_accent` when the atom has scripts and its base is one
    /// character: the scripts join the nucleus and the accent is lifted over
    /// the resulting box (δ grows by the height gained).
    fn accent_over_scripted_char(
        &mut self,
        accent: char,
        base: &MathList,
        atom: &Atom,
        style: Style,
    ) -> Option<MathBox> {
        if atom.superscript.is_none() && atom.subscript.is_none() {
            return None;
        }
        let [
            Atom {
                nucleus: Nucleus::Symbol(ch),
                superscript: None,
                subscript: None,
                left_scripts: None,
                ..
            },
        ] = base.atoms.as_slice()
        else {
            return None;
        };
        let scripted = Atom {
            class: AtomClass::Ord,
            nucleus: Nucleus::Symbol(*ch),
            superscript: atom.superscript.clone(),
            subscript: atom.subscript.clone(),
            limits: Limits::default(),
            // The character keeps its own provenance; the accent glyph
            // inherits the accent atom's when the caller's list tags it.
            tag: base.atoms[0].tag,
            delimiter_tags: [SourceTag::NONE; 2],
            left_scripts: None,
        };
        let g = self.m.glyph(*ch, style.size_class())?;
        Some(self.make_accent(accent, &MathList::from(scripted), Some((*ch, g)), style))
    }

    /// Upright operator text: roman glyphs side by side. Characters followed
    /// by another character of the same font are `math_text_char`s of a font
    /// with a nonzero space, so they get no italic correction (tex.web §752);
    /// the last one is a plain `math_char` and keeps it (pdfTeX \showbox:
    /// `\kern0.05731` after `lim` in cmr12).
    fn make_text(&mut self, text: &str, style: Style) -> MathBox {
        let size = style.size_class();
        let mut chars: Vec<char> = text.chars().collect();
        let mut items = Vec::new();
        let mut last_italic = 0.0;
        let mut steps = 0;
        // The run's characters are math characters of one family, so
        // `make_ord` (tex.web §752) applies between every two of them: the
        // font's kerns and ligatures. A `|=:|>>` character does not combine.
        let mut no_combine = false;
        let mut i = 0;
        while i < chars.len() {
            let mut kern = 0.0;
            while !std::mem::take(&mut no_combine) {
                let Some(&next) = chars.get(i + 1) else { break };
                let Some(pair) =
                    self.m
                        .ord_pair(MathChar::Text(chars[i]), MathChar::Text(next), size)
                else {
                    break;
                };
                match pair.ligature {
                    None => kern = pair.kern,
                    Some(OrdLigature {
                        op,
                        ch: MathChar::Text(ch),
                    }) if steps < LIGATURE_LIMIT => {
                        steps += 1;
                        match op {
                            1 | 5 => chars[i] = ch,
                            2 | 6 => chars[i + 1] = ch,
                            3 | 7 | 11 => chars.insert(i + 1, ch),
                            _ => {
                                chars[i] = ch;
                                chars.remove(i + 1);
                            }
                        }
                        if op <= 3 {
                            continue;
                        }
                        no_combine = op == 11;
                    }
                    Some(_) => {}
                }
                break;
            }
            match self.m.text_glyph(chars[i], size) {
                Some(g) => {
                    last_italic = g.italic;
                    items.push(MathBox::glyph(&g));
                    if kern != 0.0 {
                        items.push(MathBox::kern(kern));
                    }
                }
                None => self.limitations.push(Limitation::MissingGlyph(chars[i])),
            }
            i += 1;
        }
        if last_italic != 0.0 {
            items.push(MathBox::kern(last_italic));
        }
        MathBox::hlist(items)
    }

    fn make_text_styled(&mut self, text: &str, style: Style, text_style: TextStyle) -> MathBox {
        let mut items = Vec::new();
        let mut last_italic = 0.0;
        for ch in text.chars() {
            if ch == ' ' {
                items.push(MathBox::kern(self.m.text_space(style.size_class())));
                last_italic = 0.0;
                continue;
            }
            match self
                .m
                .text_glyph_with_style(ch, style.size_class(), text_style)
            {
                Some(g) => {
                    last_italic = g.italic;
                    items.push(MathBox::glyph(&g));
                }
                None => self.limitations.push(Limitation::MissingGlyph(ch)),
            }
        }
        if last_italic != 0.0 {
            items.push(MathBox::kern(last_italic));
        }
        MathBox::hlist(items)
    }

    fn make_text_run(&mut self, pieces: &[TextPiece], style: Style) -> MathBox {
        let inline_style = Style {
            level: match style.level {
                crate::style::StyleLevel::Display | crate::style::StyleLevel::Text => {
                    crate::style::StyleLevel::Text
                }
                crate::style::StyleLevel::Script => crate::style::StyleLevel::Script,
                crate::style::StyleLevel::ScriptScript => crate::style::StyleLevel::ScriptScript,
            },
            cramped: style.cramped,
        };
        let boxes = pieces
            .iter()
            .map(|piece| match piece {
                TextPiece::Text { text, style } => {
                    self.make_text_styled(text, inline_style, *style)
                }
                TextPiece::Math(list) => self.list(list, inline_style),
            })
            .collect();
        MathBox::hlist(boxes)
    }

    /// Rule 9 and `make_over`: `overbar(x, 3θ, θ)` with x in cramped style.
    fn make_over(&mut self, body: &MathList, style: Style) -> MathBox {
        let theta = self.params(style).default_rule_thickness;
        let x = self.clean_box(body, style.cramped());
        overbar(x, 3.0 * theta, theta)
    }

    /// Rule 10 and `make_under`: x, kern 3θ, rule θ, and θ of extra depth.
    fn make_under(&mut self, body: &MathList, style: Style) -> MathBox {
        let theta = self.params(style).default_rule_thickness;
        let x = self.clean_box(body, style);
        let w = x.width;
        let rule_dy = x.depth + 3.0 * theta + theta;
        MathBox {
            tag: SourceTag::NONE,
            width: w,
            height: x.height,
            depth: x.depth + 3.0 * theta + theta + theta,
            kind: BoxKind::VBox(vec![
                Child {
                    dx: 0.0,
                    dy: 0.0,
                    content: x,
                },
                Child {
                    dx: 0.0,
                    dy: rule_dy,
                    content: MathBox::rule(w, theta, 0.0),
                },
            ]),
        }
    }

    /// Rule 13 / 13a and `make_op`.
    fn make_op(&mut self, atom: &Atom, style: Style) -> MathBox {
        let p = self.params(style);
        let limits = match atom.limits {
            Limits::DisplayLimits => style.is_display(),
            Limits::Limits => true,
            Limits::NoLimits => false,
        };
        let (unpacked_nucleus, unpacked_tag) = unpacked(&atom.nucleus);
        let (nucleus, delta) = match unpacked_nucleus {
            n @ (Nucleus::Symbol(_) | Nucleus::TextChar(_)) => {
                let mut g = match n {
                    Nucleus::Symbol(ch) => self.glyph(*ch, style),
                    Nucleus::TextChar(ch) => self.text_char(*ch, style),
                    _ => None,
                };
                if let (true, Nucleus::Symbol(ch)) = (style.is_display(), n)
                    && let Some(large) = self.m.large_operator(*ch, style.size_class())
                {
                    g = Some(large);
                }
                match g {
                    Some(g) => {
                        // clean_box of a char includes its italic correction;
                        // it is removed again when a subscript must tuck under.
                        let keep_italic = !(atom.subscript.is_some() && !limits);
                        let mut x = if keep_italic && g.italic != 0.0 {
                            MathBox::hlist(vec![MathBox::glyph(&g), MathBox::kern(g.italic)])
                        } else {
                            MathBox::glyph(&g)
                        };
                        let shift = (x.height - x.depth) / 2.0 - p.axis_height;
                        x = x.shifted(shift);
                        (x, g.italic)
                    }
                    None => (MathBox::empty(), 0.0),
                }
            }
            Nucleus::List(list) => (self.clean_box(list, style), 0.0),
            other => {
                let inner = Atom {
                    class: AtomClass::Ord,
                    nucleus: other.clone(),
                    superscript: None,
                    subscript: None,
                    limits: Limits::default(),
                    tag: atom.tag,
                    delimiter_tags: atom.delimiter_tags,
                    left_scripts: None,
                };
                (self.atom(&inner, AtomClass::Ord, style, false), 0.0)
            }
        };
        let mut nucleus = nucleus;
        nucleus.inherit_tag(unpacked_tag);
        self.op_scripts(nucleus, delta, limits, atom, style)
    }

    /// amsmath `\sideset{#1}{#2}{#3}` (`amsmath.sty` 921-929) for an atom
    /// that is `#3` with its scripts `#2` and `left` `#1` (see
    /// [`Atom::left_scripts`]): `\mathop{\box4\box6}` without the
    /// `\kern-\dimen@` that cancels the ordinary `\hbox to\dimen@{}`
    /// the list's builder puts in front.
    fn make_sideset(&mut self, atom: &Atom, left: &LeftScripts) -> MathBox {
        // Every `\@mathmeasure` is a fresh `$\displaystyle ..$` in an
        // `\hbox`: display style (text size, uncramped) at any depth.
        let style = Style::DISPLAY;
        let bare = |superscript: Option<MathList>, subscript: Option<MathList>| Atom {
            class: AtomClass::Op,
            superscript,
            subscript,
            limits: Limits::NoLimits,
            left_scripts: None,
            ..atom.clone()
        };
        // `\@mathmeasure\z@\displaystyle{#3}`: only box 0's height and
        // depth are used, so its glyph lookups are reported once, by box 6.
        let reported = self.limitations.len();
        let measured = self.make_op(&bare(None, None), style);
        self.limitations.truncate(reported);
        // `\vbox to\ht\z@{}\dp\@ne\dp\z@`, then `{\copy\tw@#1}`: a box
        // nucleus, so Rule 18a starts the shifts from its height and depth.
        let strut = MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::VBox(Vec::new()),
            width: 0.0,
            height: measured.height,
            depth: measured.depth,
        };
        let carrier = Atom {
            class: AtomClass::Ord,
            nucleus: Nucleus::Empty,
            superscript: left.superscript.clone(),
            subscript: left.subscript.clone(),
            limits: Limits::default(),
            tag: SourceTag::NONE,
            delimiter_tags: [SourceTag::NONE; 2],
            left_scripts: None,
        };
        let left_box = self.make_scripts(strut, 0.0, false, &carrier, style);
        // `\@mathmeasure6\displaystyle{#3\nolimits#2}`.
        let right = bare(atom.superscript.clone(), atom.subscript.clone());
        let right_box = self.make_op(&right, style);
        MathBox::hlist(vec![left_box, right_box])
    }

    /// The rest of `make_op` once the nucleus is boxed: scripts beside it,
    /// or Rule 13a limits over and under it.
    fn op_scripts(
        &mut self,
        nucleus: MathBox,
        delta: f64,
        limits: bool,
        atom: &Atom,
        style: Style,
    ) -> MathBox {
        let p = self.params(style);
        if !limits {
            return self.make_scripts(nucleus, delta, false, atom, style);
        }
        // Rule 13a: limits above and below, centred, italic-shifted by δ/2.
        let x = atom
            .superscript
            .as_ref()
            .map(|s| self.clean_box(s, style.sup()));
        let z = atom
            .subscript
            .as_ref()
            .map(|s| self.clean_box(s, style.sub()));
        let y = nucleus;
        let mut w = y.width;
        if let Some(x) = &x {
            w = w.max(x.width);
        }
        if let Some(z) = &z {
            w = w.max(z.width);
        }
        let y = y.rebox(w);
        let mut children = Vec::new();
        let mut height = y.height;
        let mut depth = y.depth;
        if let Some(x) = x {
            let x = x.rebox(w);
            let shift_up = (p.big_op_spacing3 - x.depth).max(p.big_op_spacing1);
            // x's baseline sits shift_up + d(x) above y's top.
            let dy = -(y.height + shift_up + x.depth);
            height = y.height + shift_up + x.depth + x.height + p.big_op_spacing5;
            children.push(Child {
                dx: delta / 2.0,
                dy,
                content: x,
            });
        }
        children.push(Child {
            dx: 0.0,
            dy: 0.0,
            content: y,
        });
        if let Some(z) = z {
            let z = z.rebox(w);
            let shift_down = (p.big_op_spacing4 - z.height).max(p.big_op_spacing2);
            let dy = depth + shift_down + z.height;
            depth = depth + shift_down + z.height + z.depth + p.big_op_spacing5;
            children.push(Child {
                dx: -delta / 2.0,
                dy,
                content: z,
            });
        }
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::VBox(children),
            width: w,
            height,
            depth,
        }
    }

    /// One piece of an `\arrowfill@` in `\displaystyle`: the relation's
    /// character box (with its italic correction), smashed for the minus
    /// (`\relbar` = `\mathrel{\mathpalette\mathsm@sh\std@minus}`).
    fn arrow_piece(&mut self, ch: char, style: Style) -> MathBox {
        let Some(g) = self.glyph(ch, style) else {
            return MathBox::empty();
        };
        let mut b = if g.italic != 0.0 {
            MathBox::hlist(vec![MathBox::glyph(&g), MathBox::kern(g.italic)])
        } else {
            MathBox::hlist(vec![MathBox::glyph(&g)])
        };
        if ch == '-' || ch == '\u{2212}' {
            b.height = 0.0;
            b.depth = 0.0;
        }
        b
    }

    /// amsmath `\ext@arrow` (see [`Nucleus::ExtArrow`]).
    fn make_ext_arrow(
        &mut self,
        pieces: [char; 3],
        kerns: [f64; 4],
        above: &MathList,
        below: &MathList,
        style: Style,
    ) -> MathBox {
        let script_mu = self.params(Style::SCRIPT).mu();
        // `\hbox{$\scriptstyle\mkern#3mu{#6}\mkern#4mu$}`: `{#6}` is a
        // sub-formula, whose `clean_box` drops a lone character's italic
        // correction (tex.web §721).
        let label = |this: &mut Self, list: &MathList| -> f64 {
            let group = match list.atoms.as_slice() {
                [a] if a.superscript.is_none()
                    && a.subscript.is_none()
                    && a.class != AtomClass::Op =>
                {
                    match a.nucleus {
                        Nucleus::Symbol(ch) => this
                            .m
                            .glyph(ch, Style::SCRIPT.size_class())
                            .map(|g| g.width),
                        _ => None,
                    }
                }
                _ => None,
            };
            let group = group.unwrap_or_else(|| this.clean_box(list, Style::SCRIPT).width);
            (kerns[2] + kerns[3]) * script_mu + group
        };
        let min_width = label(self, below).max(label(self, above));
        // `\setbox\z@\hbox{#5\displaystyle}` widened to the labels.
        let nucleus = self.arrow_fill(pieces, min_width, Style::DISPLAY);
        // `\mathop{..}\limits^{\mkern#1mu #7\mkern#2mu}_{\mkern#1mu #6\mkern#2mu}`,
        // each script only when its label is non-empty (`\if0#1` omits a 0 kern).
        let script = |label: &MathList| -> Option<MathList> {
            if label.is_empty() {
                return None;
            }
            let mut atoms = Vec::with_capacity(label.atoms.len() + 2);
            if kerns[0] != 0.0 {
                atoms.push(Atom::glue(kerns[0], 0.0));
            }
            atoms.extend(label.atoms.iter().cloned());
            if kerns[1] != 0.0 {
                atoms.push(Atom::glue(kerns[1], 0.0));
            }
            Some(MathList::new(atoms))
        };
        let op = Atom {
            class: AtomClass::Op,
            nucleus: Nucleus::Empty,
            superscript: script(above),
            subscript: script(below),
            limits: Limits::Limits,
            tag: SourceTag::NONE,
            delimiter_tags: [SourceTag::NONE; 2],
            left_scripts: None,
        };
        self.op_scripts(nucleus, 0.0, true, &op, style)
    }

    /// `\arrowfill@#1#2#3#4` (`amsmath.sty` 971-976) in `style`: `#1`,
    /// `\mkern-7mu`, `\cleaders\hbox{$\mkern-2mu#2\mkern-2mu$}\hfill`,
    /// `\mkern-7mu`, `#3`, every muskip zero, packed to the larger of its
    /// natural width (the `\hfill` has none) and `min_width`.
    fn arrow_fill(&mut self, pieces: [char; 3], min_width: f64, style: Style) -> MathBox {
        let mu = self.params(style).mu();
        let left = self.arrow_piece(pieces[0], style);
        let right = self.arrow_piece(pieces[2], style);
        let natural = left.width - 14.0 * mu + right.width;
        let width = natural.max(min_width);
        let stretch = width - natural;
        let mut children = Vec::new();
        let mut x = 0.0;
        let (mut height, mut depth) = (
            left.height.max(right.height).max(0.0),
            left.depth.max(right.depth).max(0.0),
        );
        children.push(Child {
            dx: x,
            dy: 0.0,
            content: left,
        });
        x += children[0].content.width - 7.0 * mu;
        if stretch > 0.0 {
            // `\cleaders`: as many whole fill boxes as fit in the glue (plus
            // TeX's 10sp rounding allowance), centred (tex.web §626).
            let fill = self.arrow_piece(pieces[1], style);
            let sp = |v: f64| (v * 65536.0).round() as i64;
            let leader = sp(fill.width - 4.0 * mu);
            let rule = sp(stretch) + 10;
            if leader > 0 {
                height = height.max(fill.height);
                depth = depth.max(fill.depth);
                let (n, lr) = (rule / leader, rule % leader);
                for i in 0..n {
                    children.push(Child {
                        dx: x + (lr / 2 + i * leader) as f64 / 65536.0 - 2.0 * mu,
                        dy: 0.0,
                        content: fill.clone(),
                    });
                }
            }
        }
        x += stretch - 7.0 * mu;
        children.push(Child {
            dx: x,
            dy: 0.0,
            content: right,
        });
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::HBox(children),
            width,
            height,
            depth,
        }
    }

    /// amsmath `\overarrow@#1#2#3` = `\vbox{\ialign{##\crcr#1#2\crcr
    /// \noalign{\nointerlineskip}$\m@th\hfil#2#3\hfil$\crcr}}` and
    /// `\underarrow@` = `\vtop{\ialign{##\crcr$\m@th\hfil#2#3\hfil$\crcr
    /// \noalign{\nointerlineskip\kern1.3\ex@}#1#2\crcr}}` (`amsmath.sty`
    /// 983-1006): `#2` is the `\mathpalette` style (never cramped), the
    /// arrow row an `\arrowfill@` as wide as the column.
    fn make_over_arrow(
        &mut self,
        pieces: [char; 3],
        body: &MathList,
        under: bool,
        gap: f64,
        style: Style,
    ) -> MathBox {
        let style = Style {
            cramped: false,
            ..style
        };
        let x = self.clean_box(body, style);
        let fill = self.arrow_fill(pieces, x.width, style);
        let x = x.rebox(fill.width);
        if under {
            MathBox::vtop(vec![(0.0, x), (0.0, MathBox::kern(gap)), (0.0, fill)])
        } else {
            MathBox::vbox(vec![(0.0, fill), (0.0, x)])
        }
    }

    /// `\overbrace` / `\underbrace` (see [`Nucleus::Brace`]): the brace row
    /// is `\hbox to` the column width of cmex `\braceld` "7A, `\bracerd`
    /// "7B, `\bracelu` "7C and `\braceru` "7D (`fontmath.ltx` 447-456) with
    /// two `\leaders\vrule height \ht(\braceld) depth 0pt \hfill`.
    fn make_brace(&mut self, body: &MathList, under: bool) -> MathBox {
        let x = self.clean_box(body, Style::DISPLAY);
        let ch = if under { '\u{23DF}' } else { '\u{23DE}' };
        // `\downbracefill`: ld, fill, ru, lu, fill, rd;
        // `\upbracefill`:   lu, fill, rd, ld, fill, ru.
        let codes: [u8; 4] = if under {
            [0x7C, 0x7B, 0x7A, 0x7D]
        } else {
            [0x7A, 0x7D, 0x7C, 0x7B]
        };
        // `\downbracefill`/`\upbracefill` set `\braceld`..`\braceru` inside
        // their own `$...$` (plain.tex 352-360), so the pieces come from
        // `\textfont3` of the current math size however deeply the brace is
        // nested in scripts — never `\scriptfont3`. pdfTeX `\showbox` of an
        // 11 pt article loading amsmath (where the two differ: `\textfont3`
        // is `cmex10 at 10.95pt`, `\scriptfont3` is `cmex8`) gives the same
        // 1.31396 pt piece from cmex10 at 10.95 pt for `\overbrace{a+b}`,
        // `x^{\overbrace{a+b}}`, `x^{y^{\overbrace{a+b}}}` and an
        // `\underbrace` in a fraction numerator; in `\footnotesize` all of
        // them come from cmex9 instead. Hence [`SizeClass::Text`], not the
        // style's own size class.
        let at = crate::metrics::SizeClass::Text;
        let pieces: Option<Vec<Glyph>> = codes
            .iter()
            .map(|c| self.m.extension_glyph(*c, ch, at))
            .collect();
        let (Some(pieces), Some(ld)) = (pieces, self.m.extension_glyph(0x7A, ch, at)) else {
            self.limitations.push(Limitation::MissingGlyph(ch));
            return x;
        };
        let natural: f64 = pieces.iter().map(|g| g.width).sum();
        let w = natural.max(x.width);
        let half = (w - natural) / 2.0;
        let mut row = Vec::with_capacity(6);
        for (i, g) in pieces.iter().enumerate() {
            row.push(MathBox::glyph(g));
            if (i == 0 || i == 2) && half > 0.0 {
                row.push(MathBox::rule(half, ld.height, 0.0));
            }
        }
        let row = MathBox::hlist(row);
        let x = x.rebox(w);
        let kern = || MathBox::kern(3.0);
        if under {
            MathBox::vtop(vec![(0.0, x), (0.0, kern()), (0.0, row), (0.0, kern())])
        } else {
            MathBox::vbox(vec![(0.0, kern()), (0.0, row), (0.0, kern()), (0.0, x)])
        }
    }

    /// Rule 18 and `make_scripts`.
    fn make_scripts(
        &mut self,
        nucleus: MathBox,
        delta: f64,
        is_char: bool,
        atom: &Atom,
        style: Style,
    ) -> MathBox {
        if atom.superscript.is_none() && atom.subscript.is_none() {
            return nucleus;
        }
        let p = self.params(style);
        let t = self.params(style.sup());
        let (mut shift_up, mut shift_down) = if is_char {
            (0.0, 0.0)
        } else {
            (nucleus.height - t.sup_drop, nucleus.depth + t.sub_drop)
        };
        let Some(sup) = &atom.superscript else {
            // Rule 18b: subscript only.
            let mut x = self.clean_box(atom.subscript.as_ref().unwrap(), style.sub());
            x.width += p.script_space;
            shift_down = shift_down.max(p.sub1);
            let clr = x.height - p.x_height.abs() * 4.0 / 5.0;
            shift_down = shift_down.max(clr);
            return MathBox::hbox(vec![(0.0, nucleus), (shift_down, x)]);
        };
        // Rule 18c: superscript.
        let mut x = self.clean_box(sup, style.sup());
        x.width += p.script_space;
        let clr = if style.cramped {
            p.sup3
        } else if style.is_display() {
            p.sup1
        } else {
            p.sup2
        };
        shift_up = shift_up.max(clr);
        let clr = x.depth + p.x_height.abs() / 4.0;
        shift_up = shift_up.max(clr);
        let Some(sub) = &atom.subscript else {
            return MathBox::hbox(vec![(0.0, nucleus), (-shift_up, x)]);
        };
        // Rule 18d/e: both scripts.
        let mut y = self.clean_box(sub, style.sub());
        y.width += p.script_space;
        shift_down = shift_down.max(p.sub2);
        let clr = 4.0 * p.default_rule_thickness - ((shift_up - x.depth) - (y.height - shift_down));
        if clr > 0.0 {
            shift_down += clr;
            let clr = p.x_height.abs() * 4.0 / 5.0 - (shift_up - x.depth);
            if clr > 0.0 {
                shift_up += clr;
                shift_down -= clr;
            }
        }
        let width = (x.width + delta).max(y.width);
        let height = x.height + shift_up;
        let depth = y.depth + shift_down;
        let scripts = MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::VBox(vec![
                Child {
                    dx: delta,
                    dy: -shift_up,
                    content: x,
                },
                Child {
                    dx: 0.0,
                    dy: shift_down,
                    content: y,
                },
            ]),
            width,
            height,
            depth,
        };
        MathBox::hbox(vec![(0.0, nucleus), (0.0, scripts)])
    }

    /// Rule 15 and `make_fraction`, with null delimiters on both sides.
    fn make_fraction(
        &mut self,
        num: &MathList,
        den: &MathList,
        thickness: Option<f64>,
        delims: (Option<char>, Option<char>),
        style: Style,
    ) -> MathBox {
        let p = self.params(style);
        let theta = thickness.unwrap_or(p.default_rule_thickness);
        let mut x = self.clean_box(num, style.num());
        let mut z = self.clean_box(den, style.denom());
        let (mut u, mut v) = if style.is_display() {
            (p.num1, p.denom1)
        } else if theta != 0.0 {
            (p.num2, p.denom2)
        } else {
            (p.num3, p.denom2)
        };
        if x.width < z.width {
            x = x.rebox(z.width);
        } else {
            z = z.rebox(x.width);
        }
        let a = p.axis_height;
        if theta == 0.0 {
            let clr = if style.is_display() { 7.0 } else { 3.0 } * p.default_rule_thickness;
            let delta = (clr - ((u - x.depth) - (z.height - v))) / 2.0;
            if delta > 0.0 {
                u += delta;
                v += delta;
            }
        } else {
            let clr = if style.is_display() {
                3.0 * theta
            } else {
                theta
            };
            let delta = theta / 2.0;
            let delta1 = clr - ((u - x.depth) - (a + delta));
            let delta2 = clr - ((a - delta) - (z.height - v));
            if delta1 > 0.0 {
                u += delta1;
            }
            if delta2 > 0.0 {
                v += delta2;
            }
        }
        let w = x.width;
        let mut children = vec![Child {
            dx: 0.0,
            dy: -u,
            content: x,
        }];
        if theta != 0.0 {
            // The rule is centred on the axis: from a+θ/2 to a−θ/2.
            children.push(Child {
                dx: 0.0,
                dy: -(a - theta / 2.0),
                content: MathBox::rule(w, theta, 0.0),
            });
        }
        let height = children[0].content.height + u;
        let depth = z.depth + v;
        children.push(Child {
            dx: 0.0,
            dy: v,
            content: z,
        });
        let body = MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::VBox(children),
            width: w,
            height,
            depth,
        };
        // Rule 15e: delimiters of size `\delim1` (display) or `\delim2`,
        // centred on the axis; a null delimiter is `\nulldelimiterspace`.
        let delta = if style.is_display() {
            p.delim1
        } else {
            p.delim2
        };
        let open = self.left_right_delimiter(delims.0, delta, style, &p);
        let close = self.left_right_delimiter(delims.1, delta, style, &p);
        MathBox::hlist(vec![open, body, close])
    }

    /// `\big`/`\Big`/`\bigg`/`\Bigg` under either definition ([`BigSizing`]).
    /// The inner formula is text style whatever the outer style is.
    ///
    /// Both spellings are `\left<delim><empty box><no delimiter>`, so the
    /// delimiter comes from Rule 19 applied to that empty box; the two
    /// differ in the box.
    ///
    /// * amsmath (`\vcenter to <factor>\big@size{}`) straddles the axis:
    ///   height `s/2 + a`, depth `s/2 - a`, so δ = `s/2` and the target
    ///   `2δ` is `s` itself.
    /// * the kernel (`\vbox to <pt>{}`) has height `pt` and depth **0**, so
    ///   δ = `max(pt - a, a)` and the target is `2 max(pt - a, a)`. For
    ///   `\Big` at 10pt that is `2 max(11.5 - 2.5, 2.5)` = 18pt, which is
    ///   why the kernel's `\Big[` lands on `cmex` `h` (18.00017pt) at every
    ///   body size while amsmath's follows the math size.
    fn make_big_delimiter(&mut self, delim: Option<char>, sizing: BigSizing) -> MathBox {
        let p = self.params(Style::TEXT);
        let a = p.axis_height;
        // `target` is Rule 19's 2δ; `(vh, vd)` the empty box's own height
        // and depth, which the result is still at least as large as.
        let (target, vh, vd) = match sizing {
            BigSizing::Amsmath { factor } => {
                let strut = self
                    .m
                    .text_glyph('(', crate::metrics::SizeClass::Text)
                    .map(|g| g.height + g.depth)
                    .unwrap_or(p.size);
                let s = factor * 1.2 * strut;
                (s, s / 2.0 + a, s / 2.0 - a)
            }
            BigSizing::Kernel { pt } => (2.0 * (pt - a).max(a), pt, 0.0),
        };
        let mut items = Vec::new();
        if let Some(ch) = delim {
            let wanted = (target * p.delimiter_factor).max(target - p.delimiter_shortfall);
            let d = self.left_right_delimiter(Some(ch), wanted, Style::TEXT, &p);
            items.push(d);
        }
        let mut b = MathBox::hlist(items);
        b.height = b.height.max(vh);
        b.depth = b.depth.max(vd);
        b
    }

    /// `\finph@nt`: an empty box taking the chosen dimensions of `body`,
    /// which `\mathpalette` sets in the current style without cramping.
    fn make_phantom(
        &mut self,
        body: &MathList,
        horizontal: bool,
        vertical: bool,
        style: Style,
    ) -> MathBox {
        let b = self.clean_box(
            body,
            Style {
                cramped: false,
                ..style
            },
        );
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::HBox(Vec::new()),
            width: if horizontal { b.width } else { 0.0 },
            height: if vertical { b.height } else { 0.0 },
            depth: if vertical { b.depth } else { 0.0 },
        }
    }

    /// amsmath `subarray` (see [`Nucleus::SubArray`]): an `\ialign` of
    /// `\scriptstyle` rows with interline glue, then Rule 8's `\vcenter`
    /// on the current style's axis.
    fn make_subarray(&mut self, rows: &[MathList], align: char, style: Style) -> MathBox {
        let sp = self.params(Style::SCRIPT);
        let baselineskip = sp.num3 + sp.denom2;
        let lineskip = 3.0 * sp.default_rule_thickness;
        let cells: Vec<MathBox> = rows
            .iter()
            .map(|r| self.clean_box(r, Style::SCRIPT))
            .collect();
        let width = cells.iter().map(|c| c.width).fold(0.0, f64::max);
        let mut baselines = Vec::with_capacity(cells.len());
        let mut y = 0.0;
        for (i, c) in cells.iter().enumerate() {
            if i == 0 {
                y = c.height;
            } else {
                let prev_depth = cells[i - 1].depth;
                let glue = baselineskip - prev_depth - c.height;
                y += prev_depth + c.height + if glue < lineskip { lineskip } else { glue };
            }
            baselines.push(y);
        }
        let total = y + cells.last().map_or(0.0, |c| c.depth);
        let a = self.params(style).axis_height;
        let height = total / 2.0 + a;
        let depth = total / 2.0 - a;
        let children = cells
            .into_iter()
            .zip(baselines)
            .map(|(c, base)| {
                let slack = width - c.width;
                let dx = match align {
                    'c' => slack / 2.0,
                    'r' => slack,
                    _ => 0.0,
                };
                Child {
                    dx,
                    dy: base - height,
                    content: c,
                }
            })
            .collect();
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::VBox(children),
            width,
            height,
            depth,
        }
    }

    /// `var_delimiter` without its final axis shift: the first glyph in
    /// `sizes` at least `wanted` tall, else the largest (reported).
    fn var_delimiter(
        &mut self,
        sizes: &[Glyph],
        extensible: Option<Extensible>,
        wanted: f64,
        on_missing: impl FnOnce(f64, f64) -> Limitation,
    ) -> Option<MathBox> {
        // `char_box` widths include the italic correction; it is a kern
        // after the glyph so the glyph box itself keeps its TFM width.
        let char_box = |g: &Glyph| {
            if g.italic == 0.0 {
                MathBox::glyph(g)
            } else {
                MathBox::hlist(vec![MathBox::glyph(g), MathBox::kern(g.italic)])
            }
        };
        if let Some(chosen) = sizes.iter().find(|g| g.total_height() >= wanted) {
            return Some(char_box(chosen));
        }
        if let Some(recipe) = extensible {
            return Some(stack_extensible(&recipe, wanted));
        }
        let chosen = sizes.last()?;
        self.limitations
            .push(on_missing(wanted, chosen.total_height()));
        Some(char_box(chosen))
    }

    /// Rule 11 and `make_radical`.
    fn make_radical(
        &mut self,
        radicand: &MathList,
        degree: Option<&MathList>,
        style: Style,
    ) -> MathBox {
        let z = self.make_sqrt(radicand, style);
        let Some(degree) = degree else {
            return z;
        };
        // LaTeX `\r@@t`: \mkern5mu \raise.6(ht-dp) {scriptscript degree}
        // \mkern-10mu, then the radical box. `\m@th` sets the degree
        // uncramped in scriptscript style.
        let mu = self.params(style).mu();
        let r = self.clean_box(degree, Style::SCRIPT_SCRIPT);
        let raise = 0.6 * (z.height - z.depth);
        MathBox::hbox(vec![
            (0.0, MathBox::kern(5.0 * mu)),
            (-raise, r),
            (0.0, MathBox::kern(-10.0 * mu)),
            (0.0, z),
        ])
    }

    fn make_sqrt(&mut self, radicand: &MathList, style: Style) -> MathBox {
        let p = self.params(style);
        let x = self.clean_box(radicand, style.cramped());
        let theta = p.default_rule_thickness;
        let mut clr = if style.is_display() {
            theta + p.x_height.abs() / 4.0
        } else {
            theta + theta / 4.0
        };
        let wanted = x.height + x.depth + clr + theta;
        let sizes = self.m.radical_sizes(style.size_class());
        let ext = self.m.radical_extensible(style.size_class());
        let Some(y) = self.var_delimiter(&sizes, ext, wanted, |wanted, used| {
            Limitation::RadicalTooSmall { wanted, used }
        }) else {
            self.limitations.push(Limitation::MissingGlyph('\u{221A}'));
            return x;
        };
        let delta = y.depth - (x.height + x.depth + clr);
        if delta > 0.0 {
            clr += delta / 2.0;
        }
        // The sign's baseline is raised so its top meets the rule's top.
        let sign_dy = -(x.height + clr);
        let rule_thickness = y.height;
        let bar = MathBox::rule(x.width, rule_thickness, 0.0);
        // `overbar(x, clr, t)` = vpack(kern t, rule t, kern clr, x): TeX adds
        // an extra kern of the rule thickness above the rule, so the box is
        // taller than the sign by exactly one rule thickness.
        let overbar = MathBox {
            tag: SourceTag::NONE,
            width: x.width,
            height: x.height + clr + 2.0 * rule_thickness,
            depth: x.depth,
            kind: BoxKind::VBox(vec![
                Child {
                    dx: 0.0,
                    dy: -(x.height + clr),
                    content: bar,
                },
                Child {
                    dx: 0.0,
                    dy: 0.0,
                    content: x,
                },
            ]),
        };
        MathBox::hbox(vec![(sign_dy, y), (0.0, overbar)])
    }

    /// Rule 12 and `make_math_accent`.
    fn make_accent(
        &mut self,
        accent: char,
        base: &MathList,
        scripted_char: Option<(char, Glyph)>,
        style: Style,
    ) -> MathBox {
        let p = self.params(style);
        // tex.web §738/§742: a scripted character is re-boxed in the current
        // (uncramped) style after its scripts move under the accent; a plain
        // base is set cramped.
        let x = match scripted_char {
            Some(_) => self.clean_box(base, style),
            None => self.clean_box(base, style.cramped()),
        };
        let sizes = self.m.accent_sizes(accent, style.size_class());
        if sizes.is_empty() {
            self.limitations.push(Limitation::MissingAccent(accent));
            return x;
        }
        // The accent is centred over the bare character's width even when
        // scripts follow it (`w` is taken before the swap).
        let w = scripted_char.map(|(_, g)| g.width).unwrap_or(x.width);
        let mut h = x.height;
        // Skew only applies when the base is a single symbol (possibly with
        // scripts moved under the accent, see `accent_over_scripted_char`).
        let skew_of = |ch: char| {
            self.m
                .glyph(ch, style.size_class())
                .map(|g| g.skew)
                .unwrap_or(0.0)
        };
        let s = match (scripted_char, base.atoms.as_slice()) {
            (Some((ch, _)), _) => skew_of(ch),
            (
                None,
                [
                    Atom {
                        nucleus: Nucleus::Symbol(ch),
                        superscript: None,
                        subscript: None,
                        left_scripts: None,
                        ..
                    },
                ],
            ) => skew_of(*ch),
            _ => 0.0,
        };
        let mut chosen = &sizes[0];
        for g in &sizes[1..] {
            if g.width <= w {
                chosen = g;
            } else {
                break;
            }
        }
        // δ from the bare character's height, then grown by the height the
        // scripts added, so the accent clears them (make_math_accent).
        let delta = match scripted_char {
            Some((_, g)) => {
                let d = g.height.min(p.x_height) + (x.height - g.height);
                h = x.height;
                d
            }
            None => h.min(p.x_height),
        };
        let y = MathBox::glyph(chosen);
        // `char_box` widths include the italic correction (1.846pt for the
        // cmmi12 \vec accent), which TeX centres with; the box itself keeps
        // width 0 in TeX, so only the shift depends on it.
        let accent_dx = s + (w - (y.width + chosen.italic)) / 2.0;
        // Stack: accent, kern −δ, base; baseline at the base's baseline.
        let accent_dy = -(h - delta) - y.depth;
        let mut height = (h - delta) + y.depth + y.height;
        if height < h {
            height = h;
        }
        MathBox {
            tag: SourceTag::NONE,
            width: x.width,
            height,
            depth: x.depth,
            kind: BoxKind::VBox(vec![
                Child {
                    dx: accent_dx,
                    dy: accent_dy,
                    content: y,
                },
                Child {
                    dx: 0.0,
                    dy: 0.0,
                    content: x,
                },
            ]),
        }
    }

    /// Rule 19 and `make_left_right`.
    fn make_left_right(
        &mut self,
        left: Option<char>,
        right: Option<char>,
        body: &MathList,
        style: Style,
    ) -> MathBox {
        let p = self.params(style);
        let inner = self.clean_box(body, style);
        let a = p.axis_height;
        let delta1 = (inner.height - a).max(inner.depth + a);
        let wanted = (delta1 * 2.0 * p.delimiter_factor).max(2.0 * delta1 - p.delimiter_shortfall);
        let open = self.left_right_delimiter(left, wanted, style, &p);
        let close = self.left_right_delimiter(right, wanted, style, &p);
        MathBox::hlist(vec![open, inner, close])
    }

    fn left_right_delimiter(
        &mut self,
        ch: Option<char>,
        wanted: f64,
        style: Style,
        p: &MathParams,
    ) -> MathBox {
        let Some(ch) = ch else {
            return MathBox::kern(p.null_delimiter_space);
        };
        let sizes = self.m.delimiter_sizes(ch, style.size_class());
        let ext = self.m.delimiter_extensible(ch, style.size_class());
        match self.var_delimiter(&sizes, ext, wanted, |wanted, used| {
            Limitation::DelimiterTooSmall { ch, wanted, used }
        }) {
            // Centre the delimiter on the axis (`var_delimiter`'s last step).
            Some(b) => {
                let shift = (b.height - b.depth) / 2.0 - p.axis_height;
                b.shifted(shift)
            }
            None => {
                self.limitations.push(Limitation::MissingGlyph(ch));
                MathBox::kern(p.null_delimiter_space)
            }
        }
    }
}

/// Gives the delimiter boxes of a `[open, body, close]` hlist
/// (`make_fraction`, `make_left_right`) their own commands' provenance.
fn tag_delimiters(b: &mut MathBox, tags: [SourceTag; 2]) {
    if tags.iter().all(SourceTag::is_none) {
        return;
    }
    if let BoxKind::HBox(children) = &mut b.kind
        && let [open, _, close] = children.as_mut_slice()
    {
        open.content.inherit_tag(tags[0]);
        close.content.inherit_tag(tags[1]);
    }
}

/// `overbar(b, k, t)`: vpack(kern t, rule t, kern k, b); baseline of `b`.
fn overbar(b: MathBox, k: f64, t: f64) -> MathBox {
    let w = b.width;
    MathBox {
        tag: SourceTag::NONE,
        width: w,
        height: b.height + k + 2.0 * t,
        depth: b.depth,
        kind: BoxKind::VBox(vec![
            Child {
                dx: 0.0,
                dy: -(b.height + k),
                content: MathBox::rule(w, t, 0.0),
            },
            Child {
                dx: 0.0,
                dy: 0.0,
                content: b,
            },
        ]),
    }
}

/// tex.web §713: stack `bot`, n×`rep`, `mid`, n×`rep`, `top` until the total
/// reaches `wanted`. The box's baseline is the top piece's baseline
/// (`height = h(top piece)`, `depth = total − height`), as TeX's vlist
/// packing gives, so the caller's centring/raising arithmetic is unchanged.
fn stack_extensible(r: &Extensible, wanted: f64) -> MathBox {
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
    // Top to bottom.
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
        tag: SourceTag::NONE,
        width: r.rep.width + r.rep.italic,
        height,
        depth: w - height,
        kind: BoxKind::VBox(children),
    }
}
