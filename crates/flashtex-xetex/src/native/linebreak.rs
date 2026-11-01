//! ICU's line breaker for `\XeTeXlinebreaklocale`: XeTeX_ext.c's
//! `linebreakstart` and `linebreaknext`, which `do_locale_linebreaks`
//! (xetex.web) calls to cut a native word into the pieces between which it
//! puts `\XeTeXlinebreakpenalty` and `\XeTeXlinebreakskip`.
//!
//! XeTeX opens `ubrk_open(UBRK_LINE, locale)` of TeX Live 2026's ICU 78.2
//! and returns `ubrk_next` until `UBRK_DONE` (-1). This module runs the same
//! compiled rules: ICU's own `.brk` tables, vendored unmodified from TeX
//! Live's ICU data (third_party/icu), by a port of ICU's
//! `RuleBasedBreakIterator` for forward iteration: `handleNext` (the state
//! machine, its look-ahead rules and the character categories of its
//! `UCPTrie`), and `BreakCache::populateFollowing` with
//! `DictionaryCache::populateDictionary` (rbbi.cpp, rbbi_cache.cpp,
//! ucptrie.cpp of ICU 78.2).
//!
//! Which table ICU loads depends on the locale, through its `brkitr`
//! resources (`BreakIterator::makeInstance` and `buildInstance`,
//! `ures_openNoDefault`, so there is no fallback to the process's default
//! locale), as third_party/icu/README.md lists: `zh` uses `line_cj`, `ja`
//! and `ko` use `line_normal`, every other locale `line`; the `lb` keyword
//! (`ja@lb=loose`) picks `line_strict`, `line_normal` or `line_loose` of the
//! locale. `lw=phrase` (ICU's phrase breaking for Japanese and Korean) is
//! not ported: it is treated as absent.
//!
//! **Dictionaries.** The rules leave the Southeast Asian scripts (class SA)
//! to ICU's dictionary engines (dictbe.cpp): a run of Thai, Lao, Myanmar or
//! Khmer is cut into words. Those engines are not ported: a run is cut by
//! ICU4X's dictionary segmenter (`icu_segmenter` 2.1, whose dictionaries
//! are exported from ICU 78's), with ICU's minimum run lengths and the Thai
//! engine's MAI YAMOK rule (measured: without it, a break before `ๆ`); the
//! rest of those engines' heuristics are not ported. An SA run of
//! another script gets no breaks, as ICU's `UnhandledEngine` gives it none.
//! Which characters make one run comes from Unicode 17's UCD
//! (`linebreak_sa.rs`, tools/xetex-linebreak/gen_sa_runs.py).
//!
//! Graphite's breaking (`\XeTeXlinebreaklocale "G"` with a Graphite font)
//! is not reached: this port has no Graphite engine.

use icu_segmenter::options::LineBreakOptions;
use icu_segmenter::LineSegmenter;

#[path = "linebreak_sa.rs"]
mod sa;

/// ICU's compiled line rules (third_party/icu/brkitr), chosen by the locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rules {
    Line,
    LineCj,
    LineNormal,
    LineNormalCj,
    LineLoose,
    LineLooseCj,
}

/// The script of a LineBreak=SA character, as ICU's break engine factory
/// sees it (brkeng.cpp's `loadEngineFor`): the four with a dictionary
/// engine, and the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaScript {
    Thai,
    Lao,
    Myanmar,
    Khmer,
    Other,
}

