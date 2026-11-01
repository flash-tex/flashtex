//! Oracle comparison: replays every fixture of `tests/oracle/fixtures.txt`
//! through a small TeX-subset interpreter driving [`BoxEngine`] and the
//! LaTeX layer, and compares against pdfTeX data in `expected.txt`
//! (dimensions, `\badness`, `\showbox` text, diagnostics, positions).

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::type_complexity, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

mod dsl;

use std::collections::HashMap;

use flashtex_tex_boxes::engine::BoxEngine;
use flashtex_tex_boxes::latex;
use flashtex_tex_boxes::node::{BoxNode, ListKind, Node, Whatsit};
use flashtex_tex_boxes::pack::{PackOrigin, PackParams, PackSpec, hpack};
use flashtex_tex_boxes::shipout::{ship_out, whatsit_positions};
use flashtex_tex_boxes::{BoxDim, NoChars};

struct Expected {
    line: i32,
    dim: Vec<i64>,
    diag: String,
    boxtext: String,
    pos: Option<Vec<(i32, i32)>>,
}

fn load() -> (Vec<(String, String, String)>, HashMap<String, Expected>, HashMap<String, i64>) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle");
    let fixtures_src = std::fs::read_to_string(format!("{dir}/fixtures.txt")).unwrap();
    let mut fixtures = Vec::new();
    for line in fixtures_src.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, " | ");
        let name = parts.next().unwrap().trim().to_string();
        let setup = dsl::expand_repeats(parts.next().unwrap().trim());
        let body = dsl::expand_repeats(parts.next().unwrap().trim());
        fixtures.push((name, setup, body));
    }
    let exp_src = std::fs::read_to_string(format!("{dir}/expected.txt")).unwrap();
    let mut expected = HashMap::new();
    let mut params = HashMap::new();
    let mut lines = exp_src.lines().peekable();
    while let Some(l) = lines.next() {
        if let Some(rest) = l.strip_prefix("PARAM ") {
            let mut it = rest.split(' ');
            params.insert(it.next().unwrap().to_string(), it.next().unwrap().parse().unwrap());
        } else if let Some(name) = l.strip_prefix("FIX ") {
            let mut e = Expected { line: 0, dim: vec![], diag: String::new(), boxtext: String::new(), pos: None };
            let mut section = "";
            let mut diag_lines: Vec<&str> = vec![];
            let mut box_lines: Vec<&str> = vec![];
            for l in lines.by_ref() {
                if l == "END" {
                    break;
                } else if let Some(n) = l.strip_prefix("LINE ") {
                    e.line = n.parse().unwrap();
                } else if let Some(d) = l.strip_prefix("DIM ") {
                    e.dim = d.split(' ').map(|x| x.parse().unwrap()).collect();
                } else if l == "DIAG" || l == "BOX" {
                    section = if l == "DIAG" { "diag" } else { "box" };
                } else if let Some(p) = l.strip_prefix("POS ") {
                    e.pos = Some(
                        p.split(' ')
                            .map(|xy| {
                                let (x, y) = xy.split_once(',').unwrap();
                                (x.parse().unwrap(), y.parse().unwrap())
                            })
                            .collect(),
                    );
                } else if let Some(text) = l.strip_prefix('|') {
                    if section == "diag" { diag_lines.push(text) } else { box_lines.push(text) }
                }
            }
            e.diag = diag_lines.iter().map(|l| format!("{l}\n")).collect();
            e.boxtext = box_lines.join("\n");
            expected.insert(name.to_string(), e);
        }
    }
    (fixtures, expected, params)
}

fn new_engine(line: i32, term_dirty: bool) -> BoxEngine {
    let mut e = BoxEngine::new(Box::new(NoChars));
    latex::setup_article_10pt(&mut e);
    e.set_int("showboxdepth", 10000, true);
    e.set_int("showboxbreadth", 10000, true);
    e.max_print_line = 100000;
    e.line = line;
    // pdfTeX's terminal state at the start of a fixture in the oracle run:
    // `\showbox0`'s error() ends with print_ln (§90), leaving the terminal
    // clean, but a preceding position pass ships out a page and leaves ` [n]`
    // on the terminal line. The first fixture follows `(./oracle.aux)`.
    e.term_offset = usize::from(term_dirty);
    e
}

fn run(setup: &str, body: &str, line: i32, with_pos: bool, term_dirty: bool) -> Result<BoxEngine, String> {
    let mut e = new_engine(line, term_dirty);
    let src = format!("\\begingroup {setup}\\global\\setbox0{body}\\endgroup");
    dsl::execute(&mut e, &src, with_pos).map_err(|err| format!("{err}"))?;
    Ok(e)
}

