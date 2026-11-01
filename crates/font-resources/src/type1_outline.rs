//! Original bounded Type1 charstring subset. No hints, PostScript or FontMatrix application.
use crate::{
    cff::{CubicCommand, CubicPoint, HintPolicy, MatrixCommand, Rational, RationalPoint},
    pfb::Identity,
    sha256,
    type1_records::Records,
    Coordinate,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Record(crate::type1_records::Error),
    Truncated,
    Stack,
    Width,
    Path,
    Call,
    Trailing,
    Budget,
    Arithmetic,
    UnrepresentableDivision,
    Unsupported(u16),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSource {
    pub subroutine_chain: Vec<usize>,
    pub byte_offset: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub hints: HintPolicy,
    pub exact_division: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            hints: HintPolicy::Reject,
            exact_division: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stem {
    pub vertical: bool,
    pub relative_position: Coordinate,
    pub position: Coordinate,
    pub width: Coordinate,
    pub triple: bool,
    pub source: CommandSource,
}
pub struct Outline {
    pub identity: Identity,
    pub glyph_name: String,
    pub charstring_sha256: String,
    pub sidebearing: CubicPoint,
    pub advance: CubicPoint,
    pub commands: Vec<CubicCommand>,
    pub sources: Vec<CommandSource>,
    pub policy: Policy,
    pub stems: Vec<Stem>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RationalStem {
    pub vertical: bool,
    pub relative_position: Rational,
    pub position: Rational,
    pub width: Rational,
    pub triple: bool,
    pub source: CommandSource,
}
pub struct RationalOutline {
    pub identity: Identity,
    pub glyph_name: String,
    pub charstring_sha256: String,
    pub sidebearing: RationalPoint,
    pub advance: RationalPoint,
    pub commands: Vec<MatrixCommand>,
    pub sources: Vec<CommandSource>,
    pub policy: Policy,
    pub stems: Vec<RationalStem>,
}
impl RationalOutline {
    /// Explicit exact-only conversion. Never rounds non-dyadic geometry.
    pub fn try_into_dyadic(self) -> Result<Outline, Error> {
        convert(self)
    }
}
fn point(x: i32, y: i32) -> RationalPoint {
    RationalPoint {
        x: integer(x),
        y: integer(y),
    }
}
struct Decoder<F> {
    lookup: F,
    stack: Vec<Rational>,
    policy: Policy,
    rational_division: bool,
    stems: Vec<RationalStem>,
    at: RationalPoint,
    bearing: RationalPoint,
    advance: RationalPoint,
    width: bool,
    open: bool,
    commands: Vec<MatrixCommand>,
    sources: Vec<CommandSource>,
    calls: Vec<usize>,
    steps: usize,
    bytes: usize,
}
#[derive(PartialEq)]
enum Flow {
    End,
    Return,
}
impl<F: FnMut(usize) -> Result<Vec<u8>, Error>> Decoder<F> {
    fn args(&mut self, n: usize) -> Result<Vec<Rational>, Error> {
        if self.stack.len() != n {
            return Err(Error::Stack);
        }
        Ok(std::mem::take(&mut self.stack))
    }
    fn delta(&mut self, x: Rational, y: Rational) -> Result<RationalPoint, Error> {
        self.at.x = self.at.x.checked_add(x).map_err(|_| Error::Arithmetic)?;
        self.at.y = self.at.y.checked_add(y).map_err(|_| Error::Arithmetic)?;
        if !self.rational_division {
            to_coordinate(self.at.x)?;
            to_coordinate(self.at.y)?;
        }
        Ok(self.at)
    }
    fn emit(&mut self, command: MatrixCommand, offset: usize) -> Result<(), Error> {
        if self.commands.len() == 16384 {
            return Err(Error::Budget);
        }
        self.commands.push(command);
        self.sources.push(CommandSource {
            subroutine_chain: self.calls.clone(),
            byte_offset: offset,
        });
        Ok(())
    }
    fn run(&mut self, b: &[u8], nested: bool) -> Result<Flow, Error> {
        self.bytes = self.bytes.checked_add(b.len()).ok_or(Error::Budget)?;
        if self.bytes > 16 * 1024 * 1024 {
            return Err(Error::Budget);
        }
        let mut i = 0;
        while i < b.len() {
            self.steps += 1;
            if self.steps > 65536 {
                return Err(Error::Budget);
            }
            let offset = i;
            let op = b[i];
            i += 1;
            let mut byte = || {
                let v = *b.get(i).ok_or(Error::Truncated)?;
                i += 1;
                Ok::<_, Error>(v)
            };
            if op >= 32 {
                let n = match op {
                    32..=246 => i32::from(op) - 139,
                    247..=250 => (i32::from(op) - 247) * 256 + i32::from(byte()?) + 108,
                    251..=254 => -(i32::from(op) - 251) * 256 - i32::from(byte()?) - 108,
                    255 => i32::from_be_bytes([byte()?, byte()?, byte()?, byte()?]),
                    _ => unreachable!(),
                };
                if self.stack.len() == 24 {
                    return Err(Error::Stack);
                }
                self.stack.push(integer(n));
                continue;
            }
            let op = if op == 12 {
                0x100 + u16::from(byte()?)
            } else {
                u16::from(op)
            };
            if !self.width && matches!(op, 4..=9 | 14 | 21 | 22 | 30 | 31) {
                return Err(Error::Width);
            }
            match op {
                0x10c if self.policy.exact_division => {
                    let denominator = self.stack.pop().ok_or(Error::Stack)?;
                    let numerator = self.stack.pop().ok_or(Error::Stack)?;
                    let quotient = rational_divide(numerator, denominator)?;
                    if !self.rational_division {
                        to_coordinate(quotient)?;
                    }
                    self.stack.push(quotient);
                }
                1 | 3 | 0x101 | 0x102 if self.policy.hints == HintPolicy::Unhinted => {
                    if !self.width {
                        return Err(Error::Width);
                    }
                    let triple = op >= 0x100;
                    let a = self.args(if triple { 6 } else { 2 })?;
                    if self.stems.len() + a.len() / 2 > 4096 {
                        return Err(Error::Budget);
                    }
                    let vertical = matches!(op, 3 | 0x101);
                    let bearing = if vertical {
                        self.bearing.x
                    } else {
                        self.bearing.y
                    };
                    for pair in a.as_chunks::<2>().0 {
                        self.stems.push(RationalStem {
                            vertical,
                            relative_position: pair[0],
                            position: pair[0]
                                .checked_add(bearing)
                                .map_err(|_| Error::Arithmetic)?,
                            width: pair[1],
                            triple,
                            source: CommandSource {
                                subroutine_chain: self.calls.clone(),
                                byte_offset: offset,
                            },
                        });
                    }
                }

                13 | 0x107 => {
                    if self.width {
                        return Err(Error::Width);
                    }
                    let a = self.args(if op == 13 { 2 } else { 4 })?;
                    self.bearing = if op == 13 {
                        cpoint(a[0], integer(0))
                    } else {
                        cpoint(a[0], a[1])
                    };
                    self.advance = if op == 13 {
                        cpoint(a[1], integer(0))
                    } else {
                        cpoint(a[2], a[3])
                    };
                    self.at = self.bearing;
                    self.width = true;
                }
                4 | 21 | 22 => {
                    if self.open {
                        return Err(Error::Path);
                    }
                    let a = self.args(if op == 21 { 2 } else { 1 })?;
                    let (x, y) = match op {
                        4 => (integer(0), a[0]),
                        22 => (a[0], integer(0)),
                        _ => (a[0], a[1]),
                    };
                    let p = self.delta(x, y)?;
                    self.emit(MatrixCommand::MoveTo(p), offset)?;
                    self.open = true;
                }
                5..=7 => {
                    if !self.open {
                        return Err(Error::Path);
                    }
                    let a = self.args(if op == 5 { 2 } else { 1 })?;
                    let (x, y) = match op {
                        6 => (a[0], integer(0)),
                        7 => (integer(0), a[0]),
                        _ => (a[0], a[1]),
                    };
                    let p = self.delta(x, y)?;
                    self.emit(MatrixCommand::LineTo(p), offset)?;
                }
                8 | 30 | 31 => {
                    if !self.open {
                        return Err(Error::Path);
                    }
                    let a = self.args(if op == 8 { 6 } else { 4 })?;
                    let d = match op {
                        30 => [integer(0), a[0], a[1], a[2], a[3], integer(0)],
                        31 => [a[0], integer(0), a[1], a[2], integer(0), a[3]],
                        _ => [a[0], a[1], a[2], a[3], a[4], a[5]],
                    };
                    let control1 = self.delta(d[0], d[1])?;
                    let control2 = self.delta(d[2], d[3])?;
                    let end = self.delta(d[4], d[5])?;
                    self.emit(
                        MatrixCommand::CurveTo {
                            control1,
                            control2,
                            end,
                        },
                        offset,
                    )?;
                }
                9 => {
                    self.args(0)?;
                    if !self.open {
                        return Err(Error::Path);
                    }
                    self.emit(MatrixCommand::Close, offset)?;
                    self.open = false;
                }
                10 => {
                    let operand = self.stack.pop().ok_or(Error::Stack)?;
                    if operand.denominator() != 1 {
                        return Err(Error::Call);
                    }
                    let index = usize::try_from(operand.numerator()).map_err(|_| Error::Call)?;
                    if self.calls.len() == 16 || self.calls.contains(&index) {
                        return Err(Error::Call);
                    }
                    let bytes = (self.lookup)(index)?;
                    self.calls.push(index);
                    let flow = self.run(&bytes, true)?;
                    self.calls.pop();
                    if flow == Flow::End {
                        if i != b.len() {
                            return Err(Error::Trailing);
                        }
                        return Ok(flow);
                    }
                }
                11 => {
                    if !nested {
                        return Err(Error::Call);
                    }
                    if i != b.len() {
                        return Err(Error::Trailing);
                    }
                    return Ok(Flow::Return);
                }
                14 => {
                    self.args(0)?;
                    if self.open {
                        return Err(Error::Path);
                    }
                    if i != b.len() {
                        return Err(Error::Trailing);
                    }
                    return Ok(Flow::End);
                }
                other => return Err(Error::Unsupported(other)),
            }
        }
        Err(Error::Truncated)
    }
}
pub fn interpret(records: &Records<'_>, glyph_name: &str) -> Result<Outline, Error> {
    interpret_with_policy(records, glyph_name, Policy::default())
}
pub fn interpret_with_policy(
    records: &Records<'_>,
    glyph_name: &str,
    policy: Policy,
) -> Result<Outline, Error> {
    convert(run_records(records, glyph_name, policy, false)?)
}
/// Exact rational raw character-space API; no FontMatrix/PaintType or renderer activation.
pub fn interpret_rational(
    records: &Records<'_>,
    glyph_name: &str,
    policy: Policy,
) -> Result<RationalOutline, Error> {
    run_records(records, glyph_name, policy, true)
}
fn run_records(
    records: &Records<'_>,
    glyph_name: &str,
    policy: Policy,
    rational: bool,
) -> Result<RationalOutline, Error> {
    let bytes = records.decrypted_glyph(glyph_name).map_err(Error::Record)?;
    let mut d = decoder(|i| records.decrypted_subr(i).map_err(Error::Record));
    d.policy = policy;
    d.rational_division = rational;
    if d.run(&bytes, false)? != Flow::End {
        return Err(Error::Call);
    }
    Ok(RationalOutline {
        identity: records.identity().clone(),
        glyph_name: glyph_name.into(),
        charstring_sha256: sha256(&bytes),
        sidebearing: d.bearing,
        advance: d.advance,
        commands: d.commands,
        sources: d.sources,
        policy,
        stems: d.stems,
    })
}
fn convert(raw: RationalOutline) -> Result<Outline, Error> {
    if raw.commands.len() > 16384
        || raw.commands.len() != raw.sources.len()
        || raw.stems.len() > 4096
        || raw.sources.iter().any(|s| s.subroutine_chain.len() > 16)
    {
        return Err(Error::Budget);
    }
    let cv = |p: RationalPoint| {
        Ok::<_, Error>(CubicPoint {
            x: to_coordinate(p.x)?,
            y: to_coordinate(p.y)?,
        })
    };
    let commands = raw
        .commands
        .into_iter()
        .map(|c| {
            Ok(match c {
                MatrixCommand::MoveTo(p) => CubicCommand::MoveTo(cv(p)?),
                MatrixCommand::LineTo(p) => CubicCommand::LineTo(cv(p)?),
                MatrixCommand::CurveTo {
                    control1,
                    control2,
                    end,
                } => CubicCommand::CurveTo {
                    control1: cv(control1)?,
                    control2: cv(control2)?,
                    end: cv(end)?,
                },
                MatrixCommand::Close => CubicCommand::Close,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let stems = raw
        .stems
        .into_iter()
        .map(|s| {
            Ok(Stem {
                vertical: s.vertical,
                relative_position: to_coordinate(s.relative_position)?,
                position: to_coordinate(s.position)?,
                width: to_coordinate(s.width)?,
                triple: s.triple,
                source: s.source,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Outline {
        identity: raw.identity,
        glyph_name: raw.glyph_name,
        charstring_sha256: raw.charstring_sha256,
        sidebearing: cv(raw.sidebearing)?,
        advance: cv(raw.advance)?,
        commands,
        sources: raw.sources,
        policy: raw.policy,
        stems,
    })
}
fn decoder<F: FnMut(usize) -> Result<Vec<u8>, Error>>(lookup: F) -> Decoder<F> {
    Decoder {
        lookup,
        stack: vec![],
        policy: Policy::default(),
        rational_division: false,
        stems: vec![],
        at: point(0, 0),
        bearing: point(0, 0),
        advance: point(0, 0),
        width: false,
        open: false,
        commands: vec![],
        sources: vec![],
        calls: vec![],
        steps: 0,
        bytes: 0,
    }
}
fn integer(n: i32) -> Rational {
    Rational::new(i128::from(n), 1).expect("integer denominator is positive")
}
fn cpoint(x: Rational, y: Rational) -> RationalPoint {
    RationalPoint { x, y }
}
fn rational_divide(a: Rational, b: Rational) -> Result<Rational, Error> {
    if b.numerator() == 0 {
        return Err(Error::Arithmetic);
    }
    let sign = if b.numerator() < 0 { -1i128 } else { 1 };
    let inverse = Rational::new(
        b.denominator().checked_mul(sign).ok_or(Error::Arithmetic)?,
        b.numerator().checked_abs().ok_or(Error::Arithmetic)?,
    )
    .map_err(|_| Error::Arithmetic)?;
    a.checked_mul(inverse).map_err(|_| Error::Arithmetic)
}
fn to_coordinate(ratio: Rational) -> Result<Coordinate, Error> {
    let denominator = ratio.denominator() as u128;
    if !denominator.is_power_of_two() {
        return Err(Error::UnrepresentableDivision);
    }
    Coordinate::new(ratio.numerator(), denominator.trailing_zeros())
        .map_err(|_| Error::UnrepresentableDivision)
}
#[cfg(test)]
fn divide(a: Coordinate, b: Coordinate) -> Result<Coordinate, Error> {
    to_coordinate(rational_divide(
        Rational::new(a.numerator(), 1i128 << a.shift()).map_err(|_| Error::Arithmetic)?,
        Rational::new(b.numerator(), 1i128 << b.shift()).map_err(|_| Error::Arithmetic)?,
    )?)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn num(n: i32, b: &mut Vec<u8>) {
        b.push(255);
        b.extend(n.to_be_bytes())
    }
    fn base() -> Vec<u8> {
        let mut b = vec![];
        num(10, &mut b);
        num(500, &mut b);
        b.push(13);
        b
    }
    #[test]
    fn exact_lines_curves_closepoint_and_subroutine_provenance() {
        let mut b = base();
        num(0, &mut b);
        num(0, &mut b);
        b.push(21);
        num(0, &mut b);
        b.push(10);
        b.push(9);
        num(1, &mut b);
        b.push(22);
        b.push(9);
        b.push(14);
        let mut sub = vec![];
        for n in [1, 2, 3, 4, 5, 6] {
            num(n, &mut sub)
        }
        sub.extend([8, 11]);
        let mut d = decoder(|_| Ok(sub.clone()));
        assert!(d.run(&b, false) == Ok(Flow::End));
        assert_eq!(d.advance, point(500, 0));
        assert_eq!(
            d.commands[1],
            MatrixCommand::CurveTo {
                control1: point(11, 2),
                control2: point(14, 6),
                end: point(19, 12)
            }
        );
        assert_eq!(d.commands[3], MatrixCommand::MoveTo(point(20, 12)));
        assert_eq!(d.sources[1].subroutine_chain, vec![0]);
    }
    #[test]
    fn unsupported_stack_cycles_and_invalid_control_flow() {
        for op in [1, 3, 0x100, 0x106, 0x10c, 0x110, 0x111] {
            let mut b = base();
            if op >= 256 {
                b.extend([12, (op - 256) as u8])
            } else {
                b.push(op as u8)
            }
            let mut d = decoder(|_| Err(Error::Call));
            assert_eq!(d.run(&b, false).err(), Some(Error::Unsupported(op)));
        }
        let mut b = base();
        for _ in 0..25 {
            num(0, &mut b)
        }
        assert_eq!(
            decoder(|_| Err(Error::Call)).run(&b, false).err(),
            Some(Error::Stack)
        );
        let mut b = base();
        num(0, &mut b);
        b.push(10);
        let mut sub = vec![];
        num(0, &mut sub);
        sub.extend([10, 11]);
        assert_eq!(
            decoder(|_| Ok(sub.clone())).run(&b, false).err(),
            Some(Error::Call)
        );
        assert_eq!(
            decoder(|_| Err(Error::Call)).run(&[255, 1], false).err(),
            Some(Error::Truncated)
        );
    }
    #[test]
    fn width_subroutine_and_hard_execution_output_limits() {
        let mut b = vec![];
        num(0, &mut b);
        b.extend([10, 14]);
        let mut width = base();
        width.push(11);
        let mut d = decoder(|_| Ok(width.clone()));
        assert!(d.run(&b, false) == Ok(Flow::End));
        assert_eq!(d.advance, point(500, 0));
        let mut b = base();
        num(0, &mut b);
        num(0, &mut b);
        b.push(21);
        for _ in 0..16384 {
            num(i32::MAX, &mut b);
            b.push(6);
        }
        b.extend([9, 14]);
        assert_eq!(
            decoder(|_| Err(Error::Call)).run(&b, false).err(),
            Some(Error::Budget)
        );
        let mut b = base();
        for _ in 0..22000 {
            num(0, &mut b);
            b.push(10);
        }
        b.push(14);
        assert_eq!(
            decoder(|_| Ok(vec![11])).run(&b, false).err(),
            Some(Error::Budget)
        );
    }
    #[test]
    fn exact_division_and_explicit_stem_policy_preserve_raw_values() {
        let mut b = base();
        num(3, &mut b);
        num(2, &mut b);
        b.extend([12, 12]);
        num(-20, &mut b);
        b.push(3);
        b.push(14);
        assert_eq!(
            decoder(|_| Err(Error::Call)).run(&b, false).err(),
            Some(Error::Unsupported(268))
        );
        let mut d = decoder(|_| Err(Error::Call));
        d.policy = Policy {
            hints: HintPolicy::Unhinted,
            exact_division: true,
        };
        assert!(d.run(&b, false) == Ok(Flow::End));
        assert_eq!(d.stems[0].relative_position, Rational::new(3, 2).unwrap());
        assert_eq!(d.stems[0].position, Rational::new(23, 2).unwrap());
        assert_eq!(d.stems[0].width, integer(-20));
        assert!(d.commands.is_empty());
        assert_eq!(
            divide(Coordinate::from_integer(1), Coordinate::from_integer(3)),
            Err(Error::UnrepresentableDivision)
        );
        assert_eq!(
            divide(Coordinate::from_integer(1), Coordinate::from_integer(0)),
            Err(Error::Arithmetic)
        );
        assert_eq!(
            divide(Coordinate::from_integer(3), Coordinate::from_integer(-2)),
            Ok(Coordinate::new(-3, 1).unwrap())
        );
        let mut b = base();
        num(1, &mut b);
        b.push(1);
        let mut d = decoder(|_| Err(Error::Call));
        d.policy.hints = HintPolicy::Unhinted;
        assert_eq!(d.run(&b, false).err(), Some(Error::Stack));
    }
    #[test]
    fn rational_width_translation_curve_and_overflow() {
        let mut bytes = Vec::new();
        num(1, &mut bytes);
        num(3, &mut bytes);
        bytes.extend([12, 12]);
        num(100, &mut bytes);
        num(3, &mut bytes);
        bytes.extend([12, 12, 13]);
        for n in [1, 3] {
            num(n, &mut bytes)
        }
        bytes.extend([12, 12]);
        num(0, &mut bytes);
        bytes.push(21);
        for n in [1, 1, 1, 1, 1, 1] {
            num(n, &mut bytes)
        }
        bytes.extend([8, 9, 14]);
        let mut d = decoder(|_| Err(Error::Call));
        d.policy = Policy {
            hints: HintPolicy::Unhinted,
            exact_division: true,
        };
        d.rational_division = true;
        assert!(d.run(&bytes, false) == Ok(Flow::End));
        assert_eq!(d.bearing.x, Rational::new(1, 3).unwrap());
        assert_eq!(d.advance.x, Rational::new(100, 3).unwrap());
        assert_eq!(
            d.commands[0],
            MatrixCommand::MoveTo(RationalPoint {
                x: Rational::new(2, 3).unwrap(),
                y: integer(0)
            })
        );
        assert_eq!(
            d.commands[1],
            MatrixCommand::CurveTo {
                control1: RationalPoint {
                    x: Rational::new(5, 3).unwrap(),
                    y: integer(1)
                },
                control2: RationalPoint {
                    x: Rational::new(8, 3).unwrap(),
                    y: integer(2)
                },
                end: RationalPoint {
                    x: Rational::new(11, 3).unwrap(),
                    y: integer(3)
                }
            }
        );
        assert_eq!(
            rational_divide(
                Rational::new(i128::MAX, 1).unwrap(),
                Rational::new(1, 2).unwrap()
            ),
            Err(Error::Arithmetic)
        );
    }
}
