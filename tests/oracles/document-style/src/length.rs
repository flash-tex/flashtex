//! Lengths in TeX points and TeX-style glue.
//!
//! Every dimension in this crate is a TeX point (`pt`, 72.27 per inch) so the
//! numbers match the LaTeX class files exactly. PDF user space uses big points
//! (`bp`, 72 per inch); convert with [`Pt::to_bp`] at the adapter boundary.

use std::fmt;

/// TeX points per inch.
pub const PT_PER_IN: f64 = 72.27;
/// TeX points per PostScript/PDF big point.
pub const PT_PER_BP: f64 = 72.27 / 72.0;
/// TeX points per millimetre.
pub const PT_PER_MM: f64 = 72.27 / 25.4;
/// TeX points per centimetre.
pub const PT_PER_CM: f64 = PT_PER_MM * 10.0;
/// TeX points per pica.
pub const PT_PER_PC: f64 = 12.0;

/// A length in TeX points.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Pt(pub f64);

impl Pt {
    pub const ZERO: Pt = Pt(0.0);

    pub fn inches(v: f64) -> Pt {
        Pt(v * PT_PER_IN)
    }
    pub fn mm(v: f64) -> Pt {
        Pt(v * PT_PER_MM)
    }
    pub fn cm(v: f64) -> Pt {
        Pt(v * PT_PER_CM)
    }
    pub fn bp(v: f64) -> Pt {
        Pt(v * PT_PER_BP)
    }
    /// Value in PDF big points.
    pub fn to_bp(self) -> f64 {
        self.0 / PT_PER_BP
    }
    pub fn abs(self) -> Pt {
        Pt(self.0.abs())
    }
    /// LaTeX `\@settopoint`: `\divide#1\p@ \multiply#1\p@`, which truncates the
    /// length toward zero to a whole number of points.
    pub fn settopoint(self) -> Pt {
        Pt(self.0.trunc())
    }
    /// Parse a LaTeX absolute length such as `1in`, `2cm`, `400pt`, `12bp`,
    /// `3pc`, or `25mm`. Relative units (`em`, `ex`) are rejected because they
    /// depend on the current font; resolve them with [`crate::fonts::FontParams`].
    pub fn parse(text: &str) -> Result<Pt, LengthError> {
        let s = text.trim();
        let split = s
            .char_indices()
            .find(|(_, c)| c.is_ascii_alphabetic())
            .map(|(i, _)| i)
            .ok_or_else(|| LengthError::MissingUnit(s.to_string()))?;
        let (num, unit) = s.split_at(split);
        let value: f64 = num
            .trim()
            .parse()
            .map_err(|_| LengthError::BadNumber(s.to_string()))?;
        let per = match unit.trim() {
            "pt" => 1.0,
            "bp" => PT_PER_BP,
            "in" => PT_PER_IN,
            "cm" => PT_PER_CM,
            "mm" => PT_PER_MM,
            "pc" => PT_PER_PC,
            other => return Err(LengthError::UnknownUnit(other.to_string())),
        };
        Ok(Pt(value * per))
    }
}

impl std::ops::Add for Pt {
    type Output = Pt;
    fn add(self, o: Pt) -> Pt {
        Pt(self.0 + o.0)
    }
}
impl std::ops::Sub for Pt {
    type Output = Pt;
    fn sub(self, o: Pt) -> Pt {
        Pt(self.0 - o.0)
    }
}
impl std::ops::Mul<f64> for Pt {
    type Output = Pt;
    fn mul(self, k: f64) -> Pt {
        Pt(self.0 * k)
    }
}
impl std::ops::Neg for Pt {
    type Output = Pt;
    fn neg(self) -> Pt {
        Pt(-self.0)
    }
}
impl std::ops::AddAssign for Pt {
    fn add_assign(&mut self, o: Pt) {
        self.0 += o.0;
    }
}
impl fmt::Display for Pt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}pt", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LengthError {
    MissingUnit(String),
    BadNumber(String),
    UnknownUnit(String),
}

impl fmt::Display for LengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LengthError::MissingUnit(s) => write!(f, "length `{s}` has no unit"),
            LengthError::BadNumber(s) => write!(f, "length `{s}` has no valid number"),
            LengthError::UnknownUnit(u) => write!(f, "unsupported length unit `{u}`"),
        }
    }
}

impl std::error::Error for LengthError {}

/// TeX glue: a natural length with finite stretch and shrink, all in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Skip {
    pub pt: f64,
    pub plus: f64,
    pub minus: f64,
}

impl Skip {
    pub const ZERO: Skip = Skip {
        pt: 0.0,
        plus: 0.0,
        minus: 0.0,
    };

    pub const fn new(pt: f64, plus: f64, minus: f64) -> Skip {
        Skip { pt, plus, minus }
    }
    pub const fn fixed(pt: f64) -> Skip {
        Skip {
            pt,
            plus: 0.0,
            minus: 0.0,
        }
    }
    /// Scale every component (used for `ex`/`em` multiples).
    pub fn scale(self, k: f64) -> Skip {
        Skip {
            pt: self.pt * k,
            plus: self.plus * k,
            minus: self.minus * k,
        }
    }
    /// Componentwise sum, as TeX adds glue.
    pub fn plus(self, o: Skip) -> Skip {
        Skip {
            pt: self.pt + o.pt,
            plus: self.plus + o.plus,
            minus: self.minus + o.minus,
        }
    }
    /// `\addvspace` semantics: when two skips meet only the larger natural
    /// part survives, together with its stretch and shrink.
    pub fn addvspace(self, o: Skip) -> Skip {
        if o.pt > self.pt { o } else { self }
    }
    pub fn natural(self) -> Pt {
        Pt(self.pt)
    }
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}pt", self.pt)?;
        if self.plus != 0.0 {
            write!(f, " plus {}pt", self.plus)?;
        }
        if self.minus != 0.0 {
            write!(f, " minus {}pt", self.minus)?;
        }
        Ok(())
    }
}
