//! Type 1 charstrings read as dvipdfmx reads them (dvipdfm-x 20260317,
//! t1_char.c): `t1char_build_charpath` turns a decrypted charstring into
//! a path of Type 2 operators (side bearing folded into the first moveto,
//! flex and hint replacement through the OtherSubrs, `seac` kept aside),
//! and `do_postproc` takes the path's bounding box (of the points the
//! operators move to, control points included, as the C does) and joins
//! neighbouring operators.
//!
//! `t1char_get_metrics` ([`metrics`]) is what type1.c's `get_font_attr`
//! uses for a Type 1 font's `/CapHeight`, `/Ascent` and `/Descent`.

const CS_ARG_STACK_MAX: usize = 48;
const CS_STEM_ZONE_MAX: usize = 96;
const PS_ARG_STACK_MAX: usize = CS_STEM_ZONE_MAX * 2 + 2;
const CS_SUBR_NEST_MAX: i32 = 10;

const CS_HINT_DECL: i32 = -1;
const CS_FLEX_CTRL: i32 = -2;

// single-byte operators (Type 1 and the Type 2 ones the path uses)
const HSTEM: i32 = 1;
const VSTEM: i32 = 3;
const VMOVETO: i32 = 4;
const RLINETO: i32 = 5;
const HLINETO: i32 = 6;
const VLINETO: i32 = 7;
const RRCURVETO: i32 = 8;
const CLOSEPATH: i32 = 9;
const CALLSUBR: u8 = 10;
const RETURN: u8 = 11;
const ESCAPE: u8 = 12;
const HSBW: i32 = 13;
const ENDCHAR: i32 = 14;
pub const CNTRMASK: i32 = 20;
const RMOVETO: i32 = 21;
const HMOVETO: i32 = 22;
const RCURVELINE: i32 = 24;
const RLINECURVE: i32 = 25;
const VHCURVETO: i32 = 30;
const HVCURVETO: i32 = 31;
// two-byte operators (12 x)
const DOTSECTION: u8 = 0;
const VSTEM3: u8 = 1;
const HSTEM3: u8 = 2;
const SEAC: u8 = 6;
const SBW: u8 = 7;
const DIV: u8 = 12;
const CALLOTHERSUBR: u8 = 16;
const POP: u8 = 17;
const SETCURRENTPOINT: u8 = 33;
/// The Type 2 flex operators, as path types (12 x + 1000).
pub const HFLEX: i32 = 1034;
pub const FLEX: i32 = 1035;
pub const HFLEX1: i32 = 1036;

const PHASE_INIT: i32 = 0;
const PHASE_PATH: i32 = 2;
const PHASE_FLEX: i32 = 3;

const FLAG_USE_HINTMASK: u32 = 1;
const FLAG_USE_CNTRMASK: u32 = 2;
const FLAG_USE_SEAC: u32 = 4;

