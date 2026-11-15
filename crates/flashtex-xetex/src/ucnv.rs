//! ICU 78.2's character set converters as XeTeX uses them (issue #2382):
//! `\XeTeXinputencoding` and `\XeTeXdefaultencoding` with a name that is not
//! one of XeTeX's own open an ICU converter (`ucnv_open`), and XeTeX_ext.c's
//! `input_line` turns each line's bytes into characters with
//! `ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, cnv, ...)`.
//!
//! The port follows ICU's sources (TeX Live 2026's `libs/icu/icu-src`,
//! ICU 78.2) routine by routine:
//!
//! * [`open`]: `ucnv_open` → `ucnv_loadSharedData` (ucnv_bld.cpp): the UTF-8
//!   fast path (`UCNV_FAST_IS_UTF8`), `parseConverterOptions`, the alias
//!   table (`ucnv_io_getConverterName`, `findConverter`,
//!   `ucnv_io_stripASCIIForCompare` in ucnv_io.cpp, over ICU's own
//!   `cnvalias.icu`), `getAlgorithmicTypeFromName`, then a data converter
//!   loaded by its exact name, and the converter's `open`.
//! * [`to_unicode`]: `ucnv_toAlgorithmic` → `ucnv_convertEx` (ucnv.cpp): the
//!   converter is reset, the whole line is converted with `flush`, through
//!   ICU's UTF-16 pivot to UTF-32, with the default callbacks
//!   (`UCNV_TO_U_CALLBACK_SUBSTITUTE`, `ucnv_cbToUWriteSub`; and
//!   `UCNV_FROM_U_CALLBACK_SUBSTITUTE` into UTF-32, whose substitute is
//!   U+FFFD).
//!
//! Converters ported: UTF-8 and CESU-8 (ucnv_u8.cpp), ISO-8859-1 and US-ASCII
//! (ucnvlat1.cpp), the UTF-16 and UTF-32 families with their BOM and
//! `version` handling (ucnv_u16.cpp, ucnv_u32.cpp), and every table of TeX
//! Live's ICU data, single- and multi-byte (`crate::ucnv_mbcs`), whose
//! `.cnv` files are vendored unmodified in third_party/icu/cnv. Not ported
//! (S1): ICU's algorithmic UTF-7, IMAP-mailbox-name, SCSU, BOCU-1, ISO-2022,
//! HZ, LMBCS, ISCII and x11-compound-text: [`open`] reports such a name as
//! [`Opened::Unported`].

use std::sync::OnceLock;

/// `UCNV_MAX_CONVERTER_NAME_LENGTH`.
const MAX_CONVERTER_NAME_LENGTH: usize = 60;
/// `UCNV_OPTION_VERSION`, `UCNV_OPTION_SWAP_LFNL` (ucnv_bld.h).
const OPTION_VERSION: u32 = 0xf;
pub(crate) const OPTION_SWAP_LFNL: u32 = 0x10;

/// ICU's error codes that [`open`] gives (utypes.h).
pub const U_ILLEGAL_ARGUMENT_ERROR: i32 = 1;
pub const U_FILE_ACCESS_ERROR: i32 = 4;
/// `U_BUFFER_OVERFLOW_ERROR`, `U_STRING_NOT_TERMINATED_WARNING`.
pub const U_BUFFER_OVERFLOW_ERROR: i32 = 15;
pub const U_STRING_NOT_TERMINATED_WARNING: i32 = -124;

/// A converter's implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Utf8,
    Cesu8,
    Latin1,
    Ascii,
    Utf16Be,
    Utf16Le,
    /// `UTF-16` (with a byte order mark, big-endian without).
    Utf16,
    Utf32Be,
    Utf32Le,
    Utf32,
    /// A data converter (a `.cnv` table): its index in
    /// [`crate::ucnv_data::DATA`].
    Data(u16),
}

/// An open converter (`UConverter`): its implementation and `options`. It
/// keeps no state between lines: `ucnv_toAlgorithmic` resets it each time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Converter {
    kind: Kind,
    options: u32,
}

/// What `ucnv_open` gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    Converter(Converter),
    /// `ucnv_open` fails, with this `UErrorCode`.
    Failed(i32),
    /// ICU opens a converter this port does not have (see the module
    /// documentation).
    Unported,
}

