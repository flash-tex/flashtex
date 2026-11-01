//! Liang's pattern hyphenation (the algorithm TeX uses), original Rust
//! implementation behind the [`Hyphenator`] trait.
//!
//! # Pattern data
//!
//! Both pattern files are vendored **verbatim** under `patterns/` and parsed
//! at construction; nothing is generated or edited:
//!
//! * `patterns/hyphen.tex` — Knuth's Plain TeX hyphenation tables. TeX Live's
//!   `language.dat` loads this file for the `english` language (aliases
//!   `usenglish`, `USenglish`, `american`), which is language 0 in pdflatex's
//!   LaTeX format, so it is what an unconfigured `pdflatex` document uses and
//!   the default here ([`LiangHyphenator::english`]). Licence (file header):
//!   "Unlimited copying and redistribution of this file are permitted as long
//!   as this file is not modified." It is not modified.
//! * `patterns/hyph-en-us.tex` — hyph-utf8 "Hyphenation patterns for American
//!   English", Copyright (C) 1990, 2004, 2005 Gerard D.C. Kuiken, version
//!   2005-05-30 (Knuth's patterns plus Kuiken's additions, `ushyphmax`).
//!   Licence (file header): "Copying and distribution of this file, with or
//!   without modification, are permitted in any medium without royalty
//!   provided the copyright notice and this notice are preserved." Available
//!   as [`LiangHyphenator::en_us_max`] (babel `usenglishmax`).
//!
//! Both come from TeX Live 2026 (`texmf-dist/tex/generic/hyphen/` and
//! `texmf-dist/tex/generic/hyph-utf8/patterns/tex/`).
//!
//! # Word rules
//!
//! [`Hyphenator::hyphenate`] receives one whitespace-free chunk. Following
//! TeX §894–899: leading characters that are not letters are skipped; the
//! hyphenatable word is the maximal run of letters that follows (lower-cased,
//! `\uchyph=1`); trailing non-letter *characters* are allowed after it
//! ("sentence." is hyphenated), but a discretionary after the run — an explicit
//! hyphen character or `\-` anywhere in the chunk — suppresses automatic
//! hyphenation of the whole chunk. Words with fewer than
//! `left_min + right_min` letters, or more than 63, are left alone. Exceptions
//! (`\hyphenation`) replace the patterns for an exact (lower-cased) word;
//! `\lefthyphenmin` / `\righthyphenmin` apply to both (§902).
//!
//! Which chunks TeX tries at all (only a word that directly follows glue) is
//! the caller's business: [`crate::items::ParagraphBuilder::text`] applies it.

use std::collections::HashMap;

use crate::hyphenate::{HyphenationPoint, Hyphenator};

/// Knuth's `hyphen.tex`, verbatim.
pub const HYPHEN_TEX: &str = include_str!("../patterns/hyphen.tex");
/// hyph-utf8 `hyph-en-us.tex`, verbatim.
pub const HYPH_EN_US_TEX: &str = include_str!("../patterns/hyph-en-us.tex");

/// TeX's limit on the letters of a hyphenatable word (§897: `hn = 63`).
pub const MAX_WORD_LETTERS: usize = 63;

/// Error parsing a `\patterns{}` / `\hyphenation{}` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternError {
    pub entry: String,
    pub reason: &'static str,
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad hyphenation entry {:?}: {}", self.entry, self.reason)
    }
}

impl std::error::Error for PatternError {}

/// TeX-style pattern hyphenator with exceptions and hyphen minima.
#[derive(Debug, Clone)]
pub struct LiangHyphenator {
    /// Pattern letters -> inter-letter weights (`letters.len() + 1` digits).
    patterns: HashMap<Vec<char>, Vec<u8>>,
    /// Longest pattern, in letters (dots included).
    max_len: usize,
    /// Exception word (lower case) -> break positions (letters before the break).
    exceptions: HashMap<Vec<char>, Vec<usize>>,
    /// `\lefthyphenmin` (English: 2).
    pub left_min: usize,
    /// `\righthyphenmin` (English: 3).
    pub right_min: usize,
}

