//! siunitx v3 (TeX Live 2026, `siunitx.sty` 2026 release) number and unit
//! commands: `\num`, `\unit`/`\si`, `\qty`/`\SI`, `\numlist`, `\numrange`,
//! `\qtylist`, `\qtyrange`, `\SIlist`, `\SIrange`, `\ang`, `\sisetup`,
//! `\DeclareSIUnit` and the package options.
//!
//! siunitx is expl3; nothing here ports that code. The documented output is
//! modelled and every form is checked against pdflatex (render-pipeline
//! `tests/siunitx_oracle.rs`, `fixtures/siunitx`). With the v3 default
//! `mode = math` (`siunitx.sty` 5570-5577, `reset-math-version` and the
//! `reset-text-*` keys true) every command sets an inline formula in the
//! upright math fonts whatever the surrounding text face, so the output is
//! an ordinary [`MathList`] and needs no new node type:
//!
//! - numbers: digits, the math `.` (cmmi), `\,` group separators (3mu),
//!   `\times 10^{e}`, a leading `-`/`+` as an Ord, `(12)` uncertainties;
//! - units: each prefixed unit is one upright run (`Nucleus::Text`, the
//!   `\mathrm{kg}` box), `\ohm` is `\Omega`, `\micro` the text `µ`
//!   (`\textmu` in `\text`, `siunitx.sty` 9611-9621), `\degree` and
//!   `\celsius` an empty nucleus with a `\circ` superscript
//!   (`\__siunitx_unit_non_latin:n {"00B0}` falls back to `{}^{\circ}`
//!   under pdfTeX), powers as superscripts, `\per` a `-1` power
//!   (`per-mode = power`, 7820), a `\frac` or a `/`;
//! - quantities: number, `quantity-product` (a `\penalty10000` and a
//!   text-font `\,` kern outside math, 3mu glue inside), unit;
//! - lists and ranges: `\text{, }`, `\text{ and }`, `\text{ to }` between
//!   the items (`list-separator`, `list-final-separator`,
//!   `list-pair-separator`, `range-phrase`, 4913-4954).
//!
//! Option defaults are `siunitx.sty` 2880-2931 (numbers), 4888-4954 (lists),
//! 7820 (`per-mode`). Settings are document-global: `\sisetup` inside a
//! group is not undone at the group's end (a documented approximation).
//! Not modelled (diagnosed, never approximated): `S` table columns,
//! `\complexnum`/`\complexqty`, rounding, `locale`, `mode = text`.

use std::cell::RefCell;

use crate::diagnostics::Diagnostic;
use crate::lexer::{self, Token, TokenKind};
use crate::math::{self, MathAtom, MathList, MathPackages, Nucleus};
use crate::Span;

/// Commands that typeset material, with (required arguments, whether an
/// optional pre-unit bracket follows the first required argument).
pub const TYPESET_COMMANDS: &[(&str, usize, bool)] = &[
    ("num", 1, false),
    ("unit", 1, false),
    ("si", 1, false),
    ("qty", 2, false),
    ("SI", 2, true),
    ("numlist", 1, false),
    ("numrange", 2, false),
    ("qtylist", 2, false),
    ("qtyrange", 3, false),
    ("SIlist", 2, false),
    ("SIrange", 3, false),
    ("ang", 1, false),
];

/// The argument shape of a typesetting command.
pub fn arity(name: &str) -> Option<(usize, bool)> {
    TYPESET_COMMANDS
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|&(_, required, pre)| (required, pre))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroupDigits {
    All,
    None,
    Integer,
    Decimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PerMode {
    Power,
    Fraction,
    Symbol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UncertaintyMode {
    Compact,
    Separate,
}

/// The option values in force.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    group_digits: GroupDigits,
    group_minimum_digits: usize,
    group_separator: String,
    output_decimal_marker: String,
    exponent_product: String,
    retain_explicit_plus: bool,
    retain_zero_exponent: bool,
    drop_zero_decimal: bool,
    add_integer_zero: bool,
    uncertainty_mode: UncertaintyMode,
    per_mode: PerMode,
    per_symbol: String,
    bracket_unit_denominator: bool,
    inter_unit_product: String,
    /// `None` is the default `\,`: a text-font kern outside math, 3mu inside.
    quantity_product: Option<String>,
    list_separator: String,
    list_final_separator: String,
    list_pair_separator: String,
    range_phrase: String,
    /// `\DeclareSIUnit{\name}{definition}`, most recent last.
    units: Vec<(String, String)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            group_digits: GroupDigits::All,
            group_minimum_digits: 5,
            group_separator: "\\,".into(),
            output_decimal_marker: ".".into(),
            exponent_product: "\\times".into(),
            retain_explicit_plus: false,
            retain_zero_exponent: false,
            drop_zero_decimal: false,
            add_integer_zero: true,
            uncertainty_mode: UncertaintyMode::Compact,
            per_mode: PerMode::Power,
            per_symbol: "/".into(),
            bracket_unit_denominator: true,
            inter_unit_product: "\\,".into(),
            quantity_product: None,
            list_separator: ", ".into(),
            list_final_separator: " and ".into(),
            list_pair_separator: " and ".into(),
            range_phrase: " to ".into(),
            units: Vec::new(),
        }
    }
}

thread_local! {
    static SETTINGS: RefCell<Settings> = RefCell::new(Settings::default());
}

/// Restores the package defaults; called when a parse starts.
pub fn reset() {
    SETTINGS.with(|s| *s.borrow_mut() = Settings::default());
}

