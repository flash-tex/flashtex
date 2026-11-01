//! The port against ICU itself: `testdata/icu-vectors.txt` holds what ICU's
//! `ubidi_setPara` (paragraph levels `UBIDI_DEFAULT_LTR` and
//! `UBIDI_DEFAULT_RTL`), `ubidi_getDirection`, `ubidi_getLevels`,
//! `ubidi_countRuns` and `ubidi_getVisualRun` return for 1,052 texts: the
//! words and edge cases of its `HAND` list and 1,000 random strings over every bidi class
//! (explicit embeddings, overrides and isolates, brackets, numbers and
//! separators, paragraph separators, surrogate pairs). `icu_oracle.py`
//! wrote it through ctypes from macOS's system ICU, 78.1, whose bidi
//! algorithm and Unicode 17 data are those of 78.2, TeX Live 2026's.

use super::*;

/// `ubidi_getLevels` on a paragraph object.
fn levels(b: &UBiDi) -> Vec<u8> {
    b.get_levels()
}

fn hex_units(s: &str) -> Vec<u16> {
    (0..s.len() / 4)
        .map(|i| u16::from_str_radix(&s[4 * i..4 * i + 4], 16).unwrap())
        .collect()
}

#[test]
fn equals_icu_on_the_vectors() {
    let data = include_str!("testdata/icu-vectors.txt");
    let mut n = 0;
    let mut mixed = 0;
    let mut bad = Vec::new();
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 5, "{line}");
        let level = u8::from_str_radix(f[0], 16).unwrap();
        let text = hex_units(f[1]);
        let want_dir: i32 = f[2].parse().unwrap();
        let mut b = UBiDi::set_para(&text, level);
        let dir = b.direction() as i32;
        let got_levels: String = if text.is_empty() {
            String::new()
        } else {
            levels(&b).iter().map(|l| format!("{l:02x}")).collect()
        };
        let runs: Vec<String> = (0..b.count_runs())
            .map(|i| {
                let (d, s, l) = b.get_visual_run(i);
                format!("{}:{s}:{l}", d as i32)
            })
            .collect();
        let got = (dir, got_levels, runs.join(","));
        let want = (want_dir, f[3].to_string(), f[4].to_string());
        if got != want {
            bad.push(format!(
                "{level:02x} {:04x?}\n  ICU  {want:?}\n  port {got:?}",
                text
            ));
        }
        n += 1;
        mixed += (want_dir == 2) as usize;
    }
    assert!(n > 2000 && mixed > 1000, "{n} vectors, {mixed} mixed");
    assert!(
        bad.is_empty(),
        "{} of {n} differ from ICU:\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
}

fn u(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn visual_runs(b: &mut UBiDi) -> Vec<(Direction, i32, i32)> {
    (0..b.count_runs()).map(|i| b.get_visual_run(i)).collect()
}

/// The runs XeTeX lays out for the corpus's Hebrew: a word is one
/// right-to-left run; digits after it are a left-to-right run, visually
/// first (u03's "מספרים 123 ו").
#[test]
fn hebrew_words_and_numbers() {
    let t = u("שלום");
    let b = UBiDi::set_para(&t, UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Rtl);
    let b = UBiDi::set_para(&t, UBIDI_DEFAULT_LTR);
    assert_eq!(b.direction(), Direction::Rtl);

    let t = u("שלום123");
    let mut b = UBiDi::set_para(&t, UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Mixed);
    assert_eq!(levels(&b), [1, 1, 1, 1, 2, 2, 2]);
    assert_eq!(
        visual_runs(&mut b),
        [(Direction::Ltr, 4, 3), (Direction::Rtl, 0, 4)]
    );

    // Latin text stays one left-to-right run with a Hebrew font's default
    let t = u("office");
    let b = UBiDi::set_para(&t, UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Ltr);
    // digits alone in a right-to-left default paragraph: level 2, mixed
    let t = u("123");
    let mut b = UBiDi::set_para(&t, UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Mixed);
    assert_eq!(visual_runs(&mut b), [(Direction::Ltr, 0, 3)]);
    // an empty text has the default's direction
    let b = UBiDi::set_para(&[], UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Rtl);
}

/// Arabic letters (AL) make European digits Arabic numbers (W2); Arabic
/// digits (AN) after a word.
#[test]
fn arabic() {
    let t = u("مرحبا.");
    let b = UBiDi::set_para(&t, UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Rtl);
    let t = u("أرقام١٢٣٤");
    let mut b = UBiDi::set_para(&t, UBIDI_DEFAULT_RTL);
    assert_eq!(b.direction(), Direction::Mixed);
    assert_eq!(levels(&b), [1, 1, 1, 1, 1, 2, 2, 2, 2]);
    assert_eq!(
        visual_runs(&mut b),
        [(Direction::Ltr, 5, 4), (Direction::Rtl, 0, 5)]
    );
}

/// Brackets take the direction of their content or context (N0).
#[test]
fn brackets() {
    let t = u("abc (שלום) def");
    let mut b = UBiDi::set_para(&t, UBIDI_DEFAULT_LTR);
    assert_eq!(levels(&b), [0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0]);
    assert_eq!(
        visual_runs(&mut b),
        [
            (Direction::Ltr, 0, 5),
            (Direction::Rtl, 5, 4),
            (Direction::Ltr, 9, 5)
        ]
    );
}
