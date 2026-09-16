//! The render cache is keyed by a hash of a block's items, so two different
//! item lists must never produce the same hash stream.
//!
//! They used to. `incremental::hash_items` opened each item with a
//! hand-written number, and four pairs had drifted onto the same one:
//! `Item::Table`/`Item::Logo` both 9, `Item::Footnote`/`Item::Rule` both 10,
//! `Item::ColorBox`/`Item::Kern` both 11, and in `hash_math`
//! `Nucleus::Rule`/`Nucleus::SubArray` both 15. The tag is now
//! `std::mem::discriminant` of the value, which is distinct for distinct
//! variants by construction, so a variant added later cannot silently reuse
//! another's tag — which is exactly how the third Item pair appeared:
//! `Kern` took 11 first, `ColorBox` landed on 11 later.
//!
//! `no_two_item_kinds_share_a_tag` and `no_two_math_nucleus_kinds_share_a_tag`
//! fail if any two kinds ever map to the same value.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use flashtex_compiler::color::DeviceColor;
use flashtex_compiler::math::MathList;
use flashtex_compiler::parser::{parse, FillLeader, UnderlineGeom};
use flashtex_compiler::text_builtins::{DimenUnit, PhysicalUnit, TextDimen, TextLogo, TextRule};
use flashtex_compiler::Span;
use flashtex_render_pipeline::adapter::{
    self, ColorBoxItem, Item, Labels, Segment, TextStyle, UnderlineItem, Word,
};
use flashtex_render_pipeline::incremental::{block_origin, hash_items, hash_math, kind_tag};
use flashtex_render_pipeline::RenderOptions;

fn key(items: &[Item]) -> u64 {
    let mut h = DefaultHasher::new();
    hash_items(items, 0, &mut h);
    h.finish()
}

fn math_key(list: &MathList) -> u64 {
    let mut h = DefaultHasher::new();
    hash_math(list, &mut h);
    h.finish()
}

fn dim(integer: i32) -> TextDimen {
    TextDimen { negative: false, integer, frac: Vec::new(), unit: DimenUnit::Physical(PhysicalUnit::Pt) }
}

