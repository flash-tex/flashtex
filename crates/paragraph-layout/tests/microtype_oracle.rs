//! pdflatex + microtype oracle for `layout_paragraph_microtype`: every
//! fixture of `crates/microtype/tests/oracle/expected/` (pdfTeX 1.40.27
//! `\showbox` dumps: the paragraph's hlist, the fonts' widths and
//! `\lpcode`/`\rpcode`/`\efcode`, and the broken result) is converted into
//! this crate's items and broken by this crate's breaker; line breaks,
//! per-glyph expansion, margin kerns and glyph x positions must be
//! pdfTeX's. The dump parser and pdfTeX's `hlist_out` positions are the
//! microtype crate's own test support, shared by path. No TeX is run.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::collapsible_if, clippy::needless_late_init, clippy::while_immutable_condition, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

mod mt_convert;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use flashtex_paragraph_layout::{
    Algorithm, BreakMode, FORCED_BREAK, Item, LineBreakParams, MicroItem, Microtype, layout_paragraph_microtype,
};
use mt_convert::*;

struct Outcome {
    name: String,
    breaks_ok: bool,
    expansion_ok: bool,
    margins_ok: bool,
    max_dx_pt: f64,
    detail: String,
}

fn run(path: &Path) -> Outcome {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let fx = load_fixture(&name, &fs::read_to_string(path).unwrap());
    let env = Env::from_fixture(&fx);
    let mut list = strip_list2(fx.list2.clone());
    if !matches!(list.last(), Some(Node::Glue(g)) if g.name.as_deref() == Some("parfillskip")) {
        list = finish_list1(list, &env);
    }
    // Discretionaries already in the pass-1 list are explicit.
    let explicit_ctx = disc_contexts(&fx.list1);
    let explicit: Vec<bool> = disc_contexts(&list).iter().map(|c| explicit_ctx.contains(c)).collect();
    let mut conv = Conv { fonts: &fx.fonts, params: BTreeMap::new(), ids: BTreeMap::new() };
    let (mut items, mut micro) = conv.items(&list, &explicit, fx.int("hyphenpenalty"), fx.int("exhyphenpenalty"));
    items.push(Item::penalty(FORCED_BREAK));
    micro.push(MicroItem::default());
    let params = LineBreakParams {
        line_width: pt(env.hsize),
        mode: BreakMode::Justified,
        algorithm: Algorithm::TotalFit,
        pretolerance: f64::from(env.pretolerance),
        tolerance: f64::from(env.tolerance),
        emergency_stretch: pt(env.emergency_stretch),
        line_penalty: f64::from(env.line_penalty),
        adj_demerits: f64::from(env.adj_demerits),
        double_hyphen_demerits: f64::from(env.double_hyphen_demerits),
        final_hyphen_demerits: f64::from(env.final_hyphen_demerits),
        parindent: 0.0,
        left_skip: glue(&env.left_skip),
        right_skip: glue(&env.right_skip),
        baselineskip: 12.0,
        lineskip: 1.0,
        lineskiplimit: 0.0,
        hfuzz: 0.1,
        hbadness: 1000.0,
    };
    let mt = Microtype { protrude_chars: env.protrude_chars, adjust_spacing: env.adjust_spacing, items: micro };
    let (lines, mlines) = layout_paragraph_microtype(&items, &params, &mt).expect("layout");
    let expected: Vec<&Node> = fx.result.iter().filter(|n| matches!(n, Node::HBox { .. })).collect();
    let mut o = Outcome { name, breaks_ok: lines.lines.len() == expected.len(), expansion_ok: true, margins_ok: true, max_dx_pt: 0.0, detail: String::new() };
    for (k, (line, exp)) in lines.lines.iter().zip(&expected).enumerate() {
        let Node::HBox { children, sign, set, order, .. } = exp else { unreachable!() };
        let got: Vec<(u8, i32, f64)> = line
            .runs
            .iter()
            .enumerate()
            .flat_map(|(ri, r)| {
                let e = &mlines[k].expansion[ri];
                r.glyphs.iter().enumerate().filter(|_| !r.glyphs.is_empty() && !e.is_empty()).map(move |(gi, g)| (g.gid as u8, e[gi], (r.x + g.x_offset) * 65536.0)).collect::<Vec<_>>()
            })
            .collect();
        let want_chars: Vec<(u8, i32)> = children.iter().filter_map(|n| match n {
            Node::Char { code, expansion, .. } => Some((*code, *expansion)),
            _ => None,
        }).collect();
        let want_x = glyph_positions(&env, children, *sign, *order, *set);
        let codes = |v: &[(u8, i32)]| v.iter().map(|c| c.0).collect::<Vec<_>>();
        let got_ce: Vec<(u8, i32)> = got.iter().map(|g| (g.0, g.1)).collect();
        if codes(&got_ce) != codes(&want_chars) {
            o.breaks_ok = false;
            if o.detail.is_empty() {
                let s = |v: Vec<u8>| v.into_iter().map(char::from).collect::<String>();
                o.detail = format!("line {k}: got {:?} want {:?}", s(codes(&got_ce)), s(codes(&want_chars)));
            }
            continue;
        }
        if got_ce != want_chars {
            o.expansion_ok = false;
            if o.detail.is_empty() {
                o.detail = format!("line {k}: ratio {} got {:?} want {:?}", mlines[k].expand_ratio, &got_ce[..3], &want_chars[..3]);
            }
        }
        let margins: Vec<i32> = children.iter().filter_map(|n| match n {
            Node::Kern { width, kind: KernKind::LeftMargin | KernKind::RightMargin } => Some(*width),
            _ => None,
        }).collect();
        let got_m: Vec<i32> = [mlines[k].left_margin_kern, mlines[k].right_margin_kern].into_iter().filter(|w| *w != 0).collect();
        if got_m != margins {
            o.margins_ok = false;
            if o.detail.is_empty() {
                o.detail = format!("line {k}: margin kerns got {got_m:?} want {margins:?}");
            }
        }
        for (g, w) in got.iter().zip(&want_x) {
            o.max_dx_pt = o.max_dx_pt.max((g.2 - w.1).abs() / 65536.0);
        }
    }
    if !o.breaks_ok && o.detail.is_empty() {
        o.detail = format!("{} lines, want {}", lines.lines.len(), expected.len());
    }
    o
}

#[test]
fn microtype_breaks_expansion_and_margin_kerns_match_pdftex() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../microtype/tests/oracle/expected");
    let mut paths: Vec<_> = fs::read_dir(&dir).expect("microtype oracle fixtures").map(|e| e.unwrap().path()).collect();
    paths.sort();
    assert!(paths.len() >= 32, "found {}", paths.len());
    let mut failures = Vec::new();
    let mut maxdx = 0f64;
    for p in &paths {
        let o = run(p);
        maxdx = maxdx.max(o.max_dx_pt);
        let ok = o.breaks_ok && o.expansion_ok && o.margins_ok && o.max_dx_pt <= 0.001;
        println!("{:<34} breaks={} expansion={} margins={} max_dx={:.5}pt {}", o.name, o.breaks_ok, o.expansion_ok, o.margins_ok, o.max_dx_pt, o.detail);
        if !ok {
            failures.push(o.name);
        }
    }
    println!("SUMMARY fixtures={} failures={} max_glyph_dx={maxdx:.5}pt", paths.len(), failures.len());
    assert!(failures.is_empty(), "{failures:?}");
}
