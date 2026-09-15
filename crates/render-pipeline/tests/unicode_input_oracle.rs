//! Literal UTF-8 input against pdfLaTeX (TeX Live 2026, pdfTeX 1.40.29).
//!
//! 1. `fixtures/unicode-input/reference.json` (made by `oracle.py` there,
//!    test-only; cargo never runs TeX) holds, for all 349 characters the
//!    `*.dfu` files and `utf8.def` declare plus eight undeclared ones, the
//!    LaTeX errors `\setbox0\hbox{<char>}` raises and the box width under
//!    OT1/T1 with and without `lmodern`. `crate::inputenc::rejected` must
//!    give exactly those errors, and keep a letter exactly when pdfLaTeX's
//!    box is not empty.
//! 2. Whole documents: the errors a document reports and where every glyph
//!    lands (glyph origins read from pdfLaTeX's PDF with PyMuPDF; pdfLaTeX's
//!    separate OT1 accent glyphs are left out, the base letters kept).

mod common;

use common::*;
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display::{Item, RunRole};
use flashtex_render_pipeline::inputenc::rejected;
use flashtex_tex_text_encoding::encoding::Encoding;

#[test]
fn every_declared_character_raises_pdflatex_errors() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/unicode-input/reference.json");
    let reference = json::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let Some(Value::Obj(setups)) = reference.get("setups") else { panic!("no setups") };
    let mut failures = Vec::new();
    let mut checked = 0;
    for (setup, chars) in setups {
        let enc = if setup.starts_with("t1") { Encoding::T1 } else { Encoding::OT1 };
        let Value::Obj(chars) = chars else { panic!("{setup}: not an object") };
        for (hex, entry) in chars {
            let c = char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap();
            let errors: Vec<&str> = entry.get("errors").and_then(Value::as_arr).unwrap().iter().filter_map(Value::as_str).collect();
            let width = entry.get("width_sp").and_then(Value::as_i64).unwrap_or(0);
            let got = rejected(c, enc);
            let got_errors: Vec<&str> = got.iter().map(|r| r.message.as_str()).collect();
            if got_errors != errors {
                failures.push(format!("{setup} U+{hex} {c}: pdflatex {errors:?}, ours {got_errors:?}"));
            }
            if let Some(r) = &got {
                if r.keep.is_some() != (width > 0) {
                    failures.push(format!("{setup} U+{hex} {c}: pdflatex box {width}sp, ours keeps {:?}", r.keep));
                }
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 4 * 357);
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n\\noindent {body}\n\\end{{document}}\n")
}

/// Text glyph origins (bp) on page 1, in order, without the page number.
fn glyph_xs(r: &flashtex_render_pipeline::Rendered, n: usize) -> Vec<f64> {
    let mut xs = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role == RunRole::Text {
                xs.extend(run.glyphs.iter().map(|g| g.origin_x.to_bp()));
            }
        }
    }
    xs.truncate(n);
    xs
}

fn errors(r: &flashtex_render_pipeline::Rendered) -> Vec<String> {
    r.v2.diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "unicode_not_set_up" | "command_unavailable_in_encoding"))
        .map(|d| d.message.clone())
        .collect()
}

fn assert_positions(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}: {} glyphs, pdflatex {}: {got:?}", got.len(), want.len());
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!((g - w).abs() < 0.05, "{what}: glyph {i} at {g:.3} bp, pdflatex {w:.3} bp\n ours {got:?}\n pdflatex {want:?}");
    }
}

/// OT1 builds `ü`, `ä`, `é`, `ó`, `ź` with `\accent` and `Ł` as a box: no
/// font kern reaches them (`W`–`u` is -0.83 pt in `Wu`, 0 in `Wü`; `a`–`v`
/// -0.28 pt in `av`, 0 after `ä`). `Ø`, `ß` are OT1 slots and do kern.
#[test]
fn ot1_accented_letters_take_no_font_kerns() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let r = render_one(&doc("", "Wü Täve Vé Øy ßt Łódź"));
    // W u T a v e V e Ø y ß t L o d z
    let want = [
        133.768, 144.01, 152.866, 160.059, 165.041, 170.032, 177.773, 185.245, 192.986, 200.737, 209.324, 214.306, 221.499,
        227.725, 232.707, 238.246,
    ];
    assert_positions("OT1 accents", &glyph_xs(&r, want.len()), &want);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
}

