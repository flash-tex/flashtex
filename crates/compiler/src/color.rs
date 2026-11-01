//! Colour: `color.sty` and `xcolor.sty` definitions, expressions and the exact
//! operator values pdfTeX writes for them.
//!
//! pdfTeX never sees a colour as a number: `pdftex.def` (2025/09/29 v1.2d)
//! pastes the specification tokens of the final model into the page stream
//! (`\color@rgb` writes `#1 #2 #3 rg #1 #2 #3 RG`), and xcolor (2024/09/29
//! v3.02) computes those tokens with TeX dimension arithmetic: decimal
//! points are moved by re-reading printed dimensions (`\lshift`, `\rshift`),
//! quotients are long-divided digit by digit (`\rdivide`), and results are
//! printed with `\strip@pt`. Rounding happens at every re-read, which is why
//! `\colorlet{c}{cmykA!50}` of `cmyk .1,.2,.3,.4` is written
//! `0.05 0.09999 0.15001 0.2 k`. This module replays those macros on TeX's
//! own integer arithmetic (`round_decimals`, `print_scaled`, `xn_over_d`
//! from tex.web), so the values match pdfTeX exactly; the pinned oracle is
//! `tests/color_oracle/operators.tsv` (982 expressions, 9 package setups).
//!
//! Supported: models `rgb`, `cmy`, `cmyk`, `gray`, `RGB`, `HTML`, `Gray`,
//! `hsb`, `Hsb`, `HSB`, `tHsb` (the hsb family converts through `rgb`, as
//! pdfTeX has no hsb operator: xcolor substitutes `hsb` by `rgb` there too)
//! and `named` lookups; target models `natural`, `rgb`, `cmy`, `cmyk`,
//! `gray`; `\definecolor`/`\providecolor`/`\colorlet`/`\definecolorset`;
//! expressions `name`, `.`, `-name` (complement), `a!p`, `a!p!b`, chains;
//! the base, `dvipsnames`, `svgnames` and `x11names` sets. Not supported
//! (typed [`ColorError::Unsupported`], never approximated): the `wave` and
//! `ps` models, colour series (`!!+`, `\definecolorseries`), extended
//! expressions (`rgb:red,1;blue,2`) and functions (`>wheel`).

use std::collections::HashMap;
use std::fmt;

use crate::color_names;

// ------------------------------------------------------------------------
// TeX integer arithmetic (tex.web §§99-105, 448-455).

const UNITY: i64 = 1 << 16;
const MAX_DIMEN: i64 = (1 << 30) - 1;
const INFINITY: i64 = (1 << 31) - 1;

/// A decimal constant as TeX scans it: signs, integer part (capped at
/// `infinity`, "Number too big"), up to 17 fraction digits.
fn decimal(text: &str) -> Option<(bool, i64, Vec<u8>)> {
    let bytes = text.trim().as_bytes();
    let mut i = 0;
    let mut negative = false;
    while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ') {
        if bytes[i] == b'-' {
            negative = !negative;
        }
        i += 1;
    }
    let mut int: i64 = 0;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        int = (int * 10 + i64::from(bytes[i] - b'0')).min(INFINITY);
        any = true;
        i += 1;
    }
    let mut frac = Vec::new();
    if i < bytes.len() && matches!(bytes[i], b'.' | b',') {
        any = true;
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            if frac.len() < 17 {
                frac.push(bytes[i] - b'0');
            }
            i += 1;
        }
    }
    (any && i == bytes.len()).then_some((negative, int, frac))
}

/// `round_decimals` (§102).
fn round_decimals(digits: &[u8]) -> i64 {
    let mut a = 0;
    for &d in digits.iter().rev() {
        a = (a + i64::from(d) * 2 * UNITY) / 10;
    }
    (a + 1) / 2
}

/// `<decimal>pt` in scaled points; malformed text is `None`.
fn scan_pt(text: &str) -> Option<i64> {
    let (negative, int, frac) = decimal(text)?;
    let v = if int >= 16384 { MAX_DIMEN } else { int * UNITY + round_decimals(&frac) };
    Some(if negative { -v } else { v })
}

/// `xn_over_d` (§107): `x*n/d` truncated toward zero.
fn xn_over_d(x: i64, n: i64, d: i64) -> i64 {
    let q = (i128::from(x.abs()) * i128::from(n)) / i128::from(d);
    let q = q as i64;
    if x < 0 { -q } else { q }
}

/// `<decimal><internal dimen>` (§455): `nx_plus_y(int, v, xn_over_d(v, f, unity))`.
fn times(coefficient: &str, v: i64) -> Option<i64> {
    let (negative, int, frac) = decimal(coefficient)?;
    let y = xn_over_d(v, round_decimals(&frac), UNITY);
    let r = i128::from(int) * i128::from(v) + i128::from(y);
    let r = if r.abs() > i128::from(MAX_DIMEN) { MAX_DIMEN } else { r as i64 };
    Some(if negative { -r } else { r })
}

