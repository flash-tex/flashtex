//! ICU 78.2's data converters (`.cnv` files of type MBCS, vendored in
//! third_party/icu/cnv) to Unicode, for `crate::ucnv`: ucnvmbcs.cpp's
//! `ucnv_MBCSLoad` (with extension-only files and their base tables, and
//! the DBCS-only state of an extension of an SI/SO or 1/2-byte base),
//! `_EBCDICSwapLFNL` and `ucnv_MBCSOpen`, `ucnv_MBCSToUnicodeWithOffsets`
//! and its single-byte variants, `_extToU` with GB 18030's four-byte
//! ranges, and ucnv_ext.cpp's `ucnv_extInitialMatchToU`. The input is a
//! whole line with `flush` set, so a partial extension match never waits
//! for more input.

use crate::ucnv::{Pivot, OPTION_SWAP_LFNL};
use std::sync::OnceLock;

/// `MBCS_STATE_*` actions of a final state table entry.
const STATE_VALID_DIRECT_16: u32 = 0;
const STATE_VALID_DIRECT_20: u32 = 1;
const STATE_FALLBACK_DIRECT_16: u32 = 2;
const STATE_FALLBACK_DIRECT_20: u32 = 3;
const STATE_VALID_16: u32 = 4;
const STATE_VALID_16_PAIR: u32 = 5;
const STATE_UNASSIGNED: u32 = 6;
const STATE_ILLEGAL: u32 = 7;
const STATE_CHANGE_ONLY: u32 = 8;

/// `MBCS_OUTPUT_*`.
const OUTPUT_1: u8 = 0;
const OUTPUT_2_SISO: u8 = 12;
const OUTPUT_EXT_ONLY: u8 = 14;
const OUTPUT_DBCS_ONLY: u8 = 0xdb;

/// `_MBCS_OPTION_GB18030`: set by `ucnv_MBCSOpen` for a GB 18030 name.
pub(crate) const OPTION_GB18030: u32 = 0x8000;

/// `U_INVALID_FORMAT_ERROR`, `U_INVALID_TABLE_FORMAT`.
const U_INVALID_FORMAT_ERROR: i32 = 3;
const U_INVALID_TABLE_FORMAT: i32 = 13;

/// `MBCS_ENTRY_FINAL(0, MBCS_STATE_VALID_DIRECT_16, value)`.
const fn final_direct_16(value: u32) -> i32 {
    (0x8000_0000u32 | (STATE_VALID_DIRECT_16 << 20) | value) as i32
}

/// `MBCS_ENTRY_FINAL(0, MBCS_STATE_ILLEGAL, 0)`.
const FINAL_ILLEGAL: i32 = (0x8000_0000u32 | (STATE_ILLEGAL << 20)) as i32;

fn is_transition(entry: i32) -> bool {
    entry >= 0
}

fn transition_state(entry: i32) -> usize {
    (entry as u32 >> 24) as usize
}

fn final_state(entry: i32) -> u8 {
    ((entry as u32 >> 24) & 0x7f) as u8
}

