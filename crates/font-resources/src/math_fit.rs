//! Exact target fitting under one declared, bounded strategy. Not TeX layout.
use crate::{
    cff::Rational,
    math_variants::{Direction, MathVariants, Variant},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitStrategy {
    EqualExtendersProportionalConnectorFlexibility,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitLimits {
    pub max_repetitions: usize,
    pub max_parts: usize,
}
impl Default for FitLimits {
    fn default() -> Self {
        Self {
            max_repetitions: 1024,
            max_parts: 4096,
        }
    }
}
#[derive(Debug)]
pub enum FitError {
    InvalidTarget,
    InvalidLimits,
    Unavailable,
    Unrepresentable,
    Budget,
    Arithmetic(crate::Error),
}
impl From<crate::Error> for FitError {
    fn from(e: crate::Error) -> Self {
        Self::Arithmetic(e)
    }
}
impl std::fmt::Display for FitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FitError {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FittedPart {
    pub glyph_id: u16,
    pub part_index: usize,
    pub instance: usize,
    pub offset: Rational,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FittedAssembly {
    pub parts: Vec<FittedPart>,
    pub overlaps: Vec<Rational>,
    pub advance: Rational,
    pub italic_correction: i16,
    pub device_adjustment_present: bool,
    pub extender_repetitions: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FittedShape {
    Variant(Variant),
    Assembly(FittedAssembly),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathFit {
    pub direction: Direction,
    pub original_glyph_id: u16,
    pub target: Rational,
    pub strategy: FitStrategy,
    pub shape: FittedShape,
}
fn integer(value: i128) -> Rational {
    Rational::new(value, 1).expect("integer rational")
}
fn neg(value: Rational) -> Result<Rational, FitError> {
    Ok(Rational::new(
        value
            .numerator()
            .checked_neg()
            .ok_or_else(|| crate::Error::InvalidFont("MATH fit negation overflow".into()))?,
        value.denominator(),
    )?)
}
fn cmp_integer(value: Rational, n: i128) -> Result<std::cmp::Ordering, FitError> {
    let right = n
        .checked_mul(value.denominator())
        .ok_or_else(|| crate::Error::InvalidFont("MATH fit comparison overflow".into()))?;
    Ok(value.numerator().cmp(&right))
}
impl MathVariants {
    pub fn fit(
        &self,
        direction: Direction,
        glyph_id: u16,
        target: Rational,
        strategy: FitStrategy,
        limits: FitLimits,
    ) -> Result<MathFit, FitError> {
        if target.numerator() <= 0 {
            return Err(FitError::InvalidTarget);
        }
        if limits.max_repetitions > 1024 || limits.max_parts == 0 || limits.max_parts > 4096 {
            return Err(FitError::InvalidLimits);
        }
        let construction = self
            .constructions()
            .get(&(direction, glyph_id))
            .ok_or(FitError::Unavailable)?;
        let wrap = |shape| MathFit {
            direction,
            original_glyph_id: glyph_id,
            target,
            strategy,
            shape,
        };
        for &variant in &construction.variants {
            if !cmp_integer(target, i128::from(variant.advance))?.is_gt() {
                return Ok(wrap(FittedShape::Variant(variant)));
            }
        }
        let assembly = construction
            .assembly
            .as_ref()
            .ok_or(FitError::Unrepresentable)?;
        let mut budget_hit = false;
        let has_extenders = assembly.parts.iter().any(|p| p.extender);
        for repetitions in 0..=if has_extenders {
            limits.max_repetitions
        } else {
            0
        } {
            let mut selected = Vec::new();
            for (part_index, part) in assembly.parts.iter().enumerate() {
                let count = if part.extender { repetitions } else { 1 };
                if selected.len() + count > limits.max_parts {
                    budget_hit = true;
                    break;
                }
                for instance in 0..count {
                    selected.push((part_index, instance, part));
                }
            }
            if budget_hit {
                break;
            }
            if selected.is_empty() {
                continue;
            }
            let full: i128 = selected
                .iter()
                .map(|(_, _, p)| i128::from(p.full_advance))
                .sum();
            let maxima: Vec<_> = selected
                .windows(2)
                .map(|w| w[0].2.end_connector.min(w[1].2.start_connector))
                .collect();
            if maxima.iter().any(|&m| m < self.min_connector_overlap()) {
                continue;
            }
            let min_extent = full - maxima.iter().map(|&m| i128::from(m)).sum::<i128>();
            let max_extent = full - i128::from(self.min_connector_overlap()) * maxima.len() as i128;
            if cmp_integer(target, min_extent)?.is_lt() || cmp_integer(target, max_extent)?.is_gt()
            {
                continue;
            }
            let reduction = target.checked_add(integer(-min_extent))?;
            let flexibility = max_extent - min_extent;
            let overlaps: Vec<_> = maxima
                .iter()
                .map(|&max| -> Result<Rational, FitError> {
                    let share = if flexibility == 0 {
                        integer(0)
                    } else {
                        reduction.checked_mul(Rational::new(
                            i128::from(max - self.min_connector_overlap()),
                            flexibility,
                        )?)?
                    };
                    Ok(integer(max.into()).checked_add(neg(share)?)?)
                })
                .collect::<Result<_, _>>()?;
            let mut position = integer(0);
            let mut parts = Vec::with_capacity(selected.len());
            for (i, &(part_index, instance, part)) in selected.iter().enumerate() {
                parts.push(FittedPart {
                    glyph_id: part.glyph_id,
                    part_index,
                    instance,
                    offset: position,
                });
                position = position.checked_add(integer(part.full_advance.into()))?;
                if let Some(&overlap) = overlaps.get(i) {
                    position = position.checked_add(neg(overlap)?)?;
                }
            }
            if position != target {
                return Err(FitError::Unrepresentable);
            }
            return Ok(wrap(FittedShape::Assembly(FittedAssembly {
                parts,
                overlaps,
                advance: position,
                italic_correction: assembly.italic_correction,
                device_adjustment_present: assembly.device_adjustment_present,
                extender_repetitions: repetitions,
            })));
        }
        if budget_hit || has_extenders {
            Err(FitError::Budget)
        } else {
            Err(FitError::Unrepresentable)
        }
    }
}