fn current() -> Settings {
    SETTINGS.with(|s| s.borrow().clone())
}

/// `\sisetup{keys}`.
pub fn sisetup(keys: &str, span: Span, diagnostics: &mut Vec<Diagnostic>) {
    let mut settings = current();
    settings.apply(keys, span, diagnostics);
    SETTINGS.with(|s| *s.borrow_mut() = settings);
}

/// `\usepackage[options]{siunitx}`: the options are keys, as for `\sisetup`.
pub fn load_package(options: &str, span: Span, diagnostics: &mut Vec<Diagnostic>) {
    sisetup(options, span, diagnostics);
}

/// `\DeclareSIUnit[options]{\name}{definition}` (options are not modelled).
pub fn declare_unit(name: &str, definition: &str) {
    let name = name.trim().trim_start_matches('\\').to_string();
    SETTINGS.with(|s| {
        s.borrow_mut()
            .units
            .push((name, definition.trim().to_string()))
    });
}

/// Splits `key=value, key` at top-level commas, stripping one brace pair
/// around a value as keyval does.
fn key_values(text: &str) -> Vec<(String, Option<String>)> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '{' => {
                depth += 1;
                current.push(ch);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .map(|part| match part.split_once('=') {
            Some((key, value)) => (key.trim().to_string(), Some(strip_braces(value.trim()))),
            None => (part.trim().to_string(), None),
        })
        .collect()
}

fn strip_braces(value: &str) -> String {
    if value.starts_with('{') && value.ends_with('}') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        // Only when the outer braces match each other.
        let mut depth = 0i32;
        for ch in inner.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return value.to_string();
                    }
                }
                _ => {}
            }
        }
        if depth == 0 {
            return inner.to_string();
        }
    }
    value.to_string()
}

