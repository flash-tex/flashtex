//! Source spans and attribute ids carried from atoms to placed glyphs and
//! rules. Tags never change geometry: every fixture lays out to the same
//! positions, bit for bit, with and without them.

use flashtex_math_layout::{
    Atom, AtomClass, CmMathMetrics, MathBox, MathList, Nucleus, PositionedRuns, SourceSpan,
    SourceTag, Style, fixtures, layout, positioned_runs,
};

fn cm() -> CmMathMetrics {
    CmMathMetrics::latex_10pt()
}

fn sp(start: usize, end: usize) -> SourceTag {
    SourceTag::span(SourceSpan::new(0, start, end))
}

fn span_of(tag: SourceTag) -> Option<(usize, usize)> {
    tag.span.map(|s| (s.start, s.end))
}

fn runs(list: &MathList, style: Style) -> PositionedRuns {
    positioned_runs(&layout(list, style, &cm()), (0.0, 0.0))
}

fn sym(ch: char, start: usize) -> Atom {
    Atom::symbol(ch).with_tag(sp(start, start + ch.len_utf8()))
}

fn syms(text: &str, start: usize) -> MathList {
    let mut at = start;
    MathList::new(
        text.chars()
            .map(|c| {
                let a = sym(c, at);
                at += c.len_utf8();
                a
            })
            .collect(),
    )
}

/// Glyph spans by character (first occurrence).
fn glyph_span(r: &PositionedRuns, ch: char) -> Option<(usize, usize)> {
    span_of(
        r.glyphs
            .iter()
            .find(|g| g.ch == ch)
            .expect("glyph present")
            .tag,
    )
}

/// Tags every atom of `list` (and of every nested list) with a distinct span
/// and attribute, and every delimiter pair with its own.
fn tag_everything(list: &mut MathList, next: &mut usize) {
    for atom in &mut list.atoms {
        let n = *next;
        *next += 3;
        atom.tag = sp(n, n + 1).with_attr(Some(n as u32));
        atom.delimiter_tags = [sp(n + 1, n + 2), sp(n + 2, n + 3)];
        if let Some(s) = &mut atom.superscript {
            tag_everything(s, next);
        }
        if let Some(s) = &mut atom.subscript {
            tag_everything(s, next);
        }
        match &mut atom.nucleus {
            Nucleus::List(l)
            | Nucleus::Overline(l)
            | Nucleus::Underline(l)
            | Nucleus::Styled { body: l, .. }
            | Nucleus::Accent { base: l, .. }
            | Nucleus::Delimited { body: l, .. }
            | Nucleus::Brace { body: l, .. }
            | Nucleus::OverArrow { body: l, .. }
            | Nucleus::MeasuredAccent { base: l, .. }
            | Nucleus::Phantom { body: l, .. } => tag_everything(l, next),
            Nucleus::Fraction {
                numerator,
                denominator,
                ..
            } => {
                tag_everything(numerator, next);
                tag_everything(denominator, next);
            }
            Nucleus::Radical { radicand, degree } => {
                tag_everything(radicand, next);
                if let Some(d) = degree {
                    tag_everything(d, next);
                }
            }
            Nucleus::SubArray { rows, .. } => rows.iter_mut().for_each(|r| tag_everything(r, next)),
            Nucleus::ExtArrow { above, below, .. } => {
                tag_everything(above, next);
                tag_everything(below, next);
            }
            Nucleus::Symbol(_)
            | Nucleus::TextChar(_)
            | Nucleus::BigDelimiter { .. }
            | Nucleus::Glue { .. }
            | Nucleus::Text(_)
            | Nucleus::Empty => {}
            Nucleus::TextRun(pieces) => {
                for piece in pieces {
                    if let flashtex_math_layout::TextPiece::Math(list) = piece {
                        tag_everything(list, next);
                    }
                }
            }
        }
    }
}

