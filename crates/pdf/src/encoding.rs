//! Character encodings for the two base-14 fonts this writer uses.
//!
//! - `WinAnsiEncoding` (PDF 32000-1:2008, Annex D.2) for Times-Roman: ASCII,
//!   Latin-1, and the Windows-1252 punctuation block.
//! - The built-in encoding of the `Symbol` font (Annex D.5) for Greek letters
//!   and mathematical operators the FT-002 compiler emits as Unicode text.
//!
//! Both are placeholders until FlashTeX embeds its own fonts. A character is
//! looked up in WinAnsi first, then Symbol, then (when the caller opted in to
//! embedding, see `crate::embed`) an embedded TrueType subset; anything covered
//! by none of them is substituted with [`SUBSTITUTE`] and reported, never
//! silently dropped.

/// The byte written for a character no font here can represent.
pub const SUBSTITUTE: u8 = b'?';

/// Which base-14 font a byte belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// Times-Roman with WinAnsiEncoding, resource `/F1`.
    Times,
    /// Symbol with its built-in encoding, resource `/F2`.
    Symbol,
    /// The document's embedded font (Identity-H, two bytes per glyph),
    /// resource `/F3`. Only present when embedding was requested.
    Embedded,
    /// An additional face selected by a `font-hints-v1` hint: either another
    /// base-14 Times variant (WinAnsi, one byte per glyph) or another embedded
    /// face (two bytes per glyph). Resource `/F4`, `/F5`, … in allocation order;
    /// the writer knows which kind each slot is.
    Extra(u8),
}

impl Font {
    pub fn resource_name(self) -> String {
        match self {
            Font::Times => "F1".into(),
            Font::Symbol => "F2".into(),
            Font::Embedded => "F3".into(),
            Font::Extra(n) => format!("F{}", 4 + n as usize),
        }
    }

    /// Base-14 name; the embedded fonts' names are per document.
    pub fn base_font(self) -> &'static str {
        match self {
            Font::Times => "Times-Roman",
            Font::Symbol => "Symbol",
            Font::Embedded | Font::Extra(_) => "(per document)",
        }
    }
}

/// Maps a character to its WinAnsi byte, or `None` if it has no code.
pub fn winansi_byte(c: char) -> Option<u8> {
    let cp = c as u32;
    match cp {
        0x20..=0x7E => Some(cp as u8),
        // 0xA0..=0xFF coincide with Latin-1. 0xA0 renders as a space and 0xAD as
        // a hyphen in WinAnsi, which is what those code points mean anyway.
        0xA0..=0xFF => Some(cp as u8),
        _ => match c {
            '€' => Some(0x80),
            '‚' => Some(0x82),
            'ƒ' => Some(0x83),
            '„' => Some(0x84),
            '…' => Some(0x85),
            '†' => Some(0x86),
            '‡' => Some(0x87),
            'ˆ' => Some(0x88),
            '‰' => Some(0x89),
            'Š' => Some(0x8A),
            '‹' => Some(0x8B),
            'Œ' => Some(0x8C),
            'Ž' => Some(0x8E),
            '\u{2018}' => Some(0x91),
            '\u{2019}' => Some(0x92),
            '\u{201C}' => Some(0x93),
            '\u{201D}' => Some(0x94),
            '•' => Some(0x95),
            '–' => Some(0x96),
            '—' => Some(0x97),
            '˜' => Some(0x98),
            '™' => Some(0x99),
            'š' => Some(0x9A),
            '›' => Some(0x9B),
            'œ' => Some(0x9C),
            'ž' => Some(0x9E),
            'Ÿ' => Some(0x9F),
            _ => None,
        },
    }
}

