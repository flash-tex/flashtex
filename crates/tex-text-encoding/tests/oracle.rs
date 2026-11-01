//! Replays the committed pdflatex oracle (tests/oracle/expected.json) without TeX.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use flashtex_tex_text_encoding::encoding::Encoding;
use flashtex_tex_text_encoding::fonts::{bundled_glyph, glyph_name, pdf_base_font, Family};
use flashtex_tex_text_encoding::layout::{flatten, KernKind, Node};
use flashtex_tex_text_encoding::tfm::ScaledFont;
use flashtex_tex_text_encoding::typeset::{typeset_hbox, FontProvider, Setup};
use serde_json::Value;

struct Fixtures(HashMap<String, ScaledFont>);

impl FontProvider for Fixtures {
    fn font(&self, tfm: &str) -> Option<&ScaledFont> {
        self.0.get(tfm)
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_fonts() -> Fixtures {
    let dir = crate_dir().join("tests/fixtures/tfm");
    let mut map = HashMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_some_and(|e| e == "tfm") {
            let name = p.file_stem().unwrap().to_str().unwrap().to_string();
            let bytes = std::fs::read(&p).unwrap();
            map.insert(
                name.clone(),
                ScaledFont::from_bytes(&name, &bytes, 0).unwrap(),
            );
        }
    }
    Fixtures(map)
}

fn setup(variant: &str) -> Setup {
    let (enc, fam) = match variant {
        "ot1-cm" => (Encoding::OT1, Family::ComputerModern),
        "t1-cm" => (Encoding::T1, Family::ComputerModern),
        "ot1-lm" => (Encoding::OT1, Family::LatinModern),
        "t1-lm" | "t1-lm-inputenc" => (Encoding::T1, Family::LatinModern),
        other => panic!("unknown variant {other}"),
    };
    Setup::article10(enc, fam)
}

fn summarize_expected(nodes: &[Value]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n["kind"].as_str().unwrap() {
            "char" => format!("char {} {}", n["tfm"].as_str().unwrap_or("?"), n["code"]),
            "kern" => format!("kern {}", n["width_sp"]),
            "accent_kern" => format!("accent_kern {}", n["width_sp"]),
            "glue" => format!(
                "glue {} {} {}",
                n["width_sp"], n["stretch_sp"], n["shrink_sp"]
            ),
            "penalty" => format!("penalty {}", n["value"]),
            other => other.to_string(),
        })
        .collect()
}

fn summarize_actual(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Char { tfm, code, .. } => format!("char {tfm} {code}"),
            Node::Kern {
                width,
                kind: KernKind::Accent,
            } => format!("accent_kern {width}"),
            Node::Kern { width, .. } => format!("kern {width}"),
            Node::Glue { spec, .. } => {
                format!("glue {} {} {}", spec.width, spec.stretch, spec.shrink)
            }
            Node::Penalty(p) => format!("penalty {p}"),
            Node::HBox(_) => "hbox".into(),
            Node::VBox(_) => "vbox".into(),
        })
        .collect()
}

const POSITION_TOLERANCE_PT: f64 = 0.1;

