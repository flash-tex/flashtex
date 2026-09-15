//! Itemize label and item-text origins against pdflatex for the four
//! default `\labelitem` symbols under the three text-font setups:
//!
//! * `ot1`: no `fontenc`, no `lmodern` (LaTeX's defaults);
//! * `t1`: `\usepackage[T1]{fontenc}` without `lmodern`;
//! * `lm`: `\usepackage[T1]{fontenc}` + `\usepackage{lmodern}`.
//!
//! `\textbullet`, `\textasteriskcentered` and `\textperiodcentered` are TS1
//! symbols (latex.ltx's `\DeclareTextSymbolDefault{..}{TS1}`). Without
//! `lmodern` the TS1 font is `tcrm` (EC), whose glyphs are the `cmsy` designs:
//! bullet 0.5em, asterisk 0.5em, centred period 0.2777em (tcrm1000 widths
//! 0.499878/0.499878/0.27771; tcrm1095 and tcrm1200 are slightly narrower).
//! With `lmodern` they are `ts1-lmr`, whose bullet and centred period are
//! 0.7778em. `\labelitemii` (`\normalfont\bfseries\textendash`) is `cmbx`/
//! `ecbx`/`ec-lmbx` in the text encoding, 0.575em in all three.
//!
//! Page 1 nests four default itemize lists; page 2 nests four
//! `[leftmargin=*]` lists, whose margin is the widest label plus `\labelsep`
//! (`\enit@calcleft`), so a wrong symbol width moves the item text too.
//!
//! Coordinates: pdflatex (TeX Live 2026) PDFs of the fixture files, read with
//! PyMuPDF `get_texttrace()` origins, in bp. No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of};

const TOL: f64 = 0.1;

struct Case {
    page: u32,
    label: &'static str,
    text: &'static str,
    label_x: f64,
    text_x: f64,
}

