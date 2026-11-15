//! ICU's dictionary break engines for Thai, Lao, Myanmar and Khmer, for
//! `\XeTeXlinebreaklocale`: a port of ICU 78.2's `divideUpDictionaryRange`
//! of `ThaiBreakEngine`, `LaoBreakEngine`, `BurmeseBreakEngine` and
//! `KhmerBreakEngine` and their `PossibleWord` helper (dictbe.cpp), of
//! `BytesDictionaryMatcher::matches` and `transform` (dictionarydata.cpp),
//! and of the parts of `BytesTrie` that `matches` uses (bytestrie.cpp,
//! unicode/bytestrie.h). The sources of record are vendored unmodified in
//! third_party/icu/icu-src; the dictionaries are ICU's own
//! `brkitr/*dict.dict` from TeX Live 2026's `icudt78l.dat`
//! (third_party/icu/brkitr), the data xetex links. ICU is under the Unicode
//! licence (third_party/icu/LICENSE), which permits this port; this file
//! keeps ICU's structure and names so that it can be checked against them.
//!
//! An engine is not a plain dictionary segmentation. It looks up to three
//! words ahead to choose among the dictionary words that start at a
//! position; text that is no dictionary word is joined to a short word
//! before it, or skipped up to the next place where a word may begin and a
//! dictionary word does; it never stops before a combining mark; and, in
//! Thai, PAIYANNOI (U+0E2F) and MAIYAMOK (U+0E46) join the word before them.
//!
//! `text` is the whole UTF-16 text ICU's `UText` holds, indexed in code
//! units as `utext_getNativeIndex` counts them: the dictionary match may
//! read up to the end of the range, the suffix rules a character past it.

use super::{current32, next32, SaScript, SENTINEL};

/// `brkitr/thaidict.dict` and the others: `Dict` data files (udata header,
/// then the `DictionaryData` indexes and a `BytesTrie`).
static THAI_DICT: &[u8] = include_bytes!("../../../../third_party/icu/brkitr/thaidict.dict");
static LAO_DICT: &[u8] = include_bytes!("../../../../third_party/icu/brkitr/laodict.dict");
static BURMESE_DICT: &[u8] = include_bytes!("../../../../third_party/icu/brkitr/burmesedict.dict");
static KHMER_DICT: &[u8] = include_bytes!("../../../../third_party/icu/brkitr/khmerdict.dict");

// --- UText over UTF-16 -----------------------------------------------------

/// The `UText` the engine walks: the text and its native index.
struct UText<'a> {
    s: &'a [u16],
    pos: usize,
}

impl UText<'_> {
    fn index(&self) -> usize {
        self.pos
    }

    /// `utext_setNativeIndex`: clamped to the text, and moved back to the
    /// start of a surrogate pair it would split.
    fn set_index(&mut self, i: usize) {
        let mut i = i.min(self.s.len());
        if i > 0
            && i < self.s.len()
            && (0xDC00..0xE000).contains(&self.s[i])
            && (0xD800..0xDC00).contains(&self.s[i - 1])
        {
            i -= 1;
        }
        self.pos = i;
    }

    fn next32(&mut self) -> i32 {
        next32(self.s, &mut self.pos)
    }

    fn current32(&self) -> i32 {
        current32(self.s, self.pos)
    }

    /// `utext_previous32`: the code point before the index, the index moved
    /// to its start; `U_SENTINEL` at the start of the text.
    fn previous32(&mut self) -> i32 {
        if self.pos == 0 {
            return SENTINEL;
        }
        self.pos -= 1;
        let t = self.s[self.pos];
        if (0xDC00..0xE000).contains(&t) && self.pos > 0 {
            let u = self.s[self.pos - 1];
            if (0xD800..0xDC00).contains(&u) {
                self.pos -= 1;
                return 0x10000 + ((i32::from(u) - 0xD800) << 10) + (i32::from(t) - 0xDC00);
            }
        }
        i32::from(t)
    }

    /// `utext_moveIndex32(text, n)` for `n > 0`.
    fn move_index32(&mut self, n: usize) {
        for _ in 0..n {
            if self.next32() == SENTINEL {
                break;
            }
        }
    }
}

// --- BytesTrie ---------------------------------------------------------------

/// `UStringTrieResult`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TrieResult {
    NoMatch,
    NoValue,
    FinalValue,
    IntermediateValue,
}

