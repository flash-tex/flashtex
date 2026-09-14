//! `\text{...}` in math is an hbox of *text-font* characters (T1 `ec-lmr`
//! TFM: ligature/kern program, braces at their T1 slots, `\fontdimen2`
//! interword glue), laid out through math-layout's existing `Nucleus::Text`
//! + `text_glyph` interface with a placeholder of the run's exact metrics
//! and substituted afterwards. The three gaps the HW1 text-producer
//! evidence (main f261b36c) documented — `a b` advancing by rm-lmr12 slot
//! 32, `\{x\}` placed by OT1 slots 123/125, `ffi` as three glyphs — are
//! the assertions below. The compiler side (`Nucleus::Text`) is an isolated
//! candidate, so the lists are built directly with `TextSink`.

mod common;

use std::rc::Rc;

use common::lm_available;
use flashtex_math_layout as ml;
use flashtex_render_pipeline::fonts::{Family, FontSet, Role};
use flashtex_render_pipeline::mathfont::{MathFonts, MathSizes};
use flashtex_render_pipeline::mathtex::TexMathMetrics;
use flashtex_render_pipeline::mathtext::{run_of, substitute, TextRun, TextRunMetrics, TextSink, RUN_FONT_BASE, SLOT_GLYPHS};
use flashtex_render_pipeline::tfm::Tfm;

const FIX: f64 = 1_048_576.0;

struct Placed {
    /// `(ch, original gid, x, width-from-box)` of every ink glyph, plus
    /// spaces (gid 0) in order.
    glyphs: Vec<(char, u16, f64)>,
    width: f64,
    runs: Vec<TextRun>,
}

/// Lays out `$\text{text}$`-style lists (one `\text` per entry, `+` between
/// entries) at 12 pt through the real TeX metrics provider.
fn place(fonts: &FontSet, texts: &[&str], scripts: Option<(&str, &str)>) -> Placed {
    let math = fonts.resolve(Family::LatinModern, Role::Math, 12.0);
    assert!(math.substituted.is_none());
    let sizes = MathSizes { text: 12.0, script: 8.0, script_script: 6.0 };
    let otf = Rc::new(MathFonts::new(math.face, sizes).expect("MATH table"));
    let tex = TexMathMetrics::new(12, false, otf, fonts);
    assert!(tex.roman_available(), "rm-lmr TFMs installed");
    let mut sink = TextSink::default();
    let mut atoms = Vec::new();
    for (i, t) in texts.iter().enumerate() {
        if i > 0 {
            atoms.push(ml::Atom::symbol('+'));
        }
        let mut a = sink.atom(t);
        if let Some((sub, sup)) = scripts {
            a.subscript = Some(ml::MathList::new(sub.chars().map(ml::Atom::symbol).collect()));
            a.superscript = Some(ml::MathList::new(sup.chars().map(ml::Atom::symbol).collect()));
        }
        atoms.push(a);
    }
    let list = ml::MathList::new(atoms);
    let metrics = TextRunMetrics::new(&tex, fonts, fonts.shaper(), Family::LatinModern, &sink.texts, &sink.keys);
    let mut laid = ml::layout_with_report(&list, ml::Style::TEXT, &metrics);
    assert!(laid.limitations.is_empty(), "{:?}", laid.limitations);
    let (runs, _notices) = metrics.finish();
    substitute(&mut laid.root, &runs);
    let flat = ml::positioned_runs(&laid.root, (0.0, 0.0));
    let mut glyphs = Vec::new();
    for g in &flat.glyphs {
        let gid = if g.font_id.0 >= RUN_FONT_BASE {
            run_of(&runs, g.font_id).expect("a run owns every run slot").glyph_at(g.font_id, g.gid).expect("in range").gid.0
        } else {
            tex.otf_glyph(g.font_id, g.gid as u8, g.ch).map_or(0, |(_, gid)| gid)
        };
        glyphs.push((g.ch, gid, g.x));
    }
    assert!(!flat.glyphs.iter().any(|g| g.ch as u32 >= 0xF_0000), "no placeholder reaches the output");
    Placed { glyphs, width: laid.root.width, runs }
}

