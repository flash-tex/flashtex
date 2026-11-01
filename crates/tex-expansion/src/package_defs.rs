//! What a package or class file defines: recorded while the engine reads a
//! `.sty`/`.cls` through the package reader (`latex_packages.rs`), so an
//! editor can offer the file's commands, jump to their definitions and
//! name what a `\usepackage` brought in from the engine's own record
//! instead of a lexical scan of the file (`apps/mac/docs/package-editing.md`,
//! "what the engine should expose next").
//!
//! Only definitions made at a file's *outermost level* are recorded: the
//! defining command was read from the file's own text, not from a macro
//! body being expanded (an option's code run by `\ProcessOptions`, an
//! `\AtEndOfPackage` hook, a helper macro the file calls). A conditional or
//! a group around the statement does not matter, and neither does how the
//! name was built -- `\expandafter\newcommand\csname foo\endcsname` records
//! `foo`, which is what a lexical scan cannot see. Definitions in a package
//! the file `\RequirePackage`s are that package's own; the loading command's
//! span in [`OpenedFile::loaded_at`] links the chain.
//!
//! Nothing here runs unless a package file has been opened: the document
//! path pays one `Vec::is_empty` per control-sequence dispatch and one
//! `Option` check per definition. The records live in [`OpenedFile`], so an
//! incremental run that re-opens a file (same source id) replaces them and
//! one that reuses an earlier load keeps them (`incremental.rs`).

use crate::catcode::CatCode;
use crate::expand::{Engine, Input, Pending, Step};
use crate::latex_packages::{Declaration, OpenedFile};
use crate::macro_def::{BodyPart, ParamPart};
use crate::scopes::{Meaning, Primitive};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// What a [`PackageDefinition`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    /// A command: `\newcommand` and its LaTeX siblings, `\def`/`\edef`/
    /// `\gdef`/`\xdef`, `\let`, `\DeclareRobustCommand`, xparse's
    /// `\NewDocumentCommand` family.
    Macro,
    /// `\newenvironment`/`\renewenvironment`, `\NewDocumentEnvironment` & co.
    Environment,
    /// One of the three commands `\newif\iffoo` creates (`\iffoo`,
    /// `\footrue`, `\foofalse`); `\newboolean{foo}` makes the same three.
    Conditional,
    /// `\newcounter`.
    Counter,
    /// `\newlength`.
    Length,
    /// `\newcount`/`\newdimen`/`\newskip`/`\newtoks`.
    Register,
    /// `\newtheorem`.
    Theorem,
    /// `\DeclareMathOperator`.
    MathOperator,
}

impl DefinitionKind {
    /// The wire spelling (`metadata.packages[].definitions[].kind`).
    pub fn as_str(self) -> &'static str {
        match self {
            DefinitionKind::Macro => "macro",
            DefinitionKind::Environment => "environment",
            DefinitionKind::Conditional => "conditional",
            DefinitionKind::Counter => "counter",
            DefinitionKind::Length => "length",
            DefinitionKind::Register => "register",
            DefinitionKind::Theorem => "theorem",
            DefinitionKind::MathOperator => "math_operator",
        }
    }
}

/// One definition a package or class file made at its outermost level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDefinition {
    /// The control-sequence name without its escape character (`hello`),
    /// the environment, counter, length or theorem name, or the active
    /// character.
    pub name: String,
    pub kind: DefinitionKind,
    /// The defining command without its escape character: `newcommand`,
    /// `def`, `let`, `NewDocumentCommand`, `DeclareOption` never (options
    /// are [`OpenedFile::options`]).
    pub definer: String,
    /// Parameters the definition takes (`\let` reports the copied macro's).
    pub arity: u8,
    /// The `[default]` of a LaTeX definer's first, optional parameter.
    pub optional_default: Option<String>,
    /// The parameter shape as written: `[2][x]` for `\newcommand` & co.,
    /// the parameter text of a `\def` (`#1#2`, `[#1]#2`, `#1\stop`), an
    /// xparse argument specification (`O{x} m`), empty when there is none.
    pub signature: String,
    /// The whole defining statement in the file, `\global`/`\long`
    /// prefixes included, from the definer to the end of its last argument.
    pub span: Span,
    /// The name had a meaning before: `\renewcommand`, a `\def` over a
    /// taken name, a `\let` that replaces an existing command.
    pub overrides: bool,
    /// `\newtheorem`: the heading text, and the counter it is numbered
    /// within (`[section]`) or shares (`[thm]` before the heading).
    pub title: Option<String>,
    pub within: Option<String>,
}