impl TrieResult {
    /// `USTRINGTRIE_HAS_VALUE`.
    fn has_value(self) -> bool {
        matches!(self, TrieResult::FinalValue | TrieResult::IntermediateValue)
    }
}

const K_MAX_BRANCH_LINEAR_SUB_NODE_LENGTH: i32 = 5;
const K_MIN_LINEAR_MATCH: i32 = 0x10;
const K_MIN_VALUE_LEAD: i32 = 0x20;
const K_VALUE_IS_FINAL: i32 = 1;
const K_MIN_ONE_BYTE_VALUE_LEAD: i32 = 0x10;
const K_MIN_TWO_BYTE_VALUE_LEAD: i32 = 0x51;
const K_MIN_THREE_BYTE_VALUE_LEAD: i32 = 0x6c;
const K_FOUR_BYTE_VALUE_LEAD: i32 = 0x7e;
const K_MIN_TWO_BYTE_DELTA_LEAD: i32 = 0xc0;
const K_MIN_THREE_BYTE_DELTA_LEAD: i32 = 0xf0;
const K_FOUR_BYTE_DELTA_LEAD: i32 = 0xfe;

/// ICU's `BytesTrie`, the members `matches` uses: `first`, `next`.
struct BytesTrie<'a> {
    bytes: &'a [u8],
    /// `pos_`: `None` once the trie has stopped (`stop()`).
    pos: Option<usize>,
    /// `remainingMatchLength_`.
    remaining_match_length: i32,
}

