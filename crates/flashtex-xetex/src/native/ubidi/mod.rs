//! ICU's bidirectional algorithm, as XeTeX uses it: `measure_native_node`
//! (XeTeX_ext.c) opens a `UBiDi`, calls `ubidi_setPara` on a native word's
//! text with the paragraph level `UBIDI_DEFAULT_LTR` or `UBIDI_DEFAULT_RTL`
//! (`getDefaultDirection`) and no embedding levels, and reads back
//! `ubidi_getDirection`, then `ubidi_countRuns` and `ubidi_getVisualRun`.
//!
//! A port of ICU 78.2's `ubidi.cpp` and `ubidiln.cpp` (the ICU TeX Live 2026
//! builds xetex with; third_party/icu, the source of record), restricted to
//! that use: the reordering mode is `UBIDI_REORDER_DEFAULT` and the options
//! `UBIDI_OPTION_DEFAULT`, there is no prologue or epilogue, class callback
//! or `ubidi_setLine`, and paragraphs are not ordered left to right. The
//! code that only another mode, an option or those calls reaches is left
//! out; everything on the path above follows ICU's statement for statement,
//! with its names, so that each routine can be read against the C. ICU's
//! fixed-size arrays that spill to the heap (`simpleParas`,
//! `simpleOpenings`, `simpleIsolates`, `simpleRuns`) are vectors here; the
//! results do not depend on where they live.
//!
//! ICU is under the Unicode License V3 (and, for its older code, the ICU
//! licence); both notices are in this crate's LICENSE.

mod props;
mod props_data;

use props::{get_class, get_paired_bracket, get_paired_bracket_type, U_BPT_OPEN};

/// `UBiDiLevel` values (ubidi.h).
pub const UBIDI_DEFAULT_LTR: u8 = 0xfe;
pub const UBIDI_DEFAULT_RTL: u8 = 0xff;
pub const UBIDI_MAX_EXPLICIT_LEVEL: u8 = 125;
const UBIDI_LEVEL_OVERRIDE: u8 = 0x80;

/// `UBiDiDirection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ltr = 0,
    Rtl = 1,
    Mixed = 2,
}

impl Direction {
    fn from_bit(b: u8) -> Direction {
        if b & 1 == 0 {
            Direction::Ltr
        } else {
            Direction::Rtl
        }
    }
}

// ---- ubidiimp.h -------------------------------------------------------

type DirProp = u8;
type Flags = u32;

const L: DirProp = 0;
const R: DirProp = 1;
const EN: DirProp = 2;
const ES: DirProp = 3;
const ET: DirProp = 4;
const AN: DirProp = 5;
const CS: DirProp = 6;
const B: DirProp = 7;
const S: DirProp = 8;
const WS: DirProp = 9;
const ON: DirProp = 10;
const LRE: DirProp = 11;
const LRO: DirProp = 12;
const AL: DirProp = 13;
const RLE: DirProp = 14;
const RLO: DirProp = 15;
const PDF: DirProp = 16;
const NSM: DirProp = 17;
const BN: DirProp = 18;
const FSI: DirProp = 19;
const LRI: DirProp = 20;
const RLI: DirProp = 21;
const PDI: DirProp = 22;
const ENL: DirProp = 23; // EN after W7
const ENR: DirProp = 24; // EN not subject to W7
/// `U_CHAR_DIRECTION_COUNT`.
const U_CHAR_DIRECTION_COUNT: u8 = 23;

const fn dirprop_flag(dir: DirProp) -> Flags {
    1 << dir
}
const DIRPROP_FLAG_MULTI_RUNS: Flags = 1 << 31;

const MASK_LTR: Flags = dirprop_flag(L)
    | dirprop_flag(EN)
    | dirprop_flag(ENL)
    | dirprop_flag(ENR)
    | dirprop_flag(AN)
    | dirprop_flag(LRE)
    | dirprop_flag(LRO)
    | dirprop_flag(LRI);
const MASK_RTL: Flags =
    dirprop_flag(R) | dirprop_flag(AL) | dirprop_flag(RLE) | dirprop_flag(RLO) | dirprop_flag(RLI);
const MASK_EXPLICIT: Flags = dirprop_flag(LRE)
    | dirprop_flag(LRO)
    | dirprop_flag(RLE)
    | dirprop_flag(RLO)
    | dirprop_flag(PDF);
const MASK_ISO: Flags =
    dirprop_flag(LRI) | dirprop_flag(RLI) | dirprop_flag(FSI) | dirprop_flag(PDI);
const MASK_BN_EXPLICIT: Flags = dirprop_flag(BN) | MASK_EXPLICIT;
const MASK_B_S: Flags = dirprop_flag(B) | dirprop_flag(S);
const MASK_WS: Flags = MASK_B_S | dirprop_flag(WS) | MASK_BN_EXPLICIT | MASK_ISO;
const MASK_POSSIBLE_N: Flags =
    dirprop_flag(ON) | dirprop_flag(CS) | dirprop_flag(ES) | dirprop_flag(ET) | MASK_WS;
const MASK_EMBEDDING: Flags = dirprop_flag(NSM) | MASK_POSSIBLE_N;

/// `GET_LR_FROM_LEVEL`.
fn get_lr_from_level(level: u8) -> DirProp {
    level & 1
}
/// `IS_DEFAULT_LEVEL`.
fn is_default_level(level: u8) -> bool {
    level >= 0xfe
}

const ISOLATE: u16 = 0x0100;

const CR: u16 = 0x000D;
const LF: u16 = 0x000A;

/// `Para`.
#[derive(Clone, Copy, Default)]
struct Para {
    limit: i32,
    level: i32,
}

const FOUND_L: u16 = dirprop_flag(L) as u16;
const FOUND_R: u16 = dirprop_flag(R) as u16;

/// `Opening`.
#[derive(Clone, Copy, Default)]
struct Opening {
    /// position of the opening bracket
    position: i32,
    /// the matching char, or -position of the closing bracket
    r#match: i32,
    /// position of the last strong char found before the opening
    context_pos: i32,
    /// bits for L or R/AL found within the pair
    flags: u16,
    /// L or R according to the last strong char before the opening
    context_dir: u8,
}

/// `IsoRun`.
#[derive(Clone, Copy, Default)]
struct IsoRun {
    context_pos: i32,
    start: u16,
    limit: u16,
    level: u8,
    last_strong: DirProp,
    last_base: DirProp,
    context_dir: u8,
}

/// `BracketData`.
struct BracketData {
    openings: Vec<Opening>,
    iso_run_last: usize,
    iso_runs: [IsoRun; UBIDI_MAX_EXPLICIT_LEVEL as usize + 2],
}

/// `Isolate`.
#[derive(Clone, Copy, Default)]
struct Isolate {
    start_on: i32,
    start1: i32,
    state: i32,
    state_imp: i16,
}

/// `Run`: `logical_start` carries the odd-level bit in b31.
#[derive(Clone, Copy, Default)]
struct Run {
    logical_start: i32,
    visual_limit: i32,
}

const INDEX_ODD_BIT: u32 = 1 << 31;
fn get_index(x: i32) -> i32 {
    (x as u32 & !INDEX_ODD_BIT) as i32
}
fn get_odd_bit(x: i32) -> u32 {
    x as u32 >> 31
}

/// `NO_OVERRIDE`.
fn no_override(level: u8) -> u8 {
    level & !UBIDI_LEVEL_OVERRIDE
}

/// `DIRPROP_FLAG_LR`, `DIRPROP_FLAG_E`, `DIRPROP_FLAG_O`.
fn dirprop_flag_lr(level: u8) -> Flags {
    [dirprop_flag(L), dirprop_flag(R)][(level & 1) as usize]
}
fn dirprop_flag_e(level: u8) -> Flags {
    [dirprop_flag(LRE), dirprop_flag(RLE)][(level & 1) as usize]
}
fn dirprop_flag_o(level: u8) -> Flags {
    [dirprop_flag(LRO), dirprop_flag(RLO)][(level & 1) as usize]
}
/// `DIR_FROM_STRONG`.
fn dir_from_strong(strong: DirProp) -> DirProp {
    if strong == L {
        L
    } else {
        R
    }
}

/// `ubidi_getCustomizedClass` without a class callback.
fn customized_class(c: i32) -> DirProp {
    let dir = get_class(c);
    if dir >= U_CHAR_DIRECTION_COUNT {
        ON
    } else {
        dir
    }
}