fn strip(b: &mut MathBox) {
    b.tag = SourceTag::NONE;
    if let flashtex_math_layout::BoxKind::HBox(c) | flashtex_math_layout::BoxKind::VBox(c) =
        &mut b.kind
    {
        c.iter_mut().for_each(|c| strip(&mut c.content));
    }
}

fn corpus() -> Vec<(&'static str, MathList)> {
    let mut all = fixtures::all();
    let arrow = Atom::ext_arrow(
        ['-', '-', '\u{2192}'],
        [0.0, 3.0, 5.0, 9.0],
        MathList::symbols("f"),
        MathList::symbols("gh"),
    );
    all.push(("\\xrightarrow[gh]{f}", arrow.into()));
    all.push((
        "\\substack{i<n\\\\j}",
        Atom::subarray(vec![MathList::symbols("i<n"), MathList::symbols("j")], 'c').into(),
    ));
    all.push((
        "\\overbrace{abc}",
        Atom::brace(MathList::symbols("abc"), false).into(),
    ));
    all.push((
        "\\binom{n}{k}",
        Atom::genfrac(
            MathList::symbols("n"),
            MathList::symbols("k"),
            Some(0.0),
            Some('('),
            Some(')'),
        )
        .into(),
    ));
    all.push((
        "\\big(",
        Atom::big_delimiter(AtomClass::Open, Some('('), 1.0).into(),
    ));
    all
}

#[test]
fn tags_never_change_geometry() {
    for (name, list) in corpus() {
        for style in [Style::DISPLAY, Style::TEXT, Style::SCRIPT] {
            let plain = layout(&list, style, &cm());
            let mut tagged_list = list.clone();
            tag_everything(&mut tagged_list, &mut 0);
            let tagged = layout(&tagged_list, style, &cm());
            let mut stripped = tagged.clone();
            strip(&mut stripped);
            assert_eq!(plain, stripped, "{name}: box tree differs beyond tags");
            let (p, t) = (
                positioned_runs(&plain, (0.0, 0.0)),
                positioned_runs(&tagged, (0.0, 0.0)),
            );
            assert_eq!(p.glyphs.len(), t.glyphs.len(), "{name}");
            assert_eq!(p.rules.len(), t.rules.len(), "{name}");
            for (a, b) in p.glyphs.iter().zip(&t.glyphs) {
                assert_eq!(
                    (
                        a.x.to_bits(),
                        a.baseline_y.to_bits(),
                        a.width.to_bits(),
                        a.gid,
                        a.ch
                    ),
                    (
                        b.x.to_bits(),
                        b.baseline_y.to_bits(),
                        b.width.to_bits(),
                        b.gid,
                        b.ch
                    ),
                    "{name}"
                );
                assert!(a.tag.is_none());
                // Every placed glyph maps to some atom's source.
                assert!(
                    b.tag.span.is_some() && b.tag.attr.is_some(),
                    "{name}: untagged glyph {:?}",
                    b.ch
                );
            }
            for (a, b) in p.rules.iter().zip(&t.rules) {
                assert_eq!(
                    (a.x.to_bits(), a.y.to_bits(), a.w.to_bits(), a.h.to_bits()),
                    (b.x.to_bits(), b.y.to_bits(), b.w.to_bits(), b.h.to_bits()),
                    "{name}"
                );
                assert!(b.tag.span.is_some(), "{name}: untagged rule");
            }
        }
    }
}

#[test]
fn scripts_keep_their_own_spans() {
    // x_i^2 at bytes: x 0, i 2, 2 4.
    let list: MathList = sym('x', 0)
        .with_sub(syms("i", 2))
        .with_sup(syms("2", 4))
        .into();
    let r = runs(&list, Style::TEXT);
    assert_eq!(glyph_span(&r, 'x'), Some((0, 1)));
    assert_eq!(glyph_span(&r, 'i'), Some((2, 3)));
    assert_eq!(glyph_span(&r, '2'), Some((4, 5)));
}