fn final_action(entry: i32) -> u32 {
    (entry as u32 >> 20) & 0xf
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// The parts of a `.cnv` item: its `UConverterStaticData` fields and the
/// offsets of its `_MBCSHeader` (relative to the item).
struct Item {
    bytes: &'static [u8],
    format_version: [u8; 2],
    conversion_type: u8,
    min_bytes_per_char: u8,
    unicode_mask: u8,
    sub_char1: u8,
    /// The `_MBCSHeader` (`raw` in `ucnv_MBCSLoad`).
    raw: usize,
    header_length: usize,
}

impl Item {
    fn parse(bytes: &'static [u8]) -> Result<Item, i32> {
        let header_size = u16_at(bytes, 0) as usize;
        let sd = header_size;
        let struct_size = u32_at(bytes, sd) as usize;
        let raw = sd + struct_size;
        let version = [bytes[raw], bytes[raw + 1]];
        let options = u32_at(bytes, raw + 32);
        let header_length = if version[0] == 4 {
            8
        } else if version[0] == 5 && version[1] >= 3 && options & 0xffc0_0000 == 0 {
            // MBCS_OPT_UNKNOWN_INCOMPATIBLE_MASK; no file of TeX Live's data
            // has version 5 (MBCS_OPT_NO_FROM_U would need reconstitution)
            if options & 0x8000 != 0 {
                return Err(U_INVALID_TABLE_FORMAT);
            }
            (options & 0x3f) as usize
        } else {
            return Err(U_INVALID_TABLE_FORMAT);
        };
        Ok(Item {
            bytes,
            format_version: [bytes[16], bytes[17]],
            conversion_type: bytes[sd + 69],
            min_bytes_per_char: bytes[sd + 70],
            unicode_mask: bytes[sd + 79],
            sub_char1: bytes[sd + 80],
            raw,
            header_length,
        })
    }

    fn header(&self, field: usize) -> u32 {
        u32_at(self.bytes, self.raw + 4 + 4 * field)
    }

    fn count_states(&self) -> usize {
        self.header(0) as usize
    }

    fn flags(&self) -> u32 {
        self.header(5)
    }
}

/// A loaded data converter: `UConverterMBCSTable` as `ucnv_MBCSLoad` leaves
/// it, for converting to Unicode.
pub(crate) struct Table {
    /// `stateTable` (a copy: a DBCS-only extension changes its base's).
    states: Vec<[i32; 256]>,
    /// `swapLFNLStateTable`, when `swaplfnl` applies.
    swap_states: Option<Vec<[i32; 256]>>,
    /// `_EBCDICSwapLFNL`: whether `swaplfnl` applies, or its error.
    pub(crate) swap_lfnl: Result<bool, i32>,
    /// `toUFallbacks`: (offset, code point), sorted by offset.
    fallbacks: Vec<(u32, u32)>,
    /// The item holding `unicodeCodeUnits` (the base table's for an
    /// extension-only file), and their offset in it.
    code_units: (&'static [u8], usize),
    output_type: u8,
    dbcs_only_state: u8,
    /// `unicodeMask & UCNV_HAS_SUPPLEMENTARY`.
    supplementary: bool,
    /// `staticData->subChar1` of the converter opened.
    pub(crate) sub_char1: u8,
    /// `extIndexes`: the extension data from its indexes on.
    ext: Option<&'static [u8]>,
}

/// The table of the data converter `i` (`crate::ucnv_data::DATA`), loaded
/// once.
pub(crate) fn table(i: u16) -> Result<&'static Table, i32> {
    static TABLES: OnceLock<Vec<OnceLock<Result<Table, i32>>>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        (0..crate::ucnv_data::DATA.len())
            .map(|_| OnceLock::new())
            .collect()
    });
    tables[i as usize]
        .get_or_init(|| Table::load(crate::ucnv_data::DATA[i as usize].1))
        .as_ref()
        .map_err(|&e| e)
}

/// The vendored item named `name` (`udata_openChoice`).
pub(crate) fn find(name: &[u8]) -> Option<u16> {
    crate::ucnv_data::DATA
        .binary_search_by(|(n, _)| n.as_bytes().cmp(name))
        .ok()
        .map(|i| i as u16)
}