/// Every item the adapter builds for `body`, in order, across every
/// paragraph's `Lines` parts.
fn items_of(body: &str) -> Vec<Item> {
    let src = format!("\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let parsed = parse(&src);
    let doc = adapter::adapt(&[&src], 0, &parsed, &RenderOptions::default(), &Labels::default());
    let mut out = Vec::new();
    for block in &doc.blocks {
        let parts = match block {
            adapter::Block::Paragraph { parts, .. } => parts.clone(),
            adapter::Block::Heading { items, .. } => vec![adapter::ParaPart::Lines(items.clone())],
            _ => continue,
        };
        for part in parts {
            if let adapter::ParaPart::Lines(items) = part {
                out.extend(items);
            }
        }
    }
    out
}

/// Every math list the adapter builds for `body`: inline formulas and
/// displays alike.
fn maths_of(body: &str) -> Vec<MathList> {
    let src = format!("\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let parsed = parse(&src);
    let doc = adapter::adapt(&[&src], 0, &parsed, &RenderOptions::default(), &Labels::default());
    let mut out = Vec::new();
    for block in &doc.blocks {
        let adapter::Block::Paragraph { parts, .. } = block else { continue };
        for part in parts {
            match part {
                adapter::ParaPart::Lines(items) => {
                    for item in items {
                        if let Item::Math { list, .. } = item {
                            out.push(list.clone());
                        }
                    }
                }
                adapter::ParaPart::Display { list, .. } => out.push(list.clone()),
                adapter::ParaPart::Rows { rows, .. } => {
                    for row in rows {
                        out.extend(row.cells.iter().cloned());
                    }
                }
            }
        }
    }
    out
}

/// The first item of `body` that `want` accepts, for the kinds that are too
/// laborious to build by hand (`Table` carries a whole laid-out tabular).
fn item_from(body: &str, want: fn(&Item) -> bool) -> Item {
    items_of(body).into_iter().find(|i| want(i)).unwrap_or_else(|| panic!("no such item in {body:?}"))
}

/// One sample of every [`Item`] variant, named. Adding a variant to `Item`
/// without adding it here leaves it untested, but cannot make it collide:
/// the tag is the variant's own discriminant.
fn one_of_every_item_kind() -> Vec<(&'static str, Item)> {
    let style = TextStyle::default();
    let span = Span::new(0, 0);
    vec![
        (
            "Word",
            Item::Word(Word {
                segments: vec![Segment { text: "a".into(), chars: Vec::new(), style }],
            }),
        ),
        ("Space", Item::Space { style, factor: 1000, no_break: false }),
        ("Math", item_from("$x$", |i| matches!(i, Item::Math { .. }))),
        ("LineBreak", Item::LineBreak { skip_pt: 0.0 }),
        ("Quad", Item::Quad { em: 1.0, style }),
        ("Label", Item::Label { key: "k".into() }),
        ("ItalicCorrection", Item::ItalicCorrection),
        ("NoteParBreak", Item::NoteParBreak),
        ("HFill", Item::HFill { fill: true, leader: FillLeader::None, style }),
        ("HSpace", Item::HSpace { pt: 0.0, stretch_pt: 0.0, shrink_pt: 0.0 }),
        (
            "Table",
            item_from("x \\begin{tabular}{ll}a & b\\end{tabular} y", |i| matches!(i, Item::Table(_))),
        ),
        ("Logo", Item::Logo { logo: TextLogo::TeX, style, span }),
        (
            "Rule",
            Item::Rule { rule: TextRule { raise: dim(0), width: dim(2), height: dim(3) }, style, span },
        ),
        ("Kern", Item::Kern { amount: dim(0), style }),
        ("Footnote", Item::Footnote { number: "1".into(), mark: true, span, text: None }),
        (
            "ColorBox",
            Item::ColorBox(Box::new(ColorBoxItem {
                fill: DeviceColor::BLACK,
                frame: None,
                sep_pt: 3.0,
                rule_pt: 0.4,
                items: Vec::new(),
                span,
            })),
        ),
        ("Lap", Item::Lap { items: Vec::new() }),
        (
            "Underline",
            Item::Underline(Box::new(UnderlineItem {
                thickness_pt: 0.4,
                geom: UnderlineGeom::UlemDescender,
                items: Vec::new(),
                span,
            })),
        ),
        ("LeaveVmode", Item::LeaveVmode),
    ]
}

/// The regression test the task asks for: no two item kinds may map to the
/// same tag. This is what the hand-written numbers got wrong — `Table` and
/// `Logo` both mapped to 9, `Footnote` and `Rule` both to 10, `ColorBox`
/// and `Kern` both to 11 — and checking whole keys would not have caught
/// it, because their payloads differ in length. It checks the tag itself.
#[test]
fn no_two_item_kinds_share_a_tag() {
    let mut seen: HashMap<u64, &'static str> = HashMap::new();
    for (name, item) in one_of_every_item_kind() {
        let mut h = DefaultHasher::new();
        kind_tag(&item).hash(&mut h);
        let t = h.finish();
        if let Some(other) = seen.insert(t, name) {
            panic!("Item::{name} and Item::{other} share the tag {t:#x}");
        }
    }
    assert_eq!(seen.len(), 19, "a variant was added to Item without a sample here");
}

/// The same for the math nuclei, where `Nucleus::Rule` and
/// `Nucleus::SubArray` both mapped to 15. `MathAtom::class_override` is
/// `pub(crate)` in the compiler, so these atoms can only come from real
/// source — which is also all the cache ever sees.
#[test]
fn no_two_math_nucleus_kinds_share_a_tag() {
    let sources: &[(&str, &str)] = &[
        ("Symbol", "$a$"),
        ("Rule", "$\\rule{2pt}{3pt}$"),
        ("SubArray", "$\\substack{a}$"),
        ("Fraction", "$\\frac{a}{b}$"),
        ("Radical", "$\\sqrt{a}$"),
        ("Text", "$\\text{a}$"),
        ("Space", "$a\\,b$"),
        ("Matrix", "$\\begin{matrix}a\\end{matrix}$"),
        ("Bold", "$\\mathbf{a}$"),
        ("Framed", "$\\boxed{a}$"),
        ("Stacked", "$\\overset{a}{b}$"),
        ("Accent", "$\\hat{a}$"),
        ("SizedDelimiter", "$\\bigl(a\\bigr)$"),
        ("Group", "$\\mathbin{a}$"),
        ("GenFraction", "$\\binom{a}{b}$"),
        ("Phantom", "$\\phantom{a}$"),
        ("Operator", "$\\operatorname{ab}$"),
        ("ExtArrow", "$\\xrightarrow{a}$"),
    ];
    let mut seen: HashMap<u64, &'static str> = HashMap::new();
    for (name, src) in sources {
        let lists = maths_of(src);
        let nucleus = lists
            .iter()
            .flat_map(|l| l.atoms.iter())
            .map(|a| &a.nucleus)
            .find(|n| nucleus_name(n) == *name)
            .unwrap_or_else(|| panic!("{src} produced no Nucleus::{name}"));
        let mut h = DefaultHasher::new();
        kind_tag(nucleus).hash(&mut h);
        let t = h.finish();
        if let Some(other) = seen.insert(t, name) {
            panic!("Nucleus::{name} and Nucleus::{other} share the tag {t:#x} ({src})");
        }
    }
}

/// The variant name, so the sample search above picks the atom it meant
/// rather than whatever the parser put first.
fn nucleus_name(n: &flashtex_compiler::math::Nucleus) -> &'static str {
    use flashtex_compiler::math::Nucleus as N;
    match n {
        N::Symbol(_) => "Symbol",
        N::Rule(_) => "Rule",
        N::SubArray { .. } => "SubArray",
        N::Fraction { .. } => "Fraction",
        N::Radical(_) => "Radical",
        N::Text(_) => "Text",
        N::Space { .. } => "Space",
        N::Matrix { .. } => "Matrix",
        N::Bold(_) => "Bold",
        N::Framed { .. } => "Framed",
        N::Stacked { .. } => "Stacked",
        N::Accent { .. } => "Accent",
        N::SizedDelimiter { .. } => "SizedDelimiter",
        N::Group(_) => "Group",
        N::GenFraction { .. } => "GenFraction",
        N::Phantom { .. } => "Phantom",
        N::Operator { .. } => "Operator",
        N::ExtArrow { .. } => "ExtArrow",
    }
}

/// The measured, byte-exact collision the old numbering allowed.
///
/// With `Item::Rule` and `Item::Footnote` both opening with the byte 10 and
/// no length prefix anywhere in the stream, a one-item `Rule` list and a
/// `Footnote` list padded with segment-less `Item::Word`s (each contributes
/// exactly one 0 byte) absorb exactly the same bytes at some padding count,
/// and the two different lists hash to one cache key. Neither the padding
/// nor a NUL `\@thefnmark` is anything the adapter emits, so this was
/// latent, not reachable — but it is what the scheme permitted, and it must
/// stay impossible.
#[test]
fn a_rule_list_and_a_footnote_list_no_longer_share_a_key() {
    let rule = vec![Item::Rule {
        rule: TextRule { raise: dim(255), width: dim(0), height: dim(0) },
        style: TextStyle::default(),
        span: Span::new(0, 0),
    }];
    let target = key(&rule);
    for pad in 0..400 {
        let mut list = vec![Item::Footnote {
            number: "\u{0}".to_string(),
            mark: false,
            span: Span::new(0, 0),
            text: None,
        }];
        list.extend((0..pad).map(|_| Item::Word(Word { segments: Vec::new() })));
        assert_ne!(key(&list), target, "collision again at {pad} padding words");
    }
}

/// The pair that drifted together after the first two: `Item::Kern` and
/// `Item::ColorBox` both opened with the byte 11. No byte-exact key
/// collision was found for this pair (a Debug string's terminator blocks
/// the alignment the Rule/Footnote pair allowed), so unlike the test above
/// this one also passed on the old scheme; it pins the pair anyway.
#[test]
fn a_kern_list_and_a_colorbox_list_no_longer_share_a_key() {
    let kern = vec![Item::Kern { amount: dim(255), style: TextStyle::default() }];
    let target = key(&kern);
    for pad in 0..400 {
        let mut list = vec![Item::ColorBox(Box::new(ColorBoxItem {
            fill: DeviceColor::BLACK,
            frame: None,
            sep_pt: 3.0,
            rule_pt: 0.4,
            items: Vec::new(),
            span: Span::new(0, 0),
        }))];
        list.extend((0..pad).map(|_| Item::Word(Word { segments: Vec::new() })));
        assert_ne!(key(&list), target, "collision again at {pad} padding words");
    }
}

/// Why the `Table`/`Logo` tag 9 collision could not be reached from any
/// document: a block holding a table never gets a cache key at all.
#[test]
fn a_block_holding_a_table_is_never_keyed() {
    let table = item_from("x \\begin{tabular}{ll}a & b\\end{tabular} y", |i| matches!(i, Item::Table(_)));
    let with_table = items_of("x \\begin{tabular}{ll}a & b\\end{tabular} y");
    assert!(with_table.iter().any(|i| matches!(i, Item::Table(_))));
    assert_eq!(block_origin(&with_table), None, "a table block must not be cacheable");
    assert_eq!(block_origin(std::slice::from_ref(&table)), None);
    // A plain paragraph in the same document is keyed normally.
    assert!(block_origin(&items_of("plain words only")).is_some());
}

/// Both sides of the `Nucleus::Rule`/`Nucleus::SubArray` tag 15 collision do
/// reach live cache keys (unlike `Table`/`Footnote`, nothing excludes them),
/// so that one was the closest to reachable. Their keys must differ.
#[test]
fn a_math_rule_and_a_substack_do_not_share_a_key() {
    let rule = maths_of("$\\rule{2pt}{3pt}$");
    let substack = maths_of("$\\substack{a}$");
    assert_eq!(rule.len(), 1);
    assert_eq!(substack.len(), 1);
    assert_ne!(math_key(&rule[0]), math_key(&substack[0]));
}