impl PackageDefinition {
    /// A record with the name and kind filled in; the definer and the
    /// span are the capture's ([`Engine::note_definition`]).
    pub(crate) fn new(name: impl Into<String>, kind: DefinitionKind) -> Self {
        PackageDefinition {
            name: name.into(),
            kind,
            definer: String::new(),
            arity: 0,
            optional_default: None,
            signature: String::new(),
            span: Span::synthetic(),
            overrides: false,
            title: None,
            within: None,
        }
    }
}

/// `\ProvidesPackage{name}[date version description]` /
/// `\ProvidesClass` as the file declared itself. The bracket is LaTeX's
/// free-form `\ver@` text; its conventional `YYYY/MM/DD vX.Y description`
/// layout is split when it follows the convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provides {
    pub name: String,
    pub date: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// The declaration, from `\ProvidesPackage` to the end of the bracket.
    pub span: Span,
}

/// `\DeclareOption{name}{code}` at a file's outermost level; `*` for
/// `\DeclareOption*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredOption {
    pub name: String,
    /// The declaration, `{code}` included.
    pub span: Span,
}

/// A definer read from a package file whose definitions are being recorded
/// (`Engine::capture`): where the statement started, and what the
/// definer's implementation reported.
#[derive(Debug)]
pub(crate) struct DefinitionCapture {
    /// Index into `Engine::opened_packages`.
    file: usize,
    source_id: u32,
    start: u32,
    definer: String,
    records: Vec<PackageDefinition>,
    /// The statement's end when the definer knows it better than
    /// [`Engine::statement_end`] (`\newtheorem` hands every argument back).
    end: Option<u32>,
}

/// Commands modelled as kernel or host macros (or not modelled at all)
/// whose statement is recorded by reading its arguments and handing them
/// back unread. Each entry: the name, and its argument shape.
const RECORDED_MACROS: &[(&str, Shape)] = &[
    ("DeclareOption", Shape::Option),
    ("DeclareMathOperator", Shape::MathOperator),
    ("NewDocumentCommand", Shape::DocumentCommand),
    ("RenewDocumentCommand", Shape::DocumentCommand),
    ("ProvideDocumentCommand", Shape::DocumentCommand),
    ("DeclareDocumentCommand", Shape::DocumentCommand),
    ("NewExpandableDocumentCommand", Shape::DocumentCommand),
    ("RenewExpandableDocumentCommand", Shape::DocumentCommand),
    ("ProvideExpandableDocumentCommand", Shape::DocumentCommand),
    ("DeclareExpandableDocumentCommand", Shape::DocumentCommand),
    ("NewDocumentEnvironment", Shape::DocumentEnvironment),
    ("RenewDocumentEnvironment", Shape::DocumentEnvironment),
    ("ProvideDocumentEnvironment", Shape::DocumentEnvironment),
    ("DeclareDocumentEnvironment", Shape::DocumentEnvironment),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// `\DeclareOption{name}{code}` / `\DeclareOption*{code}`.
    Option,
    /// `\DeclareMathOperator[*]\cmd{text}` (amsopn.dtx).
    MathOperator,
    /// `\NewDocumentCommand\cmd{spec}{code}` (xparse; `{\cmd}` accepted).
    DocumentCommand,
    /// `\NewDocumentEnvironment{name}{spec}{begin}{end}`.
    DocumentEnvironment,
}