/// `U16_NEXT`: the code point at `*i`, advancing `*i`; an unpaired
/// surrogate is returned as itself.
fn u16_next(s: &[u16], i: &mut usize, length: usize) -> i32 {
    let c = s[*i] as i32;
    *i += 1;
    if (0xd800..0xdc00).contains(&c) && *i != length {
        let c2 = s[*i] as i32;
        if (0xdc00..0xe000).contains(&c2) {
            *i += 1;
            return (c << 10) + c2 - ((0xd800 << 10) + 0xdc00 - 0x10000);
        }
    }
    c
}

/// A `UBiDi` paragraph object after `ubidi_setPara`.
pub struct UBiDi<'a> {
    text: &'a [u16],
    length: i32,
    para_level: u8,
    /// `defaultParaLevel` (ICU stores the boolean `IS_DEFAULT_LEVEL`).
    default_para_level: u8,
    dir_props: Vec<DirProp>,
    levels: Vec<u8>,
    direction: Direction,
    flags: Flags,
    trailing_ws_start: i32,
    para_count: i32,
    paras: Vec<Para>,
    run_count: i32,
    runs: Vec<Run>,
    isolate_count: i32,
    isolates: Vec<Isolate>,
}

impl<'a> UBiDi<'a> {
    /// `ubidi_open` then `ubidi_setPara(pBiDi, text, length, paraLevel,
    /// NULL, &errorCode)`. `para_level` is 0..=125 or one of the defaults.
    pub fn set_para(text: &'a [u16], para_level: u8) -> UBiDi<'a> {
        assert!(
            para_level <= UBIDI_MAX_EXPLICIT_LEVEL || para_level >= UBIDI_DEFAULT_LTR,
            "ubidi_setPara: U_ILLEGAL_ARGUMENT_ERROR"
        );
        let length = text.len() as i32;
        // initialize the UBiDi structure
        let mut b = UBiDi {
            text,
            length,
            para_level,
            default_para_level: is_default_level(para_level) as u8,
            dir_props: Vec::new(),
            levels: Vec::new(),
            direction: Direction::from_bit(para_level),
            flags: 0,
            trailing_ws_start: 0,
            para_count: 1,
            paras: vec![Para::default()],
            run_count: 0,
            runs: Vec::new(),
            isolate_count: 0,
            isolates: Vec::new(),
        };

        if length == 0 {
            // an empty paragraph: the paraLevel, flags and direction only
            if is_default_level(para_level) {
                b.para_level &= 1;
                b.default_para_level = 0;
            }
            b.flags = dirprop_flag_lr(para_level);
            b.run_count = 0;
            b.para_count = 0;
            return b;
        }

        b.run_count = -1;

        // the directional properties, the flags bit-set, and the
        // paragraph level if necessary
        b.dir_props = vec![0; length as usize];
        b.get_dir_props();
        let length = b.length;
        b.trailing_ws_start = length; // the levels[] will reflect the WS run

        // determine explicit levels according to the (Xn) rules
        b.levels = vec![0; length as usize];
        let direction = b.resolve_explicit_levels();

        // allocate isolate memory
        b.isolates = vec![Isolate::default(); b.isolate_count.max(0) as usize + 1];
        b.isolate_count = -1; // current isolates stack entry == none

        // The steps after (X9) are performed only if the paragraph text has
        // mixed directionality.
        b.direction = direction;
        match direction {
            // all levels are implicitly at paraLevel
            Direction::Ltr | Direction::Rtl => b.trailing_ws_start = 0,
            Direction::Mixed => {
                // UBIDI_REORDER_DEFAULT: pImpTabPair=&impTab_DEFAULT
                if b.para_count <= 1 && b.flags & DIRPROP_FLAG_MULTI_RUNS == 0 {
                    let sor = get_lr_from_level(b.get_paralevel(0));
                    let eor = get_lr_from_level(b.get_paralevel(length - 1));
                    b.resolve_implicit_levels(0, length, sor, eor);
                } else {
                    // sor, eor: start and end types of same-level-run
                    let mut limit = 0i32;
                    // determine the first sor and set eor to it because of
                    // the loop body (sor=eor there)
                    let mut level = b.get_paralevel(0);
                    let mut next_level = b.levels[0];
                    let mut eor = if level < next_level {
                        get_lr_from_level(next_level)
                    } else {
                        get_lr_from_level(level)
                    };
                    loop {
                        // the values for this run's start are the same as
                        // for the previous run's end
                        let mut start = limit;
                        level = next_level;
                        let sor = if start > 0 && b.dir_props[(start - 1) as usize] == B {
                            // a new paragraph: sor = para level
                            get_lr_from_level(b.get_paralevel(start))
                        } else {
                            eor
                        };
                        // search for the limit of this run
                        loop {
                            limit += 1;
                            let more = limit < length
                                && (b.levels[limit as usize] == level
                                    || dirprop_flag(b.dir_props[limit as usize])
                                        & MASK_BN_EXPLICIT
                                        != 0);
                            if !more {
                                break;
                            }
                        }
                        // the correct level of the next run
                        next_level = if limit < length {
                            b.levels[limit as usize]
                        } else {
                            b.get_paralevel(length - 1)
                        };
                        // eor from max(level, nextLevel); sor is last run's eor
                        eor = if no_override(level) < no_override(next_level) {
                            get_lr_from_level(next_level)
                        } else {
                            get_lr_from_level(level)
                        };
                        // a run of overridden directional types has no
                        // implicit types to be resolved
                        if level & UBIDI_LEVEL_OVERRIDE == 0 {
                            b.resolve_implicit_levels(start, limit, sor, eor);
                        } else {
                            // remove the UBIDI_LEVEL_OVERRIDE flags
                            loop {
                                b.levels[start as usize] &= !UBIDI_LEVEL_OVERRIDE;
                                start += 1;
                                if start >= limit {
                                    break;
                                }
                            }
                        }
                        if limit >= length {
                            break;
                        }
                    }
                }
                // reset the embedding levels for some non-graphic
                // characters (L1), (X9)
                b.adjust_ws_levels();
            }
        }
        b
    }

    /// `ubidi_getDirection`.
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// `ubidi_countRuns`.
    pub fn count_runs(&mut self) -> i32 {
        self.get_runs();
        self.run_count
    }

    /// `ubidi_getVisualRun`: the run's direction, its logical start and
    /// its length.
    pub fn get_visual_run(&mut self, run_index: i32) -> (Direction, i32, i32) {
        self.get_runs();
        if run_index < 0 || run_index >= self.run_count {
            // RETURN_IF_BAD_RANGE
            return (Direction::Ltr, 0, 0);
        }
        let start = self.runs[run_index as usize].logical_start;
        let length = if run_index > 0 {
            self.runs[run_index as usize].visual_limit
                - self.runs[run_index as usize - 1].visual_limit
        } else {
            self.runs[0].visual_limit
        };
        let dir = if get_odd_bit(start) == 0 {
            Direction::Ltr
        } else {
            Direction::Rtl
        };
        (dir, get_index(start), length)
    }

    /// `ubidi_getLevels` for a paragraph object (tests): the levels up to
    /// `trailingWSStart`, then `paraLevel` (the first paragraph's level,
    /// even when there are several: ICU's own comment assumes a line).
    #[cfg(test)]
    fn get_levels(&self) -> Vec<u8> {
        let start = self.trailing_ws_start as usize;
        let mut v = self.levels[..start].to_vec();
        v.resize(self.length as usize, self.para_level);
        v
    }

    /// `GET_PARALEVEL`.
    fn get_paralevel(&self, index: i32) -> u8 {
        if self.default_para_level == 0 || index < self.paras[0].limit {
            self.para_level
        } else {
            self.get_para_level_at_index(index)
        }
    }

    /// `ubidi_getParaLevelAtIndex`.
    fn get_para_level_at_index(&self, pindex: i32) -> u8 {
        let mut i = 0;
        while i < self.para_count {
            if pindex < self.paras[i as usize].limit {
                break;
            }
            i += 1;
        }
        if i >= self.para_count {
            i = self.para_count - 1;
        }
        self.paras[i as usize].level as u8
    }

    /// `checkParaCount`: room for paras[paraCount-1].
    fn check_para_count(&mut self) {
        let count = self.para_count as usize;
        if self.paras.len() < count {
            self.paras.resize(count, Para::default());
        }
    }

    // ---- perform (P2)..(P3) ---------------------------------------------

    /// `getDirProps`: the directional properties, the flags, the paragraph
    /// level(s); FSI resolved to LRI or RLI.
    fn get_dir_props(&mut self) {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum State {
            NotSeekingStrong,     // 0: not contextual paraLevel, not after FSI
            SeekingStrongForPara, // 1: looking for first strong char in para
            SeekingStrongForFsi,  // 2: looking for first strong after FSI
            LookingForPdi,        // 3: found strong after FSI, looking for PDI
        }
        let text = self.text;
        let original_length = self.length as usize;
        let mut i = 0usize;
        let mut flags: Flags = 0;
        let is_default_level = is_default_level(self.para_level);
        let max = UBIDI_MAX_EXPLICIT_LEVEL as i32;
        let mut isolate_start_stack = [0i32; UBIDI_MAX_EXPLICIT_LEVEL as usize + 1];
        let mut previous_state_stack =
            [State::NotSeekingStrong; UBIDI_MAX_EXPLICIT_LEVEL as usize + 1];
        let mut stack_last: i32 = -1;

        let default_para_level = (self.para_level & 1) as i32;
        let mut state = if is_default_level {
            self.paras[0].level = default_para_level;
            State::SeekingStrongForPara // no prologue
        } else {
            self.paras[0].level = self.para_level as i32;
            State::NotSeekingStrong
        };
        // count paragraphs and determine the paragraph level (P2..P3)
        while i < original_length {
            let uchar = u16_next(text, &mut i, original_length);
            let dir_prop = customized_class(uchar);
            flags |= dirprop_flag(dir_prop);
            self.dir_props[i - 1] = dir_prop;
            if uchar > 0xffff {
                // set the lead surrogate's property to BN
                flags |= dirprop_flag(BN);
                self.dir_props[i - 2] = BN;
            }
            let pc = (self.para_count - 1) as usize;
            if dir_prop == L {
                if state == State::SeekingStrongForPara {
                    self.paras[pc].level = 0;
                    state = State::NotSeekingStrong;
                } else if state == State::SeekingStrongForFsi {
                    if stack_last <= max {
                        flags |= dirprop_flag(LRI);
                    }
                    state = State::LookingForPdi;
                }
                continue;
            }
            if dir_prop == R || dir_prop == AL {
                if state == State::SeekingStrongForPara {
                    self.paras[pc].level = 1;
                    state = State::NotSeekingStrong;
                } else if state == State::SeekingStrongForFsi {
                    if stack_last <= max {
                        self.dir_props[isolate_start_stack[stack_last as usize] as usize] = RLI;
                        flags |= dirprop_flag(RLI);
                    }
                    state = State::LookingForPdi;
                }
                continue;
            }
            if (FSI..=RLI).contains(&dir_prop) {
                // FSI, LRI or RLI
                stack_last += 1;
                if stack_last <= max {
                    isolate_start_stack[stack_last as usize] = i as i32 - 1;
                    previous_state_stack[stack_last as usize] = state;
                }
                if dir_prop == FSI {
                    self.dir_props[i - 1] = LRI; // default if no strong char
                    state = State::SeekingStrongForFsi;
                } else {
                    state = State::LookingForPdi;
                }
                continue;
            }
            if dir_prop == PDI {
                if state == State::SeekingStrongForFsi && stack_last <= max {
                    flags |= dirprop_flag(LRI);
                }
                if stack_last >= 0 {
                    if stack_last <= max {
                        state = previous_state_stack[stack_last as usize];
                    }
                    stack_last -= 1;
                }
                continue;
            }
            if dir_prop == B {
                if i < original_length && uchar == CR as i32 && text[i] == LF {
                    continue; // do nothing on the CR
                }
                self.paras[pc].limit = i as i32;
                if i < original_length {
                    // B not last char in text
                    self.para_count += 1;
                    self.check_para_count();
                    let pc = (self.para_count - 1) as usize;
                    if is_default_level {
                        self.paras[pc].level = default_para_level;
                        state = State::SeekingStrongForPara;
                    } else {
                        self.paras[pc].level = self.para_level as i32;
                        state = State::NotSeekingStrong;
                    }
                    stack_last = -1;
                }
                continue;
            }
        }
        // Ignore still open isolate sequences with overflow
        if stack_last > max {
            stack_last = max;
            state = State::SeekingStrongForFsi; // to be on the safe side
        }
        // Resolve direction of still unresolved open FSI sequences
        while stack_last >= 0 {
            if state == State::SeekingStrongForFsi {
                flags |= dirprop_flag(LRI);
                break;
            }
            state = previous_state_stack[stack_last as usize];
            stack_last -= 1;
        }
        let pc = (self.para_count - 1) as usize;
        self.paras[pc].limit = original_length as i32;
        if is_default_level {
            self.para_level = self.paras[0].level as u8;
        }
        // resolve the text direction for default level paragraphs
        // containing no strong character
        for p in &self.paras[..self.para_count as usize] {
            flags |= dirprop_flag_lr(p.level as u8);
        }
        self.flags = flags;
        // (lastArabicPos is read only by the inverse modes)
    }

    // ---- paired brackets ------------------------------------------------

    /// `bracketInit`.
    fn bracket_init(&self) -> BracketData {
        let mut bd = BracketData {
            openings: vec![Opening::default(); 20], // SIMPLE_OPENINGS_COUNT
            iso_run_last: 0,
            iso_runs: [IsoRun::default(); UBIDI_MAX_EXPLICIT_LEVEL as usize + 2],
        };
        let r = &mut bd.iso_runs[0];
        r.start = 0;
        r.limit = 0;
        r.level = self.get_paralevel(0);
        let t = self.get_paralevel(0) & 1;
        r.last_strong = t;
        r.last_base = t;
        r.context_dir = t;
        r.context_pos = 0;
        bd
    }

    /// `bracketProcessB`: a paragraph boundary.
    fn bracket_process_b(bd: &mut BracketData, level: u8) {
        bd.iso_run_last = 0;
        let r = &mut bd.iso_runs[0];
        r.limit = 0;
        r.level = level;
        r.last_strong = level & 1;
        r.last_base = level & 1;
        r.context_dir = level & 1;
        r.context_pos = 0;
    }

    /// `bracketProcessBoundary`: LRE, LRO, RLE, RLO, PDF.
    fn bracket_process_boundary(
        &self,
        bd: &mut BracketData,
        last_cc_pos: i32,
        mut context_level: u8,
        embedding_level: u8,
    ) {
        if dirprop_flag(self.dir_props[last_cc_pos as usize]) & MASK_ISO != 0 {
            return; // after an isolate
        }
        if no_override(embedding_level) > no_override(context_level) {
            // not a PDF
            context_level = embedding_level;
        }
        let r = &mut bd.iso_runs[bd.iso_run_last];
        r.limit = r.start;
        r.level = embedding_level;
        r.last_strong = context_level & 1;
        r.last_base = context_level & 1;
        r.context_dir = context_level & 1;
        r.context_pos = last_cc_pos;
    }

    /// `bracketProcessLRI_RLI`.
    fn bracket_process_lri_rli(bd: &mut BracketData, level: u8) {
        let last = bd.iso_run_last;
        bd.iso_runs[last].last_base = ON;
        let last_limit = bd.iso_runs[last].limit;
        bd.iso_run_last += 1;
        let r = &mut bd.iso_runs[bd.iso_run_last];
        r.start = last_limit;
        r.limit = last_limit;
        r.level = level;
        r.last_strong = level & 1;
        r.last_base = level & 1;
        r.context_dir = level & 1;
        r.context_pos = 0;
    }

    /// `bracketProcessPDI`.
    fn bracket_process_pdi(bd: &mut BracketData) {
        bd.iso_run_last -= 1;
        bd.iso_runs[bd.iso_run_last].last_base = ON;
    }

    /// `bracketAddOpening`: a newly found opening bracket.
    fn bracket_add_opening(bd: &mut BracketData, r#match: u16, position: i32) {
        let r = bd.iso_runs[bd.iso_run_last];
        let limit = r.limit as usize;
        if limit >= bd.openings.len() {
            // no available new entry
            bd.openings.resize(limit * 2, Opening::default());
        }
        bd.openings[limit] = Opening {
            position,
            r#match: r#match as i32,
            context_dir: r.context_dir,
            context_pos: r.context_pos,
            flags: 0,
        };
        let r = &mut bd.iso_runs[bd.iso_run_last];
        r.limit = r.limit.wrapping_add(1);
    }

    /// `fixN0c`: change N0c1 to N0c2 when a preceding bracket is assigned
    /// the embedding level.
    fn fix_n0c(
        &mut self,
        bd: &mut BracketData,
        opening_index: i32,
        new_prop_position: i32,
        new_prop: DirProp,
    ) {
        let limit = bd.iso_runs[bd.iso_run_last].limit as i32;
        let mut k = opening_index + 1;
        while k < limit {
            let q = bd.openings[k as usize];
            if q.r#match >= 0 {
                // not an N0c match
                k += 1;
                continue;
            }
            if new_prop_position < q.context_pos {
                break;
            }
            if new_prop_position >= q.position {
                k += 1;
                continue;
            }
            if new_prop == q.context_dir {
                break;
            }
            let opening_position = q.position;
            self.dir_props[opening_position as usize] = new_prop;
            let closing_position = -q.r#match;
            self.dir_props[closing_position as usize] = new_prop;
            bd.openings[k as usize].r#match = 0; // prevent further changes
            self.fix_n0c(bd, k, opening_position, new_prop);
            self.fix_n0c(bd, k, closing_position, new_prop);
            k += 1;
        }
    }

