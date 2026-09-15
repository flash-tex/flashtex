//! Item-text and label origins of enumitem lists against pdflatex, for the
//! horizontal keys `\enit@calcleft` resolves (enumitem.sty 874-893):
//! `labelsep`, `itemindent`, `widest`, `leftmargin=*` combined with them,
//! a `leftmargin` given as a `\settowidth`/`\setlength` register, and a
//! `labelsep` inherited by a nested `leftmargin=*` list (`\list` resets
//! `\itemindent`, not `\labelsep`). The first three rows are the nested
//! `enumerate` of `fixtures/real-world/enumitem-worksheet` at 12pt, whose
//! item text sat +0.62bp (level 1) and +1.17bp (level 2) right of
//! pdflatex's before #611 measured `\leftmargin<i>` in `lmr12`'s quad.
//!
//! Coordinates: pdflatex (TeX Live 2026) PDFs of the fixture files, read
//! with PyMuPDF `get_texttrace()` origins, in bp. No TeX runs here.
//!
//! The itemize rows (`K..j`) have no `lmodern`, so their bullet is TS1
//! `tcrm`'s 0.5em one, not `ts1-lmr`'s 0.7778em (`itemize_ts1_symbols`).

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.1;

struct Case {
    text: &'static str,
    label: Option<(&'static str, f64)>,
    text_x: f64,
}

const CASES: [[Case; 13]; 3] = [
    [
        Case { text: "K10a", label: Some(("1.", 84.177002)), text_x: 96.909203 },
        Case { text: "K10b", label: Some(("(a)", 101.113007)), text_x: 118.826508 },
        Case { text: "K10c", label: Some(("i.", 126.938004)), text_x: 137.458511 },
        Case { text: "K10d", label: Some(("1.", 79.195007)), text_x: 96.908508 },
        Case { text: "K10e", label: Some(("1.", 94.139008)), text_x: 106.871208 },
        Case { text: "K10f", label: Some(("i.", 77.535004)), text_x: 88.055504 },
        Case { text: "K10g", label: Some(("1.", 80.579002)), text_x: 93.311203 },
        Case { text: "K10h", label: Some(("1.", 72.000000)), text_x: 89.713501 },
        Case { text: "K10i", label: Some(("1.", 72.000000)), text_x: 84.732201 },
        Case { text: "K10j", label: Some(("•", 87.499001)), text_x: 97.461601 },
        Case { text: "K10k", label: Some(("1.", 72.000000)), text_x: 99.676102 },
        Case { text: "K10l", label: Some(("(a)", 102.995003)), text_x: 135.652405 },
        Case { text: "K10m", label: Some(("1.", 89.158005)), text_x: 101.890205 },
    ],
    [
        Case { text: "K11a", label: Some(("1.", 85.333000)), text_x: 99.274834 },
        Case { text: "K11b", label: Some(("(a)", 103.878998)), text_x: 123.275375 },
        Case { text: "K11c", label: Some(("i.", 132.157990)), text_x: 143.677994 },
        Case { text: "K11d", label: Some(("1.", 79.878990)), text_x: 99.275375 },
        Case { text: "K11e", label: Some(("1.", 96.242989)), text_x: 110.184822 },
        Case { text: "K11f", label: Some(("i.", 78.060989)), text_x: 89.581001 },
        Case { text: "K11g", label: Some(("1.", 81.393990)), text_x: 95.335823 },
        Case { text: "K11h", label: Some(("1.", 71.999992)), text_x: 91.396378 },
        Case { text: "K11i", label: Some(("1.", 71.999992)), text_x: 85.941826 },
        Case { text: "K11j", label: Some(("•", 89.000992)), text_x: 99.877365 },
        Case { text: "K11k", label: Some(("1.", 71.999992)), text_x: 102.305473 },
        Case { text: "K11l", label: Some(("(a)", 105.938995)), text_x: 141.699020 },
        Case { text: "K11m", label: Some(("1.", 90.787994)), text_x: 104.729828 },
    ],
    [
        Case { text: "K12a", label: Some(("1.", 86.306999)), text_x: 101.274910 },
        Case { text: "K12b", label: Some(("(a)", 106.207001)), text_x: 127.032959 },
        Case { text: "K12c", label: Some(("i.", 136.552002)), text_x: 148.913681 },
        Case { text: "K12d", label: Some(("1.", 80.454002)), text_x: 101.268005 },
        Case { text: "K12e", label: Some(("1.", 98.013000)), text_x: 112.980911 },
        Case { text: "K12f", label: Some(("i.", 78.502998)), text_x: 90.864677 },
        Case { text: "K12g", label: Some(("1.", 82.080002)), text_x: 97.047913 },
        Case { text: "K12h", label: Some(("1.", 72.000000)), text_x: 92.814003 },
        Case { text: "K12i", label: Some(("1.", 72.000000)), text_x: 86.967911 },
        Case { text: "K12j", label: Some(("•", 90.201996)), text_x: 101.918091 },
        Case { text: "K12k", label: Some(("1.", 72.000000)), text_x: 104.518143 },
        Case { text: "K12l", label: Some(("(a)", 108.418999)), text_x: 146.795197 },
        Case { text: "K12m", label: Some(("1.", 92.159996)), text_x: 107.127907 },
    ],
];

fn word<'a>(words: &'a [Word], text: &str, baseline: Option<f64>) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == 1 && w.text == text && baseline.is_none_or(|b| (w.baseline - b).abs() < 0.5))
        .unwrap_or_else(|| panic!("no page 1 word {text:?} at baseline {baseline:?}: {words:?}"))
}

#[test]
fn enumitem_item_text_and_label_origins_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let mut failures = Vec::new();
    for (index, size) in [10, 11, 12].into_iter().enumerate() {
        let source = std::fs::read_to_string(format!(
            "{}/tests/fixtures/enumitem-item-text/article-{size}.tex",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read enumitem item-text fixture");
        let rendered = render_one(&source);
        assert_eq!(rendered.v2.pages.len(), 1, "article {size} page count");
        let words = words_of(&rendered);
        for case in &CASES[index] {
            let text = word(&words, case.text, None);
            if (text.x - case.text_x).abs() >= TOL {
                failures.push(format!("{} text: {:.3} vs pdflatex {:.3}", case.text, text.x, case.text_x));
            }
            if let Some((label, x)) = case.label {
                let got = word(&words, label, Some(text.baseline)).x;
                if (got - x).abs() >= TOL {
                    failures.push(format!("{} label {label:?}: {got:.3} vs pdflatex {x:.3}", case.text));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