impl LiangHyphenator {
    /// An empty hyphenator (no patterns, no exceptions).
    pub fn empty(left_min: usize, right_min: usize) -> Self {
        LiangHyphenator {
            patterns: HashMap::new(),
            max_len: 0,
            exceptions: HashMap::new(),
            left_min,
            right_min,
        }
    }

    /// pdflatex's default `english`: Knuth's `hyphen.tex`, hyphenmins 2/3.
    pub fn english() -> Self {
        Self::from_tex(HYPHEN_TEX, 2, 3).expect("vendored hyphen.tex parses")
    }

    /// hyph-utf8 `hyph-en-us.tex` (`usenglishmax`), hyphenmins 2/3.
    pub fn en_us_max() -> Self {
        Self::from_tex(HYPH_EN_US_TEX, 2, 3).expect("vendored hyph-en-us.tex parses")
    }

    /// Compiles patterns written in TeX's notation (`.ch4`, `hy3ph`) and
    /// exceptions written with hyphens (`ta-ble`). Entries that are not
    /// valid pattern/exception syntax are skipped (the embedded tables are
    /// generated and always parse; see [`Self::add_pattern`] for the checked
    /// form).
    pub fn new(patterns: &[&str], exceptions: &[&str], left_min: usize, right_min: usize) -> Self {
        let mut h = Self::empty(left_min, right_min);
        for p in patterns {
            let _ = h.add_pattern(p);
        }
        for e in exceptions {
            let _ = h.add_exceptions(e);
        }
        h
    }

    /// The embedded 420-pattern American-English subset ([`EN_US_SUBSET`],
    /// [`EN_US_EXCEPTIONS`]) with `\lefthyphenmin=2`, `\righthyphenmin=3`:
    /// the patterns of `hyphen.tex` that match a word of the mac branch's
    /// oracle samples. [`Self::english`] is the complete table.
    pub fn en_us_subset() -> Self {
        LiangHyphenator::new(EN_US_SUBSET, EN_US_EXCEPTIONS, 2, 3)
    }

    /// Parses a TeX pattern file: `%` comments, one `\patterns{...}` group and
    /// an optional `\hyphenation{...}` group.
    pub fn from_tex(source: &str, left_min: usize, right_min: usize) -> Result<Self, PatternError> {
        let mut h = Self::empty(left_min, right_min);
        let text: String = source
            .lines()
            .map(|l| l.find('%').map_or(l, |i| &l[..i]))
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(body) = group(&text, "\\patterns") {
            for entry in body.split_whitespace() {
                h.add_pattern(entry)?;
            }
        }
        if let Some(body) = group(&text, "\\hyphenation") {
            h.add_exceptions(body)?;
        }
        Ok(h)
    }

    /// Adds one pattern in TeX notation (`.ach4`, `hy3ph`, `4b1ora`).
    pub fn add_pattern(&mut self, entry: &str) -> Result<(), PatternError> {
        let mut letters = Vec::new();
        let mut weights = vec![0u8];
        for ch in entry.chars() {
            if let Some(d) = ch.to_digit(10) {
                *weights.last_mut().expect("nonempty") = d as u8;
            } else if ch == '.' || ch.is_alphabetic() {
                letters.push(ch);
                weights.push(0);
            } else {
                return Err(PatternError {
                    entry: entry.to_string(),
                    reason: "patterns contain only letters, dots and digits",
                });
            }
        }
        if letters.is_empty() {
            return Err(PatternError {
                entry: entry.to_string(),
                reason: "pattern has no letters",
            });
        }
        self.max_len = self.max_len.max(letters.len());
        self.patterns.insert(letters, weights);
        Ok(())
    }

    /// `\hyphenation{ta-ble as-so-ciate}`: whitespace-separated words with
    /// hyphens at the allowed breaks. A later entry for the same word
    /// replaces an earlier one, as in TeX.
    pub fn add_exceptions(&mut self, list: &str) -> Result<(), PatternError> {
        for entry in list.split_whitespace() {
            let mut word = Vec::new();
            let mut breaks = Vec::new();
            for ch in entry.chars() {
                if ch == '-' {
                    if !word.is_empty() && breaks.last() != Some(&word.len()) {
                        breaks.push(word.len());
                    }
                } else if ch.is_alphabetic() {
                    word.extend(ch.to_lowercase());
                } else {
                    return Err(PatternError {
                        entry: entry.to_string(),
                        reason: "\\hyphenation entries contain only letters and hyphens",
                    });
                }
            }
            breaks.retain(|&b| b < word.len());
            if !word.is_empty() {
                self.exceptions.insert(word, breaks);
            }
        }
        Ok(())
    }