impl<'a> BytesTrie<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        BytesTrie {
            bytes,
            pos: Some(0),
            remaining_match_length: -1,
        }
    }

    fn b(&self, p: usize) -> i32 {
        i32::from(self.bytes[p])
    }

    fn value_result(node: i32) -> TrieResult {
        if node & K_VALUE_IS_FINAL != 0 {
            TrieResult::FinalValue
        } else {
            TrieResult::IntermediateValue
        }
    }

    fn skip_value_lead(mut pos: usize, lead_byte: i32) -> usize {
        if lead_byte >= (K_MIN_TWO_BYTE_VALUE_LEAD << 1) {
            if lead_byte < (K_MIN_THREE_BYTE_VALUE_LEAD << 1) {
                pos += 1;
            } else if lead_byte < (K_FOUR_BYTE_VALUE_LEAD << 1) {
                pos += 2;
            } else {
                pos += 3 + ((lead_byte >> 1) & 1) as usize;
            }
        }
        pos
    }

    fn skip_value(&self, pos: usize) -> usize {
        let lead_byte = self.b(pos);
        Self::skip_value_lead(pos + 1, lead_byte)
    }

    fn skip_delta(&self, mut pos: usize) -> usize {
        let delta = self.b(pos);
        pos += 1;
        if delta >= K_MIN_TWO_BYTE_DELTA_LEAD {
            if delta < K_MIN_THREE_BYTE_DELTA_LEAD {
                pos += 1;
            } else if delta < K_FOUR_BYTE_DELTA_LEAD {
                pos += 2;
            } else {
                pos += 3 + (delta & 1) as usize;
            }
        }
        pos
    }

    fn jump_by_delta(&self, mut pos: usize) -> usize {
        let mut delta = self.b(pos);
        pos += 1;
        if delta < K_MIN_TWO_BYTE_DELTA_LEAD {
            // nothing to do
        } else if delta < K_MIN_THREE_BYTE_DELTA_LEAD {
            delta = ((delta - K_MIN_TWO_BYTE_DELTA_LEAD) << 8) | self.b(pos);
            pos += 1;
        } else if delta < K_FOUR_BYTE_DELTA_LEAD {
            delta = ((delta - K_MIN_THREE_BYTE_DELTA_LEAD) << 16)
                | (self.b(pos) << 8)
                | self.b(pos + 1);
            pos += 2;
        } else if delta == K_FOUR_BYTE_DELTA_LEAD {
            delta = (self.b(pos) << 16) | (self.b(pos + 1) << 8) | self.b(pos + 2);
            pos += 3;
        } else {
            delta = (self.b(pos) << 24)
                | (self.b(pos + 1) << 16)
                | (self.b(pos + 2) << 8)
                | self.b(pos + 3);
            pos += 4;
        }
        pos + delta as usize
    }

    fn stop(&mut self) -> TrieResult {
        self.pos = None;
        TrieResult::NoMatch
    }

    fn branch_next(&mut self, mut pos: usize, mut length: i32, in_byte: i32) -> TrieResult {
        if length == 0 {
            length = self.b(pos);
            pos += 1;
        }
        length += 1;
        // The length of the branch is the number of units to select from.
        // The data structure encodes a binary search.
        while length > K_MAX_BRANCH_LINEAR_SUB_NODE_LENGTH {
            let c = self.b(pos);
            pos += 1;
            if in_byte < c {
                length >>= 1;
                pos = self.jump_by_delta(pos);
            } else {
                length -= length >> 1;
                pos = self.skip_delta(pos);
            }
        }
        // Drop down to linear search for the last few bytes.
        loop {
            let c = self.b(pos);
            pos += 1;
            if in_byte == c {
                let mut node = self.b(pos);
                let result = if node & K_VALUE_IS_FINAL != 0 {
                    // Leave the final value for getValue() to read.
                    TrieResult::FinalValue
                } else {
                    // Use the non-final value as the jump delta.
                    pos += 1;
                    node >>= 1;
                    let delta;
                    if node < K_MIN_TWO_BYTE_VALUE_LEAD {
                        delta = node - K_MIN_ONE_BYTE_VALUE_LEAD;
                    } else if node < K_MIN_THREE_BYTE_VALUE_LEAD {
                        delta = ((node - K_MIN_TWO_BYTE_VALUE_LEAD) << 8) | self.b(pos);
                        pos += 1;
                    } else if node < K_FOUR_BYTE_VALUE_LEAD {
                        delta = ((node - K_MIN_THREE_BYTE_VALUE_LEAD) << 16)
                            | (self.b(pos) << 8)
                            | self.b(pos + 1);
                        pos += 2;
                    } else if node == K_FOUR_BYTE_VALUE_LEAD {
                        delta = (self.b(pos) << 16) | (self.b(pos + 1) << 8) | self.b(pos + 2);
                        pos += 3;
                    } else {
                        delta = (self.b(pos) << 24)
                            | (self.b(pos + 1) << 16)
                            | (self.b(pos + 2) << 8)
                            | self.b(pos + 3);
                        pos += 4;
                    }
                    pos += delta as usize;
                    node = self.b(pos);
                    if node >= K_MIN_VALUE_LEAD {
                        Self::value_result(node)
                    } else {
                        TrieResult::NoValue
                    }
                };
                self.pos = Some(pos);
                return result;
            }
            length -= 1;
            pos = self.skip_value(pos);
            if length <= 1 {
                break;
            }
        }
        let c = self.b(pos);
        pos += 1;
        if in_byte == c {
            self.pos = Some(pos);
            let node = self.b(pos);
            if node >= K_MIN_VALUE_LEAD {
                Self::value_result(node)
            } else {
                TrieResult::NoValue
            }
        } else {
            self.stop()
        }
    }

    fn next_impl(&mut self, mut pos: usize, in_byte: i32) -> TrieResult {
        loop {
            let mut node = self.b(pos);
            pos += 1;
            if node < K_MIN_LINEAR_MATCH {
                return self.branch_next(pos, node, in_byte);
            } else if node < K_MIN_VALUE_LEAD {
                // Match the first of length+1 bytes.
                let mut length = node - K_MIN_LINEAR_MATCH; // Actual match length minus 1.
                let c = self.b(pos);
                pos += 1;
                if in_byte == c {
                    length -= 1;
                    self.remaining_match_length = length;
                    self.pos = Some(pos);
                    return if length < 0 && {
                        node = self.b(pos);
                        node >= K_MIN_VALUE_LEAD
                    } {
                        Self::value_result(node)
                    } else {
                        TrieResult::NoValue
                    };
                } else {
                    // No match.
                    break;
                }
            } else if node & K_VALUE_IS_FINAL != 0 {
                // No further matching bytes.
                break;
            } else {
                // Skip intermediate value.
                pos = Self::skip_value_lead(pos, node);
                // The next node must not also be a value node.
            }
        }
        self.stop()
    }

    /// `first(inByte)`.
    fn first(&mut self, mut in_byte: i32) -> TrieResult {
        self.remaining_match_length = -1;
        if in_byte < 0 {
            in_byte += 0x100;
        }
        self.next_impl(0, in_byte)
    }

    /// `next(inByte)`.
    fn next(&mut self, mut in_byte: i32) -> TrieResult {
        let Some(mut pos) = self.pos else {
            return TrieResult::NoMatch;
        };
        if in_byte < 0 {
            in_byte += 0x100;
        }
        let mut length = self.remaining_match_length; // Actual remaining match length minus 1.
        if length >= 0 {
            // Remaining part of a linear-match node.
            let c = self.b(pos);
            pos += 1;
            if in_byte == c {
                length -= 1;
                self.remaining_match_length = length;
                self.pos = Some(pos);
                let node = self.b(pos);
                if length < 0 && node >= K_MIN_VALUE_LEAD {
                    Self::value_result(node)
                } else {
                    TrieResult::NoValue
                }
            } else {
                self.stop()
            }
        } else {
            self.next_impl(pos, in_byte)
        }
    }
}

