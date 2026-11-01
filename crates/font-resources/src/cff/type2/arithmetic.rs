//! Original bounded implementation of Adobe Type2 specification section4.4–4.6.
//! Exact dyadic output profile; no rounding of nonrepresentable results.
use super::*;
fn neg(v: Coordinate) -> Result<Coordinate> {
    Coordinate::new(
        v.numerator()
            .checked_neg()
            .ok_or_else(|| invalid("Type2 negation overflow"))?,
        v.shift(),
    )
}
fn index(v: Coordinate) -> Result<i128> {
    if v.shift() != 0 {
        return Err(invalid("Type2 noninteger stack/storage index"));
    }
    Ok(v.numerator())
}
impl Decoder<'_> {
    fn pop(&mut self) -> Result<Coordinate> {
        self.operands
            .pop()
            .ok_or_else(|| invalid("Type2 arithmetic operand underflow"))
    }
    fn push(&mut self, value: Coordinate) -> Result<()> {
        if self.operands.len() >= 48 {
            return Err(invalid("Type2 operand stack budget"));
        }
        self.operands.push(value);
        Ok(())
    }
    pub(super) fn arithmetic(&mut self, op: u8) -> Result<()> {
        let boolean = |b: bool| Coordinate::from_integer(i32::from(b));
        match op {
            3 | 4 | 10 | 11 | 12 | 15 | 24 => {
                let b = self.pop()?;
                let a = self.pop()?;
                let v = match op {
                    3 => boolean(a.numerator() != 0 && b.numerator() != 0),
                    4 => boolean(a.numerator() != 0 || b.numerator() != 0),
                    10 => a.add(b)?,
                    11 => a.add(neg(b)?)?,
                    15 => boolean(a == b),
                    24 => a.multiply(b)?,
                    12 => {
                        if b.numerator() == 0 {
                            return Err(invalid("Type2 division by zero"));
                        }
                        let a = super::super::Rational::new(a.numerator(), 1i128 << a.shift())?;
                        let sign = if b.numerator() < 0 { -1 } else { 1 };
                        let reciprocal = super::super::Rational::new(
                            sign * (1i128 << b.shift()),
                            b.numerator()
                                .checked_abs()
                                .ok_or_else(|| invalid("Type2 divisor overflow"))?,
                        )?;
                        let q = a.checked_mul(reciprocal)?;
                        let denominator = q.denominator() as u128;
                        if !denominator.is_power_of_two() {
                            return Err(unsupported(
                                "Type2 non-dyadic quotient outside exact Coordinate profile",
                            ));
                        }
                        Coordinate::new(q.numerator(), denominator.trailing_zeros())?
                    }
                    _ => unreachable!(),
                };
                self.push(v)?;
            }
            5 | 9 | 14 | 26 => {
                let a = self.pop()?;
                let v = match op {
                    5 => boolean(a.numerator() == 0),
                    9 => {
                        if a.numerator() < 0 {
                            neg(a)?
                        } else {
                            a
                        }
                    }
                    14 => neg(a)?,
                    26 => {
                        if a.numerator() < 0 {
                            return Err(invalid("Type2 negative square root"));
                        }
                        let mut numerator = a.numerator() as u128;
                        let mut shift = a.shift();
                        if shift % 2 == 1 {
                            numerator = numerator
                                .checked_mul(2)
                                .ok_or_else(|| invalid("Type2 square root overflow"))?;
                            shift += 1;
                        }
                        let root = numerator.isqrt();
                        if root * root != numerator {
                            return Err(unsupported(
                                "Type2 irrational square root outside exact profile",
                            ));
                        }
                        Coordinate::new(
                            i128::try_from(root)
                                .map_err(|_| invalid("Type2 square root overflow"))?,
                            shift / 2,
                        )?
                    }
                    _ => unreachable!(),
                };
                self.push(v)?;
            }
            18 => {
                self.pop()?;
            }
            20 | 21 => {
                let i = usize::try_from(index(self.pop()?)?)
                    .map_err(|_| invalid("Type2 transient index"))?;
                if i >= 32 {
                    return Err(invalid("Type2 transient index"));
                }
                if op == 20 {
                    self.transient[i] = Some(self.pop()?);
                } else {
                    self.push(
                        self.transient[i]
                            .ok_or_else(|| invalid("Type2 uninitialized transient read"))?,
                    )?;
                }
            }
            22 => {
                let v2 = self.pop()?;
                let v1 = self.pop()?;
                let s2 = self.pop()?;
                let s1 = self.pop()?;
                let difference = v1.add(neg(v2)?)?;
                self.push(if difference.numerator() <= 0 { s1 } else { s2 })?;
            }
            23 => return Err(unsupported("Type2 random nondeterministic operation")),
            27 => {
                let v = *self
                    .operands
                    .last()
                    .ok_or_else(|| invalid("Type2 dup underflow"))?;
                self.push(v)?;
            }
            28 => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(b)?;
                self.push(a)?;
            }
            29 => {
                let i = index(self.pop()?)?.max(0);
                let i = usize::try_from(i).map_err(|_| invalid("Type2 index outside stack"))?;
                let at = self
                    .operands
                    .len()
                    .checked_sub(
                        i.checked_add(1)
                            .ok_or_else(|| invalid("Type2 index overflow"))?,
                    )
                    .ok_or_else(|| invalid("Type2 index outside stack"))?;
                self.push(self.operands[at])?;
            }
            30 => {
                let j = index(self.pop()?)?;
                let n = index(self.pop()?)?;
                let n = usize::try_from(n).map_err(|_| invalid("Type2 negative roll count"))?;
                if n > self.operands.len() {
                    return Err(invalid("Type2 roll outside stack"));
                }
                if n != 0 {
                    let start = self.operands.len() - n;
                    self.operands[start..].rotate_right(j.rem_euclid(n as i128) as usize);
                }
            }
            _ => {
                return Err(unsupported(&format!(
                    "Type2 escaped operator {op} unsupported"
                )))
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::font;
    use super::*;
    fn x(expression: &[u8]) -> Result<Coordinate> {
        let mut program = expression.to_vec();
        program.extend([139, 21, 14]);
        let outline = font(&program, None).cubic_outline(0)?;
        match outline.commands[0] {
            CubicCommand::MoveTo(p) => Ok(p.x),
            _ => panic!("moveto"),
        }
    }
    #[test]
    fn exact_arithmetic_and_conditionals() {
        for (code, want) in [
            (vec![141, 142, 12, 10], 5),
            (vec![141, 142, 12, 11], -1),
            (vec![141, 142, 12, 24], 6),
            (vec![135, 12, 9], 4),
            (vec![143, 12, 14], -4),
            (vec![148, 12, 26], 3),
            (vec![141, 142, 12, 3], 1),
            (vec![139, 142, 12, 4], 1),
            (vec![139, 12, 5], 1),
            (vec![142, 142, 12, 15], 1),
            (vec![146, 147, 140, 141, 12, 22], 7),
            (vec![146, 147, 142, 141, 12, 22], 8),
        ] {
            assert_eq!(x(&code).unwrap(), Coordinate::from_integer(want));
        }
        assert_eq!(
            x(&[142, 141, 12, 12]).unwrap(),
            Coordinate::new(3, 1).unwrap()
        );
        assert_eq!(
            x(&[140, 143, 12, 12, 12, 26]).unwrap(),
            Coordinate::new(1, 1).unwrap()
        );
    }
    #[test]
    fn stack_storage_roll_and_subroutine_state() {
        for code in [
            vec![144, 12, 27, 12, 10],
            vec![144, 139, 12, 20, 139, 12, 21, 141, 12, 24],
            vec![146, 144, 12, 28, 12, 18, 141, 12, 24],
            vec![144, 138, 12, 29, 12, 10],
            vec![146, 144, 141, 140, 12, 30, 12, 18, 141, 12, 24],
        ] {
            assert_eq!(x(&code).unwrap(), Coordinate::from_integer(10));
        }
        let c = font(
            &[144, 139, 12, 20, 32, 29, 139, 21, 14],
            Some(&[139, 12, 21, 11]),
        );
        assert!(c.cubic_outline(0).is_ok());
        assert!(x(&[139, 12, 21]).is_err()); // transient never implicitly initialized
    }
    #[test]
    fn malformed_unsupported_and_resource_limits() {
        for code in [
            vec![12, 10],
            vec![12, 27],
            vec![140, 139, 12, 12],
            vec![138, 12, 26],
            vec![140, 171, 12, 20],
            vec![140, 141, 12, 29],
            vec![140, 142, 140, 12, 30],
        ] {
            assert!(x(&code).is_err());
        }
        for code in [vec![12, 23], vec![140, 142, 12, 12], vec![141, 12, 26]] {
            assert!(matches!(x(&code), Err(crate::Error::UnsupportedFont(_))));
        }
        let mut overflow = vec![141];
        for _ in 0..8 {
            overflow.extend([12, 27, 12, 24]);
        }
        assert!(x(&overflow).is_err());
        let mut stack = vec![139; 48];
        stack.extend([12, 27]);
        assert!(x(&stack).is_err());
        let mut budget = Vec::new();
        for _ in 0..50001 {
            budget.extend([139, 12, 18]);
        }
        assert!(x(&budget)
            .unwrap_err()
            .to_string()
            .contains("instruction budget"));
        assert!(font(&[32, 29, 14], Some(&[32, 29, 11]))
            .cubic_outline(0)
            .is_err());
    }
}