impl Engine {
    /// Every definition recorded so far, with the file that made it, in
    /// loading and definition order (see [`OpenedFile::definitions`]).
    pub fn package_definitions(&self) -> impl Iterator<Item = (&OpenedFile, &PackageDefinition)> {
        self.opened_packages.iter().flat_map(|file| file.definitions.iter().map(move |d| (file, d)))
    }

    /// The opened package file `tok` was read from at its outermost level:
    /// the token came straight from a source text (no invocation origin)
    /// and that text is a package file's. `None` on the document path and
    /// for every token a macro body produced.
    fn recording_file(&self, tok: &Token) -> Option<usize> {
        if self.last_origin.is_some() || tok.span.is_synthetic() {
            return None;
        }
        self.opened_packages.iter().rposition(|file| file.source_id == tok.span.source_id)
    }

    /// Called by `dispatch` for every control sequence once a package
    /// file has been opened. A definer read from a package file at its
    /// outermost level runs under a capture and the step it produced is
    /// returned; a recorded macro (`RECORDED_MACROS`) or a `\Provides…`
    /// declaration has its arguments read, recorded and handed back, and
    /// `None` lets the ordinary dispatch run it.
    pub(crate) fn record_package_definer(&mut self, tok: &Token, meaning: &Meaning) -> Option<Step> {
        let file = self.recording_file(tok)?;
        match meaning {
            // The chained meaning is dispatched again with the same token:
            // recorded there, once.
            Meaning::Let(_) => None,
            Meaning::Primitive(p) if is_recorded_primitive(*p) => Some(self.run_captured(tok.clone(), *p, file)),
            Meaning::Primitive(Primitive::PreambleDeclaration(Declaration::ProvidesPackage | Declaration::ProvidesClass)) => {
                self.record_provides(tok, file);
                None
            }
            Meaning::Primitive(_) => None,
            _ => {
                let name = match &tok.kind {
                    TokenKind::ControlSequence(name) => name.as_str(),
                    _ => return None,
                };
                let shape = RECORDED_MACROS.iter().find(|(n, _)| *n == name).map(|(_, shape)| *shape)?;
                self.record_macro_statement(tok, file, shape);
                None
            }
        }
    }

    /// Run the primitive definer `p` with a capture open, then give every
    /// record it made the statement's span and file it.
    fn run_captured(&mut self, tok: Token, p: Primitive, file: usize) -> Step {
        let source_id = tok.span.source_id;
        let start = self.statement_start(&tok);
        let capture = DefinitionCapture { file, source_id, start, definer: primitive_definer(p).to_string(), records: Vec::new(), end: None };
        let previous = self.capture.replace(capture);
        let step = self.handle_primitive(tok, p);
        let capture = std::mem::replace(&mut self.capture, previous).expect("the capture opened above");
        self.file_records(capture);
        step
    }

    /// Where the statement begins: at a `\global`/`\long`/`\outer`/
    /// `\protected` prefix read from the same file, else at the definer.
    fn statement_start(&self, tok: &Token) -> u32 {
        match self.prefix_start {
            Some(prefix) if prefix.source_id == tok.span.source_id && prefix.start <= tok.span.start => prefix.start,
            _ => tok.span.start,
        }
    }

    /// The byte where the statement just read from `source_id` ends: the
    /// end of the last token read from that text -- or, when the definer
    /// looked further and handed tokens back (`peek_one` after
    /// `\newcounter{x}`, looking for `[`; `skip_spaces`), the start of the
    /// first handed-back token, with the spaces and comments between the
    /// statement and it trimmed away.
    fn statement_end(&self, source_id: u32) -> Option<u32> {
        let last = self.last_text_span.filter(|s| s.source_id == source_id)?;
        let mut end = last.end;
        let mut text: Option<&str> = None;
        for input in self.sources.iter().rev() {
            match input {
                Input::Toks(toks, pos) => {
                    for p in &toks[*pos..] {
                        if p.origin.is_none() && p.tok.span.source_id == source_id && !p.tok.span.is_synthetic() {
                            end = end.min(p.tok.span.start);
                        }
                    }
                }
                Input::Text(lexer) => {
                    if lexer.source_id() == source_id {
                        text = Some(lexer.text());
                    }
                    break;
                }
            }
        }
        if end < last.end {
            if let Some(text) = text {
                end = trim_gap(text, end as usize) as u32;
            }
        }
        Some(end)
    }