// --- BytesDictionaryMatcher --------------------------------------------------

const TRANSFORM_TYPE_OFFSET: i32 = 0x100_0000;
const TRANSFORM_TYPE_MASK: i32 = 0x7f00_0000;
const TRANSFORM_OFFSET_MASK: i32 = 0x1f_ffff;

/// `BytesDictionaryMatcher`: the trie's bytes and the transform constant,
/// as `ICULanguageBreakFactory::loadDictionaryMatcherFor` reads them from
/// the data file (`IX_STRING_TRIE_OFFSET`, `IX_TRIE_TYPE`, `IX_TRANSFORM`).
pub(super) struct Dictionary {
    characters: &'static [u8],
    transform_constant: i32,
}

impl Dictionary {
    fn open(data: &'static [u8]) -> Dictionary {
        let header_size = usize::from(u16::from_le_bytes([data[0], data[1]]));
        assert_eq!((data[2], data[3]), (0xda, 0x27), "udata magic");
        assert_eq!(&data[12..16], b"Dict", "dictionary data format");
        let d = &data[header_size..];
        let ix =
            |k: usize| i32::from_le_bytes([d[4 * k], d[4 * k + 1], d[4 * k + 2], d[4 * k + 3]]);
        let offset = ix(0) as usize;
        let total = ix(3) as usize;
        assert_eq!(ix(4) & 7, 0, "a BytesTrie dictionary (TRIE_TYPE_BYTES)");
        Dictionary {
            characters: &d[offset..total],
            transform_constant: ix(5),
        }
    }

    /// The engine's dictionary: `brkitr/thaidict.dict`, `laodict.dict`,
    /// `burmesedict.dict` or `khmerdict.dict` (`loadDictionaryMatcherFor`
    /// of the script).
    pub(super) fn of(engine: SaScript) -> &'static Dictionary {
        use std::sync::OnceLock;
        static D: [OnceLock<Dictionary>; 4] = [const { OnceLock::new() }; 4];
        let (k, data) = match engine {
            SaScript::Thai => (0, THAI_DICT),
            SaScript::Lao => (1, LAO_DICT),
            SaScript::Myanmar => (2, BURMESE_DICT),
            SaScript::Khmer => (3, KHMER_DICT),
            SaScript::Other => unreachable!("no dictionary engine"),
        };
        D[k].get_or_init(|| Dictionary::open(data))
    }

    fn transform(&self, c: i32) -> i32 {
        if (self.transform_constant & TRANSFORM_TYPE_MASK) == TRANSFORM_TYPE_OFFSET {
            if c == 0x200D {
                return 0xFF;
            } else if c == 0x200C {
                return 0xFE;
            }
            let delta = c - (self.transform_constant & TRANSFORM_OFFSET_MASK);
            if !(0..=0xFD).contains(&delta) {
                return SENTINEL;
            }
            return delta;
        }
        c
    }

    /// `matches(text, maxLength, limit, lengths, cpLengths, nullptr,
    /// prefix)`: the dictionary words that start at the text's index, in
    /// code units and code points, at most `limit` of them; and how many
    /// code points the trie read (`prefix`). The text is left after the
    /// last code point read.
    fn matches(
        &self,
        text: &mut UText,
        max_length: i32,
        limit: usize,
        lengths: &mut [i32],
        cp_lengths: &mut [i32],
        prefix: &mut i32,
    ) -> i32 {
        let mut bt = BytesTrie::new(self.characters);
        let starting_text_index = text.index() as i32;
        let mut word_count = 0usize;
        let mut code_points_matched = 0;
        loop {
            let c = text.next32();
            if c < 0 {
                break;
            }
            let result = if code_points_matched == 0 {
                bt.first(self.transform(c))
            } else {
                bt.next(self.transform(c))
            };
            let length_matched = text.index() as i32 - starting_text_index;
            code_points_matched += 1;
            if result.has_value() {
                if word_count < limit {
                    lengths[word_count] = length_matched;
                    cp_lengths[word_count] = code_points_matched;
                    word_count += 1;
                }
                if result == TrieResult::FinalValue {
                    break;
                }
            } else if result == TrieResult::NoMatch {
                break;
            }
            if length_matched >= max_length {
                break;
            }
        }
        *prefix = code_points_matched;
        word_count as i32
    }
}

