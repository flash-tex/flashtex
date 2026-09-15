//! Math glue stretch/shrink and `MathBox::pack_to` against pdfTeX.
//!
//! The oracle is `tests/glue_set/glue-set.log`, the `\showbox` output of
//! `tests/glue_set/glue-set.tex` (pdfTeX 3.141592653-2.6-1.40.29, TeX Live
//! 2026, article 10pt, Computer Modern): each `\hbox{$...$}` /
//! `\hbox to <w>{$...$}` lists its glue as `\glue(\medmuskip) 2.22217 plus
//! 1.11108 minus 2.22217` and its setting as `glue set - 0.65746`. Values
//! are read from the committed log at test time and compared at
//! `\showbox`'s five decimals. Nothing here runs TeX.

use flashtex_math_layout::{
    Atom, BoxKind, CmMathMetrics, GlueOrder, GlueSign, MathBox, MathFlex, MathList, Style, layout,
    positioned_runs,
};

/// One `\hbox` of the log: its width, glue setting and glue list.
#[derive(Debug, PartialEq)]
struct Shown {
    width: f64,
    /// (sign, ratio, order) of `glue set [-] ratio[fil|fill|filll]`.
    set: Option<(GlueSign, f64, GlueOrder)>,
    /// (natural, stretch, stretch order, shrink) of every `\glue` line.
    glue: Vec<(f64, f64, GlueOrder, f64)>,
}

fn order_of(s: &str) -> (f64, GlueOrder) {
    for (suffix, order) in [
        ("filll", GlueOrder::Filll),
        ("fill", GlueOrder::Fill),
        ("fil", GlueOrder::Fil),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return (n.parse().unwrap(), order);
        }
    }
    (s.parse().unwrap(), GlueOrder::Normal)
}

/// Every top-level `\hbox` shown in the log, consecutive duplicates (the
/// terminal echo) removed.
fn shown() -> Vec<Shown> {
    let log = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/glue_set/glue-set.log"
    ))
    .unwrap();
    let mut out: Vec<Shown> = Vec::new();
    let mut current: Option<Shown> = None;
    for line in log.lines() {
        if let Some(rest) = line.strip_prefix("\\hbox(") {
            if let Some(done) = current.take() {
                out.push(done);
            }
            let after_x = &rest[rest.find(")x").unwrap() + 2..];
            let (width, set) = match after_x.split_once(", glue set ") {
                Some((w, s)) => {
                    let (sign, s) = match s.strip_prefix("- ") {
                        Some(s) => (GlueSign::Shrinking, s),
                        None => (GlueSign::Stretching, s),
                    };
                    let (ratio, order) = order_of(s.trim());
                    (w.parse().unwrap(), Some((sign, ratio, order)))
                }
                None => (after_x.trim().parse().unwrap(), None),
            };
            current = Some(Shown {
                width,
                set,
                glue: Vec::new(),
            });
        } else if let (Some(cur), Some(rest)) = (current.as_mut(), line.strip_prefix(".\\glue")) {
            // `(\medmuskip) 2.22217 plus 1.11108 minus 2.22217` or `2.0 plus 3.0`.
            let rest = match rest.find(") ") {
                Some(i) if rest.starts_with('(') => &rest[i + 2..],
                _ => rest.trim_start(),
            };
            let words: Vec<&str> = rest.split_whitespace().collect();
            let natural = words[0].parse().unwrap();
            let mut stretch = (0.0, GlueOrder::Normal);
            let mut shrink = 0.0;
            let mut i = 1;
            while i + 1 < words.len() {
                match words[i] {
                    "plus" => stretch = order_of(words[i + 1]),
                    "minus" => shrink = words[i + 1].parse().unwrap(),
                    _ => {}
                }
                i += 2;
            }
            cur.glue.push((natural, stretch.0, stretch.1, shrink));
        } else if current.is_some() && !line.starts_with('.') {
            out.push(current.take().unwrap());
        }
    }
    out.extend(current);
    out.dedup();
    out
}

fn f5(v: f64) -> String {
    format!("{:.5}", v)
}

/// A value in scaled points (1/65536), as TeX stores and prints it.
fn sp(v: f64) -> i64 {
    (v * 65536.0).round() as i64
}

fn cm() -> CmMathMetrics {
    CmMathMetrics::latex_10pt()
}

/// The glue children of a laid-out formula, as the log lists them.
fn glue_of(b: &MathBox) -> Vec<(String, String, GlueOrder, String)> {
    let BoxKind::HBox(children) = &b.kind else {
        panic!("formula is not an hbox")
    };
    children
        .iter()
        .filter_map(|c| match c.content.kind {
            BoxKind::Glue {
                stretch, shrink, ..
            } => Some((
                f5(c.content.width),
                f5(stretch.amount),
                stretch.order,
                f5(shrink.amount),
            )),
            _ => None,
        })
        .collect()
}

fn expected_glue(s: &Shown) -> Vec<(String, String, GlueOrder, String)> {
    s.glue
        .iter()
        .map(|&(w, st, o, sh)| (f5(w), f5(st), o, f5(sh)))
        .collect()
}