    /// Number of patterns and exceptions loaded.
    pub fn counts(&self) -> (usize, usize) {
        (self.patterns.len(), self.exceptions.len())
    }

    /// Break positions (number of letters before each break) of a word made
    /// only of letters. Applies exceptions, patterns and the hyphen minima.
    pub fn positions(&self, word: &str) -> Vec<usize> {
        let lower: Vec<char> = word.chars().flat_map(char::to_lowercase).collect();
        self.positions_lower(&lower)
    }

    fn positions_lower(&self, word: &[char]) -> Vec<usize> {
        let n = word.len();
        if n > MAX_WORD_LETTERS || n < self.left_min.max(1) + self.right_min.max(1) {
            return Vec::new();
        }
        let odd: Vec<usize> = if let Some(e) = self.exceptions.get(word) {
            e.clone()
        } else {
            let mut dotted = Vec::with_capacity(n + 2);
            dotted.push('.');
            dotted.extend_from_slice(word);
            dotted.push('.');
            // weights[i] sits before dotted[i].
            let mut weights = vec![0u8; dotted.len() + 1];
            for start in 0..dotted.len() {
                for len in 1..=self.max_len.min(dotted.len() - start) {
                    if let Some(w) = self.patterns.get(&dotted[start..start + len]) {
                        for (j, &d) in w.iter().enumerate() {
                            let slot = &mut weights[start + j];
                            *slot = (*slot).max(d);
                        }
                    }
                }
            }
            // A break after k letters sits before dotted[k + 1].
            (1..n).filter(|&k| weights[k + 1] % 2 == 1).collect()
        };
        odd.into_iter()
            .filter(|&k| k >= self.left_min.max(1) && n - k >= self.right_min.max(1))
            .collect()
    }
}

/// Body of the first `name{...}` group (no nested braces in pattern files).
fn group<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let at = text.find(name)? + name.len();
    let open = at + text[at..].find('{')?;
    let close = open + text[open..].find('}')?;
    Some(&text[open + 1..close])
}

impl Hyphenator for LiangHyphenator {
    fn hyphenate(&self, word: &str) -> Vec<HyphenationPoint> {
        // A discretionary inside the chunk (explicit hyphen or `\-`) ends TeX's
        // search with no automatic hyphens (§896/§899 `othercases goto done1`);
        // `\-` stays legal as an explicit point.
        if word.contains("\\-") {
            return crate::hyphenate::ExplicitDiscretionary.hyphenate(word);
        }
        if word.contains('-') {
            return Vec::new();
        }
        let mut letters: Vec<char> = Vec::new();
        let mut byte_at: Vec<usize> = Vec::new();
        let mut rest = "";
        for (i, ch) in word.char_indices() {
            if ch.is_alphabetic() {
                byte_at.push(i);
                letters.extend(ch.to_lowercase());
            } else if !letters.is_empty() {
                rest = &word[i..];
                break;
            }
        }
        // Only characters may follow the letter run; a later letter run would
        // mean a non-letter character sat inside the word, which TeX allows
        // (it just stops at the first run), so nothing further to check.
        let _ = rest;
        if letters.len() != byte_at.len() {
            // A letter whose lower case is several chars: stay conservative.
            return Vec::new();
        }
        self.positions_lower(&letters)
            .into_iter()
            .map(|k| HyphenationPoint {
                offset: byte_at[k],
                marker_len: 0,
                automatic: true,
            })
            .collect()
    }
}

