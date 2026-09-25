//! A small pgfmath-like evaluator: numbers with TeX units, `+ - * / ^`,
//! parentheses, and the common functions (trigonometry in degrees).
//!
//! Every value remembers whether it carried a unit. A dimensioned value is
//! in TeX points; a plain number is left to the caller (TikZ multiplies
//! plain coordinates by the `x`/`y` vectors, i.e. centimetres by default).

/// TeX points per centimetre.
pub const PT_PER_CM: f64 = 72.27 / 2.54;
/// Big points (PDF points) per TeX point.
pub const BP_PER_PT: f64 = 72.0 / 72.27;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    pub v: f64,
    /// `true` when the value is a length in TeX points.
    pub dim: bool,
}

/// Units known to the evaluator, in TeX points. `em`/`ex` depend on the
/// current font size and are passed in.
fn unit_pt(unit: &str, em: f64) -> Option<f64> {
    Some(match unit {
        "pt" => 1.0,
        "cm" => PT_PER_CM,
        "mm" => PT_PER_CM / 10.0,
        "in" => 72.27,
        "bp" => 72.27 / 72.0,
        "pc" => 12.0,
        "sp" => 1.0 / 65536.0,
        "dd" => 1238.0 / 1157.0,
        "cc" => 12.0 * 1238.0 / 1157.0,
        "em" => em,
        // Latin Modern / Computer Modern x-height is 0.430555 em.
        "ex" => em * 0.430_555,
        _ => return None,
    })
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
    em: f64,
    var: Option<(&'a str, f64)>,
}

pub fn eval(text: &str, em: f64) -> Result<Value, String> {
    eval_inner(text, em, None)
}

/// Like [`eval`], but the identifier `var` evaluates to `val` (used for the
/// plot variable `x` of `\addplot`). Any other bare identifier is still an
/// error, so `exp` never collides with a variable named `x`: identifiers
/// lex whole.
pub fn eval_bound(text: &str, em: f64, var: &str, val: f64) -> Result<Value, String> {
    eval_inner(text, em, Some((var, val)))
}

fn eval_inner(text: &str, em: f64, var: Option<(&str, f64)>) -> Result<Value, String> {
    let mut p = P {
        s: text.as_bytes(),
        i: 0,
        em,
        var,
    };
    let v = p.expr()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("cannot evaluate `{}`", text.trim()));
    }
    if !v.v.is_finite() {
        return Err(format!("`{}` is not a finite number", text.trim()));
    }
    Ok(v)
}

/// Evaluates a length: plain numbers are points (TeX's `\pgfmathsetlength`).
pub fn length_pt(text: &str, em: f64) -> Result<f64, String> {
    eval(text, em).map(|v| v.v)
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Result<Value, String> {
        let mut a = self.term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.i += 1;
                    let b = self.term()?;
                    a = Value {
                        v: a.v + b.v,
                        dim: a.dim || b.dim,
                    };
                }
                Some(b'-') => {
                    self.i += 1;
                    let b = self.term()?;
                    a = Value {
                        v: a.v - b.v,
                        dim: a.dim || b.dim,
                    };
                }
                _ => return Ok(a),
            }
        }
    }

    fn term(&mut self) -> Result<Value, String> {
        let mut a = self.power()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.i += 1;
                    let b = self.power()?;
                    a = Value {
                        v: a.v * b.v,
                        dim: a.dim || b.dim,
                    };
                }
                Some(b'/') => {
                    self.i += 1;
                    let b = self.power()?;
                    a = Value {
                        v: a.v / b.v,
                        dim: a.dim != b.dim && a.dim,
                    };
                }
                _ => return Ok(a),
            }
        }
    }

    fn power(&mut self) -> Result<Value, String> {
        let a = self.unary()?;
        if self.peek() == Some(b'^') {
            self.i += 1;
            let b = self.power()?;
            return Ok(Value {
                v: a.v.powf(b.v),
                dim: a.dim,
            });
        }
        Ok(a)
    }

    fn unary(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                let v = self.unary()?;
                Ok(Value { v: -v.v, dim: v.dim })
            }
            Some(b'+') => {
                self.i += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Value, String> {
        match self.peek() {
            None => Err("unexpected end of expression".into()),
            Some(b'(') => {
                self.i += 1;
                let v = self.expr()?;
                if self.peek() != Some(b')') {
                    return Err("missing `)`".into());
                }
                self.i += 1;
                self.unit_suffix(v)
            }
            // TikZ brace groups: `({\a+\c},2)` keeps macro arithmetic
            // safe, so a `{...}` group parses like `(...)`.
            Some(b'{') => {
                self.i += 1;
                let v = self.expr()?;
                if self.peek() != Some(b'}') {
                    return Err("missing `}`".into());
                }
                self.i += 1;
                self.unit_suffix(v)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
                    self.i += 1;
                }
                let lit = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
                let v: f64 = lit.parse().map_err(|_| format!("bad number `{lit}`"))?;
                // Exponent notation `1e3` is rare in TikZ; not supported.
                self.unit_suffix(Value { v, dim: false })
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.i;
                while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
                    self.i += 1;
                }
                let name = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_string();
                match name.as_str() {
                    "pi" => return Ok(Value { v: std::f64::consts::PI, dim: false }),
                    "e" => return Ok(Value { v: std::f64::consts::E, dim: false }),
                    "true" => return Ok(Value { v: 1.0, dim: false }),
                    "false" => return Ok(Value { v: 0.0, dim: false }),
                    _ => {}
                }
                if let Some((var, val)) = self.var
                    && var == name
                {
                    return self.unit_suffix(Value { v: val, dim: false });
                }
                if self.peek() != Some(b'(') {
                    return Err(format!("unknown identifier `{name}`"));
                }
                self.i += 1;
                let mut args = Vec::new();
                if self.peek() != Some(b')') {
                    loop {
                        args.push(self.expr()?);
                        match self.peek() {
                            Some(b',') => self.i += 1,
                            Some(b')') => break,
                            _ => return Err(format!("bad arguments to `{name}`")),
                        }
                    }
                }
                self.i += 1;
                call(&name, &args)
            }
            Some(c) => Err(format!("unexpected `{}` in expression", c as char)),
        }
    }

    fn unit_suffix(&mut self, v: Value) -> Result<Value, String> {
        let save = self.i;
        self.ws();
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
            self.i += 1;
        }
        let unit = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
        if unit.is_empty() {
            self.i = save;
            return Ok(v);
        }
        match unit_pt(unit, self.em) {
            Some(f) => Ok(Value { v: v.v * f, dim: true }),
            None => {
                // A following identifier (e.g. an unsupported function)
                // is left to the caller to reject.
                self.i = save;
                Ok(v)
            }
        }
    }
}