impl Table {
    fn load(bytes: &'static [u8]) -> Result<Table, i32> {
        let item = Item::parse(bytes)?;
        let flags = item.flags();
        let output_type = (flags & 0xff) as u8;
        let ext_offset = (flags >> 8) as usize;
        let ext = (ext_offset != 0).then(|| &bytes[item.raw + ext_offset..]);
        if output_type == OUTPUT_EXT_ONLY {
            let ext = ext.ok_or(U_INVALID_TABLE_FORMAT)?;
            // the base table's name follows the header
            let at = item.raw + item.header_length * 4;
            let end = bytes[at..].iter().position(|&b| b == 0).unwrap_or(0);
            let base_name = &bytes[at..at + end];
            let sd = u16_at(bytes, 0) as usize;
            let own = &bytes[sd + 4..sd + 64];
            let own_name = &own[..own.iter().position(|&b| b == 0).unwrap_or(own.len())];
            if base_name == own_name {
                return Err(U_INVALID_TABLE_FORMAT);
            }
            let base_bytes = crate::ucnv_data::DATA
                [find(base_name).ok_or(crate::ucnv::U_FILE_ACCESS_ERROR)? as usize]
                .1;
            let base = Item::parse(base_bytes)?;
            let base_type = (base.flags() & 0xff) as u8;
            if base.conversion_type != 2 || base_type == OUTPUT_EXT_ONLY {
                return Err(U_INVALID_TABLE_FORMAT);
            }
            let mut t = Table::from_base(&base)?;
            t.ext = Some(ext);
            t.sub_char1 = item.sub_char1;
            // a DBCS version of a base table that also maps single bytes
            if item.conversion_type == 1
                || (item.conversion_type == 2 && item.min_bytes_per_char >= 2)
            {
                if t.output_type == OUTPUT_2_SISO {
                    // the DBCS state from the entry for SO=0x0e
                    let entry = t.states[0][0x0e];
                    if !is_transition(entry)
                        && final_action(entry) == STATE_CHANGE_ONLY
                        && final_state(entry) != 0
                    {
                        t.dbcs_only_state = final_state(entry);
                        t.output_type = OUTPUT_DBCS_ONLY;
                    }
                } else if base.conversion_type == 2
                    && base_bytes[u16_at(base_bytes, 0) as usize + 70] == 1
                    && base_bytes[u16_at(base_bytes, 0) as usize + 71] == 2
                    && t.states.len() <= 127
                {
                    // every final single-byte entry goes to a new all-illegal state
                    let count = t.states.len();
                    for e in t.states[0].iter_mut() {
                        if !is_transition(*e) {
                            *e = (count as i32) << 24;
                        }
                    }
                    t.states.push([FINAL_ILLEGAL; 256]);
                    t.output_type = OUTPUT_DBCS_ONLY;
                }
            }
            t.swap_lfnl = swap_lfnl(&t, &base);
            t.swap_states = swapped(&t);
            return Ok(t);
        }
        match output_type {
            0 | 1 | 2 | 3 | 8 | 9 | OUTPUT_2_SISO => {}
            _ => return Err(U_INVALID_TABLE_FORMAT),
        }
        let mut t = Table::from_base(&item)?;
        t.ext = ext;
        t.swap_lfnl = swap_lfnl(&t, &item);
        t.swap_states = swapped(&t);
        Ok(t)
    }

    /// A conversion file with a base table.
    fn from_base(item: &Item) -> Result<Table, i32> {
        let b = item.bytes;
        let count_states = item.count_states();
        let count_fallbacks = item.header(1) as usize;
        let offset_to_u_code_units = item.header(2) as usize;
        let st = item.raw + item.header_length * 4;
        let mut states = Vec::with_capacity(count_states);
        for s in 0..count_states {
            let mut row = [0i32; 256];
            for (k, e) in row.iter_mut().enumerate() {
                *e = u32_at(b, st + 1024 * s + 4 * k) as i32;
            }
            states.push(row);
        }
        let fb = st + 1024 * count_states;
        let fallbacks = (0..count_fallbacks)
            .map(|k| (u32_at(b, fb + 8 * k), u32_at(b, fb + 8 * k + 4)))
            .collect();
        // converter versions 6.1 and up hold a unicodeMask
        let fv = item.format_version;
        let mask = if fv[0] > 6 || (fv[0] == 6 && fv[1] >= 1) {
            item.unicode_mask & 3
        } else {
            3
        };
        Ok(Table {
            states,
            swap_states: None,
            swap_lfnl: Ok(false),
            fallbacks,
            code_units: (b, item.raw + offset_to_u_code_units),
            output_type: (item.flags() & 0xff) as u8,
            dbcs_only_state: 0,
            supplementary: mask & 1 != 0,
            sub_char1: item.sub_char1,
            ext: None,
        })
    }

    fn code_unit(&self, offset: u32) -> u16 {
        u16_at(self.code_units.0, self.code_units.1 + 2 * offset as usize)
    }

    /// `ucnv_MBCSGetFallback`.
    fn fallback(&self, offset: u32) -> u32 {
        let fb = &self.fallbacks;
        if !fb.is_empty() {
            let (mut start, mut limit) = (0usize, fb.len());
            while start < limit - 1 {
                let i = (start + limit) / 2;
                if offset < fb[i].0 {
                    limit = i;
                } else {
                    start = i;
                }
            }
            if offset == fb[start].0 {
                return fb[start].1;
            }
        }
        0xfffe
    }

    fn is_single_byte(&self) -> bool {
        self.states.len() == 1
    }
}

