//! Process-local cache borrowing one immutable font. Caller identity must match
//! in full; no generation switching or persistent cross-build cache reuse.
use crate::{
    cff::Rational,
    math_adapter::{BoundMathFont, MathIdentity},
    math_device::*,
    math_fit::*,
    math_kern::{Corner, MathKern},
    math_variants::{Direction, MathVariants},
};
use std::{collections::VecDeque, mem::size_of, sync::Arc};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    UnhintedKern {
        glyph_id: u16,
        corner: Corner,
        height: Rational,
    },
    Constant {
        record: ConstantDeviceRecord,
        context: DeviceContext,
    },
    Glyph {
        glyph_id: u16,
        kind: GlyphDeviceKind,
        context: DeviceContext,
    },
    Kern {
        glyph_id: u16,
        corner: Corner,
        height: Rational,
        context: KernDeviceContext,
    },
    Fit {
        glyph_id: u16,
        direction: Direction,
        target: Rational,
        strategy: FitStrategy,
        limits: FitLimits,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    UnhintedKern {
        value: crate::math_kern::Value,
        height_device_adjustment_present: bool,
    },
    Constant(ConstantCorrection),
    Glyph {
        design_units: Option<i16>,
        correction: Option<RecordCorrection>,
    },
    Kern(KernCorrection),
    Fit(MathFit),
}
#[derive(Debug)]
pub enum QueryError {
    Device(DeviceError),
    Fit(FitError),
    Resource(crate::Error),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    Hit,
    Stored,
    BypassedOversize,
}
#[derive(Debug)]
pub enum CacheError {
    InvalidLimits,
    StaleIdentity,
}
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub entries: usize,
    pub query_bytes: usize,
    pub parsed_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 128,
            query_bytes: 8 * 1024 * 1024,
            parsed_bytes: 8 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub hits: usize,
    pub computations: usize,
    pub kern_parses: usize,
    pub variant_parses: usize,
    pub query_bytes: usize,
    pub parsed_bytes: usize,
}
pub struct Reply<'a> {
    pub identity: &'a MathIdentity,
    pub status: CacheStatus,
    pub outcome: Arc<Result<Value, QueryError>>,
}
struct Entry {
    query: Query,
    outcome: Arc<Result<Value, QueryError>>,
    bytes: usize,
}
pub struct MathQueryCache<'a> {
    font: &'a BoundMathFont,
    limits: Limits,
    stats: Stats,
    entries: VecDeque<Entry>,
    kern: Option<Arc<Result<MathKern, crate::Error>>>,
    variants: Option<Arc<Result<MathVariants, crate::Error>>>,
}
fn error_bytes(e: &crate::Error) -> usize {
    1024 + format!("{e:?}").len()
}
fn record_bytes(v: &RecordCorrection) -> usize {
    v.device_table_sha256.as_ref().map_or(0, String::capacity)
}
fn value_bytes(v: &Result<Value, QueryError>) -> usize {
    512 + size_of::<Value>()
        + match v {
            Err(e) => format!("{e:?}").len() + 1024,
            Ok(Value::UnhintedKern { .. }) => 0,
            Ok(Value::Constant(c)) => c.device_table_sha256.as_ref().map_or(0, String::capacity),
            Ok(Value::Glyph { correction, .. }) => correction.as_ref().map_or(0, record_bytes),
            Ok(Value::Kern(k)) => record_bytes(&k.correction),
            Ok(Value::Fit(f)) => match &f.shape {
                FittedShape::Variant(_) => 0,
                FittedShape::Assembly(a) => {
                    a.parts.capacity() * size_of::<FittedPart>()
                        + a.overlaps.capacity() * size_of::<Rational>()
                }
            },
        }
}
impl<'a> MathQueryCache<'a> {
    pub fn new(font: &'a BoundMathFont, limits: Limits) -> Result<Self, CacheError> {
        if limits.entries > 4096
            || limits.query_bytes > 64 * 1024 * 1024
            || limits.parsed_bytes > 16 * 1024 * 1024
        {
            return Err(CacheError::InvalidLimits);
        }
        let entries = VecDeque::with_capacity(limits.entries);
        let storage_bytes = entries.capacity() * size_of::<Entry>();
        if storage_bytes > limits.query_bytes {
            return Err(CacheError::InvalidLimits);
        }
        Ok(Self {
            font,
            limits,
            stats: Stats {
                query_bytes: storage_bytes,
                ..Stats::default()
            },
            entries,
            kern: None,
            variants: None,
        })
    }
    pub fn identity(&self) -> &MathIdentity {
        self.font.identity()
    }
    pub fn stats(&self) -> Stats {
        self.stats
    }
    fn kern(&mut self) -> Arc<Result<MathKern, crate::Error>> {
        if let Some(k) = &self.kern {
            return k.clone();
        }
        self.stats.kern_parses += 1;
        let parsed = Arc::new(MathKern::parse(
            self.font.raw_math(),
            self.font.glyph_count(),
        ));
        let bytes = match parsed.as_ref() {
            Err(e) => error_bytes(e),
            Ok(k) => {
                256 + size_of::<MathKern>()
                    + k.records()
                        .values()
                        .map(|t| {
                            256 + size_of::<crate::math_kern::KernTable>()
                                + t.retained_value_bytes()
                        })
                        .sum::<usize>()
            }
        };
        if bytes
            <= self
                .limits
                .parsed_bytes
                .saturating_sub(self.stats.parsed_bytes)
        {
            self.stats.parsed_bytes += bytes;
            self.kern = Some(parsed.clone());
        }
        parsed
    }
    fn variants(&mut self) -> Arc<Result<MathVariants, crate::Error>> {
        if let Some(v) = &self.variants {
            return v.clone();
        }
        self.stats.variant_parses += 1;
        let parsed = Arc::new(MathVariants::parse(
            self.font.raw_math(),
            self.font.glyph_count(),
        ));
        let bytes = match parsed.as_ref() {
            Err(e) => error_bytes(e),
            Ok(v) => {
                256 + size_of::<MathVariants>()
                    + v.constructions()
                        .values()
                        .map(|c| {
                            256 + size_of::<crate::math_variants::Construction>()
                                + c.variants.capacity() * size_of::<crate::math_variants::Variant>()
                                + c.assembly.as_ref().map_or(0, |a| {
                                    a.parts.capacity() * size_of::<crate::math_variants::Part>()
                                })
                        })
                        .sum::<usize>()
            }
        };
        if bytes
            <= self
                .limits
                .parsed_bytes
                .saturating_sub(self.stats.parsed_bytes)
        {
            self.stats.parsed_bytes += bytes;
            self.variants = Some(parsed.clone());
        }
        parsed
    }
    fn compute(&mut self, q: &Query) -> Result<Value, QueryError> {
        match *q {
            Query::UnhintedKern {
                glyph_id,
                corner,
                height,
            } => {
                let parsed = self.kern();
                let kern = parsed
                    .as_ref()
                    .as_ref()
                    .map_err(|e| QueryError::Resource(e.clone()))?;
                let value = kern
                    .lookup(glyph_id, corner, height)
                    .map_err(QueryError::Resource)?;
                let height_device_adjustment_present =
                    kern.records().get(&(glyph_id, corner)).is_some_and(|t| {
                        t.correction_heights()
                            .iter()
                            .any(|v| v.device_adjustment_present)
                    });
                Ok(Value::UnhintedKern {
                    value,
                    height_device_adjustment_present,
                })
            }
            Query::Constant { record, context } => self
                .font
                .constant_device(record, context)
                .map(|r| Value::Constant(r.correction().clone()))
                .map_err(QueryError::Device),
            Query::Glyph {
                glyph_id,
                kind,
                context,
            } => self
                .font
                .glyph_device(glyph_id, kind, context)
                .map(|r| Value::Glyph {
                    design_units: r.design_units(),
                    correction: r.correction().cloned(),
                })
                .map_err(QueryError::Device),
            Query::Kern {
                glyph_id,
                corner,
                height,
                context,
            } => self
                .kern()
                .as_ref()
                .as_ref()
                .map_err(|e| QueryError::Resource(e.clone()))?
                .device_lookup(
                    self.font.raw_math(),
                    glyph_id,
                    corner,
                    height,
                    self.font.units_per_em(),
                    context,
                )
                .map(Value::Kern)
                .map_err(QueryError::Device),
            Query::Fit {
                glyph_id,
                direction,
                target,
                strategy,
                limits,
            } => self
                .variants()
                .as_ref()
                .as_ref()
                .map_err(|e| QueryError::Resource(e.clone()))?
                .fit(direction, glyph_id, target, strategy, limits)
                .map(Value::Fit)
                .map_err(QueryError::Fit),
        }
    }
    /// Full borrowed identity forms the immutable namespace; every query key
    /// includes all explicit device/fitting context. External Arc holders are
    /// caller-owned memory, outside this cache's retained byte budget.
    pub fn query(
        &mut self,
        expected: &MathIdentity,
        query: Query,
    ) -> Result<Reply<'a>, CacheError> {
        if expected != self.font.identity() {
            return Err(CacheError::StaleIdentity);
        }
        if let Some(index) = self.entries.iter().position(|e| e.query == query) {
            let entry = self.entries.remove(index).unwrap();
            let outcome = entry.outcome.clone();
            self.entries.push_back(entry);
            self.stats.hits += 1;
            return Ok(Reply {
                identity: self.font.identity(),
                status: CacheStatus::Hit,
                outcome,
            });
        }
        self.stats.computations += 1;
        let outcome = Arc::new(self.compute(&query));
        let bytes = value_bytes(outcome.as_ref()) + size_of::<Entry>();
        if self.limits.entries == 0
            || bytes
                > self
                    .limits
                    .query_bytes
                    .saturating_sub(self.entries.capacity() * size_of::<Entry>())
        {
            return Ok(Reply {
                identity: self.font.identity(),
                status: CacheStatus::BypassedOversize,
                outcome,
            });
        }
        while self.entries.len() >= self.limits.entries
            || bytes
                > self
                    .limits
                    .query_bytes
                    .saturating_sub(self.stats.query_bytes)
        {
            let old = self.entries.pop_front().unwrap();
            self.stats.query_bytes -= old.bytes;
        }
        self.stats.query_bytes += bytes;
        self.entries.push_back(Entry {
            query,
            outcome: outcome.clone(),
            bytes,
        });
        Ok(Reply {
            identity: self.font.identity(),
            status: CacheStatus::Stored,
            outcome,
        })
    }
}