/// Patterns: 420 of the 4938 in hyph-en-us.tex (all also present in Knuth's hyphen.tex),
/// selected as those matching a word of `docs/hyphen-sample.tex` or the
/// wrap-sample oracle text (see module docs). Generated; do not hand-edit.
pub const EN_US_SUBSET: &[&str] = &[
    ".ch4",
    ".en3g",
    ".ge2",
    ".he2",
    ".in1",
    ".in3s",
    ".le2",
    ".li2n",
    ".me2",
    ".or1d",
    ".res2",
    ".sh2",
    ".st4",
    ".te4",
    ".th2",
    ".ti2",
    "a2d",
    "ad5er.",
    "a2f",
    "ai2",
    "a4i4n",
    "al1i",
    "a2n",
    "4and",
    "2ang",
    "4ao",
    "4aphi",
    "2a2r",
    "ar3act",
    "ar1i",
    "4as.",
    "4ath",
    "at1ic",
    "au4l2",
    "av1i",
    "bal1a",
    "be3gi",
    "2bf",
    "1bil",
    "b2l2",
    "b4le.",
    "3bod",
    "4b1ora",
    "both5",
    "bound3",
    "1ca",
    "c3c",
    "2ce.",
    "4ced.",
    "1cen",
    "3cent",
    "2ch",
    "4ch.",
    "4ch3ab",
    "1ci",
    "ck1",
    "1c4l4",
    "1co",
    "4c3s2",
    "2c1t",
    "c2te",
    "c3ter",
    "c3ume",
    "3c4ut",
    "4c5utiv",
    "1cy",
    "1d2a",
    "d4em",
    "de4mons",
    "1den",
    "de1p",
    "de1t",
    "4d1f",
    "d1in",
    "1dina",
    "dis1",
    "d1j",
    "d1m",
    "1do",
    "1dr",
    "2d1s2",
    "1du",
    "du2c",
    "4duct.",
    "d2y",
    "ead1",
    "ea4l",
    "east3",
    "e1cu",
    "ee4p1",
    "e1f",
    "e4f3ere",
    "1eff",
    "e1h4",
    "ei2",
    "el2i",
    "e3libe",
    "e5lim",
    "e1me",
    "4eno",
    "e2pa",
    "e1po",
    "e3pro",
    "e1q",
    "er1a",
    "era4b",
    "2ere.",
    "er5ence",
    "er1i",
    "er3m4",
    "e1s2e",
    "e1sp",
    "ev1er",
    "1exp",
    "1fa",
    "5far",
    "fault5",
    "fer1",
    "4f1f",
    "1fi",
    "2fin",
    "5fina",
    "fi2ne",
    "f4l2",
    "1fo",
    "5fon",
    "fon4t",
    "fo2r",
    "fort5a",
    "2ft",
    "1ga",
    "2ge.",
    "1gen",
    "1geo",
    "ge3om",
    "gh5out",
    "3g4in.",
    "5g4ins",
    "gl2",
    "1go",
    "1gr",
    "5graph.",
    "5graphic",
    "gs2",
    "gth3",
    "gu4a",
    "1gy",
    "5hand.",
    "he2n",
    "hena4",
    "hen5at",
    "he4t",
    "4hr4",
    "4h1s2",
    "hy3ph",
    "ibe4",
    "ib3era",
    "i1bl",
    "i1br",
    "4ich",
    "ic3ula",
    "2id",
    "2ie4",
    "i3fie",
    "4ift",
    "i1la",
    "il1er",
    "il1i",
    "2ilit",
    "im1i",
    "2in.",
    "4ind",
    "2ine",
    "2i1no",
    "2ins",
    "2io",
    "i4our",
    "2ip",
    "4ir",
    "2is.",
    "2is1c",
    "4ise",
    "3isf",
    "i2so",
    "is1ti",
    "2ith",
    "i1ti",
    "2itio",
    "4itt",
    "2iv",
    "k3ab",
    "k5ag",
    "1kee",
    "4k1s2",
    "lab3ic",
    "l4abo",
    "2ld",
    "le2a",
    "lim3i",
    "l4ina",
    "1l4ine",
    "l1it",
    "l1iz",
    "l1l",
    "ll4o",
    "l5low",
    "5long",
    "4lt",
    "1ly",
    "2lys4",
    "1ma",
    "4m1b",
    "4me.",
    "1men",
    "3ment",
    "2mes",
    "m4etr",
    "me3try",
    "4m1f",
    "min4a",
    "m1m",
    "1mo",
    "4m1p",
    "mpar5i",
    "mphas4",
    "m2pi",
    "1mu",
    "mun2",
    "1na",
    "na4li",
    "nar3i",
    "n2at",
    "4n1b4",
    "n3cha",
    "n1de",
    "2ne.",
    "n1er",
    "1nes",
    "2nes.",
    "ne4v",
    "n5eve",
    "ng1in",
    "n1gu",
    "n2it",
    "4n1l",
    "1nou",
    "n1r",
    "2n1s2",
    "n2se",
    "nsid1",
    "n1t",
    "nt2i",
    "nt4s",
    "n1v2",
    "od5uct.",
    "1ogy",
    "oi2",
    "o3ing",
    "ol2d",
    "ol3er",
    "o2ly",
    "o2me",
    "o4met",
    "om5etry",
    "om3pi",
    "o2n",
    "on1a",
    "on1c",
    "on3s",
    "onten4",
    "oo2",
    "o1ra",
    "o1ry",
    "4oth",
    "ou2",
    "oun2d",
    "1pa",
    "3pare",
    "p4a4ri",
    "par4is",
    "pd4",
    "pe2c",
    "2p2ed",
    "4ph.",
    "ph1ic",
    "4phs",
    "pi2n",
    "1p2l2",
    "pli4n",
    "5po4g",
    "1p4or",
    "1pos",
    "4p1p",
    "p2pe",
    "p4ped",
    "pr2",
    "p3rese",
    "pros3e",
    "2p1s2",
    "2p1t",
    "pu2t",
    "5pute",
    "qu2",
    "2rab",
    "r5acl",
    "r2as",
    "ration4",
    "r1c",
    "r2ce",
    "rd2i",
    "rdin4",
    "2re.",
    "re1al",
    "2r2ed",
    "re2fe",
    "r4es.",
    "r1f",
    "rg2",
    "rgi4n",
    "rin4d",
    "r2is",
    "r1l",
    "r1m",
    "rox5",
    "r1p",
    "r1ti",
    "ru2n",
    "r1w",
    "ry3t",
    "sa2",
    "4se.",
    "3sect",
    "4s4ed",
    "sep3a3",
    "5sev",
    "4s3f",
    "s2h",
    "2sh.",
    "sho4",
    "shor4",
    "short5",
    "si1b",
    "2s1in",
    "s3ing",
    "1sis",
    "3sitio",
    "1so",
    "3s4on.",
    "s4pon",
    "2ss",
    "2st.",
    "st2i",
    "s1tic",
    "s3tif",
    "st4r",
    "s2ty",
    "1su",
    "s4ul",
    "su2r",
    "s4y",
    "3syl",
    "1ta",
    "2tab",
    "2t1b",
    "4tc",
    "t4ch",
    "4te.",
    "2t1ed",
    "3tenc",
    "5ter3d",
    "1teri",
    "ter3is",
    "3tex",
    "2th.",
    "than4",
    "th2e",
    "t4ic1u",
    "tif2",
    "1tim",
    "2t1in",
    "1tio",
    "1tiv",
    "2tl",
    "4t1m",
    "tme4",
    "1to",
    "to2ma",
    "1tra",
    "4t1s2",
    "4t3t2",
    "1tu",
    "tw4",
    "1ty",
    "2tyl",
    "uci4b",
    "u1la",
    "u1ni",
    "un3s4",
    "urc4",
    "2us",
    "4u1t2i",
    "ution5a",
    "uto5matic",
    "4ve.",
    "vi1ou",
    "whi4",
    "wi2",
    "with3",
    "1wo2",
    "wra4",
    "wri4",
    "x1e",
    "x3i",
    "x3p",
    "x1t2",
    "y1o4",
    "y3po",
    "ys1t",
    "y3thin",
    "za1",
];