impl Settings {
    fn apply(&mut self, keys: &str, span: Span, diagnostics: &mut Vec<Diagnostic>) {
        for (key, value) in key_values(keys) {
            let text = value.clone().unwrap_or_default();
            let flag = || match value.as_deref().map(str::trim) {
                None | Some("true") => Some(true),
                Some("false") => Some(false),
                _ => None,
            };
            let ok = match key.as_str() {
                "group-digits" => match text.trim() {
                    "all" | "true" | "" => {
                        self.group_digits = GroupDigits::All;
                        true
                    }
                    "none" | "false" => {
                        self.group_digits = GroupDigits::None;
                        true
                    }
                    "integer" => {
                        self.group_digits = GroupDigits::Integer;
                        true
                    }
                    "decimal" => {
                        self.group_digits = GroupDigits::Decimal;
                        true
                    }
                    _ => false,
                },
                "group-minimum-digits" => match text.trim().parse::<usize>() {
                    Ok(n) => {
                        self.group_minimum_digits = n;
                        true
                    }
                    Err(_) => false,
                },
                "group-separator" => {
                    self.group_separator = text;
                    true
                }
                "output-decimal-marker" => {
                    self.output_decimal_marker = text;
                    true
                }
                "exponent-product" => {
                    self.exponent_product = text;
                    true
                }
                "inter-unit-product" => {
                    self.inter_unit_product = text;
                    true
                }
                "quantity-product" => {
                    self.quantity_product = Some(text);
                    true
                }
                "per-symbol" => {
                    self.per_symbol = text;
                    true
                }
                "list-separator" => {
                    self.list_separator = text;
                    true
                }
                "list-final-separator" => {
                    self.list_final_separator = text;
                    true
                }
                "list-pair-separator" => {
                    self.list_pair_separator = text;
                    true
                }
                "range-phrase" => {
                    self.range_phrase = text;
                    true
                }
                "retain-explicit-plus" | "retain-zero-exponent" | "drop-zero-decimal"
                | "add-integer-zero" | "bracket-unit-denominator" => match flag() {
                    Some(on) => {
                        match key.as_str() {
                            "retain-explicit-plus" => self.retain_explicit_plus = on,
                            "retain-zero-exponent" => self.retain_zero_exponent = on,
                            "drop-zero-decimal" => self.drop_zero_decimal = on,
                            "add-integer-zero" => self.add_integer_zero = on,
                            _ => self.bracket_unit_denominator = on,
                        }
                        true
                    }
                    None => false,
                },
                "uncertainty-mode" => match text.trim() {
                    "compact" => {
                        self.uncertainty_mode = UncertaintyMode::Compact;
                        true
                    }
                    "separate" => {
                        self.uncertainty_mode = UncertaintyMode::Separate;
                        true
                    }
                    _ => false,
                },
                // `per-mode` is `display-per-mode` and `inline-per-mode`
                // together (7133); this model has one value for both.
                "per-mode" | "inline-per-mode" | "display-per-mode" => match text.trim() {
                    "power" => {
                        self.per_mode = PerMode::Power;
                        true
                    }
                    "fraction" => {
                        self.per_mode = PerMode::Fraction;
                        true
                    }
                    "symbol" => {
                        self.per_mode = PerMode::Symbol;
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if !ok {
                let shown = match &value {
                    Some(v) => format!("{key}={v}"),
                    None => key.clone(),
                };
                diagnostics.push(Diagnostic::warning(
                    format!("siunitx option '{shown}' is not implemented"),
                    Some(span),
                    Some("ignored the option and kept the current setting".into()),
                ));
            }
        }
    }
}

/// Raw source of a token list: control words spelled `\name `, control
/// symbols `\,` (a one-character word spanning two bytes), braces kept.
pub fn raw_text<'a>(tokens: impl IntoIterator<Item = &'a Token>) -> String {
    let mut out = String::new();
    for token in tokens {
        match &token.kind {
            TokenKind::Word(word) => {
                let bytes = token.span.end.saturating_sub(token.span.start);
                if word.chars().count() == 1
                    && bytes == word.len() + 1
                    && !word.chars().all(char::is_alphanumeric)
                {
                    out.push('\\');
                }
                out.push_str(word);
            }
            TokenKind::Command(name) => {
                out.push('\\');
                out.push_str(name);
                out.push(' ');
            }
            TokenKind::Space | TokenKind::ParBreak => out.push(' '),
            TokenKind::LineBreak => out.push_str("\\\\"),
            TokenKind::LBrace => out.push('{'),
            TokenKind::RBrace => out.push('}'),
            TokenKind::MathShift => out.push('$'),
            TokenKind::Superscript => out.push('^'),
            TokenKind::Subscript => out.push('_'),
            TokenKind::DisplayMathOpen | TokenKind::DisplayMathClose | TokenKind::Comment => {}
            TokenKind::Verb { text, .. } => out.push_str(text),
        }
    }
    out
}

/// Typesets one command. `math_mode` is true inside a formula (quantity
/// product `\,` is 3mu glue there, a text-font kern outside). `packages`
/// reaches the math the unit formatter re-parses, which is otherwise the one
/// path into `math::parse_tokens` with no document in scope. Returns the
/// formula's atoms, every span set to `span`.
#[allow(clippy::too_many_arguments)]
pub fn typeset(
    name: &str,
    options: Option<&str>,
    pre_unit: Option<&str>,
    args: &[String],
    math_mode: bool,
    packages: MathPackages,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<MathAtom> {
    let mut settings = current();
    if let Some(options) = options {
        settings.apply(options, span, diagnostics);
    }
    let cx = Context {
        s: &settings,
        math_mode,
        packages,
        span,
    };
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    let mut out = Vec::new();
    match name {
        "num" => out.extend(cx.number(arg(0), diagnostics)),
        "unit" | "si" => out.extend(cx.unit(arg(0), diagnostics)),
        "qty" | "SI" => {
            if let Some(pre) = pre_unit {
                out.extend(cx.unit(pre, diagnostics));
            }
            out.extend(cx.quantity(arg(0), arg(1), diagnostics));
        }
        "numlist" => {
            let items: Vec<Vec<MathAtom>> = split_list(arg(0))
                .iter()
                .map(|item| cx.number(item, diagnostics))
                .collect();
            out.extend(cx.list(items));
        }
        "qtylist" | "SIlist" => {
            let items: Vec<Vec<MathAtom>> = split_list(arg(0))
                .iter()
                .map(|item| cx.quantity(item, arg(1), diagnostics))
                .collect();
            out.extend(cx.list(items));
        }
        "numrange" => {
            out.extend(cx.number(arg(0), diagnostics));
            out.push(cx.phrase(&settings.range_phrase));
            out.extend(cx.number(arg(1), diagnostics));
        }
        "qtyrange" | "SIrange" => {
            out.extend(cx.quantity(arg(0), arg(2), diagnostics));
            out.push(cx.phrase(&settings.range_phrase));
            out.extend(cx.quantity(arg(1), arg(2), diagnostics));
        }
        "ang" => out.extend(cx.angle(arg(0), diagnostics)),
        _ => {}
    }
    out
}

fn split_list(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '{' => {
                depth += 1;
                current.push(ch);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ';' if depth == 0 => items.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    items.push(current);
    items
}

struct Context<'a> {
    s: &'a Settings,
    math_mode: bool,
    /// The document's loaded packages, for the math this re-parses.
    packages: MathPackages,
    span: Span,
}

/// A parsed number (`siunitx.sty` input parser: signs `+-`, decimal
/// markers `.,`, exponent markers `eEdD`, uncertainties `(..)` and `+-`).
#[derive(Debug, Default, PartialEq)]
struct Number {
    sign: Option<char>,
    integer: String,
    decimal: Option<String>,
    uncertainty: Option<Uncertainty>,
    exponent: Option<(Option<char>, String)>,
}

#[derive(Debug, PartialEq)]
enum Uncertainty {
    /// `1.23(4)`: digits in the last places of the value.
    Digits(String),
    /// `1.23 +- 0.04`: an absolute value.
    Value(String, Option<String>),
}

fn parse_number(input: &str) -> Option<Number> {
    let cleaned: String = input
        .replace("\\pm", "+-")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '{' && *c != '}')
        .collect();
    let chars: Vec<char> = cleaned.chars().collect();
    let mut i = 0;
    let mut n = Number::default();
    let digits = |i: &mut usize| {
        let mut s = String::new();
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            s.push(chars[*i]);
            *i += 1;
        }
        s
    };
    if i < chars.len() && matches!(chars[i], '+' | '-') && !(chars.get(i + 1) == Some(&'-') && chars[i] == '+') {
        n.sign = Some(chars[i]);
        i += 1;
    }
    n.integer = digits(&mut i);
    if i < chars.len() && matches!(chars[i], '.' | ',') {
        i += 1;
        n.decimal = Some(digits(&mut i));
    }
    if n.integer.is_empty() && n.decimal.as_deref().is_none_or(str::is_empty) {
        // Only a bare exponent (`e5`) may omit the mantissa.
        if !(i < chars.len() && matches!(chars[i], 'e' | 'E' | 'd' | 'D')) {
            return None;
        }
    }
    if i < chars.len() && chars[i] == '(' {
        i += 1;
        let whole = digits(&mut i);
        let uncertainty = if i < chars.len() && matches!(chars[i], '.' | ',') {
            i += 1;
            Uncertainty::Value(whole, Some(digits(&mut i)))
        } else {
            Uncertainty::Digits(whole)
        };
        if i >= chars.len() || chars[i] != ')' {
            return None;
        }
        i += 1;
        n.uncertainty = Some(uncertainty);
    } else if chars[i..].starts_with(&['+', '-']) {
        i += 2;
        let whole = digits(&mut i);
        let fraction = if i < chars.len() && matches!(chars[i], '.' | ',') {
            i += 1;
            Some(digits(&mut i))
        } else {
            None
        };
        if whole.is_empty() && fraction.as_deref().is_none_or(str::is_empty) {
            return None;
        }
        n.uncertainty = Some(Uncertainty::Value(whole, fraction));
    }
    if i < chars.len() && matches!(chars[i], 'e' | 'E' | 'd' | 'D') {
        i += 1;
        let sign = if i < chars.len() && matches!(chars[i], '+' | '-') {
            i += 1;
            Some(chars[i - 1])
        } else {
            None
        };
        let exponent = digits(&mut i);
        if exponent.is_empty() {
            return None;
        }
        n.exponent = Some((sign, exponent));
    }
    (i == chars.len()).then_some(n)
}

fn group(digits: &str, from_left: bool, separator: &str) -> String {
    let separator = ord_source(separator);
    let separator = if separator.starts_with('\\') && separator.chars().nth(1).is_some_and(|c| c.is_ascii_alphabetic()) {
        separator + " "
    } else {
        separator
    };
    let chars: Vec<char> = digits.chars().collect();
    let mut out = String::new();
    for (index, ch) in chars.iter().enumerate() {
        let boundary = if from_left {
            index > 0 && index % 3 == 0
        } else {
            index > 0 && (chars.len() - index) % 3 == 0
        };
        if boundary {
            out.push_str(&separator);
        }
        out.push(*ch);
    }
    out
}

/// A marker or separator value as math source: punctuation that TeX would
/// class as Punct is an Ord when siunitx inserts it (`{,}`).
fn ord_source(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() == 1 && matches!(trimmed, "," | ";" | ":") {
        format!("\\mathord{{{trimmed}}}")
    } else {
        trimmed.to_string()
    }
}

impl Context<'_> {
    /// Parses `source` as math and sets every span to the command.
    fn math(&self, source: &str) -> Vec<MathAtom> {
        let tokens = lexer::tokenize(source);
        let mut ignored = Vec::new();
        let mut list = math::parse_tokens(&tokens, self.packages, &mut ignored);
        respan_list(&mut list, self.span);
        list.atoms
    }

