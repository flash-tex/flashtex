use super::{Cff, CubicCommand, CubicPoint, DictNumber, HintMetadata, HintPolicy};
use crate::{invalid, Coordinate, Result};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
impl Rational {
    pub fn numerator(self) -> i128 {
        self.numerator
    }
    pub fn denominator(self) -> i128 {
        self.denominator
    }
    pub fn new(numerator: i128, denominator: i128) -> Result<Self> {
        if denominator <= 0 {
            return Err(invalid("CFF rational denominator"));
        }
        let common = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        Ok(Self {
            numerator: numerator / common,
            denominator: denominator / common,
        })
    }
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let common = gcd(self.denominator as u128, rhs.denominator as u128) as i128;
        let left_scale = rhs.denominator / common;
        let right_scale = self.denominator / common;
        let numerator = self
            .numerator
            .checked_mul(left_scale)
            .and_then(|n| {
                rhs.numerator
                    .checked_mul(right_scale)
                    .and_then(|r| n.checked_add(r))
            })
            .ok_or_else(|| invalid("CFF rational sum overflow"))?;
        Self::new(
            numerator,
            self.denominator
                .checked_mul(left_scale)
                .ok_or_else(|| invalid("CFF rational denominator overflow"))?,
        )
    }
    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        let a = gcd(self.numerator.unsigned_abs(), rhs.denominator as u128) as i128;
        let b = gcd(rhs.numerator.unsigned_abs(), self.denominator as u128) as i128;
        Self::new(
            (self.numerator / a)
                .checked_mul(rhs.numerator / b)
                .ok_or_else(|| invalid("CFF rational product overflow"))?,
            (self.denominator / b)
                .checked_mul(rhs.denominator / a)
                .ok_or_else(|| invalid("CFF rational denominator overflow"))?,
        )
    }
    fn coordinate(value: Coordinate) -> Result<Self> {
        Self::new(value.numerator(), 1i128 << value.shift())
    }
    pub(crate) fn operand(value: &DictNumber) -> Result<Self> {
        match value {
            DictNumber::Integer(n) => Self::new(*n as i128, 1),
            DictNumber::Decimal(text) => {
                let mut parts = text.split('E');
                let mantissa = parts.next().ok_or_else(|| invalid("CFF decimal empty"))?;
                let exponent = match parts.next() {
                    Some(value) => value
                        .parse::<i32>()
                        .map_err(|_| invalid("CFF decimal exponent"))?,
                    None => 0,
                };
                if parts.next().is_some() || exponent.unsigned_abs() > 38 {
                    return Err(invalid("CFF decimal exponent budget"));
                }
                let negative = mantissa.starts_with('-');
                let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
                let mut decimal = mantissa.split('.');
                let integer = decimal.next().unwrap_or("");
                let fraction = decimal.next().unwrap_or("");
                if decimal.next().is_some()
                    || integer.len() + fraction.len() > 38
                    || integer.is_empty() && fraction.is_empty()
                    || !integer
                        .bytes()
                        .chain(fraction.bytes())
                        .all(|b| b.is_ascii_digit())
                {
                    return Err(invalid("CFF decimal mantissa budget/syntax"));
                }
                let mut numerator = format!("{integer}{fraction}")
                    .parse::<i128>()
                    .map_err(|_| invalid("CFF decimal numerator"))?;
                if negative {
                    numerator = -numerator;
                }
                let power = fraction.len() as i32 - exponent;
                if power.unsigned_abs() > 38 {
                    return Err(invalid("CFF decimal scale budget"));
                }
                let scale = 10i128
                    .checked_pow(power.unsigned_abs())
                    .ok_or_else(|| invalid("CFF decimal scale overflow"))?;
                if power >= 0 {
                    Self::new(numerator, scale)
                } else {
                    Self::new(
                        numerator
                            .checked_mul(scale)
                            .ok_or_else(|| invalid("CFF decimal product overflow"))?,
                        1,
                    )
                }
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RationalPoint {
    pub x: Rational,
    pub y: Rational,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixCommand {
    MoveTo(RationalPoint),
    LineTo(RationalPoint),
    CurveTo {
        control1: RationalPoint,
        control2: RationalPoint,
        end: RationalPoint,
    },
    Close,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixOutline {
    pub cff_sha256: String,
    pub glyph_id: u16,
    pub matrix: [Rational; 6],
    pub advance: RationalPoint,
    pub commands: Vec<MatrixCommand>,
    pub hints: HintMetadata,
}
impl Cff {
    pub fn font_matrix(&self) -> Result<[Rational; 6]> {
        let zero = Rational::new(0, 1)?;
        let default = [
            Rational::new(1, 1000)?,
            zero,
            zero,
            Rational::new(1, 1000)?,
            zero,
            zero,
        ];
        let Some(values) = self.top.get(&0x0c07) else {
            return Ok(default);
        };
        if values.len() != 6 {
            return Err(invalid("CFF FontMatrix arity"));
        }
        let mut matrix = default;
        for (to, from) in matrix.iter_mut().zip(values) {
            *to = Rational::operand(from)?;
        }
        Ok(matrix)
    }
    /// Applies FontMatrix exactly into font/text space; no ppem scaling or grid fitting.
    pub fn matrix_outline(&self, gid: u16, policy: HintPolicy) -> Result<MatrixOutline> {
        if super::number(&self.top, 0x0c05, Some(0))? != 0 {
            return Err(super::unsupported(
                "CFF stroked PaintType is not represented by filled cubic output",
            ));
        }
        let raw = self.cubic_outline_with_policy(gid, policy)?;
        let matrix = self.font_matrix()?;
        let point = |p: CubicPoint| -> Result<RationalPoint> {
            let x = Rational::coordinate(p.x)?;
            let y = Rational::coordinate(p.y)?;
            Ok(RationalPoint {
                x: x.checked_mul(matrix[0])?
                    .checked_add(y.checked_mul(matrix[2])?)?
                    .checked_add(matrix[4])?,
                y: x.checked_mul(matrix[1])?
                    .checked_add(y.checked_mul(matrix[3])?)?
                    .checked_add(matrix[5])?,
            })
        };
        let commands = raw
            .commands
            .into_iter()
            .map(|command| {
                Ok(match command {
                    CubicCommand::MoveTo(p) => MatrixCommand::MoveTo(point(p)?),
                    CubicCommand::LineTo(p) => MatrixCommand::LineTo(point(p)?),
                    CubicCommand::CurveTo {
                        control1,
                        control2,
                        end,
                    } => MatrixCommand::CurveTo {
                        control1: point(control1)?,
                        control2: point(control2)?,
                        end: point(end)?,
                    },
                    CubicCommand::Close => MatrixCommand::Close,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let width = Rational::coordinate(raw.width)?;
        Ok(MatrixOutline {
            cff_sha256: raw.cff_sha256,
            glyph_id: gid,
            matrix,
            advance: RationalPoint {
                x: width.checked_mul(matrix[0])?,
                y: width.checked_mul(matrix[1])?,
            },
            commands,
            hints: raw.hints,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_decimal_and_default_matrix() {
        let c = Cff::parse(&super::super::tests::fixture()).unwrap();
        assert_eq!(c.font_matrix().unwrap()[0], Rational::new(1, 1000).unwrap());
        assert_eq!(
            Rational::operand(&DictNumber::Decimal("-1.25E-2".into())).unwrap(),
            Rational::new(-1, 80).unwrap()
        );
    }
    #[test]
    fn checked_fraction_arithmetic() {
        assert_eq!(
            Rational::new(1, 3)
                .unwrap()
                .checked_add(Rational::new(1, 6).unwrap())
                .unwrap(),
            Rational::new(1, 2).unwrap()
        );
        assert!(Rational::new(i128::MAX, 1)
            .unwrap()
            .checked_mul(Rational::new(2, 1).unwrap())
            .is_err());
        assert!(Rational::operand(&DictNumber::Decimal("1E100".into())).is_err());
    }
    #[test]
    fn matrix_api_preserves_mask_cycle_and_arithmetic_failures() {
        let malformed = super::super::type2::tests::font(&[139, 149, 18, 19], None);
        assert!(malformed.matrix_outline(0, HintPolicy::Unhinted).is_err());
        let recursive = super::super::type2::tests::font(&[32, 29, 14], Some(&[32, 29, 11]));
        assert!(recursive.matrix_outline(0, HintPolicy::Unhinted).is_err());
        let mut huge = super::super::type2::tests::font(&[149, 139, 21, 14], None);
        huge.top.insert(
            0x0c07,
            vec![
                DictNumber::Decimal("1E38".into()),
                DictNumber::Integer(0),
                DictNumber::Integer(0),
                DictNumber::Integer(1),
                DictNumber::Integer(0),
                DictNumber::Integer(0),
            ],
        );
        assert!(huge.matrix_outline(0, HintPolicy::Unhinted).is_err());
    }
    #[test]
    fn stroked_paint_type_is_not_silently_filled() {
        let mut c = super::super::type2::tests::font(&[139, 139, 21, 14], None);
        c.top.insert(0x0c05, vec![DictNumber::Integer(2)]);
        assert!(matches!(
            c.matrix_outline(0, HintPolicy::Unhinted),
            Err(crate::Error::UnsupportedFont(_))
        ));
    }
    #[test]
    fn affine_translation_does_not_translate_advance() {
        let mut c = super::super::type2::tests::font(&[149, 149, 159, 21, 14], None);
        c.top.insert(
            0x0c07,
            vec![
                DictNumber::Decimal("0.001".into()),
                DictNumber::Integer(0),
                DictNumber::Integer(0),
                DictNumber::Decimal("0.002".into()),
                DictNumber::Decimal("0.25".into()),
                DictNumber::Integer(0),
            ],
        );
        let out = c.matrix_outline(0, HintPolicy::Unhinted).unwrap();
        assert_eq!(out.advance.x, Rational::new(1, 100).unwrap());
        match out.commands[0] {
            MatrixCommand::MoveTo(p) => {
                assert_eq!(p.x, Rational::new(26, 100).unwrap());
                assert_eq!(p.y, Rational::new(4, 100).unwrap());
            }
            _ => panic!("move expected"),
        }
    }
}