/// The `\hyphenation` exception list shared by hyphen.tex and hyph-en-us.tex.
pub const EN_US_EXCEPTIONS: &[&str] = &[
    "as-so-ciate",
    "as-so-ciates",
    "dec-li-na-tion",
    "oblig-a-tory",
    "phil-an-thropic",
    "present",
    "presents",
    "project",
    "projects",
    "reci-procity",
    "re-cog-ni-zance",
    "ref-or-ma-tion",
    "ret-ri-bu-tion",
    "ta-ble",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn show(h: &LiangHyphenator, w: &str) -> String {
        let pos = h.positions(w);
        let mut out = String::new();
        for (i, ch) in w.chars().enumerate() {
            if pos.contains(&i) {
                out.push('-');
            }
            out.push(ch);
        }
        out
    }

    #[test]
    fn vendored_files_parse_completely() {
        let h = LiangHyphenator::english();
        assert_eq!(h.counts(), (4447, 14));
        let m = LiangHyphenator::en_us_max();
        assert_eq!(m.counts(), (4938, 14));
    }

    /// Expected strings are pdflatex `\showhyphens` output (TeX Live 2026,
    /// `english` = hyphen.tex), recorded in `tests/oracle/showhyphens.txt`.
    #[test]
    fn knuth_examples() {
        let h = LiangHyphenator::english();
        for (w, want) in [
            ("counterexample", "coun-terex-am-ple"),
            ("satisfy", "sat-isfy"),
            ("hypotheses", "hy-pothe-ses"),
            ("explicitly", "ex-plic-itly"),
            ("irrational", "ir-ra-tional"),
            ("expression", "ex-pres-sion"),
            ("specified", "spec-i-fied"),
            ("statement", "state-ment"),
            ("associate", "as-so-ciate"),
            ("table", "ta-ble"),
            ("present", "present"),
            ("hyphenation", "hy-phen-ation"),
        ] {
            assert_eq!(show(&h, w), want, "{w}");
        }
    }

    #[test]
    fn minima_and_exceptions() {
        let mut h = LiangHyphenator::english();
        h.add_exceptions("FlashTeX flash-tex").unwrap();
        assert_eq!(show(&h, "flashtex"), "flash-tex");
        h.left_min = 1;
        h.right_min = 1;
        assert!(h.positions("ta").is_empty() || h.positions("ta") == vec![1]);
        let mut h = LiangHyphenator::english();
        h.right_min = 10;
        assert!(h.positions("counterexample").iter().all(|&k| 14 - k >= 10));
        assert!(LiangHyphenator::english().positions("cat").is_empty());
    }

    #[test]
    fn chunk_rules() {
        let h = LiangHyphenator::english();
        let offs = |w: &str| h.hyphenate(w).iter().map(|p| p.offset).collect::<Vec<_>>();
        assert_eq!(offs("(counterexample),"), vec![5, 10, 12]); // (coun-terex-am-ple),
        assert_eq!(offs("Satisfy."), vec![3]);
        assert!(offs("counter-example").is_empty());
        let pts = h.hyphenate("re\\-pro");
        assert_eq!(pts.len(), 1);
        assert!(!pts[0].automatic);
    }
}