    fn atom(&self, nucleus: Nucleus) -> MathAtom {
        MathAtom {
            nucleus,
            span: self.span,
            superscript: None,
            subscript: None,
            class_override: None,
            width_em: None,
            ams_symbol: None,
        }
    }

    fn text(&self, text: &str) -> MathAtom {
        self.atom(Nucleus::Text(text.to_string()))
    }

    /// An upright unit symbol (`\mathrm{kg}`): each character is a math
    /// character of the roman family, so pdfLaTeX applies no text kerning
    /// between them. One character is one upright run; several are a
    /// boxed group of one-character runs (TeX boxes a multi-atom
    /// `\mathrm{..}` argument when it carries a script).
    fn upright(&self, text: &str) -> MathAtom {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(_), None) | (None, _) => self.text(text),
            _ => self.group(text.chars().map(|c| self.text(&c.to_string())).collect()),
        }
    }

    /// A list separator or range phrase: text-mode material, so TeX's input
    /// ligatures apply (`range-phrase = {--}` is an en dash).
    fn phrase(&self, text: &str) -> MathAtom {
        self.text(&lexer::apply_text_ligatures(text))
    }

    fn group(&self, atoms: Vec<MathAtom>) -> MathAtom {
        let mut atom = self.atom(Nucleus::Group(MathList { atoms }));
        atom.class_override = Some(math::AtomClass::Ord);
        atom
    }

    /// `{}^{script}`: an empty nucleus carrying a superscript, boxed.
    fn raised(&self, script: &str) -> MathAtom {
        let mut empty = self.atom(Nucleus::Symbol(String::new()));
        empty.superscript = Some(MathList {
            atoms: self.math(script),
        });
        empty
    }

    fn number(&self, input: &str, diagnostics: &mut Vec<Diagnostic>) -> Vec<MathAtom> {
        match parse_number(input) {
            Some(number) => self.math(&self.number_source(number)),
            None => {
                diagnostics.push(Diagnostic::error(
                    format!("siunitx: invalid number '{}'", input.trim()),
                    Some(self.span),
                    Some("typeset the input as it was written".into()),
                ));
                vec![self.text(input.trim())]
            }
        }
    }

    fn number_source(&self, mut n: Number) -> String {
        let s = self.s;
        let mut out = String::new();
        let has_mantissa = !n.integer.is_empty() || n.decimal.is_some();
        match n.sign {
            Some('-') => out.push('-'),
            Some('+') if s.retain_explicit_plus => out.push('+'),
            _ => {}
        }
        // An absolute uncertainty in compact mode becomes digits aligned to
        // the value's last decimal place.
        let mut compact = None;
        let mut separate = None;
        match n.uncertainty.take() {
            Some(Uncertainty::Digits(d)) => compact = Some(d),
            Some(Uncertainty::Value(whole, fraction)) => {
                let fraction = fraction.unwrap_or_default();
                match s.uncertainty_mode {
                    UncertaintyMode::Separate => separate = Some((whole, fraction)),
                    UncertaintyMode::Compact => {
                        let places = n.decimal.as_ref().map_or(0, String::len);
                        if fraction.len() > places {
                            let decimal = n.decimal.get_or_insert_with(String::new);
                            while decimal.len() < fraction.len() {
                                decimal.push('0');
                            }
                        }
                        let places = n.decimal.as_ref().map_or(0, String::len);
                        let mut digits = format!("{whole}{fraction}");
                        digits.extend(std::iter::repeat_n('0', places - fraction.len()));
                        let trimmed = digits.trim_start_matches('0');
                        compact = Some(if trimmed.is_empty() { "0".into() } else { trimmed.into() });
                    }
                }
            }
            None => {}
        }
        if has_mantissa {
            let integer = if n.integer.is_empty() && s.add_integer_zero {
                "0".to_string()
            } else {
                n.integer.clone()
            };
            let group_integer = matches!(s.group_digits, GroupDigits::All | GroupDigits::Integer)
                && integer.len() >= s.group_minimum_digits;
            if group_integer {
                out.push_str(&group(&integer, false, &s.group_separator));
            } else {
                out.push_str(&integer);
            }
            if let Some(decimal) = &n.decimal {
                let zero = decimal.chars().all(|c| c == '0');
                if !(s.drop_zero_decimal && zero && compact.is_none() && separate.is_none()) {
                    out.push_str(&ord_source(&s.output_decimal_marker));
                    let group_decimal =
                        matches!(s.group_digits, GroupDigits::All | GroupDigits::Decimal)
                            && decimal.len() >= s.group_minimum_digits;
                    if group_decimal {
                        out.push_str(&group(decimal, true, &s.group_separator));
                    } else {
                        out.push_str(decimal);
                    }
                }
            }
            if let Some(d) = compact {
                out.push('(');
                out.push_str(&d);
                out.push(')');
            }
            if let Some((whole, fraction)) = separate {
                out.push_str("\\pm ");
                out.push_str(if whole.is_empty() { "0" } else { &whole });
                if !fraction.is_empty() {
                    out.push_str(&ord_source(&s.output_decimal_marker));
                    out.push_str(&fraction);
                }
            }
        }
        if let Some((sign, digits)) = n.exponent {
            let digits = digits.trim_start_matches('0');
            if !digits.is_empty() || s.retain_zero_exponent || !has_mantissa {
                let digits = if digits.is_empty() { "0" } else { digits };
                if has_mantissa {
                    out.push_str(&s.exponent_product);
                    out.push(' ');
                }
                out.push_str("10^{");
                if sign == Some('-') {
                    out.push('-');
                }
                out.push_str(digits);
                out.push('}');
            }
        }
        out
    }

    fn quantity(&self, number: &str, unit: &str, diagnostics: &mut Vec<Diagnostic>) -> Vec<MathAtom> {
        let mut out = self.number(number, diagnostics);
        let unit = self.unit(unit, diagnostics);
        if !unit.is_empty() {
            match &self.s.quantity_product {
                Some(product) => out.extend(self.math(product)),
                // `\penalty10000` and `\,`: a kern of 1/6 em of the current
                // text font outside math, `\thinmuskip` glue inside.
                None if self.math_mode => out.extend(self.math("\\,")),
                None => out.push(self.atom(Nucleus::Space {
                    em: 1.0 / 6.0,
                    font_em: true,
                    nonscript: false,
                })),
            }
            out.extend(unit);
        }
        out
    }

    fn list(&self, items: Vec<Vec<MathAtom>>) -> Vec<MathAtom> {
        let count = items.len();
        let mut out = Vec::new();
        for (index, item) in items.into_iter().enumerate() {
            if index > 0 {
                let separator = if count == 2 {
                    &self.s.list_pair_separator
                } else if index == count - 1 {
                    &self.s.list_final_separator
                } else {
                    &self.s.list_separator
                };
                out.push(self.phrase(separator));
            }
            out.extend(item);
        }
        out
    }

    /// `\ang{d;m;s}`: each non-empty part followed by `{}^{\circ}`,
    /// `{}^{\prime}` or `{}^{\prime\prime}`.
    fn angle(&self, input: &str, diagnostics: &mut Vec<Diagnostic>) -> Vec<MathAtom> {
        let parts = split_list(input);
        let marks = ["\\circ", "\\prime", "\\prime\\prime"];
        let mut out = Vec::new();
        for (part, mark) in parts.iter().zip(marks) {
            if part.trim().is_empty() {
                continue;
            }
            out.extend(self.number(part, diagnostics));
            out.push(self.group(vec![self.raised(mark)]));
        }
        if parts.len() > 3 {
            diagnostics.push(Diagnostic::error(
                "siunitx: \\ang takes at most three parts (degrees;minutes;seconds)",
                Some(self.span),
                Some("typeset the first three parts".into()),
            ));
        }
        out
    }

    fn unit(&self, input: &str, diagnostics: &mut Vec<Diagnostic>) -> Vec<MathAtom> {
        if input.trim().is_empty() {
            return Vec::new();
        }
        if input.contains('\\') {
            let mut units = Vec::new();
            let mut state = UnitState::default();
            self.read_units(input, &mut units, &mut state, 0, diagnostics);
            self.format_units(units)
        } else {
            self.literal_units(input)
        }
    }

    fn read_units(
        &self,
        input: &str,
        units: &mut Vec<Unit>,
        state: &mut UnitState,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            if ch.is_whitespace() || ch == '{' || ch == '}' || ch == '~' || ch == '.' {
                i += 1;
                continue;
            }
            if ch != '\\' {
                // Literal material between macros is an upright unit.
                let mut text = String::new();
                while i < chars.len() && chars[i] != '\\' && !chars[i].is_whitespace() {
                    text.push(chars[i]);
                    i += 1;
                }
                units.push(state.take(UnitBody::Letters(text)));
                continue;
            }
            i += 1;
            let mut name = String::new();
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                name.push(chars[i]);
                i += 1;
            }
            let mut argument = || {
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '{' {
                    let mut depth = 0usize;
                    let mut text = String::new();
                    while i < chars.len() {
                        let c = chars[i];
                        i += 1;
                        match c {
                            '{' => {
                                depth += 1;
                                if depth == 1 {
                                    continue;
                                }
                            }
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        text.push(c);
                    }
                    text
                } else if i < chars.len() {
                    i += 1;
                    chars[i - 1].to_string()
                } else {
                    String::new()
                }
            };
            match name.as_str() {
                "per" => state.per = true,
                "square" => state.power = Some("2".into()),
                "cubic" => state.power = Some("3".into()),
                "raiseto" => state.power = Some(argument()),
                "squared" | "cubed" | "tothe" => {
                    let power = match name.as_str() {
                        "squared" => "2".to_string(),
                        "cubed" => "3".to_string(),
                        _ => argument(),
                    };
                    if let Some(last) = units.last_mut() {
                        last.power = Some(power);
                    }
                }
                "of" => {
                    let qualifier = argument();
                    if let Some(last) = units.last_mut() {
                        last.qualifier = Some(qualifier);
                    }
                }
                _ => {
                    if let Some(symbol) = prefix_symbol(&name) {
                        state.prefix.push_str(symbol);
                    } else if let Some((prefix, body)) = unit_body(&name) {
                        state.prefix.push_str(prefix);
                        units.push(state.take(body));
                    } else if let Some(definition) = self
                        .s
                        .units
                        .iter()
                        .rev()
                        .find(|(n, _)| *n == name)
                        .map(|(_, d)| d.clone())
                    {
                        if depth < 16 {
                            self.read_units(&definition, units, state, depth + 1, diagnostics);
                        }
                    } else {
                        diagnostics.push(Diagnostic::error(
                            format!("siunitx: unknown unit macro \\{name}"),
                            Some(self.span),
                            Some("left the macro out of the unit".into()),
                        ));
                    }
                }
            }
        }
    }

    fn unit_atom(&self, unit: &Unit) -> MathAtom {
        let prefix = unit.prefix.as_str();
        let mut atom = match &unit.body {
            UnitBody::Letters(text) => self.upright(&format!("{prefix}{text}")),
            UnitBody::Ohm if prefix.is_empty() => self.math("\\Omega").remove(0),
            UnitBody::Ohm => {
                let mut atoms = vec![self.text(prefix)];
                atoms.extend(self.math("\\Omega"));
                self.group(atoms)
            }
            UnitBody::Degree => self.group(vec![self.raised("\\circ")]),
            UnitBody::Celsius => self.group(vec![self.raised("\\circ"), self.text("C")]),
            UnitBody::ArcMinute => self.group(vec![self.raised("\\prime")]),
            UnitBody::ArcSecond => self.group(vec![self.raised("\\prime\\prime")]),
            UnitBody::Percent => self.text(&format!("{prefix}%")),
        };
        if let Some(qualifier) = &unit.qualifier {
            atom.subscript = Some(MathList {
                atoms: vec![self.text(qualifier)],
            });
        }
        atom
    }

    fn with_power(&self, unit: &Unit, negate: bool) -> MathAtom {
        let mut atom = self.unit_atom(unit);
        let power = unit.power.clone().unwrap_or_else(|| "1".into());
        let script = if negate {
            Some(format!("-{power}"))
        } else if power != "1" {
            Some(power)
        } else {
            None
        };
        if let Some(script) = script {
            atom.superscript = Some(MathList {
                atoms: self.math(&script),
            });
        }
        atom
    }

    fn product(&self, atoms: Vec<MathAtom>) -> Vec<MathAtom> {
        let mut out = Vec::new();
        for (index, atom) in atoms.into_iter().enumerate() {
            if index > 0 {
                out.extend(self.math(&self.s.inter_unit_product));
            }
            out.push(atom);
        }
        out
    }

    fn format_units(&self, units: Vec<Unit>) -> Vec<MathAtom> {
        let has_per = units.iter().any(|u| u.per);
        if !has_per || self.s.per_mode == PerMode::Power {
            let atoms = units.iter().map(|u| self.with_power(u, u.per)).collect();
            return self.product(atoms);
        }
        let numerator: Vec<MathAtom> = units
            .iter()
            .filter(|u| !u.per)
            .map(|u| self.with_power(u, false))
            .collect();
        let denominator_count = units.iter().filter(|u| u.per).count();
        let denominator: Vec<MathAtom> = units
            .iter()
            .filter(|u| u.per)
            .map(|u| self.with_power(u, false))
            .collect();
        let numerator = if numerator.is_empty() {
            self.math("1")
        } else {
            self.product(numerator)
        };
        let denominator = self.product(denominator);
        match self.s.per_mode {
            // siunitx sets the unit as its own formula, so the fraction is an
            // Ord box: no Ord-Inner thin space against the number.
            PerMode::Fraction => vec![self.group(vec![self.atom(Nucleus::Fraction {
                numerator: MathList { atoms: numerator },
                denominator: MathList { atoms: denominator },
            })])],
            _ => {
                let mut out = numerator;
                out.extend(self.math(&self.s.per_symbol));
                let bracket = self.s.bracket_unit_denominator && denominator_count > 1;
                if bracket {
                    out.extend(self.math("("));
                }
                out.extend(denominator);
                if bracket {
                    out.extend(self.math(")"));
                }
                out
            }
        }
    }

    /// Literal input (`m/s`, `kg.m`, `m^2`): letters are upright units,
    /// `.` and `~` the inter-unit product, `/` the math solidus.
    fn literal_units(&self, input: &str) -> Vec<MathAtom> {
        let mut out: Vec<MathAtom> = Vec::new();
        let chars: Vec<char> = input.trim().chars().collect();
        let mut i = 0;
        let mut text = String::new();
        let flush = |text: &mut String, out: &mut Vec<MathAtom>| {
            if !text.is_empty() {
                out.push(self.text(text));
                text.clear();
            }
        };
        while i < chars.len() {
            let ch = chars[i];
            i += 1;
            match ch {
                '.' | '~' | ' ' => {
                    flush(&mut text, &mut out);
                    if !out.is_empty() && i < chars.len() {
                        out.extend(self.math(&self.s.inter_unit_product));
                    }
                }
                '/' => {
                    flush(&mut text, &mut out);
                    out.extend(self.math("/"));
                }
                '^' | '_' => {
                    flush(&mut text, &mut out);
                    let mut script = String::new();
                    if i < chars.len() && chars[i] == '{' {
                        i += 1;
                        while i < chars.len() && chars[i] != '}' {
                            script.push(chars[i]);
                            i += 1;
                        }
                        i += 1;
                    } else {
                        while i < chars.len() && (chars[i].is_ascii_digit() || (script.is_empty() && chars[i] == '-')) {
                            script.push(chars[i]);
                            i += 1;
                        }
                    }
                    let list = Some(MathList {
                        atoms: self.math(&script),
                    });
                    if let Some(last) = out.last_mut() {
                        if ch == '^' {
                            last.superscript = list;
                        } else {
                            last.subscript = list;
                        }
                    }
                }
                '{' | '}' => {}
                _ => text.push(ch),
            }
        }
        flush(&mut text, &mut out);
        out
    }
}