/// Maps a character to its code in the Symbol font's built-in encoding.
///
/// Codes follow Adobe's Symbol encoding as tabulated in PDF 32000-1 Annex D.5
/// (glyph names in comments). Characters that WinAnsi already covers (`± × ÷
/// ° ¬ · …`) are listed here too so a caller that prefers Symbol for a math
/// run can still find them, but [`encode`] tries WinAnsi first. Only codes
/// whose glyph name and position are certain are included; nothing here is a
/// guess, and a doubtful code is omitted rather than risk the wrong glyph.
pub fn symbol_byte(c: char) -> Option<u8> {
    Some(match c {
        // Greek lowercase (Unicode U+03B1..U+03C9, Symbol a..z positions).
        'α' => 0x61, // alpha
        'β' => 0x62, // beta
        'χ' => 0x63, // chi
        'δ' => 0x64, // delta
        'ε' => 0x65, // epsilon
        'φ' => 0x66, // phi
        'γ' => 0x67, // gamma
        'η' => 0x68, // eta
        'ι' => 0x69, // iota
        'ϕ' => 0x6A, // phi1
        'κ' => 0x6B, // kappa
        'λ' => 0x6C, // lambda
        'μ' => 0x6D, // mu
        'ν' => 0x6E, // nu
        'ο' => 0x6F, // omicron
        'π' => 0x70, // pi
        'θ' => 0x71, // theta
        'ρ' => 0x72, // rho
        'σ' => 0x73, // sigma
        'τ' => 0x74, // tau
        'υ' => 0x75, // upsilon
        'ϖ' => 0x76, // omega1
        'ω' => 0x77, // omega
        'ξ' => 0x78, // xi
        'ψ' => 0x79, // psi
        'ζ' => 0x7A, // zeta
        'ς' => 0x56, // sigma1 (final sigma)
        'ϑ' => 0x4A, // theta1
        'ϒ' => 0xA1, // Upsilon1
        // Greek uppercase.
        'Α' => 0x41, // Alpha
        'Β' => 0x42, // Beta
        'Χ' => 0x43, // Chi
        'Δ' => 0x44, // Delta
        'Ε' => 0x45, // Epsilon
        'Φ' => 0x46, // Phi
        'Γ' => 0x47, // Gamma
        'Η' => 0x48, // Eta
        'Ι' => 0x49, // Iota
        'Κ' => 0x4B, // Kappa
        'Λ' => 0x4C, // Lambda
        'Μ' => 0x4D, // Mu
        'Ν' => 0x4E, // Nu
        'Ο' => 0x4F, // Omicron
        'Π' => 0x50, // Pi
        'Θ' => 0x51, // Theta
        'Ρ' => 0x52, // Rho
        'Σ' => 0x53, // Sigma
        'Τ' => 0x54, // Tau
        'Υ' => 0x55, // Upsilon
        'Ω' => 0x57, // Omega
        'Ξ' => 0x58, // Xi
        'Ψ' => 0x59, // Psi
        'Ζ' => 0x5A, // Zeta
        // Operators and relations.
        '∀' => 0x22, // universal
        '∃' => 0x24, // existential
        '∋' => 0x27, // suchthat
        '∗' => 0x2A, // asteriskmath
        '−' => 0x2D, // minus
        '≅' => 0x40, // congruent
        '∴' => 0x5C, // therefore
        '⊥' => 0x5E, // perpendicular
        '∣' => 0x7C, // bar
        '∼' => 0x7E, // similar
        '′' => 0xA2, // minute
        '≤' => 0xA3, // lessequal
        '⁄' => 0xA4, // fraction
        '∞' => 0xA5, // infinity
        '↔' => 0xAB, // arrowboth
        '←' => 0xAC, // arrowleft
        '↑' => 0xAD, // arrowup
        '→' => 0xAE, // arrowright
        '↓' => 0xAF, // arrowdown
        '°' => 0xB0, // degree
        '±' => 0xB1, // plusminus
        '″' => 0xB2, // second
        '≥' => 0xB3, // greaterequal
        '×' => 0xB4, // multiply
        '∝' => 0xB5, // proportional
        '∂' => 0xB6, // partialdiff
        '•' => 0xB7, // bullet
        '÷' => 0xB8, // divide
        '≠' => 0xB9, // notequal
        '≡' => 0xBA, // equivalence
        '≈' => 0xBB, // approxequal
        '…' => 0xBC, // ellipsis
        'ℵ' => 0xC0, // aleph
        'ℑ' => 0xC1, // Ifraktur
        'ℜ' => 0xC2, // Rfraktur
        '℘' => 0xC3, // weierstrass
        '⊗' => 0xC4, // circlemultiply
        '⊕' => 0xC5, // circleplus
        '∅' => 0xC6, // emptyset
        '∩' => 0xC7, // intersection
        '∪' => 0xC8, // union
        '⊃' => 0xC9, // propersuperset
        '⊇' => 0xCA, // reflexsuperset
        '⊄' => 0xCB, // notsubset
        '⊂' => 0xCC, // propersubset
        '⊆' => 0xCD, // reflexsubset
        '∈' => 0xCE, // element
        '∉' => 0xCF, // notelement
        '∠' => 0xD0, // angle
        '∇' => 0xD1, // gradient
        '∏' => 0xD5, // product
        '√' => 0xD6, // radical
        '·' => 0xD7, // dotmath
        '¬' => 0xD8, // logicalnot
        '∧' => 0xD9, // logicaland
        '∨' => 0xDA, // logicalor
        '⇔' => 0xDB, // arrowdblboth
        '⇐' => 0xDC, // arrowdblleft
        '⇑' => 0xDD, // arrowdblup
        '⇒' => 0xDE, // arrowdblright
        '⇓' => 0xDF, // arrowdbldown
        '◊' => 0xE0, // lozenge
        '⟨' | '\u{2329}' | '\u{3008}' => 0xE1, // angleleft
        '∑' => 0xE5, // summation
        '⟩' | '\u{232A}' | '\u{3009}' => 0xF1, // angleright
        '∫' => 0xF2, // integral
        _ => return None,
    })
}