/// Greek typed in text is an error per character, and nothing is typeset
/// for it: both spaces around `α` stay (`angle␣␣and`).
#[test]
fn undeclared_characters_are_errors_and_typeset_nothing() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let r = render_one(&doc("\\usepackage[T1]{fontenc}", "The angle α and β sum to π."));
    assert_eq!(
        errors(&r),
        [
            "LaTeX Error: Unicode character α (U+03B1) not set up for use with LaTeX.",
            "LaTeX Error: Unicode character β (U+03B2) not set up for use with LaTeX.",
            "LaTeX Error: Unicode character π (U+03C0) not set up for use with LaTeX.",
        ]
    );
    let want = [
        133.768, 140.961, 146.49, 154.231, 159.212, 164.742, 169.723, 172.493, 183.561, 188.542, 194.072, 206.236, 210.171,
        215.7, 227.317, 231.192, 239.491,
    ];
    assert_positions("Greek", &glyph_xs(&r, want.len()), &want);
    assert!(!r.v2.diagnostics.iter().any(|d| d.code == "missing_glyph"), "{:?}", r.v2.diagnostics);
}

/// A combining accent after its base is undeclared too: `e` + U+0301 sets
/// `e`, not a composed `é`.
#[test]
fn combining_marks_are_not_composed() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let r = render_one(&doc("\\usepackage[T1]{fontenc}", "Cafe\u{301} x"));
    assert_eq!(errors(&r), ["LaTeX Error: Unicode character \u{301} (U+0301) not set up for use with LaTeX."]);
    assert_positions("combining", &glyph_xs(&r, 5), &[133.768, 140.961, 145.942, 148.991, 156.732]);
}

/// OT1 has no guillemets, thorn or ogonek: `«`/`þ` typeset nothing, `ą`
/// typesets its `a`.
#[test]
fn ot1_unavailable_commands_are_errors() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let r = render_one(&doc("", "a «b» c þ d ą e"));
    assert_eq!(
        errors(&r),
        [
            "LaTeX Error: Command \\guillemetleft unavailable in encoding OT1.",
            "LaTeX Error: Command \\guillemetright unavailable in encoding OT1.",
            "LaTeX Error: Command \\th unavailable in encoding OT1.",
            "LaTeX Error: Command \\k unavailable in encoding OT1.",
        ]
    );
    assert_positions("OT1 unavailable", &glyph_xs(&r, 6), &[133.768, 142.067, 150.934, 161.992, 170.859, 179.158]);
    // The same characters are T1 slots.
    let t1 = render_one(&doc("\\usepackage[T1]{fontenc}", "a «b» c þ d ą e"));
    assert!(errors(&t1).is_empty(), "{:?}", errors(&t1));
}

/// No error is invented for a document that declares the character itself
/// or leaves pdfLaTeX's `utf8` input.
#[test]
fn declared_or_other_input_setups_are_left_alone() {
    let own = render_one(&doc("\\DeclareUnicodeCharacter{03B1}{a}", "x α y"));
    assert!(errors(&own).is_empty(), "{:?}", errors(&own));
    let latin1 = render_one(&doc("\\usepackage[latin1]{inputenc}", "x α y"));
    assert!(errors(&latin1).is_empty(), "{:?}", errors(&latin1));
    let lgr = render_one(&doc("\\usepackage[LGR,T1]{fontenc}", "x α y"));
    assert!(errors(&lgr).is_empty(), "{:?}", errors(&lgr));
}