/// The formulas of glue-set.tex in order, with the `to` width (None for
/// the natural box).
fn cases() -> Vec<(MathList, Style, Option<f64>)> {
    let abcde = || MathList::symbols("a+b+c+d=e");
    let with_glue = |g: Atom| {
        let mut l = MathList::symbols("a");
        l.atoms.push(g);
        l.atoms.extend(MathList::symbols("b+c=d").atoms);
        l
    };
    vec![
        (abcde(), Style::DISPLAY, None),
        (abcde(), Style::DISPLAY, Some(65.0)),
        (abcde(), Style::DISPLAY, Some(80.0)),
        (abcde(), Style::DISPLAY, Some(20.0)),
        (
            with_glue(Atom::glue_flex(
                3.0,
                0.0,
                MathFlex::infinite(1.0, GlueOrder::Fill),
                MathFlex::ZERO,
            )),
            Style::DISPLAY,
            Some(100.0),
        ),
        (
            with_glue(Atom::glue_flex(
                0.0,
                2.0,
                MathFlex::pt(3.0),
                MathFlex::pt(1.0),
            )),
            Style::DISPLAY,
            Some(30.0),
        ),
        (abcde(), Style::TEXT, Some(30.0)),
        (MathList::symbols("a+b=c"), Style::SCRIPT, Some(30.0)),
    ]
}

#[test]
fn the_oracle_log_has_every_case() {
    assert_eq!(shown().len(), cases().len());
}

#[test]
fn muskips_carry_plain_tex_stretch_and_shrink() {
    // \medmuskip 2.22217 plus 1.11108 minus 2.22217 and \thickmuskip
    // 2.77771 plus 2.77771 at 10pt (mu = 36408sp), none in script style.
    let m = cm();
    for ((list, style, _), s) in cases().iter().zip(shown()) {
        let b = layout(list, *style, &m);
        assert_eq!(glue_of(&b), expected_glue(&s), "{style:?} {list:?}");
    }
}

#[test]
fn natural_width_and_totals_match_the_log() {
    let m = cm();
    let (list, style, _) = &cases()[0];
    let s = &shown()[0];
    let b = layout(list, *style, &m);
    let totals = b.glue_totals();
    assert_eq!(f5(totals.natural), f5(s.width));
    // The log prints each glue rounded to 5 decimals; their sum carries up
    // to 0.5e-5 of rounding per glue.
    let stretch: f64 = s.glue.iter().map(|g| g.1).sum();
    let shrink: f64 = s.glue.iter().map(|g| g.3).sum();
    let slack = 0.5e-5 * s.glue.len() as f64 + 1e-9;
    assert!(
        (totals.stretch[0] - stretch).abs() <= slack,
        "{} vs {stretch}",
        totals.stretch[0]
    );
    assert!(
        (totals.shrink[0] - shrink).abs() <= slack,
        "{} vs {shrink}",
        totals.shrink[0]
    );
    assert_eq!(totals.stretch_order(), GlueOrder::Normal);
}

#[test]
fn pack_to_sets_glue_like_hpack() {
    let m = cm();
    for (i, ((list, style, to), s)) in cases().iter().zip(shown()).enumerate().skip(1) {
        let to = to.unwrap();
        let b = layout(list, *style, &m);
        let packed = b.pack_to(to);
        assert_eq!(f5(packed.root.width), f5(s.width), "case {}", i + 1);
        match s.set {
            Some((sign, ratio, order)) => {
                assert_eq!(packed.set.sign, sign, "case {}", i + 1);
                assert_eq!(packed.set.order, order, "case {}", i + 1);
                // `\showbox` prints `glue set` via print_glue(round(unity *
                // g)): compare in scaled points, as TeX rounds before printing.
                assert_eq!(sp(packed.set.ratio), sp(ratio), "case {}", i + 1);
            }
            // No glue: TeX shows no glue set (underfull/overfull, rigid).
            None => assert_eq!(packed.set.sign, GlueSign::Normal, "case {}", i + 1),
        }
        // The contents end where the set glue puts them: at the box width
        // unless finite shrink ran out (overfull) or nothing could stretch.
        let runs = positioned_runs(&packed.root, (0.0, 0.0));
        let last = runs
            .glyphs
            .iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        let end = last.x + last.width;
        let natural = b.width;
        let expected_end = match packed.set.sign {
            GlueSign::Normal => natural,
            _ if packed.overfull > 0.0 => to + packed.overfull,
            _ => to,
        };
        assert_eq!(f5(end), f5(expected_end), "case {}", i + 1);
        // Overfull by exactly what the finite shrink cannot absorb.
        if packed.overfull > 0.0 {
            assert_eq!(
                f5(packed.overfull),
                f5(natural - packed.totals.shrink[0] - to),
                "case {}",
                i + 1
            );
        }
    }
}

#[test]
fn shrink_moves_every_later_glyph_by_the_glue_before_it() {
    // Case 2 (`to 65pt`, glue set -0.65746): the n-th glyph after k
    // \medmuskip glues moves left by k * 2.22217 * 0.65746.
    let m = cm();
    let (list, style, to) = &cases()[1];
    let b = layout(list, *style, &m);
    let packed = b.pack_to(to.unwrap());
    let before = positioned_runs(&b, (0.0, 0.0));
    let after = positioned_runs(&packed.root, (0.0, 0.0));
    let ratio = packed.set.ratio;
    let shrink = 4.0 * 36408.0 / 65536.0;
    for (k, (g0, g1)) in before.glyphs.iter().zip(&after.glyphs).enumerate() {
        // Glyphs run a + b + c + d = e: glyph k has k \medmuskip glues
        // before it up to "d" (6), and the \thickmuskip before "=" and "e"
        // has no shrink.
        let shrinking_glues = k.min(6) as f64;
        assert_eq!(
            f5(g0.x - g1.x),
            f5(shrinking_glues * shrink * ratio),
            "glyph {k}"
        );
        assert_eq!(g0.baseline_y, g1.baseline_y);
    }
}