    /// `bracketProcessClosing`: L or R if N0b or N0c, ON if N0d.
    fn bracket_process_closing(
        &mut self,
        bd: &mut BracketData,
        open_idx: i32,
        position: i32,
    ) -> DirProp {
        let last = bd.iso_run_last;
        let p_opening = bd.openings[open_idx as usize];
        let direction = bd.iso_runs[last].level & 1;
        let mut stable = true; // assume stable until proved otherwise
        let new_prop = if (direction == 0 && p_opening.flags & FOUND_L != 0)
            || (direction == 1 && p_opening.flags & FOUND_R != 0)
        {
            direction // N0b
        } else if p_opening.flags & (FOUND_L | FOUND_R) != 0 {
            // N0c: stable if there is no containing pair or in conditions
            // too complicated and not worth checking
            stable = open_idx == bd.iso_runs[last].start as i32;
            if direction != p_opening.context_dir {
                p_opening.context_dir // N0c1
            } else {
                direction // N0c2
            }
        } else {
            // forget this and any brackets nested within this pair
            bd.iso_runs[last].limit = open_idx as u16;
            return ON; // N0d
        };
        self.dir_props[p_opening.position as usize] = new_prop;
        self.dir_props[position as usize] = new_prop;
        // Update nested N0c pairs that may be affected
        self.fix_n0c(bd, open_idx, p_opening.position, new_prop);
        if stable {
            // forget any brackets nested within this pair
            bd.iso_runs[last].limit = open_idx as u16;
            // remove lower located synonyms if any
            while bd.iso_runs[last].limit > bd.iso_runs[last].start
                && bd.openings[bd.iso_runs[last].limit as usize - 1].position == p_opening.position
            {
                bd.iso_runs[last].limit -= 1;
            }
        } else {
            bd.openings[open_idx as usize].r#match = -position;
            // neutralize lower located synonyms if any
            let mut k = open_idx - 1;
            while k >= bd.iso_runs[last].start as i32
                && bd.openings[k as usize].position == p_opening.position
            {
                bd.openings[k as usize].r#match = 0;
                k -= 1;
            }
            // neutralize any unmatched opening between the current pair;
            // this will also neutralize higher located synonyms if any
            k = open_idx + 1;
            while k < bd.iso_runs[last].limit as i32 {
                let q = &mut bd.openings[k as usize];
                if q.position >= position {
                    break;
                }
                if q.r#match > 0 {
                    q.r#match = 0;
                }
                k += 1;
            }
        }
        new_prop
    }

    /// `bracketProcessChar`: strong characters, digits and candidates for
    /// closing brackets.
    fn bracket_process_char(&mut self, bd: &mut BracketData, position: i32) {
        let last = bd.iso_run_last;
        let dir_prop = self.dir_props[position as usize];
        let mut new_prop;
        if dir_prop == ON {
            // First see if it is a matching closing bracket.
            let mut c = self.text[position as usize];
            let mut idx = bd.iso_runs[last].limit as i32 - 1;
            while idx >= bd.iso_runs[last].start as i32 {
                if bd.openings[idx as usize].r#match != c as i32 {
                    idx -= 1;
                    continue;
                }
                // We have a match
                new_prop = self.bracket_process_closing(bd, idx, position);
                if new_prop == ON {
                    // N0d
                    c = 0; // prevent handling as an opening
                    break;
                }
                let r = &mut bd.iso_runs[last];
                r.last_base = ON;
                r.context_dir = new_prop;
                r.context_pos = position;
                let level = self.levels[position as usize];
                if level & UBIDI_LEVEL_OVERRIDE != 0 {
                    // X4, X5
                    new_prop = level & 1;
                    bd.iso_runs[last].last_strong = new_prop;
                    let flag = dirprop_flag(new_prop) as u16;
                    let start = bd.iso_runs[last].start as usize;
                    for o in &mut bd.openings[start..idx as usize] {
                        o.flags |= flag;
                    }
                    // matching brackets are not overridden by LRO/RLO
                    self.levels[position as usize] &= !UBIDI_LEVEL_OVERRIDE;
                }
                // matching brackets are not overridden by LRO/RLO
                let op = bd.openings[idx as usize].position as usize;
                self.levels[op] &= !UBIDI_LEVEL_OVERRIDE;
                return;
            }
            // Not a matching closing bracket, or a case of N0d. Now see if it
            // is an opening bracket.
            let r#match: u16 = if c != 0 {
                get_paired_bracket(c as i32) as u16 // the matching char
            } else {
                0
            };
            if r#match != c && get_paired_bracket_type(c as i32) == U_BPT_OPEN {
                // special case: process synonyms; create an opening entry
                // for each synonym
                if r#match == 0x232A {
                    // RIGHT-POINTING ANGLE BRACKET
                    Self::bracket_add_opening(bd, 0x3009, position);
                } else if r#match == 0x3009 {
                    // RIGHT ANGLE BRACKET
                    Self::bracket_add_opening(bd, 0x232A, position);
                }
                Self::bracket_add_opening(bd, r#match, position);
            }
        }
        let level = self.levels[position as usize];
        let r = &mut bd.iso_runs[last];
        if level & UBIDI_LEVEL_OVERRIDE != 0 {
            // X4, X5
            new_prop = level & 1;
            if dir_prop != S && dir_prop != WS && dir_prop != ON {
                self.dir_props[position as usize] = new_prop;
            }
            r.last_base = new_prop;
            r.last_strong = new_prop;
            r.context_dir = new_prop;
            r.context_pos = position;
        } else if dir_prop <= R || dir_prop == AL {
            new_prop = dir_from_strong(dir_prop);
            r.last_base = dir_prop;
            r.last_strong = dir_prop;
            r.context_dir = new_prop;
            r.context_pos = position;
        } else if dir_prop == EN {
            r.last_base = EN;
            if r.last_strong == L {
                new_prop = L; // W7 (not NUMBERS_SPECIAL)
                self.dir_props[position as usize] = ENL;
                r.context_dir = L;
                r.context_pos = position;
            } else {
                new_prop = R; // N0
                if r.last_strong == AL {
                    self.dir_props[position as usize] = AN; // W2
                } else {
                    self.dir_props[position as usize] = ENR;
                }
                r.context_dir = R;
                r.context_pos = position;
            }
        } else if dir_prop == AN {
            new_prop = R; // N0
            r.last_base = AN;
            r.context_dir = R;
            r.context_pos = position;
        } else if dir_prop == NSM {
            // if the last real char was ON, change NSM to ON so that it
            // will stay ON even if the last real char is a bracket which
            // may be changed to L or R
            new_prop = r.last_base;
            if new_prop == ON {
                self.dir_props[position as usize] = new_prop;
            }
        } else {
            new_prop = dir_prop;
            r.last_base = dir_prop;
        }
        if new_prop <= R || new_prop == AL {
            let flag = dirprop_flag(dir_from_strong(new_prop)) as u16;
            let (start, limit) = (r.start as usize, r.limit as usize);
            for o in &mut bd.openings[start..limit] {
                if position > o.position {
                    o.flags |= flag;
                }
            }
        }
    }

    // ---- perform (X1)..(X9) ---------------------------------------------

    /// `directionFromFlags`.
    fn direction_from_flags(&self) -> Direction {
        let flags = self.flags;
        // if the text contains AN and neutrals, then some neutrals may
        // become RTL
        let rtl = flags & MASK_RTL != 0
            || (flags & dirprop_flag(AN) != 0 && flags & MASK_POSSIBLE_N != 0);
        if !rtl {
            Direction::Ltr
        } else if flags & MASK_LTR == 0 {
            Direction::Rtl
        } else {
            Direction::Mixed
        }
    }

    /// `resolveExplicitLevels`.
    fn resolve_explicit_levels(&mut self) -> Direction {
        let text = self.text;
        let length = self.length;
        let mut flags = self.flags;
        let mut level = self.get_paralevel(0);
        self.isolate_count = 0;

        // determine if the text is mixed-directional or single-directional
        let mut direction = self.direction_from_flags();

        // we may not need to resolve any explicit levels
        if direction != Direction::Mixed {
            // not mixed directionality: levels don't matter -
            // trailingWSStart will be 0
            return direction;
        }
        if flags & (MASK_EXPLICIT | MASK_ISO) == 0 {
            // no embeddings, set all levels to the paragraph level; we still
            // have to perform bracket matching
            let mut bd = self.bracket_init();
            for para_index in 0..self.para_count as usize {
                let start = if para_index == 0 {
                    0
                } else {
                    self.paras[para_index - 1].limit
                };
                let limit = self.paras[para_index].limit;
                level = self.paras[para_index].level as u8;
                for i in start..limit {
                    let iu = i as usize;
                    self.levels[iu] = level;
                    let dir_prop = self.dir_props[iu];
                    if dir_prop == BN {
                        continue;
                    }
                    if dir_prop == B {
                        if i + 1 < length {
                            if text[iu] == CR && text[iu + 1] == LF {
                                continue; // skip CR when followed by LF
                            }
                            Self::bracket_process_b(&mut bd, level);
                        }
                        continue;
                    }
                    self.bracket_process_char(&mut bd, i);
                }
            }
            return direction;
        }
        {
            // continue to perform (Xn)

            // (X1) level is set for all codes, embeddingLevel keeps track of
            // the push/pop operations; both may carry UBIDI_LEVEL_OVERRIDE
            let mut embedding_level = level;
            let mut previous_level = level; // for regular (not CC) characters
            let mut last_cc_pos = 0i32; // index of last effective LRx,RLx, PDx

            // The embedding level and the ISOLATE flag of level runs.
            let mut stack = [0u16; UBIDI_MAX_EXPLICIT_LEVEL as usize + 2];
            let mut stack_last = 0usize;
            let mut overflow_isolate_count = 0i32;
            let mut overflow_embedding_count = 0i32;
            let mut valid_isolate_count = 0i32;
            let mut bd = self.bracket_init();
            stack[0] = level as u16; // base entry: para level, no override, no isolate

            // recalculate the flags
            flags = 0;

            for i in 0..length {
                let iu = i as usize;
                let dir_prop = self.dir_props[iu];
                match dir_prop {
                    LRE | RLE | LRO | RLO => {
                        // (X2, X3, X4, X5)
                        flags |= dirprop_flag(BN);
                        self.levels[iu] = previous_level;
                        let new_level = if dir_prop == LRE || dir_prop == LRO {
                            // least greater even level
                            ((embedding_level as u32 + 2) & !(UBIDI_LEVEL_OVERRIDE as u32 | 1))
                                as u8
                        } else {
                            // least greater odd level
                            (no_override(embedding_level) + 1) | 1
                        };
                        if new_level <= UBIDI_MAX_EXPLICIT_LEVEL
                            && overflow_isolate_count == 0
                            && overflow_embedding_count == 0
                        {
                            last_cc_pos = i;
                            embedding_level = new_level;
                            if dir_prop == LRO || dir_prop == RLO {
                                embedding_level |= UBIDI_LEVEL_OVERRIDE;
                            }
                            stack_last += 1;
                            stack[stack_last] = embedding_level as u16;
                        } else if overflow_isolate_count == 0 {
                            overflow_embedding_count += 1;
                        }
                    }
                    PDF => {
                        // (X7)
                        flags |= dirprop_flag(BN);
                        self.levels[iu] = previous_level;
                        // handle all the overflow cases first
                        if overflow_isolate_count != 0 {
                            continue;
                        }
                        if overflow_embedding_count != 0 {
                            overflow_embedding_count -= 1;
                            continue;
                        }
                        if stack_last > 0 && stack[stack_last] < ISOLATE {
                            // not an isolate entry
                            last_cc_pos = i;
                            stack_last -= 1;
                            embedding_level = stack[stack_last] as u8;
                        }
                    }
                    LRI | RLI => {
                        flags |= dirprop_flag(ON) | dirprop_flag_lr(embedding_level);
                        self.levels[iu] = no_override(embedding_level);
                        if no_override(embedding_level) != no_override(previous_level) {
                            self.bracket_process_boundary(
                                &mut bd,
                                last_cc_pos,
                                previous_level,
                                embedding_level,
                            );
                            flags |= DIRPROP_FLAG_MULTI_RUNS;
                        }
                        previous_level = embedding_level;
                        // (X5a, X5b)
                        let new_level = if dir_prop == LRI {
                            // least greater even level
                            ((embedding_level as u32 + 2) & !(UBIDI_LEVEL_OVERRIDE as u32 | 1))
                                as u8
                        } else {
                            // least greater odd level
                            (no_override(embedding_level) + 1) | 1
                        };
                        if new_level <= UBIDI_MAX_EXPLICIT_LEVEL
                            && overflow_isolate_count == 0
                            && overflow_embedding_count == 0
                        {
                            flags |= dirprop_flag(dir_prop);
                            last_cc_pos = i;
                            valid_isolate_count += 1;
                            if valid_isolate_count > self.isolate_count {
                                self.isolate_count = valid_isolate_count;
                            }
                            embedding_level = new_level;
                            stack_last += 1;
                            stack[stack_last] = embedding_level as u16 + ISOLATE;
                            Self::bracket_process_lri_rli(&mut bd, embedding_level);
                        } else {
                            // make it WS so that it is handled by adjustWSLevels()
                            self.dir_props[iu] = WS;
                            overflow_isolate_count += 1;
                        }
                    }
                    PDI => {
                        if no_override(embedding_level) != no_override(previous_level) {
                            self.bracket_process_boundary(
                                &mut bd,
                                last_cc_pos,
                                previous_level,
                                embedding_level,
                            );
                            flags |= DIRPROP_FLAG_MULTI_RUNS;
                        }
                        // (X6a)
                        if overflow_isolate_count != 0 {
                            overflow_isolate_count -= 1;
                            // make it WS so that it is handled by adjustWSLevels()
                            self.dir_props[iu] = WS;
                        } else if valid_isolate_count != 0 {
                            flags |= dirprop_flag(PDI);
                            last_cc_pos = i;
                            overflow_embedding_count = 0;
                            while stack[stack_last] < ISOLATE {
                                // pop embedding entries until the last
                                // isolate entry
                                stack_last -= 1;
                            }
                            stack_last -= 1; // pop also the last isolate entry
                            valid_isolate_count -= 1;
                            Self::bracket_process_pdi(&mut bd);
                        } else {
                            // make it WS so that it is handled by adjustWSLevels()
                            self.dir_props[iu] = WS;
                        }
                        embedding_level = (stack[stack_last] & !ISOLATE) as u8;
                        flags |= dirprop_flag(ON) | dirprop_flag_lr(embedding_level);
                        previous_level = embedding_level;
                        self.levels[iu] = no_override(embedding_level);
                    }
                    B => {
                        flags |= dirprop_flag(B);
                        self.levels[iu] = self.get_paralevel(i);
                        if i + 1 < length {
                            if text[iu] == CR && text[iu + 1] == LF {
                                continue; // skip CR when followed by LF
                            }
                            overflow_embedding_count = 0;
                            overflow_isolate_count = 0;
                            valid_isolate_count = 0;
                            stack_last = 0;
                            embedding_level = self.get_paralevel(i + 1);
                            previous_level = embedding_level;
                            stack[0] = embedding_level as u16; // base entry
                            Self::bracket_process_b(&mut bd, embedding_level);
                        }
                    }
                    BN => {
                        // BN, LRE, RLE, and PDF are supposed to be removed
                        // (X9); they get their levels in adjustWSLevels()
                        self.levels[iu] = previous_level;
                        flags |= dirprop_flag(BN);
                    }
                    _ => {
                        // all other types are normal characters and get
                        // the "real" level
                        if no_override(embedding_level) != no_override(previous_level) {
                            self.bracket_process_boundary(
                                &mut bd,
                                last_cc_pos,
                                previous_level,
                                embedding_level,
                            );
                            flags |= DIRPROP_FLAG_MULTI_RUNS;
                            if embedding_level & UBIDI_LEVEL_OVERRIDE != 0 {
                                flags |= dirprop_flag_o(embedding_level);
                            } else {
                                flags |= dirprop_flag_e(embedding_level);
                            }
                        }
                        previous_level = embedding_level;
                        self.levels[iu] = embedding_level;
                        self.bracket_process_char(&mut bd, i);
                        // the dirProp may have been changed in
                        // bracketProcessChar()
                        flags |= dirprop_flag(self.dir_props[iu]);
                    }
                }
            }
            if flags & MASK_EMBEDDING != 0 {
                flags |= dirprop_flag_lr(self.para_level);
            }
            // again, determine if the text is mixed-directional or
            // single-directional
            self.flags = flags;
            direction = self.direction_from_flags();
        }
        direction
    }

    // ---- perform rules (Wn), (Nn), and (In) on a run of the text --------

    /// `setLevelsOutsideIsolates`.
    fn set_levels_outside_isolates(&mut self, start: i32, limit: i32, level: u8) {
        let mut isolate_count = 0i32;
        for k in start..limit {
            let ku = k as usize;
            let dir_prop = self.dir_props[ku];
            if dir_prop == PDI {
                isolate_count -= 1;
            }
            if isolate_count == 0 {
                self.levels[ku] = level;
            }
            if dir_prop == LRI || dir_prop == RLI {
                isolate_count += 1;
            }
        }
    }

    /// `processPropertySeq` with `impTab_DEFAULT` (whose action maps,
    /// `impAct0`, are the identity on actions 1..4).
    fn process_property_seq(&mut self, lev: &mut LevState, prop: u8, mut start: i32, limit: i32) {
        let imp_tab: &ImpTab = if lev.run_level & 1 == 0 {
            &IMP_TAB_L_DEFAULT
        } else {
            &IMP_TAB_R_DEFAULT
        };
        let start0 = start; // save original start position
        let old_state_seq = lev.state as usize;
        let cell = imp_tab[old_state_seq][prop as usize];
        lev.state = get_state(cell) as i32; // isolate the new state
        let action_seq = IMP_ACT0[get_action(cell) as usize]; // isolate the action
        let add_level = imp_tab[lev.state as usize][IMPTABLEVELS_RES];

        if action_seq != 0 {
            match action_seq {
                1 => {
                    // init ON seq
                    lev.start_on = start0;
                }
                2 => {
                    // prepend ON seq to current seq
                    start = lev.start_on;
                }
                3 => {
                    // EN/AN after R+ON
                    let level = lev.run_level + 1;
                    self.set_levels_outside_isolates(lev.start_on, start0, level);
                }
                4 => {
                    // EN/AN before R for NUMBERS_SPECIAL
                    let level = lev.run_level + 2;
                    self.set_levels_outside_isolates(lev.start_on, start0, level);
                }
                _ => unreachable!("processPropertySeq: action {action_seq}"),
            }
        }
        if add_level != 0 || start < start0 {
            let level = lev.run_level + add_level;
            if start >= lev.run_start {
                self.levels[start as usize..limit as usize].fill(level);
            } else {
                self.set_levels_outside_isolates(start, limit, level);
            }
        }
    }

    /// `resolveImplicitLevels`.
    fn resolve_implicit_levels(&mut self, start: i32, limit: i32, sor: DirProp, eor: DirProp) {
        let run_level = self.levels[start as usize];
        let mut lev = LevState {
            start_on: 0,
            state: 0,
            run_start: start,
            run_level,
        };
        // The isolates[] entries contain enough information to resume the
        // bidi algorithm in the same state as it was when it was
        // interrupted by an isolate sequence.
        let (mut start1, mut state_imp): (i32, u16) =
            if self.dir_props[start as usize] == PDI && self.isolate_count >= 0 {
                let iso = self.isolates[self.isolate_count as usize];
                lev.start_on = iso.start_on;
                lev.state = iso.state;
                self.isolate_count -= 1;
                (iso.start1, iso.state_imp as u16)
            } else {
                lev.start_on = -1;
                let state_imp = if self.dir_props[start as usize] == NSM {
                    1 + sor as u16
                } else {
                    0
                };
                lev.state = 0;
                self.process_property_seq(&mut lev, sor, start, start);
                (start, state_imp)
            };
        let mut start2 = start;

        let mut i = start;
        while i <= limit {
            let gprop = if i >= limit {
                let mut k = limit - 1;
                while k > start && dirprop_flag(self.dir_props[k as usize]) & MASK_BN_EXPLICIT != 0
                {
                    k -= 1;
                }
                let dir_prop = self.dir_props[k as usize];
                if dir_prop == LRI || dir_prop == RLI {
                    break; // no forced closing for sequence ending with LRI/RLI
                }
                eor
            } else {
                let prop = self.dir_props[i as usize];
                if prop == B {
                    self.isolate_count = -1; // current isolates stack entry == none
                }
                GROUP_PROP[prop as usize]
            };
            let old_state_imp = state_imp;
            let cell = IMP_TAB_PROPS[old_state_imp as usize][gprop as usize];
            state_imp = get_stateprops(cell) as u16; // isolate the new state
            let mut action_imp = get_actionprops(cell); // isolate the action
            if i == limit && action_imp == 0 {
                // there is an unprocessed sequence if its property == eor
                action_imp = 1; // process the last sequence
            }
            if action_imp != 0 {
                let res_prop = IMP_TAB_PROPS[old_state_imp as usize][IMPTABPROPS_RES];
                match action_imp {
                    1 => {
                        // process current seq1, init new seq1
                        self.process_property_seq(&mut lev, res_prop, start1, i);
                        start1 = i;
                    }
                    2 => {
                        // init new seq2
                        start2 = i;
                    }
                    3 => {
                        // process seq1, process seq2, init new seq1
                        self.process_property_seq(&mut lev, res_prop, start1, start2);
                        self.process_property_seq(&mut lev, DIRPROP_ON, start2, i);
                        start1 = i;
                    }
                    4 => {
                        // process seq1, set seq1=seq2, init new seq2
                        self.process_property_seq(&mut lev, res_prop, start1, start2);
                        start1 = start2;
                        start2 = i;
                    }
                    _ => unreachable!("resolveImplicitLevels: action {action_imp}"),
                }
            }
            i += 1;
        }

        // look for the last char not a BN or LRE/RLE/LRO/RLO/PDF
        let mut i = limit - 1;
        while i > start && dirprop_flag(self.dir_props[i as usize]) & MASK_BN_EXPLICIT != 0 {
            i -= 1;
        }
        let dir_prop = self.dir_props[i as usize];
        if (dir_prop == LRI || dir_prop == RLI) && limit < self.length {
            self.isolate_count += 1;
            let n = self.isolate_count as usize;
            if self.isolates.len() <= n {
                self.isolates.resize(n + 1, Isolate::default());
            }
            self.isolates[n] = Isolate {
                state_imp: state_imp as i16,
                state: lev.state,
                start1,
                start_on: lev.start_on,
            };
        } else {
            self.process_property_seq(&mut lev, eor, limit, limit);
        }
    }

    // ---- perform (L1) and (X9) ------------------------------------------

    /// `adjustWSLevels`: reset the embedding levels for some non-graphic
    /// characters (L1); levels for BN and the explicit embedding types that
    /// (X9) removes.
    fn adjust_ws_levels(&mut self) {
        if self.flags & MASK_WS == 0 {
            return;
        }
        let mut i = self.trailing_ws_start;
        while i > 0 {
            // reset a sequence of WS/BN before eop and B/S to the
            // paragraph paraLevel
            while i > 0 {
                i -= 1;
                let flag = dirprop_flag(self.dir_props[i as usize]);
                if flag & MASK_WS == 0 {
                    break;
                }
                self.levels[i as usize] = self.get_paralevel(i);
            }
            // reset BN to the next character's paraLevel until B/S, which
            // restarts above loop; here, i+1 is guaranteed to be <length
            while i > 0 {
                i -= 1;
                let flag = dirprop_flag(self.dir_props[i as usize]);
                if flag & MASK_BN_EXPLICIT != 0 {
                    self.levels[i as usize] = self.levels[i as usize + 1];
                } else if flag & MASK_B_S != 0 {
                    self.levels[i as usize] = self.get_paralevel(i);
                    break;
                }
            }
        }
    }

    // ---- ubidiln.cpp: the runs ------------------------------------------

    /// `getSingleRun`.
    fn get_single_run(&mut self, level: u8) {
        self.runs = vec![Run {
            logical_start: make_index_odd_pair(0, level),
            visual_limit: self.length,
        }];
        self.run_count = 1;
    }

    /// `reorderLine` (L2): reorder the same-level runs in the runs array.
    fn reorder_line(&mut self, mut min_level: u8, mut max_level: u8) {
        // nothing to do?
        if max_level <= (min_level | 1) {
            return;
        }
        // Reorder only down to the lowest odd level and reorder at an odd
        // minLevel in a separate, simpler loop.
        min_level += 1;

        let runs = &mut self.runs;
        let levels = &self.levels;
        let mut run_count = self.run_count;

        // do not include the WS run at paraLevel<=old minLevel except in
        // the simple loop
        if self.trailing_ws_start < self.length {
            run_count -= 1;
        }

        loop {
            max_level -= 1;
            if max_level < min_level {
                break;
            }
            let mut first_run = 0i32;
            // loop for all sequences of runs
            loop {
                // look for a sequence of runs that are all at >=maxLevel;
                // look for the first run of such a sequence
                while first_run < run_count
                    && levels[runs[first_run as usize].logical_start as usize] < max_level
                {
                    first_run += 1;
                }
                if first_run >= run_count {
                    break; // no more such runs
                }
                // look for the limit run of such a sequence (the run behind it)
                let mut limit_run = first_run;
                loop {
                    limit_run += 1;
                    let more = limit_run < run_count
                        && levels[runs[limit_run as usize].logical_start as usize] >= max_level;
                    if !more {
                        break;
                    }
                }
                // Swap the entire sequence of runs from firstRun to limitRun-1.
                let mut end_run = limit_run - 1;
                while first_run < end_run {
                    runs.swap(first_run as usize, end_run as usize);
                    first_run += 1;
                    end_run -= 1;
                }
                if limit_run == run_count {
                    break; // no more such runs
                } else {
                    first_run = limit_run + 1;
                }
            }
        }

        // now do maxLevel==old minLevel (==odd!), see above
        if min_level & 1 == 0 {
            let mut first_run = 0i32;
            // include the trailing WS run in this complete reordering
            if self.trailing_ws_start == self.length {
                run_count -= 1;
            }
            // Swap the entire sequence of all runs. (endRun==runCount)
            while first_run < run_count {
                runs.swap(first_run as usize, run_count as usize);
                first_run += 1;
                run_count -= 1;
            }
        }
    }

    /// `ubidi_getRuns`: the runs array from the levels array, reordered.
    fn get_runs(&mut self) {
        // returns immediately if the runs are already set (length==0
        // included)
        if self.run_count >= 0 {
            return;
        }
        if self.direction != Direction::Mixed {
            // simple, single-run case
            self.get_single_run(self.para_level);
            return;
        }
        // mixed directionality
        let length = self.length;
        let mut level = UBIDI_DEFAULT_LTR; // initialize with no valid level
        let mut limit = self.trailing_ws_start;
        // count the runs, there is at least one non-WS run, and limit>0
        let mut run_count = 0i32;
        for i in 0..limit {
            // increment runCount at the start of each run
            if self.levels[i as usize] != level {
                run_count += 1;
                level = self.levels[i as usize];
            }
        }
        if run_count == 1 && limit == length {
            // There is only one non-WS run and no trailing WS-run.
            self.get_single_run(self.levels[0]);
            return;
        }
        let mut min_level = UBIDI_MAX_EXPLICIT_LEVEL + 1;
        let mut max_level = 0u8;
        // now, count a (non-mergeable) WS run
        if limit < length {
            run_count += 1;
        }
        let mut runs = vec![Run::default(); run_count as usize];
        let mut run_index = 0usize;
        // search for the run limits and initialize visualLimit values with
        // the run lengths
        let mut i = 0i32;
        loop {
            // prepare this run
            let start = i;
            level = self.levels[i as usize];
            if level < min_level {
                min_level = level;
            }
            if level > max_level {
                max_level = level;
            }
            // look for the run limit
            loop {
                i += 1;
                let more = i < limit && self.levels[i as usize] == level;
                if !more {
                    break;
                }
            }
            // i is another run limit
            runs[run_index] = Run {
                logical_start: start,
                visual_limit: i - start,
            };
            run_index += 1;
            if i >= limit {
                break;
            }
        }
        if limit < length {
            // there is a separate WS run
            runs[run_index] = Run {
                logical_start: limit,
                visual_limit: length - limit,
            };
            if self.para_level < min_level {
                min_level = self.para_level;
            }
        }
        // set the object fields
        self.runs = runs;
        self.run_count = run_count;

        self.reorder_line(min_level, max_level);

        // now add the direction flags and adjust the visualLimit's to be
        // just that; this loop will also handle the trailing WS run
        limit = 0;
        for r in self.runs.iter_mut() {
            let lvl = self.levels[r.logical_start as usize];
            r.logical_start |= ((lvl & 1) as i32) << 31; // ADD_ODD_BIT_FROM_LEVEL
            limit += r.visual_limit;
            r.visual_limit = limit;
        }
        // Set the "odd" bit for the trailing WS run. For a RTL paragraph,
        // it will be the *first* run in visual order.
        if (run_index as i32) < self.run_count {
            let trailing_run = if self.para_level & 1 != 0 {
                0
            } else {
                run_index
            };
            self.runs[trailing_run].logical_start |= ((self.para_level & 1) as i32) << 31;
        }
    }
}