/// `_EBCDICSwapLFNL`'s test: an EBCDIC table (SBCS, or SI/SO stateful) with
/// the standard LF and NL mappings, both ways (`item` holds the
/// from-Unicode tables).
fn swap_lfnl(t: &Table, item: &Item) -> Result<bool, i32> {
    if !((t.output_type == OUTPUT_1 || t.output_type == OUTPUT_2_SISO)
        && t.states[0][0x25] == final_direct_16(0x0a)
        && t.states[0][0x15] == final_direct_16(0x85))
    {
        return Ok(false);
    }
    let b = item.bytes;
    let table = item.raw + item.header(3) as usize;
    let bytes = item.raw + item.header(4) as usize;
    let stage1 = |c: usize| u16_at(b, table + 2 * (c >> 10)) as usize;
    if t.output_type == OUTPUT_1 {
        // MBCS_SINGLE_RESULT_FROM_U
        let result = |c: usize| {
            let r = u16_at(b, table + 2 * (stage1(c) + ((c >> 4) & 0x3f))) as usize + (c & 0xf);
            u16_at(b, bytes + 2 * r)
        };
        if !(result(0x0a) == 0xf25 && result(0x85) == 0xf15) {
            return Ok(false);
        }
    } else {
        // MBCS_STAGE_2_FROM_U, MBCS_FROM_U_IS_ROUNDTRIP, MBCS_VALUE_2_FROM_STAGE_2
        let ok = |c: usize, want: u16| {
            let stage2 = u32_at(b, table + 4 * (stage1(c) + ((c >> 4) & 0x3f)));
            stage2 & (1 << (16 + (c & 0xf))) != 0
                && u16_at(b, bytes + 2 * (16 * (stage2 & 0xffff) as usize + (c & 0xf))) == want
        };
        if !(ok(0x0a, 0x25) && ok(0x85, 0x15)) {
            return Ok(false);
        }
    }
    if item.header(6) > 0 {
        Ok(true)
    } else {
        Err(U_INVALID_FORMAT_ERROR)
    }
}

/// The state table with LF and NL swapped, if `swaplfnl` applies.
fn swapped(t: &Table) -> Option<Vec<[i32; 256]>> {
    if t.swap_lfnl != Ok(true) {
        return None;
    }
    let mut s = t.states.clone();
    s[0][0x25] = final_direct_16(0x85);
    s[0][0x15] = final_direct_16(0x0a);
    Some(s)
}

/// `ucnv_MBCSOpen`'s options for a table: `swaplfnl` kept only where it
/// applies (or the open fails), and the GB 18030 flag for a name holding
/// `gb18030`.
pub(crate) fn open_options(t: &Table, name: &[u8], mut options: u32) -> Result<u32, i32> {
    if t.output_type == OUTPUT_DBCS_ONLY {
        options &= !OPTION_SWAP_LFNL;
    }
    if options & OPTION_SWAP_LFNL != 0 {
        match t.swap_lfnl {
            Ok(true) => {}
            Ok(false) => options &= !OPTION_SWAP_LFNL,
            Err(e) => return Err(e),
        }
    }
    let has = |s: &[u8]| name.windows(s.len()).any(|w| w == s);
    if has(b"18030") && (has(b"gb18030") || has(b"GB18030")) {
        options |= OPTION_GB18030;
    }
    Ok(options)
}

/// `hasValidTrailBytes`.
fn has_valid_trail_bytes(states: &[[i32; 256]], state: usize) -> bool {
    let row = &states[state];
    let ok = |e: i32| !is_transition(e) && final_action(e) != STATE_ILLEGAL;
    if ok(row[0xa1]) || ok(row[0x41]) || row.iter().any(|&e| ok(e)) {
        return true;
    }
    row.iter()
        .any(|&e| is_transition(e) && has_valid_trail_bytes(states, transition_state(e)))
}

/// `isSingleOrLead`.
fn is_single_or_lead(states: &[[i32; 256]], state: usize, dbcs_only: bool, b: u8) -> bool {
    let entry = states[state][b as usize];
    if is_transition(entry) {
        has_valid_trail_bytes(states, transition_state(entry))
    } else {
        let action = final_action(entry);
        if action == STATE_CHANGE_ONLY && dbcs_only {
            false
        } else {
            action != STATE_ILLEGAL
        }
    }
}

