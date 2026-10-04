//! The charstrings and subrs of a font being subset, and writet1.c's
//! `cs_mark`: which of them the glyphs a document uses need.
//!
//! pdfTeX interprets just enough of each charstring to follow its
//! `callsubr`s (with the operand stack, because a subr's number is an
//! operand) and the two glyphs of a `seac`, and checks the operand counts
//! as it goes. pdfTeX's `cs_mark` recurses for each call; here the calls are
//! an explicit stack of [`Frame`]s, so no font can exhaust the machine stack
//! (#1237), and each frame resumes after its call exactly where pdfTeX's
//! recursion returns to.

use super::cipher::{self, Cipher};
use super::encoding::standard_glyph_name;
use super::{Fail, Host, Result};
use crate::pdftex::fonts::NOTDEF;
use std::collections::{BTreeSet, HashMap};

/// A glyph's walk may take this many charstring operations per byte of the
/// font's charstrings and subrs (plus `WORK_SLACK`); past that it is a
/// font error. pdfTeX has no such limit. A walk that ends in pdfTeX parses
/// each subr it calls once per call, and the fonts that take more than
/// this call the same subrs over and over, the stack growing: pdfTeX
/// recurses until its C stack runs out.
const WORK_FACTOR: u64 = 16;
const WORK_SLACK: u64 = 1 << 16;

/// The charstring operators `cs_mark` knows (writet1.c's `CS_*` and
/// `cc_init`); an escaped operator `12 x` has the value `32 + x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    HStem,
    VStem,
    VMoveTo,
    RLineTo,
    HLineTo,
    VLineTo,
    RRCurveTo,
    ClosePath,
    CallSubr,
    Return,
    HSbw,
    EndChar,
    RMoveTo,
    HMoveTo,
    VHCurveTo,
    HVCurveTo,
    DotSection,
    VStem3,
    HStem3,
    Seac,
    Sbw,
    Div,
    CallOtherSubr,
    Pop,
    SetCurrentPoint,
}

/// The one-byte operator that escapes the next byte.
const ESCAPE: i32 = 12;
/// One more than the largest operator value (`CS_MAX`).
const OP_LIMIT: i32 = 66;

impl Op {
    fn from_value(v: i32) -> Option<Op> {
        use Op::*;
        Some(match v {
            1 => HStem,
            3 => VStem,
            4 => VMoveTo,
            5 => RLineTo,
            6 => HLineTo,
            7 => VLineTo,
            8 => RRCurveTo,
            9 => ClosePath,
            10 => CallSubr,
            11 => Return,
            13 => HSbw,
            14 => EndChar,
            21 => RMoveTo,
            22 => HMoveTo,
            30 => VHCurveTo,
            31 => HVCurveTo,
            32 => DotSection,
            33 => VStem3,
            34 => HStem3,
            38 => Seac,
            39 => Sbw,
            44 => Div,
            48 => CallOtherSubr,
            49 => Pop,
            65 => SetCurrentPoint,
            _ => return None,
        })
    }

    /// For an operator that takes its operands from the bottom of the stack
    /// (`cc_entry.bottom`), how many it takes: the stack must hold exactly
    /// that many.
    fn operands(self) -> Option<usize> {
        use Op::*;
        match self {
            HStem | VStem | RLineTo | HSbw | RMoveTo | SetCurrentPoint => Some(2),
            VMoveTo | HLineTo | VLineTo | HMoveTo => Some(1),
            RRCurveTo | VStem3 | HStem3 => Some(6),
            VHCurveTo | HVCurveTo | Sbw => Some(4),
            Seac => Some(5),
            ClosePath | CallSubr | Return | EndChar | DotSection | Div | CallOtherSubr | Pop => {
                None
            }
        }
    }

    /// Whether the operator clears the stack (`cc_entry.clear`).
    fn clears(self) -> bool {
        !matches!(
            self,
            Op::CallSubr | Op::Return | Op::Div | Op::CallOtherSubr | Op::Pop
        )
    }
}

