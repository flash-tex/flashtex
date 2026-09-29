//! Rust code generator.
//!
//! Design points (DESIGN.md §4.1, §4.2):
//!   * every WEB identifier survives verbatim (only Rust keywords get a `_`
//!     suffix), and every routine and statement group carries its `// §NNNN`;
//!   * all globals live in one `struct Globals`; every array of plain-old-data
//!     elements is a region of one flat word space (`crate::arena::Arr` in the
//!     engine crate), whose writes pass the checkpoint write barrier, and the
//!     scalar globals are listed for the checkpoint spill (`visit_scalars`);
//!   * `goto` becomes labelled Rust blocks (forward jumps) and labelled loops
//!     (backward jumps), the web2js approach;
//!   * Pascal integer arithmetic wraps, so `+`/`-`/`*` become `wrapping_*`
//!     rather than relying on a build profile.

use crate::parse::{Expr, Program, Record, Routine, Stmt, Ty, S};
use crate::tangle::Tangled;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

/// Routines that `tex.ch` would supply in web2c, hand-written in `system.rs`.
const OVERRIDES: &[&str] = &[
    "a_open_in",
    "a_open_out",
    "b_open_in",
    "b_open_out",
    "w_open_in",
    "w_open_out",
    "a_close",
    "b_close",
    "w_close",
    "input_ln",
    "init_terminal",
    "a_make_name_string",
    "b_make_name_string",
    "w_make_name_string",
];

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "union",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield", "gen",
];

fn rid(n: &str) -> String {
    if RUST_KEYWORDS.contains(&n) {
        format!("{n}_")
    } else {
        n.to_string()
    }
}

// ---------------------------------------------------------------------------
// Bit-backed record layout
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Leaf {
    /// Dot path as written in Pascal, e.g. `hh.b0`.
    path: String,
    off: u32,
    bits: u32,
    /// Rust scalar the accessor uses.
    kind: Scalar,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Scalar {
    I32,
    F64,
    F32,
    U8,
    Bool,
}

#[derive(Clone, Debug)]
struct Packed {
    name: String,
    width: u32,
    /// Leaf accessors, in declaration order.
    leaves: Vec<Leaf>,
    /// Sub-records reachable as whole values: (path, type name, offset, width).
    subs: Vec<(String, String, u32, u32)>,
}

struct Layout {
    /// Record type name -> bit-packed layout, for the records that need one.
    packed: HashMap<String, Packed>,
    /// Record type name -> plain struct fields.
    plain: HashMap<String, Vec<(String, Ty)>>,
}

struct E<'a> {
    p: &'a Program,
    t: &'a Tangled,
    lay: Layout,
    /// Routine name -> (param types, return type).
    sigs: HashMap<String, (Vec<Ty>, Option<Ty>)>,
    /// Routine -> indices of its `var` parameters.
    by_ref: HashMap<String, Vec<usize>>,
    /// The `var` parameters of the routine being emitted: `&mut` in Rust, so
    /// every use is `(*name)`.
    deref: HashSet<String>,
    globals: HashMap<String, Ty>,
    /// Types of the outer-block constants.
    const_ty: HashMap<String, Ty>,
    /// Currently visible locals/params.
    locals: Vec<HashMap<String, Ty>>,
    /// Name of the routine being emitted, for function-result assignments.
    cur_fn: Option<String>,
    /// Reverse map from a label's numeric value to the WEB macro name.
    label_names: HashMap<i64, String>,
    /// Labels currently in scope: value -> (forward block?, backward loop?).
    scope: Vec<Sc>,
    last_sec: u32,
    dispatch_depth: u32,
    /// Counter for hoisting temporaries.
    tmp: std::cell::Cell<u32>,
    /// Real operands of a unary minus (see `ex`); must stay 0.
    unary_minus_on_real: std::cell::Cell<u32>,
    warnings: Vec<String>,
    /// Array type aliases emitted as fixed-size Rust arrays (`[T; N]`), so
    /// that they can be elements of the word space: name -> (N, element).
    fixed_alias: HashMap<String, (i64, Ty)>,
    /// Array globals that live in the word space: name -> element type.
    arena_globals: HashMap<String, Ty>,
}

/// How a global is stored.
enum GKind {
    /// A file or an array of files: host state, not part of the word space.
    File,
    /// A region of the word space: element type, capacity and initial
    /// length (Rust expressions of type `usize`).
    Arr { elem: Ty, cap: String, len: String },
    /// A scalar (including records and `packed array of char`): a field of
    /// `Globals`, spilled into the word space at a checkpoint.
    Scalar,
}

// ---------------------------------------------------------------------------

fn width_of(ty: &Ty, p: &Program, lay: &Layout) -> u32 {
    match resolve(ty, p) {
        Ty::Int => 32,
        Ty::Int64 => 64,
        Ty::Real => 64,
        Ty::Real32 => 32,
        Ty::Bool => 8,
        Ty::Char => 8,
        Ty::Sub(lo, hi) => {
            if lo >= 0 && hi <= 255 {
                8
            } else if lo >= 0 && hi <= 65535 {
                16
            } else {
                32
            }
        }
        Ty::Record(_) => {
            let n = type_name_of(ty).expect("anonymous record inside a record");
            lay.packed.get(&n).map(|q| q.width).unwrap_or(64)
        }
        other => panic!("web2rust: cannot pack {other:?} into a record"),
    }
}

fn is_real(t: &Ty) -> bool {
    matches!(t, Ty::Real | Ty::Real32)
}

fn type_name_of(ty: &Ty) -> Option<String> {
    match ty {
        Ty::Named(n) => Some(n.clone()),
        _ => None,
    }
}

fn resolve(ty: &Ty, p: &Program) -> Ty {
    match ty {
        Ty::Named(n) => match p.type_map.get(n) {
            Some(t) => resolve(t, p),
            None => ty.clone(),
        },
        other => other.clone(),
    }
}

fn scalar_of(ty: &Ty, p: &Program) -> Scalar {
    match resolve(ty, p) {
        Ty::Real => Scalar::F64,
        Ty::Real32 => Scalar::F32,
        Ty::Int64 => panic!("web2rust: a longinteger inside a packed record"),
        Ty::Bool => Scalar::Bool,
        Ty::Char => Scalar::U8,
        _ => Scalar::I32,
    }
}

/// Build the bit layout of one record type.
fn pack_record(name: &str, r: &Record, p: &Program, lay: &Layout) -> Packed {
    let mut leaves = vec![];
    let mut subs = vec![];
    let mut off = 0u32;
    let add =
        |f: &crate::parse::Field, off: &mut u32, leaves: &mut Vec<Leaf>, subs: &mut Vec<_>| {
            let w = width_of(&f.ty, p, lay);
            match resolve(&f.ty, p) {
                Ty::Record(sub) => {
                    let tn = type_name_of(&f.ty).expect("named sub-record");
                    subs.push((f.name.clone(), tn.clone(), *off, w));
                    // Flatten the sub-record's own leaves under `field.leaf`.
                    let subq = lay
                        .packed
                        .get(&tn)
                        .cloned()
                        .unwrap_or_else(|| pack_record(&tn, &sub, p, lay));
                    for l in &subq.leaves {
                        leaves.push(Leaf {
                            path: format!("{}.{}", f.name, l.path),
                            off: *off + l.off,
                            bits: l.bits,
                            kind: l.kind,
                        });
                    }
                }
                _ => leaves.push(Leaf {
                    path: f.name.clone(),
                    off: *off,
                    bits: w,
                    kind: scalar_of(&f.ty, p),
                }),
            }
            *off += w;
        };
    for f in &r.fields {
        add(f, &mut off, &mut leaves, &mut subs);
    }
    let base = off;
    let mut width = off;
    for (_tag, fs) in &r.variants {
        let mut o = base;
        for f in fs {
            add(f, &mut o, &mut leaves, &mut subs);
        }
        width = width.max(o);
    }
    Packed {
        name: name.to_string(),
        width,
        leaves,
        subs,
    }
}

fn build_layout(p: &Program) -> Layout {
    let mut lay = Layout {
        packed: HashMap::new(),
        plain: HashMap::new(),
    };
    // Which record types appear as a member of a variant part? Those must be
    // bit-representable, because they overlay other members.
    let mut in_variant: HashSet<String> = HashSet::new();
    for (_n, ty, _) in &p.types {
        if let Ty::Record(r) = ty {
            for (_t, fs) in &r.variants {
                for f in fs {
                    if let Some(tn) = type_name_of(&f.ty) {
                        if matches!(resolve(&f.ty, p), Ty::Record(_)) {
                            in_variant.insert(tn);
                        }
                    }
                }
            }
        }
    }
    // Innermost first, so nested layouts are already known.
    let mut todo: Vec<(&String, &Record)> = vec![];
    for (n, ty, _) in &p.types {
        if let Ty::Record(r) = ty {
            todo.push((n, r));
        }
    }
    let mut pass = 0;
    while pass < 4 {
        for (n, r) in &todo {
            let needs = !r.variants.is_empty() || in_variant.contains(*n);
            if !needs || lay.packed.contains_key(*n) {
                continue;
            }
            let ready = r
                .fields
                .iter()
                .chain(r.variants.iter().flat_map(|(_, fs)| fs.iter()))
                .all(|f| match type_name_of(&f.ty) {
                    Some(tn) if matches!(resolve(&f.ty, p), Ty::Record(_)) => {
                        lay.packed.contains_key(&tn)
                    }
                    _ => true,
                });
            if ready {
                let q = pack_record(n, r, p, &lay);
                lay.packed.insert((*n).clone(), q);
            }
        }
        pass += 1;
    }
    for (n, r) in &todo {
        if !lay.packed.contains_key(*n) {
            let fields = r
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.ty.clone()))
                .collect();
            lay.plain.insert((*n).clone(), fields);
        }
    }
    lay
}

// ---------------------------------------------------------------------------
// Rust type names
// ---------------------------------------------------------------------------

impl<'a> E<'a> {
    fn rust_ty(&self, ty: &Ty) -> String {
        match ty {
            Ty::Int => "i32".into(),
            Ty::Int64 => "i64".into(),
            Ty::Real => "f64".into(),
            Ty::Real32 => "f32".into(),
            Ty::Bool => "bool".into(),
            Ty::Char => "u8".into(),
            Ty::Sub(..) => "i32".into(),
            Ty::Named(n) => {
                if self.lay.packed.contains_key(n) || self.lay.plain.contains_key(n) {
                    rid(n)
                } else {
                    match resolve(ty, self.p) {
                        Ty::File(_) => self.file_ty(ty),
                        _ => rid(n),
                    }
                }
            }
            Ty::File(_) => self.file_ty(ty),
            Ty::Array { lo, hi, elem } => {
                if matches!(resolve(elem, self.p), Ty::Char) {
                    format!("[u8; {}]", hi - lo + 1)
                } else {
                    format!("Vec<{}>", self.rust_ty(elem))
                }
            }
            Ty::Ptr(elem) => format!("Vec<{}>", self.rust_ty(elem)),
            Ty::Record(_) => panic!("web2rust: anonymous record type"),
        }
    }