// ---------------------------------------------------------------------------
// Extension tables: ucnv_ext.cpp
// ---------------------------------------------------------------------------

/// `UCNV_EXT_TO_U_*`.
const EXT_TO_U_INDEX: usize = 1;
const EXT_TO_U_LENGTH: usize = 2;
const EXT_TO_U_UCHARS_INDEX: usize = 3;
const EXT_TO_U_MIN_CODE_POINT: u32 = 0x1f0000;
const EXT_TO_U_MAX_CODE_POINT: u32 = 0x2fffff;
const EXT_TO_U_ROUNDTRIP_FLAG: u32 = 1 << 23;

fn cx_index(cx: &[u8], i: usize) -> i32 {
    u32_at(cx, 4 * i) as i32
}

/// `ucnv_extFindToU`: the value for `byte` in a section of `length` words
/// from word `at` of the to-Unicode table, or 0.
fn ext_find_to_u(cx: &[u8], table: usize, at: usize, length: usize, byte: u8) -> u32 {
    let word = |k: usize| u32_at(cx, table + 4 * (at + k));
    let start = word(0) >> 24;
    let limit = word(length - 1) >> 24;
    let b = byte as u32;
    if b < start || limit < b {
        return 0;
    }
    if length == (limit - start + 1) as usize {
        // direct access on a linear array
        return word((b - start) as usize) & 0xffffff;
    }
    // the section's bytes are sorted and distinct
    (0..length)
        .map(word)
        .find(|w| w >> 24 == b)
        .map_or(0, |w| w & 0xffffff)
}

/// `UCNV_EXT_TO_U_VERIFY_SISO_MATCH`.
fn verify_siso_match(siso_state: i8, len: usize) -> bool {
    siso_state < 0 || (siso_state == 0) == (len == 1)
}

/// `ucnv_extMatchToU` with `flush`: the longest match of `pre` then `src`,
/// as (length, value).
fn ext_match_to_u(cx: &[u8], siso_state: i8, pre: &[u8], src: &[u8]) -> Option<(usize, u32)> {
    if cx_index(cx, EXT_TO_U_LENGTH) <= 0 {
        return None;
    }
    let table = cx_index(cx, EXT_TO_U_INDEX) as usize;
    let mut src_length = src.len();
    if siso_state == 0 {
        // SBCS state of an SI/SO stateful converter: exactly 1 byte
        if pre.len() > 1 {
            return None;
        } else if pre.len() == 1 {
            src_length = 0;
        } else if src_length > 1 {
            src_length = 1;
        }
    }
    let (mut idx, mut i, mut j) = (0usize, 0usize, 0usize);
    let (mut match_value, mut match_length) = (0u32, 0usize);
    loop {
        let first = u32_at(cx, table + 4 * idx);
        let length = (first >> 24) as usize;
        let value = first & 0xffffff;
        // TO_U_USE_FALLBACK is true: every value counts
        if value != 0 && verify_siso_match(siso_state, i + j) {
            match_value = value;
            match_length = i + j;
        }
        let b = if i < pre.len() {
            i += 1;
            pre[i - 1]
        } else if j < src_length {
            j += 1;
            src[j - 1]
        } else {
            // all input consumed: with flush, the longest match so far
            break;
        };
        let value = ext_find_to_u(cx, table, idx + 1, length, b);
        if value == 0 {
            break;
        }
        if value < EXT_TO_U_MIN_CODE_POINT {
            // partial match, continue
            idx = value as usize;
        } else {
            if verify_siso_match(siso_state, i + j) {
                match_value = value;
                match_length = i + j;
            }
            break;
        }
    }
    (match_length != 0).then_some((match_length, match_value & !EXT_TO_U_ROUNDTRIP_FLAG))
}

/// `ucnv_extWriteToU`.
fn ext_write_to_u(cx: &[u8], value: u32, p: &mut Pivot) {
    if value <= EXT_TO_U_MAX_CODE_POINT {
        p.push(value - EXT_TO_U_MIN_CODE_POINT);
    } else {
        let uchars = cx_index(cx, EXT_TO_U_UCHARS_INDEX) as usize;
        let index = (value & 0x3ffff) as usize;
        let length = ((value >> 18) - 12) as usize;
        for k in 0..length {
            p.units.push(u16_at(cx, uchars + 2 * (index + k)));
        }
    }
}