// --- PossibleWord ------------------------------------------------------------

/// `POSSIBLE_WORD_LIST_MAX`.
const POSSIBLE_WORD_LIST_MAX: usize = 20;

/// `PossibleWord`: the dictionary words that start at one offset.
#[derive(Clone, Copy)]
struct PossibleWord {
    /// Count of candidates.
    count: i32,
    /// The longest match with a dictionary word.
    prefix: i32,
    /// Offset in the text of these candidates.
    offset: i64,
    /// The preferred candidate's index.
    mark: i32,
    /// The candidate being looked at.
    current: i32,
    /// Word lengths, in code units.
    cu_lengths: [i32; POSSIBLE_WORD_LIST_MAX],
    /// Word lengths, in code points.
    cp_lengths: [i32; POSSIBLE_WORD_LIST_MAX],
}

impl PossibleWord {
    fn new() -> Self {
        PossibleWord {
            count: 0,
            prefix: 0,
            offset: -1,
            mark: 0,
            current: 0,
            cu_lengths: [0; POSSIBLE_WORD_LIST_MAX],
            cp_lengths: [0; POSSIBLE_WORD_LIST_MAX],
        }
    }

    /// Fill the list of candidates if needed, select the longest, and
    /// return the number found.
    fn candidates(&mut self, text: &mut UText, dict: &Dictionary, range_end: usize) -> i32 {
        let start = text.index();
        if start as i64 != self.offset {
            self.offset = start as i64;
            self.count = dict.matches(
                text,
                range_end as i32 - start as i32,
                POSSIBLE_WORD_LIST_MAX,
                &mut self.cu_lengths,
                &mut self.cp_lengths,
                &mut self.prefix,
            );
            // Dictionary leaves text after longest prefix, not longest word. Back up.
            if self.count <= 0 {
                text.set_index(start);
            }
        }
        if self.count > 0 {
            text.set_index(start + self.cu_lengths[(self.count - 1) as usize] as usize);
        }
        self.current = self.count - 1;
        self.mark = self.current;
        self.count
    }

    /// Select the currently marked candidate, point after it in the text.
    fn accept_marked(&self, text: &mut UText) -> i32 {
        let len = self.cu_lengths[self.mark as usize];
        text.set_index((self.offset + i64::from(len)) as usize);
        len
    }

    /// Back up from the current candidate to the next shorter one; true if
    /// that exists, with the text after it.
    fn back_up(&mut self, text: &mut UText) -> bool {
        if self.current > 0 {
            self.current -= 1;
            text.set_index(
                (self.offset + i64::from(self.cu_lengths[self.current as usize])) as usize,
            );
            return true;
        }
        false
    }

    /// The longest prefix this candidate location shares with a dictionary
    /// word, in code points.
    fn longest_prefix(&self) -> i32 {
        self.prefix
    }

    fn mark_current(&mut self) {
        self.mark = self.current;
    }

    fn marked_cp_length(&self) -> i32 {
        self.cp_lengths[self.mark as usize]
    }
}

// --- The Thai, Lao, Burmese and Khmer engines -------------------------------

// Each engine has its own copy of these in dictbe.cpp (THAI_LOOKAHEAD,
// LAO_LOOKAHEAD, BURMESE_..., KHMER_...); all four have the same values.