/// A charstring or subr as the font gives it (`cs_entry`).
#[derive(Clone, Default)]
pub(super) struct Charstring {
    /// The glyph name (empty for a subr).
    pub name: Vec<u8>,
    /// What follows the length on the font's line: ` RD ` (or ` -| `), the
    /// encrypted charstring, the rest of the line and LF.
    pub data: Vec<u8>,
    /// The length of the encrypted charstring.
    pub cs_len: u16,
    pub used: bool,
    pub valid: bool,
}

impl Charstring {
    /// `append_cs_return`: end a subr with `return`, which its last
    /// command was not.
    fn append_return(&mut self) {
        let n = usize::from(self.cs_len);
        let code = self.data.get(4..4 + n).unwrap_or_default();
        let mut key = Cipher::CHARSTRING;
        let plain = code.iter().map(|&c| key.decrypt(c)).chain([11]);
        let mut data = self.data[..4.min(self.data.len())].to_vec();
        data.extend(cipher::encrypt(
            Cipher::CHARSTRING,
            plain.collect::<Vec<_>>(),
        ));
        data.extend_from_slice(self.data.get(4 + n..).unwrap_or_default());
        self.data = data;
        self.cs_len = self.cs_len.wrapping_add(1);
    }
}

/// What a walk marks: a glyph by name, or a subr by number.
#[derive(Clone, Copy)]
pub(super) enum Target<'a> {
    Glyph(&'a [u8]),
    Subr(i32),
}

/// Whose charstring a frame parses, for an error message (`cs_fail`).
#[derive(Clone)]
enum Owner {
    Glyph(Vec<u8>),
    Subr(i32),
}

impl Owner {
    fn error(&self, msg: impl std::fmt::Display) -> Fail {
        match self {
            Owner::Subr(s) => Fail(format!("Subr ({s}): {msg}")),
            Owner::Glyph(n) => Fail(format!(
                "CharString (/{}): {msg}",
                String::from_utf8_lossy(n)
            )),
        }
    }
}

/// The operand stack (`cc_stack`), with two running hashes of its
/// contents for [`EntryState`].
#[derive(Default)]
struct Stack {
    values: Vec<i32>,
    hashes: Vec<(u64, u64)>,
}

impl Stack {
    /// `cc_get(n)`: from the bottom for `n >= 0`, from the top for `n < 0`;
    /// 0 outside the stack.
    fn get(&self, n: i32) -> i32 {
        let i = if n < 0 {
            self.values.len() as i64 + i64::from(n)
        } else {
            i64::from(n)
        };
        usize::try_from(i)
            .ok()
            .and_then(|i| self.values.get(i))
            .copied()
            .unwrap_or(0)
    }

    /// `cc_pop(n)`.
    fn pop(&mut self, n: i32) -> Result<()> {
        if (self.values.len() as i64) < i64::from(n) {
            return Err(Fail(format!(
                "CharString: invalid access ({n}) to stack ({} entries)",
                self.values.len()
            )));
        }
        let keep = self.values.len() - n.max(0) as usize;
        self.values.truncate(keep);
        self.hashes.truncate(keep);
        Ok(())
    }

    /// `cc_push(v)`. pdfTeX writes past its 24-entry `cc_stack` unchecked
    /// (undefined behaviour: whether pdfTeX survives it depends on the
    /// platform); this stack just grows.
    fn push(&mut self, v: i32) {
        let (a, b) = self.fingerprint_or((0x243F_6A88_85A3_08D3, 0x1319_8A2E_0370_7344));
        let x = u64::from(v as u32);
        self.hashes.push((
            a.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(x),
            b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).wrapping_add(x),
        ));
        self.values.push(v);
    }

    fn clear(&mut self) {
        self.values.clear();
        self.hashes.clear();
    }

    fn fingerprint_or(&self, empty: (u64, u64)) -> (u64, u64) {
        self.hashes.last().copied().unwrap_or(empty)
    }
}

/// The state a subr is entered in, as `cs_mark` compares them: the subr,
/// the stack's depth and hashes, `lastargOtherSubr3`, and how many marks
/// and fixes the font has had. What pdfTeX's walk from a subr's entry does
/// depends on nothing else.
#[derive(Clone, PartialEq, Eq, Hash)]
struct EntryState {
    subr: usize,
    depth: usize,
    hash: (u64, u64),
    last_arg_other_subr3: i32,
    marks: u64,
}

