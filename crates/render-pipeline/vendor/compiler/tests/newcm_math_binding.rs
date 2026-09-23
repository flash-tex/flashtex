//! `\mathcal` draws real script capitals bound to New Computer Modern Math,
//! and `\varnothing` takes msbm10's width (FT-060).
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::newcm_math;
use flashtex_compiler::protocol::handle_line;
use flashtex_font_engine::{sha256, GlyphId, TrueTypeFace};

/// The copy the Mac app bundles (pinned in SUPPLEMENTARY-FACES.json).
const BUNDLED_FONT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/mac/Fonts/NewCMMath-Regular.otf"
);
/// The byte-identical TeX Live 2026 file the bundled copy was taken from.
const TEXLIVE_FONT: &str =
    "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/newcomputermodern/NewCMMath-Regular.otf";

/// pdfLaTeX 2026 oracle, `\setbox0\hbox{$\mathcal{X}$}\the\wd0` at 10pt
/// (article, amsmath+amssymb): cmsy10 width plus italic correction.
const PDFLATEX_CAL_WD_PT: [f64; 26] = [
    7.98471, 6.87221, 5.84865, 7.99171, 6.17221, 8.18053, 6.54169, 8.5417, 6.18336, 8.625, 7.76393,
    6.89725, 12.00899, 9.67848, 8.23892, 7.7778, 8.1667, 8.47504, 6.80551, 7.98811, 7.25137,
    6.95004, 10.70004, 8.59724, 7.50558, 8.04167,
];
/// Same oracle for `$\varnothing$`.
const PDFLATEX_VARNOTHING_WD_PT: f64 = 7.7778;

#[test]
fn advances_are_read_from_the_pinned_font_program() {
    let Some(bytes) = [BUNDLED_FONT, TEXLIVE_FONT]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
    else {
        eprintln!("skipped: neither {BUNDLED_FONT} nor {TEXLIVE_FONT} is present");
        return;
    };
    assert_eq!(sha256::hex(&sha256::digest(&bytes)), newcm_math::SHA256);
    let face = TrueTypeFace::parse(bytes).expect("parse New Computer Modern Math");
    // amssymb symbols only New Computer Modern Math carries (`amssymb::NEWCM_ADVANCES`).
    for (c, advance) in newcm_math::ADVANCES.iter().chain(flashtex_compiler::amssymb::NEWCM_ADVANCES) {
        let gid = face
            .char_map()
            .get(&(*c as u32))
            .unwrap_or_else(|| panic!("NewCMMath has no glyph for {c:?}"));
        let (actual, _) = face.hmetric(GlyphId(*gid)).expect("hmtx entry");
        assert_eq!(actual, *advance, "advance of {c:?} (U+{:04X})", *c as u32);
    }
}

/// Honest oracle pin: the letters whose NewCM advance is within 0.5pt of
/// pdfLaTeX's cmsy10 box at 10pt. A change in either set fails here.
#[test]
fn mathcal_widths_against_the_pdflatex_oracle() {
    let mut within = String::new();
    for (i, letter) in ('A'..='Z').enumerate() {
        let glyph = newcm_math::script(letter).unwrap();
        let ours = newcm_math::width_pt(&glyph.to_string(), 10.0).unwrap();
        if (ours - PDFLATEX_CAL_WD_PT[i]).abs() <= 0.5 {
            within.push(letter);
        }
    }
    assert_eq!(within, "EGINPRTUVWX");
    // HW2 uses \mathcal{P}.
    let p = newcm_math::width_pt("\u{1D4AB}", 10.0).unwrap();
    assert!((p - PDFLATEX_CAL_WD_PT[15]).abs() < 0.16, "{p}");
}

fn compile(text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("newcm-math"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    payload.set(
        "layout_capabilities",
        Value::Arr(vec![json::str_("font-hints-v1")]),
    );
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("cal"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply")
}

/// (text, family, x_pt, font_size_pt) for every text item.
fn items(reply: &Value) -> Vec<(String, String, f64, f64)> {
    let pages = reply.get("payload").unwrap().get("pages").unwrap();
    let mut out = Vec::new();
    for page in pages.as_arr().unwrap() {
        for item in page.get("items").unwrap().as_arr().unwrap() {
            let Some(text) = item.get("text").and_then(|v| v.as_str()) else {
                continue;
            };
            let family = item
                .get("font")
                .and_then(|f| f.get("family"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let num = |key: &str| match item.get(key) {
                Some(Value::Num(n)) => *n,
                _ => f64::NAN,
            };
            out.push((text.to_string(), family, num("x_pt"), num("font_size_pt")));
        }
    }
    out
}

fn messages(reply: &Value) -> Vec<String> {
    let diags = reply.get("payload").unwrap().get("diagnostics").unwrap();
    diags
        .as_arr()
        .unwrap()
        .iter()
        .map(|d| d.get("message").unwrap().as_str().unwrap().to_string())
        .collect()
}

#[test]
fn mathcal_emits_script_capitals_with_the_new_cm_hint() {
    // `\varnothing` is msbm "3F, declared by `amssymb.sty` and undefined in
    // base LaTeX2e; `\mathcal` is the kernel's and needs no package.
    let reply = compile("\\usepackage{amssymb}\n$\\mathcal{P}x$ and $\\varnothing x$\n");
    let items = items(&reply);
    let messages = messages(&reply);
    assert!(
        !messages.iter().any(|m| m.contains("not supported")),
        "{messages:#?}"
    );
    let cal = items
        .iter()
        .position(|(text, ..)| text == "\u{1D4AB}")
        .unwrap_or_else(|| panic!("\\mathcal{{P}} not emitted: {items:?}"));
    assert_eq!(items[cal].1, newcm_math::FAMILY);
    let (_, _, x_cal, size) = items[cal];
    let x_after = items[cal + 1].2;
    let expected = newcm_math::width_pt("\u{1D4AB}", size).unwrap();
    // Item positions are serialised to 0.01pt.
    assert!((x_after - x_cal - expected).abs() < 0.011, "{items:?}");

    let empty = items.iter().position(|(text, ..)| text == "∅").unwrap();
    let (_, _, x_empty, size) = items[empty];
    let advance = items[empty + 1].2 - x_empty;
    assert!((advance - 0.777781 * size).abs() < 0.011, "{items:?}");
    // Word-x oracle at 10pt, scaled to the compile size.
    assert!(
        (advance * 10.0 / size - PDFLATEX_VARNOTHING_WD_PT).abs() < 0.5,
        "{advance}"
    );
}

#[test]
fn mathcal_rejects_non_capitals() {
    let messages = messages(&compile("$\\mathcal{ab}$\n"));
    assert!(
        messages
            .iter()
            .any(|m| m.contains("\\mathcal supports only capital letters")),
        "{messages:#?}"
    );
}