    fn file_records(&mut self, capture: DefinitionCapture) {
        if capture.records.is_empty() {
            return;
        }
        let end = capture.end.or_else(|| self.statement_end(capture.source_id)).unwrap_or(capture.start).max(capture.start);
        let span = Span::new(capture.source_id, capture.start, end);
        let file = &mut self.opened_packages[capture.file];
        for mut record in capture.records {
            record.definer = capture.definer.clone();
            record.span = span;
            file.definitions.push(record);
        }
    }

    /// A definer reports what it defined. Nothing happens outside a
    /// capture (the document path, and every definition a macro body
    /// makes).
    pub(crate) fn note_definition(&mut self, record: PackageDefinition) {
        if let Some(capture) = &mut self.capture {
            capture.records.push(record);
        }
    }

    /// [`Engine::note_definition`] for a definer that hands its arguments
    /// back to the input and so knows where its statement ended.
    pub(crate) fn note_definition_ending_at(&mut self, record: PackageDefinition, end: Option<u32>) {
        if let Some(capture) = &mut self.capture {
            capture.records.push(record);
            capture.end = end;
        }
    }

    /// The shape of a macro meaning as a LaTeX author sees it: `(arity,
    /// signature, optional default)`. A command `\newcommand` gave an
    /// optional argument is `\@protected@testopt\cmd\\cmd{default}`
    /// (ltdefns.dtx `\@xargdef`) with the parameters in `\\cmd`, so its
    /// shape is that inner macro's, written `[n][default]`; any other
    /// macro reports its own parameter text. Nothing for a non-macro.
    pub(crate) fn latex_shape(&self, meaning: &Meaning) -> (u8, String, Option<String>) {
        let mut copied = meaning;
        while let Meaning::Let(inner) = copied {
            copied = inner.as_ref();
        }
        let Meaning::Macro(def) = copied else {
            return (0, String::new(), None);
        };
        let literal = |part: &BodyPart| match part {
            BodyPart::Literal(tok) => Some(tok.clone()),
            BodyPart::Param(_) => None,
        };
        let body: Option<Vec<Token>> = def.body.iter().map(literal).collect();
        if let Some(body) = body.filter(|_| def.params.is_empty()) {
            if let [testopt, _target, inner, open, default @ .., close] = body.as_slice() {
                if testopt.is_cs("@protected@testopt")
                    && matches!(open.kind, TokenKind::Char(_, CatCode::BeginGroup))
                    && matches!(close.kind, TokenKind::Char(_, CatCode::EndGroup))
                {
                    if let TokenKind::ControlSequence(inner) = &inner.kind {
                        if let Meaning::Macro(inner) = self.st.scopes.meaning(inner) {
                            let default = self.detokenize(default);
                            return (inner.arity, format!("[{}][{default}]", inner.arity), Some(default));
                        }
                    }
                }
            }
        }
        (def.arity, self.param_text(&def.params), None)
    }

    /// The parameter text of a `\def` as it was written, for
    /// [`PackageDefinition::signature`].
    pub(crate) fn param_text(&self, params: &[ParamPart]) -> String {
        let mut s = String::new();
        for part in params {
            match part {
                ParamPart::Literal(tok) => s.push_str(&self.detokenize(std::slice::from_ref(tok))),
                ParamPart::Param(n) => {
                    s.push('#');
                    s.push_str(&n.to_string());
                }
            }
        }
        s.trim_end().to_string()
    }