/// `print_scaled` (§103).
fn print_scaled(s: i64) -> String {
    let mut out = String::new();
    let mut s = s;
    if s < 0 {
        out.push('-');
        s = -s;
    }
    out.push_str(&(s / UNITY).to_string());
    out.push('.');
    let mut s = 10 * (s % UNITY) + 5;
    let mut delta = 10;
    loop {
        if delta > UNITY {
            s = s + 0o100000 - 50000;
        }
        out.push(char::from(b'0' + (s / UNITY) as u8));
        s = 10 * (s % UNITY);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

/// `\strip@pt`.
fn strip_pt(s: i64) -> String {
    let t = print_scaled(s);
    match t.strip_suffix(".0") {
        Some(t) => t.to_string(),
        None => t,
    }
}

/// Moves the decimal point of `text` by `places` (positive: right), the
/// textual effect of xcolor's `\lshift@`, `\rshift@@`, `\lshiftnum` and
/// `\llshiftnum`. Signs are kept verbatim.
fn move_point(text: &str, places: i32) -> String {
    let text = text.trim();
    let body_start = text.find(|c| c != '-' && c != '+').unwrap_or(text.len());
    let (signs, body) = text.split_at(body_start);
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    let mut digits: String = format!("{int}{frac}");
    let mut point = int.len() as i32 + places;
    if point < 0 {
        digits = format!("{}{digits}", "0".repeat((-point) as usize));
        point = 0;
    }
    if point as usize > digits.len() {
        digits.push_str(&"0".repeat(point as usize - digits.len()));
    }
    let (a, b) = digits.split_at(point as usize);
    format!("{signs}{a}.{b}")
}

fn shift(v: i64, places: i32) -> i64 {
    scan_pt(&move_point(&print_scaled(v), places)).unwrap_or(0)
}

/// `\lshift`, `\rshift`, `\rrshift`.
fn lshift(v: i64) -> i64 {
    shift(v, 1)
}
fn rshift(v: i64) -> i64 {
    shift(v, -1)
}
fn rrshift(v: i64) -> i64 {
    rshift(rshift(v))
}

/// `\lshiftset\dimen@{#1}` / `\llshiftset`.
fn lshiftset(text: &str) -> Result<i64, ColorError> {
    scan_pt(&move_point(text, 1)).ok_or_else(|| bad(text))
}
fn llshiftset(text: &str) -> Result<i64, ColorError> {
    scan_pt(&move_point(text, 2)).ok_or_else(|| bad(text))
}

fn bad(text: &str) -> ColorError {
    ColorError::BadSpecification(text.to_string())
}

fn pt(text: &str) -> Result<i64, ColorError> {
    scan_pt(text).ok_or_else(|| bad(text))
}

fn mul(coefficient: &str, v: i64) -> Result<i64, ColorError> {
    times(coefficient, v).ok_or_else(|| bad(coefficient))
}

/// `\rdivide\dimen@{#2}`: long division to five digits, rounded by a sixth.
fn rdivide(dimen: i64, divisor: &str) -> Result<i64, ColorError> {
    let b = pt(divisor)?;
    rdivide_sp(dimen, b).map_err(|_| ColorError::BadSpecification(format!("division by {divisor}")))
}

/// `\rdivide` with the divisor already in scaled points: xcolor's hue code
/// divides by computed dimensions (`6\@tempdimb`, `\@tempdimb`) that never
/// pass through text, so they cannot go through the `&str` form exactly.
fn rdivide_sp(dimen: i64, divisor: i64) -> Result<i64, ColorError> {
    let mut a = dimen;
    let mut b = divisor;
    let mut negative = false;
    if a < 0 {
        a = -a;
        negative = !negative;
    }
    if b < 0 {
        b = -b;
        negative = !negative;
    }
    if a < times(".1", MAX_DIMEN).unwrap_or(0) && b < times(".01", MAX_DIMEN).unwrap_or(0) {
        a = lshift(a);
        b = lshift(b);
    }
    if b == 0 {
        return Err(ColorError::BadSpecification("division by 0".into()));
    }
    let mut cnta = a;
    let mut count = cnta / b;
    let mut text = format!("{count}.");
    for step in 0..6 {
        count *= b;
        cnta -= count;
        cnta *= 10;
        count = cnta / b;
        if step < 5 {
            text.push_str(&count.to_string());
        }
    }
    let mut d = pt(&text)?;
    if count > 4 {
        d += 1;
    }
    Ok(if negative { -d } else { d })
}

// ------------------------------------------------------------------------
// xcolor's per-component calculations (`\XC@calc@...`).

/// `\XC@calcN`: clamp to [0,1], five digits (truncated), trailing zeros off.
fn calc_n(text: &str) -> Result<String, ColorError> {
    let text = text.trim();
    decimal(text).ok_or_else(|| bad(text))?;
    let (int, frac) = text.split_once('.').unwrap_or((text, "0"));
    let int_val = |suffix: &str| decimal(&format!("{int}{suffix}")).map(|(n, v, _)| if n { -v } else { v });
    let (head, digits) = if int_val("0").unwrap_or(0) > 0 {
        ("1", "00000".to_string())
    } else if int_val("1").unwrap_or(0) < 0 {
        ("0", "00000".to_string())
    } else {
        let mut d: String = frac.chars().take(5).collect();
        while d.len() < 5 {
            d.push('0');
        }
        ("0", d)
    };
    let padded = format!("{digits}00000");
    let kept = &padded[..padded.find("00000").unwrap_or(5)];
    Ok(if kept.is_empty() { head.to_string() } else { format!("{head}.{kept}") })
}

/// `\XC@calcC`: `1 - x`, normalised.
fn calc_c(text: &str) -> Result<String, ColorError> {
    let d = llshiftset(&format!("-{}", text.trim()))? + 100 * UNITY;
    calc_n(&strip_pt(rrshift(d)))
}

/// `\XC@c@lcD`: `x / scale` by `\rdivide`.
fn calc_d(text: &str, scale: &str) -> Result<String, ColorError> {
    Ok(strip_pt(rdivide(pt(text)?, scale)?))
}

/// `\XC@calcM`: `round(x * scale)`.
fn calc_m(text: &str, scale: &str) -> Result<String, ColorError> {
    let d = mul(scale, pt(text)?)? + UNITY / 2;
    let int = print_scaled(d);
    let int = int.split('.').next().unwrap_or("0");
    Ok(decimal(int).map(|(n, v, _)| if n { -v } else { v }).unwrap_or(0).to_string())
}

/// `\XC@c@lcS`: `x * scale`.
fn calc_s(text: &str, scale: &str) -> Result<String, ColorError> {
    let d = lshiftset(text)?;
    let d = if pt(scale)? < 100 * UNITY { rrshift(mul(&move_point(scale, 1), d)?) } else { rshift(mul(scale, d)?) };
    Ok(strip_pt(d))
}

/// `\XC@calcT`: `min(1, max(0, x + arg))`.
fn calc_t(text: &str, arg: &str) -> Result<String, ColorError> {
    let d = rshift(lshiftset(text)? + pt(&move_point(arg, 1))?);
    Ok(if d > UNITY {
        "1".into()
    } else if d < 0 {
        "0".into()
    } else {
        strip_pt(d)
    })
}

/// `\XC@add@`.
fn add(a: &str, b: &str) -> Result<String, ColorError> {
    Ok(strip_pt(rrshift(llshiftset(a)? + llshiftset(b)?)))
}

fn each(values: &[String], f: impl Fn(&str) -> Result<String, ColorError>) -> Result<Vec<String>, ColorError> {
    values.iter().map(|v| f(v)).collect()
}

/// `\XC@cnv@rgb@gray`: `(30r + 59g + 11b) / 100`.
fn rgb_gray(c: &[String]) -> Result<String, ColorError> {
    let mut t = mul("30", llshiftset(&c[0])?)?;
    t += mul("59", llshiftset(&c[1])?)?;
    t += mul("11", llshiftset(&c[2])?)?;
    Ok(strip_pt(rdivide(rrshift(t), "100")?))
}

/// `\XC@cnv@cmy@cmyk` with `\adjustUCRBG` = `1,1,1,1`.
fn cmy_cmyk(c: &[String]) -> Result<Vec<String>, ColorError> {
    let less = |a: &str, b: &str| -> Result<bool, ColorError> { Ok(pt(a)? < pt(b)?) };
    let k = if less(&c[0], &c[1])? {
        if less(&c[0], &c[2])? { &c[0] } else { &c[2] }
    } else if less(&c[1], &c[2])? {
        &c[1]
    } else {
        &c[2]
    };
    let ucr = calc_s("1", k)?;
    let bg = calc_s("1", k)?;
    let mut out = Vec::with_capacity(4);
    for v in &c[..3] {
        out.push(calc_n(&add(v, &format!("-{ucr}"))?)?);
    }
    out.push(calc_n(&bg)?);
    Ok(out)
}

/// `\XC@cnv@cmyk@cmy`: `min(1, x + k)`.
fn cmyk_cmy(c: &[String]) -> Result<Vec<String>, ColorError> {
    each(&c[..3], |v| calc_t(v, &c[3]))
}

/// `\rmultiply\dimen@{coefficient}`: the dimension times the decimal, digit
/// by digit (`\@rmultiply` runs Horner over every fraction digit, the
/// integer part is exact, then one `\rshift`).
fn rmultiply(dimen: i64, coefficient: &str) -> Result<i64, ColorError> {
    let (negative, int, frac) = decimal(coefficient.trim()).ok_or_else(|| bad(coefficient))?;
    let t = shift(dimen.abs(), 1);
    let mut d: i64 = 0;
    for &digit in &frac {
        d = (d + i64::from(digit) * t) / 10;
    }
    d += int * t;
    d = rshift(d);
    Ok(if negative != (dimen < 0) { -d } else { d })
}

/// `\XC@cnv@hsb@rgb`: the sector triple for `count = trunc(6h)`. Hue 1
/// lands in sector 6, the same red as sector 0.
fn hsb_sector(count: i64, f: &str) -> Result<[&str; 3], ColorError> {
    match count {
        0 | 6 => Ok(["0", f, "1"]),
        1 => Ok([f, "0", "1"]),
        2 => Ok(["1", "0", f]),
        3 => Ok(["1", f, "0"]),
        4 => Ok([f, "1", "0"]),
        5 => Ok(["0", "1", f]),
        _ => Err(bad(&format!("hsb hue sector {count}"))),
    }
}

/// `\XC@cnv@hsb@rgb#1,#2,#3`: hue/saturation/brightness fractions to rgb
/// (`1 - s * triple`, scaled by brightness, normalised).
fn hsb_rgb(h: &str, s: &str, b: &str) -> Result<Vec<String>, ColorError> {
    let d = rrshift(mul("6", llshiftset(h)?)?);
    if d < 0 || d > 6 * UNITY {
        return Err(bad(h));
    }
    let (count, frac) = (d / UNITY, d % UNITY);
    let f = strip_pt(frac);
    let f = if count % 2 == 0 { calc_c(&f)? } else { f };
    let w: Vec<String> = hsb_sector(count, &f)?.iter().map(|x| x.to_string()).collect();
    let w = each(&w, |x| calc_s(x, s))?;
    let w = each(&w, calc_c)?;
    let w = each(&w, |x| calc_s(x, b))?;
    each(&w, calc_n)
}

/// One `\XC@c@v@rgb@hsb{max}{mid}{min}N{sign}` sector: hue is
/// `(N * (max - min) +/- (max - mid)) / (6 * (max - min))`, clamped at 1,
/// saturation `(max - min) / max`, brightness `max`.
fn rgb_hsb_case(max: &str, mid: &str, min: &str, n: i64, neg: bool) -> Result<Vec<String>, ColorError> {
    let value = calc_n(max)?;
    let dmax = llshiftset(max)?;
    let delta = dmax + llshiftset(&format!("-{min}"))?;
    let mut t = llshiftset(&format!("-{}{mid}", if neg { "-" } else { "" }))?;
    t += if neg { -dmax } else { dmax };
    t += n * delta;
    let hue = rdivide_sp(t, mul("6", delta)?)?;
    let hue = if hue > UNITY { "1".into() } else { strip_pt(hue) };
    let sat = strip_pt(rdivide_sp(delta, dmax)?);
    Ok(vec![hue, sat, value])
}

/// `\XC@cnv@rgb@hsb`: rgb fractions to hsb (the six strict-max orderings,
/// `0,0,max` for gray).
fn rgb_hsb(v: &[String]) -> Result<Vec<String>, ColorError> {
    let (r, g, b) = (&v[0], &v[1], &v[2]);
    let mut count = 0;
    if !(pt(g)? > pt(r)?) {
        count += 4;
    }
    if !(pt(b)? > pt(g)?) {
        count += 2;
    }
    if !(pt(r)? > pt(b)?) {
        count += 1;
    }
    let (max, mid, min, n, neg) = match count {
        1 => (b, g, r, 3, false),
        2 => (g, r, b, 1, false),
        3 => (g, b, r, 3, true),
        4 => (r, b, g, 5, false),
        5 => (b, r, g, 5, true),
        6 => (r, g, b, 1, true),
        7 => return Ok(vec!["0".into(), "0".into(), calc_n(b)?]),
        _ => return Err(bad(&v.join(","))),
    };
    rgb_hsb_case(max, mid, min, n, neg)
}

/// `\rangetHsb` as `(tHsb, Hsb)` hue breakpoints with xcolor's `(361,361)`
/// terminator: the Tektronix hue scale against the `Hsb` degree scale.
const THSB_TABLE: &[(i64, i64)] = &[
    (0, 0),
    (60 * UNITY, 30 * UNITY),
    (120 * UNITY, 60 * UNITY),
    (180 * UNITY, 120 * UNITY),
    (210 * UNITY, 180 * UNITY),
    (240 * UNITY, 240 * UNITY),
    (360 * UNITY, 360 * UNITY),
    (361 * UNITY, 361 * UNITY),
];

/// `\XC@cnv@Hsb@` (`to_thsb`) / `\XC@cnv@tHsb@`: piecewise-linear hue
/// interpolation between the breakpoints bracketing the input hue.
fn thsb_walk(hue: i64, to_thsb: bool) -> Result<i64, ColorError> {
    let mut d = hue;
    for i in 0..THSB_TABLE.len() - 1 {
        let (t1, h1) = THSB_TABLE[i];
        let (t2, h2) = THSB_TABLE[i + 1];
        let (hi, num, den, base, origin) =
            if to_thsb { (h2, t2 - t1, h2 - h1, h1, t1) } else { (t2, h2 - h1, t2 - t1, t1, h1) };
        if d < hi {
            let ratio = rdivide_sp(num, den)?;
            d = rmultiply(d - base, &strip_pt(ratio))? + origin;
            break;
        }
    }
    Ok(d)
}

/// `\XC@cnv@tHsb` towards `Hsb`: the table maps the hue, s/b pass through.
fn thsb_hsb(v: &[String]) -> Result<Vec<String>, ColorError> {
    Ok(vec![strip_pt(thsb_walk(pt(&v[0])?, false)?), v[1].clone(), v[2].clone()])
}

/// `\XC@cnv@Hsb` towards `tHsb`.
fn hsb_thsb(v: &[String]) -> Result<Vec<String>, ColorError> {
    Ok(vec![strip_pt(thsb_walk(pt(&v[0])?, true)?), v[1].clone(), v[2].clone()])
}

/// `\XC@cnv@HTML`: six hexadecimal digits to `0..255` integers.
fn html_numbers(text: &str) -> Result<Vec<String>, ColorError> {
    let t = text.trim();
    if t.len() != 6 || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad(text));
    }
    Ok((0..3).map(|i| u8::from_str_radix(&t[2 * i..2 * i + 2], 16).unwrap_or(0).to_string()).collect())
}