/// A maximal run of consecutive characters that share one font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub font: Font,
    pub bytes: Vec<u8>,
}

/// Result of encoding one text item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// Font runs in text order. Substituted characters land in a Times run.
    pub runs: Vec<Run>,
    /// Characters that were replaced by [`SUBSTITUTE`], in order of first
    /// appearance, without duplicates.
    pub unrepresentable: Vec<char>,
}

impl Encoded {
    /// All bytes across runs, in order, ignoring font boundaries.
    pub fn bytes(&self) -> Vec<u8> {
        self.runs
            .iter()
            .flat_map(|r| r.bytes.iter().copied())
            .collect()
    }
}

/// Encodes a string into font runs: WinAnsi/Times where possible, Symbol for
/// what Times lacks, `?` in Times for everything else.
pub fn encode(text: &str) -> Encoded {
    encode_with(text, None)
}

/// True when neither base-14 font can show the character.
pub fn needs_embedding(c: char) -> bool {
    winansi_byte(c).is_none() && symbol_byte(c).is_none()
}

/// Which font is tried first for ordinary text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Face {
    /// Base-14 Times-Roman (WinAnsi) first, then Symbol, then the embedded
    /// font: the embedded font only fills gaps. For `\usepackage{times}`
    /// style documents and for when nothing is embedded.
    #[default]
    Times,
    /// The embedded font first, then Symbol for operators it lacks, then
    /// Times only for characters neither covers. Requires an embedded font.
    Embedded,
}

/// Like [`encode`], but with an embedded font available. `embedded` returns
/// the two-byte glyph id to write in an [`Font::Embedded`] run for a character
/// it covers. With [`Face::Times`] it is consulted only for characters outside
/// both base-14 fonts; with [`Face::Embedded`] it is consulted first. A
/// character no font covers still means `?` + report.
pub fn encode_with(text: &str, embedded: Option<&dyn Fn(char) -> Option<u16>>) -> Encoded {
    encode_face(text, embedded, Face::Times)
}