/// How many words in a row are "good enough"?
const LOOKAHEAD: u32 = 3;
/// Will not combine a non-word with a preceding dictionary word longer than this.
const ROOT_COMBINE_THRESHOLD: i32 = 3;
/// Will not combine a non-word that shares at least this much prefix with a
/// dictionary word, with a preceding word.
const PREFIX_COMBINE_THRESHOLD: i32 = 3;
/// Minimum word size.
const MIN_WORD: usize = 2;
/// Minimum number of characters for two words.
const MIN_WORD_SPAN: usize = MIN_WORD * 2;
/// Elision character (Thai only).
const THAI_PAIYANNOI: i32 = 0x0E2F;
/// Repeat character (Thai only).
const THAI_MAIYAMOK: i32 = 0x0E46;

/// The engine's set, `[[:Thai:]&[:LineBreak=SA:]]` or its Lao, Myanmar or
/// Khmer counterpart (linebreak_sa.rs, Unicode 17).
fn word_set(engine: SaScript, c: i32) -> bool {
    super::sa_script(c) == Some(engine)
}

/// `fMarkSet`: `[[:Thai:]&[:LineBreak=SA:]&[:M:]]` (or its counterpart)
/// and U+0020.
fn mark_set(engine: SaScript, c: i32) -> bool {
    if c == 0x0020 {
        return true;
    }
    let Ok(u) = u32::try_from(c) else {
        return false;
    };
    let k = super::sa::SA_MARKS.partition_point(|&(_, last)| last < u);
    super::sa::SA_MARKS
        .get(k)
        .is_some_and(|&(first, _)| first <= u)
        && word_set(engine, c)
}

/// `fEndWordSet`, as each engine's constructor makes it.
fn end_word_set(engine: SaScript, c: i32) -> bool {
    word_set(engine, c)
        && match engine {
            // MAI HAN-AKAT; SARA E through SARA AI MAIMALAI
            SaScript::Thai => c != 0x0E31 && !(0x0E40..=0x0E44).contains(&c),
            // prefix vowels
            SaScript::Lao => !(0x0EC0..=0x0EC4).contains(&c),
            // KHMER SIGN COENG that combines some following characters
            SaScript::Khmer => c != 0x17D2,
            _ => true,
        }
}

/// `fBeginWordSet`, as each engine's constructor makes it.
fn begin_word_set(engine: SaScript, c: i32) -> bool {
    match engine {
        // KO KAI through HO NOKHUK; SARA E through SARA AI MAIMALAI
        SaScript::Thai => matches!(c, 0x0E01..=0x0E2E | 0x0E40..=0x0E44),
        // basic consonants (with holes), digraph consonants, prefix vowels
        SaScript::Lao => matches!(c, 0x0E81..=0x0EAE | 0x0EDC..=0x0EDD | 0x0EC0..=0x0EC4),
        // basic consonants and independent vowels
        SaScript::Myanmar => matches!(c, 0x1000..=0x102A),
        SaScript::Khmer => matches!(c, 0x1780..=0x17B3),
        SaScript::Other => false,
    }
}

/// `fSuffixSet` (Thai only): PAIYANNOI and MAIYAMOK.
fn thai_suffix_set(c: i32) -> bool {
    c == THAI_PAIYANNOI || c == THAI_MAIYAMOK
}