#[test]
fn fraction_rule_maps_to_the_fraction_and_parts_to_themselves() {
    // \frac{a}{b}: the command 0..11, a at 6, b at 9.
    let list: MathList = Atom::frac(syms("a", 6), syms("b", 9))
        .with_tag(sp(0, 11))
        .into();
    let r = runs(&list, Style::DISPLAY);
    assert_eq!(glyph_span(&r, 'a'), Some((6, 7)));
    assert_eq!(glyph_span(&r, 'b'), Some((9, 10)));
    assert_eq!(r.rules.len(), 1);
    assert_eq!(span_of(r.rules[0].tag), Some((0, 11)));
}

#[test]
fn untagged_inner_atoms_inherit_the_enclosing_span() {
    // A synthesised numerator (no spans of its own) maps to the fraction.
    let list: MathList = Atom::frac(MathList::symbols("a"), syms("b", 9))
        .with_tag(sp(0, 11))
        .into();
    let r = runs(&list, Style::TEXT);
    assert_eq!(glyph_span(&r, 'a'), Some((0, 11)));
    assert_eq!(glyph_span(&r, 'b'), Some((9, 10)));
}

#[test]
fn radical_sign_and_bar_map_to_the_radical_including_extensible_pieces() {
    for tall in [false, true] {
        // \sqrt{...} at 0..40; radicand atoms tagged from 100. The tall
        // radicand (golden `extensible_radical_is_stacked_glyph_pieces`)
        // needs the stacked cmex recipe.
        let radicand = if tall {
            let mut body = fixtures::tall_braces();
            tag_everything(&mut body, &mut 100);
            body
        } else {
            syms("x", 100)
        };
        let list: MathList = Atom::sqrt(radicand).with_tag(sp(0, 40)).into();
        let r = runs(&list, Style::DISPLAY);
        let (sign, rest): (Vec<_>, Vec<_>) = r.glyphs.iter().partition(|g| g.ch == '\u{221A}');
        assert!(!sign.is_empty());
        if tall {
            assert!(
                sign.len() >= 4,
                "tall radical is built from extensible pieces"
            );
        }
        assert!(sign.iter().all(|g| span_of(g.tag) == Some((0, 40))));
        assert!(!rest.is_empty());
        assert!(
            rest.iter()
                .all(|g| g.tag.span.is_some_and(|s| s.start >= 100))
        );
        // The vinculum is the only rule not produced by an inner fraction.
        assert_eq!(
            r.rules
                .iter()
                .filter(|rule| span_of(rule.tag) == Some((0, 40)))
                .count(),
            1
        );
    }
}

#[test]
fn left_and_right_delimiters_map_to_their_own_commands() {
    // \left\{ (0..7) body \right\} (40..48), with an extensible-height body.
    let mut body = fixtures::tall_braces();
    let Nucleus::Delimited { body: inner, .. } = &mut body.atoms[0].nucleus else {
        panic!("tall_braces is a \\left...\\right atom");
    };
    let inner = inner.clone();
    let mut tagged_inner = inner;
    tag_everything(&mut tagged_inner, &mut 100);
    let list: MathList = Atom::left_right(Some('{'), Some('}'), tagged_inner)
        .with_tag(sp(0, 48))
        .with_delimiter_tags(sp(0, 7), sp(40, 48))
        .into();
    let r = runs(&list, Style::DISPLAY);
    let left: Vec<_> = r
        .glyphs
        .iter()
        .filter(|g| span_of(g.tag) == Some((0, 7)))
        .collect();
    let right: Vec<_> = r
        .glyphs
        .iter()
        .filter(|g| span_of(g.tag) == Some((40, 48)))
        .collect();
    assert!(
        left.len() > 1 && right.len() > 1,
        "extensible braces: {} / {}",
        left.len(),
        right.len()
    );
    let inner_min_x = r
        .glyphs
        .iter()
        .filter(|g| g.tag.span.is_some_and(|s| s.start >= 100))
        .map(|g| g.x)
        .fold(f64::INFINITY, f64::min);
    assert!(left.iter().all(|g| g.x < inner_min_x));
    assert!(right.iter().all(|g| g.x > inner_min_x));
    // Nothing falls back to the whole `\left...\right` atom.
    assert!(r.glyphs.iter().all(|g| span_of(g.tag) != Some((0, 48))));
}