/// What a frame does when the call it made returns.
enum Resume {
    Nothing,
    /// `mark_subr(n)` returned: check that the subr is valid.
    CallSubr(i32),
    /// `seac`: the base glyph's walk returned; mark the accent next.
    SeacAccent(&'static [u8], &'static [u8]),
    /// `seac`: both returned; put them into the glyph set.
    SeacDone(&'static [u8], &'static [u8]),
}

/// One entry being parsed: the locals of one call of pdfTeX's `cs_mark`.
struct Frame {
    owner: Owner,
    /// The entry in [`Charstrings::glyphs`] or [`Charstrings::subrs`].
    idx: usize,
    /// The encrypted charstring (a copy: a subr fixed by `append_return`
    /// while an outer call of it is parsed changes only later walks), the
    /// next byte to read, and the decryption key.
    data: Vec<u8>,
    at: usize,
    key: Cipher,
    /// `cs_len`: the bytes left (negative once a number overruns the end).
    left: i32,
    last_op: Option<Op>,
    resume: Resume,
    /// For a subr, the state it was entered in.
    entered: Option<EntryState>,
}

impl Frame {
    /// `cs_getchar()`: the next byte, decrypted (0 past the data).
    fn next(&mut self) -> u8 {
        let b = self.data.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        self.key.decrypt(b)
    }

    /// The number a byte `v >= 32` starts (Type 1 Font Format 6.2).
    fn number(&mut self, v: u8) -> i32 {
        let v = i32::from(v);
        match v {
            32..=246 => v - 139,
            247..=250 => {
                self.left -= 1;
                ((v - 247) << 8) + 108 + i32::from(self.next())
            }
            251..=254 => {
                self.left -= 1;
                -((v - 251) << 8) - 108 - i32::from(self.next())
            }
            _ => {
                self.left -= 4;
                i32::from_be_bytes([self.next(), self.next(), self.next(), self.next()])
            }
        }
    }
}

/// The walks under way: the frames, and the entry states of the subrs
/// being parsed.
#[derive(Default)]
struct Walk {
    frames: Vec<Frame>,
    /// How many frames were entered in each state.
    active: HashMap<EntryState, u32>,
    /// For a state entered a second time, the stack it was entered with
    /// and the depth of the frame that saw it.
    confirm: HashMap<EntryState, (Vec<i32>, usize)>,
}

impl Walk {
    fn push(&mut self, f: Frame) {
        if let Some(k) = &f.entered {
            *self.active.entry(k.clone()).or_insert(0) += 1;
        }
        self.frames.push(f);
    }

    fn pop(&mut self) -> Option<Frame> {
        let f = self.frames.pop()?;
        if let Some(k) = &f.entered {
            if let Some(n) = self.active.get_mut(k) {
                *n -= 1;
                if *n == 0 {
                    self.active.remove(k);
                }
            }
            if self
                .confirm
                .get(k)
                .is_some_and(|c| c.1 == self.frames.len())
            {
                self.confirm.remove(k);
            }
        }
        Some(f)
    }
}

/// What a walk reads and changes outside the charstrings.
pub(super) struct MarkEnv<'a> {
    /// `lenIV`: the random bytes that start each charstring.
    pub len_iv: i16,
    /// writet1.c's `static lastargOtherSubr3`, kept from font to font.
    pub last_arg_other_subr3: &'a mut i32,
    /// The glyphs to embed (`fd->gl_tree`): a `seac` adds its two.
    pub glyph_set: &'a mut Option<BTreeSet<Vec<u8>>>,
    pub host: &'a mut dyn Host,
}

/// The font's charstrings and subrs, and what marking has kept from one
/// glyph to the next.
#[derive(Default)]
pub(super) struct Charstrings {
    pub glyphs: Vec<Charstring>,
    by_name: HashMap<Vec<u8>, usize>,
    /// The `.notdef` glyph, once a walk has looked it up (`cs_notdef`).
    notdef: Option<usize>,
    pub subrs: Vec<Charstring>,
    /// The operand stack carries over from one glyph's walk to the next.
    stack: Stack,
    /// Marks and fixes so far (part of [`EntryState`]).
    marks: u64,
    /// The bytes of all charstrings and subrs, for `WORK_FACTOR`.
    code_bytes: u64,
}

