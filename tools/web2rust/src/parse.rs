//! Parser for the Pascal subset that WEB programs are written in, working on
//! the token stream the tangle stage produces.
//!
//! The subset `tex.web` needs: subrange types, plain and variant records,
//! arrays (including `packed array of char`), `file of`, `label`/`goto`,
//! `case` with WEB's `othercases`, `for`/`while`/`repeat`, and `forward`
//! declarations. There are no `with` statements, no Pascal sets and no pointer
//! types in `tex.web`, so those are rejected rather than translated.

use crate::tangle::Tangled;
use crate::tok::Tok;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int,
    Real,
    /// A `real` that is 32 bits wide. `tex.web` §109 declares `glue_ratio=real`
    /// and marks it `@^system dependencies@>`; web2c makes that one type a C
    /// `float`, and Knuth's master `trip.log` was produced the same way, so
    /// `--scalar glue_ratio=f32` selects it.
    Real32,
    /// web2c's `longinteger`, a 64-bit integer (pdfTeX uses it for byte
    /// offsets in the PDF file and in `print_int`); `--scalar longinteger=i64`.
    Int64,
    Bool,
    Char,
    /// A subrange `lo..hi`; carried through so record fields can be packed.
    Sub(i64, i64),
    Named(String),
    Array {
        lo: i64,
        hi: i64,
        elem: Box<Ty>,
    },
    /// `file of T`. `alpha_file` is `packed file of char`.
    File(Box<Ty>),
    /// `^T`. pdfTeX uses pointers only for arrays that web2c allocates with
    /// `xmalloc_array(T, n)` (elements `0..n`) and grows with
    /// `xrealloc_array`, so a pointer is a growable 0-based array here.
    Ptr(Box<Ty>),
    Record(Record),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub fields: Vec<Field>,
    /// `case <tag type> of n: (fields); ...` — `tex.web` never names the tag.
    pub variants: Vec<(i64, Vec<Field>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    /// An integer constant that is the whole expansion of the WEB macro of
    /// this name (`Tangled::names`).
    Named(String, i64),
    Real(String),
    Str(String),
    Var(String),
    Index(Box<Expr>, Vec<Expr>),
    Field(Box<Expr>, String),
    /// Pascal's file buffer variable `f^`.
    Deref(Box<Expr>),
    Call(String, Vec<Expr>),
    Un(&'static str, Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
}

/// A `case` label, with the macro whose whole expansion it is.
pub type CaseLabel = (i64, Option<String>);

#[derive(Clone, Debug)]
pub enum Stmt {
    Empty,
    Assign(Expr, Expr),
    Call(String, Vec<Expr>),
    Compound(Vec<S>),
    If(Expr, Box<S>, Option<Box<S>>),
    While(Expr, Box<S>),
    Repeat(Vec<S>, Expr),
    For {
        var: String,
        from: Expr,
        to: Expr,
        down: bool,
        body: Box<S>,
    },
    Case {
        sel: Expr,
        arms: Vec<(Vec<CaseLabel>, S)>,
        other: Option<Box<S>>,
    },
    Goto(i64),
}

/// A statement together with its WEB section number and any labels on it.
#[derive(Clone, Debug)]
pub struct S {
    pub sec: u32,
    pub labels: Vec<i64>,
    pub st: Stmt,
}

#[derive(Clone, Debug)]
pub struct VarDecl {
    pub name: String,
    pub ty: Ty,
    pub sec: u32,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    pub by_ref: bool,
}

#[derive(Clone, Debug)]
pub struct Routine {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Ty>,
    /// The `label` declaration; `goto` targets are found from the body.
    #[allow(dead_code)]
    pub labels: Vec<i64>,
    pub locals: Vec<VarDecl>,
    pub body: Vec<S>,
    pub sec: u32,
    /// Declared `external`: the body is hand-written Rust (the parts of
    /// pdfTeX that are C in web2c), and only the signature is translated.
    pub external: bool,
}

pub struct Program {
    pub consts: Vec<(String, Expr, u32)>,
    pub types: Vec<(String, Ty, u32)>,
    pub globals: Vec<VarDecl>,
    pub routines: Vec<Routine>,
    #[allow(dead_code)]
    pub main_labels: Vec<i64>,
    pub main: Vec<S>,
    /// Const name -> integer value, for evaluating subrange bounds.
    #[allow(dead_code)]
    pub const_vals: HashMap<String, i64>,
    /// Type name -> resolved type.
    pub type_map: HashMap<String, Ty>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

type R<T> = Result<T, String>;

struct P<'a> {
    t: &'a [Tok],
    secs: &'a [u32],
    names: &'a [Option<std::rc::Rc<str>>],
    i: usize,
    /// The top operator of the expression just parsed, unless it was
    /// parenthesised. See `precedence_clash`.
    bare: Option<&'static str>,
    clashes: Vec<String>,
    consts: HashMap<String, i64>,
    types: HashMap<String, Ty>,
}

impl<'a> P<'a> {
    fn peek(&self) -> &Tok {
        self.t.get(self.i).unwrap_or(&Tok::SecEnd)
    }
    fn sec(&self) -> u32 {
        *self.secs.get(self.i).unwrap_or(&0)
    }
    fn next(&mut self) -> Tok {
        let t = self.peek().clone();
        self.i += 1;
        t
    }
    fn ctx(&self) -> String {
        let a = self.i.saturating_sub(8);
        let b = (self.i + 8).min(self.t.len());
        let mut s = String::new();
        for (k, tk) in self.t[a..b].iter().enumerate() {
            if a + k == self.i {
                s.push_str(" >>> ");
            }
            s.push_str(&show(tk));
            s.push(' ');
        }
        format!("§{} near: {s}", self.sec())
    }
    fn err<T>(&self, msg: &str) -> R<T> {
        Err(format!("{msg} ({})", self.ctx()))
    }
    fn eat_op(&mut self, o: &str) -> bool {
        if self.peek().is_op(o) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect_op(&mut self, o: &str) -> R<()> {
        if self.eat_op(o) {
            Ok(())
        } else {
            self.err(&format!("expected `{o}`"))
        }
    }
    fn eat_kw(&mut self, k: &str) -> bool {
        if self.peek().is_id(k) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect_kw(&mut self, k: &str) -> R<()> {
        if self.eat_kw(k) {
            Ok(())
        } else {
            self.err(&format!("expected `{k}`"))
        }
    }
    fn ident(&mut self) -> R<String> {
        match self.next() {
            Tok::Id(n) => Ok(n.to_string()),
            _ => {
                self.i -= 1;
                self.err("expected an identifier")
            }
        }
    }

    /// A signed integer constant: a literal, or a `const` name.
    /// `const_int`, and the macro whose whole expansion the constant is.
    fn case_label(&mut self) -> R<CaseLabel> {
        let name = match self.peek() {
            Tok::Int(_) => self.names[self.i].as_ref().map(|n| n.to_string()),
            _ => None,
        };
        Ok((self.const_int()?, name))
    }

    fn const_int(&mut self) -> R<i64> {
        let neg = if self.eat_op("-") {
            true
        } else {
            self.eat_op("+");
            false
        };
        let v = match self.next() {
            Tok::Int(v) => v,
            Tok::Id(n) => match self.consts.get(&*n) {
                Some(v) => *v,
                None => {
                    self.i -= 1;
                    return self.err(&format!("`{n}` is not an integer constant"));
                }
            },
            Tok::Str(s) if s.chars().count() == 1 => s.chars().next().unwrap() as i64,
            _ => {
                self.i -= 1;
                return self.err("expected an integer constant");
            }
        };
        Ok(if neg { -v } else { v })
    }

    // ---- types ------------------------------------------------------------

    fn ty(&mut self) -> R<Ty> {
        self.eat_kw("packed");
        if self.eat_kw("array") {
            self.expect_op("[")?;
            let mut dims = vec![self.index_range()?];
            while self.eat_op(",") {
                dims.push(self.index_range()?);
            }
            self.expect_op("]")?;
            self.expect_kw("of")?;
            let mut elem = self.ty()?;
            for (lo, hi) in dims.into_iter().rev() {
                elem = Ty::Array {
                    lo,
                    hi,
                    elem: Box::new(elem),
                };
            }
            return Ok(elem);
        }
        if self.eat_kw("file") {
            self.expect_kw("of")?;
            let e = self.ty()?;
            return Ok(Ty::File(Box::new(e)));
        }
        if self.eat_kw("record") {
            return self.record();
        }
        if self.peek().is_id("set") {
            return self.err("Pascal sets are not supported (and tex.web has none)");
        }
        if self.eat_op("^") {
            let e = self.ty()?;
            return Ok(Ty::Ptr(Box::new(e)));
        }
        // subrange or named type
        let save = self.i;
        if let Ok(lo) = self.const_int() {
            if self.eat_op("..") {
                let hi = self.const_int()?;
                return Ok(Ty::Sub(lo, hi));
            }
        }
        self.i = save;
        let n = self.ident()?;
        Ok(match n.as_str() {
            "integer" => Ty::Int,
            "real" => Ty::Real,
            "boolean" => Ty::Bool,
            "char" => Ty::Char,
            _ => Ty::Named(n),
        })
    }

    /// An array index range: a subrange, or a type name standing for one.
    fn index_range(&mut self) -> R<(i64, i64)> {
        let t = self.ty()?;
        self.range_of(&t)
    }

    fn range_of(&self, t: &Ty) -> R<(i64, i64)> {
        match t {
            Ty::Sub(a, b) => Ok((*a, *b)),
            Ty::Char => Ok((0, 255)),
            Ty::Named(n) => match self.types.get(n) {
                Some(t2) => self.range_of(t2),
                None => Err(format!("unknown index type `{n}`")),
            },
            _ => Err(format!("{t:?} cannot be an array index type")),
        }
    }

    fn record(&mut self) -> R<Ty> {
        let mut fields = vec![];
        let mut variants = vec![];
        loop {
            if self.eat_kw("end") {
                break;
            }
            if self.eat_kw("case") {
                // `case <tag type> of n: (fields); ... end`
                let _tag = self.ty()?;
                self.expect_kw("of")?;
                loop {
                    if self.eat_kw("end") {
                        break;
                    }
                    if self.eat_op(";") {
                        continue;
                    }
                    let tag = self.const_int()?;
                    self.expect_op(":")?;
                    self.expect_op("(")?;
                    let mut vf = vec![];
                    loop {
                        if self.eat_op(")") {
                            break;
                        }
                        if self.eat_op(";") {
                            continue;
                        }
                        self.field_group(&mut vf)?;
                    }
                    variants.push((tag, vf));
                }
                break;
            }
            if self.eat_op(";") {
                continue;
            }
            self.field_group(&mut fields)?;
        }
        Ok(Ty::Record(Record { fields, variants }))
    }

    /// `a, b, c : T`
    fn field_group(&mut self, out: &mut Vec<Field>) -> R<()> {
        let mut names = vec![self.ident()?];
        while self.eat_op(",") {
            names.push(self.ident()?);
        }
        self.expect_op(":")?;
        let t = self.ty()?;
        for n in names {
            out.push(Field {
                name: n,
                ty: t.clone(),
            });
        }
        Ok(())
    }

    // ---- expressions ------------------------------------------------------

    /// web2c prints Pascal's `and`, `or` and `not` as C's `&&`, `||` and
    /// `!` without adding parentheses (web2c-parser.y), and C gives `&&` and
    /// `||` lower precedence than arithmetic and comparisons, where Pascal
    /// gives `and` that of `*` and `or` that of `+`. `tex.web` never writes an
    /// expression where the two readings differ, but pdfTeX's newer code does,
    /// and pdfTeX *is* the C reading. Such an expression is refused here, so
    /// that a change file can add the parentheses the C compiler implies.
    fn precedence_clash(&mut self, op: &str, left: Option<&str>, right: Option<&str>) -> R<()> {
        let logic = |b: Option<&str>| matches!(b, Some("and") | Some("or"));
        let clash = match op {
            "=" | "<>" | "<" | ">" | "<=" | ">=" => logic(left) || logic(right),
            "+" | "-" => logic(left) || right == Some("and"),
            "*" | "/" | "div" | "mod" => left == Some("and"),
            _ => false,
        };
        if clash {
            // Collected, so that one run lists every such expression.
            let e = self.err::<()>(&format!(
                "`{op}` next to an unparenthesised `and`/`or`: Pascal and web2c's C \
                 read this differently; parenthesise it in a change file"
            ));
            self.clashes.push(e.unwrap_err());
        }
        Ok(())
    }

    fn expr(&mut self) -> R<Expr> {
        let mut e = self.simple()?;
        loop {
            let o = match self.peek() {
                Tok::Op(o @ ("=" | "<>" | "<" | ">" | "<=" | ">=")) => *o,
                Tok::Id(n) if &**n == "in" => return self.err("Pascal `in` is not supported"),
                _ => break,
            };
            let lb = self.bare;
            self.i += 1;
            let r = self.simple()?;
            self.precedence_clash(o, lb, self.bare)?;
            e = Expr::Bin(o, Box::new(e), Box::new(r));
            self.bare = Some(o);
        }
        Ok(e)
    }

    fn simple(&mut self) -> R<Expr> {
        let mut e = if self.eat_op("-") {
            let t = self.term()?;
            self.bare = Some("neg");
            Expr::Un("-", Box::new(t))
        } else {
            self.eat_op("+");
            self.term()?
        };
        loop {
            let o = match self.peek() {
                Tok::Op(o @ ("+" | "-")) => *o,
                Tok::Id(n) if &**n == "or" => "or",
                _ => break,
            };
            let lb = self.bare;
            self.i += 1;
            let r = self.term()?;
            self.precedence_clash(o, lb, self.bare)?;
            e = Expr::Bin(o, Box::new(e), Box::new(r));
            self.bare = Some(o);
        }
        Ok(e)
    }

    fn term(&mut self) -> R<Expr> {
        let mut e = self.factor()?;
        loop {
            let o = match self.peek() {
                Tok::Op(o @ ("*" | "/")) => *o,
                Tok::Id(n) => match &**n {
                    "div" => "div",
                    "mod" => "mod",
                    "and" => "and",
                    _ => break,
                },
                _ => break,
            };
            let lb = self.bare;
            self.i += 1;
            let r = self.factor()?;
            self.precedence_clash(o, lb, self.bare)?;
            e = Expr::Bin(o, Box::new(e), Box::new(r));
            self.bare = Some(o);
        }
        Ok(e)
    }

    fn factor(&mut self) -> R<Expr> {
        let e = match self.next() {
            Tok::Int(v) => match &self.names[self.i - 1] {
                Some(n) => Expr::Named(n.to_string(), v),
                None => Expr::Int(v),
            },
            Tok::Real(s) => Expr::Real(s),
            Tok::Str(s) => Expr::Str(s),
            Tok::Op("(") => {
                let e = self.expr()?;
                self.expect_op(")")?;
                e
            }
            Tok::Op("-") => Expr::Un("-", Box::new(self.factor()?)),
            Tok::Id(n) => match &*n {
                "not" => Expr::Un("not", Box::new(self.factor()?)),
                "true" => Expr::Var("true".into()),
                "false" => Expr::Var("false".into()),
                _ => {
                    if self.peek().is_op("(") {
                        self.i += 1;
                        let args = self.args()?;
                        Expr::Call(n.to_string(), args)
                    } else {
                        Expr::Var(n.to_string())
                    }
                }
            },
            _ => {
                self.i -= 1;
                return self.err("expected an expression");
            }
        };
        // A factor is atomic: any operators it contains are inside
        // parentheses or an argument list.
        let e = self.suffixes(e)?;
        self.bare = None;
        Ok(e)
    }

    fn suffixes(&mut self, mut e: Expr) -> R<Expr> {
        loop {
            if self.eat_op("[") {
                let mut ix = vec![self.expr()?];
                while self.eat_op(",") {
                    ix.push(self.expr()?);
                }
                self.expect_op("]")?;
                e = Expr::Index(Box::new(e), ix);
            } else if self.peek().is_op(".") {
                // `.` only introduces a field here; `end.` is handled elsewhere.
                match self.t.get(self.i + 1) {
                    Some(Tok::Id(_)) => {
                        self.i += 1;
                        let f = self.ident()?;
                        e = Expr::Field(Box::new(e), f);
                    }
                    _ => break,
                }
            } else if self.eat_op("^") {
                e = Expr::Deref(Box::new(e));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn args(&mut self) -> R<Vec<Expr>> {
        let mut out = vec![];
        if self.eat_op(")") {
            return Ok(out);
        }
        loop {
            let e = self.expr()?;
            // `write(f, x:1)` — a Pascal field-width specifier.
            if self.eat_op(":") {
                let w = self.expr()?;
                out.push(Expr::Bin(":", Box::new(e), Box::new(w)));
            } else {
                out.push(e);
            }
            if self.eat_op(",") {
                continue;
            }
            self.expect_op(")")?;
            return Ok(out);
        }
    }

    // ---- statements -------------------------------------------------------

    fn stmt(&mut self) -> R<S> {
        let sec = self.sec();
        let mut labels = vec![];
        // A statement may be prefixed by numeric labels.
        while let Tok::Int(v) = *self.peek() {
            if self.t.get(self.i + 1).map(|t| t.is_op(":")) == Some(true) {
                labels.push(v);
                self.i += 2;
            } else {
                break;
            }
        }
        let st = self.bare_stmt()?;
        Ok(S { sec, labels, st })
    }

    fn bare_stmt(&mut self) -> R<Stmt> {
        match self.peek().clone() {
            Tok::Id(n) => match &*n {
                "begin" => {
                    self.i += 1;
                    let body = self.stmt_seq(&["end"])?;
                    self.expect_kw("end")?;
                    Ok(Stmt::Compound(body))
                }
                "if" => {
                    self.i += 1;
                    let c = self.expr()?;
                    self.expect_kw("then")?;
                    let t = self.stmt()?;
                    let e = if self.eat_kw("else") {
                        Some(Box::new(self.stmt()?))
                    } else {
                        None
                    };
                    Ok(Stmt::If(c, Box::new(t), e))
                }
                "while" => {
                    self.i += 1;
                    let c = self.expr()?;
                    self.expect_kw("do")?;
                    Ok(Stmt::While(c, Box::new(self.stmt()?)))
                }
                "repeat" => {
                    self.i += 1;
                    let body = self.stmt_seq(&["until"])?;
                    self.expect_kw("until")?;
                    let c = self.expr()?;
                    Ok(Stmt::Repeat(body, c))
                }
                "for" => {
                    self.i += 1;
                    let v = self.ident()?;
                    self.expect_op(":=")?;
                    let from = self.expr()?;
                    let down = if self.eat_kw("to") {
                        false
                    } else if self.eat_kw("downto") {
                        true
                    } else {
                        return self.err("expected `to` or `downto`");
                    };
                    let to = self.expr()?;
                    self.expect_kw("do")?;
                    Ok(Stmt::For {
                        var: v,
                        from,
                        to,
                        down,
                        body: Box::new(self.stmt()?),
                    })
                }
                "case" => {
                    self.i += 1;
                    let sel = self.expr()?;
                    self.expect_kw("of")?;
                    let mut arms = vec![];
                    let mut other = None;
                    loop {
                        if self.eat_kw("end") {
                            break;
                        }
                        if self.eat_op(";") {
                            continue;
                        }
                        if self.eat_kw("others") {
                            self.expect_op(":")?;
                            other = Some(Box::new(self.stmt()?));
                            continue;
                        }
                        let mut ls = vec![self.case_label()?];
                        while self.eat_op(",") {
                            ls.push(self.case_label()?);
                        }
                        self.expect_op(":")?;
                        arms.push((ls, self.stmt()?));
                    }
                    Ok(Stmt::Case { sel, arms, other })
                }
                "goto" => {
                    self.i += 1;
                    let v = self.const_int()?;
                    Ok(Stmt::Goto(v))
                }
                "with" => self.err("`with` is not supported (and tex.web has none)"),
                "end" | "until" | "else" => Ok(Stmt::Empty),
                _ => {
                    // assignment or procedure call
                    self.i += 1;
                    let base = if self.eat_op("(") {
                        let args = self.args()?;
                        let c = Expr::Call(n.to_string(), args);
                        self.suffixes(c)?
                    } else {
                        self.suffixes(Expr::Var(n.to_string()))?
                    };
                    if self.eat_op(":=") {
                        let v = self.expr()?;
                        Ok(Stmt::Assign(base, v))
                    } else {
                        match base {
                            Expr::Var(f) => Ok(Stmt::Call(f, vec![])),
                            Expr::Call(f, a) => Ok(Stmt::Call(f, a)),
                            other => Err(format!(
                                "statement is neither assignment nor call: {other:?}"
                            )),
                        }
                    }
                }
            },
            Tok::Op(";") => Ok(Stmt::Empty),
            _ => self.err("expected a statement"),
        }
    }

    fn stmt_seq(&mut self, stop: &[&str]) -> R<Vec<S>> {
        let mut out = vec![];
        loop {
            if let Tok::Id(n) = self.peek() {
                if stop.contains(&&**n) {
                    return Ok(out);
                }
            }
            if self.eat_op(";") {
                continue;
            }
            let s = self.stmt()?;
            let empty = matches!(s.st, Stmt::Empty) && s.labels.is_empty();
            if !empty {
                out.push(s);
            }
            if self.eat_op(";") {
                continue;
            }
            if let Tok::Id(n) = self.peek() {
                if stop.contains(&&**n) {
                    return Ok(out);
                }
            }
            if self.i >= self.t.len() {
                return Ok(out);
            }
            // Statements must be separated by `;`.
            return self.err("expected `;` between statements");
        }
    }
}

fn show(t: &Tok) -> String {
    match t {
        Tok::Id(n) => n.to_string(),
        Tok::Int(v) => v.to_string(),
        Tok::Real(s) => s.clone(),
        Tok::Str(s) => format!("'{s}'"),
        Tok::Op(o) => o.to_string(),
        Tok::Sec(n) => format!("§{n}"),
        Tok::SecEnd => "<eof>".into(),
    }
}

// ---------------------------------------------------------------------------

pub fn parse(t: &Tangled) -> R<Program> {
    let mut p = P {
        t: &t.tokens,
        secs: &t.secs,
        names: &t.names,
        i: 0,
        bare: None,
        clashes: vec![],
        consts: HashMap::new(),
        types: HashMap::new(),
    };
    p.expect_kw("program")?;
    let _name = p.ident()?;
    if p.eat_op("(") {
        while !p.eat_op(")") {
            p.i += 1;
        }
    }
    p.expect_op(";")?;

    let mut main_labels = vec![];
    if p.eat_kw("label") {
        loop {
            main_labels.push(p.const_int()?);
            if p.eat_op(",") {
                continue;
            }
            p.expect_op(";")?;
            break;
        }
    }

    let mut consts = vec![];
    if p.eat_kw("const") {
        while let Tok::Id(_) = p.peek() {
            if matches!(p.peek(), Tok::Id(n) if matches!(&**n, "type"|"var"|"procedure"|"function"|"begin"))
            {
                break;
            }
            let sec = p.sec();
            let n = p.ident()?;
            p.expect_op("=")?;
            let v = p.expr()?;
            p.expect_op(";")?;
            if let Expr::Int(k) | Expr::Named(_, k) = v {
                p.consts.insert(n.clone(), k);
            }
            consts.push((n, v, sec));
        }
    }

    let mut types = vec![];
    if p.eat_kw("type") {
        while let Tok::Id(_) = p.peek() {
            if matches!(p.peek(), Tok::Id(n) if matches!(&**n, "var"|"procedure"|"function"|"begin"))
            {
                break;
            }
            let sec = p.sec();
            let n = p.ident()?;
            p.expect_op("=")?;
            let ty = p.ty()?;
            p.expect_op(";")?;
            p.types.insert(n.clone(), ty.clone());
            types.push((n, ty, sec));
        }
    }

    let mut globals = vec![];
    if p.eat_kw("var") {
        while let Tok::Id(_) = p.peek() {
            if matches!(p.peek(), Tok::Id(n) if matches!(&**n, "procedure"|"function"|"begin")) {
                break;
            }
            let sec = p.sec();
            let mut names = vec![p.ident()?];
            while p.eat_op(",") {
                names.push(p.ident()?);
            }
            p.expect_op(":")?;
            let ty = p.ty()?;
            p.expect_op(";")?;
            for n in names {
                globals.push(VarDecl {
                    name: n,
                    ty: ty.clone(),
                    sec,
                });
            }
        }
    }

    let mut routines: Vec<Routine> = vec![];
    loop {
        let is_fn = if p.peek().is_id("procedure") {
            p.i += 1;
            false
        } else if p.peek().is_id("function") {
            p.i += 1;
            true
        } else {
            break;
        };
        let sec = p.sec();
        let name = p.ident()?;
        let mut params = vec![];
        if p.eat_op("(") {
            loop {
                if p.eat_op(")") {
                    break;
                }
                if p.eat_op(";") {
                    continue;
                }
                let by_ref = p.eat_kw("var");
                let mut names = vec![p.ident()?];
                while p.eat_op(",") {
                    names.push(p.ident()?);
                }
                p.expect_op(":")?;
                let ty = p.ty()?;
                for n in names {
                    params.push(Param {
                        name: n,
                        ty: ty.clone(),
                        by_ref,
                    });
                }
            }
        }
        let ret = if is_fn {
            p.expect_op(":")?;
            Some(p.ty()?)
        } else {
            None
        };
        p.expect_op(";")?;
        if p.eat_kw("forward") {
            p.expect_op(";")?;
            continue;
        }
        if p.eat_kw("external") {
            p.expect_op(";")?;
            routines.push(Routine {
                name,
                params,
                ret,
                labels: vec![],
                locals: vec![],
                body: vec![],
                sec,
                external: true,
            });
            continue;
        }
        let mut labels = vec![];
        if p.eat_kw("label") {
            loop {
                labels.push(p.const_int()?);
                if p.eat_op(",") {
                    continue;
                }
                p.expect_op(";")?;
                break;
            }
        }
        let mut locals = vec![];
        if p.eat_kw("const") {
            return p.err("local `const` sections are not supported");
        }
        if p.eat_kw("var") {
            while let Tok::Id(_) = p.peek() {
                if matches!(p.peek(), Tok::Id(n) if matches!(&**n, "begin"|"procedure"|"function"))
                {
                    break;
                }
                let lsec = p.sec();
                let mut names = vec![p.ident()?];
                while p.eat_op(",") {
                    names.push(p.ident()?);
                }
                p.expect_op(":")?;
                let ty = p.ty()?;
                p.expect_op(";")?;
                for n in names {
                    locals.push(VarDecl {
                        name: n,
                        ty: ty.clone(),
                        sec: lsec,
                    });
                }
            }
        }
        p.expect_kw("begin")?;
        let body = p.stmt_seq(&["end"])?;
        p.expect_kw("end")?;
        p.expect_op(";")?;
        routines.push(Routine {
            name,
            params,
            ret,
            labels,
            locals,
            body,
            sec,
            external: false,
        });
    }

    p.expect_kw("begin")?;
    let main = p.stmt_seq(&["end"])?;
    p.expect_kw("end")?;
    p.expect_op(".")?;
    if p.i < p.t.len() {
        return p.err("trailing tokens after `end.`");
    }

    if !p.clashes.is_empty() {
        return Err(p.clashes.join("\n"));
    }
    let const_vals = p.consts.clone();
    let type_map = p.types.clone();
    Ok(Program {
        consts,
        types,
        globals,
        routines,
        main_labels,
        main,
        const_vals,
        type_map,
    })
}