    /// `\ProvidesPackage{name}[text]`: read both arguments unexpanded,
    /// record them, and hand them back for the kernel macro.
    fn record_provides(&mut self, tok: &Token, file: usize) {
        let mut taken: Vec<Pending> = Vec::new();
        let Some(name) = self.take_braced(&mut taken) else {
            self.push_pending_as_read(taken);
            return;
        };
        let bracket = self.take_bracketed(&mut taken);
        let name = self.pending_text(&name);
        let (date, version, description) = match bracket {
            Some(inner) => split_provides_text(&self.pending_text(&inner)),
            None => (None, None, None),
        };
        let end = self.statement_end(tok.span.source_id).unwrap_or(tok.span.end);
        let span = Span::new(tok.span.source_id, tok.span.start, end.max(tok.span.end));
        self.push_pending_as_read(taken);
        self.opened_packages[file].provides = Some(Provides { name, date, version, description, span });
    }

    /// Read the arguments of a recorded macro (`RECORDED_MACROS`) without
    /// expanding them, record the statement, and hand every token back
    /// exactly as read (`push_pending_as_read`), so the macro then runs
    /// on the same input it would have read.
    fn record_macro_statement(&mut self, tok: &Token, file: usize, shape: Shape) {
        let mut taken: Vec<Pending> = Vec::new();
        let star = self.take_star(&mut taken);
        let recorded = match shape {
            Shape::Option => {
                let name = if star { Some("*".to_string()) } else { self.take_braced(&mut taken).map(|inner| self.pending_text(&inner)) };
                let code = name.is_some() && self.take_braced(&mut taken).is_some();
                match name {
                    Some(name) if code => {
                        let end = self.statement_end(tok.span.source_id).unwrap_or(tok.span.end).max(tok.span.end);
                        let span = Span::new(tok.span.source_id, tok.span.start, end);
                        self.opened_packages[file].options.push(DeclaredOption { name, span });
                    }
                    _ => {}
                }
                None
            }
            Shape::MathOperator => {
                let name = self.take_cs(&mut taken);
                let text = self.take_braced(&mut taken);
                match (name, text) {
                    (Some(name), Some(text)) => {
                        let mut record = PackageDefinition::new(name.clone(), DefinitionKind::MathOperator);
                        record.overrides = self.st.scopes.is_defined(&name);
                        record.signature = format!("{}{{{}}}", if star { "*" } else { "" }, self.pending_text(&text));
                        Some(record)
                    }
                    _ => None,
                }
            }
            Shape::DocumentCommand => {
                let name = self.take_cs(&mut taken);
                let spec = name.is_some().then(|| self.take_braced(&mut taken)).flatten();
                let body = spec.is_some().then(|| self.take_braced(&mut taken)).flatten();
                match (name, spec, body) {
                    (Some(name), Some(spec), Some(_)) => {
                        let mut record = PackageDefinition::new(name.clone(), DefinitionKind::Macro);
                        record.overrides = self.st.scopes.is_defined(&name);
                        record.signature = self.pending_text(&spec).trim().to_string();
                        record.arity = xparse_arity(&record.signature);
                        Some(record)
                    }
                    _ => None,
                }
            }
            Shape::DocumentEnvironment => {
                let name = self.take_braced(&mut taken).map(|inner| self.pending_text(&inner).trim().to_string());
                let spec = name.is_some().then(|| self.take_braced(&mut taken)).flatten();
                let begin = spec.is_some().then(|| self.take_braced(&mut taken)).flatten();
                let end = begin.is_some().then(|| self.take_braced(&mut taken)).flatten();
                match (name, spec, end) {
                    (Some(name), Some(spec), Some(_)) => {
                        let mut record = PackageDefinition::new(name.clone(), DefinitionKind::Environment);
                        record.overrides = self.st.scopes.is_defined(&name);
                        record.signature = self.pending_text(&spec).trim().to_string();
                        record.arity = xparse_arity(&record.signature);
                        Some(record)
                    }
                    _ => None,
                }
            }
        };
        if let Some(mut record) = recorded {
            let end = self.statement_end(tok.span.source_id).unwrap_or(tok.span.end).max(tok.span.end);
            record.span = Span::new(tok.span.source_id, tok.span.start, end);
            record.definer = match &tok.kind {
                TokenKind::ControlSequence(name) => name.clone(),
                _ => String::new(),
            };
            self.opened_packages[file].definitions.push(record);
        }
        self.push_pending_as_read(taken);
    }

