//! `ubidi_props.cpp`: the bidi properties of a code point, read from ICU
//! 78.2's trie (`props_data`, generated from `ubidi_props_data.h`) with
//! `utrie2.h`'s `UTRIE2_GET16`. Only what `ubidi.cpp`'s algorithm calls is
//! ported: `ubidi_getClass`, `ubidi_getPairedBracketType` and
//! `ubidi_getPairedBracket` (`u_getBidiPairedBracket`) with `getMirror`.

use super::props_data::{HIGH_START, HIGH_VALUE_INDEX, INDEXES, INDEX_LENGTH, MIRRORS, TRIE_INDEX};

// utrie2.h
const UTRIE2_SHIFT_1: u32 = 6 + 5;
const UTRIE2_SHIFT_2: u32 = 5;
const UTRIE2_SHIFT_1_2: u32 = UTRIE2_SHIFT_1 - UTRIE2_SHIFT_2;
const UTRIE2_OMITTED_BMP_INDEX_1_LENGTH: usize = 0x10000 >> UTRIE2_SHIFT_1;
const UTRIE2_INDEX_2_BLOCK_LENGTH: u32 = 1 << UTRIE2_SHIFT_1_2;
const UTRIE2_INDEX_2_MASK: u32 = UTRIE2_INDEX_2_BLOCK_LENGTH - 1;
const UTRIE2_DATA_BLOCK_LENGTH: u32 = 1 << UTRIE2_SHIFT_2;
const UTRIE2_DATA_MASK: u32 = UTRIE2_DATA_BLOCK_LENGTH - 1;
const UTRIE2_INDEX_SHIFT: u32 = 2;
const UTRIE2_LSCP_INDEX_2_OFFSET: usize = 0x10000 >> UTRIE2_SHIFT_2;
const UTRIE2_LSCP_INDEX_2_LENGTH: usize = 0x400 >> UTRIE2_SHIFT_2;
const UTRIE2_INDEX_2_BMP_LENGTH: usize = UTRIE2_LSCP_INDEX_2_OFFSET + UTRIE2_LSCP_INDEX_2_LENGTH;
const UTRIE2_UTF8_2B_INDEX_2_OFFSET: usize = UTRIE2_INDEX_2_BMP_LENGTH;
const UTRIE2_UTF8_2B_INDEX_2_LENGTH: usize = 0x800 >> 6;
const UTRIE2_INDEX_1_OFFSET: usize = UTRIE2_UTF8_2B_INDEX_2_OFFSET + UTRIE2_UTF8_2B_INDEX_2_LENGTH;
const UTRIE2_BAD_UTF8_DATA_OFFSET: usize = 0x80;

// ubidi_props.h
const UBIDI_IX_MIRROR_LENGTH: usize = 3;
const UBIDI_BPT_SHIFT: u32 = 8;
const UBIDI_MIRROR_DELTA_SHIFT: u32 = 13;
const UBIDI_CLASS_MASK: u16 = 0x0000_001f;
const UBIDI_BPT_MASK: u16 = 0x0000_0300;
const UBIDI_ESC_MIRROR_DELTA: i32 = -4;

// A 16-bit trie: its index and its data are one array.
const _: () = assert!(INDEX_LENGTH + super::props_data::DATA_LENGTH == 13388);

/// `UBidiPairedBracketType`'s `U_BPT_OPEN`.
pub const U_BPT_OPEN: u32 = 1;

/// `_UTRIE2_INDEX_RAW`.
fn index_raw(offset: usize, c: u32) -> usize {
    ((TRIE_INDEX[offset + (c >> UTRIE2_SHIFT_2) as usize] as usize) << UTRIE2_INDEX_SHIFT)
        + (c & UTRIE2_DATA_MASK) as usize
}