/// `MAKE_INDEX_ODD_PAIR`.
fn make_index_odd_pair(index: i32, level: u8) -> i32 {
    index | (((level & 1) as i32) << 31)
}

/// `LevState`, without the fields only the inverse modes read.
struct LevState {
    start_on: i32,
    state: i32,
    run_start: i32,
    run_level: u8,
}

// ---- the Properties state machine table ---------------------------------

const IMPTABPROPS_COLUMNS: usize = 16;
const IMPTABPROPS_RES: usize = IMPTABPROPS_COLUMNS - 1;
fn get_stateprops(cell: u8) -> u8 {
    cell & 0x1f
}
fn get_actionprops(cell: u8) -> u8 {
    cell >> 5
}
/// `s(action, newState)` of the properties table.
const fn sp(action: u8, new_state: u8) -> u8 {
    new_state + (action << 5)
}

/// `groupProp`: dirProp regrouped.
#[rustfmt::skip]
const GROUP_PROP: [u8; 25] = [
//  L   R   EN  ES  ET  AN  CS  B   S   WS  ON  LRE LRO AL  RLE RLO PDF NSM BN  FSI LRI RLI PDI ENL ENR
    0,  1,  2,  7,  8,  3,  9,  6,  5,  4,  4,  10, 10, 12, 10, 10, 10, 11, 10, 4,  4,  4,  4,  13, 14,
];
// reduced dirProp
const DIRPROP_L: u8 = 0;
const DIRPROP_R: u8 = 1;
const DIRPROP_EN: u8 = 2;
const DIRPROP_AN: u8 = 3;
const DIRPROP_ON: u8 = 4;
const DIRPROP_S: u8 = 5;
const DIRPROP_B: u8 = 6;