fn is_path_operator(o: i32) -> bool {
    (VMOVETO..=CLOSEPATH).contains(&o) || ((RMOVETO..=HVCURVETO).contains(&o) && o != 23 && o != 29 && o != 28)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathOp {
    pub ty: i32,
    pub args: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stem {
    pub id: usize,
    pub horizontal: bool,
    pub pos: f64,
    pub del: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Seac {
    pub adx: f64,
    pub ady: f64,
    pub bchar: u8,
    pub achar: u8,
}

/// `t1_chardesc` after `do_postproc`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glyph {
    pub flags: u32,
    /// sbx, sby, wx, wy
    pub sbw: [f64; 4],
    /// llx, lly, urx, ury
    pub bbox: [f64; 4],
    pub seac: Option<Seac>,
    pub stems: Vec<Stem>,
    pub path: Vec<PathOp>,
}

/// Why dvipdfmx stops (its `ERROR`): fatal for the run.
pub type Fatal = String;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Status {
    Ok,
    SubrReturn,
    CharEnd,
    ParseError,
    StackError,
}

impl Status {
    fn code(self) -> i32 {
        match self {
            Status::Ok => 0,
            Status::SubrReturn => 2,
            Status::CharEnd => 3,
            Status::ParseError => -1,
            Status::StackError => -2,
        }
    }
}

struct Builder<'a> {
    cd: Glyph,
    status: Status,
    phase: i32,
    nest: i32,
    cs: Vec<f64>,
    ps: Vec<f64>,
    subrs: &'a dyn Fn(usize) -> Option<Vec<u8>>,
    nsubrs: usize,
    warn: Vec<String>,
    /// `do_othersubr13`'s number of horizontal stem groups.
    hgroups: usize,
}

macro_rules! check_stack {
    ($b:expr, $n:expr) => {
        if $b.cs.len() < $n {
            $b.status = Status::StackError;
            return Ok(());
        }
    };
}

impl Builder<'_> {
    fn add_stem(&mut self, mut pos: f64, del: f64, horizontal: bool) -> Option<usize> {
        pos += if horizontal { self.cd.sbw[1] } else { self.cd.sbw[0] };
        if let Some(s) = self
            .cd
            .stems
            .iter()
            .find(|s| s.horizontal == horizontal && s.pos == pos && s.del == del)
        {
            return Some(s.id);
        }
        if self.cd.stems.len() == CS_STEM_ZONE_MAX {
            return None;
        }
        let id = self.cd.stems.len();
        self.cd.stems.push(Stem {
            id,
            horizontal,
            pos,
            del,
        });
        Some(id)
    }

    fn add_charpath(&mut self, ty: i32, args: &[f64]) {
        self.cd.path.push(PathOp {
            ty,
            args: args.to_vec(),
        });
        if ty >= 0 && self.phase != PHASE_FLEX && is_path_operator(ty) {
            self.phase = PHASE_PATH;
        }
    }

    /// `ADD_PATH`: the top `n` operands.
    fn add_path(&mut self, ty: i32, n: usize) {
        let args = self.cs[self.cs.len() - n..].to_vec();
        self.add_charpath(ty, &args);
    }

    fn pop(&mut self) -> f64 {
        self.cs.pop().unwrap_or(0.0)
    }

    fn do_operator1(&mut self, op: u8) -> Result<(), Fatal> {
        let mut op = op as i32;
        match op {
            CLOSEPATH => self.cs.clear(),
            HSBW => {
                check_stack!(self, 2);
                self.cd.sbw[2] = self.pop();
                self.cd.sbw[3] = 0.0;
                self.cd.sbw[0] = self.pop();
                self.cd.sbw[1] = 0.0;
                self.cs.clear();
            }
            HSTEM | VSTEM => {
                check_stack!(self, 2);
                let n = self.cs.len();
                let Some(id) = self.add_stem(self.cs[n - 2], self.cs[n - 1], op == HSTEM) else {
                    self.warn.push("Too many hints...".into());
                    self.status = Status::ParseError;
                    return Ok(());
                };
                self.cs.push(id as f64);
                self.add_path(CS_HINT_DECL, 1);
                self.cs.clear();
            }
            RMOVETO => {
                check_stack!(self, 2);
                if self.phase < PHASE_PATH {
                    let n = self.cs.len();
                    self.cs[n - 2] += self.cd.sbw[0];
                    self.cs[n - 1] += self.cd.sbw[1];
                }
                self.add_path(op, 2);
                self.cs.clear();
            }
            HMOVETO | VMOVETO => {
                check_stack!(self, 1);
                let mut argn = 1;
                if self.phase < PHASE_PATH {
                    let n = self.cs.len();
                    if op == HMOVETO {
                        self.cs[n - 1] += self.cd.sbw[0];
                        if self.cd.sbw[1] != 0.0 {
                            self.cs.push(self.cd.sbw[1]);
                            argn = 2;
                            op = RMOVETO;
                        }
                    } else {
                        self.cs[n - 1] += self.cd.sbw[1];
                        if self.cd.sbw[0] != 0.0 {
                            let v = self.cs[n - 1];
                            self.cs[n - 1] = self.cd.sbw[0];
                            self.cs.push(v);
                            argn = 2;
                            op = RMOVETO;
                        }
                    }
                }
                self.add_path(op, argn);
                self.cs.clear();
            }
            ENDCHAR => {
                self.status = Status::CharEnd;
                self.cs.clear();
            }
            RLINETO => {
                check_stack!(self, 2);
                self.add_path(op, 2);
                self.cs.clear();
            }
            HLINETO | VLINETO => {
                check_stack!(self, 1);
                self.add_path(op, 1);
                self.cs.clear();
            }
            RRCURVETO => {
                check_stack!(self, 6);
                self.add_path(op, 6);
                self.cs.clear();
            }
            VHCURVETO | HVCURVETO => {
                check_stack!(self, 4);
                self.add_path(op, 4);
                self.cs.clear();
            }
            11 => {}
            10 => return Err("Unexpected callsubr.".into()),
            _ => {
                self.warn
                    .push(format!("Unknown charstring operator: 0x{op:02x}"));
                self.status = Status::ParseError;
            }
        }
        Ok(())
    }

    fn do_othersubr0(&mut self) {
        if self.ps.is_empty() {
            self.status = Status::ParseError;
            return;
        }
        let Some(f) = self.cd.path.iter().position(|p| p.ty == CS_FLEX_CTRL) else {
            // the C dereferences NULL here
            self.status = Status::ParseError;
            return;
        };
        let mut args = self.cd.path[f].args.clone();
        args.resize(13, 0.0);
        for i in 1..7 {
            match self.cd.path.get(f + i) {
                Some(c) if c.ty == CS_FLEX_CTRL && c.args.len() == 2 => {
                    if i == 1 {
                        args[0] += c.args[0];
                        args[1] += c.args[1];
                    } else {
                        args[2 * i - 2] = c.args[0];
                        args[2 * i - 1] = c.args[1];
                    }
                }
                _ => {
                    self.status = Status::ParseError;
                    return;
                }
            }
        }
        if self.cd.path.len() != f + 7 {
            self.status = Status::ParseError;
            return;
        }
        self.cd.path.truncate(f + 1);
        args[12] = self.ps.pop().unwrap_or(0.0);
        self.cd.path[f] = PathOp { ty: FLEX, args };
        self.phase = PHASE_PATH;
    }

    fn do_othersubr2(&mut self) {
        let Some(last) = self.cd.path.last_mut().filter(|_| self.phase == PHASE_FLEX) else {
            self.status = Status::ParseError;
            return;
        };
        match last.ty {
            RMOVETO => {}
            HMOVETO => last.args = vec![last.args[0], 0.0],
            VMOVETO => last.args = vec![0.0, last.args[0]],
            _ => {
                self.status = Status::ParseError;
                return;
            }
        }
        last.ty = CS_FLEX_CTRL;
    }

    fn do_othersubr13(&mut self) {
        if self.phase != PHASE_INIT {
            self.status = Status::ParseError;
            return;
        }
        let mut groups: Vec<Vec<f64>> = vec![Vec::new(); CS_STEM_ZONE_MAX];
        for horizontal in [true, false] {
            let num = self.ps.pop().unwrap_or(0.0) as i32;
            if num < 0 || num as usize > CS_STEM_ZONE_MAX {
                self.status = Status::ParseError;
                return;
            }
            let num = num as usize;
            let (mut n, mut pos) = (0usize, 0.0);
            while self.ps.len() >= 2 && n < num {
                pos += self.ps.pop().unwrap();
                let del = self.ps.pop().unwrap();
                let id = self.add_stem(
                    if del < 0.0 { pos + del } else { pos },
                    if del < 0.0 { -del } else { del },
                    horizontal,
                );
                // the C stores -1 when the stems are full
                groups[n].push(id.map_or(-1.0, |i| i as f64));
                pos += del;
                if del < 0.0 {
                    pos = 0.0;
                    n += 1;
                }
            }
            if n != num {
                self.status = Status::StackError;
                return;
            }
            if horizontal {
                self.hgroups = num;
            } else {
                for g in groups.iter().take(self.hgroups.max(num)) {
                    let g = g.clone();
                    self.add_charpath(CNTRMASK, &g);
                }
            }
        }
        self.cd.flags |= FLAG_USE_CNTRMASK;
    }

    fn do_callothersubr(&mut self) -> Result<(), Fatal> {
        check_stack!(self, 2);
        let subrno = self.pop() as i32;
        let argn = self.pop() as i32;
        check_stack!(self, argn.max(0) as usize);
        if self.ps.len() + argn.max(0) as usize > PS_ARG_STACK_MAX {
            self.status = Status::ParseError;
            return Ok(());
        }
        for _ in 0..argn.max(0) {
            let v = self.pop();
            self.ps.push(v);
        }
        match subrno {
            0 => self.do_othersubr0(),
            1 => self.phase = PHASE_FLEX,
            2 => self.do_othersubr2(),
            3 => self.cd.flags |= FLAG_USE_HINTMASK,
            12 => {
                if self.phase != PHASE_INIT {
                    self.status = Status::ParseError;
                }
            }
            13 => self.do_othersubr13(),
            _ => return Err(format!("Unknown othersubr #{subrno}.")),
        }
        Ok(())
    }

    fn do_operator2(&mut self, data: &[u8], i: &mut usize) -> Result<(), Fatal> {
        *i += 1;
        if *i + 1 > data.len() {
            self.status = Status::ParseError;
            return Ok(());
        }
        let op = data[*i];
        *i += 1;
        match op {
            SBW => {
                check_stack!(self, 4);
                self.cd.sbw[3] = self.pop();
                self.cd.sbw[2] = self.pop();
                self.cd.sbw[1] = self.pop();
                self.cd.sbw[0] = self.pop();
                self.cs.clear();
            }
            HSTEM3 | VSTEM3 => {
                check_stack!(self, 6);
                for k in (0..3).rev() {
                    let n = self.cs.len();
                    let Some(id) = self.add_stem(self.cs[n - 2 * k - 2], self.cs[n - 2 * k - 1], op == HSTEM3)
                    else {
                        self.warn.push("Too many hints...".into());
                        self.status = Status::ParseError;
                        return Ok(());
                    };
                    self.cs.push(id as f64);
                    self.add_path(CS_HINT_DECL, 1);
                    self.cs.pop();
                }
                self.cs.clear();
            }
            SETCURRENTPOINT => {
                check_stack!(self, 2);
                self.cs.clear();
            }
            POP => {
                let Some(v) = self.ps.pop() else {
                    self.status = Status::ParseError;
                    return Ok(());
                };
                if self.cs.len() + 1 > CS_ARG_STACK_MAX {
                    self.status = Status::StackError;
                    return Ok(());
                }
                self.cs.push(v);
            }
            DOTSECTION => {}
            DIV => {
                check_stack!(self, 2);
                let d = self.pop();
                let n = self.cs.len();
                self.cs[n - 1] /= d;
            }
            CALLOTHERSUBR => self.do_callothersubr()?,
            SEAC => {
                check_stack!(self, 5);
                self.cd.flags |= FLAG_USE_SEAC;
                let achar = self.pop() as i64 as u8;
                let bchar = self.pop() as i64 as u8;
                let mut ady = self.pop();
                let mut adx = self.pop();
                ady += self.cd.sbw[1];
                adx += self.cd.sbw[0] - self.pop();
                self.cd.seac = Some(Seac {
                    adx,
                    ady,
                    bchar,
                    achar,
                });
                self.cs.clear();
            }
            _ => {
                self.warn
                    .push(format!("Unknown charstring operator: 0x0c{op:02x}"));
                self.status = Status::ParseError;
            }
        }
        Ok(())
    }

    fn push(&mut self, v: i32) {
        if self.cs.len() + 1 > CS_ARG_STACK_MAX {
            self.status = Status::StackError;
            return;
        }
        self.cs.push(v as f64);
    }

    /// `t1char_build_charpath`.
    fn build(&mut self, data: &[u8]) -> Result<(), Fatal> {
        if self.nest > CS_SUBR_NEST_MAX {
            return Err("Subroutine nested too deeply.".into());
        }
        self.nest += 1;
        let mut i = 0;
        while i < data.len() && self.status == Status::Ok {
            let b0 = data[i];
            if b0 == 255 {
                i += 1;
                if i + 4 > data.len() {
                    self.status = Status::ParseError;
                    break;
                }
                let v = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
                i += 4;
                self.push(v);
            } else if b0 == RETURN {
                self.status = Status::SubrReturn;
            } else if b0 == CALLSUBR {
                if self.cs.is_empty() {
                    self.status = Status::StackError;
                } else {
                    let idx = self.pop() as i32;
                    if idx < 0 || idx as usize >= self.nsubrs {
                        return Err("Invalid Subr#.".into());
                    }
                    let subr = (self.subrs)(idx as usize).unwrap_or_default();
                    self.build(&subr)?;
                    i += 1;
                }
            } else if b0 == ESCAPE {
                self.do_operator2(data, &mut i)?;
            } else if b0 < 32 && b0 != 28 {
                i += 1;
                self.do_operator1(b0)?;
            } else {
                // get_integer
                i += 1;
                let v = if b0 == 28 {
                    if i + 2 > data.len() {
                        self.status = Status::ParseError;
                        break;
                    }
                    let r = data[i] as i32 * 256 + data[i + 1] as i32;
                    i += 2;
                    if r > 0x7fff {
                        r - 0x10000
                    } else {
                        r
                    }
                } else if (32..=246).contains(&b0) {
                    b0 as i32 - 139
                } else {
                    if i + 1 > data.len() {
                        self.status = Status::ParseError;
                        break;
                    }
                    let b1 = data[i] as i32;
                    i += 1;
                    if (247..=250).contains(&b0) {
                        (b0 as i32 - 247) * 256 + b1 + 108
                    } else {
                        -(b0 as i32 - 251) * 256 - b1 - 108
                    }
                };
                self.push(v);
            }
        }
        if self.status == Status::SubrReturn {
            self.status = Status::Ok;
        } else if self.status == Status::CharEnd && i < data.len() {
            if !(i == data.len() - 1 && data[i] == RETURN) {
                self.warn
                    .push(format!("Garbage after endchar. ({} bytes)", data.len() - i));
            }
        } else if self.status.code() < 0 {
            return Err(format!(
                "Parsing charstring failed: (status={}, stack={})",
                self.status.code(),
                self.cs.len()
            ));
        }
        self.nest -= 1;
        Ok(())
    }
}