/// `UCNV_SISO_STATE`.
fn siso_state(t: &Table, mode: u8) -> i8 {
    match t.output_type {
        OUTPUT_2_SISO => mode as i8,
        OUTPUT_DBCS_ONLY => 1,
        _ => -1,
    }
}

/// GB 18030's four-byte ranges mapped algorithmically
/// (`gb18030Ranges`, `LINEAR`).
const fn linear(x: u32) -> u32 {
    let (a, b, c, d) = (x >> 24, (x >> 16) & 0xff, (x >> 8) & 0xff, x & 0xff);
    ((a * 10 + b) * 126 + c) * 10 + d
}

const GB18030_RANGES: [[u32; 4]; 14] = [
    [0x10000, 0x10FFFF, linear(0x90308130), linear(0xE3329A35)],
    [0x9FA6, 0xD7FF, linear(0x82358F33), linear(0x8336C738)],
    [0x0452, 0x1E3E, linear(0x8130D330), linear(0x8135F436)],
    [0x1E40, 0x200F, linear(0x8135F438), linear(0x8136A531)],
    [0xE865, 0xF92B, linear(0x8336D030), linear(0x84308534)],
    [0x2643, 0x2E80, linear(0x8137A839), linear(0x8138FD38)],
    [0xFA2A, 0xFE2F, linear(0x84309C38), linear(0x84318537)],
    [0x3CE1, 0x4055, linear(0x8231D438), linear(0x8232AF32)],
    [0x361B, 0x3917, linear(0x8230A633), linear(0x8230F237)],
    [0x49B8, 0x4C76, linear(0x8234A131), linear(0x8234E733)],
    [0x4160, 0x4336, linear(0x8232C937), linear(0x8232F837)],
    [0x478E, 0x4946, linear(0x8233E838), linear(0x82349638)],
    [0x44D7, 0x464B, linear(0x8233A339), linear(0x8233C931)],
    [0xFFE6, 0xFFFF, linear(0x8431A234), linear(0x8431A439)],
];