#[derive(Default)]
struct UnitState {
    prefix: String,
    power: Option<String>,
    per: bool,
}

impl UnitState {
    fn take(&mut self, body: UnitBody) -> Unit {
        Unit {
            prefix: std::mem::take(&mut self.prefix),
            body,
            power: self.power.take(),
            per: std::mem::take(&mut self.per),
            qualifier: None,
        }
    }
}

struct Unit {
    prefix: String,
    body: UnitBody,
    power: Option<String>,
    per: bool,
    qualifier: Option<String>,
}

enum UnitBody {
    Letters(String),
    Ohm,
    Degree,
    Celsius,
    ArcMinute,
    ArcSecond,
    Percent,
}

/// SI prefixes (`siunitx.sty` 7687-7711; `\micro` is the text `µ`).
const PREFIXES: &[(&str, &str)] = &[
    ("quecto", "q"),
    ("ronto", "r"),
    ("yocto", "y"),
    ("zepto", "z"),
    ("atto", "a"),
    ("femto", "f"),
    ("pico", "p"),
    ("nano", "n"),
    ("micro", "µ"),
    ("milli", "m"),
    ("centi", "c"),
    ("deci", "d"),
    ("deca", "da"),
    ("deka", "da"),
    ("hecto", "h"),
    ("kilo", "k"),
    ("mega", "M"),
    ("giga", "G"),
    ("tera", "T"),
    ("peta", "P"),
    ("exa", "E"),
    ("zetta", "Z"),
    ("yotta", "Y"),
    ("ronna", "R"),
    ("quetta", "Q"),
];

