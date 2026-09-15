//! Item-text and label origins of the remaining enumitem horizontal keys
//! against pdflatex, resolved by `\enit@calcleft` (enumitem.sty 874-893):
//! `labelindent=` (alone: absorbed into `\labelindent`, a no-op; with
//! `leftmargin=*`: kept, moving the item text), `labelwidth=` (alone: the
//! box stays glued, a no-op for right-aligned labels; with `align=left`:
//! the label sits at the box's left edge), `left=<len>` (single:
//! `leftmargin` auto) and `left=<a>..<b>` (`labelsep` auto), `labelsep*=`
//! (plus the
//! `\itemindent` in force, tracked across later `itemindent=` keys),
//! `labelindent*=` (plus the `\leftmargin` in force), `widest*=<n>` (the
//! counter formatted as `<n>` inside the label), `align=left` label
//! placement, and a nested `labelsep*=` inherited without re-tracking.
//! The `labelsep*=`-under-`leftmargin=` row also covers `\@item`'s text push
//! under a negative `\labelwidth` (latex.ltx 16030-16308).
//!
//! Coordinates: pdflatex (TeX Live 2026) PDFs of the fixture files, read
//! with PyMuPDF `get_texttrace()` origins, in bp. No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.1;

struct Case {
    text: &'static str,
    label: Option<(&'static str, f64)>,
    text_x: f64,
}

const CASES: [[Case; 15]; 3] = [
    [
        Case { text: "K10a", label: Some(("1.", 84.177002)), text_x: 96.909203 },
        Case { text: "K10b", label: Some(("1.", 86.173004)), text_x: 98.905205 },
        Case { text: "K10c", label: Some(("1.", 84.177002)), text_x: 96.909203 },
        Case { text: "K10d", label: Some(("1.", 35.232002)), text_x: 96.910461 },
        Case { text: "K10e", label: Some(("1.", 100.346001)), text_x: 113.078201 },
        Case { text: "K10f", label: Some(("1.", 100.346001)), text_x: 128.699554 },
        Case { text: "K10g", label: Some(("1.", 60.811001)), text_x: 96.905495 },
        Case { text: "K10h", label: Some(("1.", 60.811001)), text_x: 111.082275 },
        Case { text: "K10i", label: Some(("1.", 84.177002)), text_x: 96.909203 },
        Case { text: "K10j", label: Some(("1.", 111.080002)), text_x: 123.812202 },
        Case { text: "K10k", label: Some(("1.", 76.981003)), text_x: 89.713203 },
        Case { text: "K10l", label: Some(("i)", 77.258003)), text_x: 88.884354 },
        Case { text: "K10m", label: Some(("1.", 72.000000)), text_x: 96.906494 },
        Case { text: "K10nO", label: Some(("1.", 60.811001)), text_x: 96.905495 },
        Case { text: "K10n", label: Some(("(a)", 98.350006)), text_x: 139.425812 },
    ],
    [
        Case { text: "K11a", label: Some(("1.", 85.333000)), text_x: 99.274834 },
        Case { text: "K11b", label: Some(("1.", 86.172997)), text_x: 100.114830 },
        Case { text: "K11c", label: Some(("1.", 85.333000)), text_x: 99.274834 },
        Case { text: "K11d", label: Some(("1.", 37.125000)), text_x: 99.274139 },
        Case { text: "K11e", label: Some(("1.", 100.346001)), text_x: 114.287834 },
        Case { text: "K11f", label: Some(("1.", 100.346001)), text_x: 128.698761 },
        Case { text: "K11g", label: Some(("1.", 62.441002)), text_x: 99.270126 },
        Case { text: "K11h", label: Some(("1.", 62.441002)), text_x: 113.451958 },
        Case { text: "K11i", label: Some(("1.", 85.333000)), text_x: 99.274834 },
        Case { text: "K11j", label: Some(("1.", 113.445999)), text_x: 127.387833 },
        Case { text: "K11k", label: Some(("1.", 77.455002)), text_x: 91.396835 },
        Case { text: "K11l", label: Some(("i)", 77.758003)), text_x: 90.488922 },
        Case { text: "K11m", label: Some(("1.", 72.000000)), text_x: 99.272751 },
        Case { text: "K11nO", label: Some(("1.", 62.441002)), text_x: 99.270126 },
        Case { text: "K11n", label: Some(("(a)", 99.507004)), text_x: 141.790665 },
    ],
    [
        Case { text: "K12a", label: Some(("1.", 86.306999)), text_x: 101.274910 },
        Case { text: "K12b", label: Some(("1.", 86.172997)), text_x: 101.140907 },
        Case { text: "K12c", label: Some(("1.", 86.306999)), text_x: 101.274910 },
        Case { text: "K12d", label: Some(("1.", 38.718998)), text_x: 101.268608 },
        Case { text: "K12e", label: Some(("1.", 100.345993)), text_x: 115.313904 },
        Case { text: "K12f", label: Some(("1.", 100.345993)), text_x: 128.691772 },
        Case { text: "K12g", label: Some(("1.", 63.813992)), text_x: 101.269638 },
        Case { text: "K12h", label: Some(("1.", 63.813992)), text_x: 115.448509 },
        Case { text: "K12i", label: Some(("1.", 86.306992)), text_x: 101.274902 },
        Case { text: "K12j", label: Some(("1.", 115.437988)), text_x: 130.405899 },
        Case { text: "K12k", label: Some(("1.", 77.852989)), text_x: 92.820900 },
        Case { text: "K12l", label: Some(("i)", 78.177986)), text_x: 91.842781 },
        Case { text: "K12m", label: Some(("1.", 71.999985)), text_x: 101.266312 },
        Case { text: "K12nO", label: Some(("1.", 63.813984)), text_x: 101.269630 },
        Case { text: "K12n", label: Some(("(a)", 100.479980)), text_x: 143.793671 },
    ],
];

fn word<'a>(words: &'a [Word], text: &str, baseline: Option<f64>) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == 1 && w.text == text && baseline.is_none_or(|b| (w.baseline - b).abs() < 0.5))
        .unwrap_or_else(|| panic!("no page 1 word {text:?} at baseline {baseline:?}"))
}

#[test]
fn enumitem_keys_rest_origins_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let mut failures = Vec::new();
    for (index, size) in [10, 11, 12].into_iter().enumerate() {
        let source = std::fs::read_to_string(format!(
            "{}/tests/fixtures/enumitem-keys-rest/article-{size}.tex",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read enumitem keys-rest fixture");
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