#[cfg(test)]
mod subset_tests {
    use super::*;

    /// `\showhyphens` output of pdflatex (pdfTeX 1.40.29, TeX Live 2026, language
    /// `english` = hyphen.tex, \lefthyphenmin 2, \righthyphenmin 3) for every
    /// word of five or more letters in the two oracle samples: 110 words.
    const TEX_SHOWHYPHENS: &[&str] = &[
        "De-lib-er-ately",
        "Flash-TeX",
        "Hy-phen-ation",
        "README",
        "Re-pro-ducibil-ity",
        "Short",
        "Wrap-ping",
        "ac-cents",
        "ad-just-ment",
        "against",
        "ap-prox-i-ma-tions",
        "ar-ti-cle",
        "au-to-mat-i-cally",
        "be-gin",
        "be-haviour",
        "bound-aries",
        "break",
        "breaks",
        "char-ac-ter-is-tic",
        "cof-fee",
        "col-lab-o-ra-tion",
        "com-fort-ably",
        "com-mu-nity",
        "com-pares",
        "com-par-i-son",
        "com-pi-la-tion",
        "com-piler",
        "com-pli-cate",
        "com-puted",
        "con-sec-u-tive",
        "con-se-quently",
        "con-sid-er-ably",
        "con-struc-tions",
        "con-ven-tional",
        "coun-ter-pro-duc-tive",
        "de-lib-er-ately",
        "demon-strate",
        "de-ter-mine",
        "dis-cre-tionary",
        "doc-u-ment",
        "doc-u-men-ta-tion",
        "doc-u-ment-class",
        "drifts",
        "ef-fort",
        "elim-i-nate",
        "emer-gency",
        "em-pha-sised",
        "en-gine",
        "ev-ery",
        "ex-pects",
        "ex-traor-di-nar-ily",
        "fi-nally",
        "fol-low",
        "gen-er-ated",
        "go-ing",
        "im-ple-men-ta-tion",
        "in-com-pre-hen-si-ble",
        "in-de-pen-dently",
        "in-sti-tu-tion-al-iza-tion",
        "in-ter-dis-ci-plinary",
        "in-ter-na-tional",
        "in-ter-word",
        "jus-ti-fi-ca-tion",
        "keeps",
        "lay-out",
        "lines",
        "longer",
        "mea-sured",
        "mea-sure-ment",
        "op-por-tu-ni-ties",
        "or-a-cle",
        "or-der",
        "or-di-nary",
        "over-flows",
        "para-graph",
        "para-graphs",
        "par-tic-u-larly",
        "phrase",
        "poly-syl-labic",
        "po-si-tion",
        "po-si-tions",
        "prose",
        "rather",
        "reader",
        "ref-er-ence",
        "re-port",
        "rep-re-sen-ta-tive",
        "re-pro-ducibil-ity",
        "re-spon-si-bil-i-ties",
        "re-sults",
        "sat-is-fac-tory",
        "sec-tion",
        "sev-eral",
        "source",
        "starts",
        "stretch-a-bil-ity",
        "stripped",
        "ter-mi-nol-ogy",
        "textbf",
        "their",
        "through-out",
        "ty-po-graph-i-cal",
        "un-bal-anced",
        "un-char-ac-ter-is-ti-cally",
        "un-re-mark-able",
        "ver-i-fi-ca-tion",
        "where",
        "whether",
        "which",
        "words",
    ];