fn hex2(n: &str) -> String {
    let v: u32 = n.parse().unwrap_or(0);
    format!("{:X}{:X}", (v / 16) % 16, v % 16)
}

// ------------------------------------------------------------------------
// Models and specifications.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Model {
    Rgb,
    Cmy,
    Cmyk,
    Gray,
    /// `RGB` (0..`\rangeRGB` = 255).
    RgbInt,
    Html,
    /// `Gray` (0..`\rangeGray` = 15).
    GrayInt,
    /// `hsb` (hue/saturation/brightness fractions, the core model).
    Hsb,
    /// `Hsb` (hue 0..`\rangeHsb` = 360, s/b fractions).
    HsbDeg,
    /// `HSB` (0..`\rangeHSB` = 240).
    HsbInt,
    /// `tHsb` (Tektronix hue mapping, s/b fractions).
    THsb,
    Named,
}

fn model(name: &str) -> Result<Model, ColorError> {
    Ok(match name.trim() {
        "rgb" => Model::Rgb,
        "cmy" => Model::Cmy,
        "cmyk" => Model::Cmyk,
        "gray" => Model::Gray,
        "RGB" => Model::RgbInt,
        "HTML" => Model::Html,
        "Gray" => Model::GrayInt,
        "hsb" => Model::Hsb,
        "Hsb" => Model::HsbDeg,
        "HSB" => Model::HsbInt,
        "tHsb" => Model::THsb,
        "named" => Model::Named,
        m @ ("wave" | "ps") => {
            return Err(ColorError::Unsupported(format!("colour model `{m}`")));
        }
        m => return Err(ColorError::UndefinedModel(m.to_string())),
    })
}