    /// An optional `*` after optional spaces, appended to `taken` with the
    /// spaces; anything else is handed back.
    fn take_star(&mut self, taken: &mut Vec<Pending>) -> bool {
        let mut spaces = Vec::new();
        loop {
            let Some(p) = self.next_raw() else {
                self.push_pending_as_read(spaces);
                return false;
            };
            match &p.tok.kind {
                TokenKind::Char(_, CatCode::Space) => spaces.push(p),
                TokenKind::Char('*', CatCode::Other) => {
                    taken.extend(spaces);
                    taken.push(p);
                    return true;
                }
                _ => {
                    self.push_pending_as_read(vec![p]);
                    self.push_pending_as_read(spaces);
                    return false;
                }
            }
        }
    }

    /// A control sequence argument, bare (`\foo`) or braced (`{\foo}`),
    /// after optional spaces: its name. Everything read goes to `taken`.
    fn take_cs(&mut self, taken: &mut Vec<Pending>) -> Option<String> {
        let mut spaces = Vec::new();
        loop {
            let Some(p) = self.next_raw() else {
                self.push_pending_as_read(spaces);
                return None;
            };
            match &p.tok.kind {
                TokenKind::Char(_, CatCode::Space) => spaces.push(p),
                TokenKind::ControlSequence(name) => {
                    let name = name.clone();
                    taken.extend(spaces);
                    taken.push(p);
                    return Some(name);
                }
                TokenKind::ActiveChar(c) => {
                    let name = c.to_string();
                    taken.extend(spaces);
                    taken.push(p);
                    return Some(name);
                }
                TokenKind::Char(_, CatCode::BeginGroup) => {
                    self.push_pending_as_read(vec![p]);
                    taken.extend(spaces);
                    let inner = self.take_braced(taken)?;
                    return inner.iter().find_map(|p| match &p.tok.kind {
                        TokenKind::ControlSequence(name) => Some(name.clone()),
                        TokenKind::ActiveChar(c) => Some(c.to_string()),
                        _ => None,
                    });
                }
                _ => {
                    self.push_pending_as_read(vec![p]);
                    self.push_pending_as_read(spaces);
                    return None;
                }
            }
        }
    }

    fn pending_text(&self, toks: &[Pending]) -> String {
        self.detokenize(&toks.iter().map(|p| p.tok.clone()).collect::<Vec<_>>())
    }
}

/// The primitives whose definitions are captured (`Engine::note_definition`
/// in their implementations).
fn is_recorded_primitive(p: Primitive) -> bool {
    use Primitive::*;
    matches!(
        p,
        Def | Edef
            | Gdef
            | Xdef
            | Let
            | NewCommand
            | RenewCommand
            | ProvideCommand
            | DeclareRobustCommand
            | NewEnvironment
            | RenewEnvironment
            | NewTheorem
            | Newif
            | NewBoolean
            | NewCounter
            | NewLength
            | Newcount
            | Newdimen
            | Newskip
            | Newtoks
    )
}

/// The definer's name as LaTeX spells it (`Primitive` names are lower-case
/// identifiers; `\DeclareRobustCommand` is not).
fn primitive_definer(p: Primitive) -> &'static str {
    use Primitive::*;
    match p {
        Def => "def",
        Edef => "edef",
        Gdef => "gdef",
        Xdef => "xdef",
        Let => "let",
        NewCommand => "newcommand",
        RenewCommand => "renewcommand",
        ProvideCommand => "providecommand",
        DeclareRobustCommand => "DeclareRobustCommand",
        NewEnvironment => "newenvironment",
        RenewEnvironment => "renewenvironment",
        NewTheorem => "newtheorem",
        Newif => "newif",
        NewBoolean => "newboolean",
        NewCounter => "newcounter",
        NewLength => "newlength",
        Newcount => "newcount",
        Newdimen => "newdimen",
        Newskip => "newskip",
        Newtoks => "newtoks",
        _ => "",
    }
}