/// The package, class and babel-language tables of `crate::inputenc` are
/// exactly what `fixtures/unicode-input/packages.py` measured with
/// pdflatex (`packages.json`): a name is listed only when it loads, declares
/// no character outside the probe, and changes no error except to remove
/// the listed ones under OT1 (or to give exactly T1's errors). Everything
/// else is absent, so the check fails open for it.
#[test]
fn package_tables_are_pdflatex_measurements() {
    use flashtex_render_pipeline::inputenc::*;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/unicode-input/packages.json");
    let reference = json::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let Some(Value::Obj(entries)) = reference.get("entries") else { panic!("no entries") };
    let chars = |entry: &Value, enc: &str| -> String {
        let hexes = entry.get(enc).and_then(|e| e.get("accepts")).and_then(Value::as_arr).unwrap();
        hexes.iter().map(|h| char::from_u32(u32::from_str_radix(h.as_str().unwrap(), 16).unwrap()).unwrap()).collect()
    };
    let mut failures = Vec::new();
    let mut seen = Vec::new();
    for (key, entry) in entries {
        let effect = entry.get("effect").and_then(Value::as_str).unwrap();
        let (neutral, t1, accepting): (&[&str], &[&str], &[(&str, &str)]) = if let Some(_) = key.strip_prefix("class:") {
            (NEUTRAL_CLASSES, T1_CLASSES, ACCEPTING_CLASSES)
        } else if key.starts_with("babel:") {
            (&[], &[], &[])
        } else {
            (NEUTRAL_PACKAGES, T1_PACKAGES, ACCEPTING_PACKAGES)
        };
        let name = key.split_once(':').map_or(key.as_str(), |(_, n)| n);
        if matches!(name, "inputenc" | "fontenc") {
            continue; // read by their options, not the table
        }
        let ours = if let Some(language) = key.strip_prefix("babel:") {
            BABEL_LANGUAGES.contains(&language).then(|| format!("accepts {BABEL_ACCEPTS}"))
        } else if neutral.contains(&name) {
            Some("neutral".to_string())
        } else if t1.contains(&name) {
            Some("t1".to_string())
        } else {
            accepting.iter().find(|(n, _)| *n == name).map(|(_, c)| format!("accepts {c}"))
        };
        let measured = match effect {
            "fail-open" => None,
            "neutral" => Some("neutral".to_string()),
            "t1" => Some("t1".to_string()),
            "accepts" => {
                assert_eq!(chars(entry, "t1"), "", "{key}: accepts under T1");
                Some(format!("accepts {}", chars(entry, "ot1")))
            }
            other => panic!("{key}: effect {other}"),
        };
        if ours != measured {
            failures.push(format!("{key}: pdflatex {measured:?}, ours {ours:?}"));
        }
        seen.push(key.clone());
    }
    let listed = NEUTRAL_PACKAGES
        .iter()
        .chain(T1_PACKAGES)
        .chain(ACCEPTING_PACKAGES.iter().map(|(n, _)| n))
        .map(|n| n.to_string())
        .chain(NEUTRAL_CLASSES.iter().chain(T1_CLASSES).chain(ACCEPTING_CLASSES.iter().map(|(n, _)| n)).map(|n| format!("class:{n}")))
        .chain(BABEL_LANGUAGES.iter().map(|n| format!("babel:{n}")));
    for name in listed {
        if !seen.contains(&name) {
            failures.push(format!("{name}: listed but not measured"));
        }
    }
    let fontenc = reference.get("fontenc").unwrap();
    for (options, want) in [("T1,OT1", "ot1"), ("OT1,T1", "t1"), ("TS1,T1", "t1"), ("TS1,OT1", "ot1")] {
        assert_eq!(fontenc.get(options).and_then(|o| o.get("equals")).and_then(Value::as_str), Some(want), "fontenc [{options}]");
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}

/// Review #545, finding 1. pdflatex compiles each of these cleanly: the
/// character is declared in an `\input` file, or in a local package, or in a
/// package the project does not carry (which may declare anything).
#[test]
fn declarations_outside_the_entry_count() {
    let main = |load: &str| {
        format!("\\documentclass{{article}}\n\\usepackage[utf8]{{inputenc}}\n{load}\n\\begin{{document}}\nSnowman word: ☃ end.\n\\end{{document}}\n")
    };
    let input = main("\\input{macros}");
    let r = render_docs(&[("main.tex", &input), ("macros.tex", "\\DeclareUnicodeCharacter{2603}{X}\n")], "main.tex");
    assert!(errors(&r).is_empty(), "\\input: {:?}", errors(&r));
    let sty = main("\\usepackage{mystyle}");
    let r = render_docs(&[("main.tex", &sty), ("mystyle.sty", "\\ProvidesPackage{mystyle}\n\\DeclareUnicodeCharacter{2603}{X}\n")], "main.tex");
    assert!(errors(&r).is_empty(), "local .sty: {:?}", errors(&r));
    let r = render_one(&sty);
    assert!(errors(&r).is_empty(), "unresolved .sty: {:?}", errors(&r));
    // The control: an undeclared character in an `\input` file is still
    // pdflatex's error.
    let r = render_docs(
        &[("main.tex", "\\documentclass{article}\n\\begin{document}\n\\input{chapter}\n\\end{document}\n"), ("chapter.tex", "Greek α here.\n")],
        "main.tex",
    );
    assert_eq!(errors(&r), ["LaTeX Error: Unicode character α (U+03B1) not set up for use with LaTeX."]);
}

/// Review #545, finding 2. Under OT1, babel (`babel.def`) provides `«`,
/// `»`: `[french]` compiles cleanly in pdflatex (with only french's
/// "OT1 encoding should not be used" warning). It does not provide `þ`:
/// `[english]` still errors on it, and on nothing else.
#[test]
fn babel_provides_guillemets_under_ot1() {
    let french = render_one(&doc("\\usepackage[utf8]{inputenc}\n\\usepackage[french]{babel}", "Il a dit «bonjour» et c'est tout."));
    assert!(errors(&french).is_empty(), "{:?}", errors(&french));
    let english = render_one(&doc("\\usepackage[english]{babel}", "a «b» c þ d"));
    assert_eq!(errors(&english), ["LaTeX Error: Command \\th unavailable in encoding OT1."]);
}

/// Review #545, finding 3. A `\verb` body's characters stay active, so
/// pdflatex raises the same input errors there.
#[test]
fn verb_bodies_are_input() {
    let r = render_one(&doc("", "a \\verb|x漢y| b \\verb|«| c"));
    assert_eq!(
        errors(&r),
        [
            "LaTeX Error: Unicode character 漢 (U+6F22) not set up for use with LaTeX.",
            "LaTeX Error: Command \\guillemetleft unavailable in encoding OT1.",
        ]
    );
}