impl Charstrings {
    /// Add a glyph's charstring (the first of a name is the one found).
    pub fn add_glyph(&mut self, cs: Charstring) {
        self.by_name
            .entry(cs.name.clone())
            .or_insert(self.glyphs.len());
        self.glyphs.push(cs);
    }

    /// Forget the glyphs (`t1_subset_charstrings` starts afresh).
    pub fn clear_glyphs(&mut self) {
        self.glyphs.clear();
        self.by_name.clear();
        self.notdef = None;
    }

    /// Mark `target` and everything it calls (`cs_mark`).
    ///
    /// Two walks pdfTeX never finishes are font errors here (DESIGN.md
    /// §4.5), where pdfTeX recurses until it crashes: a subr entered again
    /// in exactly the state it is already being parsed in
    /// ([`EntryState`]), and a walk longer than `WORK_FACTOR` allows (a
    /// subr that calls itself with the stack growing). Neither keeps a copy
    /// of the stack per call.
    pub fn mark(&mut self, env: &mut MarkEnv, target: Target) -> Result<()> {
        if self.code_bytes == 0 {
            let all = self.glyphs.iter().chain(&self.subrs);
            self.code_bytes = all.map(|e| u64::from(e.cs_len)).sum::<u64>().max(1);
        }
        let limit = WORK_FACTOR * self.code_bytes + WORK_SLACK;
        let mut steps: u64 = 0;
        let mut walk = Walk::default();
        self.call(env, &mut walk, target)?;
        while let Some(fr) = walk.frames.last_mut() {
            if fr.left <= 0 {
                // the frame's charstring has ended: pdfTeX's `cs_mark` returns
                let fr = walk.pop();
                if let Some(Frame {
                    owner: Owner::Subr(s),
                    idx,
                    last_op,
                    ..
                }) = fr
                {
                    if last_op != Some(Op::Return) {
                        env.host.warn(
                            format!(
                                "last command in subr `{s}' is not a RETURN; \
                                 I will add it now but please consider fixing the font"
                            )
                            .as_bytes(),
                        );
                        self.subrs[idx].append_return();
                        self.marks += 1;
                    }
                }
                self.resume(env, &mut walk)?;
                continue;
            }
            steps += 1;
            if steps > limit {
                return Err(fr.owner.error(format_args!(
                    "more than {limit} charstring operations, {WORK_FACTOR} \
                     times the font's charstring bytes: the subrs it calls do not end"
                )));
            }
            fr.left -= 1;
            let b = fr.next();
            if b >= 32 {
                let n = fr.number(b);
                self.stack.push(n);
                continue;
            }
            let mut v = i32::from(b);
            if v == ESCAPE {
                v = i32::from(fr.next()) + 32;
                fr.left -= 1;
            }
            let Some(op) = Op::from_value(v) else {
                return Err(fr.owner.error(if v >= OP_LIMIT {
                    format!("command value out of range: {v}")
                } else {
                    format!("command not valid: {v}")
                }));
            };
            if let Some(n) = op.operands() {
                let depth = self.stack.values.len();
                if depth != n {
                    let which = if depth < n { "less" } else { "more" };
                    return Err(fr.owner.error(format_args!(
                        "{which} arguments on stack ({depth}) than required ({n})"
                    )));
                }
            }
            fr.last_op = Some(op);
            match op {
                Op::CallSubr => {
                    let n = self.stack.get(-1);
                    self.stack.pop(1)?;
                    fr.resume = Resume::CallSubr(n);
                    self.call(env, &mut walk, Target::Subr(n))?;
                }
                Op::Div => {
                    self.stack.pop(2)?;
                    self.stack.push(0);
                }
                Op::CallOtherSubr => {
                    if self.stack.get(-1) == 3 {
                        *env.last_arg_other_subr3 = self.stack.get(-3);
                    }
                    let n = self.stack.get(-2) + 2;
                    self.stack.pop(n)?;
                }
                Op::Pop => {
                    // the only case when we care about the value being
                    // pushed onto stack is when POP follows CALLOTHERSUBR
                    // (changing hints by OtherSubrs[3])
                    self.stack.push(*env.last_arg_other_subr3);
                }
                Op::Seac => {
                    let base = standard_glyph_name(self.stack.get(3) as usize);
                    let accent = standard_glyph_name(self.stack.get(4) as usize);
                    self.stack.clear();
                    fr.resume = Resume::SeacAccent(base, accent);
                    self.call(env, &mut walk, Target::Glyph(base))?;
                }
                _ if op.clears() => self.stack.clear(),
                _ => {}
            }
        }
        Ok(())
    }