/// `end` backed over the spaces, line ends and `%` comments (the default
/// catcodes: a `%` behind an odd run of backslashes is a character) that
/// separate a statement from the token that was looked at after it.
fn trim_gap(text: &str, mut end: usize) -> usize {
    let bytes = text.as_bytes();
    end = end.min(bytes.len());
    loop {
        while end > 0 && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        // A comment runs from an unescaped `%` to the end of its line, so
        // one on the line the trimming stopped in ends the statement.
        let line_start = bytes[..end].iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        let line = &bytes[line_start..end];
        let escaped = |i: usize| line[..i].iter().rev().take_while(|c| **c == b'\\').count() % 2 == 1;
        match line.iter().enumerate().find(|(i, b)| **b == b'%' && !escaped(*i)) {
            Some((i, _)) => end = line_start + i,
            None => return end,
        }
    }
}

/// `YYYY/MM/DD vX.Y description` (ltclass.dtx's documented layout of the
/// `\ProvidesPackage` bracket): the date when the first word is one, the
/// version when the next word starts with `v` and a digit, the rest as
/// the description. A bracket that follows no convention is all
/// description.
fn split_provides_text(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut words = text.split_whitespace().peekable();
    let is_date = |w: &str| w.len() >= 8 && w.chars().all(|c| c.is_ascii_digit() || c == '/' || c == '-') && w.chars().filter(|c| *c == '/' || *c == '-').count() == 2;
    let date = match words.peek() {
        Some(w) if is_date(w) => words.next().map(str::to_string),
        _ => None,
    };
    let version = match words.peek() {
        Some(w) if w.len() >= 2 && w.starts_with('v') && w[1..].starts_with(|c: char| c.is_ascii_digit()) => words.next().map(str::to_string),
        _ => None,
    };
    let description = words.collect::<Vec<_>>().join(" ");
    (date, version, (!description.is_empty()).then_some(description))
}

/// The number of arguments an xparse argument specification declares:
/// one per specifier letter at brace depth 0, with the delimiter
/// characters of `r`/`R`/`d`/`D` (two) and `t` (one) skipped and the
/// braced defaults/processors skipped by depth. A hint for an editor, not
/// a validation of the specification.
fn xparse_arity(spec: &str) -> u8 {
    let mut depth = 0i32;
    let mut count = 0u8;
    let mut skip = 0usize;
    for c in spec.chars() {
        if skip > 0 {
            skip -= 1;
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ if depth > 0 => {}
            'r' | 'R' | 'd' | 'D' => {
                count = count.saturating_add(1);
                skip = 2;
            }
            't' => {
                count = count.saturating_add(1);
                skip = 1;
            }
            c if c.is_ascii_alphabetic() => count = count.saturating_add(1),
            _ => {}
        }
    }
    count.min(9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provides_text_follows_the_documented_layout() {
        assert_eq!(
            split_provides_text("2024/01/02 v1.3 my macros"),
            (Some("2024/01/02".into()), Some("v1.3".into()), Some("my macros".into()))
        );
        assert_eq!(split_provides_text("2024-01-02"), (Some("2024-01-02".into()), None, None));
        assert_eq!(split_provides_text("just words"), (None, None, Some("just words".into())));
        assert_eq!(split_provides_text(""), (None, None, None));
    }

    #[test]
    fn xparse_arity_counts_specifiers() {
        assert_eq!(xparse_arity("m"), 1);
        assert_eq!(xparse_arity("O{x} m"), 2);
        assert_eq!(xparse_arity("s o m"), 3);
        assert_eq!(xparse_arity("r() m"), 2);
        assert_eq!(xparse_arity("t+ D<>{y} m"), 3);
        assert_eq!(xparse_arity(">{\\SplitList{,}} m"), 1);
        assert_eq!(xparse_arity("+m !o"), 2);
        assert_eq!(xparse_arity(""), 0);
    }
}