/// `do_postproc`: the bounding box, and neighbouring operators joined.
fn postproc(cd: &mut Glyph) -> Result<(), Fatal> {
    if cd.path.is_empty() {
        return Ok(());
    }
    let mut b = [100000.0, 100000.0, -100000.0, -100000.0];
    let upd = |b: &mut [f64; 4], x: f64, y: f64| {
        if b[0] > x {
            b[0] = x;
        }
        if b[2] < x {
            b[2] = x;
        }
        if b[1] > y {
            b[1] = y;
        }
        if b[3] < y {
            b[3] = y;
        }
    };
    let (mut x, mut y) = (0.0, 0.0);
    let mut out: Vec<PathOp> = Vec::with_capacity(cd.path.len());
    for mut cur in std::mem::take(&mut cd.path) {
        let a = cur.args.clone();
        let mut joined = false;
        let try_compact = |prev: Option<&PathOp>| {
            prev.is_some_and(|p| p.args.len() + a.len() < CS_ARG_STACK_MAX)
        };
        match cur.ty {
            RMOVETO => {
                x += a[0];
                y += a[1];
                upd(&mut b, x, y);
            }
            RLINETO => {
                x += a[0];
                y += a[1];
                upd(&mut b, x, y);
                if try_compact(out.last()) {
                    let p = out.last_mut().unwrap();
                    if p.ty == RLINETO {
                        p.args.extend_from_slice(&a);
                        joined = true;
                    } else if p.ty == RRCURVETO {
                        p.args.extend_from_slice(&a);
                        p.ty = RCURVELINE;
                        joined = true;
                    }
                }
            }
            HMOVETO => {
                x += a[0];
                upd(&mut b, x, y);
            }
            VMOVETO => {
                y += a[0];
                upd(&mut b, x, y);
            }
            HLINETO | VLINETO => {
                if cur.ty == HLINETO {
                    x += a[0];
                } else {
                    y += a[0];
                }
                upd(&mut b, x, y);
                if try_compact(out.last()) {
                    let (same, other) = (cur.ty, if cur.ty == HLINETO { VLINETO } else { HLINETO });
                    let p = out.last_mut().unwrap();
                    if (p.ty == other && p.args.len() % 2 == 1) || (p.ty == same && p.args.len() % 2 == 0) {
                        p.args.extend_from_slice(&a);
                        joined = true;
                    }
                }
            }
            RRCURVETO => {
                for i in 0..3 {
                    x += a[2 * i];
                    y += a[2 * i + 1];
                    upd(&mut b, x, y);
                }
                if try_compact(out.last()) {
                    let p = out.last_mut().unwrap();
                    if p.ty == RRCURVETO {
                        p.args.extend_from_slice(&a);
                        joined = true;
                    } else if p.ty == RLINETO {
                        p.args.extend_from_slice(&a);
                        p.ty = RLINECURVE;
                        joined = true;
                    }
                }
            }
            VHCURVETO | HVCURVETO => {
                if cur.ty == VHCURVETO {
                    y += a[0];
                } else {
                    x += a[0];
                }
                upd(&mut b, x, y);
                x += a[1];
                y += a[2];
                upd(&mut b, x, y);
                if cur.ty == VHCURVETO {
                    x += a[3];
                } else {
                    y += a[3];
                }
                upd(&mut b, x, y);
                if try_compact(out.last()) {
                    let (same, other) = (cur.ty, if cur.ty == VHCURVETO { HVCURVETO } else { VHCURVETO });
                    let p = out.last_mut().unwrap();
                    if (p.ty == other && (p.args.len() / 4) % 2 == 1)
                        || (p.ty == same && (p.args.len() / 4) % 2 == 0)
                    {
                        p.args.extend_from_slice(&a);
                        joined = true;
                    }
                }
            }
            FLEX => {
                for i in 0..6 {
                    // the C adds args[3] for every y (`2*1+1`)
                    x += a[2 * i];
                    y += a[3];
                    upd(&mut b, x, y);
                }
                if a[12] == 50.0 {
                    if a[1] == 0.0 && a[11] == 0.0 && a[5] == 0.0 && a[7] == 0.0 && a[3] + a[9] == 0.0 {
                        cur.args = vec![a[0], a[2], a[3], a[4], a[6], a[8], a[10]];
                        cur.ty = HFLEX;
                    } else if a[5] == 0.0 && a[7] == 0.0 && (a[1] + a[3] + a[9] + a[11]) == 0.0 {
                        cur.args = vec![a[0], a[1], a[2], a[3], a[4], a[6], a[8], a[9], a[10]];
                        cur.ty = HFLEX1;
                    }
                }
            }
            CS_HINT_DECL | CNTRMASK => {}
            t => return Err(format!("Unexpected Type 2 charstring command {t}.")),
        }
        if !joined {
            out.push(cur);
        }
    }
    cd.path = out;
    if b[0] > b[2] {
        b[0] = cd.sbw[2];
        b[2] = cd.sbw[2];
    }
    if b[1] > b[3] {
        b[1] = cd.sbw[3];
        b[3] = cd.sbw[3];
    }
    cd.bbox = b;
    Ok(())
}