fn prefix_symbol(name: &str) -> Option<&'static str> {
    PREFIXES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// Unit macros (`siunitx.sty` 7678-7750, 9634): (name, symbol). A symbol
/// starting with `\` is a special body.
pub const UNITS: &[(&str, &str)] = &[
    ("ampere", "A"),
    ("candela", "cd"),
    ("kelvin", "K"),
    ("kilogram", "kg"),
    ("gram", "g"),
    ("metre", "m"),
    ("meter", "m"),
    ("mole", "mol"),
    ("second", "s"),
    ("becquerel", "Bq"),
    ("degreeCelsius", "\\celsius"),
    ("celsius", "\\celsius"),
    ("coulomb", "C"),
    ("farad", "F"),
    ("gray", "Gy"),
    ("hertz", "Hz"),
    ("henry", "H"),
    ("joule", "J"),
    ("katal", "kat"),
    ("lumen", "lm"),
    ("lux", "lx"),
    ("newton", "N"),
    ("ohm", "\\ohm"),
    ("pascal", "Pa"),
    ("radian", "rad"),
    ("siemens", "S"),
    ("sievert", "Sv"),
    ("steradian", "sr"),
    ("tesla", "T"),
    ("volt", "V"),
    ("watt", "W"),
    ("weber", "Wb"),
    ("astronomicalunit", "au"),
    ("bel", "B"),
    ("decibel", "dB"),
    ("dalton", "Da"),
    ("day", "d"),
    ("electronvolt", "eV"),
    ("hectare", "ha"),
    ("hour", "h"),
    ("litre", "L"),
    ("liter", "L"),
    ("minute", "min"),
    ("neper", "Np"),
    ("tonne", "t"),
    ("arcminute", "\\arcminute"),
    ("arcsecond", "\\arcsecond"),
    ("degree", "\\degree"),
    ("percent", "\\percent"),
];