impl Converter {
    /// The converter as one number, for a checkpoint (`FileSnap`).
    pub fn to_id(self) -> i32 {
        let k = match self.kind {
            Kind::Utf8 => 0,
            Kind::Cesu8 => 1,
            Kind::Latin1 => 2,
            Kind::Ascii => 3,
            Kind::Utf16Be => 4,
            Kind::Utf16Le => 5,
            Kind::Utf16 => 6,
            Kind::Utf32Be => 7,
            Kind::Utf32Le => 8,
            Kind::Utf32 => 9,
            Kind::Data(i) => 16 + i as i32,
        };
        (k << 16) | (self.options & 0xffff) as i32
    }

    /// The converter [`Converter::to_id`] gave `id`.
    pub fn from_id(id: i32) -> Option<Converter> {
        if id < 0 {
            return None;
        }
        let kind = match id >> 16 {
            0 => Kind::Utf8,
            1 => Kind::Cesu8,
            2 => Kind::Latin1,
            3 => Kind::Ascii,
            4 => Kind::Utf16Be,
            5 => Kind::Utf16Le,
            6 => Kind::Utf16,
            7 => Kind::Utf32Be,
            8 => Kind::Utf32Le,
            9 => Kind::Utf32,
            k if k >= 16 && ((k - 16) as usize) < crate::ucnv_data::DATA.len() => {
                Kind::Data((k - 16) as u16)
            }
            _ => return None,
        };
        Some(Converter {
            kind,
            options: (id & 0xffff) as u32,
        })
    }

    fn version(self) -> u32 {
        self.options & OPTION_VERSION
    }
}

// ---------------------------------------------------------------------------
// Names: ucnv_io.cpp
// ---------------------------------------------------------------------------

/// `GET_ASCII_TYPE`'s classes (`asciiTypes`): 0 ignored, 1 the digit zero,
/// 2 another digit, else the lowercase letter.
fn ascii_type(c: u8) -> u8 {
    match c {
        b'0' => 1,
        b'1'..=b'9' => 2,
        b'a'..=b'z' => c,
        b'A'..=b'Z' => c + 32,
        // other ASCII, and every byte from 0x80 (`(int8_t)c < 0`)
        _ => 0,
    }
}

/// `ucnv_io_stripASCIIForCompare`: the name lowercased, with every
/// character but letters and digits left out, and a zero left out that
/// begins a number and is followed by another digit.
fn strip_for_compare(name: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(name.len());
    let mut after_digit = false;
    for (i, &c) in name.iter().enumerate() {
        match ascii_type(c) {
            0 => {
                after_digit = false;
                continue;
            }
            1 => {
                if !after_digit {
                    let next = name.get(i + 1).map_or(0, |&n| ascii_type(n));
                    if next == 1 || next == 2 {
                        continue;
                    }
                }
                out.push(c);
            }
            2 => {
                after_digit = true;
                out.push(c);
            }
            t => {
                after_digit = false;
                out.push(t);
            }
        }
    }
    out
}

/// ICU's alias table, `cnvalias.icu` from TeX Live 2026's ICU data
/// (`initAliasData`'s view of it).
struct AliasTable {
    data: &'static [u8],
    converter_list: usize,
    converter_list_size: usize,
    alias_list: usize,
    untagged_conv_array: usize,
    untagged_conv_array_size: usize,
    contains_cnv_option_info: bool,
    normalized: bool,
    string_table: usize,
    normalized_string_table: usize,
}

static CNVALIAS: &[u8] = include_bytes!("../../../third_party/icu/cnv/cnvalias.icu");