fn arity(m: Model) -> usize {
    match m {
        Model::Cmyk => 4,
        Model::Gray | Model::GrayInt | Model::Html | Model::Named => 1,
        _ => 3,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Spec {
    model: Model,
    values: Vec<String>,
}

impl Spec {
    fn new(model: Model, text: &str) -> Result<Spec, ColorError> {
        let values: Vec<String> = text.split(',').map(|v| v.trim().to_string()).collect();
        if values.len() != arity(model) || values.iter().any(|v| v.is_empty()) {
            return Err(bad(text));
        }
        if !matches!(model, Model::Html | Model::Named) {
            for v in &values {
                decimal(v).ok_or_else(|| bad(v))?;
            }
        }
        Ok(Spec { model, values })
    }
}

/// `\convertcolorspec{from}{spec}{to}`.
fn convert(spec: &Spec, to: Model) -> Result<Spec, ColorError> {
    let from = spec.model;
    if from == to {
        return Ok(spec.clone());
    }
    let v = &spec.values;
    let done = |model, values| Ok(Spec { model, values });
    let unsupported = || Err(ColorError::Unsupported(format!("conversion {from:?} -> {to:?}")));
    match from {
        Model::Rgb => match to {
            Model::Cmy => done(to, each(v, calc_c)?),
            Model::Cmyk => done(to, cmy_cmyk(&each(v, calc_c)?)?),
            Model::RgbInt => done(to, each(v, |x| calc_m(x, "255"))?),
            Model::Html => done(to, vec![each(v, |x| calc_m(x, "255"))?.iter().map(|n| hex2(n)).collect()]),
            Model::Gray => done(to, vec![rgb_gray(v)?]),
            Model::GrayInt => convert(&Spec { model: Model::Gray, values: vec![rgb_gray(v)?] }, to),
            Model::Hsb => done(to, rgb_hsb(v)?),
            Model::HsbDeg | Model::HsbInt | Model::THsb => {
                convert(&Spec { model: Model::Hsb, values: rgb_hsb(v)? }, to)
            }
            _ => unsupported(),
        },
        Model::Cmy => match to {
            Model::Cmyk => done(to, cmy_cmyk(v)?),
            Model::Gray => done(to, vec![calc_c(&rgb_gray(v)?)?]),
            Model::GrayInt => convert(&Spec { model: Model::Gray, values: vec![calc_c(&rgb_gray(v)?)?] }, to),
            _ => convert(&Spec { model: Model::Rgb, values: each(v, calc_c)? }, to),
        },
        Model::Cmyk => match to {
            Model::Gray | Model::GrayInt => {
                let g = calc_c(&calc_t(&rgb_gray(v)?, &v[3])?)?;
                convert(&Spec { model: Model::Gray, values: vec![g] }, to)
            }
            _ => convert(&Spec { model: Model::Cmy, values: cmyk_cmy(v)? }, to),
        },
        Model::Gray => {
            let g = &v[0];
            match to {
                Model::Rgb => {
                    let n = calc_n(g)?;
                    done(to, vec![n.clone(), n.clone(), n])
                }
                Model::Cmy => {
                    let c = calc_c(g)?;
                    done(to, vec![c.clone(), c.clone(), c])
                }
                Model::Cmyk => done(to, vec!["0".into(), "0".into(), "0".into(), calc_c(g)?]),
                Model::RgbInt => {
                    let m = calc_m(g, "255")?;
                    done(to, vec![m.clone(), m.clone(), m])
                }
                Model::Html => done(to, vec![hex2(&calc_m(g, "255")?).repeat(3)]),
                Model::GrayInt => done(to, vec![calc_m(g, "15")?]),
                Model::Hsb => done(to, vec!["0".into(), "0".into(), calc_n(g)?]),
                Model::HsbDeg | Model::HsbInt | Model::THsb => convert(
                    &Spec { model: Model::Hsb, values: vec!["0".into(), "0".into(), calc_n(g)?] },
                    to,
                ),
                _ => unsupported(),
            }
        }
        Model::Hsb => match to {
            Model::Rgb => done(to, hsb_rgb(&v[0], &v[1], &v[2])?),
            Model::HsbDeg => done(to, vec![calc_s(&v[0], "360")?, v[1].clone(), v[2].clone()]),
            Model::HsbInt => done(to, each(v, |x| calc_m(x, "240"))?),
            Model::THsb => convert(
                &Spec {
                    model: Model::HsbDeg,
                    values: vec![calc_s(&v[0], "360")?, v[1].clone(), v[2].clone()],
                },
                to,
            ),
            _ => convert(&Spec { model: Model::Rgb, values: hsb_rgb(&v[0], &v[1], &v[2])? }, to),
        },
        Model::HsbDeg => match to {
            Model::Hsb => done(to, vec![calc_d(&v[0], "360")?, v[1].clone(), v[2].clone()]),
            Model::THsb => done(to, hsb_thsb(v)?),
            _ => convert(
                &Spec { model: Model::Hsb, values: vec![calc_d(&v[0], "360")?, v[1].clone(), v[2].clone()] },
                to,
            ),
        },
        Model::HsbInt => match to {
            Model::Hsb => done(to, each(v, |x| calc_d(x, "240"))?),
            _ => convert(&Spec { model: Model::Hsb, values: each(v, |x| calc_d(x, "240"))? }, to),
        },
        Model::THsb => match to {
            Model::HsbDeg => done(to, thsb_hsb(v)?),
            _ => convert(&Spec { model: Model::HsbDeg, values: thsb_hsb(v)? }, to),
        }
        Model::RgbInt => convert(&Spec { model: Model::Rgb, values: each(v, |x| calc_d(x, "255"))? }, to),
        Model::Html => {
            let rgb = each(&html_numbers(&v[0])?, |x| calc_d(x, "255"))?;
            convert(&Spec { model: Model::Rgb, values: rgb }, to)
        }
        Model::GrayInt => convert(&Spec { model: Model::Gray, values: vec![calc_d(&v[0], "15")?] }, to),
        Model::Named => unsupported(),
    }
}

/// `\XC@coremodel`: `RGB`/`HTML`/`Gray` scale down to their fraction model
/// and `Hsb`/`HSB`/`tHsb` divide down to `hsb`; `hsb` itself has no pdfTeX
/// operator (xcolor substitutes it by `rgb` there), so like the page stream
/// it is stored as `rgb`.
fn core_model(spec: Spec) -> Result<Spec, ColorError> {
    match spec.model {
        Model::RgbInt | Model::Html => convert(&spec, Model::Rgb),
        Model::GrayInt => convert(&spec, Model::Gray),
        Model::HsbDeg | Model::HsbInt | Model::THsb => convert(&spec, Model::Hsb),
        Model::Hsb => convert(&Spec { model: spec.model, values: each(&spec.values, calc_n)? }, Model::Rgb),
        Model::Named => Ok(spec),
        model => Ok(Spec { model, values: each(&spec.values, calc_n)? }),
    }
}

/// `\XC@cnv@<model>@compl` or `\XC@calcC` on every component.
fn complement(spec: &Spec) -> Result<Spec, ColorError> {
    let values = match spec.model {
        Model::Cmyk => cmy_cmyk(&each(&cmyk_cmy(&spec.values)?, calc_c)?)?,
        Model::Rgb | Model::Cmy | Model::Gray => each(&spec.values, calc_c)?,
        _ => return Err(ColorError::Unsupported(format!("complement in {:?}", spec.model))),
    };
    Ok(Spec { model: spec.model, values })
}

/// `\XC@mix`: `p * a + (100 - p) * b`, per component.
fn mix(a: &Spec, b: &[String], percent: i64) -> Result<Spec, ColorError> {
    let other = 100 * UNITY - percent;
    let mut values = Vec::with_capacity(a.values.len());
    for (x, y) in a.values.iter().zip(b) {
        let d = mul(x, percent)? + mul(y, other)?;
        values.push(strip_pt(rrshift(d)));
    }
    Ok(Spec { model: a.model, values })
}

fn white_in(model: Model) -> Option<Vec<String>> {
    let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect();
    match model {
        Model::Rgb => Some(v(&["1", "1", "1"])),
        Model::Cmy => Some(v(&["0", "0", "0"])),
        Model::Cmyk => Some(v(&["0", "0", "0", "0"])),
        Model::Gray => Some(v(&["1"])),
        _ => None,
    }
}

// ------------------------------------------------------------------------
// Device colours (what the page stream receives).

/// The colour space of a PDF colour operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// `g`/`G`.
    Gray,
    /// `rg`/`RG`.
    Rgb,
    /// `k`/`K`.
    Cmyk,
}

/// A colour exactly as pdfTeX writes it: the operator's colour space and
/// its operand values in billionths (pdfTeX's operands are the decimal
/// tokens xcolor computed, at most five digits for computed values; a
/// longer literal `\color[rgb]{0.1234567891}` keeps nine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceColor {
    pub space: ColorSpace,
    values: [u32; 4],
}

