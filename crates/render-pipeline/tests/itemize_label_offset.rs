//! Itemize/enumerate label origins measured against pdflatex with Latin Modern.
//! The PDF coordinates were read with PyMuPDF from the fixture files using
//! get_texttrace() origins.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.1;

struct Expected {
    label: &'static str,
    text: &'static str,
    label_x: f64,
    text_x: f64,
}

const CASES: [[Expected; 4]; 3] = [
    [
        Expected { label: "•", text: "I10L1", label_x: 145.945007, text_x: 158.677200 },
        Expected { label: "–", text: "I10L2", label_x: 169.883011, text_x: 180.592804 },
        Expected { label: "∗", text: "I10L3", label_x: 189.260010, text_x: 199.222595 },
        Expected { label: "·", text: "I10L4", label_x: 203.429016, text_x: 216.161209 },
    ],
    [
        Expected { label: "•", text: "I11L1", label_x: 139.130997, text_x: 153.072815 },
        Expected { label: "–", text: "I11L2", label_x: 165.343994, text_x: 177.071274 },
        Expected { label: "∗", text: "I11L3", label_x: 186.561997, text_x: 197.471085 },
        Expected { label: "·", text: "I11L4", label_x: 202.076996, text_x: 216.018814 },
    ],
    [
        Expected { label: "•", text: "I12L1", label_x: 125.161003, text_x: 140.128922 },
        Expected { label: "–", text: "I12L2", label_x: 153.294998, text_x: 165.883820 },
        Expected { label: "∗", text: "I12L3", label_x: 176.057999, text_x: 187.774094 },
        Expected { label: "·", text: "I12L4", label_x: 192.705002, text_x: 207.672913 },
    ],
];

const ENUMERATE_CASES: [[Expected; 4]; 3] = [
    [
        Expected { label: "1.", text: "E10L1", label_x: 145.945007, text_x: 158.677200 },
        Expected { label: "(a)", text: "E10L2", label_x: 162.881012, text_x: 180.594513 },
        Expected { label: "i.", text: "E10L3", label_x: 188.707016, text_x: 199.227524 },
        Expected { label: "A.", text: "E10L4", label_x: 200.939011, text_x: 216.161865 },
    ],
    [
        Expected { label: "1.", text: "E11L1", label_x: 139.132004, text_x: 153.073822 },
        Expected { label: "(a)", text: "E11L2", label_x: 157.677002, text_x: 177.073364 },
        Expected { label: "i.", text: "E11L3", label_x: 185.955994, text_x: 197.475998 },
        Expected { label: "A.", text: "E11L4", label_x: 199.349991, text_x: 216.019089 },
    ],
    [
        Expected { label: "1.", text: "E12L1", label_x: 125.161003, text_x: 140.128922 },
        Expected { label: "(a)", text: "E12L2", label_x: 145.061996, text_x: 165.876007 },
        Expected { label: "i.", text: "E12L3", label_x: 175.405991, text_x: 187.767670 },
        Expected { label: "A.", text: "E12L4", label_x: 189.782990, text_x: 207.667969 },
    ],
];

fn word<'a>(words: &'a [Word], page: u32, text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == page && w.text == text)
        .unwrap_or_else(|| panic!("no page {page} word {text:?}: {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

fn check_cases(words: &[Word], page: u32, cases: &[Expected; 4]) {
    for case in cases {
        close(word(words, page, case.label).x, case.label_x, case.label);
        close(word(words, page, case.text).x, case.text_x, case.text);
    }
}

#[test]
fn list_labels_and_item_text_origins_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    for (index, size) in [10, 11, 12].into_iter().enumerate() {
        let source = std::fs::read_to_string(format!(
            "{}/tests/fixtures/itemize-label-offset/article-{size}.tex",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read list-origin fixture");
        let rendered = render_one(&source);
        assert_eq!(rendered.v2.pages.len(), 3, "article {size} page count");
        let words = words_of(&rendered);
        check_cases(&words, 1, &CASES[index]);
        check_cases(&words, 2, &ENUMERATE_CASES[index]);

        let label = word(&words, 3, "A");
        let body_text = format!("D{size}body");
        let body = word(&words, 3, &body_text);
        close(label.x, [133.768005, 125.797997, 110.853996][index], "description label");
        close(body.x, [225.503647, 226.216278, 218.462723][index], "description body");
    }
}