#[test]
fn delimiters_without_their_own_tags_map_to_the_atom() {
    // \binom{n}{k} (0..12): the parentheses come from the command itself.
    let list: MathList =
        Atom::genfrac(syms("n", 7), syms("k", 10), Some(0.0), Some('('), Some(')'))
            .with_tag(sp(0, 12))
            .into();
    let r = runs(&list, Style::TEXT);
    assert_eq!(glyph_span(&r, 'n'), Some((7, 8)));
    assert_eq!(glyph_span(&r, 'k'), Some((10, 11)));
    let parens: Vec<_> = r
        .glyphs
        .iter()
        .filter(|g| g.ch == '(' || g.ch == ')')
        .collect();
    assert_eq!(parens.len(), 2);
    assert!(parens.iter().all(|g| span_of(g.tag) == Some((0, 12))));
}

#[test]
fn accents_map_the_mark_to_the_command_and_the_base_to_itself() {
    // \hat{x} (0..7, x at 5) and \hat{x}^2 (the scripted-character path).
    for scripted in [false, true] {
        let mut atom = Atom::accent('^', syms("x", 5)).with_tag(sp(0, 7));
        if scripted {
            atom = atom.with_sup(syms("2", 8));
        }
        let r = runs(&atom.into(), Style::TEXT);
        assert_eq!(glyph_span(&r, 'x'), Some((5, 6)));
        if scripted {
            assert_eq!(glyph_span(&r, '2'), Some((8, 9)));
        }
        let marks: Vec<_> = r
            .glyphs
            .iter()
            .filter(|g| g.ch != 'x' && g.ch != '2')
            .collect();
        assert_eq!(marks.len(), 1);
        assert_eq!(span_of(marks[0].tag), Some((0, 7)));
    }
}

#[test]
fn operator_text_maps_to_the_operator_and_limits_to_themselves() {
    // \lim_{x\to 0} with \lim at 0..4, subscript atoms from 6.
    let list: MathList = Atom::text_op("lim")
        .with_tag(sp(0, 4))
        .with_sub(syms("x\u{2192}0", 6))
        .into();
    let r = runs(&list, Style::DISPLAY);
    for ch in ['l', 'i', 'm'] {
        assert_eq!(glyph_span(&r, ch), Some((0, 4)), "{ch}");
    }
    assert_eq!(glyph_span(&r, 'x'), Some((6, 7)));
    assert_eq!(glyph_span(&r, '0'), Some((6 + 1 + 3, 6 + 1 + 3 + 1)));
    // \operatorname{sin}: an Op whose nucleus is a list of text atoms.
    let op: MathList = Atom::new(AtomClass::Op, Nucleus::List(syms("sin", 14)))
        .with_tag(sp(0, 18))
        .into();
    let r = runs(&op, Style::TEXT);
    assert_eq!(glyph_span(&r, 's'), Some((14, 15)));
    assert_eq!(glyph_span(&r, 'n'), Some((16, 17)));
}