/// `_UTRIE2_INDEX_FROM_SUPP`.
fn index_from_supp(c: u32) -> usize {
    let i1 = TRIE_INDEX[(UTRIE2_INDEX_1_OFFSET - UTRIE2_OMITTED_BMP_INDEX_1_LENGTH)
        + (c >> UTRIE2_SHIFT_1) as usize] as usize;
    ((TRIE_INDEX[i1 + ((c >> UTRIE2_SHIFT_2) & UTRIE2_INDEX_2_MASK) as usize] as usize)
        << UTRIE2_INDEX_SHIFT)
        + (c & UTRIE2_DATA_MASK) as usize
}

/// `UTRIE2_GET16(&ubidi_props_singleton.trie, c)`: `_UTRIE2_GET` with the
/// trie's index as its data (a 16-bit trie) and `indexLength` as the ASCII
/// offset (`_UTRIE2_INDEX_FROM_CP`).
pub fn get16(c: i32) -> u16 {
    let u = c as u32;
    let i = if u < 0xd800 {
        index_raw(0, u)
    } else if u <= 0xffff {
        index_raw(
            if u <= 0xdbff {
                UTRIE2_LSCP_INDEX_2_OFFSET - (0xd800 >> UTRIE2_SHIFT_2)
            } else {
                0
            },
            u,
        )
    } else if u > 0x10ffff {
        INDEX_LENGTH + UTRIE2_BAD_UTF8_DATA_OFFSET
    } else if u >= HIGH_START {
        HIGH_VALUE_INDEX
    } else {
        index_from_supp(u)
    };
    TRIE_INDEX[i]
}

/// `ubidi_getClass`: the code point's `UCharDirection`.
pub fn get_class(c: i32) -> u8 {
    (get16(c) & UBIDI_CLASS_MASK) as u8
}

/// `ubidi_getPairedBracketType`.
pub fn get_paired_bracket_type(c: i32) -> u32 {
    ((get16(c) & UBIDI_BPT_MASK) >> UBIDI_BPT_SHIFT) as u32
}

/// `UBIDI_GET_MIRROR_DELTA` (an arithmetic right shift of the signed word).
fn mirror_delta(props: u16) -> i32 {
    ((props as i16) >> UBIDI_MIRROR_DELTA_SHIFT) as i32
}

/// `getMirror`.
fn get_mirror(c: i32, props: u16) -> i32 {
    let delta = mirror_delta(props);
    if delta != UBIDI_ESC_MIRROR_DELTA {
        return c + delta;
    }
    // look for the mirror code point in the mirrors[] table
    let length = INDEXES[UBIDI_IX_MIRROR_LENGTH] as usize;
    for &m in MIRRORS.iter().take(length) {
        let c2 = (m & 0x1f_ffff) as i32; // UBIDI_GET_MIRROR_CODE_POINT
        if c == c2 {
            // UBIDI_GET_MIRROR_INDEX
            return (MIRRORS[(m >> 21) as usize] & 0x1f_ffff) as i32;
        } else if c < c2 {
            break;
        }
    }
    c // not found: c itself
}