#[test]
fn pdflatex_oracle_replays() {
    let fonts = load_fonts();
    let text = std::fs::read_to_string(crate_dir().join("tests/oracle/expected.json")).unwrap();
    let oracle: Value = serde_json::from_str(&text).unwrap();
    let otf: Value = serde_json::from_str(
        &std::fs::read_to_string(crate_dir().join("tests/oracle/otf-glyph-names.json")).unwrap(),
    )
    .unwrap();
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut max_dev = 0.0f64;
    let cases = oracle["cases"].as_array().unwrap();
    let mut accent_cases = 0;
    let mut sf_glues = 0;
    for case in cases {
        let key = case["key"].as_str().unwrap().to_string();
        let input = case["input"].as_str().unwrap();
        let out = typeset_hbox(&fonts, setup(case["variant"].as_str().unwrap()), input);
        let mut why = Vec::new();
        if !out.unsupported.is_empty() {
            why.push(format!("unsupported: {:?}", out.unsupported));
        }
        let exp_errors: Vec<String> = case["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        if exp_errors != out.errors {
            why.push(format!(
                "errors: expected {exp_errors:?} got {:?}",
                out.errors
            ));
        }
        let exp_nodes = summarize_expected(case["nodes"].as_array().unwrap());
        let act_nodes = summarize_actual(&out.list);
        if exp_nodes != act_nodes {
            why.push(format!("nodes: expected {exp_nodes:?} got {act_nodes:?}"));
        }
        sf_glues += act_nodes.iter().filter(|n| n.starts_with("glue")).count();
        if act_nodes.iter().any(|n| n.starts_with("accent_kern")) {
            accent_cases += 1;
        }
        let bx = &case["box"];
        if !bx.is_null() {
            let exp = (
                bx["width_sp"].as_i64().unwrap(),
                bx["height_sp"].as_i64().unwrap(),
                bx["depth_sp"].as_i64().unwrap(),
            );
            let act = (
                out.hbox.width as i64,
                out.hbox.height as i64,
                out.hbox.depth as i64,
            );
            if exp != act {
                why.push(format!("box (w,h,d): expected {exp:?} got {act:?}"));
            }
        }
        let placed = flatten(&out.list);
        let exp_glyphs = case["glyphs"].as_array().unwrap();
        if placed.len() != exp_glyphs.len() {
            why.push(format!(
                "glyph count: expected {} got {}",
                exp_glyphs.len(),
                placed.len()
            ));
        } else {
            for (g, e) in placed.iter().zip(exp_glyphs) {
                let pdf = pdf_base_font(&g.tfm).unwrap_or_default();
                if pdf != e["pdf_font"].as_str().unwrap()
                    || g.code as u64 != e["code"].as_u64().unwrap()
                {
                    why.push(format!(
                        "glyph: expected {} {} got {pdf} {}",
                        e["pdf_font"], e["code"], g.code
                    ));
                    continue;
                }
                let name = glyph_name(&g.tfm, g.code);
                if name != e["glyph"].as_str() {
                    why.push(format!("glyph name: expected {} got {name:?}", e["glyph"]));
                }
                if let Some(b) = bundled_glyph(&g.tfm, g.code) {
                    let present = otf[&b.file]["glyphs"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|n| n == b.glyph));
                    if !present && !otf[&b.file].is_null() {
                        why.push(format!("OTF {} lacks glyph {}", b.file, b.glyph));
                    }
                }
                let x = g.x as f64 / 65536.0;
                let y = -(g.y as f64) / 65536.0;
                let dx = (x - e["x_pt"].as_f64().unwrap()).abs();
                let dy = (y - e["y_pt"].as_f64().unwrap()).abs();
                max_dev = max_dev.max(dx).max(dy);
                if dx > POSITION_TOLERANCE_PT || dy > POSITION_TOLERANCE_PT {
                    why.push(format!(
                        "position of {} {}: expected ({}, {}) got ({x:.4}, {y:.4})",
                        e["glyph"], g.code, e["x_pt"], e["y_pt"]
                    ));
                }
            }
        }
        if !why.is_empty() {
            failures.insert(key, why);
        }
    }
    let total = cases.len();
    let passed = total - failures.len();
    println!("oracle: {passed}/{total} cases pass; max glyph position deviation {max_dev:.4} pt; {accent_cases} cases with \\accent kerns; {sf_glues} interword glues compared");
    for (k, why) in &failures {
        println!("FAIL {k}");
        for w in why {
            println!("    {w}");
        }
    }
    let known: Vec<&str> = include_str!("oracle/known-failures.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let unexpected: Vec<&String> = failures
        .keys()
        .filter(|k| !known.contains(&k.as_str()))
        .collect();
    let fixed: Vec<&&str> = known
        .iter()
        .filter(|k| !failures.contains_key(**k))
        .collect();
    assert!(
        unexpected.is_empty(),
        "unexpected oracle failures: {unexpected:?}"
    );
    assert!(
        fixed.is_empty(),
        "known failures now pass; remove them from known-failures.txt: {fixed:?}"
    );
}