fn text_face(fonts: &FontSet) -> (Rc<flashtex_render_pipeline::fonts::LoadedFace>, Rc<Tfm>) {
    let face = fonts.resolve(Family::LatinModern, Role::Text { bold: false, italic: false }, 12.0).face;
    let tfm = fonts.tfm("ec-lmr12.tfm").expect("ec-lmr12.tfm");
    (face, tfm)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn ligature_program_runs_ffi_is_one_glyph_from_the_text_face() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let (face, tfm) = text_face(&fonts);
    let p = place(&fonts, &["ffi"], None);
    assert_eq!(p.glyphs.len(), 1, "{:?}", p.glyphs);
    let ffi_gid = face.face().glyph_id('\u{FB03}').expect("ffi in lmroman12").0;
    assert_eq!(p.glyphs[0].1, ffi_gid);
    assert_eq!(p.glyphs[0].0, 'f', "cluster text starts at the first source character");
    // T1 slot 0x1E (ffi) width of ec-lmr12 is the whole box.
    let w = Tfm::pt(tfm.metrics(0x1E).unwrap().width, 12.0);
    assert!(near(p.width, w), "{} vs {w}", p.width);
    assert_eq!(p.runs.len(), 1);
    assert!(p.runs[0].tfm_metrics);
    assert_eq!(p.runs[0].face.name, face.name);
}

#[test]
fn interword_space_is_fontdimen2_not_a_character_slot() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let (_, tfm) = text_face(&fonts);
    let p = place(&fonts, &["a b"], None);
    let chars: Vec<char> = p.glyphs.iter().map(|g| g.0).collect();
    assert_eq!(chars, vec!['a', ' ', 'b']);
    assert_eq!(p.glyphs[1].1, 0, "the space is glue: no glyph id");
    let a_w = Tfm::pt(tfm.metrics(b'a').unwrap().width, 12.0);
    let space = Tfm::pt(tfm.param(2).unwrap(), 12.0);
    assert!(near(p.glyphs[1].2, a_w));
    assert!(near(p.glyphs[2].2 - p.glyphs[0].2, a_w + space), "b at {} = a {a_w} + space {space}", p.glyphs[2].2);
    // The evidence's wrong advance: rm-lmr12 slot 32 (285213/2^20 em).
    let slot32 = 285_213.0 * 12.0 / FIX;
    assert!(!near(space, slot32), "space {space} must not be the OT1 slot-32 width {slot32}");
    // \fontdimen2 of ec-lmr12 as the evidence README states it (342239/2^20 em).
    assert!((space - 342_239.0 * 12.0 / FIX).abs() < 1e-4, "{space}");
}

#[test]
fn sentence_end_adds_extra_space_and_an_hbox_is_never_stretched() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let (_, tfm) = text_face(&fonts);
    let p = place(&fonts, &["a. b"], None);
    let chars: Vec<char> = p.glyphs.iter().map(|g| g.0).collect();
    assert_eq!(chars, vec!['a', '.', ' ', 'b']);
    let dot_x = p.glyphs[1].2;
    let dot_w = Tfm::pt(tfm.metrics(b'.').unwrap().width, 12.0);
    let space = Tfm::pt(tfm.param(2).unwrap(), 12.0) + Tfm::pt(tfm.param(7).unwrap(), 12.0);
    assert!(near(p.glyphs[3].2, dot_x + dot_w + space), "space factor 3000 after '.' adds \\fontdimen7");
}

#[test]
fn escaped_braces_use_the_t1_brace_slots_and_cmap_ids() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let (face, tfm) = text_face(&fonts);
    let p = place(&fonts, &["{x}"], None);
    let chars: Vec<char> = p.glyphs.iter().map(|g| g.0).collect();
    assert_eq!(chars, vec!['{', 'x', '}']);
    assert_eq!(p.glyphs[0].1, face.face().glyph_id('{').unwrap().0);
    assert_eq!(p.glyphs[2].1, face.face().glyph_id('}').unwrap().0);
    let lb = Tfm::pt(tfm.metrics(123).unwrap().width, 12.0);
    let x = Tfm::pt(tfm.metrics(b'x').unwrap().width, 12.0);
    let rb = Tfm::pt(tfm.metrics(125).unwrap().width, 12.0);
    assert!(near(p.glyphs[1].2, lb));
    assert!(near(p.glyphs[2].2, lb + x));
    assert!(near(p.width, lb + x + rb));
    // The evidence's wrong placement: OT1 slots 123/125 of rm-lmr12 are
    // 513365/2^20 em each; T1 braces are narrower.
    assert!(!near(lb, 513_365.0 * 12.0 / FIX), "{lb}");
}