/// `ubidi_getPairedBracket` (`u_getBidiPairedBracket`).
pub fn get_paired_bracket(c: i32) -> i32 {
    let props = get16(c);
    if props & UBIDI_BPT_MASK == 0 {
        c
    } else {
        get_mirror(c, props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The generated arrays and trie fields are ICU's, value for value.
    #[test]
    fn data_is_the_vendored_headers() {
        let h = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/icu/icu-src/source/common/ubidi_props_data.h");
        let src = std::fs::read_to_string(h).unwrap();
        let body = |name: &str| -> Vec<i64> {
            let at = src.find(&format!(" {name}[")).unwrap();
            let open = at + src[at..].find('{').unwrap();
            let close = open + src[open..].find('}').unwrap();
            src[open + 1..close]
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| match v.strip_prefix("0x") {
                    Some(x) => i64::from_str_radix(x, 16).unwrap(),
                    None => v.parse().unwrap(),
                })
                .collect()
        };
        assert_eq!(
            body("ubidi_props_indexes"),
            INDEXES.iter().map(|&v| i64::from(v)).collect::<Vec<_>>()
        );
        assert_eq!(
            body("ubidi_props_trieIndex"),
            TRIE_INDEX.iter().map(|&v| i64::from(v)).collect::<Vec<_>>()
        );
        assert_eq!(
            body("ubidi_props_mirrors"),
            MIRRORS.iter().map(|&v| i64::from(v)).collect::<Vec<_>>()
        );
        let trie = src.split("ubidi_props_trieIndex+").nth(1).unwrap();
        let fields: Vec<&str> = trie.split(',').map(str::trim).collect();
        assert_eq!(fields[2], INDEX_LENGTH.to_string());
        assert_eq!(fields[3], super::super::props_data::DATA_LENGTH.to_string());
        assert_eq!(fields[8], format!("{HIGH_START:#x}"));
        assert_eq!(fields[9], format!("{HIGH_VALUE_INDEX:#x}"));
    }

    /// Bidi classes from UnicodeData.txt (Unicode 17, ICU 78).
    #[test]
    fn classes() {
        // L R EN ES ET AN CS B S WS ON ... (uchar.h's UCharDirection)
        assert_eq!(get_class('a' as i32), 0);
        assert_eq!(get_class(0x05D0), 1); // HEBREW LETTER ALEF: R
        assert_eq!(get_class('1' as i32), 2); // EN
        assert_eq!(get_class('+' as i32), 3); // ES
        assert_eq!(get_class('$' as i32), 4); // ET
        assert_eq!(get_class(0x0661), 5); // ARABIC-INDIC DIGIT ONE: AN
        assert_eq!(get_class(',' as i32), 6); // CS
        assert_eq!(get_class(0x2029), 7); // PARAGRAPH SEPARATOR: B
        assert_eq!(get_class('\t' as i32), 8); // S
        assert_eq!(get_class(' ' as i32), 9); // WS
        assert_eq!(get_class('!' as i32), 10); // ON
        assert_eq!(get_class(0x202A), 11); // LRE
        assert_eq!(get_class(0x0627), 13); // ARABIC LETTER ALEF: AL
        assert_eq!(get_class(0x05B4), 17); // HEBREW POINT HIRIQ: NSM
        assert_eq!(get_class(0x200D), 18); // ZWJ: BN
        assert_eq!(get_class(0x2068), 19); // FSI
        assert_eq!(get_class(0x2069), 22); // PDI
        assert_eq!(get_class(0x10900), 1); // PHOENICIAN LETTER ALF: R
        assert_eq!(get_class(0x1EE00), 13); // ARABIC MATHEMATICAL ALEF: AL
        assert_eq!(get_class(0x1F600), 10); // an emoji: ON
        assert_eq!(get_class(0x20000), 0); // CJK Extension B: L

        // unassigned code points take their block's default (DerivedBidiClass)
        assert_eq!(get_class(0x05FF), 1);
        assert_eq!(get_class(0x08FF), 17); // ARABIC MARK SIDEWAYS NOON GHUNNA: NSM
        assert_eq!(get_class(0xE0001), 18); // LANGUAGE TAG: BN
    }

    #[test]
    fn brackets() {
        assert_eq!(get_paired_bracket_type('(' as i32), U_BPT_OPEN);
        assert_eq!(get_paired_bracket_type(')' as i32), 2);
        assert_eq!(get_paired_bracket_type('a' as i32), 0);
        assert_eq!(get_paired_bracket('(' as i32), ')' as i32);
        assert_eq!(get_paired_bracket(')' as i32), '(' as i32);
        assert_eq!(get_paired_bracket('[' as i32), ']' as i32);
        assert_eq!(get_paired_bracket('{' as i32), '}' as i32);
        assert_eq!(get_paired_bracket(0x2329), 0x232A); // LEFT-POINTING ANGLE BRACKET
        assert_eq!(get_paired_bracket(0x3008), 0x3009); // LEFT ANGLE BRACKET
        assert_eq!(get_paired_bracket(0xFF08), 0xFF09);
        // mirrored but not a bracket: c itself
        assert_eq!(get_paired_bracket('<' as i32), '<' as i32);
        assert_eq!(get_paired_bracket('a' as i32), 'a' as i32);
    }
}