fn unit_body(name: &str) -> Option<(&'static str, UnitBody)> {
    let (_, symbol) = UNITS.iter().find(|(n, _)| *n == name)?;
    Some(match *symbol {
        "\\celsius" => ("", UnitBody::Celsius),
        "\\ohm" => ("", UnitBody::Ohm),
        "\\arcminute" => ("", UnitBody::ArcMinute),
        "\\arcsecond" => ("", UnitBody::ArcSecond),
        "\\degree" => ("", UnitBody::Degree),
        "\\percent" => ("", UnitBody::Percent),
        // `\kilogram` is `\kilo\gram`, `\decibel` `\deci\bel` (7678, 7736):
        // one run either way.
        other => ("", UnitBody::Letters(other.to_string())),
    })
}

fn respan_list(list: &mut MathList, span: Span) {
    for atom in &mut list.atoms {
        respan_atom(atom, span);
    }
}

fn respan_atom(atom: &mut MathAtom, span: Span) {
    atom.span = span;
    if let Some(list) = &mut atom.superscript {
        respan_list(list, span);
    }
    if let Some(list) = &mut atom.subscript {
        respan_list(list, span);
    }
    match &mut atom.nucleus {
        Nucleus::Symbol(_)
        | Nucleus::SizedDelimiter { .. }
        | Nucleus::Text(_)
        | Nucleus::Space { .. }
        | Nucleus::Bold(_)
        | Nucleus::Rule(_) => {}
        Nucleus::Fraction {
            numerator,
            denominator,
        }
        | Nucleus::GenFraction {
            numerator,
            denominator,
            ..
        } => {
            respan_list(numerator, span);
            respan_list(denominator, span);
        }
        Nucleus::Radical(body)
        | Nucleus::Group(body)
        | Nucleus::Framed { body, .. }
        | Nucleus::Accent { body, .. }
        | Nucleus::Phantom { body, .. }
        | Nucleus::Operator { body, .. } => respan_list(body, span),
        Nucleus::Stacked { base, over, under } => {
            respan_list(base, span);
            for part in [over, under].into_iter().flatten() {
                respan_list(part, span);
            }
        }
        Nucleus::Matrix { rows, .. } => {
            for cell in rows.iter_mut().flatten() {
                respan_list(cell, span);
            }
        }
        Nucleus::ExtArrow { above, below, .. } => {
            respan_list(above, span);
            respan_list(below, span);
        }
        Nucleus::SubArray { rows, .. } => {
            for row in rows {
                respan_list(row, span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(input: &str) -> String {
        let settings = Settings::default();
        let cx = Context {
            s: &settings,
            math_mode: false,
            packages: MathPackages::KERNEL,
            span: Span::new(0, 0),
        };
        cx.number_source(parse_number(input).expect("parses"))
    }

    #[test]
    fn numbers_format_like_siunitx_defaults() {
        assert_eq!(source("12345.6789"), "12\\,345.6789");
        assert_eq!(source("0.00012345"), "0.000\\,123\\,45");
        assert_eq!(source("1234"), "1234");
        assert_eq!(source(".5"), "0.5");
        assert_eq!(source("1,5"), "1.5");
        assert_eq!(source("-1.5e-3"), "-1.5\\times 10^{-3}");
        assert_eq!(source("+3"), "3");
        assert_eq!(source("e5"), "10^{5}");
        assert_eq!(source("-4e0"), "-4");
        assert_eq!(source("1.5e+3"), "1.5\\times 10^{3}");
        assert_eq!(source("1d-3"), "1\\times 10^{-3}");
        assert_eq!(source("1.2(3)"), "1.2(3)");
        assert_eq!(source("1.2+-0.3"), "1.2(3)");
        assert_eq!(source("12.345 \\pm 0.067"), "12.345(67)");
        assert_eq!(source("1.23(4)e5"), "1.23(4)\\times 10^{5}");
        assert!(parse_number("x").is_none());
        assert!(parse_number("1.2.3").is_none());
    }

    #[test]
    fn keys_split_at_top_level_commas_and_strip_one_brace_pair() {
        assert_eq!(
            key_values("output-decimal-marker={,}, group-digits=none,retain-explicit-plus"),
            vec![
                ("output-decimal-marker".to_string(), Some(",".to_string())),
                ("group-digits".to_string(), Some("none".to_string())),
                ("retain-explicit-plus".to_string(), None),
            ]
        );
        let mut settings = Settings::default();
        let mut diagnostics = Vec::new();
        settings.apply("list-separator={; }, bogus=1", Span::new(0, 0), &mut diagnostics);
        assert_eq!(settings.list_separator, "; ");
        assert_eq!(diagnostics.len(), 1);
    }
}