/// `impTabProps`.
#[rustfmt::skip]
const IMP_TAB_PROPS: [[u8; IMPTABPROPS_COLUMNS]; 24] = [
//                        L ,     R ,    EN ,    AN ,    ON ,     S ,     B ,    ES ,    ET ,    CS ,    BN ,   NSM ,    AL ,   ENL ,   ENR , Res
/* 0 Init        */ [     1 ,     2 ,     4 ,     5 ,     7 ,    15 ,    17 ,     7 ,     9 ,     7 ,     0 ,     7 ,     3 ,    18 ,    21 , DIRPROP_ON ],
/* 1 L           */ [     1 ,sp(1,2),sp(1,4),sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(1,7),sp(1,9),sp(1,7),    1 ,     1 ,sp(1,3),sp(1,18),sp(1,21),  DIRPROP_L ],
/* 2 R           */ [sp(1,1),     2 ,sp(1,4),sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(1,7),sp(1,9),sp(1,7),    2 ,     2 ,sp(1,3),sp(1,18),sp(1,21),  DIRPROP_R ],
/* 3 AL          */ [sp(1,1),sp(1,2),sp(1,6),sp(1,6),sp(1,8),sp(1,16),sp(1,17),sp(1,8),sp(1,8),sp(1,8),    3 ,     3 ,     3 ,sp(1,18),sp(1,21),  DIRPROP_R ],
/* 4 EN          */ [sp(1,1),sp(1,2),     4 ,sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(2,10),   11 ,sp(2,10),    4 ,     4 ,sp(1,3),    18 ,    21 , DIRPROP_EN ],
/* 5 AN          */ [sp(1,1),sp(1,2),sp(1,4),     5 ,sp(1,7),sp(1,15),sp(1,17),sp(1,7),sp(1,9),sp(2,12),    5 ,     5 ,sp(1,3),sp(1,18),sp(1,21), DIRPROP_AN ],
/* 6 AL:EN/AN    */ [sp(1,1),sp(1,2),     6 ,     6 ,sp(1,8),sp(1,16),sp(1,17),sp(1,8),sp(1,8),sp(2,13),    6 ,     6 ,sp(1,3),    18 ,    21 , DIRPROP_AN ],
/* 7 ON          */ [sp(1,1),sp(1,2),sp(1,4),sp(1,5),     7 ,sp(1,15),sp(1,17),     7 ,sp(2,14),     7 ,     7 ,     7 ,sp(1,3),sp(1,18),sp(1,21), DIRPROP_ON ],
/* 8 AL:ON       */ [sp(1,1),sp(1,2),sp(1,6),sp(1,6),     8 ,sp(1,16),sp(1,17),     8 ,     8 ,     8 ,     8 ,     8 ,sp(1,3),sp(1,18),sp(1,21), DIRPROP_ON ],
/* 9 ET          */ [sp(1,1),sp(1,2),     4 ,sp(1,5),     7 ,sp(1,15),sp(1,17),     7 ,     9 ,     7 ,     9 ,     9 ,sp(1,3),    18 ,    21 , DIRPROP_ON ],
/*10 EN+ES/CS    */ [sp(3,1),sp(3,2),     4 ,sp(3,5),sp(4,7),sp(3,15),sp(3,17),sp(4,7),sp(4,14),sp(4,7),   10 ,sp(4,7),sp(3,3),    18 ,    21 , DIRPROP_EN ],
/*11 EN+ET       */ [sp(1,1),sp(1,2),     4 ,sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(1,7),    11 ,sp(1,7),   11 ,    11 ,sp(1,3),    18 ,    21 , DIRPROP_EN ],
/*12 AN+CS       */ [sp(3,1),sp(3,2),sp(3,4),     5 ,sp(4,7),sp(3,15),sp(3,17),sp(4,7),sp(4,14),sp(4,7),   12 ,sp(4,7),sp(3,3),sp(3,18),sp(3,21), DIRPROP_AN ],
/*13 AL:EN/AN+CS */ [sp(3,1),sp(3,2),     6 ,     6 ,sp(4,8),sp(3,16),sp(3,17),sp(4,8),sp(4,8),sp(4,8),   13 ,sp(4,8),sp(3,3),    18 ,    21 , DIRPROP_AN ],
/*14 ON+ET       */ [sp(1,1),sp(1,2),sp(4,4),sp(1,5),     7 ,sp(1,15),sp(1,17),     7 ,    14 ,     7 ,   14 ,    14 ,sp(1,3),sp(4,18),sp(4,21), DIRPROP_ON ],
/*15 S           */ [sp(1,1),sp(1,2),sp(1,4),sp(1,5),sp(1,7),    15 ,sp(1,17),sp(1,7),sp(1,9),sp(1,7),   15 ,sp(1,7),sp(1,3),sp(1,18),sp(1,21),  DIRPROP_S ],
/*16 AL:S        */ [sp(1,1),sp(1,2),sp(1,6),sp(1,6),sp(1,8),    16 ,sp(1,17),sp(1,8),sp(1,8),sp(1,8),   16 ,sp(1,8),sp(1,3),sp(1,18),sp(1,21),  DIRPROP_S ],
/*17 B           */ [sp(1,1),sp(1,2),sp(1,4),sp(1,5),sp(1,7),sp(1,15),    17 ,sp(1,7),sp(1,9),sp(1,7),   17 ,sp(1,7),sp(1,3),sp(1,18),sp(1,21),  DIRPROP_B ],
/*18 ENL         */ [sp(1,1),sp(1,2),    18 ,sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(2,19),   20 ,sp(2,19),   18 ,    18 ,sp(1,3),    18 ,    21 ,  DIRPROP_L ],
/*19 ENL+ES/CS   */ [sp(3,1),sp(3,2),    18 ,sp(3,5),sp(4,7),sp(3,15),sp(3,17),sp(4,7),sp(4,14),sp(4,7),   19 ,sp(4,7),sp(3,3),    18 ,    21 ,  DIRPROP_L ],
/*20 ENL+ET      */ [sp(1,1),sp(1,2),    18 ,sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(1,7),    20 ,sp(1,7),   20 ,    20 ,sp(1,3),    18 ,    21 ,  DIRPROP_L ],
/*21 ENR         */ [sp(1,1),sp(1,2),    21 ,sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(2,22),   23 ,sp(2,22),   21 ,    21 ,sp(1,3),    18 ,    21 , DIRPROP_AN ],
/*22 ENR+ES/CS   */ [sp(3,1),sp(3,2),    21 ,sp(3,5),sp(4,7),sp(3,15),sp(3,17),sp(4,7),sp(4,14),sp(4,7),   22 ,sp(4,7),sp(3,3),    18 ,    21 , DIRPROP_AN ],
/*23 ENR+ET      */ [sp(1,1),sp(1,2),    21 ,sp(1,5),sp(1,7),sp(1,15),sp(1,17),sp(1,7),    23 ,sp(1,7),   23 ,    23 ,sp(1,3),    18 ,    21 , DIRPROP_AN ],
];