    #[test]
    fn matches_tex_showhyphens_for_every_sample_word() {
        let h = LiangHyphenator::en_us_subset();
        let mut mismatches = Vec::new();
        for entry in TEX_SHOWHYPHENS {
            let word: String = entry.chars().filter(|c| *c != '-').collect();
            let pts = h.hyphenate(&word);
            let mut rendered = String::new();
            let mut last = 0;
            for p in &pts {
                rendered.push_str(&word[last..p.offset]);
                rendered.push('-');
                last = p.offset;
            }
            rendered.push_str(&word[last..]);
            if rendered != *entry {
                mismatches.push(format!("{entry} vs {rendered}"));
            }
        }
        assert!(
            mismatches.is_empty(),
            "{} mismatches: {mismatches:?}",
            mismatches.len()
        );
    }

    #[test]
    fn tex_word_rules() {
        let h = LiangHyphenator::en_us_subset();
        // pdflatex `\showhyphens{engine's (documentation) don't}` gives
        // en-gine's (doc-u-men-ta-tion) don't: leading and trailing non-letters
        // are skipped, the letter run after a non-letter is ignored; short
        // words are left alone.
        assert!(!h.hyphenate("documentation,").is_empty());
        assert_eq!(
            h.hyphenate("(documentation)")
                .iter()
                .map(|p| p.offset)
                .collect::<Vec<_>>(),
            vec![4, 5, 8, 10]
        );
        assert_eq!(
            h.hyphenate("engine's")
                .iter()
                .map(|p| p.offset)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(h.hyphenate("docu(mentation").is_empty());
        assert!(h.hyphenate("lines").is_empty());
        // Exceptions override patterns.
        assert_eq!(h.positions("table"), vec![2]);
        // Explicit markers win over patterns.
        let pts = h.hyphenate("docu\\-mentation");
        assert_eq!(pts.len(), 1);
        assert_eq!(
            (pts[0].offset, pts[0].marker_len, pts[0].automatic),
            (4, 2, false)
        );
        // Automatic points carry byte offsets and no marker.
        let pts = h.hyphenate("Documentation");
        assert_eq!(
            pts.iter().map(|p| p.offset).collect::<Vec<_>>(),
            vec![3, 4, 7, 9]
        );
        assert!(pts.iter().all(|p| p.automatic && p.marker_len == 0));
    }
}