/// The rules `ubrk_open(UBRK_LINE, locale)` loads for `locale`, the bytes
/// of the `\XeTeXlinebreaklocale` string (`gettexstring`, UTF-8).
pub fn rules_for_locale(locale: &[u8]) -> Rules {
    let locale = String::from_utf8_lossy(locale);
    // ICU's locale ID: the language is the first subtag, before `_`, `-`,
    // `.` or `@`; keywords follow `@`, separated by `;`.
    let (id, keywords) = match locale.find('@') {
        Some(i) => (&locale[..i], &locale[i + 1..]),
        None => (&locale[..], ""),
    };
    let lang = id
        .split(['_', '-', '.'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let lb = keywords.split(';').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        k.trim().eq_ignore_ascii_case("lb").then(|| v.trim())
    });
    let lb = match lb {
        Some(v @ ("strict" | "normal" | "loose")) => Some(v),
        _ => None,
    };
    // brkitr/zh.txt, zh_Hant.txt, ja.txt, ko.txt and root.txt of ICU 78.2.
    match (lang.as_str(), lb) {
        ("zh", None | Some("strict")) => Rules::LineCj,
        ("ja" | "ko", None) => Rules::LineNormal,
        ("ja" | "ko", Some("strict")) => Rules::LineCj,
        ("zh" | "ja" | "ko", Some("normal")) => Rules::LineNormalCj,
        ("zh" | "ja" | "ko", _) => Rules::LineLooseCj,
        (_, Some("normal")) => Rules::LineNormal,
        (_, Some("loose")) => Rules::LineLoose,
        _ => Rules::Line,
    }
}

fn table(rules: Rules) -> &'static [u8] {
    match rules {
        Rules::Line => include_bytes!("../../../../third_party/icu/brkitr/line.brk"),
        Rules::LineCj => include_bytes!("../../../../third_party/icu/brkitr/line_cj.brk"),
        Rules::LineNormal => include_bytes!("../../../../third_party/icu/brkitr/line_normal.brk"),
        Rules::LineNormalCj => {
            include_bytes!("../../../../third_party/icu/brkitr/line_normal_cj.brk")
        }
        Rules::LineLoose => include_bytes!("../../../../third_party/icu/brkitr/line_loose.brk"),
        Rules::LineLooseCj => {
            include_bytes!("../../../../third_party/icu/brkitr/line_loose_cj.brk")
        }
    }
}

fn u16_at(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}

fn u32_at(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

/// `U_SENTINEL`: what `UTEXT_NEXT32` returns at the end of the text.
const SENTINEL: i32 = -1;

/// `UTEXT_NEXT32` over UTF-16: the code point at `*i`, `*i` moved past it
/// (a lone surrogate is returned as itself); `U_SENTINEL` at the end.
fn next32(text: &[u16], i: &mut usize) -> i32 {
    let c = current32(text, *i);
    if c != SENTINEL {
        *i += if c > 0xFFFF { 2 } else { 1 };
    }
    c
}

/// `utext_current32`: the code point at `i`, without moving.
fn current32(text: &[u16], i: usize) -> i32 {
    let Some(&u) = text.get(i) else {
        return SENTINEL;
    };
    if (0xD800..0xDC00).contains(&u) {
        if let Some(&t) = text.get(i + 1) {
            if (0xDC00..0xE000).contains(&t) {
                return 0x10000 + ((i32::from(u) - 0xD800) << 10) + (i32::from(t) - 0xDC00);
            }
        }
    }
    i32::from(u)
}

/// A `UCPTrie` (ucptrie.cpp, `ucptrie_openFromBinary` and `ucptrie_get`):
/// the character categories of a `.brk` table.
struct Trie<'a> {
    d: &'a [u8],
    /// Byte offsets of the index (`u16`s) and of the data.
    index: usize,
    data: usize,
    fast: bool,
    /// `UCPTRIE_VALUE_BITS_16` (0), `_32` (1) or `_8` (2).
    width: u16,
    data_length: u32,
    high_start: u32,
}