const BILLION: u64 = 1_000_000_000;

impl DeviceColor {
    pub const BLACK: DeviceColor = DeviceColor { space: ColorSpace::Gray, values: [0; 4] };
    pub const WHITE: DeviceColor = DeviceColor { space: ColorSpace::Gray, values: [BILLION as u32; 4] };
    /// xcolor `red` (`1 0 0 rg`): beamer's default alert colour (`\alert`).
    pub const RED: DeviceColor = DeviceColor { space: ColorSpace::Rgb, values: [BILLION as u32, 0, 0, 0] };

    /// Operand values in billionths, one per component.
    pub fn billionths(&self) -> &[u32] {
        &self.values[..self.len()]
    }

    fn len(&self) -> usize {
        match self.space {
            ColorSpace::Gray => 1,
            ColorSpace::Rgb => 3,
            ColorSpace::Cmyk => 4,
        }
    }

    /// Component values as `f64` (`0.0..=1.0`).
    pub fn components(&self) -> Vec<f64> {
        self.billionths().iter().map(|v| f64::from(*v) / BILLION as f64).collect()
    }

    /// Operands in shortest decimal form (`0.3 0 0.7`).
    pub fn operands(&self) -> String {
        self.billionths().iter().map(|v| format_billionths(*v)).collect::<Vec<_>>().join(" ")
    }

    /// The fill operator (`0.3 0 0.7 rg`).
    pub fn fill_operator(&self) -> String {
        let op = match self.space {
            ColorSpace::Gray => "g",
            ColorSpace::Rgb => "rg",
            ColorSpace::Cmyk => "k",
        };
        format!("{} {op}", self.operands())
    }

    /// The stroke operator (`0.3 0 0.7 RG`).
    pub fn stroke_operator(&self) -> String {
        let op = match self.space {
            ColorSpace::Gray => "G",
            ColorSpace::Rgb => "RG",
            ColorSpace::Cmyk => "K",
        };
        format!("{} {op}", self.operands())
    }

    /// Naive sRGB preview (gray: `g,g,g`; CMYK: `1 - min(1, c + k)`), the
    /// same documented conversion as `flashtex_vector_graphics::Color::to_rgb`.
    pub fn to_rgb(&self) -> (f64, f64, f64) {
        let c = self.components();
        match self.space {
            ColorSpace::Gray => (c[0], c[0], c[0]),
            ColorSpace::Rgb => (c[0], c[1], c[2]),
            ColorSpace::Cmyk => (
                1.0 - (c[0] + c[3]).min(1.0),
                1.0 - (c[1] + c[3]).min(1.0),
                1.0 - (c[2] + c[3]).min(1.0),
            ),
        }
    }

    /// Builds a colour from operand values in billionths (tests, adapters).
    pub fn from_billionths(space: ColorSpace, values: &[u32]) -> Option<DeviceColor> {
        let mut c = DeviceColor { space, values: [0; 4] };
        if values.len() != c.len() || values.iter().any(|v| u64::from(*v) > BILLION) {
            return None;
        }
        c.values[..values.len()].copy_from_slice(values);
        Some(c)
    }
}

fn format_billionths(v: u32) -> String {
    let int = u64::from(v) / BILLION;
    let frac = u64::from(v) % BILLION;
    if frac == 0 {
        return int.to_string();
    }
    let digits = format!("{frac:09}");
    format!("{int}.{}", digits.trim_end_matches('0'))
}

/// A `pdftex.def` operand token as a value in billionths, after its
/// `\c@lor@arg` range check.
fn operand(text: &str) -> Result<u32, ColorError> {
    let (negative, int, frac) = decimal(text).ok_or_else(|| bad(text))?;
    let mut nine = frac.clone();
    nine.truncate(9);
    while nine.len() < 9 {
        nine.push(0);
    }
    let f = nine.iter().fold(0u64, |a, d| a * 10 + u64::from(*d));
    let v = (int as u64).saturating_mul(BILLION).saturating_add(f);
    // `\c@lor@arg`: negative is `\maxdimen`; above 1pt (after TeX's own
    // rounding of the token) is an error.
    let scaled = scan_pt(text).unwrap_or(MAX_DIMEN);
    if (negative && v > 0) || scaled > UNITY {
        return Err(ColorError::OutOfRange(text.trim().to_string()));
    }
    Ok(v.min(BILLION) as u32)
}

fn device(space: ColorSpace, values: &[String]) -> Result<DeviceColor, ColorError> {
    let mut c = DeviceColor { space, values: [0; 4] };
    for (i, v) in values.iter().enumerate().take(4) {
        c.values[i] = operand(v)?;
    }
    Ok(c)
}

/// `\color@<model>` of `pdftex.def`, with xcolor's model substitutions
/// (`cmy` as `cmyk` with `k = 0`, `HTML` through `rgb`, `Gray` through
/// `gray`, the whole `hsb` family through `rgb`: pdfTeX has no hsb operator).
fn driver(spec: &Spec) -> Result<DeviceColor, ColorError> {
    match spec.model {
        Model::Rgb => device(ColorSpace::Rgb, &spec.values),
        Model::Cmyk => device(ColorSpace::Cmyk, &spec.values),
        Model::Gray => device(ColorSpace::Gray, &spec.values),
        Model::Cmy => {
            let mut v = spec.values.clone();
            v.push("0".into());
            device(ColorSpace::Cmyk, &v)
        }
        // `\c@lor@@RGB`: `\dimen@#1\p@ \divide\dimen@\@cclv`.
        Model::RgbInt => {
            let v = spec.values.iter().map(|x| Ok(strip_pt(pt(x)? / 255))).collect::<Result<Vec<_>, ColorError>>()?;
            device(ColorSpace::Rgb, &v)
        }
        Model::Html => driver(&convert(spec, Model::Rgb)?),
        Model::GrayInt => driver(&convert(spec, Model::Gray)?),
        Model::Hsb => driver(&convert(spec, Model::Rgb)?),
        Model::HsbDeg | Model::HsbInt | Model::THsb => driver(&convert(spec, Model::Hsb)?),
        Model::Named => Err(ColorError::Unsupported("named colour without a definition".into())),
    }
}

// ------------------------------------------------------------------------
// Package state.

