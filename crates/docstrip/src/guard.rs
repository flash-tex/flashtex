//! Guard expressions (docstrip.dtx §"Conditional inclusion of code",
//! lines 728–788, and §"The evaluation of boolean expressions", lines
//! 1981–2320).
//!
//! ```text
//! Expression ::= Secondary | Secondary {'|' , ','} Expression
//! Secondary  ::= Primary | Primary '&' Secondary
//! Primary    ::= Terminal | '!' Primary | '(' Expression ')'
//! ```
//!
//! A terminal is any run of characters other than `> & ! | , ( )`
//! (`\eT@…`, lines 2091–2094) — spaces included, there is no trimming —
//! and is true iff it occurs in the option list, which docstrip tests by
//! looking for `,term,` in `,options,` (`\t@<Terminal>`, line 2113): the
//! options are matched as literal comma-separated bytes, so
//! `{package, ltx}` does not satisfy the guard `ltx`.

/// A parsed guard expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    Term(Vec<u8>),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

/// Parses the text between `%<` (after any modifier) and the closing `>`.
/// Errors are docstrip's own messages (`empty terminal`, `expected right
/// parenthesis`, `spurious …`).
pub fn parse(text: &[u8]) -> Result<Expr, String> {
    let mut p = Parser { text, at: 0 };
    let e = p.expression()?;
    if p.at < text.len() {
        return Err(format!("Error in expression: spurious {}", text[p.at] as char));
    }
    Ok(e)
}

struct Parser<'a> {
    text: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    // \Expression / \ExpressionX / \ExpressionXX (lines 2258–2299)
    fn expression(&mut self) -> Result<Expr, String> {
        let left = self.secondary()?;
        if matches!(self.peek(), Some(b'|' | b',')) {
            self.at += 1;
            let right = self.expression()?;
            return Ok(Expr::Or(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    // \Secondary / \SecondaryX / \SecondaryXX (lines 2211–2256)
    fn secondary(&mut self) -> Result<Expr, String> {
        let left = self.primary()?;
        if self.peek() == Some(b'&') {
            self.at += 1;
            let right = self.secondary()?;
            return Ok(Expr::And(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    // \Primary / \NPrimary / \PExpression (lines 2152–2209)
    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(b'!') => {
                self.at += 1;
                Ok(Expr::Not(Box::new(self.primary()?)))
            }
            Some(b'(') => {
                self.at += 1;
                let e = self.expression()?;
                if self.peek() != Some(b')') {
                    return Err("Error in expression: expected right parenthesis".into());
                }
                self.at += 1;
                Ok(e)
            }
            _ => self.terminal(),
        }
    }

    // \Terminal / \TerminalX (lines 2068–2124)
    fn terminal(&mut self) -> Result<Expr, String> {
        let start = self.at;
        while let Some(b) = self.peek() {
            if matches!(b, b'>' | b'&' | b'!' | b'|' | b',' | b'(' | b')') {
                break;
            }
            self.at += 1;
        }
        if self.at == start {
            return Err("Error in expression: empty terminal".into());
        }
        Ok(Expr::Term(self.text[start..self.at].to_vec()))
    }
}

/// Whether `expr` holds for the option list `options` (the second
/// argument of `\from`, verbatim).
pub fn eval(expr: &Expr, options: &[u8]) -> bool {
    match expr {
        Expr::Term(t) => has_option(options, t),
        Expr::Not(e) => !eval(e, options),
        Expr::And(a, b) => eval(a, options) && eval(b, options),
        Expr::Or(a, b) => eval(a, options) || eval(b, options),
    }
}

/// `,term,` occurs in `,options,`.
pub fn has_option(options: &[u8], term: &[u8]) -> bool {
    let mut hay = Vec::with_capacity(options.len() + 2);
    hay.push(b',');
    hay.extend_from_slice(options);
    hay.push(b',');
    let mut needle = Vec::with_capacity(term.len() + 2);
    needle.push(b',');
    needle.extend_from_slice(term);
    needle.push(b',');
    hay.windows(needle.len()).any(|w| w == needle.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holds(expr: &str, options: &str) -> bool {
        eval(&parse(expr.as_bytes()).unwrap(), options.as_bytes())
    }

    #[test]
    fn terminals_are_matched_literally_in_the_option_list() {
        assert!(holds("package", "package"));
        assert!(holds("package", "package,ltx"));
        assert!(holds("ltx", "package,ltx"));
        assert!(!holds("pack", "package"));
        assert!(!holds("ltx", "package, ltx"), "no trimming: the option is ` ltx`");
        assert!(holds(" ltx", "package, ltx"));
        assert!(!holds("package", ""));
        assert!(holds("cfg-t", "config,cfg-t,m-t"));
        assert!(holds("pdf-", "pdf-"));
    }

    #[test]
    fn operators_and_precedence() {
        assert!(holds("a|b", "b"));
        assert!(holds("a,b", "b"), "comma is disjunction");
        assert!(!holds("a&b", "b"));
        assert!(holds("a&b", "a,b"));
        assert!(holds("!a", "b"));
        assert!(!holds("!a", "a"));
        assert!(holds("a|b&c", "a"), "& binds tighter than |");
        assert!(holds("a|b&c", "b,c"));
        assert!(!holds("a|b&c", "b"));
        assert!(holds("(a|b)&c", "b,c"));
        assert!(!holds("(a|b)&c", "a"));
        assert!(holds("!(a|b)", "c"));
        assert!(holds("!!a", "a"));
        assert!(holds("a&!b", "a"));
        assert!(!holds("a&!b", "a,b"));
    }

    #[test]
    fn errors_are_docstrips() {
        assert_eq!(parse(b"").unwrap_err(), "Error in expression: empty terminal");
        assert_eq!(parse(b"a&").unwrap_err(), "Error in expression: empty terminal");
        assert_eq!(parse(b"(a").unwrap_err(), "Error in expression: expected right parenthesis");
        assert_eq!(parse(b"a)").unwrap_err(), "Error in expression: spurious )");
        assert_eq!(parse(b"a|b").unwrap(), Expr::Or(Box::new(Expr::Term(b"a".to_vec())), Box::new(Expr::Term(b"b".to_vec()))));
    }
}