impl<'a> Trie<'a> {
    fn new(d: &'a [u8], at: usize) -> Trie<'a> {
        assert_eq!(u32_at(d, at), 0x5472_6933, "UCPTrie signature (Tri3)");
        let options = u16_at(d, at + 4);
        let index_length = u16_at(d, at + 6) as usize;
        let data_length = (u32::from(options & 0xF000) << 4) | u32::from(u16_at(d, at + 8));
        let shifted_high_start = u32::from(u16_at(d, at + 14));
        let index = at + 16;
        Trie {
            d,
            index,
            data: index + 2 * index_length,
            fast: (options >> 6) & 3 == 0,
            width: options & 7,
            data_length,
            high_start: shifted_high_start << 9,
        }
    }

    fn ix(&self, i: u32) -> u32 {
        u32::from(u16_at(self.d, self.index + 2 * i as usize))
    }

    /// `ucptrie_internalSmallIndex`.
    fn small_index(&self, c: u32) -> u32 {
        let mut i1 = c >> 14; // UCPTRIE_SHIFT_1
        if self.fast {
            // UCPTRIE_BMP_INDEX_LENGTH - UCPTRIE_OMITTED_BMP_INDEX_1_LENGTH
            i1 += 0x400 - 4;
        } else {
            i1 += 0x40; // UCPTRIE_SMALL_INDEX_LENGTH
        }
        let mut i3_block = self.ix(self.ix(i1) + ((c >> 9) & 0x1F));
        let mut i3 = (c >> 4) & 0x1F;
        let data_block = if i3_block & 0x8000 == 0 {
            self.ix(i3_block + i3)
        } else {
            // 18-bit indexes stored in groups of 9 entries per 8 indexes.
            i3_block = (i3_block & 0x7FFF) + (i3 & !7) + (i3 >> 3);
            i3 &= 7;
            let hi = (self.ix(i3_block) << (2 + 2 * i3)) & 0x30000;
            hi | self.ix(i3_block + 1 + i3)
        };
        data_block + (c & 0xF)
    }

    /// `ucptrie_get`; a negative `c` (`U_SENTINEL`) reads the error value.
    fn get(&self, c: i32) -> u16 {
        let c = c as u32;
        let fast_max = if self.fast { 0xFFFF } else { 0xFFF };
        let i: u32 = if c <= fast_max {
            self.ix(c >> 6) + (c & 0x3F)
        } else if c > 0x10FFFF {
            self.data_length - 1
        } else if c >= self.high_start {
            self.data_length - 2
        } else {
            self.small_index(c)
        };
        let i = i as usize;
        match self.width {
            0 => u16_at(self.d, self.data + 2 * i),
            1 => u32_at(self.d, self.data + 4 * i) as u16,
            _ => u16::from(self.d[self.data + i]),
        }
    }
}

/// A compiled rule-based break iterator (rbbidata.h's `RBBIDataHeader` and
/// its forward `RBBIStateTable`).
struct Rbbi<'a> {
    d: &'a [u8],
    trie: Trie<'a>,
    /// The forward table's first row.
    rows: usize,
    row_len: usize,
    rows8: bool,
    dict_start: u16,
    look_ahead_size: usize,
    bof_required: bool,
}

const RBBI_BOF_REQUIRED: u32 = 2;
const RBBI_8BITS_ROWS: u32 = 4;
const START_STATE: usize = 1;
const STOP_STATE: usize = 0;