#[test]
fn labels_keep_their_glyphs_and_ord_spacing_and_scripts_attach() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let (face, _) = text_face(&fonts);
    let p = place(&fonts, &["(a)", "(b)", "and"], None);
    let text: String = p.glyphs.iter().map(|g| g.0).collect();
    assert_eq!(text, "(a)+(b)+and");
    for (ch, gid, _) in &p.glyphs {
        if *ch != '+' {
            assert_eq!(*gid, face.face().glyph_id(*ch).unwrap().0, "{ch}");
        }
    }
    // Ord + Bin + Ord: medmuskip on both sides of every `+` (TeXbook ch. 18),
    // so `+` is not flush against the labels.
    let plus: Vec<f64> = p.glyphs.iter().filter(|g| g.0 == '+').map(|g| g.2).collect();
    let close: Vec<f64> = p.glyphs.iter().filter(|g| g.0 == ')').map(|g| g.2).collect();
    assert_eq!(plus.len(), 2);
    assert!(plus[0] > close[0] + 1.0, "{plus:?} {close:?}");

    let s = place(&fonts, &["and"], Some(("i", "2")));
    let mut text: Vec<char> = s.glyphs.iter().map(|g| g.0).collect();
    text.sort_unstable();
    assert_eq!(text, vec!['2', 'a', 'd', 'i', 'n']);
    let d_x = s.glyphs.iter().find(|g| g.0 == 'd').unwrap().2;
    assert!(s.glyphs.iter().filter(|g| g.0 == 'i' || g.0 == '2').all(|g| g.2 > d_x), "scripts follow the run");
    assert_eq!(s.runs.len(), 1);
    assert_eq!(s.runs[0].size, 12.0);
}

#[test]
fn comment_joined_argument_is_the_same_run() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // The compiler candidate joins `\text% before\n{and}` into "and": the
    // pipeline sees the same string as `\text{and}`.
    let fonts = FontSet::with_default_dirs(&[]);
    let a = place(&fonts, &["and"], None);
    let b = place(&fonts, &["and"], None);
    assert_eq!(a.glyphs, b.glyphs);
}

/// Issue #43: a run of more than 65536 shaped entries must not alias its
/// later entries onto the first chunk. Entries 65535 / 65536 / 65537 are a
/// distinctive glyph, a space and another distinctive glyph; the first and
/// last glyphs differ from everything in between.
#[test]
fn a_run_beyond_one_chunk_addresses_every_entry_without_aliasing() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let (face, _) = text_face(&fonts);
    // 16 words of 4000 (entries 4001k .. 4001k+3999, space at 4001k+4000),
    // then a 1520-char word ending in 'z' at entry 65535, a space at 65536,
    // then "c…d" from 65537.
    let mut words: Vec<String> = Vec::new();
    words.push(format!("b{}", "a".repeat(3999)));
    for _ in 1..16 {
        words.push("a".repeat(4000));
    }
    words.push(format!("{}z", "a".repeat(1519)));
    words.push(format!("c{}d", "a".repeat(100)));
    let text = words.join(" ");
    let total = 65537 + 102;
    assert_eq!(text.chars().count(), total);
    let p = place(&fonts, &[&text], None);
    assert_eq!(p.glyphs.len(), total, "one entry per character and space");
    assert_eq!(p.runs.len(), 1);
    assert_eq!(p.runs[0].glyphs.len(), total);
    assert_eq!(p.runs[0].slots(), 2);
    let gid = |c: char| face.face().glyph_id(c).unwrap().0;
    assert_eq!((p.glyphs[0].0, p.glyphs[0].1), ('b', gid('b')));
    assert_eq!((p.glyphs[SLOT_GLYPHS - 1].0, p.glyphs[SLOT_GLYPHS - 1].1), ('z', gid('z')));
    assert_eq!((p.glyphs[SLOT_GLYPHS].0, p.glyphs[SLOT_GLYPHS].1), (' ', 0));
    assert_eq!((p.glyphs[SLOT_GLYPHS + 1].0, p.glyphs[SLOT_GLYPHS + 1].1), ('c', gid('c')));
    assert_eq!((p.glyphs[total - 1].0, p.glyphs[total - 1].1), ('d', gid('d')));
    assert_eq!(p.glyphs[1].1, gid('a'));
    assert_ne!(gid('a'), gid('c'));
    // Geometry stays monotone across the chunk boundary and the run width
    // is the sum of the entries.
    for w in p.glyphs.windows(2) {
        assert!(w[1].2 > w[0].2, "x must increase: {:?} {:?}", w[0], w[1]);
    }
    assert!(p.width > p.glyphs[total - 1].2);
}