#[test]
fn article_parameters_match_oracle() {
    let (_, _, params) = load();
    let e = new_engine(0, false);
    assert_eq!(params["baselineskip"], i64::from(e.skip("baselineskip").0.width));
    assert_eq!(params["strutht"], i64::from(e.box_dimen(latex::STRUTBOX, BoxDim::Height)));
    assert_eq!(params["strutdp"], i64::from(e.box_dimen(latex::STRUTBOX, BoxDim::Depth)));
    assert_eq!(params["axisheight"], i64::from(e.axis_height));
    assert_eq!(params["parindent"], i64::from(e.dimen("parindent")));
    assert_eq!(params["hsize"], i64::from(e.dimen("hsize")));
    assert_eq!(params["fboxsep"], i64::from(e.dimen("fboxsep")));
    assert_eq!(params["fboxrule"], i64::from(e.dimen("fboxrule")));
    assert_eq!(params["hfuzz"], i64::from(e.dimen("hfuzz")));
    assert_eq!(params["lineskip"], i64::from(e.skip("lineskip").0.width));
}

#[test]
fn oracle_fixtures() {
    let (fixtures, expected, _) = load();
    assert!(fixtures.len() >= 80, "need at least 80 fixtures, have {}", fixtures.len());
    let mut failures = Vec::new();
    let mut counts = [0usize; 5]; // dims, badness, box, diag, pos
    let mut pos_total = 0;
    let mut term_dirty = true;
    for (name, setup, body) in &fixtures {
        let exp = expected.get(name).unwrap_or_else(|| panic!("no expected data for {name}; rerun generate.py"));
        let dirty = std::mem::replace(&mut term_dirty, exp.pos.is_some());
        let e = match run(setup, body, exp.line, false, dirty) {
            Ok(e) => e,
            Err(err) => {
                failures.push(format!("{name}: interpreter error: {err}"));
                continue;
            }
        };
        let dims = [0u32; 0];
        let _ = dims;
        let got_dim = vec![
            i64::from(e.box_dimen(0, BoxDim::Width)),
            i64::from(e.box_dimen(0, BoxDim::Height)),
            i64::from(e.box_dimen(0, BoxDim::Depth)),
        ];
        if got_dim == exp.dim[..3] {
            counts[0] += 1;
        } else {
            failures.push(format!("{name}: dims got {got_dim:?} want {:?}", &exp.dim[..3]));
        }
        if i64::from(e.last_badness()) == exp.dim[3] {
            counts[1] += 1;
        } else {
            failures.push(format!("{name}: badness got {} want {}", e.last_badness(), exp.dim[3]));
        }
        let got_box = e.show_box(0);
        let got_box = got_box.trim_matches('\n');
        if got_box == exp.boxtext {
            counts[2] += 1;
        } else {
            failures.push(format!("{name}: showbox differs\n--- got\n{got_box}\n--- want\n{}", exp.boxtext));
        }
        if e.log() == exp.diag {
            counts[3] += 1;
        } else {
            failures.push(format!("{name}: diagnostics differ\n--- got\n{:?}\n--- want\n{:?}", e.log(), exp.diag));
        }
        if let Some(want) = &exp.pos {
            pos_total += 1;
            match run(setup, body, exp.line, true, dirty) {
                Ok(mut e) => {
                    let b = e.take_box_register(0).expect("box0");
                    let wrapper = hpack(
                        vec![Node::Whatsit(Whatsit { tag: 0, display: "pdfsavepos".into() }), Node::Box(b)],
                        PackSpec::NATURAL,
                        false,
                        &PackParams::default(),
                        PackOrigin::default(),
                        &NoChars,
                    );
                    let events = ship_out(&wrapper.node, 0, 0, &NoChars);
                    let pos = whatsit_positions(&events);
                    let (h0, v0) = (pos[0].1, pos[0].2);
                    let got: Vec<(i32, i32)> = pos.iter().map(|(_, h, v)| (h - h0, v - v0)).collect();
                    if &got == want {
                        counts[4] += 1;
                    } else {
                        failures.push(format!("{name}: positions got {got:?}\n want {want:?}"));
                    }
                }
                Err(err) => failures.push(format!("{name}: interpreter error (pos): {err}")),
            }
        }
    }
    let _ = BoxNode::null(ListKind::H);
    let n = fixtures.len();
    eprintln!(
        "oracle: {n} fixtures; dims {}/{n}, badness {}/{n}, showbox {}/{n}, diagnostics {}/{n}, positions {}/{pos_total}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );
    if !failures.is_empty() {
        panic!("{} mismatches:\n{}", failures.len(), failures.join("\n\n"));
    }
}