impl<'a> Rbbi<'a> {
    fn new(d: &'a [u8]) -> Rbbi<'a> {
        // The `UDataInfo` header of the data item comes first.
        let h = u16_at(d, 0) as usize;
        assert_eq!(u32_at(d, h), 0xB1A0, "RBBIDataHeader magic");
        assert_eq!(d[h + 4], 6, "RBBI data format version 6");
        let ft = h + u32_at(d, h + 16) as usize;
        let trie = Trie::new(d, h + u32_at(d, h + 32) as usize);
        let flags = u32_at(d, ft + 16);
        Rbbi {
            d,
            trie,
            rows: ft + 20,
            row_len: u32_at(d, ft + 4) as usize,
            rows8: flags & RBBI_8BITS_ROWS != 0,
            dict_start: u32_at(d, ft + 8) as u16,
            look_ahead_size: u32_at(d, ft + 12) as usize,
            bof_required: flags & RBBI_BOF_REQUIRED != 0,
        }
    }

    /// Field `k` of state `state`'s row: 0 `fAccepting`, 1 `fLookAhead`,
    /// 2 `fTagsIdx`, 3 + category `fNextState[category]`.
    fn row(&self, state: usize, k: usize) -> usize {
        let at = self.rows + self.row_len * state;
        if self.rows8 {
            usize::from(self.d[at + k])
        } else {
            usize::from(u16_at(self.d, at + 2 * k))
        }
    }

    /// `RuleBasedBreakIterator::handleNext` from `pos`: the next boundary
    /// and how many dictionary characters the match read
    /// (`fDictionaryCharCount`), or `None` (`UBRK_DONE`) at the end.
    fn handle_next(
        &self,
        text: &[u16],
        pos: usize,
        look_ahead: &mut [i32],
    ) -> Option<(usize, u32)> {
        #[derive(PartialEq)]
        enum Mode {
            Start,
            Run,
            End,
        }
        let mut dict_count = 0;
        let initial = pos;
        let mut idx = pos;
        let mut c = next32(text, &mut idx);
        if c == SENTINEL {
            return None;
        }
        let mut result = initial;
        let mut state = START_STATE;
        let mut mode = Mode::Run;
        let mut category = 0;
        if self.bof_required {
            category = 2;
            mode = Mode::Start;
        }
        loop {
            if c == SENTINEL {
                if mode == Mode::End {
                    break;
                }
                mode = Mode::End;
                category = 1;
            }
            if mode == Mode::Run {
                category = self.trie.get(c);
                dict_count += u32::from(category >= self.dict_start);
            }
            state = self.row(state, 3 + usize::from(category));
            let accepting = self.row(state, 0);
            if accepting == 1 {
                // ACCEPTING_UNCONDITIONAL
                if mode != Mode::Start {
                    result = idx;
                }
            } else if accepting > 1 {
                // A look-ahead match is completed.
                let r = look_ahead[accepting];
                if r >= 0 {
                    return Some((r as usize, dict_count));
                }
            }
            let rule = self.row(state, 1);
            if rule > 1 {
                look_ahead[rule] = idx as i32;
            }
            if state == STOP_STATE {
                break;
            }
            if mode == Mode::Run {
                c = next32(text, &mut idx);
            } else if mode == Mode::Start {
                mode = Mode::Run;
            }
        }
        // The rules always match at least one character; if not, force
        // the iterator ahead by one.
        if result == initial {
            let mut i = initial;
            next32(text, &mut i);
            result = i;
        }
        Some((result, dict_count))
    }

    /// `DictionaryCache::populateDictionary` over `[start, end)`, a segment
    /// the rules found that holds dictionary characters: the boundaries the
    /// dictionary engines put in it, with `start` and `end`, or `None` when
    /// they put none.
    fn populate_dictionary(&self, text: &[u16], start: usize, end: usize) -> Option<Vec<usize>> {
        let mut breaks: Vec<usize> = vec![];
        let mut i = start;
        let mut c = current32(text, i);
        let mut category = self.trie.get(c);
        loop {
            while i < end && category < self.dict_start {
                next32(text, &mut i);
                c = current32(text, i);
                category = self.trie.get(c);
            }
            if i >= end {
                break;
            }
            // The engine for `c`'s script takes the run of characters of its
            // set (DictionaryBreakEngine::findBreaks); an unhandled script's
            // run gets no breaks.
            let script = sa_script(c);
            let run_start = i;
            loop {
                next32(text, &mut i);
                let d = current32(text, i);
                if i >= end || d == SENTINEL || sa_script(d) != script {
                    break;
                }
            }
            if let Some(script) = script.filter(|&s| s != SaScript::Other) {
                breaks.extend(
                    dictionary_breaks(script, &text[run_start..i])
                        .into_iter()
                        .map(|b| run_start + b),
                );
            }
            c = current32(text, i);
            category = self.trie.get(c);
        }
        if breaks.is_empty() {
            return None;
        }
        if start < breaks[0] {
            breaks.insert(0, start);
        }
        if end > *breaks.last().unwrap() {
            breaks.push(end);
        }
        Some(breaks)
    }
}

/// The script of LineBreak=SA character `c`, `None` for any other.
fn sa_script(c: i32) -> Option<SaScript> {
    let c = u32::try_from(c).ok()?;
    let k = sa::SA_RANGES.partition_point(|&(_, last, _)| last < c);
    sa::SA_RANGES
        .get(k)
        .filter(|&&(first, _, _)| first <= c)
        .map(|&(_, _, s)| s)
}

/// The word boundaries inside a run of one dictionary script, after its
/// start and before its end (`divideUpDictionaryRange`), from ICU4X's
/// dictionary segmenter. ICU's engines find none in a run too short for
/// two words: Thai one of at most 4 code points (`THAI_MIN_WORD_SPAN`,
/// counted by `utext_moveIndex32`), the others one of under 4 code units.
fn dictionary_breaks(script: SaScript, run: &[u16]) -> Vec<usize> {
    let too_short = match script {
        SaScript::Thai => {
            let mut i = 0;
            for _ in 0..4 {
                next32(run, &mut i);
            }
            i >= run.len()
        }
        _ => run.len() < 4,
    };
    if too_short {
        return vec![];
    }
    LineSegmenter::new_dictionary(LineBreakOptions::default())
        .segment_utf16(run)
        .filter(|&b| b > 0 && b < run.len())
        // ThaiBreakEngine's suffix rule: MAI YAMOK (U+0E46), where no
        // dictionary word starts, joins the word before it unless that
        // ends in MAI YAMOK too. (No Thai word starts with it.)
        .filter(|&b| !(script == SaScript::Thai && run[b] == 0x0E46 && run[b - 1] != 0x0E46))
        .collect()
}

/// The boundaries `ubrk_next` returns for `text` (UTF-16) after
/// `ubrk_setText`, in order (`BreakCache::populateFollowing`): every
/// line-break opportunity after the start, the end of the text last (none
/// for an empty text).
pub fn boundaries(rules: Rules, text: &[u16]) -> Vec<i32> {
    let rbbi = Rbbi::new(table(rules));
    // `fLookAheadMatches`: each look-ahead rule records its position
    // before its accepting state reads it.
    let mut look_ahead = vec![-1; rbbi.look_ahead_size];
    // The dictionary cache: the boundaries of the last segment the
    // dictionaries cut (`fBreaks`, from `fStart` to `fLimit`).
    let mut dict: Option<Vec<usize>> = None;
    let following = |dict: &Option<Vec<usize>>, pos: usize| -> Option<usize> {
        let b = dict.as_ref()?;
        if pos < b[0] || pos >= *b.last().unwrap() {
            return None;
        }
        b.iter().copied().find(|&x| x > pos)
    };
    let mut out = vec![];
    let mut pos = 0;
    loop {
        if let Some(b) = following(&dict, pos) {
            out.push(b as i32);
            pos = b;
            continue;
        }
        let Some((next, dict_chars)) = rbbi.handle_next(text, pos, &mut look_ahead) else {
            break;
        };
        // populateDictionary returns at once for a segment of one code
        // unit, keeping the cache; otherwise it starts the cache afresh.
        if dict_chars > 0 && next - pos > 1 {
            dict = rbbi.populate_dictionary(text, pos, next);
            if let Some(b) = following(&dict, pos) {
                out.push(b as i32);
                pos = b;
                continue;
            }
        }
        out.push(next as i32);
        pos = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn locales_choose_icus_brkitr_rules() {
        assert_eq!(rules_for_locale(b""), Rules::Line);
        assert_eq!(rules_for_locale(b"en"), Rules::Line);
        assert_eq!(rules_for_locale(b"th"), Rules::Line);
        assert_eq!(rules_for_locale(b"kh"), Rules::Line);
        assert_eq!(rules_for_locale(b"zh"), Rules::LineCj);
        assert_eq!(rules_for_locale(b"zh_Hant_TW"), Rules::LineCj);
        assert_eq!(rules_for_locale(b"zh-CN"), Rules::LineCj);
        assert_eq!(rules_for_locale(b"ja"), Rules::LineNormal);
        assert_eq!(rules_for_locale(b"ko_KR"), Rules::LineNormal);
        assert_eq!(rules_for_locale(b"ja@lb=strict"), Rules::LineCj);
        assert_eq!(rules_for_locale(b"ja@lb=loose"), Rules::LineLooseCj);
        assert_eq!(rules_for_locale(b"zh@lb=normal"), Rules::LineNormalCj);
        assert_eq!(rules_for_locale(b"en@lb=normal"), Rules::LineNormal);
        assert_eq!(rules_for_locale(b"en@lb=bogus"), Rules::Line);
    }

    #[test]
    fn every_table_loads() {
        for r in [
            Rules::Line,
            Rules::LineCj,
            Rules::LineNormal,
            Rules::LineNormalCj,
            Rules::LineLoose,
            Rules::LineLooseCj,
        ] {
            let t = Rbbi::new(table(r));
            assert!(t.look_ahead_size > 0 && t.row_len > 0, "{r:?}");
            // A, an ideograph and a Thai letter: different categories, the
            // Thai one a dictionary category.
            let (a, id, th) = (t.trie.get(0x41), t.trie.get(0x4E2D), t.trie.get(0x0E01));
            assert!(a != id && th >= t.dict_start && a < t.dict_start, "{r:?}");
        }
        assert_eq!(sa_script(0x0E01), Some(SaScript::Thai));
        assert_eq!(sa_script(0x0E3F), None);
        assert_eq!(sa_script(0x17D2), Some(SaScript::Khmer));
        assert_eq!(sa_script(0x1A20), Some(SaScript::Other));
        assert_eq!(sa_script(0x41), None);
    }

    /// The fixture's text, with its `\uXXXX` escapes.
    fn unescape(t: &str) -> Vec<u16> {
        let mut out = vec![];
        let mut rest = t;
        while let Some(i) = rest.find("\\u") {
            out.extend(rest[..i].encode_utf16());
            out.push(u16::from_str_radix(&rest[i + 2..i + 6], 16).unwrap());
            rest = &rest[i + 6..];
        }
        out.extend(rest.encode_utf16());
        out
    }

    /// The pieces TeX Live 2026's xetex (ICU 78.2) cuts native words into
    /// under `\XeTeXlinebreaklocale`, one word per line of
    /// `linebreak_xetex.tsv`: root, `zh`, `zh_TW`, `ja`, `ko`, the `lb`
    /// keyword, Thai, Lao, Khmer and Myanmar (ICU's dictionaries), Tibetan,
    /// Hebrew, Arabic, Devanagari, emoji and format characters.
    #[test]
    fn boundaries_as_xetex_cuts_words() {
        let mut bad = vec![];
        let mut n = 0;
        for line in include_str!("linebreak_xetex.tsv").lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let want: Vec<i32> = f[2].split(',').map(|x| x.parse().unwrap()).collect();
            let got = boundaries(rules_for_locale(f[0].as_bytes()), &unescape(f[1]));
            n += 1;
            if got != want {
                bad.push(format!(
                    "{} {:?}: xetex {:?}, here {:?}",
                    f[0], f[1], want, got
                ));
            }
        }
        assert!(n >= 30, "{n} fixture lines");
        assert!(
            bad.is_empty(),
            "{} of {n} differ:\n{}",
            bad.len(),
            bad.join("\n")
        );
        assert_eq!(boundaries(Rules::Line, &[]), Vec::<i32>::new());
        assert_eq!(boundaries(Rules::LineCj, &u("ab")), vec![2]);
        // a lone surrogate is one character
        assert_eq!(boundaries(Rules::Line, &[0xD800]), vec![1]);
    }
}
