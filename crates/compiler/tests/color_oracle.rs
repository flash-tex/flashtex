//! Colour operators against pdfTeX: every row of
//! `tests/color_oracle/operators.tsv` (made by `scripts/color_oracle.py`
//! with TeX Live 2026; cargo never runs TeX) is replayed through
//! `flashtex_compiler::color` and must produce the same fill and stroke
//! operator values exactly.

use flashtex_compiler::color::{Colors, DeviceColor};
use std::collections::BTreeMap;

const TSV: &str = include_str!("color_oracle/operators.tsv");

/// The preamble of the xcolor suites (`DEFS` in the generator).
const XCOLOR_DEFS: &[(&str, &str, &str, &str)] = &[
    ("define", "rgbA", "rgb", "0.1,0.2,0.3"),
    ("define", "rgbB", "rgb", "1,.5,.25"),
    ("define", "rgbC", "rgb", "0.123456,0.98765,0.5"),
    ("define", "RGBa", "RGB", "12,200,33"),
    ("define", "RGBb", "RGB", "255,128,0"),
    ("define", "HTMLa", "HTML", "FF8800"),
    ("define", "HTMLb", "HTML", "1a2B3c"),
    ("define", "cmykA", "cmyk", ".1,.2,.3,.4"),
    ("define", "cmykB", "cmyk", "0,1,0.5,0"),
    ("define", "grayA", "gray", "0.35"),
    ("define", "GrayA", "Gray", "7"),
    ("let", "letA", "", "red!40!blue"),
    ("let", "letB", "", "cmykA!50"),
    ("let", "letC", "", "rgbA"),
    ("let", "letD", "cmyk", "rgbB"),
    ("let", "letE", "gray", "red"),
];

const COLOR_DEFS: &[(&str, &str, &str, &str)] = &[
    ("define", "rgbA", "rgb", "0.1,0.2,0.3"),
    ("define", "cmykA", "cmyk", ".1,.2,.3,.4"),
    ("define", "grayA", "gray", "0.35"),
    ("define", "RGBa", "RGB", "12,200,33"),
];

fn setup(suite: &str) -> Colors {
    let (mut colors, defs) = match suite {
        "xcolor" => (Colors::xcolor("", None).0, XCOLOR_DEFS),
        "xcolor-rgb" => (Colors::xcolor("rgb", None).0, XCOLOR_DEFS),
        "xcolor-cmyk" => (Colors::xcolor("cmyk", None).0, XCOLOR_DEFS),
        "xcolor-gray" => (Colors::xcolor("gray", None).0, XCOLOR_DEFS),
        "xcolor-dvipsnames" => (Colors::xcolor("dvipsnames", None).0, &[][..]),
        "xcolor-svgnames" => (Colors::xcolor("svgnames", None).0, &[][..]),
        "xcolor-x11names" => (Colors::xcolor("x11names", None).0, &[][..]),
        "color" => (Colors::color_sty("").0, COLOR_DEFS),
        "color-dvipsnames" => (Colors::color_sty("dvipsnames").0, &[][..]),
        other => panic!("unknown suite {other}"),
    };
    for (kind, name, model, spec) in defs {
        let r = match *kind {
            "define" => colors.define("", name, model, spec),
            _ => colors.colorlet("", name, model, spec, None),
        };
        r.unwrap_or_else(|e| panic!("{suite}: {name}: {e}"));
    }
    colors
}

fn evaluate(colors: &mut Colors, label: &str) -> Result<DeviceColor, String> {
    let mut with_current = |current: &str, expr: &str| {
        let cur = colors.resolve(None, current, None).map_err(|e| e.to_string())?;
        colors.resolve(None, expr, Some(cur)).map_err(|e| e.to_string())
    };
    let undeclared = |model: &str, spec: &str| (Some(model.to_string()), spec.to_string());
    let (model, expr) = match label {
        "dot-mix" => return with_current("blue", ".!50"),
        "dot-mix-cmyk" => return with_current("cmykA", ".!30!red"),
        "dot-plain" => return with_current("RGBa", "."),
        "undeclared-rgb" => undeclared("rgb", "0.1,0.2,0.3"),
        "undeclared-cmyk" => undeclared("cmyk", "0,1,0.5,0"),
        "undeclared-gray" => undeclared("gray", "0.5"),
        "undeclared-RGB" => undeclared("RGB", "1,2,3"),
        "undeclared-HTML" => undeclared("HTML", "ABCDEF"),
        "undeclared-rgb-long" => undeclared("rgb", "0.333333,0.666666,0.999999"),
        "color-decl" => (None, "red!60".to_string()),
        "color-decl-undeclared" => undeclared("cmyk", ".2,.4,.6,.8"),
        l if l.starts_with("[named]") => undeclared("named", &l["[named]".len()..]),
        l => (None, l.to_string()),
    };
    colors.resolve(model.as_deref(), &expr, None).map_err(|e| e.to_string())
}

/// Operands as values (`0.50` = `.5` = `0.5`), operator kept.
fn canonical(op: &str) -> String {
    let mut tokens: Vec<String> = op.split_whitespace().map(str::to_string).collect();
    let name = tokens.pop().unwrap_or_default();
    let values: Vec<String> = tokens
        .iter()
        .map(|t| {
            let (int, frac) = t.split_once('.').unwrap_or((t, ""));
            let int = int.trim_start_matches('0');
            let int = if int.is_empty() { "0" } else { int };
            let frac = frac.trim_end_matches('0');
            if frac.is_empty() { int.to_string() } else { format!("{int}.{frac}") }
        })
        .collect();
    format!("{} {name}", values.join(" "))
}

#[test]
fn colour_operators_match_pdftex() {
    let mut suites: BTreeMap<&str, Colors> = BTreeMap::new();
    let (mut rows, mut failures) = (0, Vec::new());
    for line in TSV.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let cols: Vec<&str> = line.split('\t').collect();
        let (suite, label, fill, stroke) = (cols[0], cols[1], cols[2], cols[3]);
        let colors = suites.entry(suite).or_insert_with(|| setup(suite));
        rows += 1;
        match evaluate(colors, label) {
            Ok(d) => {
                if d.fill_operator() != canonical(fill) || d.stroke_operator() != canonical(stroke) {
                    failures.push(format!("{suite}\t{label}\texpected {fill} | got {}", d.fill_operator()));
                }
            }
            Err(e) => failures.push(format!("{suite}\t{label}\texpected {fill} | error {e}")),
        }
    }
    assert!(rows > 900, "only {rows} oracle rows");
    assert!(failures.is_empty(), "{} of {rows} differ:\n{}", failures.len(), failures.join("\n"));
}