// ---- the levels state machine tables ------------------------------------

const IMPTABLEVELS_COLUMNS: usize = DIRPROP_B as usize + 2;
const IMPTABLEVELS_RES: usize = IMPTABLEVELS_COLUMNS - 1;
fn get_state(cell: u8) -> u8 {
    cell & 0x0f
}
fn get_action(cell: u8) -> u8 {
    cell >> 4
}
/// `s(action, newState)` of the levels tables.
const fn sl(action: u8, new_state: u8) -> u8 {
    new_state + (action << 4)
}
type ImpTab = [[u8; IMPTABLEVELS_COLUMNS]; 6];

/// `impTabL_DEFAULT`: even paragraph level. Conditional sequences receive
/// the lower possible level until proven otherwise.
#[rustfmt::skip]
const IMP_TAB_L_DEFAULT: ImpTab = [
//                         L ,     R ,    EN ,    AN ,    ON ,     S ,     B , Res
/* 0 : init       */ [     0 ,     1 ,     0 ,     2 ,     0 ,     0 ,     0 ,  0 ],
/* 1 : R          */ [     0 ,     1 ,     3 ,     3 ,sl(1,4),sl(1,4),     0 ,  1 ],
/* 2 : AN         */ [     0 ,     1 ,     0 ,     2 ,sl(1,5),sl(1,5),     0 ,  2 ],
/* 3 : R+EN/AN    */ [     0 ,     1 ,     3 ,     3 ,sl(1,4),sl(1,4),     0 ,  2 ],
/* 4 : R+ON       */ [     0 ,sl(2,1),sl(3,3),sl(3,3),     4 ,     4 ,     0 ,  0 ],
/* 5 : AN+ON      */ [     0 ,sl(2,1),     0 ,sl(3,2),     5 ,     5 ,     0 ,  0 ],
];
/// `impTabR_DEFAULT`: odd paragraph level.
#[rustfmt::skip]
const IMP_TAB_R_DEFAULT: ImpTab = [
//                         L ,     R ,    EN ,    AN ,    ON ,     S ,     B , Res
/* 0 : init       */ [     1 ,     0 ,     2 ,     2 ,     0 ,     0 ,     0 ,  0 ],
/* 1 : L          */ [     1 ,     0 ,     1 ,     3 ,sl(1,4),sl(1,4),     0 ,  1 ],
/* 2 : EN/AN      */ [     1 ,     0 ,     2 ,     2 ,     0 ,     0 ,     0 ,  1 ],
/* 3 : L+AN       */ [     1 ,     0 ,     1 ,     3 ,     5 ,     5 ,     0 ,  1 ],
/* 4 : L+ON       */ [sl(2,1),     0 ,sl(2,1),     3 ,     4 ,     4 ,     0 ,  0 ],
/* 5 : L+AN+ON    */ [     1 ,     0 ,     1 ,     3 ,     5 ,     5 ,     0 ,  0 ],
];
/// `impAct0`.
const IMP_ACT0: [u8; 5] = [0, 1, 2, 3, 4];

#[cfg(test)]
mod tests;
