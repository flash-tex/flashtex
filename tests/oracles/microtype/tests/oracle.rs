//! pdflatex oracle: line breaks, per-glyph expansion, margin kerns and glyph
//! x positions for every fixture in `tests/oracle/expected/`, reproduced by
//! the reference pdfTeX line breaker in `tests/common` on top of the crate's
//! primitives. No TeX is run.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::collapsible_if, clippy::needless_late_init, clippy::while_immutable_condition, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

mod common;

use std::fs;
use std::path::Path;

use common::*;

struct Outcome {
    name: String,
    lines: usize,
    breaks_ok: bool,
    expansion_ok: bool,
    margins_ok: bool,
    max_dx_pt: f64,
    detail: String,
}

fn line_signature(nodes: &[Node]) -> Vec<(String, u8, i32)> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Char { font, code, expansion, .. } => Some((font.clone(), *code, *expansion)),
            _ => None,
        })
        .collect()
}

fn margin_kerns(nodes: &[Node]) -> Vec<(KernKind, i32)> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Kern { width, kind: k @ (KernKind::LeftMargin | KernKind::RightMargin) } => Some((*k, *width)),
            _ => None,
        })
        .collect()
}

fn run(path: &Path) -> Outcome {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let text = fs::read_to_string(path).unwrap();
    let fx = load_fixture(&name, &text);
    let env = Env::from_fixture(&fx);
    let list1 = finish_list1(fx.list1.clone(), &env);
    let list2 = strip_list2(fx.list2.clone());
    let br = line_break(&env, &list1, &list2);
    let lines = post_line_break(&env, &br);
    let expected: Vec<&Node> = fx.result.iter().filter(|n| matches!(n, Node::HBox { .. })).collect();

    let mut breaks_ok = lines.len() == expected.len();
    let mut expansion_ok = true;
    let mut margins_ok = true;
    let mut max_dx = 0f64;
    let mut detail = String::new();
    for (k, (got, exp)) in lines.iter().zip(expected.iter()).enumerate() {
        let Node::HBox { children, sign, set, order, .. } = exp else { unreachable!() };
        let sg = line_signature(&got.nodes);
        let se = line_signature(children);
        let strip = |v: &Vec<(String, u8, i32)>| v.iter().map(|(f, c, _)| (f.clone(), *c)).collect::<Vec<_>>();
        if strip(&sg) != strip(&se) {
            breaks_ok = false;
            if detail.is_empty() {
                let txt = |v: &Vec<(String, u8, i32)>| v.iter().map(|(_, c, _)| *c as char).collect::<String>();
                detail = format!("line {k}: got {:?}\n   expected {:?}", txt(&sg), txt(&se));
            }
            continue;
        }
        if sg != se {
            expansion_ok = false;
            if detail.is_empty() {
                detail = format!("line {k}: expansion got ratio {} {:?} expected {:?}", got.ratio, &sg[..3.min(sg.len())], &se[..3.min(se.len())]);
            }
        }
        if margin_kerns(&got.nodes) != margin_kerns(children) {
            margins_ok = false;
            if detail.is_empty() {
                detail = format!("line {k}: margin kerns got {:?} expected {:?}", margin_kerns(&got.nodes), margin_kerns(children));
            }
        }
        let pg = glyph_positions(&env, &got.nodes, got.sign, got.order, got.set);
        let pe = glyph_positions(&env, children, *sign, *order, *set);
        for (a, b) in pg.iter().zip(pe.iter()) {
            max_dx = max_dx.max((a.1 - b.1).abs() / 65536.0);
        }
    }
    if !breaks_ok && detail.is_empty() {
        detail = format!("{} lines, expected {}", lines.len(), expected.len());
    }
    Outcome { name, lines: expected.len(), breaks_ok, expansion_ok, margins_ok, max_dx_pt: max_dx, detail }
}

#[test]
fn pdflatex_oracle_fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/oracle/expected");
    let mut paths: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    assert!(paths.len() >= 25, "need >= 25 oracle fixtures, found {}", paths.len());
    let outcomes: Vec<Outcome> = paths.iter().map(|p| run(p)).collect();
    let mut failures = vec![];
    let (mut nb, mut ne, mut nm, mut maxdx) = (0, 0, 0, 0f64);
    for o in &outcomes {
        nb += o.breaks_ok as usize;
        ne += (o.breaks_ok && o.expansion_ok) as usize;
        nm += (o.breaks_ok && o.margins_ok) as usize;
        maxdx = maxdx.max(o.max_dx_pt);
        let ok = o.breaks_ok && o.expansion_ok && o.margins_ok && o.max_dx_pt <= 0.1;
        println!(
            "{:<40} lines={:>2} breaks={} expansion={} margins={} max_dx={:.5}pt {}",
            o.name, o.lines, o.breaks_ok, o.expansion_ok, o.margins_ok, o.max_dx_pt, o.detail
        );
        if !ok {
            failures.push(o.name.clone());
        }
    }
    println!(
        "SUMMARY fixtures={} breaks_identical={} expansion_identical={} margin_kerns_identical={} max_glyph_dx={:.5}pt",
        outcomes.len(), nb, ne, nm, maxdx
    );
    assert!(failures.is_empty(), "oracle mismatches: {failures:?}");
}
