mod arithmetic;
use super::{byte, integer, number, unsupported, Cff};
use crate::{invalid, Coordinate, Result};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubicPoint {
    pub x: Coordinate,
    pub y: Coordinate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubicCommand {
    MoveTo(CubicPoint),
    LineTo(CubicPoint),
    CurveTo {
        control1: CubicPoint,
        control2: CubicPoint,
        end: CubicPoint,
    },
    Close,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HintPolicy {
    Reject,
    Unhinted,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StemHint {
    pub vertical: bool,
    pub delta: Coordinate,
    pub width: Coordinate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintMask {
    pub counter: bool,
    pub stem_count: usize,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintMetadata {
    pub policy: HintPolicy,
    pub stems: Vec<StemHint>,
    pub masks: Vec<HintMask>,
    pub flex_depths: Vec<Coordinate>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubicOutline {
    pub cff_sha256: String,
    pub glyph_id: u16,
    pub width: Coordinate,
    pub commands: Vec<CubicCommand>,
    pub hints: HintMetadata,
}
struct Decoder<'a> {
    cff: &'a Cff,
    transient: [Option<Coordinate>; 32],
    operands: Vec<Coordinate>,
    point: CubicPoint,
    open: bool,
    width: Coordinate,
    width_seen: bool,
    commands: Vec<CubicCommand>,
    calls: Vec<(bool, usize)>,
    steps: usize,
    hints: HintMetadata,
    hint_phase: u8,
    path_started: bool,
}
#[derive(PartialEq)]
enum Flow {
    Return,
    End,
}
impl Cff {
    /// Raw charstring-space cubic geometry. FontMatrix is preserved in Top DICT,
    /// not silently applied or replaced; consumer must interpret it explicitly.
    pub fn cubic_outline(&self, gid: u16) -> Result<CubicOutline> {
        self.cubic_outline_with_policy(gid, HintPolicy::Reject)
    }
    pub fn cubic_outline_with_policy(&self, gid: u16, policy: HintPolicy) -> Result<CubicOutline> {
        let zero = Coordinate::from_integer(0);
        let mut d = Decoder {
            cff: self,
            transient: [None; 32],
            operands: Vec::new(),
            point: CubicPoint { x: zero, y: zero },
            open: false,
            width: Coordinate::from_integer(number(&self.private, 20, Some(0))?),
            width_seen: false,
            commands: Vec::new(),
            calls: Vec::new(),
            steps: 0,
            hints: HintMetadata {
                policy,
                stems: Vec::new(),
                masks: Vec::new(),
                flex_depths: Vec::new(),
            },
            hint_phase: 0,
            path_started: false,
        };
        if d.execute(self.charstring(gid)?, false)? != Flow::End {
            return Err(invalid("Type2 glyph lacks endchar"));
        }
        Ok(CubicOutline {
            cff_sha256: self.sha256.clone(),
            glyph_id: gid,
            width: d.width,
            commands: d.commands,
            hints: d.hints,
        })
    }
}
impl Decoder<'_> {
    fn emit(&mut self, command: CubicCommand) -> Result<()> {
        if self.commands.len() >= 100000 {
            return Err(invalid("Type2 output budget"));
        }
        self.commands.push(command);
        Ok(())
    }
    fn delta(&mut self, x: Coordinate, y: Coordinate) -> Result<CubicPoint> {
        self.point.x = self.point.x.add(x)?;
        self.point.y = self.point.y.add(y)?;
        Ok(self.point)
    }
    fn curve(&mut self, v: &[Coordinate]) -> Result<()> {
        if v.len() != 6 || !self.open {
            return Err(invalid("Type2 curve arity/missing moveto"));
        }
        let control1 = self.delta(v[0], v[1])?;
        let control2 = self.delta(v[2], v[3])?;
        let end = self.delta(v[4], v[5])?;
        self.emit(CubicCommand::CurveTo {
            control1,
            control2,
            end,
        })
    }
    fn take_width(&mut self, expected: usize) -> Result<()> {
        if !self.width_seen {
            if self.operands.len() == expected + 1 {
                let delta = self.operands.remove(0);
                self.width =
                    Coordinate::from_integer(number(&self.cff.private, 21, Some(0))?).add(delta)?;
            }
            self.width_seen = true;
        }
        if self.operands.len() != expected {
            return Err(invalid("Type2 moveto/endchar operand count"));
        }
        Ok(())
    }
    fn stem_operands(&mut self, vertical: bool, allow_empty: bool) -> Result<()> {
        if !self.width_seen {
            if self.operands.len() % 2 == 1 {
                let width = self.operands.remove(0);
                self.width =
                    Coordinate::from_integer(number(&self.cff.private, 21, Some(0))?).add(width)?;
            }
            self.width_seen = true;
        }
        if !self.operands.len().is_multiple_of(2) || (!allow_empty && self.operands.is_empty()) {
            return Err(invalid("Type2 stem operand count"));
        }
        if !self.operands.is_empty() {
            if self.path_started || self.hint_phase > u8::from(vertical) {
                return Err(invalid("Type2 stems after mask/path or out of order"));
            }
            self.hint_phase = u8::from(vertical);
            let operands = std::mem::take(&mut self.operands);
            for pair in operands.as_chunks::<2>().0 {
                if pair[1].numerator() < 0
                    && pair[1] != Coordinate::from_integer(-20)
                    && pair[1] != Coordinate::from_integer(-21)
                {
                    return Err(invalid("Type2 undefined negative stem width"));
                }
                self.hints.stems.push(StemHint {
                    vertical,
                    delta: pair[0],
                    width: pair[1],
                });
                if self.hints.stems.len() > 96 {
                    return Err(invalid("Type2 stem budget"));
                }
            }
        }
        Ok(())
    }
    fn flex(&mut self, op: u8) -> Result<()> {
        if self.hints.policy != HintPolicy::Unhinted {
            return Err(unsupported("Type2 flex requires explicit Unhinted policy"));
        }
        let v = std::mem::take(&mut self.operands);
        let zero = Coordinate::from_integer(0);
        let neg = |v: Coordinate| {
            Coordinate::new(
                v.numerator()
                    .checked_neg()
                    .ok_or_else(|| invalid("Type2 coordinate overflow"))?,
                v.shift(),
            )
        };
        let (a, b, depth) = match (op, v.len()) {
            (34, 7) => (
                [v[0], zero, v[1], v[2], v[3], zero],
                [v[4], zero, v[5], neg(v[2])?, v[6], zero],
                Coordinate::from_integer(50),
            ),
            (35, 13) => (
                v[..6].try_into().unwrap(),
                v[6..12].try_into().unwrap(),
                v[12],
            ),
            (36, 9) => (
                [v[0], v[1], v[2], v[3], v[4], zero],
                [
                    v[5],
                    zero,
                    v[6],
                    v[7],
                    v[8],
                    neg(v[1].add(v[3])?.add(v[7])?)?,
                ],
                Coordinate::from_integer(50),
            ),
            (37, 11) => {
                let dx = v[0].add(v[2])?.add(v[4])?.add(v[6])?.add(v[8])?;
                let dy = v[1].add(v[3])?.add(v[5])?.add(v[7])?.add(v[9])?;
                let common = dx.shift().max(dy.shift());
                let ax = dx
                    .numerator()
                    .checked_abs()
                    .and_then(|n| n.checked_mul(1i128 << (common - dx.shift())))
                    .ok_or_else(|| invalid("Type2 flex comparison overflow"))?;
                let ay = dy
                    .numerator()
                    .checked_abs()
                    .and_then(|n| n.checked_mul(1i128 << (common - dy.shift())))
                    .ok_or_else(|| invalid("Type2 flex comparison overflow"))?;
                let (last_x, last_y) = if ax > ay {
                    (v[10], neg(dy)?)
                } else {
                    (neg(dx)?, v[10])
                };
                (
                    v[..6].try_into().unwrap(),
                    [v[6], v[7], v[8], v[9], last_x, last_y],
                    Coordinate::from_integer(50),
                )
            }
            _ => return Err(invalid("Type2 flex operand count")),
        };
        if depth.numerator() < 0 {
            return Err(invalid("Type2 negative flex depth"));
        }
        self.hints.flex_depths.push(depth);
        self.curve(&a)?;
        self.curve(&b)
    }
    fn execute(&mut self, data: &[u8], subroutine: bool) -> Result<Flow> {
        let mut at = 0;
        let zero = Coordinate::from_integer(0);
        while at < data.len() {
            self.steps += 1;
            if self.steps > 100000 {
                return Err(invalid("Type2 instruction budget"));
            }
            let op = byte(data, &mut at)?;
            if op >= 32 || op == 28 {
                let value = if op == 255 {
                    let value = crate::u32_at(data, at)? as i32;
                    at += 4;
                    Coordinate::new(value as i128, 16)?
                } else {
                    Coordinate::from_integer(integer(data, &mut at, op)?)
                };
                self.operands.push(value);
                if self.operands.len() > 48 {
                    return Err(invalid("Type2 operand stack budget"));
                }
                continue;
            }
            match op {
                1 | 3 | 18 | 23 => {
                    if self.hints.policy != HintPolicy::Unhinted {
                        return Err(unsupported(
                            "Type2 hints/masks unsupported in staged unhinted decoder",
                        ));
                    }
                    self.stem_operands(op == 3 || op == 23, false)?;
                }
                19 | 20 => {
                    if self.hints.policy != HintPolicy::Unhinted {
                        return Err(unsupported(
                            "Type2 hints/masks unsupported in staged unhinted decoder",
                        ));
                    }
                    self.stem_operands(true, true)?;
                    let count = self.hints.stems.len();
                    if count == 0 {
                        return Err(invalid("Type2 mask without stems"));
                    }
                    let length = count.div_ceil(8);
                    let mask = data
                        .get(at..at + length)
                        .ok_or_else(|| invalid("Type2 truncated hint mask"))?
                        .to_vec();
                    at += length;
                    if !count.is_multiple_of(8)
                        && mask[length - 1] & ((1u8 << (8 - count % 8)) - 1) != 0
                    {
                        return Err(invalid("Type2 nonzero unused mask bits"));
                    }
                    self.hint_phase = 2;
                    self.hints.masks.push(HintMask {
                        counter: op == 20,
                        stem_count: count,
                        bytes: mask,
                    });
                }
                10 | 29 => {
                    let index = self
                        .operands
                        .pop()
                        .ok_or_else(|| invalid("Type2 subroutine operand missing"))?;
                    if index.shift() != 0 {
                        return Err(invalid("Type2 noninteger subroutine index"));
                    }
                    let global = op == 29;
                    let ranges = if global {
                        &self.cff.global_subrs
                    } else {
                        &self.cff.local_subrs
                    };
                    let bias = if ranges.len() < 1240 {
                        107
                    } else if ranges.len() < 33900 {
                        1131
                    } else {
                        32768
                    };
                    let index = index
                        .numerator()
                        .checked_add(bias)
                        .and_then(|i| usize::try_from(i).ok())
                        .ok_or_else(|| invalid("Type2 subroutine index"))?;
                    let range = ranges
                        .get(index)
                        .ok_or_else(|| invalid("Type2 subroutine index outside INDEX"))?
                        .clone();
                    if self.calls.len() >= 10 || self.calls.contains(&(global, index)) {
                        return Err(invalid("Type2 subroutine recursion/depth"));
                    }
                    self.calls.push((global, index));
                    let bytes = self.cff.bytes.clone();
                    let flow = self.execute(&bytes[range], true)?;
                    self.calls.pop();
                    if flow == Flow::End {
                        return Ok(flow);
                    }
                }
                11 => {
                    if !subroutine {
                        return Err(invalid("Type2 return outside subroutine"));
                    }
                    return Ok(Flow::Return);
                }
                14 => {
                    if self.operands.len() >= 4 {
                        return Err(unsupported("Type2 seac endchar unsupported"));
                    }
                    self.take_width(0)?;
                    if self.open {
                        self.emit(CubicCommand::Close)?;
                        self.open = false;
                    }
                    if at != data.len() {
                        return Err(invalid("Type2 bytes after endchar"));
                    }
                    return Ok(Flow::End);
                }
                4 | 21 | 22 => {
                    self.take_width(if op == 21 { 2 } else { 1 })?;
                    let v = std::mem::take(&mut self.operands);
                    if self.open {
                        self.emit(CubicCommand::Close)?;
                    }
                    let (x, y) = match op {
                        4 => (zero, v[0]),
                        22 => (v[0], zero),
                        _ => (v[0], v[1]),
                    };
                    let point = self.delta(x, y)?;
                    self.emit(CubicCommand::MoveTo(point))?;
                    self.open = true;
                    self.path_started = true;
                }
                5 => {
                    let v = std::mem::take(&mut self.operands);
                    if v.is_empty() || !v.len().is_multiple_of(2) || !self.open {
                        return Err(invalid("Type2 rlineto arity/moveto"));
                    }
                    for p in v.as_chunks::<2>().0 {
                        let point = self.delta(p[0], p[1])?;
                        self.emit(CubicCommand::LineTo(point))?;
                    }
                }
                6 | 7 => {
                    let v = std::mem::take(&mut self.operands);
                    if v.is_empty() || !self.open {
                        return Err(invalid("Type2 alternating line arity/moveto"));
                    }
                    let mut horizontal = op == 6;
                    for delta in v {
                        let point = if horizontal {
                            self.delta(delta, zero)?
                        } else {
                            self.delta(zero, delta)?
                        };
                        self.emit(CubicCommand::LineTo(point))?;
                        horizontal = !horizontal;
                    }
                }
                8 => {
                    let v = std::mem::take(&mut self.operands);
                    if v.is_empty() || !v.len().is_multiple_of(6) {
                        return Err(invalid("Type2 rrcurveto arity"));
                    }
                    for curve in v.as_chunks::<6>().0 {
                        self.curve(curve)?;
                    }
                }
                24 => {
                    let v = std::mem::take(&mut self.operands);
                    if v.len() < 8 || !(v.len() - 2).is_multiple_of(6) {
                        return Err(invalid("Type2 rcurveline arity"));
                    }
                    for curve in v[..v.len() - 2].as_chunks::<6>().0 {
                        self.curve(curve)?;
                    }
                    let point = self.delta(v[v.len() - 2], v[v.len() - 1])?;
                    self.emit(CubicCommand::LineTo(point))?;
                }
                25 => {
                    let v = std::mem::take(&mut self.operands);
                    if v.len() < 8 || !(v.len() - 6).is_multiple_of(2) || !self.open {
                        return Err(invalid("Type2 rlinecurve arity"));
                    }
                    for p in v[..v.len() - 6].as_chunks::<2>().0 {
                        let point = self.delta(p[0], p[1])?;
                        self.emit(CubicCommand::LineTo(point))?;
                    }
                    self.curve(&v[v.len() - 6..])?;
                }
                26 | 27 => {
                    let v = std::mem::take(&mut self.operands);
                    let extra = v.len() % 4;
                    if v.len() < 4 || extra > 1 {
                        return Err(invalid("Type2 vv/hh curve arity"));
                    }
                    let mut optional = if extra == 1 { v[0] } else { zero };
                    for p in v[extra..].as_chunks::<4>().0 {
                        let curve = if op == 26 {
                            [optional, p[0], p[1], p[2], zero, p[3]]
                        } else {
                            [p[0], optional, p[1], p[2], p[3], zero]
                        };
                        self.curve(&curve)?;
                        optional = zero;
                    }
                }
                30 | 31 => {
                    let v = std::mem::take(&mut self.operands);
                    let extra = v.len() % 4;
                    if v.len() < 4 || extra > 1 {
                        return Err(invalid("Type2 hv/vh curve arity"));
                    }
                    let count = v.len() / 4;
                    let mut horizontal = op == 31;
                    for (i, p) in v[..count * 4].as_chunks::<4>().0.iter().enumerate() {
                        let optional = if extra == 1 && i + 1 == count {
                            v[v.len() - 1]
                        } else {
                            zero
                        };
                        let curve = if horizontal {
                            [p[0], zero, p[1], p[2], optional, p[3]]
                        } else {
                            [zero, p[0], p[1], p[2], p[3], optional]
                        };
                        self.curve(&curve)?;
                        horizontal = !horizontal;
                    }
                }
                12 => {
                    let escaped = byte(data, &mut at)?;
                    if (34..=37).contains(&escaped) {
                        self.flex(escaped)?;
                    } else {
                        self.arithmetic(escaped)?;
                    }
                }
                _ => return Err(invalid("Type2 reserved/unsupported operator")),
            }
        }
        Err(invalid("Type2 program ended without return/endchar"))
    }
}
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(crate) fn font(program: &[u8], subr: Option<&[u8]>) -> Cff {
        let mut c = Cff::parse(&super::super::tests::fixture()).unwrap();
        let mut bytes = c.bytes.to_vec();
        let start = bytes.len();
        bytes.extend(program);
        c.charstrings = std::iter::once(start..bytes.len()).collect();
        if let Some(subr) = subr {
            let start = bytes.len();
            bytes.extend(subr);
            c.global_subrs = std::iter::once(start..bytes.len()).collect();
        }
        c.bytes = bytes.into();
        c
    }
    #[test]
    fn unhinted_masks_have_exact_byte_accounting_and_policy() {
        let mut program = vec![149];
        for _ in 0..9 {
            program.extend([139, 149]);
        }
        program.extend([18, 19, 255, 128, 139, 139, 21, 14]);
        let c = font(&program, None);
        assert!(c.cubic_outline(0).is_err());
        let out = c
            .cubic_outline_with_policy(0, HintPolicy::Unhinted)
            .unwrap();
        assert_eq!(out.width, Coordinate::from_integer(10));
        assert_eq!(out.hints.stems.len(), 9);
        assert_eq!(out.hints.masks[0].bytes, vec![255, 128]);
        assert_eq!(out.hints.policy, HintPolicy::Unhinted);
        let mask = program.len() - 5;
        program[mask] = 129;
        assert!(font(&program, None)
            .cubic_outline_with_policy(0, HintPolicy::Unhinted)
            .is_err());
    }
    #[test]
    fn malformed_masks_stems_and_budget_fail() {
        for program in [
            vec![19, 14],
            vec![139, 149, 18, 19],
            vec![139, 139, 21, 139, 149, 18, 14],
            vec![139, 149, 18, 139, 3, 14],
        ] {
            assert!(font(&program, None)
                .cubic_outline_with_policy(0, HintPolicy::Unhinted)
                .is_err());
        }
        let mut program = Vec::new();
        for _ in 0..5 {
            for _ in 0..20 {
                program.extend([139, 149]);
            }
            program.push(18);
        }
        program.push(14);
        assert!(font(&program, None)
            .cubic_outline_with_policy(0, HintPolicy::Unhinted)
            .is_err());
    }
    #[test]
    fn all_flex_forms_keep_two_exact_unhinted_curves() {
        for (op, args) in [
            (34, vec![10, 10, 5, 10, 10, 10, 10]),
            (35, vec![10, 0, 10, 5, 10, 0, 10, 0, 10, -5, 10, 0, 50]),
            (36, vec![10, 2, 10, 3, 10, 10, 10, -2, 10]),
            (37, vec![10, 2, 10, 3, 10, 0, 10, 0, 10, -2, 10]),
        ] {
            let mut program = vec![139, 139, 21];
            program.extend(args.iter().map(|n| (n + 139) as u8));
            program.extend([12, op, 14]);
            let c = font(&program, None);
            assert!(c.cubic_outline(0).is_err());
            let out = c
                .cubic_outline_with_policy(0, HintPolicy::Unhinted)
                .unwrap();
            assert_eq!(out.commands.len(), 4);
            assert_eq!(out.hints.flex_depths, vec![Coordinate::from_integer(50)]);
            match out.commands[2] {
                CubicCommand::CurveTo { end, .. } => {
                    assert_eq!(end.x, Coordinate::from_integer(60));
                    assert_eq!(end.y, Coordinate::from_integer(0));
                }
                _ => panic!("curve expected"),
            }
            program.remove(3);
            assert!(font(&program, None)
                .cubic_outline_with_policy(0, HintPolicy::Unhinted)
                .is_err());
        }
    }
    #[test]
    fn exact_cubic_points_and_width() {
        let c = font(
            &[149, 139, 139, 21, 149, 139, 149, 149, 149, 139, 8, 14],
            None,
        );
        let out = c.cubic_outline(0).unwrap();
        assert_eq!(out.width, Coordinate::from_integer(10));
        assert_eq!(out.commands.len(), 3);
        match out.commands[1] {
            CubicCommand::CurveTo {
                control1,
                control2,
                end,
            } => {
                assert_eq!(control1.x, Coordinate::from_integer(10));
                assert_eq!(control2.y, Coordinate::from_integer(10));
                assert_eq!(end.x, Coordinate::from_integer(30));
            }
            _ => panic!("curve expected"),
        }
    }
    #[test]
    fn subroutine_bias_and_cycles() {
        let c = font(&[139, 139, 21, 32, 29, 14], Some(&[149, 139, 5, 11]));
        assert!(c.cubic_outline(0).is_ok());
        let c = font(&[32, 29, 14], Some(&[32, 29, 11]));
        assert!(c.cubic_outline(0).is_err());
    }
    #[test]
    fn hints_masks_stack_and_truncation_fail() {
        for program in [
            vec![139, 139, 1, 14],
            vec![19, 14],
            vec![139; 49],
            vec![255, 0],
            vec![11],
            vec![139, 5, 14],
        ] {
            assert!(font(&program, None).cubic_outline(0).is_err());
        }
    }
    #[test]
    fn fixed_fraction_and_alternating_curves() {
        let c = font(
            &[255, 0, 0, 128, 0, 139, 21, 149, 149, 149, 149, 31, 14],
            None,
        );
        let out = c.cubic_outline(0).unwrap();
        match out.commands[0] {
            CubicCommand::MoveTo(p) => assert_eq!((p.x.numerator(), p.x.shift()), (1, 1)),
            _ => unreachable!(),
        };
    }
}