/// `_extToU`: an extension mapping of the unassigned sequence `bytes` (and
/// maybe more of `src` from `*s` on), or GB 18030's; true if one is
/// written.
fn ext_to_u(
    t: &Table,
    options: u32,
    mode: u8,
    bytes: &[u8],
    src: &[u8],
    s: &mut usize,
    p: &mut Pivot,
) -> bool {
    if let Some(cx) = t.ext {
        if let Some((len, value)) = ext_match_to_u(cx, siso_state(t, mode), bytes, &src[*s..]) {
            // the match may end before the first sequence does
            *s = *s + len - bytes.len();
            ext_write_to_u(cx, value, p);
            return true;
        }
    }
    if bytes.len() == 4 && options & OPTION_GB18030 != 0 {
        let l = linear(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
        if let Some(r) = GB18030_RANGES.iter().find(|r| r[2] <= l && l <= r[3]) {
            p.push(r[0] + (l - r[2]));
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// To Unicode
// ---------------------------------------------------------------------------

/// `ucnv_MBCSToUnicodeWithOffsets` over the whole line, flushed, with the
/// substitute callback for each error and a truncated sequence at the end.
pub(crate) fn to_u(t: &Table, options: u32, src: &[u8], p: &mut Pivot) {
    p.sub_char1 = t.sub_char1;
    let states: &[[i32; 256]] = match (&t.swap_states, options & OPTION_SWAP_LFNL != 0) {
        (Some(s), true) => s,
        _ => &t.states,
    };
    if t.is_single_byte() {
        single_to_u(t, states, options, src, p);
        return;
    }
    let mut s = 0usize;
    // toUBytes[0..byteIndex]
    let mut bytes: Vec<u8> = Vec::with_capacity(4);
    let mut offset: u32 = 0;
    let mut state: usize = t.dbcs_only_state as usize;
    while s < src.len() {
        let b = src[s];
        s += 1;
        bytes.push(b);
        let entry = states[state][b as usize];
        if is_transition(entry) {
            state = transition_state(entry);
            offset += entry as u32 & 0xffffff;
            continue;
        }
        // `cnv->mode`: the state the sequence's last byte was read in
        let mode = state as u8;
        state = final_state(entry) as usize;
        let mut done = false;
        let mut illegal = false;
        match final_action(entry) {
            STATE_VALID_16 => {
                offset += entry as u32 & 0xffff;
                let c = t.code_unit(offset);
                if c < 0xfffe {
                    p.units.push(c);
                    done = true;
                } else if c == 0xfffe {
                    let f = t.fallback(offset);
                    if f != 0xfffe {
                        p.units.push(f as u16);
                        done = true;
                    }
                } else {
                    illegal = true;
                }
            }
            STATE_VALID_DIRECT_16 | STATE_FALLBACK_DIRECT_16 => {
                p.units.push(entry as u16);
                done = true;
            }
            STATE_VALID_16_PAIR => {
                offset += entry as u32 & 0xffff;
                let c = t.code_unit(offset);
                offset += 1;
                if c < 0xd800 {
                    p.units.push(c);
                    done = true;
                } else if c <= 0xdfff {
                    // a roundtrip or fallback surrogate pair
                    p.units.push(c & 0xdbff);
                    p.units.push(t.code_unit(offset));
                    done = true;
                } else if c & 0xfffe == 0xe000 {
                    p.units.push(t.code_unit(offset));
                    done = true;
                } else if c == 0xffff {
                    illegal = true;
                }
            }
            STATE_VALID_DIRECT_20 | STATE_FALLBACK_DIRECT_20 => {
                p.push(0x10000 + (entry as u32 & 0xfffff));
                done = true;
            }
            STATE_CHANGE_ONLY => {
                if t.dbcs_only_state == 0 {
                    done = true;
                } else {
                    // SI/SO are illegal for DBCS-only conversion
                    state = mode as usize;
                    illegal = true;
                }
            }
            STATE_UNASSIGNED => {}
            STATE_ILLEGAL => illegal = true,
            // reserved
            _ => done = true,
        }
        offset = 0;
        if done {
            bytes.clear();
            continue;
        }
        if illegal {
            if bytes.len() > 1 {
                // Ticket 5691: stop the illegal sequence before the first
                // non-initial byte that could start a character
                let dbcs_only = t.dbcs_only_state != 0;
                let mut i = 1;
                while i < bytes.len() && !is_single_or_lead(states, state, dbcs_only, bytes[i]) {
                    i += 1;
                }
                if i < bytes.len() {
                    s -= bytes.len() - i;
                    bytes.truncate(i);
                }
            }
            p.sub(bytes.len());
        } else if !ext_to_u(t, options, mode, &bytes, src, &mut s, p) {
            // unassigned: U_INVALID_CHAR_FOUND over the sequence
            p.sub(bytes.len());
        }
        bytes.clear();
        // the conversion resumes from `cnv->mode`
        if state == 0 {
            state = t.dbcs_only_state as usize;
        }
    }
    if !bytes.is_empty() {
        // U_TRUNCATED_CHAR_FOUND
        p.sub(bytes.len());
    }
}

/// `ucnv_MBCSSingleToBMPWithOffsets`, or `ucnv_MBCSSingleToUnicodeWithOffsets`
/// for a table with supplementary code points.
fn single_to_u(t: &Table, states: &[[i32; 256]], options: u32, src: &[u8], p: &mut Pivot) {
    let row = &states[0];
    let mut s = 0usize;
    while s < src.len() {
        let b = src[s];
        s += 1;
        let entry = row[b as usize];
        // MBCS_ENTRY_FINAL_IS_VALID_DIRECT_16
        if entry < 0x8010_0000u32 as i32 {
            p.units.push(entry as u16);
            continue;
        }
        let action = final_action(entry);
        if t.supplementary
            && (action == STATE_VALID_DIRECT_20 || action == STATE_FALLBACK_DIRECT_20)
        {
            p.push(0x10000 + (entry as u32 & 0xfffff));
        } else if action == STATE_FALLBACK_DIRECT_16 {
            p.units.push(entry as u16);
        } else if action == STATE_UNASSIGNED {
            // `_extToU` over the byte (toUBytes[0], toULength 1)
            if !ext_to_u(t, options, 0, &[b], src, &mut s, p) {
                p.sub(1);
            }
        } else if action == STATE_ILLEGAL {
            // U_ILLEGAL_CHAR_FOUND with toULength 0
            p.sub(0);
        }
        // else reserved: the byte is skipped
    }
}