/// `divideUpDictionaryRange(text, rangeStart, rangeEnd, foundBreaks)` of
/// `ThaiBreakEngine`, `LaoBreakEngine`, `BurmeseBreakEngine` or
/// `KhmerBreakEngine`: pushes the word ends it finds in
/// `[rangeStart, rangeEnd)` to `found_breaks` and returns how many words it
/// found. The four differ only in their dictionary and sets, in how the
/// minimum span is counted (code points for Thai, code units for the
/// others) and in Thai's suffix rules.
pub(super) fn divide_up_dictionary_range(
    engine: SaScript,
    text: &[u16],
    range_start: usize,
    range_end: usize,
    found_breaks: &mut Vec<usize>,
) -> u32 {
    let dict = Dictionary::of(engine);
    let mut text = UText { s: text, pos: 0 };
    if engine == SaScript::Thai {
        text.set_index(range_start);
        text.move_index32(MIN_WORD_SPAN);
        if text.index() >= range_end {
            return 0; // Not enough characters for two words
        }
    } else if range_end - range_start < MIN_WORD_SPAN {
        return 0; // Not enough characters for two words
    }
    text.set_index(range_start);

    let mut words_found: u32 = 0;
    let mut words = [PossibleWord::new(); LOOKAHEAD as usize];
    let w = |words_found: u32, k: u32| ((words_found + k) % LOOKAHEAD) as usize;

    loop {
        let current = text.index();
        if current >= range_end {
            break;
        }
        let mut cp_word_length = 0; // Word length in code points.
        let mut cu_word_length = 0usize; // Word length in code units.

        // Look for candidate words at the current position
        let candidates = words[w(words_found, 0)].candidates(&mut text, dict, range_end);

        // If we found exactly one, use that
        if candidates == 1 {
            cu_word_length = words[w(words_found, 0)].accept_marked(&mut text) as usize;
            cp_word_length = words[w(words_found, 0)].marked_cp_length();
            words_found += 1;
        }
        // If there was more than one, see which one can take us forward the most words
        else if candidates > 1 {
            'found_best: {
                // If we're already at the end of the range, we're done
                if text.index() >= range_end {
                    break 'found_best;
                }
                loop {
                    if words[w(words_found, 1)].candidates(&mut text, dict, range_end) > 0 {
                        // Followed by another dictionary word; mark first word as a good candidate
                        words[w(words_found, 0)].mark_current();

                        // If we're already at the end of the range, we're done
                        if text.index() >= range_end {
                            break 'found_best;
                        }

                        // See if any of the possible second words is followed by a third word
                        loop {
                            // If we find a third word, stop right away
                            if words[w(words_found, 2)].candidates(&mut text, dict, range_end) != 0
                            {
                                words[w(words_found, 0)].mark_current();
                                break 'found_best;
                            }
                            if !words[w(words_found, 1)].back_up(&mut text) {
                                break;
                            }
                        }
                    }
                    if !words[w(words_found, 0)].back_up(&mut text) {
                        break;
                    }
                }
            }
            // foundBest: set the text position to after the accepted word.
            cu_word_length = words[w(words_found, 0)].accept_marked(&mut text) as usize;
            cp_word_length = words[w(words_found, 0)].marked_cp_length();
            words_found += 1;
        }

        // We come here after having either found a word or not. We look ahead to the
        // next word. If it's not a dictionary word, we will combine it with the word we
        // just found (if there is one), but only if the preceding word does not exceed
        // the threshold.
        // The text iterator should now be positioned at the end of the word we found.

        // `uc` (0 in ICU) is set before every read.
        let mut uc: i32;
        if text.index() < range_end && cp_word_length < ROOT_COMBINE_THRESHOLD {
            // if it is a dictionary word, do nothing. If it isn't, then if there is
            // no preceding word, or the non-word shares less than the minimum threshold
            // of characters with a dictionary word, then scan to resynchronize
            if words[w(words_found, 0)].candidates(&mut text, dict, range_end) <= 0
                && (cu_word_length == 0
                    || words[w(words_found, 0)].longest_prefix() < PREFIX_COMBINE_THRESHOLD)
            {
                // Look for a plausible word boundary
                let mut remaining = range_end as i64 - (current + cu_word_length) as i64;
                let mut chars = 0usize;
                loop {
                    let pc_index = text.index();
                    let pc = text.next32();
                    let pc_size = text.index() - pc_index;
                    chars += pc_size;
                    remaining -= pc_size as i64;
                    if remaining <= 0 {
                        break;
                    }
                    uc = text.current32();
                    if end_word_set(engine, pc) && begin_word_set(engine, uc) {
                        // Maybe. See if it's in the dictionary.
                        let num_candidates =
                            words[w(words_found, 1)].candidates(&mut text, dict, range_end);
                        text.set_index(current + cu_word_length + chars);
                        if num_candidates > 0 {
                            break;
                        }
                    }
                }

                // Bump the word count if there wasn't already one
                if cu_word_length == 0 {
                    words_found += 1;
                }

                // Update the length with the passed-over characters
                cu_word_length += chars;
            } else {
                // Back up to where we were for next iteration
                text.set_index(current + cu_word_length);
            }
        }

        // Never stop before a combining mark.
        loop {
            let curr_pos = text.index();
            if !(curr_pos < range_end && mark_set(engine, text.current32())) {
                break;
            }
            text.next32();
            cu_word_length += text.index() - curr_pos;
        }

        // Look ahead for possible suffixes if a dictionary word does not follow.
        // We do this in code rather than using a rule so that the heuristic
        // resynch continues to function. For example, one of the suffix characters
        // could be a typo in the middle of a word.
        if engine == SaScript::Thai && text.index() < range_end && cu_word_length > 0 {
            if words[w(words_found, 0)].candidates(&mut text, dict, range_end) <= 0 && {
                uc = text.current32();
                thai_suffix_set(uc)
            } {
                if uc == THAI_PAIYANNOI {
                    if !thai_suffix_set(text.previous32()) {
                        // Skip over previous end and PAIYANNOI
                        text.next32();
                        let paiyannoi_index = text.index();
                        text.next32();
                        cu_word_length += text.index() - paiyannoi_index; // Add PAIYANNOI to word
                        uc = text.current32(); // Fetch next character
                    } else {
                        // Restore prior position
                        text.next32();
                    }
                }
                if uc == THAI_MAIYAMOK {
                    if text.previous32() != THAI_MAIYAMOK {
                        // Skip over previous end and MAIYAMOK
                        text.next32();
                        let maiyamok_index = text.index();
                        text.next32();
                        cu_word_length += text.index() - maiyamok_index; // Add MAIYAMOK to word
                    } else {
                        // Restore prior position
                        text.next32();
                    }
                }
            } else {
                text.set_index(current + cu_word_length);
            }
        }

        // Did we find a word on this iteration? If so, push it on the break stack
        if cu_word_length > 0 {
            found_breaks.push(current + cu_word_length);
        }
    }

    // Don't return a break for the end of the dictionary range if there is one there.
    if found_breaks.last().copied().unwrap_or(0) >= range_end {
        found_breaks.pop();
        words_found = words_found.wrapping_sub(1);
    }

    words_found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    /// The dictionary's words that start at the beginning of `s`, in code
    /// points, and the prefix the trie read.
    fn words(s: &str) -> (Vec<i32>, i32) {
        let s = u(s);
        let mut t = UText { s: &s, pos: 0 };
        let (mut cu, mut cp, mut prefix) = ([0; 20], [0; 20], 0);
        let n = Dictionary::of(SaScript::Thai).matches(
            &mut t,
            s.len() as i32,
            20,
            &mut cu,
            &mut cp,
            &mut prefix,
        );
        (cp[..n as usize].to_vec(), prefix)
    }

    #[test]
    fn thai_dictionary_matches_words_and_prefixes() {
        // ภา, ภาษ and ภาษา "language" are words; ภาษาไทย is not one.
        assert_eq!(words("ภาษาไทยดี"), (vec![2, 3, 4], 5));
        // สวัสดี is a word; สวัสด is not one, but a prefix of it.
        let (w, p) = words("สวัสด");
        assert!(!w.contains(&5), "{w:?}");
        assert_eq!(p, 5);
        // ฯ starts ฯลฯ but no word: the prefix counts the code point
        // that failed to match too.
        assert_eq!(words("ฯฯ"), (vec![], 2));
    }

    #[test]
    fn sets_follow_icu() {
        use SaScript::*;
        let m = |e, c| mark_set(e, c);
        assert!(m(Thai, 0x0E31) && m(Thai, 0x20) && !m(Thai, 0x0E32) && !m(Thai, 0x0E33));
        assert!(m(Thai, 0x0E4E) && !m(Thai, 0x0E4F) && !m(Lao, 0x0E31) && m(Lao, 0x0EB1));
        assert!(m(Khmer, 0x17D2) && m(Myanmar, 0x102B) && m(Lao, 0x20) && !m(Khmer, 0x1780));
        assert!(!end_word_set(Thai, 0x0E40) && end_word_set(Thai, 0x0E45));
        assert!(!end_word_set(Lao, 0x0EC0) && !end_word_set(Khmer, 0x17D2));
        assert!(end_word_set(Myanmar, 0x103A) && !end_word_set(Myanmar, 0x0E01));
        assert!(begin_word_set(Thai, 0x0E2E) && !begin_word_set(Thai, 0x0E2F));
        assert!(begin_word_set(Lao, 0x0EDD) && !begin_word_set(Khmer, 0x17B4));
    }
}