impl AliasTable {
    fn get() -> &'static AliasTable {
        static T: OnceLock<AliasTable> = OnceLock::new();
        T.get_or_init(|| AliasTable::load(CNVALIAS))
    }

    fn load(item: &'static [u8]) -> AliasTable {
        let header = u16::from_le_bytes([item[0], item[1]]) as usize;
        let data = &item[header..];
        let u32_at = |i: usize| u32::from_le_bytes(data[4 * i..4 * i + 4].try_into().unwrap());
        let table_start = u32_at(0) as usize;
        let size = |s: usize| u32_at(s) as usize;
        // offsets in uint16_t units from the start of the data
        let mut off = (table_start + 1) * 2;
        let converter_list = off;
        off += size(1);
        let _tag_list = off;
        off += size(2);
        let alias_list = off;
        off += size(3);
        let untagged_conv_array = off;
        off += size(4);
        off += size(5); // taggedAliasArray
        off += size(6); // taggedAliasLists
        let option_table = off;
        let u16_at = |i: usize| u16::from_le_bytes([data[2 * i], data[2 * i + 1]]);
        // `UConverterAliasOptions`: stringNormalizationType (0 none, 1
        // UCNV_IO_STD_NORMALIZED), containsCnvOptionInfo
        let (norm_type, contains) = if size(7) > 0 && u16_at(option_table) < 2 {
            (u16_at(option_table), u16_at(option_table + 1))
        } else {
            (0, 0)
        };
        off += size(7);
        let string_table = off;
        off += size(8);
        let normalized = norm_type == 1;
        AliasTable {
            data,
            converter_list,
            converter_list_size: size(1),
            alias_list,
            untagged_conv_array,
            untagged_conv_array_size: size(4),
            contains_cnv_option_info: contains != 0,
            normalized,
            string_table,
            normalized_string_table: if normalized { off } else { string_table },
        }
    }

    fn u16(&self, i: usize) -> usize {
        u16::from_le_bytes([self.data[2 * i], self.data[2 * i + 1]]) as usize
    }

    fn string_at(&self, table: usize, idx: usize) -> &'static [u8] {
        let data: &'static [u8] = self.data;
        let s = &data[2 * (table + idx)..];
        &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]
    }

    /// `findConverter`: the converter number of `alias`, and whether its
    /// canonical name may hold options.
    fn find_converter(&self, alias: &[u8]) -> Option<(usize, bool)> {
        let key: Vec<u8>;
        let alias = if self.normalized {
            if alias.len() >= MAX_CONVERTER_NAME_LENGTH {
                return None;
            }
            key = strip_for_compare(alias);
            &key[..]
        } else {
            alias
        };
        let (mut start, mut limit) = (0usize, self.untagged_conv_array_size);
        let mut last_mid = usize::MAX;
        loop {
            let mid = (start + limit) / 2;
            if last_mid == mid {
                return None;
            }
            last_mid = mid;
            let entry = self.u16(self.alias_list + mid);
            let result = if self.normalized {
                alias.cmp(self.string_at(self.normalized_string_table, entry))
            } else {
                compare_names(alias, self.string_at(self.string_table, entry))
            };
            match result {
                std::cmp::Ordering::Less => limit = mid,
                std::cmp::Ordering::Greater => start = mid,
                std::cmp::Ordering::Equal => {
                    let v = self.u16(self.untagged_conv_array + mid);
                    let contains_option = !self.contains_cnv_option_info || v & 0x4000 != 0;
                    return Some((v & 0xfff, contains_option));
                }
            }
        }
    }

    /// `ucnv_io_getConverterName`: the canonical name of `alias` (tried
    /// again without a leading `x-`), and whether it may hold options.
    fn converter_name(&self, alias: &[u8]) -> Option<(&'static [u8], bool)> {
        let mut a = alias;
        for i in 0..2 {
            if i == 1 {
                match a.strip_prefix(b"x-") {
                    Some(rest) => a = rest,
                    None => break,
                }
            }
            if a.is_empty() {
                break;
            }
            if let Some((n, opt)) = self.find_converter(a) {
                if n < self.converter_list_size {
                    let s = self.u16(self.converter_list + n);
                    return Some((self.string_at(self.string_table, s), opt));
                }
            }
        }
        None
    }
}

/// `ucnv_compareNames` (for a table that is not normalized).
fn compare_names(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    strip_for_compare(a).cmp(&strip_for_compare(b))
}

/// `UConverterNamePieces` as `parseConverterOptions` fills it.
struct Pieces {
    cnv_name: Vec<u8>,
    options: u32,
}