#[test]
fn big_operators_extensible_arrows_braces_and_subarrays() {
    // \sum_{i}^{n}: the operator glyph maps to \sum.
    let sum: MathList = Atom::symbol('\u{2211}')
        .with_tag(sp(0, 4))
        .with_sub(syms("i", 6))
        .with_sup(syms("n", 10))
        .into();
    let r = runs(&sum, Style::DISPLAY);
    assert_eq!(glyph_span(&r, '\u{2211}'), Some((0, 4)));
    assert_eq!(glyph_span(&r, 'i'), Some((6, 7)));
    assert_eq!(glyph_span(&r, 'n'), Some((10, 11)));

    // \xrightarrow[g]{f} (0..20): every arrow piece maps to the command.
    let arrow: MathList = Atom::ext_arrow(
        ['-', '-', '\u{2192}'],
        [0.0, 3.0, 5.0, 9.0],
        syms("f", 17),
        syms("g", 13),
    )
    .with_tag(sp(0, 20))
    .into();
    let r = runs(&arrow, Style::TEXT);
    assert_eq!(glyph_span(&r, 'f'), Some((17, 18)));
    assert_eq!(glyph_span(&r, 'g'), Some((13, 14)));
    let pieces: Vec<_> = r
        .glyphs
        .iter()
        .filter(|g| g.ch != 'f' && g.ch != 'g')
        .collect();
    assert!(pieces.len() >= 2);
    assert!(pieces.iter().all(|g| span_of(g.tag) == Some((0, 20))));

    // \overbrace{ab} (0..14): brace pieces and fill rules map to the command.
    let brace: MathList = Atom::brace(syms("ab", 11), false)
        .with_tag(sp(0, 14))
        .into();
    let r = runs(&brace, Style::DISPLAY);
    assert_eq!(glyph_span(&r, 'a'), Some((11, 12)));
    let pieces: Vec<_> = r
        .glyphs
        .iter()
        .filter(|g| g.ch != 'a' && g.ch != 'b')
        .collect();
    assert_eq!(pieces.len(), 4);
    assert!(pieces.iter().all(|g| span_of(g.tag) == Some((0, 14))));
    assert!(
        r.rules
            .iter()
            .all(|rule| span_of(rule.tag) == Some((0, 14)))
    );

    // \substack{i\\j}: each row keeps its own spans.
    let stack: MathList = Atom::subarray(vec![syms("i", 10), syms("j", 13)], 'c')
        .with_tag(sp(0, 15))
        .into();
    let r = runs(&stack, Style::DISPLAY);
    assert_eq!(glyph_span(&r, 'i'), Some((10, 11)));
    assert_eq!(glyph_span(&r, 'j'), Some((13, 14)));
}

#[test]
fn attributes_inherit_per_field_and_the_innermost_wins() {
    // {\color{red} a {\color{blue} b} c} with red = 1, blue = 2: a and c
    // keep their own spans but take red from the group; b keeps blue.
    let blue: MathList = MathList::new(vec![sym('b', 20).with_tag(SourceTag {
        span: Some(SourceSpan::new(0, 20, 21)),
        attr: Some(2),
    })]);
    let group = MathList::new(vec![sym('a', 12), Atom::group(blue), sym('c', 24)]);
    let list: MathList = Atom::group(group)
        .with_tag(SourceTag {
            span: None,
            attr: Some(1),
        })
        .into();
    let r = runs(&list, Style::TEXT);
    let get = |ch| r.glyphs.iter().find(|g| g.ch == ch).unwrap().tag;
    assert_eq!(get('a').attr, Some(1));
    assert_eq!(span_of(get('a')), Some((12, 13)));
    assert_eq!(get('b').attr, Some(2));
    assert_eq!(span_of(get('b')), Some((20, 21)));
    assert_eq!(get('c').attr, Some(1));
    // Styled bodies and overlines pass tags through as well.
    let styled: MathList = MathList::new(vec![
        Atom::styled(Style::SCRIPT, syms("x", 3)).with_tag(sp(0, 20)),
        Atom::overline(syms("y", 30)).with_tag(sp(21, 33).with_attr(Some(5))),
    ]);
    let r = runs(&styled, Style::TEXT);
    assert_eq!(glyph_span(&r, 'x'), Some((3, 4)));
    assert_eq!(glyph_span(&r, 'y'), Some((30, 31)));
    assert_eq!(
        r.glyphs.iter().find(|g| g.ch == 'y').unwrap().tag.attr,
        Some(5)
    );
    assert_eq!(r.rules.len(), 1);
    assert_eq!(span_of(r.rules[0].tag), Some((21, 33)));
    assert_eq!(r.rules[0].tag.attr, Some(5));
}