/// [`encode_with`] with an explicit primary face.
pub fn encode_face(
    text: &str,
    embedded: Option<&dyn Fn(char) -> Option<u16>>,
    face: Face,
) -> Encoded {
    let primary = match face {
        Face::Times => Font::Times,
        Face::Embedded => Font::Embedded,
    };
    encode_runs(
        text,
        &Faces {
            face,
            primary,
            primary_lookup: embedded,
            fallback_lookup: embedded,
        },
    )
}

/// The fonts available to one text item.
pub struct Faces<'a> {
    /// Lookup order (see [`Face`]).
    pub face: Face,
    /// The primary font's resource: a Times variant (WinAnsi bytes) or
    /// [`Font::Symbol`] (its built-in encoding, tried before Times) when
    /// `face` is [`Face::Times`], an embedded face (glyph ids) when
    /// [`Face::Embedded`].
    pub primary: Font,
    /// Glyph lookup for the primary font when it is embedded.
    pub primary_lookup: Option<&'a dyn Fn(char) -> Option<u16>>,
    /// Glyph lookup for the document's embedded font (`/F3`), used as the
    /// last resort when `face` is [`Face::Times`].
    pub fallback_lookup: Option<&'a dyn Fn(char) -> Option<u16>>,
}

/// Splits `text` into font runs for the given faces.
pub fn encode_runs(text: &str, faces: &Faces<'_>) -> Encoded {
    let mut runs: Vec<Run> = Vec::new();
    let mut unrepresentable = Vec::new();
    for c in text.chars() {
        let symbol_run = || symbol_byte(c).map(|b| (Font::Symbol, vec![b]));
        let found = match faces.face {
            Face::Times => {
                let embedded_run = || {
                    faces
                        .fallback_lookup
                        .and_then(|f| f(c))
                        .map(|gid| (Font::Embedded, gid.to_be_bytes().to_vec()))
                };
                if faces.primary == Font::Symbol {
                    // Symbol-hinted items: characters both fonts carry
                    // (× ÷ ± ·) must come from Symbol, whose metrics layout used.
                    let times_run = || winansi_byte(c).map(|b| (Font::Times, vec![b]));
                    symbol_run().or_else(times_run).or_else(embedded_run)
                } else {
                    let primary_run = || winansi_byte(c).map(|b| (faces.primary, vec![b]));
                    primary_run().or_else(symbol_run).or_else(embedded_run)
                }
            }
            Face::Embedded => {
                let primary_run = || {
                    faces
                        .primary_lookup
                        .and_then(|f| f(c))
                        .map(|gid| (faces.primary, gid.to_be_bytes().to_vec()))
                };
                let times_run = || winansi_byte(c).map(|b| (Font::Times, vec![b]));
                primary_run().or_else(symbol_run).or_else(times_run)
            }
        };
        let (font, bytes): (Font, Vec<u8>) = match found {
            Some(run) => run,
            None => {
                if !unrepresentable.contains(&c) {
                    unrepresentable.push(c);
                }
                (Font::Times, vec![SUBSTITUTE])
            }
        };
        match runs.last_mut() {
            Some(run) if run.font == font => run.bytes.extend_from_slice(&bytes),
            _ => runs.push(Run { font, bytes }),
        }
    }
    Encoded {
        runs,
        unrepresentable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin1_are_identity() {
        assert_eq!(winansi_byte('A'), Some(0x41));
        assert_eq!(winansi_byte('é'), Some(0xE9));
        assert_eq!(winansi_byte('ï'), Some(0xEF));
        assert_eq!(winansi_byte('ÿ'), Some(0xFF));
    }

    #[test]
    fn windows_1252_punctuation_maps() {
        assert_eq!(winansi_byte('—'), Some(0x97));
        assert_eq!(winansi_byte('€'), Some(0x80));
        assert_eq!(winansi_byte('\u{201C}'), Some(0x93));
    }

    #[test]
    fn controls_and_non_latin_are_not_winansi() {
        assert_eq!(winansi_byte('\n'), None);
        assert_eq!(winansi_byte('∫'), None);
        assert_eq!(winansi_byte('😀'), None);
        assert_eq!(winansi_byte('\u{81}'), None);
    }

    #[test]
    fn symbol_covers_greek_and_operators() {
        assert_eq!(symbol_byte('α'), Some(0x61));
        assert_eq!(symbol_byte('Ω'), Some(0x57));
        assert_eq!(symbol_byte('√'), Some(0xD6));
        assert_eq!(symbol_byte('∑'), Some(0xE5));
        assert_eq!(symbol_byte('∫'), Some(0xF2));
        assert_eq!(symbol_byte('⟨'), Some(0xE1));
        assert_eq!(symbol_byte('⟩'), Some(0xF1));
        assert_eq!(symbol_byte('😀'), None);
        assert_eq!(symbol_byte('─'), None, "box drawing is a rule, not a glyph");
    }

    #[test]
    fn encode_splits_runs_and_reports_substitutions_once() {
        let e = encode("a∫b∫😀αβ");
        assert_eq!(
            e.runs,
            vec![
                Run {
                    font: Font::Times,
                    bytes: b"a".to_vec()
                },
                Run {
                    font: Font::Symbol,
                    bytes: vec![0xF2]
                },
                Run {
                    font: Font::Times,
                    bytes: b"b".to_vec()
                },
                Run {
                    font: Font::Symbol,
                    bytes: vec![0xF2]
                },
                Run {
                    font: Font::Times,
                    bytes: b"?".to_vec()
                },
                Run {
                    font: Font::Symbol,
                    bytes: vec![0x61, 0x62]
                },
            ]
        );
        assert_eq!(e.unrepresentable, vec!['😀']);
        assert_eq!(e.bytes(), b"a\xF2b\xF2?\x61\x62");
    }

    #[test]
    fn embedded_lookup_is_consulted_only_after_both_base_fonts() {
        let lookup = |c: char| match c {
            '中' => Some(0x0102u16),
            'a' | 'α' => Some(0xFFFF),
            _ => None,
        };
        let e = encode_with("aα中😀", Some(&lookup));
        assert_eq!(
            e.runs,
            vec![
                Run {
                    font: Font::Times,
                    bytes: b"a".to_vec()
                },
                Run {
                    font: Font::Symbol,
                    bytes: vec![0x61]
                },
                Run {
                    font: Font::Embedded,
                    bytes: vec![0x01, 0x02]
                },
                Run {
                    font: Font::Times,
                    bytes: b"?".to_vec()
                },
            ]
        );
        assert_eq!(e.unrepresentable, vec!['😀']);
        assert!(needs_embedding('中') && !needs_embedding('α') && !needs_embedding('a'));
    }

    #[test]
    fn embedded_face_is_consulted_first_then_symbol_then_times() {
        let lookup = |c: char| match c {
            'a' | 'é' | '中' => Some(0x0010u16),
            _ => None,
        };
        // 'a' and 'é' go to the embedded font even though WinAnsi has them;
        // 'α' falls back to Symbol; 'ç' to Times (WinAnsi) since the lookup
        // lacks it; '😀' is still substituted.
        let e = encode_face("aéαç中😀", Some(&lookup), Face::Embedded);
        let fonts: Vec<(Font, usize)> = e.runs.iter().map(|r| (r.font, r.bytes.len())).collect();
        assert_eq!(
            fonts,
            vec![
                (Font::Embedded, 4),
                (Font::Symbol, 1),
                (Font::Times, 1),
                (Font::Embedded, 2),
                (Font::Times, 1),
            ]
        );
        assert_eq!(e.runs[2].bytes, vec![0xE7]);
        assert_eq!(e.unrepresentable, vec!['😀']);
        assert_eq!(
            encode_face("a", None, Face::Embedded).runs[0].font,
            Font::Times
        );
    }

    #[test]
    fn winansi_wins_over_symbol_for_shared_characters() {
        let e = encode("±×÷");
        assert_eq!(e.runs.len(), 1);
        assert_eq!(e.runs[0].font, Font::Times);
        assert_eq!(e.runs[0].bytes, vec![0xB1, 0xD7, 0xF7]);
    }
}
