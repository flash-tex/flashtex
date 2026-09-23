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

/// TeX page/paragraph lengths the host supplies, in TeX points.
///
/// TikZ coordinates such as `(0,.6\baselineskip)` scale the current register
/// value (`0.6 * \baselineskip`), and a bare `\linewidth` means one times the
/// register. The host (the render pipeline) passes its live values; when it
/// has none, [`TexLengths::default`] applies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TexLengths {
    pub baselineskip_pt: f64,
    pub linewidth_pt: f64,
    pub textwidth_pt: f64,
    pub parindent_pt: f64,
}

impl TexLengths {
    /// Looks up `\baselineskip`, `\linewidth`, `\textwidth` or `\parindent`.
    /// Anything else (e.g. `\parskip`) is unknown and stays an error so the
    /// caller keeps warning on it.
    fn register(&self, name: &str) -> Option<f64> {
        Some(match name {
            "baselineskip" => self.baselineskip_pt,
            "linewidth" => self.linewidth_pt,
            "textwidth" => self.textwidth_pt,
            "parindent" => self.parindent_pt,
            _ => return None,
        })
    }
}

impl Default for TexLengths {
    /// Fixed defaults from a 10pt `article` document, confirmed with
    /// `pdflatex -interaction=nonstopmode` + `\typeout{\the...}`:
    /// `\baselineskip=12.0pt`, `\textwidth=\linewidth=345.0pt`,
    /// `\parindent=15.0pt`.
    fn default() -> Self {
        TexLengths {
            baselineskip_pt: 12.0,
            linewidth_pt: 345.0,
            textwidth_pt: 345.0,
            parindent_pt: 15.0,
        }
    }
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
    tex: TexLengths,
}

pub fn eval(text: &str, em: f64) -> Result<Value, String> {
    eval_with(text, em, &TexLengths::default())
}

/// Same as [`eval`], but scales `\baselineskip`, `\linewidth`, `\textwidth`
/// and `\parindent` by the host-supplied `tex` values instead of the
/// [`TexLengths::default`] fallbacks.
pub fn eval_with(text: &str, em: f64, tex: &TexLengths) -> Result<Value, String> {
    let mut p = P {
        s: text.as_bytes(),
        i: 0,
        em,
        tex: *tex,
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

/// Same as [`length_pt`], with host-supplied [`TexLengths`].
pub fn length_pt_with(text: &str, em: f64, tex: &TexLengths) -> Result<f64, String> {
    eval_with(text, em, tex).map(|v| v.v)
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
            Some(b'\\') => {
                // A bare TeX length register: `\linewidth` is one times
                // the register. Unknown control sequences stay errors so
                // the caller keeps warning on them.
                self.i += 1;
                let name = self.control_word();
                match self.tex.register(&name) {
                    Some(f) => Ok(Value { v: f, dim: true }),
                    None => Err(format!("unknown TeX length `\\{name}`")),
                }
            }
            Some(c) => Err(format!("unexpected `{}` in expression", c as char)),
        }
    }

    /// Scans a TeX control word (maximal letter run) after a `\`.
    fn control_word(&mut self) -> String {
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_string()
    }

    fn unit_suffix(&mut self, v: Value) -> Result<Value, String> {
        let save = self.i;
        self.ws();
        // A scaled register: `.6\baselineskip` is 0.6 times the register.
        if self.s.get(self.i) == Some(&b'\\') {
            self.i += 1;
            let name = self.control_word();
            return match self.tex.register(&name) {
                Some(f) => Ok(Value { v: v.v * f, dim: true }),
                None => Err(format!("unknown TeX length `\\{name}`")),
            };
        }
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
    fn tex_length_registers() {
        // Defaults are the 10pt article values pdflatex reports
        // (`\typeout{\the\baselineskip}` = 12.0pt): `.6\baselineskip` = 7.2pt.
        let v = eval(".6\\baselineskip", 10.0).unwrap();
        assert!(v.dim && (v.v - 7.2).abs() < 1e-9, "{v:?}");
        // A host-supplied \linewidth is honoured by eval_with.
        let host = TexLengths { linewidth_pt: 200.0, ..TexLengths::default() };
        let v = eval_with("0.5\\linewidth", 10.0, &host).unwrap();
        assert!(v.dim && (v.v - 100.0).abs() < 1e-9, "{v:?}");
        // A bare register is one times its value and composes in arithmetic.
        let v = eval_with("\\textwidth", 10.0, &host).unwrap();
        assert!(v.dim && (v.v - 345.0).abs() < 1e-9, "{v:?}");
        assert!((eval("\\parindent + 2pt", 10.0).unwrap().v - 17.0).abs() < 1e-9);
        assert!((length_pt_with("2\\parindent", 10.0, &host).unwrap() - 30.0).abs() < 1e-9);
        // Unknown registers stay errors (the TikZ layer warns on them).
        assert!(eval("\\parskip", 10.0).is_err());
        assert!(eval("2\\foo", 10.0).is_err());
    }
}