    /// Mark `target` as a call of the frame on top of `walk`: parse it next,
    /// or, where pdfTeX's `cs_mark` returns at once, go on with the caller.
    fn call(&mut self, env: &mut MarkEnv, walk: &mut Walk, target: Target) -> Result<()> {
        match self.enter(env, walk, target)? {
            Some(f) => walk.push(f),
            None => self.resume(env, walk)?,
        }
        Ok(())
    }

    /// The start of pdfTeX's `cs_mark`, up to its parsing loop: find the
    /// entry, mark it, and give the frame that parses it; `None` where
    /// pdfTeX returns at once (an invalid entry, a glyph marked already, an
    /// undefined glyph).
    fn enter(
        &mut self,
        env: &mut MarkEnv,
        walk: &mut Walk,
        target: Target,
    ) -> Result<Option<Frame>> {
        let (owner, idx, is_subr) = match target {
            Target::Subr(s) => {
                let idx = usize::try_from(s).ok().filter(|&i| i < self.subrs.len());
                let Some(idx) = idx else {
                    return Err(Fail(format!("Subrs array: entry index out of range ({s})")));
                };
                if !self.subrs[idx].valid {
                    return Ok(None);
                }
                (Owner::Subr(s), idx, true)
            }
            Target::Glyph(name) => {
                let idx = match self.notdef.filter(|_| name == NOTDEF) {
                    Some(i) => i,
                    None => {
                        let Some(&i) = self.by_name.get(name) else {
                            let mut msg = b"glyph `".to_vec();
                            msg.extend_from_slice(name);
                            msg.extend_from_slice(b"' undefined");
                            env.host.warn(&msg);
                            return Ok(None);
                        };
                        if self.glyphs[i].name == NOTDEF {
                            self.notdef = Some(i);
                        }
                        i
                    }
                };
                (Owner::Glyph(name.to_vec()), idx, false)
            }
        };
        let entry = if is_subr {
            &mut self.subrs[idx]
        } else {
            &mut self.glyphs[idx]
        };
        // only marked charstrings and invalid entries can be skipped; valid
        // marked subrs must be parsed to keep the stack in sync
        if !entry.valid || (entry.used && !is_subr) {
            return Ok(None);
        }
        if !entry.used {
            entry.used = true;
            self.marks += 1;
        }
        let entry = if is_subr {
            &self.subrs[idx]
        } else {
            &self.glyphs[idx]
        };
        let mut frame = Frame {
            owner,
            idx,
            data: entry.data.clone(),
            at: 4,
            key: Cipher::CHARSTRING,
            left: i32::from(entry.cs_len),
            last_op: None,
            resume: Resume::Nothing,
            entered: None,
        };
        if is_subr {
            let state = EntryState {
                subr: idx,
                depth: self.stack.values.len(),
                hash: self.stack.fingerprint_or((0, 0)),
                last_arg_other_subr3: *env.last_arg_other_subr3,
                marks: self.marks,
            };
            // A frame being parsed was entered in a state with this key. If
            // the key comes again and the stack is the one saved at the
            // first repeat, that frame (still being parsed) was entered in
            // exactly this state: the call pdfTeX never returns from.
            if walk.active.contains_key(&state) {
                match walk.confirm.get(&state) {
                    Some((stack, _)) if *stack == self.stack.values => {
                        let caller = walk.frames.last().map_or(&frame.owner, |f| &f.owner);
                        return Err(caller.error(format_args!(
                            "cannot call subr ({idx}): it would call itself without end"
                        )));
                    }
                    _ => {
                        let seen = (self.stack.values.clone(), walk.frames.len());
                        walk.confirm.insert(state.clone(), seen);
                    }
                }
            }
            frame.entered = Some(state);
        }
        for _ in 0..env.len_iv {
            frame.next();
            frame.left -= 1;
        }
        Ok(Some(frame))
    }