/// `t1char_get_metrics` for a decrypted charstring and its font's Subrs
/// (`subr(i)`, decrypted, of `nsubrs`): the glyph's path, side bearing,
/// width and bounding box; warnings go to `warn`. An `Err` is
/// dvipdfmx's `ERROR`.
pub fn metrics(
    cs: &[u8],
    subr: &dyn Fn(usize) -> Option<Vec<u8>>,
    nsubrs: usize,
    warn: &mut Vec<String>,
) -> Result<Glyph, Fatal> {
    let mut b = Builder {
        cd: Glyph::default(),
        status: Status::Ok,
        phase: PHASE_INIT,
        nest: 0,
        cs: Vec::new(),
        ps: Vec::new(),
        subrs: subr,
        nsubrs,
        warn: Vec::new(),
        hgroups: 0,
    };
    let r = b.build(cs);
    warn.append(&mut b.warn);
    r?;
    if !b.cs.is_empty() || !b.ps.is_empty() {
        warn.push(format!("Stack not empty. ({}, {})", b.cs.len(), b.ps.len()));
    }
    let mut cd = b.cd;
    postproc(&mut cd)?;
    Ok(cd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(v: i32) -> Vec<u8> {
        if (-107..=107).contains(&v) {
            vec![(v + 139) as u8]
        } else if (108..=1131).contains(&v) {
            let w = v - 108;
            vec![(247 + w / 256) as u8, (w % 256) as u8]
        } else if (-1131..=-108).contains(&v) {
            let w = -v - 108;
            vec![(251 + w / 256) as u8, (w % 256) as u8]
        } else {
            let mut o = vec![255];
            o.extend_from_slice(&v.to_be_bytes());
            o
        }
    }

    fn cs(items: &[&[i32]]) -> Vec<u8> {
        // each item: operands..., then the operator as a negative number
        // (-1000 - x for 12 x)
        let mut out = Vec::new();
        for it in items {
            for &v in &it[..it.len() - 1] {
                out.extend(enc(v));
            }
            let op = -it[it.len() - 1];
            if op >= 1000 {
                out.extend([12, (op - 1000) as u8]);
            } else {
                out.push(op as u8);
            }
        }
        out
    }

    #[test]
    fn bbox_of_a_simple_glyph() {
        // 50 600 hsbw 0 0 rmoveto 100 0 rlineto 0 700 rlineto
        // 10 20 30 40 50 60 rrcurveto closepath endchar
        let c = cs(&[
            &[50, 600, -13],
            &[0, 0, -21],
            &[100, 0, -5],
            &[0, 700, -5],
            &[10, 20, 30, 40, 50, 60, -8],
            &[-9],
            &[-14],
        ]);
        let g = metrics(&c, &|_| None, 0, &mut Vec::new()).unwrap();
        assert_eq!(g.sbw, [50.0, 0.0, 600.0, 0.0]);
        assert_eq!(g.bbox, [50.0, 0.0, 240.0, 820.0]);
        assert_eq!(g.path[1].ty, RLINECURVE);
    }

    #[test]
    fn no_path() {
        // no path at all: the box stays 0 (`do_postproc` returns early)
        let c = cs(&[&[0, 333, -13], &[-14]]);
        let g = metrics(&c, &|_| None, 0, &mut Vec::new()).unwrap();
        assert_eq!(g.bbox, [0.0; 4]);
        // a hint but no point: the width
        let c = cs(&[&[0, 333, -13], &[10, 20, -1], &[-14]]);
        let g = metrics(&c, &|_| None, 0, &mut Vec::new()).unwrap();
        assert_eq!(g.bbox, [333.0, 0.0, 333.0, 0.0]);
    }

    #[test]
    fn subrs_and_div() {
        // 0 1000 hsbw 1 callsubr endchar; subr 1: 0 7 2 div rmoveto return
        let c = cs(&[&[0, 1000, -13], &[1, -10], &[-14]]);
        let s1 = cs(&[&[0, 7, 2, -1012], &[-21], &[-11]]);
        let subr = move |i: usize| (i == 1).then(|| s1.clone());
        let g = metrics(&c, &subr, 2, &mut Vec::new()).unwrap();
        assert_eq!(g.bbox, [0.0, 3.5, 0.0, 3.5]);
        assert!(metrics(&cs(&[&[5, -10]]), &subr, 2, &mut Vec::new()).is_err());
    }
}