const CASES: [(&str, u32, [Case; 8]); 9] = [
    ("ot1", 10, [
        Case { page: 1, label: "•", text: "PL1", label_x: 148.714005, text_x: 158.676590 },
        Case { page: 1, label: "–", text: "PL2", label_x: 169.883011, text_x: 180.592804 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 189.261017, text_x: 199.223602 },
        Case { page: 1, label: "·", text: "PL4", label_x: 208.411011, text_x: 216.161911 },
        Case { page: 2, label: "•", text: "SL1", label_x: 133.768005, text_x: 143.730591 },
        Case { page: 2, label: "–", text: "SL2", label_x: 143.730011, text_x: 154.439804 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 154.440018, text_x: 164.402603 },
        Case { page: 2, label: "·", text: "SL4", label_x: 164.401016, text_x: 172.151917 },
    ]),
    ("ot1", 11, [
        Case { page: 1, label: "•", text: "PL1", label_x: 142.192993, text_x: 153.069366 },
        Case { page: 1, label: "–", text: "PL2", label_x: 165.343994, text_x: 177.071274 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 186.592987, text_x: 197.469360 },
        Case { page: 1, label: "·", text: "PL4", label_x: 207.546982, text_x: 216.012436 },
        Case { page: 2, label: "•", text: "SL1", label_x: 125.797997, text_x: 136.674362 },
        Case { page: 2, label: "–", text: "SL2", label_x: 136.675995, text_x: 148.403275 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 148.403992, text_x: 159.280365 },
        Case { page: 2, label: "·", text: "SL4", label_x: 159.281998, text_x: 167.747452 },
    ]),
    ("ot1", 12, [
        Case { page: 1, label: "•", text: "PL1", label_x: 128.414993, text_x: 140.131088 },
        Case { page: 1, label: "–", text: "PL2", label_x: 153.294998, text_x: 165.883820 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 176.057999, text_x: 187.774094 },
        Case { page: 1, label: "·", text: "PL4", label_x: 198.558990, text_x: 207.668854 },
        Case { page: 2, label: "•", text: "SL1", label_x: 110.853996, text_x: 122.570091 },
        Case { page: 2, label: "–", text: "SL2", label_x: 122.558998, text_x: 135.147827 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 135.136993, text_x: 146.853088 },
        Case { page: 2, label: "·", text: "SL4", label_x: 146.840988, text_x: 155.950851 },
    ]),
    ("t1", 10, [
        Case { page: 1, label: "•", text: "PL1", label_x: 148.714005, text_x: 158.676590 },
        Case { page: 1, label: "–", text: "PL2", label_x: 169.884003, text_x: 180.593796 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 189.261002, text_x: 199.223587 },
        Case { page: 1, label: "·", text: "PL4", label_x: 208.410995, text_x: 216.161896 },
        Case { page: 2, label: "•", text: "SL1", label_x: 133.768005, text_x: 143.730591 },
        Case { page: 2, label: "–", text: "SL2", label_x: 143.730011, text_x: 154.439804 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 154.438019, text_x: 164.400604 },
        Case { page: 2, label: "·", text: "SL4", label_x: 164.400024, text_x: 172.150925 },
    ]),
    ("t1", 11, [
        Case { page: 1, label: "•", text: "PL1", label_x: 142.192993, text_x: 153.069366 },
        Case { page: 1, label: "–", text: "PL2", label_x: 165.384995, text_x: 177.068634 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 186.592987, text_x: 197.469360 },
        Case { page: 1, label: "·", text: "PL4", label_x: 207.546982, text_x: 216.012436 },
        Case { page: 2, label: "•", text: "SL1", label_x: 125.797997, text_x: 136.674362 },
        Case { page: 2, label: "–", text: "SL2", label_x: 136.675995, text_x: 148.359634 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 148.362000, text_x: 159.238373 },
        Case { page: 2, label: "·", text: "SL4", label_x: 159.240997, text_x: 167.706451 },
    ]),
    ("t1", 12, [
        Case { page: 1, label: "•", text: "PL1", label_x: 128.414993, text_x: 140.131088 },
        Case { page: 1, label: "–", text: "PL2", label_x: 153.295990, text_x: 165.872864 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 176.057983, text_x: 187.774078 },
        Case { page: 1, label: "·", text: "PL4", label_x: 198.558990, text_x: 207.668854 },
        Case { page: 2, label: "•", text: "SL1", label_x: 110.853996, text_x: 122.570091 },
        Case { page: 2, label: "–", text: "SL2", label_x: 122.558998, text_x: 135.135864 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 135.134995, text_x: 146.851089 },
        Case { page: 2, label: "·", text: "SL4", label_x: 146.839996, text_x: 155.949860 },
    ]),
    ("lm", 10, [
        Case { page: 1, label: "•", text: "PL1", label_x: 145.945007, text_x: 158.677200 },
        Case { page: 1, label: "–", text: "PL2", label_x: 169.883011, text_x: 180.592804 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 189.260010, text_x: 199.222595 },
        Case { page: 1, label: "·", text: "PL4", label_x: 203.429016, text_x: 216.161209 },
        Case { page: 2, label: "•", text: "SL1", label_x: 133.768005, text_x: 146.500198 },
        Case { page: 2, label: "–", text: "SL2", label_x: 146.499008, text_x: 157.208801 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 157.208008, text_x: 167.170593 },
        Case { page: 2, label: "·", text: "SL4", label_x: 167.171005, text_x: 179.903198 },
    ]),
    ("lm", 11, [
        Case { page: 1, label: "•", text: "PL1", label_x: 139.130997, text_x: 153.072815 },
        Case { page: 1, label: "–", text: "PL2", label_x: 165.343994, text_x: 177.071274 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 186.561996, text_x: 197.471085 },
        Case { page: 1, label: "·", text: "PL4", label_x: 202.076996, text_x: 216.018814 },
        Case { page: 2, label: "•", text: "SL1", label_x: 125.797997, text_x: 139.739822 },
        Case { page: 2, label: "–", text: "SL2", label_x: 139.737991, text_x: 151.465271 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 151.464996, text_x: 162.374084 },
        Case { page: 2, label: "·", text: "SL4", label_x: 162.373993, text_x: 176.315811 },
    ]),
    ("lm", 12, [
        Case { page: 1, label: "•", text: "PL1", label_x: 125.161003, text_x: 140.128922 },
        Case { page: 1, label: "–", text: "PL2", label_x: 153.294998, text_x: 165.883820 },
        Case { page: 1, label: "∗", text: "PL3", label_x: 176.057999, text_x: 187.774094 },
        Case { page: 1, label: "·", text: "PL4", label_x: 192.705002, text_x: 207.672913 },
        Case { page: 2, label: "•", text: "SL1", label_x: 110.853996, text_x: 125.821907 },
        Case { page: 2, label: "–", text: "SL2", label_x: 125.811996, text_x: 138.400818 },
        Case { page: 2, label: "∗", text: "SL3", label_x: 138.389999, text_x: 150.106094 },
        Case { page: 2, label: "·", text: "SL4", label_x: 150.093994, text_x: 165.061905 },
    ]),
];


#[test]
fn ts1_itemize_symbols_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let mut failures = Vec::new();
    for (setup, size, cases) in &CASES {
        let source = std::fs::read_to_string(format!(
            "{}/tests/fixtures/itemize-ts1-symbols/{setup}-{size}.tex",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read TS1 itemize fixture");
        let rendered = render_one(&source);
        assert_eq!(rendered.v2.pages.len(), 2, "{setup}-{size} page count");
        let words = words_of(&rendered);
        for case in cases {
            let Some(text) = words.iter().find(|w| w.page == case.page && w.text == case.text) else {
                failures.push(format!("{setup}-{size} {}: no item text", case.text));
                continue;
            };
            if (text.x - case.text_x).abs() >= TOL {
                failures.push(format!("{setup}-{size} {} text: {:.3} vs pdflatex {:.3}", case.text, text.x, case.text_x));
            }
            // The label must sit on the item's baseline.
            let label = words
                .iter()
                .filter(|w| w.page == case.page && w.text == case.label && (w.baseline - text.baseline).abs() < 0.01)
                .max_by(|a, b| a.x.total_cmp(&b.x));
            match label {
                Some(label) if (label.x - case.label_x).abs() >= TOL => failures.push(format!(
                    "{setup}-{size} {} label {}: {:.3} vs pdflatex {:.3}",
                    case.text, case.label, label.x, case.label_x
                )),
                Some(_) => {}
                None => failures.push(format!("{setup}-{size} {}: no label {:?}", case.text, case.label)),
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