    fn file_ty(&self, ty: &Ty) -> String {
        let inner = match resolve(ty, self.p) {
            Ty::File(e) => *e,
            _ => unreachable!(),
        };
        match resolve(&inner, self.p) {
            Ty::Char => "crate::system::AlphaFile".into(),
            Ty::Record(_) => "crate::system::WordFile".into(),
            _ => "crate::system::ByteFile".into(),
        }
    }

    fn default_of(&self, ty: &Ty) -> String {
        if let Ty::Named(n) = ty {
            if let Some((len, elem)) = self.fixed_alias.get(n) {
                return format!("[{}; {len}]", self.default_of(elem));
            }
        }
        match resolve(ty, self.p) {
            Ty::Int | Ty::Sub(..) | Ty::Int64 => "0".into(),
            Ty::Real => "0.0".into(),
            Ty::Real32 => "0.0".into(),
            Ty::Bool => "false".into(),
            Ty::Char => "0".into(),
            Ty::File(_) => "Default::default()".into(),
            Ty::Ptr(_) => "Vec::new()".into(),
            Ty::Record(_) => format!("{}::default()", self.rust_ty(ty)),
            Ty::Array { lo, hi, elem } => {
                if matches!(resolve(&elem, self.p), Ty::Char) {
                    format!("[0u8; {}]", hi - lo + 1)
                } else if !matches!(resolve(&elem, self.p), Ty::File(_)) {
                    format!("vec![{}; {}]", self.default_of(&elem), hi - lo + 1)
                } else {
                    // Not `vec![x; n]`: file types are deliberately not Clone.
                    format!(
                        "(0..{}).map(|_| {}).collect::<Vec<_>>()",
                        hi - lo + 1,
                        self.default_of(&elem)
                    )
                }
            }
            _ => "Default::default()".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Type inference over expressions
// ---------------------------------------------------------------------------

impl<'a> E<'a> {
    fn lookup_local(&self, n: &str) -> Option<Ty> {
        self.locals.iter().rev().find_map(|m| m.get(n).cloned())
    }

    fn lookup(&self, n: &str) -> Option<Ty> {
        for m in self.locals.iter().rev() {
            if let Some(t) = m.get(n) {
                return Some(t.clone());
            }
        }
        if let Some(t) = self.globals.get(n) {
            return Some(t.clone());
        }
        if let Some(t) = self.const_ty.get(n) {
            return Some(t.clone());
        }
        if let Some(f) = self.cur_fn.as_ref() {
            if f == n {
                if let Some((_, Some(r))) = self.sigs.get(n) {
                    return Some(r.clone());
                }
            }
        }
        None
    }

    fn ty_of(&self, e: &Expr) -> Ty {
        match e {
            Expr::Int(_) => Ty::Int,
            Expr::Real(_) => Ty::Real,
            Expr::Str(s) if s.chars().count() == 1 => Ty::Char,
            Expr::Str(_) => Ty::Named("__string".into()),
            Expr::Var(n) => {
                if n == "true" || n == "false" {
                    return Ty::Bool;
                }
                if let Some(t) = self.lookup(n) {
                    return t;
                }
                if let Some((_, Some(r))) = self.sigs.get(n) {
                    return r.clone();
                }
                Ty::Int
            }
            Expr::Index(b, ix) => {
                let mut t = self.ty_of(b);
                for _ in ix {
                    t = match resolve(&t, self.p) {
                        Ty::Array { elem, .. } | Ty::Ptr(elem) => *elem,
                        _ => Ty::Int,
                    };
                }
                t
            }
            Expr::Field(b, f) => {
                let bt = self.ty_of(b);
                self.field_ty(&bt, f).unwrap_or(Ty::Int)
            }
            Expr::Deref(b) => match resolve(&self.ty_of(b), self.p) {
                Ty::File(e) => *e,
                _ => Ty::Int,
            },
            Expr::Call(f, args) => match f.as_str() {
                "abs" => self.ty_of(&args[0]),
                "odd" | "eof" | "eoln" => Ty::Bool,
                "chr" => Ty::Char,
                "ord" | "round" | "trunc" | "erstat" => Ty::Int,
                _ => match self.sigs.get(f) {
                    Some((_, Some(r))) => r.clone(),
                    _ => Ty::Int,
                },
            },
            Expr::Un("not", _) => Ty::Bool,
            Expr::Un(_, a) => self.ty_of(a),
            Expr::Bin(o, a, b) => match *o {
                "=" | "<>" | "<" | ">" | "<=" | ">=" | "and" | "or" => Ty::Bool,
                "/" => Ty::Real,
                "div" | "mod" => {
                    let (ta, tb) = (
                        resolve(&self.ty_of(a), self.p),
                        resolve(&self.ty_of(b), self.p),
                    );
                    if ta == Ty::Int64 || tb == Ty::Int64 {
                        Ty::Int64
                    } else {
                        Ty::Int
                    }
                }
                ":" => self.ty_of(a),
                _ => {
                    let (ta, tb) = (
                        resolve(&self.ty_of(a), self.p),
                        resolve(&self.ty_of(b), self.p),
                    );
                    if ta == Ty::Real || tb == Ty::Real {
                        Ty::Real
                    } else if ta == Ty::Int64 || tb == Ty::Int64 {
                        Ty::Int64
                    } else {
                        Ty::Int
                    }
                }
            },
        }
    }

    fn field_ty(&self, base: &Ty, f: &str) -> Option<Ty> {
        let n = match base {
            Ty::Named(n) => n.clone(),
            _ => return None,
        };
        if let Some(q) = self.lay.packed.get(&n) {
            for l in &q.leaves {
                if l.path == f {
                    return Some(match l.kind {
                        Scalar::F64 => Ty::Real,
                        Scalar::F32 => Ty::Real32,
                        Scalar::Bool => Ty::Bool,
                        Scalar::U8 => Ty::Char,
                        Scalar::I32 => Ty::Int,
                    });
                }
            }
            for (path, tn, _, _) in &q.subs {
                if path == f {
                    return Some(Ty::Named(tn.clone()));
                }
            }
            return None;
        }
        if let Some(fs) = self.lay.plain.get(&n) {
            for (fname, ft) in fs {
                if fname == f {
                    return Some(ft.clone());
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Expression emission
// ---------------------------------------------------------------------------

impl<'a> E<'a> {
    fn place(&self, n: &str) -> String {
        if self.const_ty.contains_key(n) && !self.globals.contains_key(n) {
            return rid(n);
        }
        for m in self.locals.iter().rev() {
            if m.contains_key(n) {
                if self.deref.contains(n) {
                    return format!("(*{})", rid(n));
                }
                return rid(n);
            }
        }
        if self.globals.contains_key(n) {
            return format!("self.{}", rid(n));
        }
        if self.cur_fn.as_deref() == Some(n) {
            return rid(n); // the function result variable
        }
        rid(n)
    }

    fn ex(&self, e: &Expr) -> String {
        match e {
            Expr::Int(v) => format!("{v}i32"),
            Expr::Real(s) => {
                if s.contains('.') {
                    format!("{s}f64")
                } else {
                    format!("{s}.0f64")
                }
            }
            Expr::Str(s) if s.chars().count() == 1 => byte_lit(s.chars().next().unwrap()),
            Expr::Str(s) => format!("{:?}", s),
            Expr::Var(n) => match n.as_str() {
                "true" => "true".into(),
                "false" => "false".into(),
                _ => {
                    if self.lookup(n).is_none() && self.sigs.contains_key(n) {
                        format!("self.{}()", rid(n))
                    } else if self.const_ty.contains_key(n) {
                        rid(n)
                    } else {
                        self.place(n)
                    }
                }
            },
            Expr::Index(b, ix) => {
                let bt = resolve(&self.ty_of(b), self.p);
                let lo = match &bt {
                    Ty::Array { lo, .. } => *lo,
                    _ => 0,
                };
                let inner = self.ex(&ix[0]);
                let idx = if lo == 0 {
                    format!("({inner}) as usize")
                } else if lo > 0 {
                    format!("(({inner}) - {lo}) as usize")
                } else {
                    format!("(({inner}) + {}) as usize", -lo)
                };
                let mut s = format!("{}[{}]", self.ex(b), idx);
                for extra in &ix[1..] {
                    s = format!("{s}[({}) as usize]", self.ex(extra));
                }
                s
            }
            Expr::Field(b, f) => {
                let bt = self.ty_of(b);
                if let Ty::Named(n) = &bt {
                    if let Some(q) = self.lay.packed.get(n) {
                        if q.leaves.iter().any(|l| l.path == *f)
                            || q.subs.iter().any(|(p, _, _, _)| p == f)
                        {
                            return format!("{}.{}()", self.ex(b), rid(f));
                        }
                    }
                }
                format!("{}.{}", self.ex(b), rid(f))
            }
            Expr::Deref(b) => format!("{}.buf", self.ex(b)),
            Expr::Call(f, args) => self.call_expr(f, args),
            Expr::Un("not", a) => format!("(!{})", self.cond(a)),
            Expr::Un("-", a) => {
                if is_real(&resolve(&self.ty_of(a), self.p)) {
                    // web2c prints a unary minus as `- (integer)`, which would
                    // truncate a real operand, unless the operand is a number
                    // (its lexer then reads a negative constant: tex.web's
                    // `-billion`). tex.web never negates any other real, and
                    // this makes sure pdftex.web does not either.
                    if !matches!(**a, Expr::Real(_)) {
                        self.unary_minus_on_real
                            .set(self.unary_minus_on_real.get() + 1);
                    }
                    format!("(-{})", self.ex(a))
                } else {
                    format!("({}).wrapping_neg()", self.ex(a))
                }
            }
            Expr::Un(o, a) => format!("({o}{})", self.ex(a)),
            Expr::Bin(o, a, b) => self.bin(o, a, b),
        }
    }

    fn bin(&self, o: &str, a: &Expr, b: &Expr) -> String {
        let (ta, tb) = (
            resolve(&self.ty_of(a), self.p),
            resolve(&self.ty_of(b), self.p),
        );
        let real = is_real(&ta) || is_real(&tb);
        // pdfTeX's C-style code mixes booleans, characters and 64-bit
        // integers with integers; C promotes them all to integers.
        let int_like =
            |t: &Ty| matches!(t, Ty::Int | Ty::Sub(..) | Ty::Char | Ty::Bool | Ty::Int64);
        if !real
            && int_like(&ta)
            && int_like(&tb)
            && ta != tb
            && !(matches!(ta, Ty::Int | Ty::Sub(..)) && matches!(tb, Ty::Int | Ty::Sub(..)))
            && !matches!(o, "and" | "or")
        {
            let wide = ta == Ty::Int64 || tb == Ty::Int64;
            let cv = |x: &Expr, t: &Ty| -> String {
                let v = self.ex(x);
                match (t, wide) {
                    (Ty::Int64, _) => v,
                    (Ty::Bool, true) | (Ty::Char, true) => format!("(({v}) as i64)"),
                    (_, true) => format!("(({v}) as i64)"),
                    (Ty::Bool, false) | (Ty::Char, false) => format!("(({v}) as i32)"),
                    _ => v,
                }
            };
            let (x, y) = (cv(a, &ta), cv(b, &tb));
            return match o {
                "=" => format!("({x} == {y})"),
                "<>" => format!("({x} != {y})"),
                "<" | ">" | "<=" | ">=" => format!("({x} {o} {y})"),
                "div" => format!("({x} / {y})"),
                "mod" => format!("({x} % {y})"),
                "+" => format!("({x}).wrapping_add({y})"),
                "-" => format!("({x}).wrapping_sub({y})"),
                "*" => format!("({x}).wrapping_mul({y})"),
                "/" => format!("(({x}) as f64 / ({y}) as f64)"),
                _ => panic!("web2rust: operator {o:?}"),
            };
        }
        // Arithmetic is always done in f64: a 32-bit `glue_ratio` is only a
        // storage width, exactly as a C `float` widens to `double` when it is
        // assigned to a `real` local (§186, §625).
        let ca = |x: &Expr| {
            let t = resolve(&self.ty_of(x), self.p);
            if real && t != Ty::Real {
                format!("(({}) as f64)", self.ex(x))
            } else {
                self.ex(x)
            }
        };
        match o {
            "=" => format!("({} == {})", ca(a), ca(b)),
            "<>" => format!("({} != {})", ca(a), ca(b)),
            "<" | ">" | "<=" | ">=" => format!("({} {o} {})", ca(a), ca(b)),
            "and" => format!("({} && {})", self.cond(a), self.cond(b)),
            "or" => format!("({} || {})", self.cond(a), self.cond(b)),
            "div" => format!("({} / {})", self.ex(a), self.ex(b)),
            "mod" => format!("({} % {})", self.ex(a), self.ex(b)),
            // Pascal's `/` is real division whatever the operands are.
            "/" => {
                let f = |x: &Expr| {
                    if resolve(&self.ty_of(x), self.p) == Ty::Real {
                        self.ex(x)
                    } else {
                        format!("(({}) as f64)", self.ex(x))
                    }
                };
                format!("({} / {})", f(a), f(b))
            }
            "+" | "-" | "*" => {
                if real {
                    format!("({} {o} {})", ca(a), ca(b))
                } else {
                    let m = match o {
                        "+" => "wrapping_add",
                        "-" => "wrapping_sub",
                        _ => "wrapping_mul",
                    };
                    format!("({}).{m}({})", self.ex(a), self.ex(b))
                }
            }
            _ => panic!("web2rust: operator {o:?}"),
        }
    }

    fn call_expr(&self, f: &str, args: &[Expr]) -> String {
        let a = |i: usize| self.ex(&args[i]);
        match f {
            "abs" => {
                if is_real(&resolve(&self.ty_of(&args[0]), self.p)) {
                    format!("({}).abs()", a(0))
                } else {
                    format!("({}).wrapping_abs()", a(0))
                }
            }
            "odd" => format!("((({}) % 2) != 0)", a(0)),
            "chr" => format!("(({}) as u8)", a(0)),
            "ord" => format!("(({}) as i32)", a(0)),
            "round" => format!("crate::system::pas_round({})", a(0)),
            "trunc" => format!("(({}) as i32)", a(0)),
            "eof" => format!("crate::system::eof(&{})", a(0)),
            "eoln" => format!("crate::system::eoln(&{})", a(0)),
            "erstat" => format!("crate::system::erstat(&{})", a(0)),
            _ => {
                if let Some(k) = self.by_ref_arg(f) {
                    return self.by_ref_call(f, args, k);
                }
                self.plain_call(f, args)
            }
        }
    }

    /// Does this expression call one of the translated routines? Such a call
    /// needs `&mut self`, so it cannot appear as an argument of another one.
    fn has_call(&self, e: &Expr) -> bool {
        match e {
            Expr::Call(f, args) => {
                self.sigs.contains_key(f) || args.iter().any(|x| self.has_call(x))
            }
            Expr::Var(n) => self.lookup(n).is_none() && self.sigs.contains_key(n),
            Expr::Index(b, ix) => self.has_call(b) || ix.iter().any(|x| self.has_call(x)),
            Expr::Field(b, _) | Expr::Deref(b) | Expr::Un(_, b) => self.has_call(b),
            Expr::Bin(_, a, b) => self.has_call(a) || self.has_call(b),
            _ => false,
        }
    }

    /// Does it read anything out of `Globals`? Then it must be evaluated before
    /// a `&mut self.<field>` borrow is taken for the assignment target.
    fn mentions_self(&self, e: &Expr) -> bool {
        match e {
            Expr::Call(..) => true,
            Expr::Var(n) => !self.is_local(n),
            Expr::Index(b, ix) => self.mentions_self(b) || ix.iter().any(|x| self.mentions_self(x)),
            Expr::Field(b, _) | Expr::Deref(b) | Expr::Un(_, b) => self.mentions_self(b),
            Expr::Bin(_, a, b) => self.mentions_self(a) || self.mentions_self(b),
            _ => false,
        }
    }

    fn is_local(&self, n: &str) -> bool {
        self.locals.iter().any(|m| m.contains_key(n))
    }

    /// Does `base`'s type have `path` (e.g. `hh.rh`) as a packed leaf?
    fn has_leaf_path(&self, base: &Expr, path: &str) -> bool {
        match self.ty_of(base) {
            Ty::Named(n) => match self.lay.packed.get(&n) {
                Some(q) => q.leaves.iter().any(|l| l.path == path),
                None => false,
            },
            _ => false,
        }
    }

    fn fresh(&self) -> u32 {
        let v = self.tmp.get();
        self.tmp.set(v + 1);
        v
    }

    /// `self.f(a, b)`, with the arguments hoisted into temporaries when one of
    /// them needs `&mut self` itself. Pascal evaluates arguments before the
    /// call anyway, so this changes nothing but the borrow checker's view.
    fn plain_call(&self, f: &str, args: &[Expr]) -> String {
        let want = |i: usize| {
            self.sigs
                .get(f)
                .and_then(|(ps, _)| ps.get(i).cloned())
                .unwrap_or(Ty::Int)
        };
        if args.iter().any(|x| self.has_call(x)) {
            let n = self.fresh();
            let mut s = String::from("{ ");
            for (i, x) in args.iter().enumerate() {
                let _ = write!(s, "let __a{n}_{i} = {}; ", self.coerce(x, &want(i)));
            }
            let _ = write!(s, "self.{}(", rid(f));
            for i in 0..args.len() {
                if i > 0 {
                    s.push_str(", ");
                }
                let _ = write!(s, "__a{n}_{i}");
            }
            s.push_str(") }");
            return s;
        }
        let mut s = format!("self.{}(", rid(f));
        for (i, x) in args.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&self.coerce(x, &want(i)));
        }
        s.push(')');
        s
    }

    /// A condition: C's `if (x)` on an integer is `x != 0`.
    fn cond(&self, e: &Expr) -> String {
        self.coerce(e, &Ty::Bool)
    }

    /// Emit `e` converted to `want` where Pascal's implicit widening applies.
    fn coerce(&self, e: &Expr, want: &Ty) -> String {
        let have = resolve(&self.ty_of(e), self.p);
        let w = resolve(want, self.p);
        let s = self.ex(e);
        match (&have, &w) {
            (Ty::Char, Ty::Int) | (Ty::Char, Ty::Sub(..)) => format!("(({s}) as i32)"),
            (Ty::Int, Ty::Char) | (Ty::Sub(..), Ty::Char) => format!("(({s}) as u8)"),
            (Ty::Int, Ty::Real) | (Ty::Sub(..), Ty::Real) => format!("(({s}) as f64)"),
            (Ty::Int, Ty::Real32) | (Ty::Sub(..), Ty::Real32) => format!("(({s}) as f32)"),
            (Ty::Real, Ty::Real32) => format!("(({s}) as f32)"),
            (Ty::Real32, Ty::Real) => format!("(({s}) as f64)"),
            (Ty::Real32, Ty::Int) | (Ty::Real32, Ty::Sub(..)) => format!("(({s}) as f64)"),
            // The C conversions web2c's output relies on (pdfTeX only):
            // bool <-> int, real -> int by truncation, int <-> longinteger.
            (Ty::Bool, Ty::Int) | (Ty::Bool, Ty::Sub(..)) => format!("(({s}) as i32)"),
            (Ty::Int, Ty::Bool) | (Ty::Sub(..), Ty::Bool) => format!("(({s}) != 0)"),
            (Ty::Real, Ty::Int) | (Ty::Real, Ty::Sub(..)) => format!("(({s}) as i32)"),
            (Ty::Int, Ty::Int64) | (Ty::Sub(..), Ty::Int64) | (Ty::Char, Ty::Int64) => {
                format!("(({s}) as i64)")
            }
            (Ty::Int64, Ty::Int) | (Ty::Int64, Ty::Sub(..)) => format!("(({s}) as i32)"),
            (Ty::Int64, Ty::Real) => format!("(({s}) as f64)"),
            _ => s,
        }
    }
}

fn byte_lit(c: char) -> String {
    match c {
        '\\' => "b'\\\\'".into(),
        '\'' => "b'\\''".into(),
        c if (c as u32) >= 0x20 && (c as u32) < 0x7f => format!("b'{c}'"),
        c => format!("{}u8", c as u32),
    }
}

// ---------------------------------------------------------------------------
// goto analysis
// ---------------------------------------------------------------------------

/// A label found in one statement list.
#[derive(Debug, Clone)]
struct Lab {
    val: i64,
    /// Index in the statement list.
    pos: usize,
    /// Smallest statement index containing a forward `goto` to it.
    fwd_from: Option<usize>,
    /// Largest statement index containing a backward `goto` to it.
    back_to: Option<usize>,
}

fn gotos_in(s: &S, out: &mut Vec<i64>) {
    match &s.st {
        Stmt::Goto(v) => out.push(*v),
        Stmt::Compound(v) => v.iter().for_each(|x| gotos_in(x, out)),
        Stmt::If(_, a, b) => {
            gotos_in(a, out);
            if let Some(b) = b {
                gotos_in(b, out);
            }
        }
        Stmt::While(_, b) | Stmt::For { body: b, .. } => gotos_in(b, out),
        Stmt::Repeat(v, _) => v.iter().for_each(|x| gotos_in(x, out)),
        Stmt::Case { arms, other, .. } => {
            for (_, a) in arms {
                gotos_in(a, out);
            }
            if let Some(o) = other {
                gotos_in(o, out);
            }
        }
        _ => {}
    }
}

fn analyse_labels(list: &[S]) -> Vec<Lab> {
    let mut labs: Vec<Lab> = vec![];
    for (i, s) in list.iter().enumerate() {
        for &v in &s.labels {
            labs.push(Lab {
                val: v,
                pos: i,
                fwd_from: None,
                back_to: None,
            });
        }
    }
    if labs.is_empty() {
        return labs;
    }
    let per: Vec<Vec<i64>> = list
        .iter()
        .map(|s| {
            let mut g = vec![];
            gotos_in(s, &mut g);
            g
        })
        .collect();
    for l in labs.iter_mut() {
        for (i, gs) in per.iter().enumerate() {
            if !gs.contains(&l.val) {
                continue;
            }
            if i < l.pos {
                l.fwd_from = Some(l.fwd_from.map_or(i, |x| x.min(i)));
            } else {
                l.back_to = Some(l.back_to.map_or(i, |x| x.max(i)));
            }
        }
    }
    labs
}

#[derive(Debug, Clone)]
struct Wrap {
    val: i64,
    /// `true` = labelled block for forward jumps, `false` = labelled loop.
    forward: bool,
    start: usize,
    end: usize,
}

/// A label scope that is currently open around the statement being emitted.
enum Sc {
    /// `'l_NAME_f: { ... }` — a forward jump is `break`.
    Block(i64),
    /// `'l_NAME_b: loop { ... }` — a backward jump is `continue`.
    Loop(i64),
    /// A guarded dispatch chain: `goto L` sets the guard and `continue`s.
    Dispatch {
        guard: String,
        lbl: String,
        seg: HashMap<i64, usize>,
    },
}

/// Can the labels of one statement list be expressed as nested Rust blocks and
/// loops? A forward block always spans `[0, pos)` and a backward loop always
/// spans `[pos, end)`, so the two families each nest internally; they only
/// clash when a backward label sits before a forward one.
fn nestable(labs: &[Lab]) -> bool {
    let fwd_max = labs
        .iter()
        .filter(|l| l.fwd_from.is_some())
        .map(|l| l.pos)
        .max();
    let back_min = labs
        .iter()
        .filter(|l| l.back_to.is_some())
        .map(|l| l.pos)
        .min();
    match (fwd_max, back_min) {
        (Some(f), Some(b)) => f <= b,
        _ => true,
    }
}

fn wraps_for(labs: &[Lab], n: usize) -> Vec<Wrap> {
    let mut w = vec![];
    for l in labs {
        // Widest safe regions: widening a forward block's start and a backward
        // loop's end never changes where a jump lands.
        if l.fwd_from.is_some() {
            w.push(Wrap {
                val: l.val,
                forward: true,
                start: 0,
                end: l.pos,
            });
        }
        if l.back_to.is_some() {
            w.push(Wrap {
                val: l.val,
                forward: false,
                start: l.pos,
                end: n,
            });
        }
    }
    // Outermost first: earliest start, then widest.
    w.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    w
}

// ---------------------------------------------------------------------------
// Statement emission
// ---------------------------------------------------------------------------

impl<'a> E<'a> {
    fn lname(&self, v: i64, forward: bool) -> String {
        let base = match self.label_names.get(&v) {
            Some(n) => n.clone(),
            None => format!("L{v}"),
        };
        format!("'l_{base}_{}", if forward { "f" } else { "b" })
    }

    fn seq(&mut self, list: &[S], o: &mut String, ind: usize) {
        let labs = analyse_labels(list);
        if labs.is_empty() {
            for s in list {
                self.stmt(s, o, ind);
            }
            return;
        }
        if nestable(&labs) {
            let wraps = wraps_for(&labs, list.len());
            self.region(list, &wraps, 0, list.len(), 0, o, ind);
        } else {
            self.dispatch(list, &labs, o, ind);
        }
    }

    /// The general `goto` translation, used where labelled blocks and loops
    /// cannot nest (in `tex.web` this is `main_control` and the other routines
    /// whose labels are jumped to both forwards and backwards).
    ///
    /// The statement list is cut into segments at each label. A guard variable
    /// holds the segment to resume at, and the chain of `if guard <= n` tests
    /// gives Pascal's fall-through from one segment to the next for free.
    fn dispatch(&mut self, list: &[S], labs: &[Lab], o: &mut String, ind: usize) {
        let mut cuts: Vec<usize> = labs.iter().map(|l| l.pos).collect();
        cuts.sort_unstable();
        cuts.dedup();
        if cuts.first() == Some(&0) {
            cuts.remove(0);
        }
        // Segment i covers [bounds[i], bounds[i+1]).
        let mut bounds = vec![0usize];
        bounds.extend(cuts.iter().copied());
        bounds.push(list.len());
        let mut seg: HashMap<i64, usize> = HashMap::new();
        for l in labs {
            let i = bounds.iter().position(|&b| b == l.pos).unwrap_or(0);
            seg.insert(l.val, i);
        }
        self.dispatch_depth += 1;
        let d = self.dispatch_depth;
        let guard = format!("__goto_{d}");
        let lbl = format!("'l_dispatch_{d}");
        let pad = "    ".repeat(ind);
        let p1 = "    ".repeat(ind + 1);
        let p2 = "    ".repeat(ind + 2);
        let names: Vec<String> = labs
            .iter()
            .map(|l| {
                self.label_names
                    .get(&l.val)
                    .cloned()
                    .unwrap_or_else(|| format!("L{}", l.val))
            })
            .collect();
        let _ = writeln!(o, "{pad}// goto labels: {}", names.join(", "));
        let _ = writeln!(o, "{pad}let mut {guard}: i32 = 0;");
        let _ = writeln!(o, "{pad}{lbl}: loop {{");
        self.scope.push(Sc::Dispatch {
            guard: guard.clone(),
            lbl: lbl.clone(),
            seg,
        });
        for i in 0..bounds.len() - 1 {
            let (a, b) = (bounds[i], bounds[i + 1]);
            let who: Vec<&str> = labs
                .iter()
                .zip(names.iter())
                .filter(|(l, _)| l.pos == a && a != 0)
                .map(|(_, n)| n.as_str())
                .collect();
            if who.is_empty() {
                let _ = writeln!(o, "{p1}if {guard} <= {i} {{");
            } else {
                let _ = writeln!(o, "{p1}if {guard} <= {i} {{ // {}", who.join(", "));
            }
            for k in a..b {
                self.stmt(&list[k], o, ind + 2);
            }
            let _ = writeln!(o, "{p1}}}");
            let _ = p2;
        }
        self.scope.pop();
        let _ = writeln!(o, "{p1}break {lbl};");
        let _ = writeln!(o, "{pad}}}");
        self.dispatch_depth -= 1;
    }

    /// Emit statements `[lo,hi)`, wrapping them in the labelled blocks and
    /// loops of `wraps` that fall entirely inside that range. `wi0` is the
    /// first wrap still available, so a wrap is never re-entered.
    #[allow(clippy::too_many_arguments)]
    fn region(
        &mut self,
        list: &[S],
        wraps: &[Wrap],
        lo: usize,
        hi: usize,
        wi0: usize,
        o: &mut String,
        ind: usize,
    ) {
        let mut i = lo;
        let mut wi = wi0;
        while i < hi {
            let mut pick = None;
            for k in wi..wraps.len() {
                let w = &wraps[k];
                if w.start < i || w.start >= hi || w.end > hi || w.end <= w.start {
                    continue;
                }
                pick = Some(k);
                break;
            }
            match pick {
                Some(k) => {
                    let w = wraps[k].clone();
                    for j in i..w.start {
                        self.stmt(&list[j], o, ind);
                    }
                    let nm = self.lname(w.val, w.forward);
                    let pad = "    ".repeat(ind);
                    if w.forward {
                        let _ = writeln!(o, "{pad}{nm}: {{");
                    } else {
                        let _ = writeln!(o, "{pad}{nm}: loop {{");
                    }
                    self.scope.push(if w.forward {
                        Sc::Block(w.val)
                    } else {
                        Sc::Loop(w.val)
                    });
                    self.region(list, wraps, w.start, w.end, k + 1, o, ind + 1);
                    self.scope.pop();
                    if !w.forward {
                        let _ = writeln!(o, "{}break {nm};", "    ".repeat(ind + 1));
                    }
                    let _ = writeln!(o, "{pad}}}");
                    i = w.end;
                    wi = k + 1;
                }
                None => {
                    for j in i..hi {
                        self.stmt(&list[j], o, ind);
                    }
                    i = hi;
                }
            }
        }
    }

    /// A single sub-statement (an `if` branch, a loop body, a `case` arm). It
    /// may itself carry a label, as in `while p<>null do reswitch: if ...`
    /// (§619), so it goes through `seq` as a one-element list.
    fn sub(&mut self, s: &S, o: &mut String, ind: usize) {
        if s.labels.is_empty() {
            self.stmt(s, o, ind);
        } else {
            self.seq(std::slice::from_ref(s), o, ind);
        }
    }

    fn stmt(&mut self, s: &S, o: &mut String, ind: usize) {
        let pad = "    ".repeat(ind);
        if s.sec != self.last_sec && s.sec != 0 {
            self.last_sec = s.sec;
            let _ = writeln!(o, "{pad}// §{}", s.sec);
        }
        match &s.st {
            Stmt::Empty => {}
            Stmt::Goto(v) => {
                let mut done = false;
                for sc in self.scope.iter().rev() {
                    match sc {
                        Sc::Block(x) if x == v => {
                            let _ = writeln!(o, "{pad}break {};", self.lname(*v, true));
                            done = true;
                        }
                        Sc::Loop(x) if x == v => {
                            let _ = writeln!(o, "{pad}continue {};", self.lname(*v, false));
                            done = true;
                        }
                        Sc::Dispatch { guard, lbl, seg } => {
                            if let Some(i) = seg.get(v) {
                                let _ = writeln!(o, "{pad}{{ {guard} = {i}; continue {lbl}; }}");
                                done = true;
                            }
                        }
                        _ => {}
                    }
                    if done {
                        break;
                    }
                }
                if !done {
                    // Non-local: the main program's labels, reached from inside
                    // a procedure (`jump_out`). §1332/§1335.
                    match *v {
                        9998 => {
                            let _ = writeln!(o, "{pad}crate::system::end_of_TEX(self);");
                        }
                        9999 => {
                            let _ = writeln!(o, "{pad}crate::system::final_end(self);");
                        }
                        // `return` (`goto exit`) in a routine with no `exit:`
                        // label: pdfTeX's own routines rely on web2c turning
                        // every `goto 10` into a C `return`
                        // (web2c-parser.y, `doreturn`), which is what `goto
                        // exit` means when `exit:` ends the routine.
                        v if self.label_names.get(&v).map(String::as_str) == Some("exit")
                            && self.cur_fn.is_some() =>
                        {
                            let f = self.cur_fn.clone().unwrap();
                            if self.sigs.get(&f).map(|s| s.1.is_some()) == Some(true) {
                                let _ = writeln!(o, "{pad}return {};", rid(&f));
                            } else {
                                let _ = writeln!(o, "{pad}return;");
                            }
                        }
                        other => {
                            self.warnings.push(format!(
                                "unresolved goto {other} in {}",
                                self.cur_fn.clone().unwrap_or_else(|| "main".into())
                            ));
                            let _ = writeln!(
                                o,
                                "{pad}unreachable!(\"web2rust: unresolved goto {other}\");"
                            );
                        }
                    }
                }
            }
            Stmt::Assign(lhs, rhs) => {
                let line = self.assign(lhs, rhs);
                let _ = writeln!(o, "{pad}{line}");
            }
            Stmt::Call(f, args) => {
                let line = self.call_stmt(f, args, ind);
                let _ = write!(o, "{line}");
            }
            Stmt::Compound(v) => {
                let _ = writeln!(o, "{pad}{{");
                self.seq(v, o, ind + 1);
                let _ = writeln!(o, "{pad}}}");
            }
            Stmt::If(c, a, b) => {
                let _ = writeln!(o, "{pad}if {} {{", self.cond(c));
                self.sub(a, o, ind + 1);
                match b {
                    Some(b) => {
                        let _ = writeln!(o, "{pad}}} else {{");
                        self.sub(b, o, ind + 1);
                        let _ = writeln!(o, "{pad}}}");
                    }
                    None => {
                        let _ = writeln!(o, "{pad}}}");
                    }
                }
            }
            Stmt::While(c, b) => {
                let _ = writeln!(o, "{pad}while {} {{", self.cond(c));
                self.sub(b, o, ind + 1);
                let _ = writeln!(o, "{pad}}}");
            }
            Stmt::Repeat(v, c) => {
                let _ = writeln!(o, "{pad}loop {{");
                self.seq(v, o, ind + 1);
                let _ = writeln!(
                    o,
                    "{}if {} {{ break; }}",
                    "    ".repeat(ind + 1),
                    self.cond(c)
                );
                let _ = writeln!(o, "{pad}}}");
            }
            Stmt::For {
                var,
                from,
                to,
                down,
                body,
            } => {
                let v = self.place(var);
                let vt = self.lookup(var).unwrap_or(Ty::Int);
                let lim = format!("__for_end_{}", ind);
                let _ = writeln!(o, "{pad}{{");
                let p1 = "    ".repeat(ind + 1);
                let _ = writeln!(o, "{p1}let {lim} = {};", self.coerce(to, &vt));
                let _ = writeln!(o, "{p1}{v} = {};", self.coerce(from, &vt));
                let cmp = if *down { ">=" } else { "<=" };
                let step = if *down {
                    "wrapping_sub"
                } else {
                    "wrapping_add"
                };
                let _ = writeln!(o, "{p1}while {v} {cmp} {lim} {{");
                self.sub(body, o, ind + 2);
                let _ = writeln!(o, "{}{v} = {v}.{step}(1);", "    ".repeat(ind + 2));
                let _ = writeln!(o, "{p1}}}");
                let _ = writeln!(o, "{pad}}}");
            }
            Stmt::Case { sel, arms, other } => {
                let _ = writeln!(o, "{pad}match {} {{", self.ex(sel));
                let p1 = "    ".repeat(ind + 1);
                for (ls, st) in arms {
                    let pats: Vec<String> = ls.iter().map(|v| v.to_string()).collect();
                    let _ = writeln!(o, "{p1}{} => {{", pats.join(" | "));
                    self.sub(st, o, ind + 2);
                    let _ = writeln!(o, "{p1}}}");
                }
                match other {
                    Some(st) => {
                        let _ = writeln!(o, "{p1}_ => {{");
                        self.sub(st, o, ind + 2);
                        let _ = writeln!(o, "{p1}}}");
                    }
                    None => {
                        let _ = writeln!(o, "{p1}_ => {{}}");
                    }
                }
                let _ = writeln!(o, "{pad}}}");
            }
        }
    }

    /// `x[i] = <expr reading x>` and `x[i].set_f(<expr reading x>)` both need
    /// the value computed before the target is borrowed mutably.
    fn hoist_rhs(&self, base: &Expr, rhs: &Expr, want: &Ty, setter: Option<&str>) -> String {
        let mut lets = String::new();
        let target = self.place_lets(base, &mut lets);
        let n = self.fresh();
        let v = self.coerce(rhs, want);
        match setter {
            Some(m) => format!("{{ {lets}let __v{n} = {v}; {target}.{m}(__v{n}); }}"),
            None => format!("{{ {lets}let __v{n} = {v}; {target} = __v{n}; }}"),
        }
    }

    /// Render an assignment target, hoisting any subscript that itself reads
    /// out of `Globals` (`mem[link(q)+1] := ...`), since the subscript has to
    /// be computed before the array is borrowed mutably.
    fn place_lets(&self, e: &Expr, lets: &mut String) -> String {
        match e {
            Expr::Index(b, ix) => {
                let base = self.place_lets(b, lets);
                let bt = resolve(&self.ty_of(b), self.p);
                let lo = match &bt {
                    Ty::Array { lo, .. } => *lo,
                    _ => 0,
                };
                let mut hoist = |x: &Expr| {
                    let t = self.ex(x);
                    if self.mentions_self(x) {
                        let n = self.fresh();
                        let _ = write!(lets, "let __ix{n} = {t}; ");
                        format!("__ix{n}")
                    } else {
                        t
                    }
                };
                let inner = hoist(&ix[0]);
                let idx = if lo == 0 {
                    format!("({inner}) as usize")
                } else if lo > 0 {
                    format!("(({inner}) - {lo}) as usize")
                } else {
                    format!("(({inner}) + {}) as usize", -lo)
                };
                let mut s = format!("{base}[{idx}]");
                for extra in &ix[1..] {
                    let e2 = hoist(extra);
                    s = format!("{s}[({e2}) as usize]");
                }
                s
            }
            Expr::Field(b, f) => format!("{}.{}", self.place_lets(b, lets), rid(f)),
            other => self.ex(other),
        }
    }

    /// A type named in an argument of `xmalloc_array`/`xrealloc_array`.
    fn named_ty(&self, e: &Expr) -> Ty {
        match e {
            Expr::Var(n) => match n.as_str() {
                "integer" => Ty::Int,
                "boolean" => Ty::Bool,
                "real" => Ty::Real,
                "char" => Ty::Char,
                _ => Ty::Named(n.clone()),
            },
            other => panic!("web2rust: expected a type name, got {other:?}"),
        }
    }

    fn assign(&self, lhs: &Expr, rhs: &Expr) -> String {
        // web2c's run-time arrays: `p := xmalloc_array(T, n)` allocates the
        // elements `0..n`, `p := xrealloc_array(p, T, n)` resizes to them and
        // keeps the old contents.
        if let (Expr::Var(g), Expr::Call(f, a)) = (lhs, rhs) {
            if self.arena_globals.contains_key(g) && self.lookup_local(g).is_none() {
                match f.as_str() {
                    "xmalloc_array" => {
                        return format!(
                            "self.{}.alloc_len((({}) as usize) + 1);",
                            rid(g),
                            self.ex(&a[1])
                        );
                    }
                    "xrealloc_array" => {
                        return format!(
                            "self.{}.resize_len((({}) as usize) + 1);",
                            rid(g),
                            self.ex(&a[2])
                        );
                    }
                    _ => {}
                }
            }
        }
        if let Expr::Call(f, a) = rhs {
            match f.as_str() {
                "xmalloc_array" => {
                    let d = self.default_of(&self.named_ty(&a[0]));
                    return format!(
                        "{} = vec![{d}; (({}) as usize) + 1];",
                        self.ex(lhs),
                        self.ex(&a[1])
                    );
                }
                "xrealloc_array" => {
                    let d = self.default_of(&self.named_ty(&a[1]));
                    let n = self.fresh();
                    return format!(
                        "{{ let __n{n} = (({}) as usize) + 1; {}.resize(__n{n}, {d}); }}",
                        self.ex(&a[2]),
                        self.ex(lhs)
                    );
                }
                _ => {}
            }
        }
        // Function result: `f := e` inside `function f`.
        if let Expr::Var(n) = lhs {
            if self.cur_fn.as_deref() == Some(n.as_str()) && self.lookup(n).is_none() {
                let want = self
                    .sigs
                    .get(n)
                    .and_then(|(_, r)| r.clone())
                    .unwrap_or(Ty::Int);
                return format!("{} = {};", rid(n), self.coerce(rhs, &want));
            }
        }
        // Setter for a bit-packed record field.
        if let Expr::Field(b, f) = lhs {
            let bt = self.ty_of(b);
            if let Ty::Named(n) = &bt {
                if let Some(q) = self.lay.packed.get(n) {
                    let leaf = q.leaves.iter().find(|l| l.path == *f);
                    let sub = q.subs.iter().find(|(p, _, _, _)| p == f);
                    // `mem[p].hh.rh := v` must go through the flattened setter
                    // on `mem[p]` below; `.hh()` would return a copy.
                    let two_level = match &**b {
                        Expr::Field(b2, f2) => self.has_leaf_path(b2, &format!("{f2}.{f}")),
                        _ => false,
                    };
                    if (leaf.is_some() || sub.is_some()) && !two_level {
                        let want = self.field_ty(&bt, f).unwrap_or(Ty::Int);
                        let m = format!("set_{}", rid(f));
                        if self.mentions_self(rhs) {
                            return self.hoist_rhs(b, rhs, &want, Some(&m));
                        }
                        let mut lets = String::new();
                        let target = self.place_lets(b, &mut lets);
                        let v = self.coerce(rhs, &want);
                        if lets.is_empty() {
                            return format!("{target}.{m}({v});");
                        }
                        return format!("{{ {lets}{target}.{m}({v}); }}");
                    }
                }
            }
            // `mem[p].hh.rh := v` — flattened setter on the outer record.
            if let Expr::Field(b2, f2) = &**b {
                let b2t = self.ty_of(b2);
                if let Ty::Named(n2) = &b2t {
                    if let Some(q) = self.lay.packed.get(n2) {
                        let path = format!("{f2}.{f}");
                        if let Some(l) = q.leaves.iter().find(|l| l.path == path) {
                            let want = match l.kind {
                                Scalar::F64 => Ty::Real,
                                Scalar::F32 => Ty::Real32,
                                Scalar::Bool => Ty::Bool,
                                Scalar::U8 => Ty::Char,
                                Scalar::I32 => Ty::Int,
                            };
                            let m = format!("set_{}_{}", rid(f2), rid(f));
                            if self.mentions_self(rhs) {
                                return self.hoist_rhs(b2, rhs, &want, Some(&m));
                            }
                            let mut lets = String::new();
                            let target = self.place_lets(b2, &mut lets);
                            let v = self.coerce(rhs, &want);
                            if lets.is_empty() {
                                return format!("{target}.{m}({v});");
                            }
                            return format!("{{ {lets}{target}.{m}({v}); }}");
                        }
                    }
                }
            }
        }
        let want = self.ty_of(lhs);
        // Whole-array assignment from a string: `months := 'JANFEB...'` (§536)
        // and `name_of_file := pool_name` (§51).
        if let Ty::Array { elem, .. } = &resolve(&want, self.p) {
            if matches!(resolve(elem, self.p), Ty::Char) {
                let src = match rhs {
                    Expr::Str(txt) => format!("{txt:?}"),
                    other => self.ex(other),
                };
                return format!("crate::system::copy_str(&mut {}, {src});", self.ex(lhs));
            }
        }
        if matches!(lhs, Expr::Index(..)) && self.mentions_self(rhs) {
            return self.hoist_rhs(lhs, rhs, &want, None);
        }
        format!("{} = {};", self.ex(lhs), self.coerce(rhs, &want))
    }

    /// A procedure call statement, including the Pascal I/O built-ins.
    fn call_stmt(&self, f: &str, args: &[Expr], ind: usize) -> String {
        let pad = "    ".repeat(ind);
        let sys = |n: &str, a: String| format!("{pad}crate::system::{n}({a});\n");
        match f {
            "get" => {
                return sys(
                    &self.file_fn("get", &args[0]),
                    format!("&mut {}", self.ex(&args[0])),
                )
            }
            "put" => {
                return sys(
                    &self.file_fn("put", &args[0]),
                    format!("&mut {}", self.ex(&args[0])),
                )
            }
            "reset" => {
                let mut a = format!("&mut {}", self.ex(&args[0]));
                for x in &args[1..] {
                    let _ = write!(a, ", {}", self.ex(x));
                }
                return sys(&self.file_fn("reset", &args[0]), a);
            }
            "rewrite" => {
                let mut a = format!("&mut {}", self.ex(&args[0]));
                for x in &args[1..] {
                    let _ = write!(a, ", {}", self.ex(x));
                }
                return sys(&self.file_fn("rewrite", &args[0]), a);
            }
            "close" => return sys("close", format!("&mut {}", self.ex(&args[0]))),
            "break" => return sys("break_out", format!("&mut {}", self.ex(&args[0]))),
            "break_in" => {
                return sys(
                    "break_in",
                    format!("&mut {}, {}", self.ex(&args[0]), self.ex(&args[1])),
                )
            }
            "read_ln" => return sys("read_ln", format!("&mut {}", self.ex(&args[0]))),
            "read" => {
                let mut s = String::new();
                for x in &args[1..] {
                    let _ = writeln!(
                        s,
                        "{pad}{} = crate::system::{}(&mut {});",
                        self.ex(x),
                        self.file_fn("read", &args[0]),
                        self.ex(&args[0])
                    );
                }
                return s;
            }
            "write" | "write_ln" => return self.write_call(f == "write_ln", args, ind),
            _ => {}
        }
        if let Some(k) = self.by_ref_arg(f) {
            let body = self.by_ref_call(f, args, k);
            return format!("{pad}{body};\n");
        }
        format!("{pad}{};\n", self.plain_call(f, args))
    }

    /// Indices of the `var` parameters, if the routine has any. In `tex.web`
    /// these are only the file-handling routines of §§27-28 and `input_ln`;
    /// e-TeX adds `reverse`.
    fn by_ref_arg(&self, f: &str) -> Option<Vec<usize>> {
        self.by_ref.get(f).cloned()
    }

    /// `input_ln(term_in, true)` cannot become `self.input_ln(&mut self.term_in,
    /// ...)`, so the file is moved out of `Globals`, passed by reference, and
    /// moved back. Every such routine lives in `system.rs`.
    fn by_ref_call(&self, f: &str, args: &[Expr], ks: Vec<usize>) -> String {
        let mut pre = String::new();
        let mut post = String::new();
        let mut call = format!("self.{}(", rid(f));
        for (i, x) in args.iter().enumerate() {
            if i > 0 {
                call.push_str(", ");
            }
            if ks.contains(&i) {
                let place = self.ex(x);
                let _ = write!(pre, "let mut __f{i} = ::core::mem::take(&mut {place}); ");
                let _ = write!(post, "{place} = __f{i}; ");
                let _ = write!(call, "&mut __f{i}");
            } else {
                let want = self
                    .sigs
                    .get(f)
                    .and_then(|(ps, _)| ps.get(i).cloned())
                    .unwrap_or(Ty::Int);
                let v = self.coerce(x, &want);
                if self.has_call(x) {
                    // Evaluated first, as Pascal does, so that `self` is not
                    // borrowed twice.
                    let _ = write!(pre, "let __a{i} = {v}; ");
                    let _ = write!(call, "__a{i}");
                } else {
                    call.push_str(&v);
                }
            }
        }
        call.push(')');
        format!("{{ {pre}let __r = {call}; {post}__r }}")
    }

    fn file_fn(&self, op: &str, f: &Expr) -> String {
        let t = self.file_ty(&self.ty_of(f));
        let suffix = if t.ends_with("AlphaFile") {
            "char"
        } else if t.ends_with("WordFile") {
            "word"
        } else {
            "byte"
        };
        format!("{op}_{suffix}")
    }

    fn write_call(&self, ln: bool, args: &[Expr], ind: usize) -> String {
        let pad = "    ".repeat(ind);
        let mut s = String::new();
        let file = self.ex(&args[0]);
        // On a binary file (`file of eight_bits`, `file of memory_word`)
        // Pascal's `write` stores one element, it does not format anything.
        let kind = self.file_ty(&self.ty_of(&args[0]));
        if !kind.ends_with("AlphaFile") {
            let f = if kind.ends_with("WordFile") {
                "write_word"
            } else {
                "write_byte"
            };
            for a in &args[1..] {
                let _ = writeln!(s, "{pad}{{");
                let _ = writeln!(s, "{pad}    let __w = {};", self.ex(a));
                let _ = writeln!(s, "{pad}    crate::system::{f}(&mut {file}, __w);");
                let _ = writeln!(s, "{pad}}}");
            }
            return s;
        }
        let _ = writeln!(s, "{pad}{{");
        let p1 = "    ".repeat(ind + 1);
        // Hoist the arguments so `&mut self.<file>` never overlaps them.
        let mut calls: Vec<String> = vec![];
        for (i, a) in args[1..].iter().enumerate() {
            let (val, width) = match a {
                Expr::Bin(":", v, w) => (&**v, Some(&**w)),
                other => (other, None),
            };
            let ty = resolve(&self.ty_of(val), self.p);
            let name = format!("__w{i}");
            match (&ty, val) {
                (_, Expr::Str(t)) if t.chars().count() != 1 => {
                    calls.push(format!("{p1}crate::system::wr_str(&mut {file}, {:?});", t));
                    continue;
                }
                (Ty::Char, _) => {
                    let _ = writeln!(s, "{p1}let {name} = {};", self.ex(val));
                    calls.push(format!("{p1}crate::system::wr_char(&mut {file}, {name});"));
                }
                (Ty::Real, _) | (Ty::Real32, _) => {
                    let _ = writeln!(s, "{p1}let {name} = {};", self.ex(val));
                    let w = width.map(|w| self.ex(w)).unwrap_or_else(|| "1".into());
                    calls.push(format!(
                        "{p1}crate::system::wr_real(&mut {file}, {name}, {w});"
                    ));
                }
                (Ty::Bool, _) => {
                    let _ = writeln!(s, "{p1}let {name} = {};", self.ex(val));
                    calls.push(format!("{p1}crate::system::wr_bool(&mut {file}, {name});"));
                }
                _ => {
                    let _ = writeln!(s, "{p1}let {name} = {};", self.ex(val));
                    let w = width.map(|w| self.ex(w)).unwrap_or_else(|| "1".into());
                    calls.push(format!(
                        "{p1}crate::system::wr_int(&mut {file}, {name}, {w});"
                    ));
                }
            }
        }
        for c in calls {
            let _ = writeln!(s, "{c}");
        }
        if ln {
            let _ = writeln!(s, "{p1}crate::system::wr_ln(&mut {file});");
        }
        let _ = writeln!(s, "{pad}}}");
        s
    }
}

// ---------------------------------------------------------------------------
// Whole-program emission
// ---------------------------------------------------------------------------

fn doc_of(t: &Tangled, sec: u32) -> String {
    let raw = t.comment.get(sec as usize).cloned().unwrap_or_default();
    let mut out = String::new();
    let mut n = 0;
    for line in raw.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with("@^") || l.starts_with("@.") || l.starts_with("@:") {
            continue;
        }
        let l = l.replace('|', "`");
        let _ = writeln!(out, "/// {l}");
        n += 1;
        if n >= 14 {
            let _ = writeln!(out, "/// ...");
            break;
        }
    }
    out
}

/// `sources`: the WEB file, then the change files applied to it, as given on
/// the command line (for the generated `mod.rs` header).
pub fn emit(
    p: &Program,
    t: &Tangled,
    out_dir: &Path,
    sources: &[String],
    arena_caps: &[(String, String)],
) -> Result<(), String> {
    let lay = build_layout(p);
    let mut sigs: HashMap<String, (Vec<Ty>, Option<Ty>)> = HashMap::new();
    for r in &p.routines {
        sigs.insert(
            r.name.clone(),
            (
                r.params.iter().map(|x| x.ty.clone()).collect(),
                r.ret.clone(),
            ),
        );
    }
    let mut by_ref: HashMap<String, Vec<usize>> = HashMap::new();
    for r in &p.routines {
        let v: Vec<usize> = (0..r.params.len())
            .filter(|&i| r.params[i].by_ref)
            .collect();
        if !v.is_empty() {
            by_ref.insert(r.name.clone(), v);
        }
    }
    let globals: HashMap<String, Ty> = p
        .globals
        .iter()
        .map(|g| (g.name.clone(), g.ty.clone()))
        .collect();
    let const_ty: HashMap<String, Ty> = p
        .consts
        .iter()
        .map(|(n, v, _)| {
            let t = match v {
                Expr::Str(_) => Ty::Named("__string".into()),
                _ => Ty::Int,
            };
            (n.clone(), t)
        })
        .collect();

    // Recover WEB's names for the `goto` labels from the numeric macro table.
    let mut label_names = HashMap::new();
    for (name, val) in &t.label_macros {
        label_names.entry(*val).or_insert_with(|| name.clone());
    }

    let mut e = E {
        p,
        t,
        lay,
        sigs,
        by_ref,
        globals,
        const_ty,
        locals: vec![],
        deref: HashSet::new(),
        cur_fn: None,
        label_names,
        scope: vec![],
        last_sec: 0,
        dispatch_depth: 0,
        tmp: std::cell::Cell::new(0),
        unary_minus_on_real: std::cell::Cell::new(0),
        warnings: vec![],
        fixed_alias: HashMap::new(),
        arena_globals: HashMap::new(),
    };

    // Array type aliases become `[T; N]`, so that an array of them can live
    // in the word space (pdfTeX's `char_used_array`).
    for (n, ty, _) in &p.types {
        if let Ty::Array { lo, hi, elem } = ty {
            let et = resolve(elem, p);
            if !matches!(et, Ty::Char | Ty::File(_)) && hi - lo < 4096 {
                e.fixed_alias
                    .insert(n.clone(), (hi - lo + 1, (**elem).clone()));
            }
        }
    }
    let allocs = collect_allocs(p);
    let mut kinds: Vec<GKind> = vec![];
    for g in &p.globals {
        let k = e.classify(&g.name, &g.ty, arena_caps, &allocs)?;
        if let GKind::Arr { elem, .. } = &k {
            e.arena_globals.insert(g.name.clone(), elem.clone());
        }
        kinds.push(k);
    }
    for (n, _) in arena_caps {
        if !e.arena_globals.contains_key(n) {
            return Err(format!("--arena-cap {n}: not an array global"));
        }
    }

    std::fs::create_dir_all(out_dir).map_err(|x| x.to_string())?;

    // ---- consts.rs -------------------------------------------------------
    let mut s = header("Constants from WEB's `@<Constants in the outer block@>`.");
    for (n, v, sec) in &p.consts {
        let _ = writeln!(s, "// §{sec}");
        match v {
            Expr::Str(txt) => {
                let _ = writeln!(s, "pub const {}: &str = {:?};", rid(n), txt);
            }
            other => {
                let _ = writeln!(s, "pub const {}: i32 = {};", rid(n), e.ex(other));
            }
        }
    }
    write_file(out_dir, "consts.rs", &s)?;

    // ---- types.rs --------------------------------------------------------
    let mut s = header("Types from WEB's `@<Types in the outer block@>`.");
    let _ = writeln!(s, "use super::consts::*;\n");
    for (n, ty, sec) in &p.types {
        let _ = writeln!(s, "// §{sec}");
        match ty {
            Ty::Record(_) => {
                if let Some(q) = e.lay.packed.get(n).cloned() {
                    emit_packed(&mut s, &q, &e);
                } else if let Some(fs) = e.lay.plain.get(n).cloned() {
                    let _ = writeln!(
                        s,
                        "#[derive(Clone, Copy, Default, PartialEq, Debug)]\npub struct {} {{",
                        rid(n)
                    );
                    for (fname, fty) in &fs {
                        let _ = writeln!(s, "    pub {}: {},", rid(fname), e.rust_ty(fty));
                    }
                    let _ = writeln!(s, "}}");
                }
            }
            other => {
                let rhs = match e.fixed_alias.get(n) {
                    Some((len, elem)) => format!("[{}; {len}]", e.rust_ty(elem)),
                    None => e.rust_ty(other),
                };
                let _ = writeln!(s, "pub type {} = {};", rid(n), rhs);
            }
        }
    }
    write_file(out_dir, "types.rs", &s)?;

    // ---- globals.rs ------------------------------------------------------
    let mut s = header("All WEB globals in one arena struct (DESIGN.md §4.2).");
    let _ = writeln!(s, "use super::consts::*;\nuse super::types::*;\n");
    let _ = writeln!(s, "pub struct Globals {{");
    for (g, k) in p.globals.iter().zip(&kinds) {
        let _ = writeln!(s, "    // §{}", g.sec);
        let ty = match k {
            GKind::Arr { elem, .. } => format!("crate::arena::Arr<{}>", e.rust_ty(elem)),
            _ => e.rust_ty(&g.ty),
        };
        let _ = writeln!(s, "    pub {}: {},", rid(&g.name), ty);
    }
    let _ = writeln!(
        s,
        "    /// The word space every `Arr` above lives in (crates/flashtex-engine/src/arena.rs)."
    );
    let _ = writeln!(s, "    pub arena: crate::arena::Arena,");
    let _ = writeln!(s, "}}\n");
    // The scalar region: every scalar global, in declaration order, then the
    // length of every growable array.
    let _ = writeln!(
        s,
        "/// Bytes of the scalar globals' region at the start of the word space."
    );
    let mut sum = String::from("pub const SCALAR_BYTES: usize = 0");
    for (g, k) in p.globals.iter().zip(&kinds) {
        match k {
            GKind::Scalar => {
                let _ = write!(sum, "\n    + crate::arena::slot::<{}>()", e.rust_ty(&g.ty));
            }
            GKind::Arr { len, .. } if len == "0" => {
                sum.push_str("\n    + crate::arena::slot::<usize>()");
            }
            _ => {}
        }
    }
    let _ = writeln!(s, "{sum};\n");
    let _ = writeln!(s, "impl Globals {{");
    let _ = writeln!(s, "    pub fn new() -> Box<Globals> {{");
    let _ = writeln!(
        s,
        "        let mut __plan = crate::arena::Plan::new(SCALAR_BYTES);"
    );
    for (g, k) in p.globals.iter().zip(&kinds) {
        if let GKind::Arr { elem, cap, .. } = k {
            let _ = writeln!(
                s,
                "        let __r_{} = __plan.reserve::<{}>({cap});",
                g.name,
                e.rust_ty(elem)
            );
        }
    }
    let _ = writeln!(s, "        let __arena = __plan.build();");
    let _ = writeln!(s, "        Box::new(Globals {{");
    for (g, k) in p.globals.iter().zip(&kinds) {
        let init = match k {
            GKind::Arr { len, .. } => format!("__arena.arr(__r_{}, {len})", g.name),
            _ => e.default_of(&g.ty),
        };
        let _ = writeln!(s, "            {}: {},", rid(&g.name), init);
    }
    let _ = writeln!(s, "            arena: __arena,");
    let _ = writeln!(s, "        }})");
    let _ = writeln!(s, "    }}\n");
    let _ = writeln!(
        s,
        "    /// Every scalar global, then every growable array's length, in the order\n    /// of `SCALAR_BYTES` (the checkpoint spill and fill)."
    );
    let _ = writeln!(
        s,
        "    pub fn visit_scalars<V: crate::arena::Visit>(&mut self, v: &mut V) {{"
    );
    for (g, k) in p.globals.iter().zip(&kinds) {
        match k {
            GKind::Scalar => {
                let _ = writeln!(s, "        v.pod(&mut self.{});", rid(&g.name));
            }
            GKind::Arr { len, .. } if len == "0" => {
                let _ = writeln!(s, "        v.arr_len(&mut self.{});", rid(&g.name));
            }
            _ => {}
        }
    }
    let _ = writeln!(s, "    }}\n");
    let _ = writeln!(
        s,
        "    /// Every file global, in declaration order (the checkpoint's host-state record)."
    );
    let _ = writeln!(
        s,
        "    pub fn visit_files<V: crate::system::FileVisit>(&mut self, v: &mut V) {{"
    );
    for (g, k) in p.globals.iter().zip(&kinds) {
        if let GKind::File = k {
            let (elem, many) = match resolve(&g.ty, p) {
                Ty::Array { elem, .. } | Ty::Ptr(elem) => (*elem, true),
                other => (other, false),
            };
            let method = match e.file_ty(&elem).as_str() {
                "crate::system::AlphaFile" => "alpha",
                "crate::system::ByteFile" => "byte",
                _ => "word",
            };
            if many {
                let _ = writeln!(
                    s,
                    "        for f in self.{}.iter_mut() {{ v.{method}(f); }}",
                    rid(&g.name)
                );
            } else {
                let _ = writeln!(s, "        v.{method}(&mut self.{});", rid(&g.name));
            }
        }
    }
    let _ = writeln!(s, "    }}");
    let _ = writeln!(s, "}}");
    write_file(out_dir, "globals.rs", &s)?;

    // ---- bodies ----------------------------------------------------------
    let todo: Vec<&Routine> = p
        .routines
        .iter()
        .filter(|r| !OVERRIDES.contains(&r.name.as_str()) && !r.external)
        .collect();
    let per_file = todo.len().div_ceil(8);
    let mut files = vec![];
    for (fi, chunk) in todo.chunks(per_file.max(1)).enumerate() {
        let name = format!("body_{fi}.rs");
        let mut s = header("Translated WEB procedures and functions.");
        let _ = writeln!(
            s,
            "use super::consts::*;\nuse super::globals::Globals;\nuse super::types::*;\n"
        );
        let _ = writeln!(s, "impl Globals {{");
        for r in chunk {
            emit_routine(&mut s, r, &mut e);
        }
        let _ = writeln!(s, "}}");
        write_file(out_dir, &name, &s)?;
        files.push(format!("body_{fi}"));
    }

    // ---- main_body.rs ----------------------------------------------------
    let mut s = header("The WEB main program, §1332 (`@p begin ... end.`).");
    let _ = writeln!(
        s,
        "use super::consts::*;\nuse super::globals::Globals;\nuse super::types::*;\n"
    );
    let _ = writeln!(s, "impl Globals {{");
    let _ = writeln!(s, "    /// The body of WEB's outer block.");
    let _ = writeln!(s, "    pub fn tex_body(&mut self) {{");
    e.cur_fn = None;
    e.locals = vec![HashMap::new()];
    e.scope.clear();
    let mut body = String::new();
    e.seq(&p.main, &mut body, 2);
    s.push_str(&body);
    let _ = writeln!(s, "    }}");
    let _ = writeln!(s, "}}");
    write_file(out_dir, "main_body.rs", &s)?;

    // ---- mod.rs ----------------------------------------------------------
    let mut what = format!("Generated by tools/web2rust from {}", sources[0]);
    for ch in &sources[1..] {
        what.push_str(&format!("\n// with the change file {ch}"));
    }
    what.push('.');
    let mut s = header(&what);
    let _ = writeln!(s, "pub mod consts;\npub mod globals;\npub mod types;");
    for f in &files {
        let _ = writeln!(s, "mod {f};");
    }
    let _ = writeln!(s, "mod main_body;\n");
    let _ = writeln!(s, "pub use globals::Globals;");
    write_file(out_dir, "mod.rs", &s)?;

    if e.unary_minus_on_real.get() > 0 {
        return Err(format!(
            "{} unary minus on a real operand: web2c's C truncates it to an integer",
            e.unary_minus_on_real.get()
        ));
    }
    if !e.warnings.is_empty() {
        for w in &e.warnings {
            eprintln!("web2rust: warning: {w}");
        }
    }
    eprintln!(
        "web2rust: emitted {} routines into {} files",
        todo.len(),
        files.len() + 4
    );
    Ok(())
}

impl<'a> E<'a> {
    /// Where global `name` of type `ty` lives (see `GKind`).
    fn classify(
        &self,
        name: &str,
        ty: &Ty,
        caps: &[(String, String)],
        allocs: &HashMap<String, Allocs>,
    ) -> Result<GKind, String> {
        let is_file = |t: &Ty| matches!(resolve(t, self.p), Ty::File(_));
        Ok(match resolve(ty, self.p) {
            Ty::File(_) => GKind::File,
            Ty::Array { elem, .. } if is_file(&elem) => GKind::File,
            Ty::Array { elem, .. } if matches!(resolve(&elem, self.p), Ty::Char) => GKind::Scalar,
            Ty::Array { lo, hi, elem } => {
                let n = (hi - lo + 1).to_string();
                GKind::Arr {
                    elem: *elem,
                    cap: n.clone(),
                    len: n,
                }
            }
            Ty::Ptr(elem) if is_file(&elem) => GKind::File,
            Ty::Ptr(elem) => {
                // Elements 0..max: the option if given; else the one
                // allocation's size, when it is a constant and the array is
                // never grown.
                let max = if let Some((_, e)) = caps.iter().find(|(n, _)| n == name) {
                    e.clone()
                } else {
                    match allocs.get(name) {
                        Some(a) if !a.grown && a.sizes.len() == 1 && self.is_const(&a.sizes[0]) => {
                            self.ex(&a.sizes[0])
                        }
                        _ => {
                            return Err(format!(
                                "array global `{name}` is allocated with a size that is not a \
                                 constant or is grown: give its largest index with \
                                 --arena-cap {name}=EXPR"
                            ))
                        }
                    }
                };
                GKind::Arr {
                    elem: *elem,
                    cap: format!("(({max}) as usize) + 1"),
                    len: "0".into(),
                }
            }
            _ => GKind::Scalar,
        })
    }

    /// Whether `e` is made of literals and outer-block constants only.
    fn is_const(&self, e: &Expr) -> bool {
        match e {
            Expr::Int(_) => true,
            Expr::Var(n) => self.const_ty.contains_key(n) && !self.globals.contains_key(n),
            Expr::Un(_, a) => self.is_const(a),
            Expr::Bin(_, a, b) => self.is_const(a) && self.is_const(b),
            _ => false,
        }
    }
}

/// How the program allocates a pointer global.
#[derive(Default)]
struct Allocs {
    /// The `n` of each `g := xmalloc_array(T, n)`.
    sizes: Vec<Expr>,
    /// Some `g := xrealloc_array(g, T, n)` exists.
    grown: bool,
}

fn collect_allocs(p: &Program) -> HashMap<String, Allocs> {
    fn walk(s: &S, out: &mut HashMap<String, Allocs>) {
        match &s.st {
            Stmt::Assign(Expr::Var(g), Expr::Call(f, a)) => match f.as_str() {
                "xmalloc_array" => out.entry(g.clone()).or_default().sizes.push(a[1].clone()),
                "xrealloc_array" => out.entry(g.clone()).or_default().grown = true,
                _ => {}
            },
            Stmt::Compound(v) | Stmt::Repeat(v, _) => v.iter().for_each(|x| walk(x, out)),
            Stmt::If(_, a, b) => {
                walk(a, out);
                if let Some(b) = b {
                    walk(b, out);
                }
            }
            Stmt::While(_, b) => walk(b, out),
            Stmt::For { body, .. } => walk(body, out),
            Stmt::Case { arms, other, .. } => {
                arms.iter().for_each(|(_, x)| walk(x, out));
                if let Some(o) = other {
                    walk(o, out);
                }
            }
            _ => {}
        }
    }
    let mut out = HashMap::new();
    for r in &p.routines {
        r.body.iter().for_each(|x| walk(x, &mut out));
    }
    p.main.iter().for_each(|x| walk(x, &mut out));
    out
}

fn emit_routine(s: &mut String, r: &Routine, e: &mut E) {
    let doc = doc_of(e.t, r.sec);
    for l in doc.lines() {
        let _ = writeln!(s, "    {l}");
    }
    let _ = writeln!(s, "    // §{}", r.sec);
    let mut sig = format!("    pub fn {}(&mut self", rid(&r.name));
    for pm in &r.params {
        let t = e.rust_ty(&pm.ty);
        if pm.by_ref {
            let _ = write!(sig, ", {}: &mut {t}", rid(&pm.name));
        } else {
            let _ = write!(sig, ", mut {}: {t}", rid(&pm.name));
        }
    }
    sig.push(')');
    if let Some(rt) = &r.ret {
        let _ = write!(sig, " -> {}", e.rust_ty(rt));
    }
    let _ = writeln!(s, "{sig} {{");

    let mut scope: HashMap<String, Ty> = HashMap::new();
    for pm in &r.params {
        scope.insert(pm.name.clone(), pm.ty.clone());
    }
    for l in &r.locals {
        scope.insert(l.name.clone(), l.ty.clone());
    }
    e.locals = vec![scope];
    e.deref = r
        .params
        .iter()
        .filter(|pm| pm.by_ref)
        .map(|pm| pm.name.clone())
        .collect();
    e.cur_fn = Some(r.name.clone());
    e.scope.clear();
    e.last_sec = r.sec;

    if let Some(rt) = &r.ret {
        let _ = writeln!(
            s,
            "        let mut {}: {} = {};",
            rid(&r.name),
            e.rust_ty(rt),
            e.default_of(rt)
        );
    }
    for l in &r.locals {
        let _ = writeln!(
            s,
            "        let mut {}: {} = {}; // §{}",
            rid(&l.name),
            e.rust_ty(&l.ty),
            e.default_of(&l.ty),
            l.sec
        );
    }
    let mut body = String::new();
    e.seq(&r.body, &mut body, 2);
    s.push_str(&body);
    if r.ret.is_some() {
        let _ = writeln!(s, "        {}", rid(&r.name));
    }
    let _ = writeln!(s, "    }}\n");
    e.locals.clear();
    e.deref.clear();
    e.cur_fn = None;
}

fn emit_packed(s: &mut String, q: &Packed, e: &E) {
    let backing = if q.width <= 32 { "u32" } else { "u64" };
    let has_f64 = q.leaves.iter().any(|l| l.kind == Scalar::F64);
    let note = if has_f64 {
        "///\n\
         /// A 64-bit `real` member is stored with its bits rotated by 32, so that\n\
         /// its sign and exponent land where an overlapping `integer` member reads\n\
         /// them. tex.web §186 is marked `@^system dependencies@>` and assumes a\n\
         /// nonzero real has absolute value 2^20 or more when taken as an integer,\n\
         /// which holds for a 32-bit real; the rotation makes it hold for a 64-bit\n\
         /// one. Representation only (DESIGN.md §4.2). With\n\
         /// `--scalar glue_ratio=f32` the question does not arise.\n"
    } else {
        ""
    };
    let _ = writeln!(
        s,
        "/// Bit-packed Pascal record ({} bits). The variant part of the WEB\n\
         /// declaration is a real overlay, so the fields alias exactly as they do\n\
         /// in `tex.web`.\n\
         {note}\
         #[derive(Clone, Copy, Default, PartialEq, Debug)]\n\
         #[repr(transparent)]\n\
         pub struct {}(pub {backing});",
        q.width,
        rid(&q.name)
    );
    let _ = writeln!(s, "impl {} {{", rid(&q.name));
    // Width-independent access, so the hand-written system layer does not have
    // to know whether the backing store is 32 or 64 bits wide.
    let _ = writeln!(s, "    #[inline(always)]");
    let _ = writeln!(s, "    pub fn to_bits(&self) -> u64 {{ self.0 as u64 }}");
    let _ = writeln!(s, "    #[inline(always)]");
    let _ = writeln!(
        s,
        "    pub fn from_bits(v: u64) -> Self {{ {}(v as {backing}) }}",
        rid(&q.name)
    );
    for l in &q.leaves {
        let m = l.path.replace('.', "_");
        let mask: u64 = if l.bits >= 64 {
            u64::MAX
        } else {
            (1u64 << l.bits) - 1
        };
        let (ret, get, set) = match l.kind {
            Scalar::F64 => (
                "f64".to_string(),
                // Stored rotated by 32 bits; see the note on the type.
                "f64::from_bits((self.0 as u64).rotate_left(32))".to_string(),
                String::new(),
            ),
            Scalar::F32 => (
                "f32".to_string(),
                format!("f32::from_bits(((self.0 >> {}) & {mask}) as u32)", l.off),
                String::new(),
            ),
            Scalar::Bool => (
                "bool".to_string(),
                format!("((self.0 >> {}) & {mask}) != 0", l.off),
                "0".into(),
            ),
            Scalar::U8 => (
                "u8".to_string(),
                format!("((self.0 >> {}) & {mask}) as u8", l.off),
                "0".into(),
            ),
            Scalar::I32 => (
                "i32".to_string(),
                if l.bits >= 32 {
                    format!("((self.0 >> {}) & {mask}) as u32 as i32", l.off)
                } else {
                    format!("((self.0 >> {}) & {mask}) as i32", l.off)
                },
                "0".into(),
            ),
        };
        let _ = set;
        let _ = writeln!(s, "    #[inline(always)]");
        let _ = writeln!(s, "    pub fn {m}(&self) -> {ret} {{ {get} }}");
        let _ = writeln!(s, "    #[inline(always)]");
        let setter = match l.kind {
            Scalar::F64 => format!("self.0 = v.to_bits().rotate_right(32) as {backing};"),
            Scalar::F32 => format!(
                "self.0 = (self.0 & !(({mask} as {backing}) << {off})) | (((v.to_bits() as {backing}) & ({mask} as {backing})) << {off});",
                off = l.off
            ),
            Scalar::Bool => format!(
                "self.0 = (self.0 & !({mask} << {off})) | (((v as {backing}) & {mask}) << {off});",
                off = l.off
            ),
            _ => format!(
                "self.0 = (self.0 & !(({mask} as {backing}) << {off})) | (((v as {backing}) & ({mask} as {backing})) << {off});",
                off = l.off
            ),
        };
        let _ = writeln!(s, "    pub fn set_{m}(&mut self, v: {ret}) {{ {setter} }}");
    }
    for (path, tn, off, w) in &q.subs {
        let sub_backing = e
            .lay
            .packed
            .get(tn)
            .map(|x| if x.width <= 32 { "u32" } else { "u64" })
            .unwrap_or("u32");
        let mask: u64 = if *w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
        let _ = writeln!(s, "    #[inline(always)]");
        let _ = writeln!(
            s,
            "    pub fn {}(&self) -> {} {{ {}(((self.0 >> {off}) & ({mask} as {backing})) as {sub_backing}) }}",
            rid(path),
            rid(tn),
            rid(tn)
        );
        let _ = writeln!(s, "    #[inline(always)]");
        let _ = writeln!(
            s,
            "    pub fn set_{}(&mut self, v: {}) {{ self.0 = (self.0 & !(({mask} as {backing}) << {off})) | (((v.0 as {backing}) & ({mask} as {backing})) << {off}); }}",
            rid(path),
            rid(tn)
        );
    }
    let _ = writeln!(s, "}}");
}

fn header(what: &str) -> String {
    format!(
        "// GENERATED FILE -- DO NOT EDIT.\n\
         // {what}\n\
         // Regenerate with the command in tools/web2rust/README.md.\n\
         #![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]\n\
         #![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]\n\
         #![allow(unused_imports, unused_labels, while_true)]\n\
         #![allow(dead_code, unreachable_code, unused_comparisons, clippy::all)]\n\n"
    )
}

fn write_file(dir: &Path, name: &str, body: &str) -> Result<(), String> {
    std::fs::write(dir.join(name), body).map_err(|e| format!("{name}: {e}"))
}