#[derive(Debug, Clone, PartialEq)]
pub enum ColorError {
    /// `Undefined color `name'`.
    UndefinedColor(String),
    /// `Undefined color model `name'`.
    UndefinedModel(String),
    /// A specification that does not fit its model.
    BadSpecification(String),
    /// `Argument `x' not in range [0,1]`.
    OutOfRange(String),
    /// Valid xcolor this implementation does not replay.
    Unsupported(String),
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::UndefinedColor(n) => write!(f, "undefined colour `{n}`"),
            ColorError::UndefinedModel(m) => write!(f, "undefined colour model `{m}`"),
            ColorError::BadSpecification(s) => write!(f, "invalid colour specification `{s}`"),
            ColorError::OutOfRange(s) => write!(f, "colour argument `{s}` not in range [0,1]"),
            ColorError::Unsupported(s) => write!(f, "{s} is not supported"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Defined {
    /// An xcolor definition: `\xcolor@{class}{driver}{model}{spec}`.
    Xcolor(Spec),
    /// A color.sty definition: only the driver tokens.
    Driver(DeviceColor),
}

/// The colour package loaded and the colours defined so far.
#[derive(Debug, Clone, PartialEq)]
pub struct Colors {
    xcolor: bool,
    /// `None` is xcolor's `natural`.
    target: Option<Model>,
    defined: HashMap<String, Defined>,
    /// color.sty's `named` model (`\DefineNamedColor`, `dvipsnames`).
    named: HashMap<String, DeviceColor>,
    /// The specification that produced each device colour handed out, so
    /// `.` (the current colour) is re-read in its own model.
    specs: HashMap<DeviceColor, Spec>,
}

/// `\definecolorset{rgb/hsb/cmyk/gray}...` and friends at the end of
/// xcolor.sty (v3.02, lines 1427-1450).
const BASE_SETS: &[(&str, &str)] = &[
    (
        "rgb/hsb/cmyk/gray",
        "red,1,0,0/0,1,1/0,1,1,0/.3;green,0,1,0/.33333,1,1/1,0,1,0/.59;blue,0,0,1/.66667,1,1/1,1,0,0/.11;\
         brown,.75,.5,.25/.083333,.66667,.75/0,.25,.5,.25/.5475;lime,.75,1,0/.20833,1,1/.25,0,1,0/.815;\
         orange,1,.5,0/.083333,1,1/0,.5,1,0/.595;pink,1,.75,.75/0,.25,1/0,.25,.25,0/.825;\
         purple,.75,0,.25/.94444,1,.75/0,.75,.5,.25/.2525;teal,0,.5,.5/.5,1,.5/.5,0,0,.5/.35;\
         violet,.5,0,.5/.83333,1,.5/0,.5,0,.5/.205",
    ),
    (
        "cmyk/rgb/hsb/gray",
        "cyan,1,0,0,0/0,1,1/.5,1,1/.7;magenta,0,1,0,0/1,0,1/.83333,1,1/.41;\
         yellow,0,0,1,0/1,1,0/.16667,1,1/.89;olive,0,0,1,.5/.5,.5,0/.16667,1,.5/.39",
    ),
    (
        "gray/rgb/hsb/cmyk",
        "black,0/0,0,0/0,0,0/0,0,0,1;darkgray,.25/.25,.25,.25/0,0,.25/0,0,0,.75;\
         gray,.5/.5,.5,.5/0,0,.5/0,0,0,.5;lightgray,.75/.75,.75,.75/0,0,.75/0,0,0,.25;\
         white,1/1,1,1/0,0,1/0,0,0,0",
    ),
];

/// color.sty's own definitions (color.sty `\definecolor{black}{gray}{0}`...).
const COLOR_STY: &[(&str, ColorSpace, &[u32])] = &[
    ("black", ColorSpace::Gray, &[0]),
    ("white", ColorSpace::Gray, &[1_000_000_000]),
    ("red", ColorSpace::Rgb, &[1_000_000_000, 0, 0]),
    ("green", ColorSpace::Rgb, &[0, 1_000_000_000, 0]),
    ("blue", ColorSpace::Rgb, &[0, 0, 1_000_000_000]),
    ("cyan", ColorSpace::Cmyk, &[1_000_000_000, 0, 0, 0]),
    ("magenta", ColorSpace::Cmyk, &[0, 1_000_000_000, 0, 0]),
    ("yellow", ColorSpace::Cmyk, &[0, 0, 1_000_000_000, 0]),
];

impl Colors {
    /// `\usepackage[options]{color}`. Returns the options that are not
    /// replayed (drivers and `usenames` are accepted silently).
    pub fn color_sty(options: &str) -> (Colors, Vec<String>) {
        let mut c = Colors {
            xcolor: false,
            target: None,
            defined: HashMap::new(),
            named: HashMap::new(),
            specs: HashMap::new(),
        };
        for (name, space, values) in COLOR_STY {
            let d = DeviceColor::from_billionths(*space, values).unwrap_or(DeviceColor::BLACK);
            c.defined.insert(name.to_string(), Defined::Driver(d));
        }
        let mut unsupported = Vec::new();
        let mut usenames = false;
        for option in split_options(options) {
            match option {
                "dvipsnames" => {
                    for (name, spec) in color_names::DVIPS {
                        if let Ok(d) = driver(&Spec { model: Model::Cmyk, values: split_values(spec) }) {
                            c.named.insert(name.to_string(), d);
                        }
                    }
                }
                "usenames" => usenames = true,
                "pdftex" => {}
                other => unsupported.push(other.to_string()),
            }
        }
        if usenames {
            for (name, d) in c.named.clone() {
                c.defined.insert(name, Defined::Driver(d));
            }
        }
        (c, unsupported)
    }

    /// `\usepackage[options]{xcolor}`, optionally after color.sty (whose
    /// definitions stay usable by name).
    pub fn xcolor(options: &str, previous: Option<&Colors>) -> (Colors, Vec<String>) {
        let mut c = Colors {
            xcolor: true,
            target: None,
            defined: previous.map(|p| p.defined.clone()).unwrap_or_default(),
            named: previous.map(|p| p.named.clone()).unwrap_or_default(),
            specs: HashMap::new(),
        };
        let mut unsupported = Vec::new();
        let mut sets = Vec::new();
        for option in split_options(options) {
            match option {
                "natural" => c.target = None,
                "rgb" => c.target = Some(Model::Rgb),
                "cmy" => c.target = Some(Model::Cmy),
                "cmyk" => c.target = Some(Model::Cmyk),
                "gray" => c.target = Some(Model::Gray),
                "dvipsnames" | "dvipsnames*" | "svgnames" | "svgnames*" | "x11names" | "x11names*" => {
                    sets.push(option.trim_end_matches('*'))
                }
                // Accepted without an effect on the page: the pdfTeX driver,
                // obsolete options, error display, and `table` (colortbl is
                // reported where `\rowcolor`/`\cellcolor` are used).
                "pdftex" | "usenames" | "hyperref" | "fixpdftex" | "showerrors" | "hideerrors" | "table" => {}
                other => unsupported.push(other.to_string()),
            }
        }
        for (models, body) in BASE_SETS {
            let _ = c.define_set("", models, "", "", body);
        }
        for set in sets {
            match set {
                "dvipsnames" => {
                    for (name, spec) in color_names::DVIPS {
                        let _ = c.define("named", name, "cmyk", spec);
                    }
                }
                "svgnames" => {
                    for (name, spec) in color_names::SVG {
                        let _ = c.define("", name, "rgb", spec);
                    }
                }
                _ => {
                    for (name, spec) in color_names::X11 {
                        let _ = c.define("", name, "rgb", spec);
                    }
                }
            }
        }
        (c, unsupported)
    }

    pub fn is_xcolor(&self) -> bool {
        self.xcolor
    }

    /// The document's default colour when it is not pdfTeX's page default
    /// `0 g`: xcolor's `\color{black}` at load converted to a target model
    /// (`\default@color`, written by `\normalcolor` at every shipout).
    pub fn default_color(&self) -> Option<DeviceColor> {
        let target = self.target.filter(|_| self.xcolor)?;
        let black = Spec { model: Model::Gray, values: vec!["0".into()] };
        convert(&black, target).ok().and_then(|s| driver(&s).ok())
    }

    /// `\selectcolormodel{model}`: later definitions and uses convert to it.
    pub fn select_target(&mut self, model_name: &str) -> Result<(), ColorError> {
        self.require_xcolor("\\selectcolormodel")?;
        self.target = match model_name.trim() {
            "natural" => None,
            name => match model(name)? {
                m @ (Model::Rgb | Model::Cmy | Model::Cmyk | Model::Gray) => Some(m),
                _ => return Err(ColorError::Unsupported(format!("target model `{name}`"))),
            },
        };
        Ok(())
    }

    /// Whether `name` is a defined colour.
    pub fn is_defined(&self, name: &str) -> bool {
        self.defined.contains_key(name.trim())
    }

    /// `\definecolor[class]{name}{model}{spec}` (model lists like
    /// `rgb/cmyk` pick the target model's specification).
    pub fn define(&mut self, class: &str, name: &str, models: &str, spec: &str) -> Result<(), ColorError> {
        let name = name.trim().to_string();
        if !self.xcolor {
            let m = model(models)?;
            let d = if m == Model::Named {
                *self.named.get(spec.trim()).ok_or_else(|| ColorError::UndefinedColor(spec.trim().to_string()))?
            } else {
                driver(&Spec::new(m, spec)?)?
            };
            if class.trim() == "named" {
                self.named.insert(name.clone(), d);
            }
            self.defined.insert(name, Defined::Driver(d));
            return Ok(());
        }
        let (m, text) = self.select(models, spec)?;
        if m == Model::Named {
            let target = self.defined.get(text.trim()).cloned().ok_or_else(|| ColorError::UndefinedColor(text))?;
            self.defined.insert(name, target);
            return Ok(());
        }
        let mut s = Spec::new(m, &text)?;
        if let Some(t) = self.target {
            s = convert(&s, t)?;
        }
        let s = core_model(s)?;
        driver(&s)?;
        self.defined.insert(name, Defined::Xcolor(s));
        Ok(())
    }

    /// `\providecolor`: `\definecolor` unless the name is defined.
    pub fn provide(&mut self, class: &str, name: &str, models: &str, spec: &str) -> Result<(), ColorError> {
        if self.is_defined(name) {
            return Ok(());
        }
        self.define(class, name, models, spec)
    }

    /// `\definecolorset[class]{models}{head}{tail}{name,spec;...}`.
    pub fn define_set(&mut self, class: &str, models: &str, head: &str, tail: &str, body: &str) -> Result<(), ColorError> {
        self.require_xcolor("\\definecolorset")?;
        let mut first = Ok(());
        for entry in body.split(';') {
            let entry = entry.trim();
            let Some((name, spec)) = entry.split_once(',') else { continue };
            let r = self.define(class, &format!("{head}{}{tail}", name.trim()), models, spec);
            if first.is_ok() {
                first = r;
            }
        }
        first
    }

    /// `\colorlet[class]{name}[model]{expression}`.
    pub fn colorlet(
        &mut self,
        class: &str,
        name: &str,
        model_name: &str,
        expression: &str,
        current: Option<DeviceColor>,
    ) -> Result<(), ColorError> {
        self.require_xcolor("\\colorlet")?;
        let e = expression.trim();
        let plain = !e.contains(['>', ':', '!']) && !e.starts_with('-') && e != ".";
        if plain && model_name.trim().is_empty() && class.trim().is_empty() {
            let target = self.defined.get(e).cloned().ok_or_else(|| ColorError::UndefinedColor(e.to_string()))?;
            self.defined.insert(name.trim().to_string(), target);
            return Ok(());
        }
        let mut s = self.split(e, current)?;
        if !model_name.trim().is_empty() {
            s = convert(&s, model(model_name)?)?;
        }
        let values = s.values.join(",");
        let m = model_name_of(s.model);
        self.define(class, name, m, &values)
    }

    /// The device colour of `\color{expression}` or `\color[model]{spec}`
    /// with `current` as `.`.
    pub fn resolve(
        &mut self,
        model_name: Option<&str>,
        expression: &str,
        current: Option<DeviceColor>,
    ) -> Result<DeviceColor, ColorError> {
        let spec = self.resolve_spec(model_name, expression, current)?;
        let d = match &spec {
            Err(d) => *d,
            Ok(s) => driver(s)?,
        };
        if let Ok(s) = spec {
            self.specs.insert(d, s);
        }
        Ok(d)
    }

    /// `Ok(spec)` for an xcolor colour, `Err(device)` for driver-only ones.
    fn resolve_spec(
        &mut self,
        model_name: Option<&str>,
        expression: &str,
        current: Option<DeviceColor>,
    ) -> Result<Result<Spec, DeviceColor>, ColorError> {
        let e = expression.trim();
        if let Some(models) = model_name.filter(|m| !m.trim().is_empty()) {
            if !self.xcolor {
                let m = model(models)?;
                if m == Model::Named {
                    return self.named.get(e).map(|d| Err(*d)).ok_or_else(|| ColorError::UndefinedColor(e.to_string()));
                }
                return Ok(Ok(Spec::new(m, e)?));
            }
            let (m, text) = self.select(models, e)?;
            if m == Model::Named {
                return match self.defined.get(text.trim()) {
                    Some(Defined::Xcolor(s)) => Ok(Ok(s.clone())),
                    Some(Defined::Driver(d)) => Ok(Err(*d)),
                    None => Err(ColorError::UndefinedColor(text)),
                };
            }
            let mut s = Spec::new(m, &text)?;
            if let Some(t) = self.target {
                s = convert(&s, t)?;
            }
            return Ok(Ok(s));
        }
        match self.defined.get(e) {
            Some(Defined::Driver(d)) => return Ok(Err(*d)),
            Some(Defined::Xcolor(s)) if self.target.is_none() => return Ok(Ok(s.clone())),
            None if !self.xcolor => return Err(ColorError::UndefinedColor(e.to_string())),
            _ => {}
        }
        let mut s = self.split(e, current)?;
        if let Some(t) = self.target {
            s = convert(&s, t)?;
        }
        Ok(Ok(s))
    }

    fn require_xcolor(&self, what: &str) -> Result<(), ColorError> {
        if self.xcolor { Ok(()) } else { Err(ColorError::Unsupported(format!("{what} without xcolor"))) }
    }

    /// `\XC@getmod`: the model of a `/` list that matches the target (else
    /// the first) and the matching specification.
    fn select(&self, models: &str, spec: &str) -> Result<(Model, String), ColorError> {
        if models.contains(':') {
            return Err(ColorError::Unsupported(format!("colour model `{models}`")));
        }
        let names: Vec<&str> = models.split('/').map(str::trim).collect();
        let specs: Vec<&str> = spec.split('/').collect();
        let mut pos = 0;
        if let Some(t) = self.target {
            for (i, n) in names.iter().enumerate() {
                if model(n).ok() == Some(t) {
                    pos = i;
                    break;
                }
            }
        }
        let m = model(names[pos])?;
        let text = specs.get(pos).ok_or_else(|| bad(spec))?;
        Ok((m, text.trim().to_string()))
    }

    /// The current colour `.` as an xcolor specification.
    fn current_spec(&self, current: Option<DeviceColor>) -> Spec {
        let black = || {
            let s = Spec { model: Model::Gray, values: vec!["0".into()] };
            match self.target {
                Some(t) => convert(&s, t).unwrap_or(s),
                None => s,
            }
        };
        let Some(d) = current else { return black() };
        if let Some(s) = self.specs.get(&d) {
            return s.clone();
        }
        let model = match d.space {
            ColorSpace::Gray => Model::Gray,
            ColorSpace::Rgb => Model::Rgb,
            ColorSpace::Cmyk => Model::Cmyk,
        };
        Spec { model, values: d.operands().split(' ').map(str::to_string).collect() }
    }

    fn lookup(&self, name: &str, current: Option<DeviceColor>) -> Result<Spec, ColorError> {
        if name == "." {
            return Ok(self.current_spec(current));
        }
        match self.defined.get(name) {
            Some(Defined::Xcolor(s)) => Ok(s.clone()),
            Some(Defined::Driver(_)) => {
                Err(ColorError::Unsupported(format!("expression on color.sty colour `{name}`")))
            }
            None => Err(ColorError::UndefinedColor(name.to_string())),
        }
    }

    /// `\XC@split` for `-...-name!p!name!p...`.
    fn split(&self, expression: &str, current: Option<DeviceColor>) -> Result<Spec, ColorError> {
        let e = expression.trim();
        if e.contains(['>', ':']) {
            return Err(ColorError::Unsupported(format!("colour expression `{e}`")));
        }
        let dashes = e.len() - e.trim_start_matches('-').len();
        let body = &e[dashes..];
        let (name, rest) = body.split_once('!').unwrap_or((body, ""));
        let mut spec = self.lookup(name.trim(), current)?;
        let parts: Vec<&str> = if rest.is_empty() { Vec::new() } else { rest.split('!').collect() };
        let mut i = 0;
        while i < parts.len() {
            let pct = parts[i].trim();
            let other = parts.get(i + 1).map(|s| s.trim()).unwrap_or("");
            i += 2;
            if pct.is_empty() {
                if other.is_empty() {
                    break;
                }
                return Err(ColorError::Unsupported(format!("colour series `{e}`")));
            }
            let percent = pt(pct)?;
            let other = if other.is_empty() { "white" } else { other };
            if percent == 100 * UNITY {
                continue;
            }
            if percent == 0 {
                spec = self.split(other, current)?;
                continue;
            }
            let values = match white_in(spec.model).filter(|_| other == "white") {
                Some(w) => w,
                None => {
                    let o = self.split(other, current)?;
                    if spec.model == Model::Gray {
                        spec = convert(&spec, o.model)?;
                        o.values
                    } else {
                        convert(&o, spec.model)?.values
                    }
                }
            };
            spec = mix(&spec, &values, percent)?;
        }
        if dashes % 2 == 1 {
            spec = complement(&spec)?;
        }
        Ok(spec)
    }
}

fn model_name_of(m: Model) -> &'static str {
    match m {
        Model::Rgb => "rgb",
        Model::Cmy => "cmy",
        Model::Cmyk => "cmyk",
        Model::Gray => "gray",
        Model::RgbInt => "RGB",
        Model::Html => "HTML",
        Model::GrayInt => "Gray",
        Model::Hsb => "hsb",
        Model::HsbDeg => "Hsb",
        Model::HsbInt => "HSB",
        Model::THsb => "tHsb",
        Model::Named => "named",
    }
}

fn split_options(options: &str) -> impl Iterator<Item = &str> {
    options.split(',').map(str::trim).filter(|o| !o.is_empty())
}

fn split_values(spec: &str) -> Vec<String> {
    spec.split(',').map(|v| v.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tex_arithmetic_matches_tex() {
        assert_eq!(scan_pt("0.3"), Some(19661));
        assert_eq!(print_scaled(19661), "0.3");
        assert_eq!(print_scaled(65536), "1.0");
        assert_eq!(strip_pt(65536), "1");
        assert_eq!(print_scaled(-32768), "-0.5");
        assert_eq!(move_point("0.5", -1), ".05");
        assert_eq!(move_point("-3.0", -1), "-.30");
        assert_eq!(move_point("0.12345", 2), "012.345");
        assert_eq!(move_point("1", 2), "100.");
        assert_eq!(times("0.5", 3 * UNITY), Some(98304));
    }

    #[test]
    fn xcolor_arithmetic_artefacts() {
        let (mut c, _) = Colors::xcolor("", None);
        c.define("", "mc", "cmyk", ".1,.2,.3,.4").unwrap();
        let d = c.resolve(None, "mc!50", None).unwrap();
        assert_eq!(d.fill_operator(), "0.05 0.09999 0.15001 0.2 k");
        c.define("", "mine", "RGB", "12,200,33").unwrap();
        assert_eq!(c.resolve(None, "mine", None).unwrap().operands(), "0.04706 0.78432 0.12941");
    }

    /// `\definecolor{myorange}{hsb}{0.08, 1, 1}` resolves through the RGB
    /// pipeline (hue 0.08 * 360 = ~29 degrees: a red-orange), and none of
    /// `hsb`/`Hsb`/`HSB`/`tHsb` report `Unsupported`. Pins are TeX Live 2026
    /// xcolor `\convertcolorspec` output (see `hsb_conversions_match_xcolor`).
    #[test]
    fn hsb_orange_resolves_to_rgb() {
        let (mut c, _) = Colors::xcolor("", None);
        c.define("", "myorange", "hsb", "0.08, 1, 1").unwrap();
        assert_eq!(c.resolve(None, "myorange", None).unwrap().fill_operator(), "1 0.48 0 rg");
        assert_eq!(c.resolve(Some("hsb"), "0.08,1,1", None).unwrap().fill_operator(), "1 0.48 0 rg");
        assert_eq!(c.resolve(Some("Hsb"), "28.8,1,1", None).unwrap().fill_operator(), "1 0.48 0 rg");
        assert_eq!(
            c.resolve(Some("HSB"), "19,240,240", None).unwrap().fill_operator(),
            "1 0.47508 0 rg"
        );
        assert_eq!(c.resolve(Some("tHsb"), "30,1,1", None).unwrap().fill_operator(), "1 0.25002 0 rg");
        // Expressions keep working: the hsb definition mixes like its rgb twin.
        c.define("", "rgborange", "rgb", "1,0.48,0").unwrap();
        assert_eq!(
            c.resolve(None, "myorange!50", None).unwrap().fill_operator(),
            c.resolve(None, "rgborange!50", None).unwrap().fill_operator()
        );
    }

    /// Model-to-model conversions against xcolor's documented algorithm
    /// (`\XC@cnv@hsb@rgb`, `\XC@cnv@rgb@hsb`, the `Hsb`/`HSB` scalings and the
    /// `tHsb` hue table), pinned to TeX Live 2026 `\convertcolorspec` output.
    #[test]
    fn hsb_conversions_match_xcolor() {
        let hsb = |v: &[&str]| Spec {
            model: Model::Hsb,
            values: v.iter().map(|x| x.to_string()).collect(),
        };
        let orange = hsb(&["0.08", "1", "1"]);
        assert_eq!(convert(&orange, Model::Rgb).unwrap().values, vec!["1", "0.48", "0"]);
        assert_eq!(convert(&orange, Model::HsbDeg).unwrap().values, vec!["28.80011", "1", "1"]);
        assert_eq!(convert(&orange, Model::HsbInt).unwrap().values, vec!["19", "240", "240"]);
        assert_eq!(convert(&orange, Model::THsb).unwrap().values, vec!["57.60022", "1", "1"]);
        // TeX-arithmetic fingerprints: the sector edge, hue 1, xcolor's brown.
        assert_eq!(
            convert(&hsb(&["0.33333", "1", "1"]), Model::Rgb).unwrap().values,
            vec!["0.00002", "1", "0"]
        );
        assert_eq!(
            convert(&hsb(&["1", "1", "1"]), Model::Rgb).unwrap().values,
            vec!["1", "0", "0"]
        );
        assert_eq!(
            convert(&hsb(&["0.08333", "0.66667", "0.75"]), Model::Rgb).unwrap().values,
            vec!["0.75", "0.5", "0.25"]
        );
        // Back towards hsb: rgb sectors, gray, and the tHsb table both ways.
        let rgb = |v: &[&str]| Spec {
            model: Model::Rgb,
            values: v.iter().map(|x| x.to_string()).collect(),
        };
        assert_eq!(convert(&rgb(&["1", "0.5", "0"]), Model::Hsb).unwrap().values, vec!["0.08333", "1", "1"]);
        assert_eq!(
            convert(&rgb(&["0.2", "0.7", "0.4"]), Model::Hsb).unwrap().values,
            vec!["0.4", "0.7143", "0.7"]
        );
        assert_eq!(convert(&rgb(&["0.5", "0.5", "0.5"]), Model::Hsb).unwrap().values, vec!["0", "0", "0.5"]);
        let gray = Spec { model: Model::Gray, values: vec!["0.35".into()] };
        assert_eq!(convert(&gray, Model::Hsb).unwrap().values, vec!["0", "0", "0.35"]);
        let deg = |h: &str| Spec { model: Model::HsbDeg, values: vec![h.into(), "1".into(), "1".into()] };
        assert_eq!(convert(&deg("45"), Model::THsb).unwrap().values, vec!["90", "1", "1"]);
        let thsb = Spec { model: Model::THsb, values: vec!["90".into(), "1".into(), "1".into()] };
        assert_eq!(convert(&thsb, Model::HsbDeg).unwrap().values, vec!["45", "1", "1"]);
        // `wave` and `ps` stay unsupported.
        assert!(matches!(model("wave"), Err(ColorError::Unsupported(_))));
        assert!(matches!(model("ps"), Err(ColorError::Unsupported(_))));
    }
}