    /// What pdfTeX's `cs_mark` does after one of its calls returns, for the
    /// frame on top of `walk`.
    fn resume(&mut self, env: &mut MarkEnv, walk: &mut Walk) -> Result<()> {
        let Some(fr) = walk.frames.last_mut() else {
            return Ok(());
        };
        match std::mem::replace(&mut fr.resume, Resume::Nothing) {
            Resume::Nothing => {}
            Resume::CallSubr(n) => {
                let valid = usize::try_from(n)
                    .ok()
                    .and_then(|i| self.subrs.get(i))
                    .is_some_and(|s| s.valid);
                if !valid {
                    return Err(fr.owner.error(format_args!("cannot call subr ({n})")));
                }
            }
            Resume::SeacAccent(base, accent) => {
                fr.resume = Resume::SeacDone(base, accent);
                self.call(env, walk, Target::Glyph(accent))?;
            }
            Resume::SeacDone(base, accent) => {
                // base and accent characters are needed in CharSet
                if let Some(gl) = env.glyph_set.as_mut() {
                    gl.insert(base.to_vec());
                    gl.insert(accent.to_vec());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_table_is_writet1s() {
        // (value, bottom, nargs, clear) from writet1.c's cc_init()
        let table = [
            (1, true, 2, true),
            (3, true, 2, true),
            (4, true, 1, true),
            (5, true, 2, true),
            (6, true, 1, true),
            (7, true, 1, true),
            (8, true, 6, true),
            (9, false, 0, true),
            (10, false, 1, false),
            (11, false, 0, false),
            (13, true, 2, true),
            (14, false, 0, true),
            (21, true, 2, true),
            (22, true, 1, true),
            (30, true, 4, true),
            (31, true, 4, true),
            (32, false, 0, true),
            (33, true, 6, true),
            (34, true, 6, true),
            (38, true, 5, true),
            (39, true, 4, true),
            (44, false, 2, false),
            (48, false, 0, false),
            (49, false, 0, false),
            (65, true, 2, true),
        ];
        for v in 0..OP_LIMIT {
            let row = table.iter().find(|r| r.0 == v);
            let op = Op::from_value(v);
            assert_eq!(op.is_some(), row.is_some(), "value {v}");
            if let (Some(op), Some(&(_, bottom, nargs, clear))) = (op, row) {
                assert_eq!(op.operands(), bottom.then_some(nargs), "value {v}");
                assert_eq!(op.clears(), clear, "value {v}");
            }
        }
    }

    #[test]
    fn numbers() {
        // encrypt the bytes so that Frame::next gives them back
        let frame = |plain: &[u8]| Frame {
            owner: Owner::Subr(0),
            idx: 0,
            data: cipher::encrypt(Cipher::CHARSTRING, plain.iter().copied()),
            at: 0,
            key: Cipher::CHARSTRING,
            left: 10,
            last_op: None,
            resume: Resume::Nothing,
            entered: None,
        };
        let read = |plain: &[u8]| {
            let mut f = frame(plain);
            let b = f.next();
            f.number(b)
        };
        assert_eq!(read(&[139]), 0);
        assert_eq!(read(&[32]), -107);
        assert_eq!(read(&[247, 0]), 108);
        assert_eq!(read(&[250, 255]), 1131);
        assert_eq!(read(&[251, 0]), -108);
        assert_eq!(read(&[255, 0xff, 0xff, 0xff, 0xfe]), -2);
    }

    #[test]
    fn append_return_reencrypts() {
        let plain = [139u8, 139, 139, 139, 140, 10];
        let mut cs = Charstring {
            data: [
                b" RD ".as_slice(),
                &cipher::encrypt(Cipher::CHARSTRING, plain),
                b" NP\n",
            ]
            .concat(),
            cs_len: plain.len() as u16,
            ..Default::default()
        };
        cs.append_return();
        let mut key = Cipher::CHARSTRING;
        let got: Vec<u8> = cs.data[4..4 + 7].iter().map(|&c| key.decrypt(c)).collect();
        assert_eq!(got, [139, 139, 139, 139, 140, 10, 11]);
        assert_eq!(&cs.data[11..], b" NP\n");
        assert_eq!(cs.cs_len, 7);
    }
}