/// `parseConverterOptions`: the name before the first `,` into
/// `cnv_name`, and the options after it (`version=`, `swaplfnl`;
/// `locale=` matters only to converters this port does not have). Fails
/// (`U_ILLEGAL_ARGUMENT_ERROR`) for a name of 60 bytes or more.
fn parse_converter_options(in_name: &[u8], p: &mut Pieces) -> Result<(), i32> {
    let mut i = 0;
    let at = |i: usize| in_name.get(i).copied().unwrap_or(0);
    let mut name = Vec::new();
    while at(i) != 0 && at(i) != b',' {
        if name.len() + 1 >= MAX_CONVERTER_NAME_LENGTH {
            p.cnv_name.clear();
            return Err(U_ILLEGAL_ARGUMENT_ERROR);
        }
        name.push(at(i));
        i += 1;
    }
    p.cnv_name = name;
    while at(i) != 0 {
        if at(i) == b',' {
            i += 1;
        }
        let rest = &in_name[i.min(in_name.len())..];
        if rest.starts_with(b"locale=") {
            i += 7;
            let mut len = 0;
            while at(i) != 0 && at(i) != b',' {
                i += 1;
                len += 1;
                // ULOC_FULLNAME_CAPACITY
                if len >= 157 {
                    return Err(U_ILLEGAL_ARGUMENT_ERROR);
                }
            }
        } else if rest.starts_with(b"version=") {
            i += 8;
            let c = at(i);
            if c == 0 {
                p.options &= !OPTION_VERSION;
                return Ok(());
            } else if c.wrapping_sub(b'0') < 10 {
                p.options = (p.options & !OPTION_VERSION) | (c - b'0') as u32;
                i += 1;
            }
        } else if rest.starts_with(b"swaplfnl") {
            i += 8;
            p.options |= OPTION_SWAP_LFNL;
        } else {
            // ignore any other options
            loop {
                let c = at(i);
                i += 1;
                if c == 0 {
                    return Ok(());
                }
                if c == b',' {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// What `getAlgorithmicTypeFromName` finds.
enum Algorithmic {
    Ported(Kind),
    Unported,
}

/// `getAlgorithmicTypeFromName` (`cnvNameType`, little-endian host: TeX
/// Live's macOS arm64 and x86_64 builds).
fn algorithmic_type(name: &[u8]) -> Option<Algorithmic> {
    use Algorithmic::{Ported as P, Unported as U};
    const TABLE: &[&str] = &[
        "bocu1",
        "cesu8",
        "hz",
        "imapmailboxname",
        "iscii",
        "iso2022",
        "iso88591",
        "lmbcs1",
        "lmbcs11",
        "lmbcs16",
        "lmbcs17",
        "lmbcs18",
        "lmbcs19",
        "lmbcs2",
        "lmbcs3",
        "lmbcs4",
        "lmbcs5",
        "lmbcs6",
        "lmbcs8",
        "scsu",
        "usascii",
        "utf16",
        "utf16be",
        "utf16le",
        "utf16oppositeendian",
        "utf16platformendian",
        "utf32",
        "utf32be",
        "utf32le",
        "utf32oppositeendian",
        "utf32platformendian",
        "utf7",
        "utf8",
        "x11compoundtext",
    ];
    let stripped = strip_for_compare(name);
    let (mut start, mut limit) = (0usize, TABLE.len());
    let mut last_mid = usize::MAX;
    let found = loop {
        let mid = (start + limit) / 2;
        if last_mid == mid {
            return None;
        }
        last_mid = mid;
        match stripped[..].cmp(TABLE[mid].as_bytes()) {
            std::cmp::Ordering::Less => limit = mid,
            std::cmp::Ordering::Greater => start = mid,
            std::cmp::Ordering::Equal => break TABLE[mid],
        }
    };
    Some(match found {
        "cesu8" => P(Kind::Cesu8),
        "iso88591" => P(Kind::Latin1),
        "usascii" => P(Kind::Ascii),
        "utf16" => P(Kind::Utf16),
        "utf16be" | "utf16oppositeendian" => P(Kind::Utf16Be),
        "utf16le" | "utf16platformendian" => P(Kind::Utf16Le),
        "utf32" => P(Kind::Utf32),
        "utf32be" | "utf32oppositeendian" => P(Kind::Utf32Be),
        "utf32le" | "utf32platformendian" => P(Kind::Utf32Le),
        "utf8" => P(Kind::Utf8),
        _ => U,
    })
}

/// `ucnv_open(name)`.
pub fn open(name: &[u8]) -> Opened {
    // a C string: up to its first NUL
    let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
    // `UCNV_FAST_IS_UTF8`
    if matches!(name, b"UTF-8" | b"UTF8" | b"utf-8" | b"utf8") {
        return Opened::Converter(Converter {
            kind: Kind::Utf8,
            options: 0,
        });
    }
    let mut p = Pieces {
        cnv_name: Vec::new(),
        options: 0,
    };
    if let Err(e) = parse_converter_options(name, &mut p) {
        return Opened::Failed(e);
    }
    let mut real: Vec<u8> = p.cnv_name.clone();
    if let Some((canonical, may_contain_option)) = AliasTable::get().converter_name(&p.cnv_name) {
        real = canonical.to_vec();
        if may_contain_option {
            if let Err(e) = parse_converter_options(canonical, &mut p) {
                return Opened::Failed(e);
            }
            real = p.cnv_name.clone();
        }
    }
    let kind = match algorithmic_type(&real) {
        Some(Algorithmic::Ported(k)) => k,
        Some(Algorithmic::Unported) => return Opened::Unported,
        None => {
            // a data converter, by its exact name (`udata_openChoice`)
            match crate::ucnv_mbcs::find(&real) {
                Some(i) => Kind::Data(i),
                None => return Opened::Failed(U_FILE_ACCESS_ERROR),
            }
        }
    };
    let mut cnv = Converter {
        kind,
        options: p.options,
    };
    // the converter's `open`
    match kind {
        Kind::Utf16Be | Kind::Utf16Le if cnv.version() > 1 => {
            return Opened::Failed(U_ILLEGAL_ARGUMENT_ERROR)
        }
        Kind::Utf16 if cnv.version() > 2 => return Opened::Failed(U_ILLEGAL_ARGUMENT_ERROR),
        // `ucnv_MBCSLoad` (an error: `ucnv_open` fails) and `ucnv_MBCSOpen`
        Kind::Data(i) => {
            let opened = crate::ucnv_mbcs::table(i)
                .and_then(|t| crate::ucnv_mbcs::open_options(t, &real, cnv.options));
            match opened {
                Ok(o) => cnv.options = o,
                Err(e) => return Opened::Failed(e),
            }
        }
        _ => {}
    }
    Opened::Converter(cnv)
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

/// What a converter's `toUnicode` writes into the pivot: UTF-16 code units,
/// with `_toUnicodeWithCallback` calling `UCNV_TO_U_CALLBACK_SUBSTITUTE`
/// for each error.
pub(crate) struct Pivot {
    pub(crate) units: Vec<u16>,
    /// `cnv->subChar1`.
    pub(crate) sub_char1: u8,
}

impl Pivot {
    pub(crate) fn push(&mut self, c: u32) {
        if c <= 0xffff {
            self.units.push(c as u16);
        } else {
            self.units.push((0xd7c0 + (c >> 10)) as u16);
            self.units.push((0xdc00 | (c & 0x3ff)) as u16);
        }
    }

    /// `ucnv_cbToUWriteSub` for an error over `invalid_len` bytes.
    pub(crate) fn sub(&mut self, invalid_len: usize) {
        if invalid_len == 1 && self.sub_char1 != 0 {
            self.units.push(0x1a);
        } else {
            self.units.push(0xfffd);
        }
    }
}

/// `ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, cnv, ...)` of a line: its
/// characters, appended to `out`.
pub fn to_unicode(cnv: Converter, src: &[u8], out: &mut Vec<u32>) {
    let mut p = Pivot {
        units: Vec::with_capacity(src.len()),
        sub_char1: 0,
    };
    match cnv.kind {
        Kind::Utf8 => utf8_to_u(src, false, &mut p),
        Kind::Cesu8 => utf8_to_u(src, true, &mut p),
        Kind::Latin1 => p.units.extend(src.iter().map(|&b| b as u16)),
        Kind::Ascii => {
            for &b in src {
                if b <= 0x7f {
                    p.units.push(b as u16);
                } else {
                    p.sub(1);
                }
            }
        }
        Kind::Utf16Be | Kind::Utf16Le | Kind::Utf16 => utf16_to_u(cnv, src, &mut p),
        Kind::Utf32Be => utf32_to_u(src, true, &mut p),
        Kind::Utf32Le => utf32_to_u(src, false, &mut p),
        Kind::Utf32 => utf32_bom_to_u(src, &mut p),
        Kind::Data(i) => {
            if let Ok(t) = crate::ucnv_mbcs::table(i) {
                crate::ucnv_mbcs::to_u(t, cnv.options, src, &mut p);
            }
        }
    }
    // The pivot into UTF-32 (`T_UConverter_fromUnicode_UTF32_LE`): a
    // surrogate pair is one character, an unpaired surrogate (only CESU-8
    // gives one) is substituted with U+FFFD.
    let u = &p.units;
    let mut i = 0;
    while i < u.len() {
        let c = u[i] as u32;
        i += 1;
        if (0xd800..0xdc00).contains(&c) {
            if i < u.len() && (0xdc00..0xe000).contains(&(u[i] as u32)) {
                out.push(0x10000 + ((c - 0xd800) << 10) + (u[i] as u32 - 0xdc00));
                i += 1;
            } else {
                out.push(0xfffd);
            }
        } else if (0xdc00..0xe000).contains(&c) {
            out.push(0xfffd);
        } else {
            out.push(c);
        }
    }
}

/// `offsetsFromUTF8`.
const OFFSETS_FROM_UTF8: [u32; 5] = [0, 0, 0x3080, 0xE2080, 0x3C82080];

/// `icu::UTF8::isValidTrail` (ustr_imp.h): trail byte `i` of a sequence of
/// `length` bytes led by `lead`.
fn is_valid_trail(lead: u32, t: u8, i: usize, length: usize) -> bool {
    if length <= 2 || i > 1 {
        (0x80..0xc0).contains(&t)
    } else if length == 3 {
        // U8_IS_VALID_LEAD3_AND_T1
        const BITS: &[u8; 16] = b"\x20\x30\x30\x30\x30\x30\x30\x30\x30\x30\x30\x30\x30\x10\x30\x30";
        BITS[(lead & 0xf) as usize] & (1 << (t >> 5)) != 0
    } else {
        // U8_IS_VALID_LEAD4_AND_T1
        const BITS: &[u8; 16] = b"\x00\x00\x00\x00\x00\x00\x00\x00\x1E\x0F\x0F\x0F\x00\x00\x00\x00";
        BITS[(t >> 4) as usize] & (1 << (lead & 7)) != 0
    }
}

/// `ucnv_toUnicode_UTF8` (and CESU-8's) over the whole line, flushed.
fn utf8_to_u(src: &[u8], cesu: bool, p: &mut Pivot) {
    let mut s = 0;
    while s < src.len() {
        let lead = src[s] as u32;
        s += 1;
        if lead < 0x80 {
            p.units.push(lead as u16);
            continue;
        }
        // U8_COUNT_BYTES_NON_ASCII
        let in_bytes = if (0xc2..=0xf4).contains(&lead) {
            2 + (lead >= 0xe0) as usize + (lead >= 0xf0) as usize
        } else {
            0
        };
        let mut ch = lead;
        let mut i = 1;
        while i < in_bytes {
            if s < src.len() {
                let t = src[s];
                if !is_valid_trail(ch, t, i, in_bytes)
                    && !(cesu && i == 1 && lead == 0xed && (0x80..0xc0).contains(&t))
                {
                    break;
                }
                ch = (ch << 6) + t as u32;
                s += 1;
                i += 1;
            } else {
                // a truncated sequence at the end of the input: one
                // U_TRUNCATED_CHAR_FOUND
                p.sub(i);
                return;
            }
        }
        if i == in_bytes && (!cesu || i <= 3) {
            p.push(ch.wrapping_sub(OFFSETS_FROM_UTF8[in_bytes]));
        } else {
            // U_ILLEGAL_CHAR_FOUND over the `i` bytes read
            p.sub(i);
        }
    }
}

/// `_UTF16BEToUnicodeWithOffsets` / `_UTF16LEToUnicodeWithOffsets` (mode
/// 8 and 9) over the rest of the line, flushed: an unpaired surrogate is
/// substituted, and so is an incomplete code unit or pair at the end.
fn utf16_core(src: &[u8], be: bool, p: &mut Pivot) {
    let unit = |i: usize| -> u16 {
        if be {
            (src[i] as u16) << 8 | src[i + 1] as u16
        } else {
            (src[i + 1] as u16) << 8 | src[i] as u16
        }
    };
    let mut i = 0;
    while i + 1 < src.len() {
        let c = unit(i);
        i += 2;
        if !(0xd800..0xe000).contains(&c) {
            p.units.push(c);
        } else if c < 0xdc00 {
            if i + 1 < src.len() {
                let t = unit(i);
                if (0xdc00..0xe000).contains(&t) {
                    p.units.push(c);
                    p.units.push(t);
                    i += 2;
                } else {
                    // unmatched lead surrogate (toULength 2); the unit
                    // after it is read again
                    p.sub(2);
                }
            } else {
                // a lead surrogate at the end, with the odd byte after it
                // if any: one truncated sequence
                p.sub(2 + src.len() - i);
                return;
            }
        } else {
            // unmatched trail surrogate
            p.sub(2);
        }
    }
    if i < src.len() {
        // one byte left: truncated
        p.sub(1);
    }
}

/// `_UTF16ToUnicodeWithOffsets`: the byte order mark of `UTF-16` (and of
/// Java's `UnicodeBig`/`UnicodeLittle`, `UTF-16BE,version=1` and
/// `UTF-16LE,version=1`), then the code units.
fn utf16_to_u(cnv: Converter, src: &[u8], p: &mut Pivot) {
    let is_be = cnv.kind == Kind::Utf16Be;
    let is_le = cnv.kind == Kind::Utf16Le;
    // `_UTF16BEReset`/`_UTF16LEReset`: version 0 has no BOM handling
    if (is_be || is_le) && cnv.version() == 0 {
        utf16_core(src, is_be, p);
        return;
    }
    match src.len() {
        0 => return,
        // state 1 at the end: the framework reports a truncated character
        1 => {
            p.sub(1);
            return;
        }
        _ => {}
    }
    let (b0, b1) = (src[0], src[1]);
    let mut state = 1;
    if b0 == 0xfe && b1 == 0xff {
        state = if is_le { 7 } else { 8 };
    } else if b0 == 0xff && b1 == 0xfe {
        state = if is_be { 6 } else { 9 };
    } else if cnv.kind == Kind::Utf16 && cnv.version() == 1 {
        state = 6;
    }
    if state >= 8 {
        // a BOM: consumed
        utf16_core(&src[2..], state == 8, p);
    } else if state < 6 {
        // no BOM: `UnicodeLittle` defaults to little-endian, the others to
        // big-endian
        utf16_core(src, !is_le, p);
    } else {
        // a missing or reverse BOM: U_ILLEGAL_ESCAPE_SEQUENCE over the two
        // bytes, then mode 8 (state 6) or 9 (state 7)
        p.sub(2);
        utf16_core(&src[2..], state == 6, p);
    }
}

/// `T_UConverter_toUnicode_UTF32_BE`/`_LE` over the rest of the line,
/// flushed.
fn utf32_to_u(src: &[u8], be: bool, p: &mut Pivot) {
    let whole = src.len() / 4;
    for k in 0..whole {
        let c = [src[4 * k], src[4 * k + 1], src[4 * k + 2], src[4 * k + 3]];
        let ch = if be {
            u32::from_be_bytes(c)
        } else {
            u32::from_le_bytes(c)
        };
        if ch <= 0x10ffff && !(0xd800..0xe000).contains(&ch) {
            p.push(ch);
        } else {
            p.sub(4);
        }
    }
    let rest = src.len() - 4 * whole;
    if rest > 0 {
        p.sub(rest);
    }
}

/// `_UTF32ToUnicodeWithOffsets`: a byte order mark (`00 00 FE FF` or
/// `FF FE 00 00`) is consumed, else big-endian from the start.
fn utf32_bom_to_u(src: &[u8], p: &mut Pivot) {
    if src.starts_with(&[0, 0, 0xfe, 0xff]) {
        utf32_to_u(&src[4..], true, p);
    } else if src.starts_with(&[0xff, 0xfe, 0, 0]) {
        utf32_to_u(&src[4..], false, p);
    } else {
        // shorter than 4 bytes (a BOM's beginning or not): replayed as
        // big-endian, one truncated character
        utf32_to_u(src, true, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(name: &str, bytes: &[u8]) -> Vec<u32> {
        let Opened::Converter(c) = open(name.as_bytes()) else {
            panic!("{name} does not open");
        };
        let mut out = Vec::new();
        to_unicode(c, bytes, &mut out);
        out
    }

    #[test]
    fn data_list_is_the_vendored_files() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/icu/cnv");
        let mut on_disk: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| {
                let n = e.unwrap().file_name().into_string().unwrap();
                n.strip_suffix(".cnv").map(str::to_string)
            })
            .collect();
        on_disk.sort();
        let listed: Vec<String> = crate::ucnv_data::DATA
            .iter()
            .map(|(n, _)| n.to_string())
            .collect();
        assert_eq!(listed, on_disk);
        // every table loads (`ucnv_MBCSLoad`), extension-only ones with
        // their base tables
        for (i, (n, _)) in crate::ucnv_data::DATA.iter().enumerate() {
            assert!(crate::ucnv_mbcs::table(i as u16).is_ok(), "{n}");
        }
    }

    #[test]
    fn strip_drops_leading_zeros_and_punctuation() {
        assert_eq!(strip_for_compare(b"ISO_8859-01"), b"iso88591");
        assert_eq!(strip_for_compare(b"UTF-008"), b"utf8");
        assert_eq!(strip_for_compare(b"ibm-100"), b"ibm100");
        assert_eq!(strip_for_compare(b"x\xc3\xa9-1"), b"x1");
    }

    #[test]
    fn names_resolve_as_icu_does() {
        let kind = |n: &str| match open(n.as_bytes()) {
            Opened::Converter(c) => Some(c.kind),
            _ => None,
        };
        assert_eq!(kind("UTF-8"), Some(Kind::Utf8));
        assert_eq!(kind("unicode-1-1-utf-8"), Some(Kind::Utf8));
        assert_eq!(kind("latin1"), Some(Kind::Latin1));
        assert_eq!(kind("US-ASCII"), Some(Kind::Ascii));
        assert_eq!(kind("UnicodeLittleUnmarked"), Some(Kind::Utf16Le));
        assert_eq!(kind("x-utf-16be"), Some(Kind::Utf16Be));
        assert!(matches!(kind("cp1252"), Some(Kind::Data(_))));
        assert!(matches!(kind("windows-1252"), Some(Kind::Data(_))));
        assert_eq!(kind("no-such-encoding"), None);
        assert!(matches!(kind("Shift_JIS"), Some(Kind::Data(_))));
        assert_eq!(open(b"UTF-7"), Opened::Unported);
    }

    #[test]
    fn converts_as_icu_does() {
        assert_eq!(conv("cp1252", b"\x80\x81a"), [0x20ac, 0x81, 0x61]);
        assert_eq!(conv("US-ASCII", b"a\x80"), [0x61, 0xfffd]);
        assert_eq!(conv("UTF-16BE", b"\x00a\x00"), [0x61, 0xfffd]);
        assert_eq!(conv("UTF-16", b"\xff\xfea\x00"), [0x61]);
        assert_eq!(
            conv("UTF-8", b"a\xe9b\xed\xa0\x80"),
            [0x61, 0xfffd, 0x62, 0xfffd, 0xfffd, 0xfffd]
        );
        assert_eq!(conv("CESU-8", b"\xed\xa0\x80\xed\xb0\x80"), [0x10000]);
        assert_eq!(conv("Shift_JIS", b"\x82\xa0a"), [0x3042, 0x61]);
        assert_eq!(conv("GB18030", b"\x81\x30\x81\x30"), [0x80]);
        assert_eq!(conv("GB18030", b"\x90\x30\x81\x30"), [0x10000]);
    }
}