fn call(name: &str, a: &[Value]) -> Result<Value, String> {
    let n = |i: usize| a.get(i).map(|v| v.v).ok_or_else(|| format!("`{name}` needs more arguments"));
    let plain = |v: f64| Ok(Value { v, dim: false });
    let d = std::f64::consts::PI / 180.0;
    match name {
        "sin" => plain((n(0)? * d).sin()),
        "cos" => plain((n(0)? * d).cos()),
        "tan" => plain((n(0)? * d).tan()),
        "asin" => plain(n(0)?.asin() / d),
        "acos" => plain(n(0)?.acos() / d),
        "atan" => plain(n(0)?.atan() / d),
        "atan2" => plain(n(0)?.atan2(n(1)?) / d),
        "deg" => plain(n(0)? / d),
        "rad" => plain(n(0)? * d),
        "sqrt" => Ok(Value { v: n(0)?.sqrt(), dim: a[0].dim }),
        "abs" => Ok(Value { v: n(0)?.abs(), dim: a[0].dim }),
        "exp" => plain(n(0)?.exp()),
        "ln" => plain(n(0)?.ln()),
        "log10" => plain(n(0)?.log10()),
        "int" | "trunc" => plain(n(0)?.trunc()),
        "round" => plain(n(0)?.round()),
        "floor" => plain(n(0)?.floor()),
        "ceil" => plain(n(0)?.ceil()),
        "mod" => plain(n(0)? % n(1)?),
        "min" => Ok(Value { v: n(0)?.min(n(1)?), dim: a[0].dim }),
        "max" => Ok(Value { v: n(0)?.max(n(1)?), dim: a[0].dim }),
        "veclen" => Ok(Value { v: n(0)?.hypot(n(1)?), dim: a[0].dim || a[1].dim }),
        _ => Err(format!("unsupported math function `{name}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_and_arithmetic() {
        assert_eq!(eval("2", 10.0).unwrap(), Value { v: 2.0, dim: false });
        let v = eval("1cm + 2pt", 10.0).unwrap();
        assert!(v.dim && (v.v - (PT_PER_CM + 2.0)).abs() < 1e-9);
        assert!((eval("2*3+4^2/8", 10.0).unwrap().v - 8.0).abs() < 1e-12);
        assert!((eval("sin(30)", 10.0).unwrap().v - 0.5).abs() < 1e-12);
        assert!((eval("-(1+2)*.5", 10.0).unwrap().v + 1.5).abs() < 1e-12);
        assert!((eval("{3+1}", 10.0).unwrap().v - 4.0).abs() < 1e-12);
        assert!((eval("{{2}*3}", 10.0).unwrap().v - 6.0).abs() < 1e-12);
        assert!(eval("{1+2", 10.0).is_err());
        assert!((length_pt(".3333em", 10.0).unwrap() - 3.333).abs() < 1e-9);
        assert!(eval("foo", 10.0).is_err());
    }

    #[test]
    fn bound_variable_resolves_plot_x() {
        assert!((eval_bound("x^2", 10.0, "x", 3.0).unwrap().v - 9.0).abs() < 1e-12);
        assert!((eval_bound("2*x+1", 10.0, "x", 4.0).unwrap().v - 9.0).abs() < 1e-12);
        // Functions still resolve: identifiers lex whole, so `exp` never
        // collides with a variable named `x`.
        assert!((eval_bound("exp(1)", 10.0, "x", 5.0).unwrap().v - std::f64::consts::E).abs() < 1e-12);
        assert!(eval_bound("exp", 10.0, "x", 1.0).is_err());
        assert!(eval_bound("y", 10.0, "x", 1.0).is_err());
        assert!(eval("x", 10.0).is_err());
    }
}
