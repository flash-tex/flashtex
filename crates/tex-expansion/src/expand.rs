//! The expansion/execution engine: TeX's `get_next` + `expand` + a slice of
//! `main_control` folded into one pass (TeXbook ch. 20 & 23-24). Since
//! FlashTeX's typesetting stage is a separate crate, this engine's output
//! is the fully expanded, fully "assignment-executed" content-token
//! stream: macros are gone, conditionals are resolved, `\def`/`\let`/
//! register assignments have taken effect, and what remains are character
//! tokens plus any control sequences we don't recognize (left untouched
//! for the typesetting layer, e.g. `\section`, `\hskip`, font commands).
//!
//! All mutable engine state that influences future expansion lives in
//! [`State`], which is `Clone` so the incremental expander
//! (`incremental.rs`) can snapshot it at safe points.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::catcode::CatCode;
use crate::conditionals::{ConditionalStack, IfBranch, IfShape};
use crate::error::{Diagnostic, Limits};
use crate::lexer::{Lexer, State as LexState};
use crate::macro_def::{BodyPart, MacroDef, MacroFlags, ParamPart};
use crate::prelude::PRELUDE;
use crate::registers::{absolute_unit_sp_per_unit, scale_decimal, DefaultFontMetrics, FontMetrics, Glue};
use crate::scopes::{IntParam, Meaning, Primitive, RegisterKind, Scopes};
use crate::span::Span;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone)]
pub(crate) struct Pending {
    pub tok: Token,
    /// `\noexpand`-frozen: do not expand when next read.
    pub frozen: bool,
    /// Span of the outermost macro invocation whose expansion put this
    /// token into the input (`None`: read straight from a source text, or
    /// not yet attributed -- the push functions fill it in). See
    /// [`Engine::next_content_token_with_origin`].
    pub(crate) origin: Option<Span>,
}

#[derive(Debug, Clone)]
pub(crate) enum Input {
    Text(Lexer),
    Toks(Vec<Pending>, usize),
}

/// TeX's `scanner_status` (tex.web §305): what kind of token list is
/// currently being absorbed, so an `\outer` macro or end-of-file inside
/// it can be reported with TeX's exact wording and recovered from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ScannerStatus {
    Normal,
    /// Skipping conditional text; carries the span of the `\if` and its
    /// name for the "Incomplete \if...; all text was ignored after line
    /// N" message. The line is computed only when that error is reported:
    /// counting newlines up to every `\if` made a runaway `\loop` quadratic.
    Skipping { if_name: String, at: Span },
    /// Scanning a `\def` body ("definition of \foo").
    Defining(String),
    /// Scanning macro arguments ("use of \foo").
    Matching(String),
    /// Scanning a `\toks`/`\uppercase`/... text ("text of \foo").
    Absorbing(String),
}

/// Host callback for `\settowidth`/`\settoheight`/`\settodepth`: measure
/// the box that the (already fully expanded) content tokens would set.
/// Returned in scaled points. `DefaultBoxMeasurer` reports zero for
/// everything (documented placeholder; wire a real measurer in the
/// compiler).
pub trait BoxMeasurer {
    fn width(&self, tokens: &[Token]) -> i64;
    fn height(&self, tokens: &[Token]) -> i64;
    fn depth(&self, tokens: &[Token]) -> i64;
}

pub struct DefaultBoxMeasurer;
impl BoxMeasurer for DefaultBoxMeasurer {
    fn width(&self, _: &[Token]) -> i64 {
        0
    }
    fn height(&self, _: &[Token]) -> i64 {
        0
    }
    fn depth(&self, _: &[Token]) -> i64 {
        0
    }
}

/// What `\label{key}` recorded: the key, the fully expanded
/// `\@currentlabel` at that point (set by the most recent
/// `\refstepcounter`), and the span of the `\label` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRecord {
    pub key: String,
    pub current_label: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Vertical,
    Horizontal,
    Math,
    InnerVertical,
    InnerHorizontal,
    InnerMath,
}

impl Mode {
    fn is_v(&self) -> bool {
        matches!(self, Mode::Vertical | Mode::InnerVertical)
    }
    fn is_h(&self) -> bool {
        matches!(self, Mode::Horizontal | Mode::InnerHorizontal)
    }
    fn is_m(&self) -> bool {
        matches!(self, Mode::Math | Mode::InnerMath)
    }
    fn is_inner(&self) -> bool {
        matches!(self, Mode::InnerVertical | Mode::InnerHorizontal | Mode::InnerMath)
    }
}

/// Everything that influences future expansion and is not the input
/// stack itself. Snapshotted at incremental checkpoints; every table is
/// copy-on-write, so a snapshot costs a handful of reference-count bumps.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    pub scopes: Scopes,
    pub conditionals: ConditionalStack,
    pub pending_global: bool,
    pub pending_long: bool,
    pub pending_outer: bool,
    pub pending_protected: bool,
    pub after_assignment: Option<Token>,
    pub mode: Mode,
    /// `\@addtoreset` lists: parent counter -> children reset when the
    /// parent is stepped (LaTeX's `\cl@<parent>`), in insertion order.
    pub counter_children: Rc<HashMap<String, Vec<String>>>,
    pub next_free_register: u16,
    pub next_source_id: u32,
    /// Source ids `1..prelude_source_end` belong to the kernel prelude and
    /// host preludes, which run before any document input is read. A
    /// diagnostic raised at a token of such a macro body is reported at the
    /// document invocation being expanded instead (see `Engine::err`).
    pub prelude_source_end: u32,
    pub scanner_status: ScannerStatus,
    /// `\long` state of the macro whose arguments are being scanned.
    pub matching_long: bool,
    /// Set when a non-`\long` macro's argument scan hit `\par`.
    pub runaway_par: bool,
    /// The `\par` was inserted by `\outer` recovery: abort silently.
    pub runaway_par_silent: bool,
    /// >0 while scanning an `\edef`/`\xdef` body (or another expand-only
    /// context) so `\the\toks`/`\unexpanded` results are frozen.
    pub edef_depth: u32,
    pub in_csname: u32,
    /// Host option, see `Engine::set_emit_unbalanced_close`.
    pub emit_unbalanced_close: bool,
    /// "group nesting limit exceeded" was reported and no `{` has opened a
    /// group since: one report per excursion past the limit, not one per
    /// refused `{`.
    pub group_limit_reported: bool,
    /// The same for "conditional nesting limit exceeded".
    pub conditional_limit_reported: bool,
}

impl State {
    /// This state with every stored span passed through `f` (`None` if `f`
    /// rejects one). `f` must leave document spans with `end <=
    /// identity_bound` unchanged (see `Scopes::map_spans`).
    pub(crate) fn map_spans(&self, f: &dyn Fn(Span) -> Option<Span>, identity_bound: u32) -> Option<State> {
        let after_assignment = match &self.after_assignment {
            Some(t) => Some(Token::new(t.kind.clone(), f(t.span)?)),
            None => None,
        };
        Some(State { scopes: self.scopes.map_spans(f, identity_bound)?, after_assignment, ..self.clone() })
    }

    /// `self.map_spans(f, identity_bound) == Some(new.clone())`, without
    /// building the mapped state (see `Scopes::eq_mapped`).
    pub(crate) fn eq_mapped(&self, new: &State, f: &dyn Fn(Span) -> Option<Span>, identity_bound: u32) -> bool {
        let State {
            scopes,
            conditionals,
            pending_global,
            pending_long,
            pending_outer,
            pending_protected,
            after_assignment,
            mode,
            counter_children,
            next_free_register,
            next_source_id,
            prelude_source_end,
            scanner_status,
            matching_long,
            runaway_par,
            runaway_par_silent,
            edef_depth,
            in_csname,
            emit_unbalanced_close,
            group_limit_reported,
            conditional_limit_reported,
        } = self;
        conditionals == &new.conditionals
            && *pending_global == new.pending_global
            && *pending_long == new.pending_long
            && *pending_outer == new.pending_outer
            && *pending_protected == new.pending_protected
            && *mode == new.mode
            && *next_free_register == new.next_free_register
            && *next_source_id == new.next_source_id
            && *prelude_source_end == new.prelude_source_end
            && scanner_status == &new.scanner_status
            && *matching_long == new.matching_long
            && *runaway_par == new.runaway_par
            && *runaway_par_silent == new.runaway_par_silent
            && *edef_depth == new.edef_depth
            && *in_csname == new.in_csname
            && *emit_unbalanced_close == new.emit_unbalanced_close
            && *group_limit_reported == new.group_limit_reported
            && *conditional_limit_reported == new.conditional_limit_reported
            && match (after_assignment, &new.after_assignment) {
                (None, None) => true,
                (Some(a), Some(b)) => crate::scopes::token_eq_mapped(a, b, f),
                _ => false,
            }
            && (Rc::ptr_eq(counter_children, &new.counter_children) || counter_children == &new.counter_children)
            && scopes.eq_mapped(&new.scopes, f, identity_bound)
    }
}

const PRIMITIVE_TABLE: &[(&str, Primitive)] = &[
    ("relax", Primitive::Relax),
    ("par", Primitive::Par),
    ("def", Primitive::Def),
    ("edef", Primitive::Edef),
    ("gdef", Primitive::Gdef),
    ("xdef", Primitive::Xdef),
    ("let", Primitive::Let),
    ("futurelet", Primitive::Futurelet),
    ("global", Primitive::Global),
    ("long", Primitive::Long),
    ("outer", Primitive::Outer),
    ("protected", Primitive::Protected),
    ("expandafter", Primitive::Expandafter),
    ("noexpand", Primitive::Noexpand),
    ("csname", Primitive::Csname),
    ("endcsname", Primitive::Endcsname),
    ("string", Primitive::String),
    ("number", Primitive::Number),
    ("romannumeral", Primitive::Romannumeral),
    ("meaning", Primitive::MeaningOf),
    ("the", Primitive::The),
    ("unexpanded", Primitive::Unexpanded),
    ("detokenize", Primitive::Detokenize),
    ("expanded", Primitive::Expanded),
    ("eTeXversion", Primitive::IntPar(IntParam::ETeXVersion)),
    ("input", Primitive::InputFile),
    ("jobname", Primitive::Jobname),
    ("eTeXrevision", Primitive::ETeXRevision),
    ("pdfstrcmp", Primitive::Pdfstrcmp),
    ("strcmp", Primitive::Pdfstrcmp),
    ("scantokens", Primitive::Scantokens),
    ("afterassignment", Primitive::Afterassignment),
    ("uppercase", Primitive::Uppercase),
    ("lowercase", Primitive::Lowercase),
    ("uccode", Primitive::Uccode),
    ("lccode", Primitive::Lccode),
    ("chardef", Primitive::Chardef),
    ("mathchardef", Primitive::Mathchardef),
    ("escapechar", Primitive::IntPar(IntParam::Escapechar)),
    ("endlinechar", Primitive::IntPar(IntParam::Endlinechar)),
    ("newlinechar", Primitive::IntPar(IntParam::Newlinechar)),
    ("begingroup", Primitive::Begingroup),
    ("endgroup", Primitive::Endgroup),
    ("aftergroup", Primitive::Aftergroup),
    ("catcode", Primitive::Catcode),
    ("ignorespaces", Primitive::Ignorespaces),
    ("endinput", Primitive::Endinput),
    ("if", Primitive::If),
    ("ifcat", Primitive::Ifcat),
    ("ifx", Primitive::Ifx),
    ("ifnum", Primitive::Ifnum),
    ("ifdim", Primitive::Ifdim),
    ("ifodd", Primitive::Ifodd),
    ("ifvmode", Primitive::Ifvmode),
    ("ifhmode", Primitive::Ifhmode),
    ("ifmmode", Primitive::Ifmmode),
    ("ifinner", Primitive::Ifinner),
    ("ifcase", Primitive::Ifcase),
    ("iftrue", Primitive::Iftrue),
    ("iffalse", Primitive::Iffalse),
    ("ifdefined", Primitive::Ifdefined),
    ("ifcsname", Primitive::Ifcsname),
    ("ifhbox", Primitive::Ifhbox),
    ("ifvbox", Primitive::Ifvbox),
    ("ifvoid", Primitive::Ifvoid),
    ("ifeof", Primitive::Ifeof),
    ("ifincsname", Primitive::Ifincsname),
    ("or", Primitive::Or),
    ("else", Primitive::Else),
    ("fi", Primitive::Fi),
    ("newif", Primitive::Newif),
    ("unless", Primitive::Unless),
    ("count", Primitive::Count),
    ("dimen", Primitive::Dimen),
    ("skip", Primitive::Skip),
    ("toks", Primitive::Toks),
    ("countdef", Primitive::Countdef),
    ("dimendef", Primitive::Dimendef),
    ("skipdef", Primitive::Skipdef),
    ("toksdef", Primitive::Toksdef),
    ("newcount", Primitive::Newcount),
    ("newdimen", Primitive::Newdimen),
    ("newskip", Primitive::Newskip),
    ("newtoks", Primitive::Newtoks),
    ("advance", Primitive::Advance),
    ("multiply", Primitive::Multiply),
    ("divide", Primitive::Divide),
    ("numexpr", Primitive::Numexpr),
    ("dimexpr", Primitive::Dimexpr),
    ("newcommand", Primitive::NewCommand),
    ("renewcommand", Primitive::RenewCommand),
    ("providecommand", Primitive::ProvideCommand),
    ("DeclareRobustCommand", Primitive::DeclareRobustCommand),
    ("newenvironment", Primitive::NewEnvironment),
    ("renewenvironment", Primitive::RenewEnvironment),
    ("begin", Primitive::Begin),
    ("end", Primitive::End),
    ("newcounter", Primitive::NewCounter),
    ("setcounter", Primitive::SetCounter),
    ("addtocounter", Primitive::AddToCounter),
    ("stepcounter", Primitive::StepCounter),
    ("refstepcounter", Primitive::RefStepCounter),
    ("@addtoreset", Primitive::AddToReset),
    ("@removefromreset", Primitive::RemoveFromReset),
    ("counterwithin", Primitive::CounterWithin),
    ("counterwithout", Primitive::CounterWithout),
    ("label", Primitive::Label),
    ("value", Primitive::Value),
    ("arabic", Primitive::Arabic),
    ("roman", Primitive::RomanLower),
    ("Roman", Primitive::RomanUpper),
    ("alph", Primitive::AlphLower),
    ("Alph", Primitive::AlphUpper),
    ("fnsymbol", Primitive::Fnsymbol),
    ("newlength", Primitive::NewLength),
    ("settowidth", Primitive::SetToWidth),
    ("settoheight", Primitive::SetToHeight),
    ("settodepth", Primitive::SetToDepth),
    ("define@key", Primitive::DefineKey),
    ("setkeys", Primitive::SetKeys),
    ("verb", Primitive::Verb),
    ("flashtex@stop", Primitive::StopInput),
];

/// Name of the private sentinel control sequence used to bound nested
/// full expansions (`\settowidth`, `\label`).
const SENTINEL: &str = "flashtex@sentinel";
/// TeX Live's `stack_size`: the deepest input stack (macro bodies being read).
const TEX_INPUT_STACK_SIZE: usize = 10_000;
/// tex.web `infinity`: the largest integer TeX's scanner accepts (§445).
const TEX_INFINITY: i64 = 0x7FFF_FFFF;
/// tex.web `max_dimen` (§421): 16383.99998pt.
const TEX_MAX_DIMEN: i64 = 0x3FFF_FFFF;

/// A digit string as TeX's `scan_int` reads it: past `infinity` it is
/// `infinity` (the caller reports "Number too big.").
fn parse_clamped(digits: &str, radix: u32) -> (i64, bool) {
    let mut value: i64 = 0;
    for c in digits.chars() {
        let d = c.to_digit(radix).unwrap_or(0) as i64;
        value = value * radix as i64 + d;
        if value > TEX_INFINITY {
            return (TEX_INFINITY, true);
        }
    }
    (value, false)
}

/// What one dispatch step produced.
pub(crate) enum Step {
    Emit(Token),
    Continue,
    Eof,
}

/// A snapshot of the engine at a safe point (input stack = the base
/// lexer only), from which expansion can be resumed over an edited
/// buffer. See `incremental.rs`.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Byte offset in the base source where the lexer stands.
    pub pos: usize,
    pub(crate) lex_state: LexState,
    pub(crate) state: State,
    pub(crate) steps: u64,
    /// Number of output tokens / diagnostics / labels produced so far.
    pub out_len: usize,
    pub diag_len: usize,
    pub label_len: usize,
}

pub struct Engine {
    src: Rc<str>,
    pub(crate) sources: Vec<Input>,
    limits: Limits,
    steps: u64,
    pub(crate) st: State,
    diagnostics: Vec<Diagnostic>,
    /// Diagnostics reported since the engine was last at a safe point (see
    /// [`Engine::report`]).
    reported: HashSet<Diagnostic>,
    labels: Vec<LabelRecord>,
    metrics: Rc<dyn FontMetrics>,
    measurer: Rc<dyn BoxMeasurer>,
    /// Set by `\end{document}` or a hard resource limit.
    stopped: bool,
    /// Tokens already decided to be output (a stack, popped first by
    /// `next_content_token`), e.g. prefixes passed through ahead of an
    /// unmodelled control sequence.
    emit_queue: Vec<Token>,
    /// Host file access for `\input` (None: `\input` passes through).
    file_reader: Option<Rc<dyn Fn(&str) -> Option<String>>>,
    /// Files opened by `\input`, as (source id, name).
    opened_files: Vec<(u32, String)>,
    /// Invocation origin of the most recently read raw token (host
    /// integration; see `next_content_token_with_origin`).
    last_origin: Option<Span>,
    /// Span of the last token read from source text (the document or an
    /// `\input` file; preludes run in engines of their own).
    last_text_span: Option<Span>,
}

impl Engine {
    pub fn new(source: &str) -> Self {
        Self::with_limits(source, Limits::default())
    }

    pub fn with_limits(source: &str, limits: Limits) -> Self {
        let st = Self::initial_state();
        Self::from_parts(Rc::from(source), 0, LexState::NewLine, st, limits)
    }

    /// The post-prelude state every document starts from. Computed by
    /// running the LaTeX-kernel prelude once; cached per thread since it
    /// is identical for every engine.
    pub(crate) fn initial_state() -> State {
        thread_local! {
            static INITIAL: State = build_initial_state();
        }
        INITIAL.with(|s| s.clone())
    }

    pub(crate) fn from_parts(src: Rc<str>, pos: usize, lex_state: LexState, st: State, limits: Limits) -> Self {
        let lexer = Lexer::resume(src.clone(), 0, pos, lex_state);
        Engine {
            src,
            sources: vec![Input::Text(lexer)],
            limits,
            steps: 0,
            st,
            diagnostics: Vec::new(),
            reported: HashSet::new(),
            labels: Vec::new(),
            metrics: Rc::new(DefaultFontMetrics),
            measurer: Rc::new(DefaultBoxMeasurer),
            stopped: false,
            emit_queue: Vec::new(),
            file_reader: None,
            opened_files: Vec::new(),
            last_origin: None,
            last_text_span: None,
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.st.mode = mode;
    }

    pub fn set_font_metrics(&mut self, metrics: Rc<dyn FontMetrics>) {
        self.metrics = metrics;
    }

    /// Let `\input <name>` read files: `reader(name)` returns the file's
    /// text (the host decides how names resolve, e.g. adding `.tex` or
    /// asking kpathsea). Without a reader `\input` is passed through to
    /// the typesetting layer untouched.
    pub fn set_file_reader(&mut self, reader: Rc<dyn Fn(&str) -> Option<String>>) {
        self.file_reader = Some(reader);
    }

    /// `(source id, name)` of every file `\input` has opened; token spans
    /// with that `source_id` point into it.
    pub fn opened_files(&self) -> &[(u32, String)] {
        &self.opened_files
    }

    /// [`Engine::next_content_token`], plus the span of the outermost macro
    /// invocation whose expansion produced the token (`None` when it was
    /// read directly from a source text). A token substituted from a macro
    /// argument keeps its own source span; hosts tell it apart from
    /// replacement-text tokens by comparing that span with the invocation.
    pub fn next_content_token_with_origin(&mut self) -> Option<(Token, Option<Span>)> {
        let tok = self.next_content_token()?;
        Some((tok, self.last_origin))
    }

    /// Declare a control sequence the host typesets itself (`\section`,
    /// `\hfill`, ...). It is still emitted unchanged, but counts as defined:
    /// `\newcommand` refuses it and `\renewcommand` accepts it, as in LaTeX.
    /// No effect on a name this engine already defines.
    pub fn declare_host_command(&mut self, name: &str) {
        if !self.st.scopes.is_defined(name) {
            self.st.scopes.assign_cs(name, Meaning::Primitive(Primitive::Host), true);
        }
    }

    /// Execute host-supplied TeX definitions (no output is kept) before the
    /// document is read. The definitions become part of the assignment
    /// state, so incremental checkpoints carry them; their spans carry a
    /// source id of their own. Run it before reading any document token.
    pub fn run_host_prelude(&mut self, text: &str) {
        let mut st = std::mem::replace(&mut self.st, Self::initial_state());
        let id = st.next_source_id;
        st.next_source_id += 1;
        st.prelude_source_end = st.next_source_id;
        let mut prelude = Engine::from_parts(Rc::from(text), 0, LexState::NewLine, st, self.limits);
        prelude.sources[0] = Input::Text(Lexer::new(Rc::from(text), id));
        let _ = prelude.run();
        self.st = prelude.st;
    }

    /// Host option: pass an unbalanced closing brace ("Too many }'s") on
    /// to the output instead of dropping it, so a host parser can report and
    /// recover from it at its own position. The error is still recorded.
    pub fn set_emit_unbalanced_close(&mut self, emit: bool) {
        self.st.emit_unbalanced_close = emit;
    }

    /// Start reading `text` before the rest of the current input, as a
    /// host-resolved `\input` does; returns the source id its spans carry.
    pub fn push_input(&mut self, text: &str) -> u32 {
        let id = self.st.next_source_id;
        self.st.next_source_id += 1;
        self.prune_exhausted();
        self.sources.push(Input::Text(Lexer::new(Rc::from(text), id)));
        id
    }

    /// Source ids of the source texts currently open (outermost first).
    pub fn open_input_ids(&self) -> Vec<u32> {
        self.sources
            .iter()
            .filter_map(|input| match input {
                Input::Text(lexer) => Some(lexer.source_id()),
                Input::Toks(..) => None,
            })
            .collect()
    }

    /// The innermost open source text and the byte offset its tokenizer
    /// has reached.
    pub fn input_position(&self) -> Option<(u32, usize)> {
        self.sources.iter().rev().find_map(|input| match input {
            Input::Text(lexer) => Some((lexer.source_id(), lexer.pos())),
            Input::Toks(..) => None,
        })
    }

    pub fn set_box_measurer(&mut self, measurer: Rc<dyn BoxMeasurer>) {
        self.measurer = measurer;
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn labels(&self) -> &[LabelRecord] {
        &self.labels
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn push_diagnostic(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    pub fn take_labels(&mut self) -> Vec<LabelRecord> {
        std::mem::take(&mut self.labels)
    }

    pub fn source(&self) -> &str {
        &self.src
    }

    fn err(&mut self, msg: impl Into<String>, span: Span) {
        let span = self.reported_span(span);
        self.report(Diagnostic::error(msg, span));
    }

    fn warn(&mut self, msg: impl Into<String>, span: Span) {
        let span = self.reported_span(span);
        self.report(Diagnostic::warning(msg, span));
    }

    /// Record `d` unless the identical diagnostic (severity, message, span)
    /// was already recorded since the engine was last at a safe point. A
    /// runaway loop never reaches one, so it reports each of its messages
    /// once instead of once per iteration (a `\loop` of `\ifnum` with a
    /// missing number reported "Missing number" 2.2 million times).
    ///
    /// The set is emptied whenever [`Engine::next_content_token`] returns
    /// at a safe point, and safe points are where incremental checkpoints
    /// are taken and where a re-run converges, so a restored engine (whose
    /// set starts empty) makes exactly the decisions a from-scratch run
    /// makes.
    fn report(&mut self, d: Diagnostic) {
        if self.reported.contains(&d) {
            return;
        }
        self.reported.insert(d.clone());
        self.diagnostics.push(d);
    }

    /// [`Engine::safe_point`] without pruning exhausted inputs.
    fn at_safe_point(&self) -> bool {
        let live = self.sources.iter().skip(1).any(|input| match input {
            Input::Toks(toks, pos) => *pos < toks.len(),
            Input::Text(l) => !l.at_end(),
        });
        if live || self.stopped || !self.emit_queue.is_empty() || self.base_lexer().state() != LexState::NewLine {
            return false;
        }
        let st = &self.st;
        !(st.pending_global
            || st.pending_long
            || st.pending_outer
            || st.pending_protected
            || st.after_assignment.is_some()
            || st.scanner_status != ScannerStatus::Normal
            || st.edef_depth != 0
            || st.in_csname != 0)
    }

    /// Where a diagnostic at `span` is reported: a token of a prelude macro
    /// body (kernel or host) has no bytes in any document, so the document
    /// invocation being expanded stands in for it, or else the last token
    /// read from source text (look-ahead such as `\@ifnextchar`'s
    /// `\futurelet` reads the document and drops the invocation origin).
    fn reported_span(&self, span: Span) -> Span {
        if !self.is_prelude_span(span) {
            return span;
        }
        match self.last_origin {
            Some(origin) if !origin.is_synthetic() && !self.is_prelude_span(origin) => origin,
            _ => self.last_text_span.unwrap_or(span),
        }
    }

    fn is_prelude_span(&self, span: Span) -> bool {
        !span.is_synthetic() && span.source_id != 0 && span.source_id < self.st.prelude_source_end
    }

    /// The escape character as TeX would print it (`\escapechar`), or
    /// nothing when it is negative / out of range.
    fn esc(&self) -> String {
        let e = self.st.scopes.int_param(IntParam::Escapechar);
        if (0..256).contains(&e) {
            char::from_u32(e as u32).map(|c| c.to_string()).unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// TeX's `print_cs`: escape char + name, plus a trailing space when
    /// the name's last character is a letter (so control words stay
    /// separable when re-read), per tex.web §262.
    fn print_cs(&self, name: &str) -> String {
        let mut chars = name.chars();
        match (chars.next(), chars.next()) {
            // The null control sequence prints as `\csname\endcsname `.
            (None, _) => format!("{e}csname{e}endcsname ", e = self.esc()),
            // Single-character name: space only if that character is
            // currently a letter.
            (Some(c), None) => {
                let mut s = self.esc();
                s.push(c);
                if self.st.scopes.catcode(c) == CatCode::Letter {
                    s.push(' ');
                }
                s
            }
            // Multi-letter name (even `\foo@` or a `\csname`-built one
            // ending in a non-letter): always followed by a space.
            _ => format!("{}{name} ", self.esc()),
        }
    }

    fn base_lexer(&self) -> &Lexer {
        match &self.sources[0] {
            Input::Text(l) => l,
            _ => unreachable!("base input is always the source lexer"),
        }
    }

    fn line_of_span(&self, span: Span) -> usize {
        if span.source_id == 0 {
            self.base_lexer().line_of(span.start as usize)
        } else {
            0
        }
    }

    // ---- raw token stream -------------------------------------------------

    fn push_tokens(&mut self, toks: Vec<Token>) {
        let origin = self.last_origin;
        self.push_tokens_with_origin(toks, origin);
    }

    fn push_tokens_with_origin(&mut self, toks: Vec<Token>, origin: Option<Span>) {
        if toks.is_empty() {
            return;
        }
        self.prune_exhausted();
        if self.input_capacity_exceeded() {
            return;
        }
        let pend = toks.into_iter().map(|tok| Pending { tok, frozen: false, origin }).collect();
        self.sources.push(Input::Toks(pend, 0));
    }

    fn push_pending(&mut self, mut toks: Vec<Pending>) {
        if toks.is_empty() {
            return;
        }
        for p in &mut toks {
            if p.origin.is_none() {
                p.origin = self.last_origin;
            }
        }
        self.prune_exhausted();
        if self.input_capacity_exceeded() {
            return;
        }
        self.sources.push(Input::Toks(toks, 0));
    }

    fn push_frozen(&mut self, tok: Token) {
        self.prune_exhausted();
        let origin = self.last_origin;
        self.sources.push(Input::Toks(vec![Pending { tok, frozen: true, origin }], 0));
    }

    /// TeX's input stack and main memory limits for pending token lists. A
    /// macro that re-invokes itself before the end of its own body (so it
    /// is not a tail call) adds an input level per call: with a long body,
    /// `\def\a{\csname a\endcsname [[[...]]]}\a` grew past 30 GB before the
    /// step limit. TeX stops with "TeX capacity exceeded"; so does this.
    fn input_capacity_exceeded(&mut self) -> bool {
        let levels = self.sources.len();
        let message = if levels >= TEX_INPUT_STACK_SIZE {
            "TeX capacity exceeded, sorry [input stack size=10000]."
        } else if levels % 64 == 0 && self.pending_token_count() > self.limits.max_output_tokens {
            "TeX capacity exceeded, sorry [main memory size=5000000]."
        } else {
            return false;
        };
        let at = self.last_origin.unwrap_or(Span::synthetic());
        self.err(message, at);
        self.stopped = true;
        true
    }

    /// Tokens still to be read from every token-list input level.
    fn pending_token_count(&self) -> u64 {
        self.sources
            .iter()
            .map(|input| match input {
                Input::Toks(toks, pos) => toks.len().saturating_sub(*pos) as u64,
                Input::Text(_) => 0,
            })
            .sum()
    }

    /// Pop exhausted token-list inputs off the top of the stack (they are
    /// otherwise popped lazily on the next read).
    pub(crate) fn prune_exhausted(&mut self) {
        while self.sources.len() > 1 {
            match self.sources.last() {
                Some(Input::Toks(toks, pos)) if *pos >= toks.len() => {
                    self.sources.pop();
                }
                Some(Input::Text(l)) if l.at_end() => {
                    self.sources.pop();
                }
                _ => break,
            }
        }
    }

    fn next_raw_unchecked(&mut self) -> Option<Pending> {
        loop {
            if self.stopped {
                return None;
            }
            match self.sources.last_mut()? {
                Input::Text(lexer) => {
                    if let Some(tok) = lexer.next_token(self.st.scopes.cat_table(), self.st.scopes.int_param(IntParam::Endlinechar)) {
                        self.last_origin = None;
                        self.last_text_span = Some(tok.span);
                        return Some(Pending { tok, frozen: false, origin: None });
                    } else if self.sources.len() == 1 {
                        return None;
                    } else {
                        self.sources.pop();
                        continue;
                    }
                }
                Input::Toks(toks, pos) => {
                    if *pos < toks.len() {
                        let p = &toks[*pos];
                        *pos += 1;
                        let origin = p.origin;
                        let pending = Pending { tok: p.tok.clone(), frozen: p.frozen, origin };
                        self.last_origin = origin;
                        return Some(pending);
                    } else {
                        self.sources.pop();
                        continue;
                    }
                }
            }
        }
    }

    /// `get_next` with TeX's `check_outer_validity` (tex.web §336): while
    /// a definition/argument/conditional-skip is being scanned, an
    /// `\outer` macro token (or end of the base file) is an error that
    /// TeX reports with a specific message and recovers from by inserting
    /// a closing token and re-reading the forbidden token afterwards.
    fn next_raw(&mut self) -> Option<Pending> {
        let p = self.next_raw_unchecked();
        if self.st.scanner_status == ScannerStatus::Normal {
            return p;
        }
        match p {
            Some(p) => {
                if p.frozen {
                    return Some(p);
                }
                let is_outer = match &p.tok.kind {
                    TokenKind::ControlSequence(name) => self.st.scopes.meaning_ref(name).map_or(false, meaning_is_outer),
                    TokenKind::ActiveChar(c) => meaning_is_outer(&self.st.scopes.active_meaning(*c)),
                    _ => false,
                };
                if !is_outer {
                    return Some(p);
                }
                self.report_outer_and_recover(p.tok)
            }
            None => {
                // The base file ended inside a definition/argument/... .
                // TeX: "File ended while scanning use of \foo" and the
                // same recovery insertions (§338), except for skipping
                // where it is "Incomplete \if; all text was ignored".
                self.report_file_ended_and_recover()
            }
        }
    }

    fn report_outer_and_recover(&mut self, outer_tok: Token) -> Option<Pending> {
        let status = self.st.scanner_status.clone();
        let (msg, recovery): (String, Token) = match status {
            ScannerStatus::Defining(name) => (
                format!("Runaway definition?\n! Forbidden control sequence found while scanning definition of {name}."),
                Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)),
            ),
            ScannerStatus::Matching(name) => (
                format!("Runaway argument?\n! Forbidden control sequence found while scanning use of {name}."),
                Token::synthetic(TokenKind::ControlSequence("par".into())),
            ),
            ScannerStatus::Absorbing(name) => (
                format!("Runaway text?\n! Forbidden control sequence found while scanning text of {name}."),
                Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)),
            ),
            ScannerStatus::Skipping { if_name, at } => (
                format!("Incomplete {if_name}; all text was ignored after line {}.", self.line_of_span(at)),
                Token::synthetic(TokenKind::ControlSequence("fi".into())),
            ),
            ScannerStatus::Normal => unreachable!(),
        };
        let span = outer_tok.span;
        self.err(msg, span);
        if matches!(self.st.scanner_status, ScannerStatus::Matching(_)) {
            // long_state := outer_call: the inserted \par aborts the
            // macro call silently (§339/§396).
            self.st.matching_long = false;
            self.st.runaway_par_silent = true;
        }
        // Back up the outer token, insert the recovery token ahead of it,
        // and return a space in place of the forbidden token (§336).
        self.push_tokens(vec![recovery, outer_tok]);
        Some(Pending { tok: Token::new(TokenKind::Char(' ', CatCode::Space), span), frozen: false, origin: None })
    }

    fn report_file_ended_and_recover(&mut self) -> Option<Pending> {
        let status = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Normal);
        let span = Span::new(0, self.src.len() as u32, self.src.len() as u32);
        match status {
            ScannerStatus::Defining(name) => {
                self.err(format!("Runaway definition?\n! File ended while scanning definition of {name}."), span);
            }
            ScannerStatus::Matching(name) => {
                self.err(format!("Runaway argument?\n! File ended while scanning use of {name}."), span);
                // §339: long_state := outer_call, so the inserted \par aborts
                // the macro call (silently) instead of expanding the body with
                // a partial argument. Expanding it made `\loop{x}` (no
                // \repeat) iterate to the step limit.
                self.st.runaway_par = true;
                self.st.runaway_par_silent = true;
            }
            ScannerStatus::Absorbing(name) => {
                self.err(format!("Runaway text?\n! File ended while scanning text of {name}."), span);
            }
            ScannerStatus::Skipping { if_name, at } => {
                let line = self.line_of_span(at);
                self.err(format!("Incomplete {if_name}; all text was ignored after line {line}."), span);
            }
            ScannerStatus::Normal => {}
        }
        None
    }

    fn next_raw_token(&mut self) -> Option<Token> {
        self.next_raw().map(|p| p.tok)
    }

    // ---- main dispatch loop -------------------------------------------

    /// Read and fully process the next content token: expands macros and
    /// expandable primitives, executes assignments/definitions/grouping,
    /// and returns the next token meant for the typesetting layer (or
    /// `None` at end of input).
    pub fn next_content_token(&mut self) -> Option<Token> {
        let token = self.next_content_token_unchecked();
        if !self.reported.is_empty() && self.at_safe_point() {
            self.reported.clear();
        }
        token
    }

    fn next_content_token_unchecked(&mut self) -> Option<Token> {
        loop {
            if !self.tick() {
                return None;
            }
            if let Some(t) = self.emit_queue.pop() {
                return Some(t);
            }
            let pending = self.next_raw()?;
            // A `\noexpand`ed expandable token reaching main control acts
            // like `\relax` (TeXbook p. 213): it produces nothing.
            if pending.frozen && self.is_expandable(&pending.tok) {
                continue;
            }
            match self.step(pending.tok) {
                Step::Emit(t) => {
                    if self.prefix_pending() {
                        if let Some(t) = self.prefix_before_content(t) {
                            return Some(t);
                        }
                        continue;
                    }
                    return Some(t);
                }
                Step::Continue => continue,
                Step::Eof => return None,
            }
        }
    }

    /// Count one expansion step outside the main loop (macro calls,
    /// expand-only reads). Past `max_expansion_steps` the engine stops
    /// with one diagnostic -- the analogue of TeX's "capacity exceeded"
    /// for e.g. `\def\a{x\a}\edef\b{\a}`, which otherwise never returns
    /// to the main loop.
    fn tick(&mut self) -> bool {
        if self.stopped {
            return false;
        }
        self.steps += 1;
        if self.steps > self.limits.max_expansion_steps {
            let at = self.last_origin.unwrap_or(Span::synthetic());
            self.err("expansion step limit exceeded (possible infinite macro loop)", at);
            self.stopped = true;
            return false;
        }
        true
    }

    fn prefix_pending(&self) -> bool {
        self.st.pending_global || self.st.pending_long || self.st.pending_outer || self.st.pending_protected
    }

    fn clear_prefixes(&mut self) {
        self.take_prefixes();
    }

    /// A content token arrived while `\global`/`\long`/`\outer`/
    /// `\protected` was pending (tex.web §1211 `prefixed_command`):
    /// spaces and `\relax` are skipped with the prefix kept; a control
    /// sequence this crate does not model (`\global\setbox`, `\global
    /// \font`, ...) gets the prefixes passed through ahead of it for the
    /// typesetter; anything else is TeX's "You can't use a prefix with"
    /// error and the prefixes are dropped.
    fn prefix_before_content(&mut self, t: Token) -> Option<Token> {
        if matches!(t.kind, TokenKind::Char(_, CatCode::Space)) || t.is_cs("relax") {
            return None;
        }
        let passthrough = match &t.kind {
            TokenKind::ControlSequence(_) | TokenKind::ActiveChar(_) => {
                matches!(self.meaning_of_token(&t), Meaning::Undefined)
            }
            _ => false,
        };
        if passthrough {
            let mut prefixes = Vec::new();
            for (on, name) in [
                (self.st.pending_global, "global"),
                (self.st.pending_long, "long"),
                (self.st.pending_outer, "outer"),
                (self.st.pending_protected, "protected"),
            ] {
                if on {
                    prefixes.push(Token::synthetic(TokenKind::ControlSequence(name.into())));
                }
            }
            self.clear_prefixes();
            // emit_queue is a stack: push in reverse.
            self.emit_queue.push(t);
            while prefixes.len() > 1 {
                self.emit_queue.push(prefixes.pop().unwrap());
            }
            return prefixes.pop();
        }
        let what = self.cmd_text(&t);
        self.err(format!("You can't use a prefix with `{what}'."), t.span);
        self.clear_prefixes();
        Some(t)
    }

    /// TeX's `print_cmd_chr` for a token as it would appear in an error
    /// message ("the letter a", "\\begingroup", ...).
    fn cmd_text(&self, t: &Token) -> String {
        match &t.kind {
            TokenKind::Char(c, cat) => char_meaning(*c, *cat),
            TokenKind::ControlSequence(name) => match self.st.scopes.meaning_ref(name) {
                Some(Meaning::Primitive(p)) => format!("{}{}", self.esc(), primitive_name(*p)),
                _ => format!("{}{name}", self.esc()),
            },
            _ => t.display_name(),
        }
    }

    /// Prefixes for a non-`\def` assignment: `\long`/`\outer`/
    /// `\protected` are an error there ("You can't use `\long' or
    /// `\outer' with ...") and are dropped; returns whether `\global` was
    /// given.
    fn take_assignment_prefixes(&mut self, cmd: &str) -> bool {
        let (global, flags) = self.take_prefixes();
        let e = self.esc();
        if flags.long || flags.outer {
            self.err(format!("You can't use `{e}long' or `{e}outer' with `{e}{cmd}'."), Span::synthetic());
        } else if flags.protected {
            self.err(format!("You can't use `{e}protected' with `{e}{cmd}'."), Span::synthetic());
        }
        global
    }

    /// Drain the whole input into a token vector plus diagnostics. This is
    /// the primary library entry point; see `lib.rs::expand_str`.
    pub fn run(&mut self) -> Vec<Token> {
        let mut out = Vec::new();
        while let Some(tok) = self.next_content_token() {
            out.push(tok);
            if out.len() as u64 > self.limits.max_output_tokens {
                self.err("output token limit exceeded", Span::synthetic());
                break;
            }
        }
        out
    }

    // ---- incremental support ----------------------------------------

    /// If the engine is at a *safe point* -- nothing pending on the input
    /// stack except the base lexer, which has just consumed an end-of-line
    /// (so the next token starts a fresh line), no half-read prefix
    /// (`\global` etc.) and no argument/definition scan in progress --
    /// return the lexer's byte position. Expansion can be resumed from a
    /// snapshot taken here over any buffer that is byte-identical up to
    /// and including this position.
    pub fn safe_point(&mut self) -> Option<usize> {
        self.prune_exhausted();
        if self.sources.len() != 1 || self.stopped || !self.emit_queue.is_empty() {
            return None;
        }
        let lexer = self.base_lexer();
        if lexer.state() != LexState::NewLine {
            return None;
        }
        let st = &self.st;
        if st.pending_global
            || st.pending_long
            || st.pending_outer
            || st.pending_protected
            || st.after_assignment.is_some()
            || st.scanner_status != ScannerStatus::Normal
            || st.edef_depth != 0
            || st.in_csname != 0
        {
            return None;
        }
        Some(lexer.pos())
    }

    pub fn snapshot(&self, out_len: usize) -> Checkpoint {
        Checkpoint {
            pos: self.base_lexer().pos(),
            lex_state: self.base_lexer().state(),
            state: self.st.clone(),
            steps: self.steps,
            out_len,
            diag_len: self.diagnostics.len(),
            label_len: self.labels.len(),
        }
    }

    /// Rebuild an engine positioned at `cp` over (possibly edited) `src`.
    pub fn restore(src: Rc<str>, cp: &Checkpoint, limits: Limits) -> Self {
        let mut e = Self::from_parts(src, cp.pos, cp.lex_state, cp.state.clone(), limits);
        e.steps = cp.steps;
        e
    }

    pub(crate) fn state(&self) -> &State {
        &self.st
    }

    pub(crate) fn lex_state(&self) -> LexState {
        self.base_lexer().state()
    }

    pub fn at_end(&mut self) -> bool {
        self.prune_exhausted();
        self.stopped || (self.sources.len() == 1 && self.base_lexer().at_end())
    }

    pub fn steps(&self) -> u64 {
        self.steps
    }

    fn step(&mut self, tok: Token) -> Step {
        match tok.kind.clone() {
            TokenKind::Char(_, _) => {
                if let Some(step) = self.maybe_handle_brace(&tok) {
                    step
                } else {
                    Step::Emit(tok)
                }
            }
            TokenKind::Param(_) => Step::Emit(tok),
            TokenKind::Eof => Step::Eof,
            TokenKind::ControlSequence(name) => {
                let meaning = self.st.scopes.meaning(&name);
                self.dispatch(tok, meaning)
            }
            TokenKind::ActiveChar(c) => {
                let meaning = self.st.scopes.active_meaning(c);
                match meaning {
                    Meaning::Undefined => Step::Emit(tok),
                    m => self.dispatch(tok, m),
                }
            }
        }
    }

    fn dispatch(&mut self, tok: Token, meaning: Meaning) -> Step {
        match meaning {
            Meaning::Macro(def) => {
                self.call_macro(&tok, &def);
                Step::Continue
            }
            Meaning::Let(inner) => self.dispatch(tok, *inner),
            Meaning::CharLike(t) => {
                // A `\let`-to-character token behaves like that character
                // in main control (including grouping for `\let\bgroup={`).
                let t2 = Token::new(t.kind, tok.span);
                if let Some(step) = self.maybe_handle_brace(&t2) {
                    step
                } else {
                    Step::Emit(t2)
                }
            }
            Meaning::RegisterAlias(kind, idx) => self.handle_register_ref(tok, kind, idx),
            Meaning::CharDef(n) => match char::from_u32(n as u32) {
                Some(c) => Step::Emit(Token::new(TokenKind::Char(c, CatCode::Other), tok.span)),
                None => Step::Continue,
            },
            Meaning::MathCharDef(_) => Step::Emit(tok),
            Meaning::Primitive(p) => self.handle_primitive(tok, p),
            Meaning::Undefined => Step::Emit(tok),
        }
    }

    /// Would `expand` do something with this token? (Macros and
    /// expandable primitives; `\protected` macros count as *not*
    /// expandable in expand-only contexts, per e-TeX.)
    fn is_expandable(&self, tok: &Token) -> bool {
        match &tok.kind {
            TokenKind::ControlSequence(name) => {
                self.st.scopes.meaning_ref(name).map_or(false, |m| meaning_is_expandable(m, self.st.edef_depth > 0))
            }
            TokenKind::ActiveChar(c) => meaning_is_expandable(&self.st.scopes.active_meaning(*c), self.st.edef_depth > 0),
            _ => false,
        }
    }

    // ---- grouping (`{`/`}` characters) --------------------------------

    /// Characters with catcode BeginGroup/EndGroup (explicit, or implicit
    /// via `\let\bgroup={`) open/close scopes. The token itself is also
    /// emitted, with its span: after expansion TeX hands `{`/`}` to the
    /// stomach, and downstream consumers need the group/argument
    /// boundaries. `\aftergroup` tokens are reinserted right after the
    /// emitted `}`. Unbalanced `}` ("Too many }'s") is dropped, as TeX
    /// does.
    fn maybe_handle_brace(&mut self, tok: &Token) -> Option<Step> {
        if let TokenKind::Char(_, cat) = tok.kind {
            match cat {
                CatCode::BeginGroup => {
                    if self.st.scopes.depth() as u32 > self.limits.max_group_depth {
                        if !self.st.group_limit_reported {
                            self.st.group_limit_reported = true;
                            self.err("group nesting limit exceeded", tok.span);
                        }
                        return Some(Step::Continue);
                    }
                    self.st.group_limit_reported = false;
                    self.st.scopes.push_group();
                    return Some(Step::Emit(tok.clone()));
                }
                CatCode::EndGroup => {
                    if self.st.scopes.depth() <= 1 {
                        self.err("Too many }'s.", tok.span);
                        return Some(if self.st.emit_unbalanced_close { Step::Emit(tok.clone()) } else { Step::Continue });
                    }
                    let after = self.st.scopes.pop_group();
                    self.push_tokens(after);
                    return Some(Step::Emit(tok.clone()));
                }
                _ => {}
            }
        }
        None
    }

    // ---- macro definition & calling -----------------------------------

    /// Scan a `\def`-style parameter text up to (not including) the
    /// opening `{` of the body, per TeXbook p.203-205.
    /// The third result is `false` when the parameter text was ended by a
    /// `}` instead of the body's `{` (tex.web §475 "Missing { inserted":
    /// the `}` is consumed and the definition gets an empty body).
    fn scan_param_text(&mut self) -> (Vec<ParamPart>, MacroFlags, bool) {
        let mut params = Vec::new();
        let mut flags = MacroFlags::default();
        loop {
            let tok = match self.next_raw_token() {
                Some(t) => t,
                None => break,
            };
            match &tok.kind {
                TokenKind::Char(_, CatCode::BeginGroup) => {
                    // pushed back: caller's scan_braced_body will re-read it
                    self.push_tokens(vec![tok]);
                    break;
                }
                TokenKind::Char(_, CatCode::EndGroup) => {
                    self.err("Missing { inserted.", tok.span);
                    return (params, flags, false);
                }
                TokenKind::Char(_, CatCode::Param) => {
                    // `#` in a parameter text is followed either by a
                    // digit 1..9 naming the parameter slot, or directly by
                    // `{` -- the `#{` marker (TeXbook p.205) meaning "the
                    // last parameter is delimited by the upcoming brace
                    // group", with no parameter number of its own. The
                    // `{` is pushed back for `scan_braced_group` to
                    // consume as the start of the body.
                    match self.next_raw_token() {
                        Some(next) => match next.kind {
                            TokenKind::Char(d, _) if d.is_ascii_digit() && d != '0' => {
                                let n = d.to_digit(10).unwrap() as u8;
                                params.push(ParamPart::Param(n));
                            }
                            TokenKind::Char(_, CatCode::BeginGroup) => {
                                flags.brace_delimited_last = true;
                                self.push_tokens(vec![next]);
                            }
                            _ => {
                                self.err("Parameters must be numbered consecutively.", tok.span);
                                self.push_tokens(vec![next]);
                            }
                        },
                        None => {}
                    }
                }
                _ => params.push(ParamPart::Literal(tok)),
            }
        }
        (params, flags, true)
    }

    /// Scan a brace-delimited token list `{ ... }` with correct nested
    /// brace counting (TeXbook p.212's `scan_toks`), returning the inner
    /// tokens (braces not included). If `expand` is set, tokens are read
    /// through expand-only dispatch (for `\edef`/`\xdef`), honoring
    /// `\noexpand` freezing; otherwise tokens are taken completely raw
    /// (for `\def`/`\gdef` bodies, and for ordinary `{...}` arguments).
    fn scan_braced_group(&mut self, expand: bool) -> Vec<Token> {
        self.scan_braced_group_pending(expand).into_iter().map(|p| p.tok).collect()
    }

    fn scan_braced_group_pending(&mut self, expand: bool) -> Vec<Pending> {
        // Expect and consume the opening brace (skip intervening spaces).
        loop {
            match self.next_raw_token() {
                Some(t) => match t.kind {
                    TokenKind::Char(_, CatCode::Space) => continue,
                    TokenKind::Char(_, CatCode::BeginGroup) => break,
                    _ => {
                        // Missing '{': TeX says "Missing { inserted" and
                        // treats the single token as the whole group.
                        self.err("Missing { inserted.", t.span);
                        return vec![Pending { tok: t, frozen: false, origin: None }];
                    }
                },
                None => return Vec::new(),
            }
        }
        if expand {
            self.st.edef_depth += 1;
        }
        let mut depth = 1i32;
        let mut out = Vec::new();
        loop {
            let next = if expand { self.next_expanding_raw() } else { self.next_raw() };
            let pending = match next {
                Some(p) => p,
                None => break,
            };
            match &pending.tok.kind {
                TokenKind::Char(_, CatCode::BeginGroup) => {
                    depth += 1;
                    out.push(pending);
                }
                TokenKind::Char(_, CatCode::EndGroup) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(pending);
                }
                TokenKind::ControlSequence(n) if n == "par" && !self.st.matching_long && matches!(self.st.scanner_status, ScannerStatus::Matching(_)) => {
                    // Non-\long macro argument: a \par aborts the call.
                    self.st.runaway_par = true;
                    self.push_tokens(vec![pending.tok]);
                    break;
                }
                _ => out.push(pending),
            }
        }
        if expand {
            self.st.edef_depth -= 1;
        }
        out
    }

    /// Like `next_raw` but expands expandable tokens one at a time
    /// (respecting `\noexpand` freezing), used while scanning `\edef`
    /// bodies and other "expanded" argument contexts. Non-expandable
    /// tokens (including grouping braces, which the caller inspects) are
    /// returned as-is without executing assignments -- so this does NOT
    /// invoke `\def`/`\let`/etc as side effects; it only expands macros
    /// and expandable primitives.
    fn next_expanding_raw(&mut self) -> Option<Pending> {
        loop {
            if !self.tick() {
                return None;
            }
            let pending = self.next_raw()?;
            if pending.frozen {
                return Some(pending);
            }
            if !self.is_expandable(&pending.tok) {
                return Some(pending);
            }
            match self.step(pending.tok) {
                Step::Emit(t) => return Some(Pending { tok: t, frozen: false, origin: None }),
                Step::Continue => continue,
                Step::Eof => return None,
            }
        }
    }

    fn take_prefixes(&mut self) -> (bool, MacroFlags) {
        let global = self.st.pending_global;
        let flags = MacroFlags {
            long: self.st.pending_long,
            outer: self.st.pending_outer,
            protected: self.st.pending_protected,
            brace_delimited_last: false,
        };
        self.st.pending_global = false;
        self.st.pending_long = false;
        self.st.pending_outer = false;
        self.st.pending_protected = false;
        (global, flags)
    }

    /// Run the `\afterassignment` token, if any, now that an assignment
    /// has completed (TeXbook p. 279).
    fn finish_assignment(&mut self) {
        if let Some(t) = self.st.after_assignment.take() {
            self.push_tokens(vec![t]);
        }
    }

    fn do_def(&mut self, kind: Primitive) {
        let (global0, flags0) = self.take_prefixes();
        let global = global0 || matches!(kind, Primitive::Gdef | Primitive::Xdef);
        let expand_body = matches!(kind, Primitive::Edef | Primitive::Xdef);

        let name_tok = match self.next_raw_token() {
            Some(t) => t,
            None => return,
        };
        let warning_name = self.cs_display(&name_tok);
        let saved_status = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Defining(warning_name));
        let (params, flags1, has_body) = self.scan_param_text();
        let flags = MacroFlags { brace_delimited_last: flags1.brace_delimited_last, ..flags0 };
        let body_toks = if has_body { fold_param_tokens(self.scan_braced_group_pending(expand_body)) } else { Vec::new() };
        self.st.scanner_status = saved_status;
        let arity = params
            .iter()
            .filter_map(|p| if let ParamPart::Param(n) = p { Some(*n) } else { None })
            .max()
            .unwrap_or(0);
        let body: Vec<BodyPart> = body_toks
            .into_iter()
            .map(|t| match t.kind {
                TokenKind::Param(n) => BodyPart::Param(n),
                _ => BodyPart::Literal(t),
            })
            .collect();
        let def = Rc::new(MacroDef { params, body, flags, arity });
        self.define_cs_token(&name_tok, Meaning::Macro(def), global);
        self.finish_assignment();
    }

    fn cs_display(&self, tok: &Token) -> String {
        match &tok.kind {
            TokenKind::ControlSequence(name) => self.print_cs(name).trim_end().to_string(),
            TokenKind::ActiveChar(c) => c.to_string(),
            _ => tok.display_name(),
        }
    }

    fn define_cs_token(&mut self, name_tok: &Token, meaning: Meaning, global: bool) {
        match &name_tok.kind {
            TokenKind::ControlSequence(name) => self.st.scopes.assign_cs(name, meaning, global),
            TokenKind::ActiveChar(c) => self.st.scopes.assign_active(*c, meaning, global),
            _ => self.err("Missing control sequence inserted.", name_tok.span),
        }
    }

    fn call_macro(&mut self, call_tok: &Token, def: &Rc<MacroDef>) {
        let origin = Some(self.last_origin.unwrap_or(call_tok.span));
        if !self.tick() {
            return;
        }
        let mut args: HashMap<u8, Vec<Token>> = HashMap::new();
        let warning_name = self.cs_display(call_tok);
        let saved_status = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Matching(warning_name.clone()));
        let saved_long = self.st.matching_long;
        self.st.matching_long = def.flags.long;
        self.st.runaway_par = false;
        self.st.runaway_par_silent = false;
        let mut aborted = false;
        let mut i = 0usize;
        while i < def.params.len() {
            match &def.params[i] {
                ParamPart::Literal(lit) => {
                    // Delimiter: must match the next raw token exactly.
                    match self.next_raw_token() {
                        Some(t) if t.same_token(lit) => {}
                        Some(t) => {
                            self.err(format!("Use of {warning_name} doesn't match its definition."), t.span);
                            self.push_tokens(vec![t]);
                            aborted = true;
                            break;
                        }
                        None => {
                            aborted = true;
                            break;
                        }
                    }
                    i += 1;
                }
                ParamPart::Param(n) => {
                    // Determine if this parameter is delimited by looking
                    // at the following param-text entries.
                    let mut delim: Vec<Token> = Vec::new();
                    let mut j = i + 1;
                    while let Some(ParamPart::Literal(lit)) = def.params.get(j) {
                        delim.push(lit.clone());
                        j += 1;
                    }
                    let brace_delim_last = def.flags.brace_delimited_last && j == def.params.len();
                    let arg = if delim.is_empty() && !brace_delim_last {
                        self.scan_undelimited_arg()
                    } else if brace_delim_last {
                        // `#{`: delimited by the next `{`, which stays in
                        // the input (TeXbook p. 205).
                        let brace = Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup));
                        let mut d = delim.clone();
                        d.push(brace);
                        let arg = self.scan_delimited_arg_keep_brace(&d);
                        arg
                    } else {
                        self.scan_delimited_arg(&delim)
                    };
                    if self.st.runaway_par {
                        aborted = true;
                        break;
                    }
                    args.insert(*n, arg);
                    // `scan_delimited_arg` already consumed the following
                    // literal delimiter tokens from the input, so skip
                    // past those same `ParamPart::Literal` entries in the
                    // param text instead of matching them again.
                    i = if delim.is_empty() { i + 1 } else { j };
                }
            }
        }
        if self.st.runaway_par {
            if !self.st.runaway_par_silent {
                self.err(format!("Runaway argument?\n! Paragraph ended before {warning_name} was complete."), call_tok.span);
            }
            self.st.runaway_par = false;
            self.st.runaway_par_silent = false;
        }
        self.st.scanner_status = saved_status;
        self.st.matching_long = saved_long;
        if aborted {
            return;
        }
        // `\def\a#1{\a{#1#1}}\a x` doubles its argument on every call: the
        // step limit is far away when the token lists exhaust memory. TeX
        // runs out of main memory; so does this, at the output-token budget.
        let size: u64 = def
            .body
            .iter()
            .map(|part| match part {
                BodyPart::Literal(_) => 1,
                BodyPart::Param(n) => args.get(n).map_or(0, |a| a.len() as u64),
            })
            .sum();
        if size > self.limits.max_output_tokens {
            self.err("TeX capacity exceeded, sorry [main memory size=5000000].", call_tok.span);
            self.stopped = true;
            return;
        }
        let expansion = substitute_body(&def.body, &args);
        self.push_tokens_with_origin(expansion, origin);
    }

    /// Scan `[...]` (opening bracket already consumed), tracking nested
    /// `{`/`}` so braced content inside the optional argument is not
    /// mistaken for the closing bracket. (LaTeX does *not* nest `[`/`]`:
    /// `\foo[a[b]c]` gives `a[b` -- matched here.)
    fn scan_bracketed_optional(&mut self) -> Vec<Token> {
        let mut out = Vec::new();
        let mut brace_depth = 0i32;
        loop {
            let t = match self.next_raw_token() {
                Some(t) => t,
                None => break,
            };
            match &t.kind {
                TokenKind::Char(']', CatCode::Other) if brace_depth == 0 => break,
                TokenKind::Char(_, CatCode::BeginGroup) => {
                    brace_depth += 1;
                    out.push(t);
                }
                TokenKind::Char(_, CatCode::EndGroup) => {
                    brace_depth -= 1;
                    out.push(t);
                }
                TokenKind::ControlSequence(n) if n == "par" && !self.st.matching_long && matches!(self.st.scanner_status, ScannerStatus::Matching(_)) => {
                    self.st.runaway_par = true;
                    self.push_tokens(vec![t]);
                    break;
                }
                _ => out.push(t),
            }
        }
        out
    }

    fn scan_undelimited_arg(&mut self) -> Vec<Token> {
        // Skip leading spaces (TeXbook: spaces are ignored before an
        // undelimited argument).
        loop {
            match self.next_raw_token() {
                Some(t) => match t.kind {
                    TokenKind::Char(_, CatCode::Space) => continue,
                    TokenKind::Char(_, CatCode::BeginGroup) => {
                        self.push_tokens(vec![t]);
                        return self.scan_braced_group(false);
                    }
                    TokenKind::ControlSequence(ref n) if n == "par" && !self.st.matching_long => {
                        self.st.runaway_par = true;
                        self.push_tokens(vec![t]);
                        return Vec::new();
                    }
                    TokenKind::Char(_, CatCode::EndGroup) => {
                        self.extra_right_brace(t);
                        return Vec::new();
                    }
                    _ => return vec![t],
                },
                None => return Vec::new(),
            }
        }
    }

    /// tex.web §395 "Report an extra right brace": a `}` that would close
    /// a group opened before the macro call. TeX backs it up, reports
    /// "Argument of \foo has an extra }", and inserts `\par`, which then
    /// aborts the call with "Paragraph ended before \foo was complete"
    /// even for a `\long` macro (long_state := call).
    fn extra_right_brace(&mut self, brace: Token) {
        let name = match &self.st.scanner_status {
            ScannerStatus::Matching(n) => n.clone(),
            _ => String::new(),
        };
        self.err(format!("Argument of {name} has an extra }}."), brace.span);
        let par = Token::new(TokenKind::ControlSequence("par".into()), brace.span);
        self.push_tokens(vec![par, brace]);
        self.st.runaway_par = true;
        self.st.runaway_par_silent = false;
    }

    fn scan_delimited_arg(&mut self, delim: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut brace_depth = 0i32;
        loop {
            // Try to match the delimiter at brace depth 0.
            if brace_depth == 0 && self.peek_matches(delim) {
                self.consume_n(delim.len());
                break;
            }
            match self.next_raw_token() {
                Some(t) => {
                    match t.kind {
                        TokenKind::Char(_, CatCode::EndGroup) if brace_depth == 0 => {
                            self.extra_right_brace(t);
                            return out;
                        }
                        TokenKind::Char(_, CatCode::BeginGroup) => brace_depth += 1,
                        TokenKind::Char(_, CatCode::EndGroup) => brace_depth -= 1,
                        TokenKind::ControlSequence(ref n) if n == "par" && !self.st.matching_long => {
                            self.st.runaway_par = true;
                            self.push_tokens(vec![t]);
                            return out;
                        }
                        _ => {}
                    }
                    out.push(t);
                }
                None => break,
            }
        }
        // Strip one matching outer brace pair, per TeX's rule that a
        // delimited argument enclosed in its own braces has them removed
        // -- but only if that pair encloses the *whole* argument
        // (`{a}{b}` keeps its braces, TeXbook p. 204).
        if out.len() >= 2 && encloses_whole(&out) {
            out = out[1..out.len() - 1].to_vec();
        }
        out
    }

    /// Like `scan_delimited_arg` for the `#{` form: the final `{` of the
    /// delimiter is matched but not consumed.
    fn scan_delimited_arg_keep_brace(&mut self, delim: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut brace_depth = 0i32;
        loop {
            if brace_depth == 0 && self.peek_matches(delim) {
                self.consume_n(delim.len() - 1);
                break;
            }
            match self.next_raw_token() {
                Some(t) => {
                    match t.kind {
                        TokenKind::Char(_, CatCode::BeginGroup) => brace_depth += 1,
                        TokenKind::Char(_, CatCode::EndGroup) => brace_depth -= 1,
                        TokenKind::ControlSequence(ref n) if n == "par" && !self.st.matching_long => {
                            self.st.runaway_par = true;
                            self.push_tokens(vec![t]);
                            return out;
                        }
                        _ => {}
                    }
                    out.push(t);
                }
                None => break,
            }
        }
        if out.len() >= 2 && encloses_whole(&out) {
            out = out[1..out.len() - 1].to_vec();
        }
        out
    }

    fn peek_matches(&mut self, delim: &[Token]) -> bool {
        // Peek by pulling raw tokens into a buffer, then pushing them back.
        let mut buf = Vec::with_capacity(delim.len());
        for expected in delim {
            match self.next_raw_token() {
                Some(t) => {
                    let ok = t.same_token(expected);
                    buf.push(t);
                    if !ok {
                        self.push_tokens(buf);
                        return false;
                    }
                }
                None => {
                    self.push_tokens(buf);
                    return false;
                }
            }
        }
        self.push_tokens(buf);
        true
    }

    fn consume_n(&mut self, n: usize) {
        for _ in 0..n {
            self.next_raw_token();
        }
    }

    // ---- \let / \futurelet ---------------------------------------------

    fn do_let(&mut self) {
        let global = self.take_assignment_prefixes("let");
        let name_tok = match self.next_raw_token() {
            Some(t) => t,
            None => return,
        };
        // optional spaces, one optional '=', one optional space
        self.skip_spaces();
        if let Some(t) = self.peek_one() {
            if let TokenKind::Char('=', CatCode::Other) = t.kind {
                self.next_raw_token();
                if let Some(t2) = self.peek_one() {
                    if let TokenKind::Char(_, CatCode::Space) = t2.kind {
                        self.next_raw_token();
                    }
                }
            }
        }
        let rhs = match self.next_raw_token() {
            Some(t) => t,
            None => return,
        };
        let meaning = self.meaning_of_token(&rhs);
        self.define_cs_token(&name_tok, meaning, global);
        self.finish_assignment();
    }

    fn do_futurelet(&mut self) {
        let global = self.take_assignment_prefixes("futurelet");
        let name_tok = match self.next_raw_token() {
            Some(t) => t,
            None => return,
        };
        let t1 = self.next_raw_token();
        let t2 = self.next_raw_token();
        if let Some(t2) = &t2 {
            let meaning = self.meaning_of_token(t2);
            self.define_cs_token(&name_tok, meaning, global);
        }
        let mut reinsert = Vec::new();
        if let Some(t1) = t1 {
            reinsert.push(t1);
        }
        if let Some(t2) = t2 {
            reinsert.push(t2);
        }
        self.push_tokens(reinsert);
        self.finish_assignment();
    }

    fn meaning_of_token(&self, tok: &Token) -> Meaning {
        match &tok.kind {
            TokenKind::ControlSequence(name) => self.st.scopes.meaning(name),
            TokenKind::ActiveChar(c) => self.st.scopes.active_meaning(*c),
            _ => Meaning::CharLike(tok.clone()),
        }
    }

    fn skip_spaces(&mut self) {
        loop {
            match self.peek_one() {
                Some(t) if matches!(t.kind, TokenKind::Char(_, CatCode::Space)) => {
                    self.next_raw_token();
                }
                _ => break,
            }
        }
    }

    fn peek_one(&mut self) -> Option<Token> {
        let p = self.next_raw()?;
        let t = p.tok.clone();
        self.push_pending(vec![p]);
        Some(t)
    }

    /// Like `peek_one`, but expands macros/expandable primitives first
    /// (real TeX's `scan_int`/`scan_dimen` read every token via
    /// `get_x_token`, so e.g. `\value{name}` or a user macro that expands
    /// to digits works as a `<number>`, not just literal digit
    /// characters).
    fn peek_one_expanding(&mut self) -> Option<Token> {
        let p = self.next_expanding_raw()?;
        let t = p.tok.clone();
        self.push_pending(vec![p]);
        Some(t)
    }

    fn next_expanding_token(&mut self) -> Option<Token> {
        self.next_expanding_raw().map(|p| p.tok)
    }

    /// Read a `{name}` argument and flatten it to a plain string (each
    /// inner token contributes its display character; used for
    /// environment/counter names which are always plain letters).
    fn read_name_arg(&mut self) -> String {
        // Names are expanded (LaTeX reads them via \csname), so a macro
        // expanding to the name works too.
        let toks = self.scan_braced_group(true);
        toks.iter().map(|t| t.display_name().replace('\\', "")).collect::<String>().trim().to_string()
    }

    /// Read the next non-space token; if it is `{`, read the group and
    /// use its first token (LaTeX's `\newcommand{\foo}` vs `\newcommand\foo`).
    fn read_cs_arg(&mut self) -> Option<Token> {
        self.skip_spaces();
        let t = self.next_raw_token()?;
        if matches!(t.kind, TokenKind::Char(_, CatCode::BeginGroup)) {
            self.push_tokens(vec![t]);
            let inner = self.scan_braced_group(false);
            inner.into_iter().find(|t| !matches!(t.kind, TokenKind::Char(_, CatCode::Space)))
        } else {
            Some(t)
        }
    }

    /// Fully expand and execute `toks` in a group, collecting the content
    /// tokens they produce (used for `\settowidth`'s box content and
    /// `\label`'s `\@currentlabel`). Bounded by a frozen sentinel so the
    /// surrounding input is untouched.
    fn expand_fully(&mut self, toks: Vec<Token>, group: bool) -> Vec<Token> {
        let mut list: Vec<Pending> = Vec::with_capacity(toks.len() + 3);
        if group {
            list.push(Pending { tok: Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup)), frozen: false, origin: None });
        }
        list.extend(toks.into_iter().map(|tok| Pending { tok, frozen: false, origin: None }));
        if group {
            list.push(Pending { tok: Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)), frozen: false, origin: None });
        }
        list.push(Pending { tok: Token::synthetic(TokenKind::ControlSequence(SENTINEL.into())), frozen: true, origin: None });
        self.push_pending(list);
        let mut out = Vec::new();
        while let Some(t) = self.next_content_token() {
            if t.is_cs(SENTINEL) {
                break;
            }
            out.push(t);
        }
        out
    }

    // ---- primitive handling --------------------------------------------

    fn handle_primitive(&mut self, tok: Token, p: Primitive) -> Step {
        use Primitive::*;
        if self.prefix_pending()
            && matches!(p, Par | Begingroup | Endgroup | Aftergroup | Afterassignment | Ignorespaces | Uppercase | Lowercase | Endcsname)
        {
            let what = format!("{}{}", self.esc(), primitive_name(p));
            self.err(format!("You can't use a prefix with `{what}'."), tok.span);
            self.clear_prefixes();
        }
        match p {
            // Any token whose meaning is \relax (`\let\protect\relax`, an
            // undefined `\csname`) reaches the typesetter as `\relax`.
            Relax => Step::Emit(Token::new(TokenKind::ControlSequence("relax".into()), tok.span)),
            Par => Step::Emit(tok),
            Def | Edef | Gdef | Xdef => {
                self.do_def(p);
                Step::Continue
            }
            Let => {
                self.do_let();
                Step::Continue
            }
            Futurelet => {
                self.do_futurelet();
                Step::Continue
            }
            Global => {
                self.st.pending_global = true;
                Step::Continue
            }
            Long => {
                self.st.pending_long = true;
                Step::Continue
            }
            Outer => {
                self.st.pending_outer = true;
                Step::Continue
            }
            Protected => {
                self.st.pending_protected = true;
                Step::Continue
            }
            Expandafter => {
                let t1 = self.next_raw();
                let t2 = self.next_raw();
                if let Some(p2) = t2 {
                    if !p2.frozen && self.is_expandable(&p2.tok) {
                        match self.step(p2.tok) {
                            Step::Emit(t) => self.push_tokens(vec![t]),
                            Step::Continue => {}
                            Step::Eof => {}
                        }
                    } else {
                        self.push_pending(vec![p2]);
                    }
                }
                if let Some(p1) = t1 {
                    self.push_pending(vec![p1]);
                }
                Step::Continue
            }
            Noexpand => {
                if let Some(t) = self.next_raw_token() {
                    self.push_frozen(t);
                }
                Step::Continue
            }
            Csname => {
                let name = self.scan_csname_text(&tok);
                if !self.st.scopes.is_defined(&name) {
                    self.st.scopes.assign_cs(&name, Meaning::Primitive(Primitive::Relax), false);
                }
                self.push_tokens(vec![Token::new(TokenKind::ControlSequence(name), tok.span)]);
                Step::Continue
            }
            Endcsname => {
                self.err("Extra \\endcsname.", tok.span);
                Step::Continue
            }
            String => {
                if let Some(t) = self.next_raw_token() {
                    let s = self.string_of(&t);
                    self.push_tokens(chars_as_other(&s, tok.span));
                }
                Step::Continue
            }
            Number => {
                let n = self.scan_number();
                self.push_tokens(chars_as_other(&n.to_string(), tok.span));
                Step::Continue
            }
            Romannumeral => {
                let n = self.scan_number();
                self.push_tokens(chars_as_other(&to_roman(n), tok.span));
                Step::Continue
            }
            MeaningOf => {
                if let Some(t) = self.next_raw_token() {
                    let s = self.meaning_string(&t);
                    self.push_tokens(chars_as_other(&s, tok.span));
                }
                Step::Continue
            }
            The => {
                let toks = self.do_the(&tok);
                if self.st.edef_depth > 0 {
                    // `\the\toks` inside `\edef`: the tokens are inserted
                    // without further expansion (TeXbook p. 216).
                    let pend = toks.into_iter().map(|t| Pending { tok: t, frozen: true, origin: None }).collect();
                    self.push_pending(pend);
                } else {
                    self.push_tokens(toks);
                }
                Step::Continue
            }
            Unexpanded => {
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing("\\unexpanded".into()));
                let toks = self.scan_braced_group(false);
                self.st.scanner_status = saved;
                if self.st.edef_depth > 0 {
                    let pend = toks.into_iter().map(|t| Pending { tok: t, frozen: true, origin: None }).collect();
                    self.push_pending(pend);
                } else {
                    self.push_tokens(toks);
                }
                Step::Continue
            }
            Detokenize => {
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing("\\detokenize".into()));
                let toks = self.scan_braced_group(false);
                self.st.scanner_status = saved;
                let s = self.detokenize(&toks);
                self.push_tokens(chars_as_other(&s, tok.span));
                Step::Continue
            }
            ETeXRevision => {
                self.push_tokens(chars_as_other(".6", tok.span));
                Step::Continue
            }
            Jobname => {
                self.push_tokens(chars_as_other("texput", tok.span));
                Step::Continue
            }
            InputFile => {
                let Some(reader) = self.file_reader.clone() else {
                    return Step::Emit(tok);
                };
                let name = self.scan_file_name();
                match reader(&name) {
                    Some(text) => {
                        let id = self.st.next_source_id;
                        self.st.next_source_id += 1;
                        self.opened_files.push((id, name));
                        self.prune_exhausted();
                        self.sources.push(Input::Text(Lexer::new(Rc::from(text), id)));
                    }
                    None => self.err(format!("LaTeX Error: File `{name}' not found."), tok.span),
                }
                Step::Continue
            }
            Immediate => Step::Continue,
            Write => {
                // The text is scanned but, like a non-shipped \write, not
                // expanded (expansion could run assignments via \csname).
                self.scan_number();
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing("\\write".into()));
                self.scan_braced_group(false);
                self.st.scanner_status = saved;
                Step::Continue
            }
            Message | Errmessage => {
                let name = if matches!(p, Message) { "\\message" } else { "\\errmessage" };
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing(name.into()));
                let toks = self.scan_braced_group(true);
                self.st.scanner_status = saved;
                if matches!(p, Errmessage) {
                    let text = self.detokenize(&toks);
                    self.err(text, tok.span);
                }
                Step::Continue
            }
            Openout | Openin => {
                self.scan_number();
                self.expect_equals();
                self.scan_file_name();
                Step::Continue
            }
            Closeout | Closein => {
                self.scan_number();
                Step::Continue
            }
            Read => {
                // No input streams: `\read<n> to \cs` defines \cs as empty
                // (real TeX would read the terminal and, in batch mode, die).
                let global = self.take_assignment_prefixes("read");
                self.scan_number();
                self.skip_spaces();
                self.maybe_consume_keyword("to");
                self.skip_spaces();
                if let Some(t) = self.next_raw_token() {
                    self.warn(format!("\\read from a closed stream: {} defined as empty.", self.cs_display(&t)), t.span);
                    self.define_cs_token(&t, Meaning::Macro(Rc::new(MacroDef::simple(Vec::new()))), global);
                }
                self.finish_assignment();
                Step::Continue
            }
            Expanded => {
                // Expand like an `\edef` body, then put the result back
                // into the input (pdfTeX `back_list`), unfrozen.
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing("\\expanded".into()));
                let toks = self.scan_braced_group(true);
                self.st.scanner_status = saved;
                self.push_tokens(toks);
                Step::Continue
            }
            Pdfstrcmp => {
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing("\\pdfstrcmp".into()));
                let a = self.scan_braced_group(true);
                let b = self.scan_braced_group(true);
                self.st.scanner_status = saved;
                let (a, b) = (self.detokenize(&a), self.detokenize(&b));
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => "-1",
                    std::cmp::Ordering::Equal => "0",
                    std::cmp::Ordering::Greater => "1",
                };
                self.push_tokens(chars_as_other(r, tok.span));
                Step::Continue
            }
            Scantokens => {
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing("\\scantokens".into()));
                let toks = self.scan_braced_group(false);
                self.st.scanner_status = saved;
                let mut s = self.detokenize(&toks);
                let nl = self.st.scopes.int_param(IntParam::Newlinechar);
                if let Some(nlc) = char::from_u32(nl as u32).filter(|_| (0..256).contains(&nl)) {
                    s = s.replace(nlc, "\n");
                }
                let endline = self.st.scopes.int_param(IntParam::Endlinechar);
                if (0..256).contains(&endline) {
                    s.push('\n');
                }
                let id = self.st.next_source_id;
                self.st.next_source_id += 1;
                self.sources.push(Input::Text(Lexer::new(Rc::from(s), id)));
                Step::Continue
            }
            Afterassignment => {
                if let Some(t) = self.next_raw_token() {
                    self.st.after_assignment = Some(t);
                }
                Step::Continue
            }
            Uppercase | Lowercase => {
                let name = if matches!(p, Uppercase) { "\\uppercase" } else { "\\lowercase" };
                let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing(name.into()));
                let toks = self.scan_braced_group(false);
                self.st.scanner_status = saved;
                let upper = matches!(p, Uppercase);
                let mapped: Vec<Token> = toks
                    .into_iter()
                    .map(|t| match t.kind {
                        TokenKind::Char(c, cat) => {
                            let m = if upper { self.st.scopes.uccode(c) } else { self.st.scopes.lccode(c) };
                            if m != '\0' {
                                Token::new(TokenKind::Char(m, cat), t.span)
                            } else {
                                t
                            }
                        }
                        TokenKind::ActiveChar(c) => {
                            let m = if upper { self.st.scopes.uccode(c) } else { self.st.scopes.lccode(c) };
                            if m != '\0' {
                                Token::new(TokenKind::ActiveChar(m), t.span)
                            } else {
                                t
                            }
                        }
                        _ => t,
                    })
                    .collect();
                self.push_tokens(mapped);
                Step::Continue
            }
            Uccode | Lccode => {
                let global = self.take_assignment_prefixes(primitive_name(p));
                let code = self.scan_number();
                self.expect_equals();
                let val = self.scan_number();
                if let (Some(ch), Some(v)) = (char::from_u32(code as u32), char::from_u32(val as u32)) {
                    if matches!(p, Uccode) {
                        self.st.scopes.set_uccode(ch, v, global);
                    } else {
                        self.st.scopes.set_lccode(ch, v, global);
                    }
                } else {
                    self.err("Bad character code", tok.span);
                }
                self.finish_assignment();
                Step::Continue
            }
            Chardef | Mathchardef => {
                let global = self.take_assignment_prefixes(primitive_name(p));
                let name_tok = self.next_raw_token();
                self.expect_equals();
                let n = self.scan_number();
                if let Some(nt) = name_tok {
                    let m = if matches!(p, Chardef) { Meaning::CharDef(n) } else { Meaning::MathCharDef(n) };
                    self.define_cs_token(&nt, m, global);
                }
                self.finish_assignment();
                Step::Continue
            }
            IntPar(ip) => {
                let global = self.take_assignment_prefixes(primitive_name(p));
                self.expect_equals();
                let v = self.scan_number();
                self.st.scopes.set_int_param(ip, v, global);
                self.finish_assignment();
                Step::Continue
            }
            // Emitted (canonically named, with the source span) so the
            // typesetter sees semi-simple group boundaries too.
            Begingroup => {
                self.st.scopes.push_group();
                Step::Emit(Token::new(TokenKind::ControlSequence("begingroup".into()), tok.span))
            }
            Endgroup => {
                if self.st.scopes.depth() <= 1 {
                    self.err("Extra \\endgroup.", tok.span);
                    return Step::Continue;
                }
                let after = self.st.scopes.pop_group();
                self.push_tokens(after);
                Step::Emit(Token::new(TokenKind::ControlSequence("endgroup".into()), tok.span))
            }
            Aftergroup => {
                if let Some(t) = self.next_raw_token() {
                    self.st.scopes.queue_aftergroup(t);
                }
                Step::Continue
            }
            Catcode => {
                let global = self.take_assignment_prefixes("catcode");
                let code = self.scan_number();
                self.expect_equals();
                let val = self.scan_number();
                if let (Some(ch), Some(cat)) = (char::from_u32(code as u32), CatCode::from_u8(val as u8)) {
                    self.st.scopes.set_catcode(ch, cat, global);
                } else {
                    self.err("Invalid code (15), should be in the range 0..15.", tok.span);
                }
                self.finish_assignment();
                Step::Continue
            }
            Ignorespaces => {
                loop {
                    match self.peek_one_expanding() {
                        Some(t) if matches!(t.kind, TokenKind::Char(_, CatCode::Space)) => {
                            self.next_raw_token();
                        }
                        _ => break,
                    }
                }
                Step::Continue
            }
            Endinput => {
                // Stop reading the innermost text input (the base file or
                // a `\scantokens` pseudo-file); token lists above it are
                // still read first, as in TeX.
                for src in self.sources.iter_mut().rev() {
                    if let Input::Text(l) = src {
                        l.finish();
                        break;
                    }
                }
                Step::Continue
            }
            If | Ifcat | Ifx | Ifnum | Ifdim | Ifodd | Ifvmode | Ifhmode | Ifmmode | Ifinner | Ifcase | Iftrue
            | Iffalse | Ifdefined | Ifcsname | Ifhbox | Ifvbox | Ifvoid | Ifeof | Ifincsname => {
                self.do_conditional(p, false, &tok);
                Step::Continue
            }
            Unless => {
                // `\unless\ifnum...`: only defined over the boolean-valued
                // (non-\ifcase) conditionals per e-TeX.
                if let Some(next) = self.next_raw_token() {
                    if let TokenKind::ControlSequence(name) = &next.kind {
                        if let Meaning::Primitive(inner) = self.st.scopes.meaning(name) {
                            if is_if_primitive(inner) && inner != Ifcase {
                                self.do_conditional(inner, true, &next);
                                return Step::Continue;
                            }
                        }
                    }
                    self.err("You can't use `\\unless' before `".to_string() + &self.cs_display(&next) + "'.", next.span);
                    self.push_tokens(vec![next]);
                }
                Step::Continue
            }
            Or | Else | Fi => {
                self.handle_stray_or_else_fi(p, tok);
                Step::Continue
            }
            Newif => {
                self.do_newif();
                Step::Continue
            }
            Count | Dimen | Skip | Toks => {
                let idx = self.scan_number() as u16;
                self.finish_register_assignment_or_pass(
                    tok,
                    match p {
                        Count => RegisterKind::Count,
                        Dimen => RegisterKind::Dimen,
                        Skip => RegisterKind::Skip,
                        _ => RegisterKind::Toks,
                    },
                    idx,
                )
            }
            Countdef | Dimendef | Skipdef | Toksdef => {
                let global = self.take_assignment_prefixes(primitive_name(p));
                let name_tok = self.next_raw_token();
                self.expect_equals();
                let idx = self.scan_number() as u16;
                let kind = match p {
                    Countdef => RegisterKind::Count,
                    Dimendef => RegisterKind::Dimen,
                    Skipdef => RegisterKind::Skip,
                    _ => RegisterKind::Toks,
                };
                if let Some(nt) = name_tok {
                    self.define_cs_token(&nt, Meaning::RegisterAlias(kind, idx), global);
                }
                self.finish_assignment();
                Step::Continue
            }
            Newcount | Newdimen | Newskip | Newtoks | NewLength => {
                // plain/LaTeX allocation macros: `\newcount\foo` (LaTeX also
                // accepts `\newlength{\foo}`). Allocation is global.
                let kind = match p {
                    Newcount => RegisterKind::Count,
                    Newdimen => RegisterKind::Dimen,
                    Newskip | NewLength => RegisterKind::Skip,
                    _ => RegisterKind::Toks,
                };
                if let Some(nt) = self.read_cs_arg() {
                    if matches!(p, NewLength) && !matches!(self.meaning_of_token(&nt), Meaning::Undefined) {
                        self.err(format!("LaTeX Error: Command {} already defined.", self.cs_display(&nt)), tok.span);
                    } else {
                        let idx = self.alloc_register();
                        self.define_cs_token(&nt, Meaning::RegisterAlias(kind, idx), true);
                    }
                }
                Step::Continue
            }
            Advance | Multiply | Divide => {
                self.do_arith(p);
                Step::Continue
            }
            Numexpr | Dimexpr => {
                let v = self.scan_expr(matches!(p, Dimexpr));
                let s = if matches!(p, Dimexpr) { format!("{}pt", print_scaled(v)) } else { v.to_string() };
                // `\numexpr`/`\dimexpr` are internal quantities; used bare
                // (outside `\the`/a number context) we splice their
                // decimal text in, mirroring `\the\numexpr...\relax`.
                self.push_tokens(chars_as_other(&s, tok.span));
                Step::Continue
            }
            NewCommand | RenewCommand | ProvideCommand | DeclareRobustCommand => {
                self.do_newcommand(p, tok.span);
                Step::Continue
            }
            NewEnvironment | RenewEnvironment => {
                self.do_newenvironment(p, tok.span);
                Step::Continue
            }
            Begin => {
                self.do_begin(&tok);
                Step::Continue
            }
            End => {
                self.do_end(&tok);
                Step::Continue
            }
            Host => Step::Emit(tok),
            StopInput => {
                self.stopped = true;
                Step::Eof
            }
            Verb => self.do_verb(&tok),
            NewCounter => {
                self.do_newcounter(tok.span);
                Step::Continue
            }
            SetCounter => {
                let name = self.read_name_arg();
                let v = self.scan_counter_value_arg();
                if let Some(idx) = self.counter_register(&name) {
                    self.st.scopes.set_count(idx, v, true);
                    self.finish_assignment();
                } else {
                    self.err(format!("LaTeX Error: No counter '{name}' defined."), tok.span);
                }
                Step::Continue
            }
            AddToCounter => {
                let name = self.read_name_arg();
                let v = self.scan_counter_value_arg();
                if let Some(idx) = self.counter_register(&name) {
                    self.st.scopes.set_count(idx, self.st.scopes.count(idx) + v, true);
                    self.finish_assignment();
                } else {
                    self.err(format!("LaTeX Error: No counter '{name}' defined."), tok.span);
                }
                Step::Continue
            }
            StepCounter | RefStepCounter => {
                let name = self.read_name_arg();
                if self.counter_register(&name).is_some() {
                    self.step_counter(&name);
                    if matches!(p, RefStepCounter) {
                        // \protected@edef\@currentlabel{\p@<name>\the<name>}
                        let toks = vec![
                            Token::synthetic(TokenKind::ControlSequence("protected@edef".into())),
                            Token::synthetic(TokenKind::ControlSequence("@currentlabel".into())),
                            Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup)),
                            Token::synthetic(TokenKind::ControlSequence(format!("p@{name}"))),
                            Token::synthetic(TokenKind::ControlSequence(format!("the{name}"))),
                            Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)),
                        ];
                        self.push_tokens(toks);
                    }
                } else {
                    self.err(format!("LaTeX Error: No counter '{name}' defined."), tok.span);
                }
                Step::Continue
            }
            AddToReset | RemoveFromReset | CounterWithin | CounterWithout => {
                let star = if matches!(p, CounterWithin | CounterWithout) { self.consume_star() } else { false };
                let child = self.read_name_arg();
                let parent = self.read_name_arg();
                if self.counter_register(&child).is_none() {
                    self.err(format!("LaTeX Error: No counter '{child}' defined."), tok.span);
                    return Step::Continue;
                }
                if self.counter_register(&parent).is_none() {
                    self.err(format!("LaTeX Error: No counter '{parent}' defined."), tok.span);
                    return Step::Continue;
                }
                match p {
                    AddToReset => self.add_to_reset(&child, &parent),
                    RemoveFromReset => self.remove_from_reset(&child, &parent),
                    CounterWithin => {
                        self.add_to_reset(&child, &parent);
                        if !star {
                            // \the<child> := \the<parent>.\arabic{<child>}
                            let body = vec![
                                Token::synthetic(TokenKind::ControlSequence(format!("the{parent}"))),
                                Token::synthetic(TokenKind::Char('.', CatCode::Other)),
                            ]
                            .into_iter()
                            .chain(arabic_call_tokens(&child))
                            .collect();
                            self.st.scopes.assign_cs(&format!("the{child}"), Meaning::Macro(Rc::new(MacroDef::simple(body))), true);
                        }
                    }
                    _ => {
                        self.remove_from_reset(&child, &parent);
                        if !star {
                            self.st.scopes.assign_cs(
                                &format!("the{child}"),
                                Meaning::Macro(Rc::new(MacroDef::simple(arabic_call_tokens(&child)))),
                                true,
                            );
                        }
                    }
                }
                Step::Continue
            }
            Label => {
                let key = self.read_name_arg();
                let toks = vec![Token::synthetic(TokenKind::ControlSequence("@currentlabel".into()))];
                let content: Vec<Token> = self.expand_fully(toks, true).into_iter().filter(|t| !is_group_token(t)).collect();
                let current_label = crate::tokens_to_display_string(&content);
                self.labels.push(LabelRecord { key, current_label, span: tok.span });
                Step::Continue
            }
            Value => {
                let name = self.read_name_arg();
                if self.counter_register(&name).is_none() {
                    self.err(format!("LaTeX Error: No counter '{name}' defined."), tok.span);
                }
                self.push_tokens(vec![Token::new(TokenKind::ControlSequence(format!("c@{name}")), tok.span)]);
                Step::Continue
            }
            Arabic | RomanLower | RomanUpper | AlphLower | AlphUpper | Fnsymbol => {
                let name = self.read_name_arg();
                let v = match self.counter_register(&name) {
                    Some(idx) => self.st.scopes.count(idx),
                    None => {
                        self.err(format!("LaTeX Error: No counter '{name}' defined."), tok.span);
                        0
                    }
                };
                let s = match p {
                    Arabic => Some(v.to_string()),
                    RomanLower => Some(to_roman(v)),
                    RomanUpper => Some(to_roman(v).to_ascii_uppercase()),
                    AlphLower => to_alph(v, false),
                    AlphUpper => to_alph(v, true),
                    Fnsymbol => to_fnsymbol(v),
                    _ => unreachable!(),
                };
                match s {
                    Some(s) => self.push_tokens(chars_as_other(&s, tok.span)),
                    None => self.err("LaTeX Error: Counter too large.", tok.span),
                }
                Step::Continue
            }
            SetToWidth | SetToHeight | SetToDepth => {
                let target = self.read_cs_arg();
                let toks = self.scan_braced_group(false);
                let content = self.expand_fully(toks, true);
                let v = match p {
                    SetToWidth => self.measurer.width(&content),
                    SetToHeight => self.measurer.height(&content),
                    _ => self.measurer.depth(&content),
                };
                if let Some(t) = target {
                    match self.meaning_of_token(&t) {
                        Meaning::RegisterAlias(RegisterKind::Skip, idx) => self.st.scopes.set_skip(idx, Glue::fixed(v), false),
                        Meaning::RegisterAlias(RegisterKind::Dimen, idx) => self.st.scopes.set_dimen(idx, v, false),
                        _ => self.err("Missing number, treated as zero.", t.span),
                    }
                }
                Step::Continue
            }
            DefineKey => {
                self.do_define_key(tok.span);
                Step::Continue
            }
            SetKeys => {
                self.do_setkeys(tok.span);
                Step::Continue
            }
        }
    }

    /// TeX's `scan_file_name`: optional spaces, then expanded character
    /// tokens up to a space (consumed) or a non-character; LaTeX's
    /// `\input{name}` form reads a braced group instead.
    fn scan_file_name(&mut self) -> String {
        let mut name = String::new();
        loop {
            match self.peek_one_expanding() {
                Some(t) if matches!(t.kind, TokenKind::Char(_, CatCode::Space)) => {
                    self.next_raw_token();
                }
                _ => break,
            }
        }
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::Char(_, CatCode::BeginGroup)) {
                let toks = self.scan_braced_group(true);
                return self.detokenize(&toks).trim().to_string();
            }
        }
        loop {
            match self.next_expanding_raw() {
                Some(p) => match p.tok.kind {
                    TokenKind::Char(_, CatCode::Space) => break,
                    TokenKind::Char(c, _) => name.push(c),
                    _ => {
                        self.push_pending(vec![p]);
                        break;
                    }
                },
                None => break,
            }
        }
        name
    }

    fn consume_star(&mut self) -> bool {
        if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('*', CatCode::Other)) {
                self.next_raw_token();
                return true;
            }
        }
        false
    }

    fn alloc_register(&mut self) -> u16 {
        let idx = self.st.next_free_register;
        self.st.next_free_register += 1;
        idx
    }

    /// Scan the text of `\csname ... \endcsname` (fully expanding), and
    /// return the name.
    fn scan_csname_text(&mut self, tok: &Token) -> String {
        let mut name = String::new();
        self.st.in_csname += 1;
        loop {
            match self.next_expanding_raw() {
                Some(p) if p.tok.is_cs("endcsname") => break,
                Some(p) => match p.tok.kind {
                    TokenKind::Char(c, _) => name.push(c),
                    TokenKind::ActiveChar(c) => name.push(c),
                    _ => {
                        self.err("Missing \\endcsname inserted.", p.tok.span);
                        self.push_pending(vec![p]);
                        break;
                    }
                },
                None => {
                    self.err("Missing \\endcsname inserted.", tok.span);
                    break;
                }
            }
        }
        self.st.in_csname -= 1;
        name
    }

    /// `\detokenize`/`\scantokens` text: every token as `\string` would
    /// show it, with a space after control words (tex.web `print_cs`).
    fn detokenize(&self, toks: &[Token]) -> String {
        let mut s = String::new();
        for t in toks {
            match &t.kind {
                TokenKind::ControlSequence(name) => s.push_str(&self.print_cs(name)),
                TokenKind::ActiveChar(c) => s.push(*c),
                // show_token_list doubles catcode-6 characters.
                TokenKind::Char(c, CatCode::Param) => {
                    s.push(*c);
                    s.push(*c);
                }
                TokenKind::Char(c, _) => s.push(*c),
                TokenKind::Param(n) => {
                    s.push('#');
                    s.push_str(&n.to_string());
                }
                TokenKind::Eof => {}
            }
        }
        s
    }

    // ---- LaTeX layer: counters -----------------------------------------

    fn counter_register(&self, name: &str) -> Option<u16> {
        match self.st.scopes.meaning_ref(&format!("c@{name}")) {
            Some(Meaning::RegisterAlias(RegisterKind::Count, idx)) => Some(*idx),
            _ => None,
        }
    }

    /// `\stepcounter`: globally add one, then reset every counter in this
    /// counter's `\@addtoreset` list (recursively, LaTeX's `\@stpelt`).
    fn step_counter(&mut self, name: &str) {
        if let Some(idx) = self.counter_register(name) {
            self.st.scopes.set_count(idx, self.st.scopes.count(idx) + 1, true);
        }
        self.reset_children(name);
    }

    fn reset_children(&mut self, name: &str) {
        let children = self.st.counter_children.get(name).cloned().unwrap_or_default();
        for child in children {
            if let Some(idx) = self.counter_register(&child) {
                self.st.scopes.set_count(idx, 0, true);
            }
            self.reset_children(&child);
        }
    }

    fn add_to_reset(&mut self, child: &str, parent: &str) {
        let list = Rc::make_mut(&mut self.st.counter_children).entry(parent.to_string()).or_default();
        if !list.iter().any(|c| c == child) {
            list.push(child.to_string());
        }
    }

    fn remove_from_reset(&mut self, child: &str, parent: &str) {
        if self.st.counter_children.get(parent).is_some_and(|list| list.iter().any(|c| c == child)) {
            if let Some(list) = Rc::make_mut(&mut self.st.counter_children).get_mut(parent) {
                list.retain(|c| c != child);
            }
        }
    }

    /// Read a `{...}` argument meant to hold a `<number>`-shaped value
    /// (used by `\setcounter`/`\addtocounter`): expands it, then parses
    /// the resulting characters as a decimal integer.
    fn scan_counter_value_arg(&mut self) -> i64 {
        // The argument is a `<number>` expression, which may itself
        // contain `\value{...}` or other expandable constructs (e.g.
        // `\setcounter{bar}{\value{foo}}`) -- so feed the raw braced
        // tokens back through `scan_number`'s full expanding scanner
        // rather than naively collecting character tokens.
        let mut toks = self.scan_braced_group(false);
        // A trailing sentinel keeps `scan_number`'s optional-space
        // lookahead from falling through this temporary source into the
        // surrounding (real) input stream once the braced content is
        // exhausted.
        toks.push(Token::synthetic(TokenKind::ControlSequence("relax".to_string())));
        self.prune_exhausted();
        let depth = self.sources.len();
        self.push_tokens(toks);
        let v = self.scan_number();
        // Discard whatever remains of the temporary source we just
        // pushed (the sentinel, plus any leftovers) -- it must never leak
        // into the surrounding stream.
        while self.sources.len() > depth {
            self.sources.pop();
        }
        v
    }

    fn do_newcounter(&mut self, span: Span) {
        let name = self.read_name_arg();
        // optional [within]
        let within = if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('[', CatCode::Other)) {
                self.next_raw_token();
                Some(self.scan_bracketed_optional().iter().map(|t| t.display_name().replace('\\', "")).collect::<String>())
            } else {
                None
            }
        } else {
            None
        };
        if self.counter_register(&name).is_some() {
            self.err(format!("LaTeX Error: Command \\c@{name} already defined."), span);
            return;
        }
        let idx = self.alloc_register();
        self.st.scopes.assign_cs(&format!("c@{name}"), Meaning::RegisterAlias(RegisterKind::Count, idx), true);
        self.st.scopes.set_count(idx, 0, true);
        // \the<name> := \arabic{<name>}; \p@<name> := \@empty
        self.st.scopes.assign_cs(&format!("the{name}"), Meaning::Macro(Rc::new(MacroDef::simple(arabic_call_tokens(&name)))), true);
        let empty = self.st.scopes.meaning("@empty");
        self.st.scopes.assign_cs(&format!("p@{name}"), empty, true);
        if let Some(parent) = within {
            if self.counter_register(&parent).is_some() {
                self.add_to_reset(&name, &parent);
            } else {
                self.err(format!("LaTeX Error: No counter '{parent}' defined."), span);
            }
        }
    }

    // ---- LaTeX layer: \newcommand & friends -----------------------------

    fn do_newcommand(&mut self, kind: Primitive, span: Span) {
        // `*`: the command is *not* `\long` (LaTeX: `\newcommand*`).
        let star = self.consume_star();
        let name_tok = match self.read_cs_arg() {
            Some(t) => t,
            None => return,
        };
        // pdflatex: `\newcommand{oops}` -> "Missing control sequence
        // inserted." (after its \@ifdefinable number noise); nothing usable is
        // defined.
        let valid_name = matches!(name_tok.kind, TokenKind::ControlSequence(_) | TokenKind::ActiveChar(_));
        if !valid_name {
            self.err("Missing control sequence inserted.", span);
        }
        let already_defined = valid_name && !matches!(self.meaning_of_token(&name_tok), Meaning::Undefined);
        match kind {
            _ if !valid_name => {}
            Primitive::NewCommand if already_defined => {
                self.err(format!("LaTeX Error: Command {} already defined.", self.cs_display(&name_tok)), span);
            }
            Primitive::RenewCommand if !already_defined => {
                self.err(format!("LaTeX Error: Command {} undefined.", self.cs_display(&name_tok)), span);
            }
            _ => {}
        }
        // For \providecommand when already defined, we still parse (and
        // discard) the rest of the syntax below to keep the input stream
        // in sync with what real TeX would have consumed.
        let nargs = self.scan_optional_bracket_number();
        let default = if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('[', CatCode::Other)) {
                self.next_raw_token();
                Some(self.scan_bracketed_optional())
            } else {
                None
            }
        } else {
            None
        };
        let warning_name = self.cs_display(&name_tok);
        let saved_status = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Defining(warning_name));
        let body_toks = fold_param_tokens(self.scan_braced_group_pending(false));
        self.st.scanner_status = saved_status;
        if matches!(nargs, Some(n) if n > 9) {
            self.err("You already have nine parameters.", span);
        }
        let arity = nargs.unwrap_or(0).clamp(0, 9) as u8;
        let mut illegal = false;
        let body: Vec<BodyPart> = body_toks
            .into_iter()
            .filter_map(|t| match t.kind {
                // TeX: "Illegal parameter number in definition of \foo."; the
                // parameter token is dropped.
                TokenKind::Param(n) if n > arity => {
                    illegal = true;
                    None
                }
                TokenKind::Param(n) => Some(BodyPart::Param(n)),
                _ => Some(BodyPart::Literal(t)),
            })
            .collect();
        if illegal && valid_name {
            let name = self.cs_display(&name_tok);
            self.err(format!("Illegal parameter number in definition of {name}."), span);
        }
        if !valid_name {
            return;
        }
        // `\newcommand` on a defined name and `\providecommand` on a
        // defined name leave the old meaning (LaTeX's `\@ifdefinable`
        // gobbles the definition); `\renewcommand` on an undefined name
        // errors but still defines.
        if matches!(kind, Primitive::ProvideCommand | Primitive::NewCommand) && already_defined {
            return;
        }
        if matches!(kind, Primitive::DeclareRobustCommand) {
            // \DeclareRobustCommand\foo: \foo -> \protect\foo<space>, the
            // real definition living in the control sequence named
            // "foo " (with a trailing space), exactly as LaTeX does it.
            if let TokenKind::ControlSequence(name) = &name_tok.kind {
                let inner = Token::new(TokenKind::ControlSequence(format!("{name} ")), name_tok.span);
                self.define_latex_command(&inner, arity, default, body, !star);
                let outer_body = vec![Token::synthetic(TokenKind::ControlSequence("protect".into())), inner];
                self.define_cs_token(&name_tok, Meaning::Macro(Rc::new(MacroDef::simple(outer_body))), false);
                return;
            }
        }
        self.define_latex_command(&name_tok, arity, default, body, !star);
    }

    /// LaTeX's `\@yargdef`/`\@xargdef` (ltdefns.dtx): bind `target` to a
    /// command with `arity` parameters. With a default for `#1`, `target`
    /// becomes `\@protected@testopt <target> \\<target> {<default>}` and
    /// the body lives in the control sequence named `\string<target>`
    /// with parameter text `[#1]#2...` -- the same two macros real LaTeX
    /// builds, so `\meaning`/`\ifx`/error messages agree with it.
    fn define_latex_command(&mut self, target: &Token, arity: u8, default: Option<Vec<Token>>, body: Vec<BodyPart>, long: bool) {
        let flags = MacroFlags { long, ..MacroFlags::default() };
        match default {
            None => {
                // ltdefns `\@yargd@f`: `\ifnum#1>\z@ \l@ngrel@x \fi` -- a
                // command without parameters is never `\long`.
                let flags = MacroFlags { long: long && arity > 0, ..flags };
                let params: Vec<ParamPart> = (1..=arity).map(ParamPart::Param).collect();
                self.define_cs_token(target, Meaning::Macro(Rc::new(MacroDef { params, body, flags, arity })), false);
            }
            Some(default_toks) => {
                let arity = arity.max(1);
                let inner = Token::new(TokenKind::ControlSequence(self.string_of(target)), target.span);
                let mut params = vec![
                    ParamPart::Literal(Token::synthetic(TokenKind::Char('[', CatCode::Other))),
                    ParamPart::Param(1),
                    ParamPart::Literal(Token::synthetic(TokenKind::Char(']', CatCode::Other))),
                ];
                params.extend((2..=arity).map(ParamPart::Param));
                self.define_cs_token(&inner, Meaning::Macro(Rc::new(MacroDef { params, body, flags, arity })), false);
                let mut outer = vec![
                    Token::synthetic(TokenKind::ControlSequence("@protected@testopt".into())),
                    target.clone(),
                    inner,
                    Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup)),
                ];
                outer.extend(default_toks);
                outer.push(Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)));
                self.define_cs_token(target, Meaning::Macro(Rc::new(MacroDef::simple(outer))), false);
            }
        }
    }

    fn scan_optional_bracket_number(&mut self) -> Option<i64> {
        self.skip_spaces();
        if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('[', CatCode::Other)) {
                self.next_raw_token();
                let n = self.scan_number();
                // consume the closing ']'
                if let Some(t2) = self.peek_one() {
                    if matches!(t2.kind, TokenKind::Char(']', CatCode::Other)) {
                        self.next_raw_token();
                    }
                }
                return Some(n);
            }
        }
        None
    }

    fn do_newenvironment(&mut self, kind: Primitive, span: Span) {
        let star = self.consume_star();
        let name = self.read_name_arg();
        let exists = self.st.scopes.is_defined(&name);
        match kind {
            Primitive::NewEnvironment if exists => {
                self.err(format!("LaTeX Error: Command \\{name} already defined."), span);
            }
            Primitive::RenewEnvironment if !exists => {
                self.err(format!("LaTeX Error: Environment {name} undefined."), span);
            }
            _ => {}
        }
        let nargs = self.scan_optional_bracket_number();
        let default = if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('[', CatCode::Other)) {
                self.next_raw_token();
                Some(self.scan_bracketed_optional())
            } else {
                None
            }
        } else {
            None
        };
        let saved_status = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Defining(format!("\\{name}")));
        let begin_toks = fold_param_tokens(self.scan_braced_group_pending(false));
        self.st.scanner_status = ScannerStatus::Defining(format!("\\end{name}"));
        let end_toks = fold_param_tokens(self.scan_braced_group_pending(false));
        self.st.scanner_status = saved_status;
        if matches!(kind, Primitive::NewEnvironment) && exists {
            return;
        }
        let arity = nargs.unwrap_or(0).clamp(0, 9) as u8;
        let to_body = |toks: Vec<Token>| -> Vec<BodyPart> {
            toks.into_iter()
                .map(|t| match t.kind {
                    TokenKind::Param(n) => BodyPart::Param(n),
                    _ => BodyPart::Literal(t),
                })
                .collect()
        };
        let begin_body = to_body(begin_toks);
        let end_body = to_body(end_toks);
        let begin_tok = Token::synthetic(TokenKind::ControlSequence(name.clone()));
        self.define_latex_command(&begin_tok, arity, default, begin_body, !star);
        let flags = MacroFlags { long: !star, ..MacroFlags::default() };
        self.st.scopes.assign_cs(
            &format!("end{name}"),
            Meaning::Macro(Rc::new(MacroDef { params: Vec::new(), body: end_body, flags, arity: 0 })),
            false,
        );
    }

    /// `\begin{name}`: LaTeX opens a group, records `\@currenvir`, then
    /// runs `\name`. `\begin{document}` runs `\@begindocumenthook` and
    /// emits a `\document` marker; verbatim environments read raw text.
    fn do_begin(&mut self, tok: &Token) {
        let name = self.read_name_arg();
        if name == "document" {
            self.push_pending(vec![
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("@begindocumenthook".into())), frozen: false, origin: None },
                // ltfiles: after the hook, \AtBeginDocument runs its argument
                // immediately.
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("global".into())), frozen: false, origin: None },
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("let".into())), frozen: false, origin: None },
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("AtBeginDocument".into())), frozen: false, origin: None },
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("@firstofone".into())), frozen: false, origin: None },
                Pending { tok: Token::new(TokenKind::ControlSequence("document".into()), tok.span), frozen: true, origin: None },
            ]);
            return;
        }
        if name == "verbatim" || name == "verbatim*" {
            self.do_verbatim_env(&name, tok);
            return;
        }
        if !self.st.scopes.is_defined(&name) {
            // LaTeX: "Environment name undefined." -- we still open the
            // group and pass `\name` through, since many environments are
            // handled by the typesetting layer rather than by macros.
            self.warn(format!("Environment {} undefined (passed through to the typesetter).", shown_name(&name)), tok.span);
        }
        self.st.scopes.push_group();
        let cur = Meaning::Macro(Rc::new(MacroDef::simple(chars_as_other(&name, Span::synthetic()))));
        self.st.scopes.assign_cs("@currenvir", cur, false);
        self.push_tokens(vec![Token::new(TokenKind::ControlSequence(name), tok.span)]);
    }

    fn do_end(&mut self, tok: &Token) {
        let name = self.read_name_arg();
        if name == "document" {
            self.push_pending(vec![
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("@enddocumenthook".into())), frozen: false, origin: None },
                Pending { tok: Token::new(TokenKind::ControlSequence("enddocument".into()), tok.span), frozen: true, origin: None },
                Pending { tok: Token::synthetic(TokenKind::ControlSequence("flashtex@stop".into())), frozen: false, origin: None },
            ]);
            return;
        }
        // \@checkend: the current environment must be this one. Compared
        // part by part, so a runaway loop over a 100k-character name does
        // not rebuild the name for every `\end`.
        let current = match self.st.scopes.meaning_ref("@currenvir") {
            Some(Meaning::Macro(def)) => Some(def.clone()),
            _ => None,
        };
        let body: &[BodyPart] = current.as_deref().map_or(&[], |def| &def.body);
        if !body_spells(body, &name) {
            let current = body_display(body, SHOWN_NAME_CHARS + 1);
            self.err(
                format!("LaTeX Error: \\begin{{{}}} ended by \\end{{{}}}.", shown_name(&current), shown_name(&name)),
                tok.span,
            );
        }
        if self.st.scopes.depth() <= 1 {
            self.err(format!("LaTeX Error: \\end{{{}}} without matching \\begin.", shown_name(&name)), tok.span);
            self.push_tokens(vec![Token::new(TokenKind::ControlSequence(format!("end{name}")), tok.span)]);
            return;
        }
        self.push_tokens(vec![
            Token::new(TokenKind::ControlSequence(format!("end{name}")), tok.span),
            Token::synthetic(TokenKind::ControlSequence("endgroup".into())),
        ]);
    }

    /// `\verb<delim>...<delim>` (and `\verb*`): read raw characters from
    /// the base lexer, ignoring catcodes. Output: the `\verb` token, the
    /// delimiter, the content as catcode-12 characters, the delimiter.
    fn do_verb(&mut self, tok: &Token) -> Step {
        let star = self.consume_star();
        self.prune_exhausted();
        if self.sources.len() != 1 {
            self.err("LaTeX Error: \\verb illegal in command argument.", tok.span);
            return Step::Continue;
        }
        let (delim, content, start) = {
            let lexer = match self.sources.last_mut() {
                Some(Input::Text(l)) => l,
                _ => unreachable!(),
            };
            let start = lexer.pos();
            let delim = match lexer.read_raw_char() {
                Some(c) => c,
                None => return Step::Continue,
            };
            let content = lexer.read_verb_until(delim);
            (delim, content, start)
        };
        let content = match content {
            Some(c) => c,
            None => {
                self.err("LaTeX Error: \\verb ended by end of line.", tok.span);
                String::new()
            }
        };
        let mut out = vec![Token::new(TokenKind::ControlSequence(if star { "verb*".into() } else { "verb".into() }), tok.span)];
        let mut pos = start;
        out.push(Token::new(TokenKind::Char(delim, CatCode::Other), Span::new(0, pos as u32, (pos + delim.len_utf8()) as u32)));
        pos += delim.len_utf8();
        for c in content.chars() {
            out.push(Token::new(TokenKind::Char(c, CatCode::Other), Span::new(0, pos as u32, (pos + c.len_utf8()) as u32)));
            pos += c.len_utf8();
        }
        out.push(Token::new(TokenKind::Char(delim, CatCode::Other), Span::new(0, pos as u32, (pos + delim.len_utf8()) as u32)));
        let pend = out.into_iter().map(|tok| Pending { tok, frozen: true, origin: None }).collect();
        self.push_pending(pend);
        Step::Continue
    }

    fn do_verbatim_env(&mut self, name: &str, tok: &Token) {
        self.prune_exhausted();
        if self.sources.len() != 1 {
            self.err("LaTeX Error: verbatim illegal in command argument.", tok.span);
            return;
        }
        let end_marker = format!("\\end{{{name}}}");
        let (text, start) = {
            let lexer = match self.sources.last_mut() {
                Some(Input::Text(l)) => l,
                _ => unreachable!(),
            };
            let start = lexer.pos();
            (lexer.read_raw_until_str(&end_marker), start)
        };
        let text = match text {
            Some(t) => t,
            None => {
                self.err(format!("Runaway text?\n! File ended while scanning text of \\begin{{{name}}}."), tok.span);
                String::new()
            }
        };
        // The newline right after \begin{verbatim} is not content.
        let (text, start) = match text.strip_prefix('\n') {
            Some(rest) => (rest.to_string(), start + 1),
            None => (text, start),
        };
        let mut out = vec![Token::new(TokenKind::ControlSequence(name.to_string()), tok.span)];
        let mut pos = start;
        for c in text.chars() {
            let span = Span::new(0, pos as u32, (pos + c.len_utf8()) as u32);
            if c == '\n' {
                out.push(Token::new(TokenKind::ControlSequence("par".into()), span));
            } else {
                out.push(Token::new(TokenKind::Char(c, CatCode::Other), span));
            }
            pos += c.len_utf8();
        }
        out.push(Token::new(TokenKind::ControlSequence(format!("end{name}")), tok.span));
        let pend = out.into_iter().map(|tok| Pending { tok, frozen: true, origin: None }).collect();
        self.push_pending(pend);
    }

    // ---- keyval ---------------------------------------------------------

    /// `\define@key{family}{key}[default]{code}` (keyval.sty): defines
    /// `\KV@family@key` as a one-parameter macro, and with a default also
    /// `\KV@family@key@default` -> `\KV@family@key{default}`.
    fn do_define_key(&mut self, _span: Span) {
        let family = self.read_name_arg();
        let key = self.read_name_arg();
        let default = if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('[', CatCode::Other)) {
                self.next_raw_token();
                Some(self.scan_bracketed_optional())
            } else {
                None
            }
        } else {
            None
        };
        let macro_name = format!("KV@{family}@{key}");
        let saved_status = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Defining(format!("\\{macro_name}")));
        let body_toks = fold_param_tokens(self.scan_braced_group_pending(false));
        self.st.scanner_status = saved_status;
        let body: Vec<BodyPart> = body_toks
            .into_iter()
            .map(|t| match t.kind {
                TokenKind::Param(n) => BodyPart::Param(n),
                _ => BodyPart::Literal(t),
            })
            .collect();
        let def = MacroDef {
            params: vec![ParamPart::Param(1)],
            body,
            // keyval.sty: \long\@namedef{KV@#1@#2}##1{#4}.
            flags: MacroFlags { long: true, ..MacroFlags::default() },
            arity: 1,
        };
        self.st.scopes.assign_cs(&macro_name, Meaning::Macro(Rc::new(def)), false);
        if let Some(default_toks) = default {
            let mut body = vec![
                Token::synthetic(TokenKind::ControlSequence(macro_name.clone())),
                Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup)),
            ];
            body.extend(default_toks);
            body.push(Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)));
            let flags = MacroFlags { long: true, ..MacroFlags::default() };
            let def = MacroDef { params: Vec::new(), body: body.into_iter().map(BodyPart::Literal).collect(), flags, arity: 0 };
            self.st.scopes.assign_cs(&format!("{macro_name}@default"), Meaning::Macro(Rc::new(def)), false);
        }
    }

    /// `\setkeys{family}{key=value, key2, ...}`: for each item, strip
    /// surrounding spaces and one level of braces from the value, then
    /// call `\KV@family@key{value}` (or the `@default` macro when no `=`
    /// is given). Unknown keys are a keyval error.
    fn do_setkeys(&mut self, span: Span) {
        let family = self.read_name_arg();
        let list = self.scan_braced_group(false);
        let mut calls: Vec<Token> = Vec::new();
        for item in split_top_level(&list, ',') {
            let item = trim_spaces(item);
            if item.is_empty() {
                continue;
            }
            let mut parts = split_top_level(&item, '=').into_iter();
            let key_toks = trim_spaces(parts.next().unwrap_or_default());
            let key: String = key_toks.iter().map(|t| t.display_name()).collect();
            let value: Option<Vec<Token>> = parts.next().map(|v| {
                // Everything after the first `=` (a value may contain `=`).
                let mut v = v;
                for extra in parts.by_ref() {
                    v.push(Token::synthetic(TokenKind::Char('=', CatCode::Other)));
                    v.extend(extra);
                }
                let v = trim_spaces(v);
                if v.len() >= 2 && encloses_whole(&v) {
                    v[1..v.len() - 1].to_vec()
                } else {
                    v
                }
            });
            let macro_name = format!("KV@{family}@{key}");
            if !self.st.scopes.is_defined(&macro_name) {
                self.err(format!("Package keyval Error: {key} undefined."), span);
                continue;
            }
            match value {
                Some(v) => {
                    calls.push(Token::new(TokenKind::ControlSequence(macro_name), span));
                    calls.push(Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup)));
                    calls.extend(v);
                    calls.push(Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)));
                }
                None => {
                    let d = format!("{macro_name}@default");
                    if self.st.scopes.is_defined(&d) {
                        calls.push(Token::new(TokenKind::ControlSequence(d), span));
                    } else {
                        self.err(format!("Package keyval Error: No value specified for key `{key}'."), span);
                    }
                }
            }
        }
        self.push_tokens(calls);
    }

    // ---- registers --------------------------------------------------------

    fn handle_register_ref(&mut self, tok: Token, kind: RegisterKind, idx: u16) -> Step {
        self.finish_register_assignment_or_pass(tok, kind, idx)
    }

    /// After reading `\count<idx>` (or a `\countdef`-alias) in main
    /// control this is an assignment `<idx> = <value>` (the `=` and
    /// spaces are optional in TeX: `\count0 5` is legal).
    fn finish_register_assignment_or_pass(&mut self, tok: Token, kind: RegisterKind, idx: u16) -> Step {
        let global = self.take_assignment_prefixes(match kind {
            RegisterKind::Count => "count",
            RegisterKind::Dimen => "dimen",
            RegisterKind::Skip => "skip",
            RegisterKind::Toks => "toks",
        });
        self.expect_equals();
        match kind {
            RegisterKind::Count => {
                let v = self.scan_number();
                self.st.scopes.set_count(idx, v, global);
            }
            RegisterKind::Dimen => {
                let v = self.scan_dimen();
                self.st.scopes.set_dimen(idx, v, global);
            }
            RegisterKind::Skip => {
                let v = self.scan_glue();
                self.st.scopes.set_skip(idx, v, global);
            }
            RegisterKind::Toks => {
                // `\toks0={...}` or `\toks0=\toks1` / `\toks0=\the\toks1`.
                self.skip_spaces();
                let next = self.peek_one_expanding();
                match next.map(|t| t.kind) {
                    Some(TokenKind::Char(_, CatCode::BeginGroup)) => {
                        let name = self.cs_display(&tok);
                        let saved = std::mem::replace(&mut self.st.scanner_status, ScannerStatus::Absorbing(name));
                        let v = self.scan_braced_group(false);
                        self.st.scanner_status = saved;
                        self.st.scopes.set_toks(idx, v, global);
                    }
                    Some(TokenKind::ControlSequence(_)) => {
                        let t = self.next_raw_token().unwrap();
                        match self.register_ref_of(&t) {
                            Some((RegisterKind::Toks, src)) => {
                                let v = self.st.scopes.toks(src);
                                self.st.scopes.set_toks(idx, v, global);
                            }
                            _ => self.err("Missing { inserted.", t.span),
                        }
                    }
                    _ => self.err("Missing { inserted.", tok.span),
                }
            }
        }
        self.finish_assignment();
        Step::Continue
    }

    fn expect_equals(&mut self) {
        self.skip_spaces();
        if let Some(t) = self.peek_one() {
            if matches!(t.kind, TokenKind::Char('=', CatCode::Other)) {
                self.next_raw_token();
            }
        }
    }

    // ---- \the -------------------------------------------------------------

    fn do_the(&mut self, the_tok: &Token) -> Vec<Token> {
        let tok = match self.next_expanding_token() {
            Some(t) => t,
            None => return Vec::new(),
        };
        if let Some((kind, idx)) = self.register_ref_of(&tok) {
            return match kind {
                RegisterKind::Count => chars_as_other(&self.st.scopes.count(idx).to_string(), tok.span),
                RegisterKind::Dimen => chars_as_other(&format!("{}pt", print_scaled(self.st.scopes.dimen(idx))), tok.span),
                RegisterKind::Skip => chars_as_other(&glue_to_string(self.st.scopes.skip(idx)), tok.span),
                RegisterKind::Toks => self.st.scopes.toks(idx),
            };
        }
        // Other internal integers: \chardef'd tokens, \catcode/\uccode/
        // \lccode tables, integer parameters, \numexpr/\dimexpr.
        match self.meaning_of_token(&tok) {
            Meaning::CharDef(n) | Meaning::MathCharDef(n) => chars_as_other(&n.to_string(), tok.span),
            Meaning::Primitive(Primitive::Catcode) => {
                let c = self.scan_number();
                let v = char::from_u32(c as u32).map(|ch| self.st.scopes.catcode(ch) as u8 as i64).unwrap_or(0);
                chars_as_other(&v.to_string(), tok.span)
            }
            Meaning::Primitive(Primitive::Uccode) => {
                let c = self.scan_number();
                let v = char::from_u32(c as u32).map(|ch| self.st.scopes.uccode(ch) as i64).unwrap_or(0);
                chars_as_other(&v.to_string(), tok.span)
            }
            Meaning::Primitive(Primitive::Lccode) => {
                let c = self.scan_number();
                let v = char::from_u32(c as u32).map(|ch| self.st.scopes.lccode(ch) as i64).unwrap_or(0);
                chars_as_other(&v.to_string(), tok.span)
            }
            Meaning::Primitive(Primitive::IntPar(p)) => chars_as_other(&self.st.scopes.int_param(p).to_string(), tok.span),
            Meaning::Primitive(Primitive::Numexpr) => {
                let v = self.scan_expr(false);
                chars_as_other(&v.to_string(), tok.span)
            }
            Meaning::Primitive(Primitive::Dimexpr) => {
                let v = self.scan_expr(true);
                chars_as_other(&format!("{}pt", print_scaled(v)), tok.span)
            }
            _ => {
                // An internal quantity this crate doesn't model (e.g.
                // `\the\parindent`, `\the\font`): pass `\the` and the
                // token through to the typesetter untouched.
                let pend = vec![Pending { tok: the_tok.clone(), frozen: true, origin: None }, Pending { tok, frozen: true, origin: None }];
                self.push_pending(pend);
                Vec::new()
            }
        }
    }

    fn register_ref_of(&mut self, tok: &Token) -> Option<(RegisterKind, u16)> {
        match &tok.kind {
            TokenKind::ControlSequence(name) => match self.st.scopes.meaning(name) {
                Meaning::RegisterAlias(kind, idx) => Some((kind, idx)),
                Meaning::Let(inner) => match *inner {
                    Meaning::RegisterAlias(kind, idx) => Some((kind, idx)),
                    _ => None,
                },
                Meaning::Primitive(p) => {
                    let kind = match p {
                        Primitive::Count => RegisterKind::Count,
                        Primitive::Dimen => RegisterKind::Dimen,
                        Primitive::Skip => RegisterKind::Skip,
                        Primitive::Toks => RegisterKind::Toks,
                        _ => return None,
                    };
                    let idx = self.scan_number() as u16;
                    Some((kind, idx))
                }
                _ => None,
            },
            _ => None,
        }
    }

    // ---- \advance / \multiply / \divide --------------------------------

    fn do_arith(&mut self, op: Primitive) {
        let global = self.take_assignment_prefixes(primitive_name(op));
        let tok = match self.next_expanding_token() {
            Some(t) => t,
            None => return,
        };
        let (kind, idx) = match self.register_ref_of(&tok) {
            Some(p) => p,
            None => {
                self.err(format!("You can't use `{}' after \\advance.", self.cs_display(&tok)), tok.span);
                self.push_tokens(vec![tok]);
                return;
            }
        };
        // optional "by"
        self.skip_spaces();
        self.maybe_consume_keyword("by");
        match op {
            Primitive::Advance => match kind {
                RegisterKind::Count => {
                    let d = self.scan_number();
                    self.st.scopes.set_count(idx, tex_wrapping_add(self.st.scopes.count(idx), d), global);
                }
                RegisterKind::Dimen => {
                    let d = self.scan_dimen();
                    self.st.scopes.set_dimen(idx, tex_wrapping_add(self.st.scopes.dimen(idx), d), global);
                }
                RegisterKind::Skip => {
                    let d = self.scan_glue();
                    let mut g = self.st.scopes.skip(idx);
                    g.value = tex_wrapping_add(g.value, d.value);
                    if d.stretch_fil == g.stretch_fil {
                        g.stretch = tex_wrapping_add(g.stretch, d.stretch);
                    } else if d.stretch_fil > g.stretch_fil {
                        g.stretch = d.stretch;
                        g.stretch_fil = d.stretch_fil;
                    }
                    if d.shrink_fil == g.shrink_fil {
                        g.shrink = tex_wrapping_add(g.shrink, d.shrink);
                    } else if d.shrink_fil > g.shrink_fil {
                        g.shrink = d.shrink;
                        g.shrink_fil = d.shrink_fil;
                    }
                    self.st.scopes.set_skip(idx, g, global);
                }
                RegisterKind::Toks => {}
            },
            Primitive::Multiply => {
                let d = self.scan_number();
                // tex.web §1240: `mult_integers` / `nx_plus_y` against
                // `infinity` / `max_dimen`; on overflow nothing is assigned.
                match kind {
                    RegisterKind::Count => match in_range(self.st.scopes.count(idx).checked_mul(d), TEX_INFINITY) {
                        Some(v) => self.st.scopes.set_count(idx, v, global),
                        None => self.err("Arithmetic overflow.", tok.span),
                    },
                    RegisterKind::Dimen => match in_range(self.st.scopes.dimen(idx).checked_mul(d), TEX_MAX_DIMEN) {
                        Some(v) => self.st.scopes.set_dimen(idx, v, global),
                        None => self.err("Arithmetic overflow.", tok.span),
                    },
                    RegisterKind::Skip => {
                        let mut g = self.st.scopes.skip(idx);
                        let scaled = (
                            in_range(g.value.checked_mul(d), TEX_MAX_DIMEN),
                            in_range(g.stretch.checked_mul(d), TEX_MAX_DIMEN),
                            in_range(g.shrink.checked_mul(d), TEX_MAX_DIMEN),
                        );
                        match scaled {
                            (Some(value), Some(stretch), Some(shrink)) => {
                                g.value = value;
                                g.stretch = stretch;
                                g.shrink = shrink;
                                self.st.scopes.set_skip(idx, g, global);
                            }
                            _ => self.err("Arithmetic overflow.", tok.span),
                        }
                    }
                    RegisterKind::Toks => {}
                }
            }
            Primitive::Divide => {
                let d = self.scan_number();
                if d != 0 {
                    match kind {
                        RegisterKind::Count => self.st.scopes.set_count(idx, self.st.scopes.count(idx) / d, global),
                        RegisterKind::Dimen => self.st.scopes.set_dimen(idx, self.st.scopes.dimen(idx) / d, global),
                        RegisterKind::Skip => {
                            let mut g = self.st.scopes.skip(idx);
                            g.value /= d;
                            g.stretch /= d;
                            g.shrink /= d;
                            self.st.scopes.set_skip(idx, g, global);
                        }
                        RegisterKind::Toks => {}
                    }
                } else {
                    self.err("Arithmetic overflow.", tok.span);
                }
            }
            _ => unreachable!(),
        }
        self.finish_assignment();
    }

    fn maybe_consume_keyword(&mut self, kw: &str) -> bool {
        let mut consumed = Vec::new();
        for expect in kw.chars() {
            match self.next_expanding_raw() {
                Some(p) => {
                    let matches_char = matches!(p.tok.kind, TokenKind::Char(c, _) if c.eq_ignore_ascii_case(&expect));
                    consumed.push(p);
                    if !matches_char {
                        self.push_pending(consumed);
                        return false;
                    }
                }
                None => {
                    self.push_pending(consumed);
                    return false;
                }
            }
        }
        self.skip_spaces();
        true
    }

    // ---- number/dimen/glue scanning -------------------------------------

    /// Scan a `<number>` (TeXbook ch. 24): optional sign, then either an
    /// integer constant (decimal/octal `'`/hex `"`/char `` ` ``) or an
    /// internal quantity (register, `\numexpr`, `\chardef` token, ...),
    /// followed by one optional space which is silently absorbed.
    pub fn scan_number(&mut self) -> i64 {
        self.skip_spaces();
        let mut neg = false;
        loop {
            match self.peek_one_expanding() {
                Some(t) => match &t.kind {
                    TokenKind::Char('+', _) => {
                        self.next_raw_token();
                        self.skip_spaces();
                    }
                    TokenKind::Char('-', _) => {
                        self.next_raw_token();
                        neg = !neg;
                        self.skip_spaces();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        // TeX scans one optional space after a numeric *constant*, but
        // not after an internal quantity (tex.web §440 vs §413).
        let mut constant = true;
        let value = match self.peek_one_expanding() {
            Some(t) => match &t.kind {
                TokenKind::Char(c, CatCode::Other) if c.is_ascii_digit() => self.scan_decimal_digits(),
                TokenKind::Char('\'', _) => {
                    self.next_raw_token();
                    self.scan_radix_digits(8)
                }
                TokenKind::Char('"', _) => {
                    self.next_raw_token();
                    self.scan_radix_digits(16)
                }
                TokenKind::Char('`', _) => {
                    self.next_raw_token();
                    let c = self.next_raw_token();
                    match c.map(|t| t.kind) {
                        Some(TokenKind::Char(ch, _)) => ch as i64,
                        Some(TokenKind::ActiveChar(ch)) => ch as i64,
                        Some(TokenKind::ControlSequence(name)) if name.chars().count() == 1 => {
                            name.chars().next().unwrap() as i64
                        }
                        _ => {
                            self.err("Improper alphabetic constant.", t.span);
                            0
                        }
                    }
                }
                TokenKind::ControlSequence(_) | TokenKind::ActiveChar(_) => match { constant = false; self.meaning_of_token(&t) } {
                    Meaning::RegisterAlias(RegisterKind::Count, idx) => {
                        self.next_raw_token();
                        self.st.scopes.count(idx)
                    }
                    Meaning::RegisterAlias(RegisterKind::Dimen, idx) => {
                        self.next_raw_token();
                        self.st.scopes.dimen(idx)
                    }
                    Meaning::RegisterAlias(RegisterKind::Skip, idx) => {
                        self.next_raw_token();
                        self.st.scopes.skip(idx).value
                    }
                    Meaning::CharDef(n) | Meaning::MathCharDef(n) => {
                        self.next_raw_token();
                        n
                    }
                    Meaning::Primitive(Primitive::Count) => {
                        self.next_raw_token();
                        let idx = self.scan_number() as u16;
                        self.st.scopes.count(idx)
                    }
                    Meaning::Primitive(Primitive::Dimen) => {
                        self.next_raw_token();
                        let idx = self.scan_number() as u16;
                        self.st.scopes.dimen(idx)
                    }
                    Meaning::Primitive(Primitive::Numexpr) => {
                        self.next_raw_token();
                        self.scan_expr(false)
                    }
                    Meaning::Primitive(Primitive::Dimexpr) => {
                        self.next_raw_token();
                        self.scan_expr(true)
                    }
                    Meaning::Primitive(Primitive::Catcode) => {
                        self.next_raw_token();
                        let c = self.scan_number();
                        char::from_u32(c as u32).map(|ch| self.st.scopes.catcode(ch) as u8 as i64).unwrap_or(0)
                    }
                    Meaning::Primitive(Primitive::Uccode) => {
                        self.next_raw_token();
                        let c = self.scan_number();
                        char::from_u32(c as u32).map(|ch| self.st.scopes.uccode(ch) as i64).unwrap_or(0)
                    }
                    Meaning::Primitive(Primitive::Lccode) => {
                        self.next_raw_token();
                        let c = self.scan_number();
                        char::from_u32(c as u32).map(|ch| self.st.scopes.lccode(ch) as i64).unwrap_or(0)
                    }
                    Meaning::Primitive(Primitive::IntPar(p)) => {
                        self.next_raw_token();
                        self.st.scopes.int_param(p)
                    }
                    _ => {
                        // TeX: "Missing number, treated as zero." -- the
                        // offending token is *not* consumed.
                        self.err("Missing number, treated as zero.", t.span);
                        0
                    }
                },
                _ => {
                    self.err("Missing number, treated as zero.", t.span);
                    0
                }
            },
            None => 0,
        };
        if constant {
            self.skip_one_optional_space();
        }
        if neg {
            -value
        } else {
            value
        }
    }

    fn scan_decimal_digits(&mut self) -> i64 {
        let mut s = String::new();
        let mut first = Span::synthetic();
        loop {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char(c, CatCode::Other) if c.is_ascii_digit() => {
                        if s.is_empty() {
                            first = t.span;
                        }
                        s.push(c);
                        self.next_raw_token();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        self.clamped_number(&s, 10, first)
    }

    /// tex.web §445: a constant past `infinity` is "Number too big." and
    /// becomes `infinity`.
    fn clamped_number(&mut self, digits: &str, radix: u32, span: Span) -> i64 {
        let (value, too_big) = parse_clamped(digits, radix);
        if too_big {
            self.err("Number too big.", span);
        }
        value
    }

    fn scan_radix_digits(&mut self, radix: u32) -> i64 {
        let mut s = String::new();
        let mut first = Span::synthetic();
        loop {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char(c, CatCode::Other) if c.is_digit(radix) => {
                        if s.is_empty() {
                            first = t.span;
                        }
                        s.push(c);
                        self.next_raw_token();
                    }
                    TokenKind::Char(c, CatCode::Letter) if radix == 16 && c.is_ascii_hexdigit() && c.is_ascii_uppercase() => {
                        s.push(c);
                        self.next_raw_token();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        self.clamped_number(&s, radix, first)
    }

    fn skip_one_optional_space(&mut self) {
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::Char(_, CatCode::Space)) {
                self.next_raw_token();
            }
        }
    }

    /// Scan a `<dimen>` value in scaled points: `<number>` (possibly with a
    /// decimal point) followed by a unit. Font-relative `em`/`ex` use the
    /// engine's `FontMetrics`.
    /// `<dimen>`, clamped like tex.web §448: a magnitude of 2^30sp or more is
    /// "Dimension too large." and becomes `max_dimen`.
    pub fn scan_dimen(&mut self) -> i64 {
        let v = self.scan_dimen_unclamped();
        if v.abs() > TEX_MAX_DIMEN {
            self.err("Dimension too large.", Span::synthetic());
            return TEX_MAX_DIMEN * v.signum();
        }
        v
    }

    fn scan_dimen_unclamped(&mut self) -> i64 {
        self.skip_spaces();
        let mut neg = false;
        loop {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char('+', _) => {
                        self.next_raw_token();
                        self.skip_spaces();
                    }
                    TokenKind::Char('-', _) => {
                        self.next_raw_token();
                        neg = !neg;
                        self.skip_spaces();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        // internal dimen register shortcut
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::ControlSequence(_) | TokenKind::ActiveChar(_)) {
                match self.meaning_of_token(&t) {
                    Meaning::RegisterAlias(RegisterKind::Dimen, idx) => {
                        self.next_raw_token();
                        let v = self.st.scopes.dimen(idx);
                        return if neg { -v } else { v };
                    }
                    Meaning::RegisterAlias(RegisterKind::Skip, idx) => {
                        self.next_raw_token();
                        let v = self.st.scopes.skip(idx).value;
                        return if neg { -v } else { v };
                    }
                    Meaning::Primitive(Primitive::Dimen) => {
                        self.next_raw_token();
                        let idx = self.scan_number() as u16;
                        let v = self.st.scopes.dimen(idx);
                        return if neg { -v } else { v };
                    }
                    Meaning::Primitive(Primitive::Dimexpr) => {
                        self.next_raw_token();
                        let v = self.scan_expr(true);
                        return if neg { -v } else { v };
                    }
                    Meaning::RegisterAlias(RegisterKind::Count, _)
                    | Meaning::CharDef(_)
                    | Meaning::MathCharDef(_)
                    | Meaning::Primitive(Primitive::Count | Primitive::Numexpr) => {
                        // <internal integer><unit>: e.g. `\count0 pt`.
                        let n = self.scan_number();
                        let unit = self.read_unit_name();
                        let per = self.unit_sp(&unit);
                        let v = scale_decimal(n, "", per);
                        self.skip_one_optional_space();
                        return if neg { -v } else { v };
                    }
                    _ => {}
                }
            }
        }
        let mut int_part = String::new();
        loop {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char(c, CatCode::Other) if c.is_ascii_digit() => {
                        int_part.push(c);
                        self.next_raw_token();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        let mut frac = String::new();
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::Char('.', _) | TokenKind::Char(',', _)) {
                self.next_raw_token();
                loop {
                    match self.peek_one_expanding() {
                        Some(t) => match t.kind {
                            TokenKind::Char(c, CatCode::Other) if c.is_ascii_digit() => {
                                frac.push(c);
                                self.next_raw_token();
                            }
                            _ => break,
                        },
                        None => break,
                    }
                }
            }
        }
        if int_part.is_empty() && frac.is_empty() {
            let span = self.peek_one_expanding().map(|t| t.span).unwrap_or(Span::synthetic());
            self.err("Missing number, treated as zero.", span);
        }
        self.skip_spaces();
        // `<factor><internal dimen>` (`.5\textwidth`, `2\dimen0`): TeX
        // multiplies with nx_plus_y(v, f, xn_over_d(v, f, 2^16)).
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::ControlSequence(_) | TokenKind::ActiveChar(_)) {
                let v = match self.meaning_of_token(&t) {
                    Meaning::RegisterAlias(RegisterKind::Dimen, idx) => {
                        self.next_raw_token();
                        Some(self.st.scopes.dimen(idx))
                    }
                    Meaning::RegisterAlias(RegisterKind::Skip, idx) => {
                        self.next_raw_token();
                        Some(self.st.scopes.skip(idx).value)
                    }
                    Meaning::Primitive(Primitive::Dimen) => {
                        self.next_raw_token();
                        let idx = self.scan_number() as u16;
                        Some(self.st.scopes.dimen(idx))
                    }
                    Meaning::Primitive(Primitive::Skip) => {
                        self.next_raw_token();
                        let idx = self.scan_number() as u16;
                        Some(self.st.scopes.skip(idx).value)
                    }
                    Meaning::Primitive(Primitive::Dimexpr) => {
                        self.next_raw_token();
                        Some(self.scan_expr(true))
                    }
                    _ => None,
                };
                if let Some(v) = v {
                    let n = self.clamped_number(&int_part, 10, t.span);
                    let f = round_decimals(&frac);
                    let r = n * v + xn_over_d(v, f, 65536);
                    return if neg { -r } else { r };
                }
            }
        }
        let unit = self.read_unit_name();
        let int_val = self.clamped_number(&int_part, 10, Span::synthetic());
        let per = self.unit_sp(&unit);
        let sp = scale_decimal(int_val, &frac, per);
        self.skip_one_optional_space();
        if neg {
            -sp
        } else {
            sp
        }
    }

    fn unit_sp(&mut self, unit: &str) -> f64 {
        match unit {
            "em" => self.metrics.quad_sp() as f64,
            "ex" => self.metrics.x_height_sp() as f64,
            other => match absolute_unit_sp_per_unit(other) {
                Some(v) => v,
                None => {
                    self.err("Illegal unit of measure (pt inserted).", Span::synthetic());
                    65536.0
                }
            },
        }
    }

    fn read_unit_name(&mut self) -> String {
        // `true` prefix (e.g. `truept`) is accepted and ignored (no
        // magnification here).
        self.maybe_consume_keyword("true");
        let mut s = String::new();
        for _ in 0..2 {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char(c, CatCode::Letter) | TokenKind::Char(c, CatCode::Other) if c.is_ascii_alphabetic() => {
                        s.push(c);
                        self.next_raw_token();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        s.to_ascii_lowercase()
    }

    pub fn scan_glue(&mut self) -> Glue {
        self.skip_spaces();
        // `\skip0=\skip1` copies the whole glue.
        if let Some(t) = self.peek_one_expanding() {
            if let TokenKind::ControlSequence(_) = t.kind {
                if let Meaning::RegisterAlias(RegisterKind::Skip, idx) = self.meaning_of_token(&t) {
                    self.next_raw_token();
                    return self.st.scopes.skip(idx);
                }
            }
        }
        let value = self.scan_dimen();
        let mut g = Glue::fixed(value);
        self.skip_spaces();
        if self.maybe_consume_keyword("plus") {
            let (v, fil) = self.scan_stretch_shrink();
            g.stretch = v;
            g.stretch_fil = fil;
        }
        self.skip_spaces();
        if self.maybe_consume_keyword("minus") {
            let (v, fil) = self.scan_stretch_shrink();
            g.shrink = v;
            g.shrink_fil = fil;
        }
        g
    }

    fn scan_stretch_shrink(&mut self) -> (i64, u8) {
        // Reuse scan_dimen's digit/frac scanning but allow "fil"+"l"*.
        let mut neg = false;
        loop {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char('+', _) => {
                        self.next_raw_token();
                    }
                    TokenKind::Char('-', _) => {
                        self.next_raw_token();
                        neg = !neg;
                    }
                    _ => break,
                },
                None => break,
            }
        }
        let mut int_part = String::new();
        loop {
            match self.peek_one_expanding() {
                Some(t) => match t.kind {
                    TokenKind::Char(c, CatCode::Other) if c.is_ascii_digit() => {
                        int_part.push(c);
                        self.next_raw_token();
                    }
                    _ => break,
                },
                None => break,
            }
        }
        let mut frac = String::new();
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::Char('.', _)) {
                self.next_raw_token();
                loop {
                    match self.peek_one_expanding() {
                        Some(t) => match t.kind {
                            TokenKind::Char(c, CatCode::Other) if c.is_ascii_digit() => {
                                frac.push(c);
                                self.next_raw_token();
                            }
                            _ => break,
                        },
                        None => break,
                    }
                }
            }
        }
        self.skip_spaces();
        if self.maybe_consume_keyword("fil") {
            let mut fil = 1u8;
            while self.maybe_consume_keyword("l") {
                // tex.web §454: an order past filll is an error, not a new order.
                if fil == 3 {
                    self.err("Illegal unit of measure (replaced by filll).", Span::synthetic());
                } else {
                    fil += 1;
                }
            }
            let int_val = self.clamped_number(&int_part, 10, Span::synthetic());
            let v = scale_decimal(int_val, &frac, 65536.0);
            self.skip_one_optional_space();
            return (if neg { -v } else { v }, fil);
        }
        let unit = self.read_unit_name();
        let per = self.unit_sp(&unit);
        let int_val = self.clamped_number(&int_part, 10, Span::synthetic());
        let v = scale_decimal(int_val, &frac, per);
        self.skip_one_optional_space();
        (if neg { -v } else { v }, 0)
    }

    /// e-TeX `\numexpr`/`\dimexpr`: `+ - * /` with standard precedence,
    /// parentheses, and a terminating `\relax` that is consumed if
    /// present. Division rounds to nearest, ties away from zero.
    fn scan_expr(&mut self, is_dimen: bool) -> i64 {
        let v = self.expr_sum(is_dimen);
        self.skip_spaces();
        if let Some(t) = self.peek_one_expanding() {
            if t.is_cs("relax") {
                self.next_raw_token();
            }
        }
        // e-TeX: an intermediate or final value past `infinity` (`max_dimen`
        // for `\dimexpr`) is "Arithmetic overflow" and the expression is 0.
        // The steps saturate, so any overflow surfaces as a final value
        // out of range.
        let limit = if is_dimen { TEX_MAX_DIMEN } else { TEX_INFINITY };
        if v.abs() > limit {
            self.err("Arithmetic overflow.", Span::synthetic());
            return 0;
        }
        v
    }

    fn expr_sum(&mut self, is_dimen: bool) -> i64 {
        let mut acc = self.expr_prod(is_dimen);
        loop {
            self.skip_spaces();
            match self.peek_one_expanding() {
                Some(t) if matches!(t.kind, TokenKind::Char('+', _)) => {
                    self.next_raw_token();
                    acc = acc.saturating_add(self.expr_prod(is_dimen));
                }
                Some(t) if matches!(t.kind, TokenKind::Char('-', _)) => {
                    self.next_raw_token();
                    acc = acc.saturating_sub(self.expr_prod(is_dimen));
                }
                _ => break,
            }
        }
        acc
    }

    fn expr_prod(&mut self, is_dimen: bool) -> i64 {
        let mut acc = self.expr_atom(is_dimen);
        loop {
            self.skip_spaces();
            match self.peek_one_expanding() {
                Some(t) if matches!(t.kind, TokenKind::Char('*', _)) => {
                    self.next_raw_token();
                    // e-TeX: the factor of a `*` is always an integer.
                    let f = self.expr_atom(false);
                    // `a*b/c` is computed with an intermediate 64-bit
                    // product then a rounded division, matching e-TeX's
                    // `fract`.
                    self.skip_spaces();
                    if let Some(t2) = self.peek_one_expanding() {
                        if matches!(t2.kind, TokenKind::Char('/', _)) {
                            self.next_raw_token();
                            let d = self.expr_atom(false);
                            acc = rounded_div(acc.saturating_mul(f), d);
                            continue;
                        }
                    }
                    acc = acc.saturating_mul(f);
                }
                Some(t) if matches!(t.kind, TokenKind::Char('/', _)) => {
                    self.next_raw_token();
                    let d = self.expr_atom(false);
                    acc = rounded_div(acc, d);
                }
                _ => break,
            }
        }
        acc
    }

    fn expr_atom(&mut self, is_dimen: bool) -> i64 {
        self.skip_spaces();
        if let Some(t) = self.peek_one_expanding() {
            if matches!(t.kind, TokenKind::Char('(', _)) {
                self.next_raw_token();
                let v = self.expr_sum(is_dimen);
                self.skip_spaces();
                if let Some(t2) = self.peek_one_expanding() {
                    if matches!(t2.kind, TokenKind::Char(')', _)) {
                        self.next_raw_token();
                    }
                }
                return v;
            }
        }
        if is_dimen {
            self.scan_dimen()
        } else {
            self.scan_number()
        }
    }

    // ---- conditionals -----------------------------------------------------

    fn do_conditional(&mut self, prim: Primitive, unless: bool, if_tok: &Token) {
        use Primitive::*;
        if self.st.conditionals.depth() as u32 > self.limits.max_conditional_depth {
            if !self.st.conditional_limit_reported {
                self.st.conditional_limit_reported = true;
                self.err("conditional nesting limit exceeded", if_tok.span);
            }
            return;
        }
        self.st.conditional_limit_reported = false;
        let if_name = format!("{}{}", self.esc(), primitive_name(prim));
        let if_at = if_tok.span;
        let shape = if matches!(prim, Ifcase) { IfShape::Case } else { IfShape::TwoWay };
        self.st.conditionals.push(shape, IfBranch::Testing, primitive_name(prim));
        if matches!(prim, Ifcase) {
            let n = self.scan_number();
            self.set_top_branch(IfBranch::Taken);
            let mut remaining = n;
            loop {
                if remaining == 0 {
                    break;
                }
                match self.skip_to_or_else_fi(&if_name, if_at) {
                    BranchEnd::Or => {
                        remaining -= 1;
                    }
                    BranchEnd::Else => {
                        return;
                    }
                    BranchEnd::Fi => {
                        self.st.conditionals.pop();
                        return;
                    }
                }
            }
            return;
        }
        let truth = match prim {
            Iftrue => true,
            Iffalse => false,
            If => {
                let a = self.scan_if_char_token();
                let b = self.scan_if_char_token();
                a == b
            }
            Ifcat => {
                let a = self.scan_if_cat_token();
                let b = self.scan_if_cat_token();
                a == b
            }
            Ifx => self.scan_ifx(),
            Ifnum => {
                let a = self.scan_number();
                let rel = self.scan_relation();
                let b = self.scan_number();
                apply_relation(a, b, rel)
            }
            Ifdim => {
                let a = self.scan_dimen();
                let rel = self.scan_relation();
                let b = self.scan_dimen();
                apply_relation(a, b, rel)
            }
            Ifodd => self.scan_number() % 2 != 0,
            Ifvmode => self.st.mode.is_v(),
            Ifhmode => self.st.mode.is_h(),
            Ifmmode => self.st.mode.is_m(),
            Ifinner => self.st.mode.is_inner(),
            Ifdefined => match self.next_raw_token() {
                Some(t) => !matches!(self.meaning_of_token(&t), Meaning::Undefined),
                None => false,
            },
            Ifcsname => {
                let name = self.scan_csname_text(if_tok);
                self.st.scopes.is_defined(&name)
            }
            Ifhbox | Ifvbox => {
                let _ = self.scan_number();
                false
            }
            Ifvoid | Ifeof => {
                let _ = self.scan_number();
                true
            }
            Ifincsname => self.st.in_csname > 0,
            _ => unreachable!(),
        };
        let truth = if unless { !truth } else { truth };
        if truth {
            self.set_top_branch(IfBranch::Taken);
        } else {
            self.set_top_branch(IfBranch::Skipping);
            match self.skip_to_or_else_fi(&if_name, if_at) {
                BranchEnd::Else => {
                    if let Some(f) = self.st.conditionals.top_mut() {
                        f.branch = IfBranch::Taken;
                    }
                }
                BranchEnd::Fi => {
                    self.st.conditionals.pop();
                }
                BranchEnd::Or => {
                    // `\or` outside `\ifcase`: TeX reports "Extra \or".
                    self.err("Extra \\or.", if_tok.span);
                    if let Some(f) = self.st.conditionals.top_mut() {
                        f.branch = IfBranch::Taken;
                    }
                }
            }
        }
    }

    fn set_top_branch(&mut self, branch: IfBranch) {
        if let Some(f) = self.st.conditionals.top_mut() {
            f.branch = branch;
        }
    }

    /// `\if`: compare the character codes of the next two tokens after
    /// full expansion, treating an (unexpandable) control sequence as if
    /// it had char code 256 (never equal to any real character but equal
    /// to another such control sequence), per TeXbook p. 209.
    fn scan_if_char_token(&mut self) -> Option<char> {
        let p = self.next_expanding_raw()?;
        match p.tok.kind {
            TokenKind::Char(c, _) => Some(c),
            TokenKind::ActiveChar(c) => Some(c),
            TokenKind::ControlSequence(_) => match self.meaning_of_token(&p.tok) {
                Meaning::CharLike(t) => match t.kind {
                    TokenKind::Char(c, _) => Some(c),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn scan_if_cat_token(&mut self) -> Option<CatCode> {
        let p = self.next_expanding_raw()?;
        match p.tok.kind {
            TokenKind::Char(_, cat) => Some(cat),
            TokenKind::ActiveChar(_) => Some(CatCode::Active),
            TokenKind::ControlSequence(_) => match self.meaning_of_token(&p.tok) {
                Meaning::CharLike(t) => match t.kind {
                    TokenKind::Char(_, cat) => Some(cat),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    /// `\ifx`: compare the *meanings* of the next two tokens without
    /// expanding either (TeXbook p. 210).
    fn scan_ifx(&mut self) -> bool {
        let t1 = self.next_raw_token();
        let t2 = self.next_raw_token();
        let (t1, t2) = match (t1, t2) {
            (Some(a), Some(b)) => (a, b),
            _ => return false,
        };
        let m1 = self.meaning_of_token(&t1);
        let m2 = self.meaning_of_token(&t2);
        meanings_equal(&m1, &m2)
    }

    fn scan_relation(&mut self) -> Relation {
        self.skip_spaces();
        match self.peek_one_expanding().map(|t| t.kind) {
            Some(TokenKind::Char('<', _)) => {
                self.next_raw_token();
                Relation::Lt
            }
            Some(TokenKind::Char('=', _)) => {
                self.next_raw_token();
                Relation::Eq
            }
            Some(TokenKind::Char('>', _)) => {
                self.next_raw_token();
                Relation::Gt
            }
            _ => {
                self.err("Missing = inserted for \\ifnum.", Span::synthetic());
                Relation::Eq
            }
        }
    }

    /// Skip tokens (respecting nested `\if...\fi`) until an
    /// `\else`/`\or`/`\fi` belonging to *this* conditional level.
    fn skip_to_or_else_fi(&mut self, if_name: &str, at: Span) -> BranchEnd {
        let saved = std::mem::replace(
            &mut self.st.scanner_status,
            ScannerStatus::Skipping { if_name: if_name.to_string(), at },
        );
        let mut depth = 0i32;
        let result = loop {
            let tok = match self.next_raw_token() {
                Some(t) => t,
                None => break BranchEnd::Fi,
            };
            let m = match &tok.kind {
                TokenKind::ControlSequence(name) => self.st.scopes.meaning_ref(name).cloned(),
                TokenKind::ActiveChar(c) => Some(self.st.scopes.active_meaning(*c)),
                _ => None,
            };
            match m.map(strip_let) {
                Some(Meaning::Primitive(p)) if is_if_primitive(p) => depth += 1,
                Some(Meaning::Primitive(Primitive::Fi)) => {
                    if depth == 0 {
                        break BranchEnd::Fi;
                    }
                    depth -= 1;
                }
                Some(Meaning::Primitive(Primitive::Else)) if depth == 0 => break BranchEnd::Else,
                Some(Meaning::Primitive(Primitive::Or)) if depth == 0 => break BranchEnd::Or,
                _ => {}
            }
        };
        if matches!(self.st.scanner_status, ScannerStatus::Skipping { .. }) {
            self.st.scanner_status = saved;
        }
        result
    }

    fn handle_stray_or_else_fi(&mut self, p: Primitive, tok: Token) {
        // Reaching `\else`/`\or`/`\fi` directly (not via skip_to_or_else_fi)
        // means we were in the *taken* branch and must now skip to `\fi`.
        match self.st.conditionals.pop() {
            Some(frame) if frame.branch == IfBranch::Testing => {
                // tex.web §510: the condition is still being scanned (e.g.
                // `\ifnum\count0=1\fi`); insert `\relax` before the token.
                self.st.conditionals.push(frame.shape, frame.branch, frame.name);
                let relax = Token::new(TokenKind::ControlSequence("relax".into()), tok.span);
                self.push_pending(vec![Pending { tok: relax, frozen: true, origin: None }, Pending { tok, frozen: false, origin: None }]);
            }
            Some(frame) => match p {
                Primitive::Fi => {} // branch simply ends here
                Primitive::Else | Primitive::Or => {
                    if matches!(p, Primitive::Or) && frame.shape != IfShape::Case {
                        self.err("Extra \\or.", tok.span);
                        self.st.conditionals.push(frame.shape, frame.branch, frame.name);
                        return;
                    }
                    if matches!(frame.branch, IfBranch::Taken) {
                        let name = format!("{}{}", self.esc(), frame.name);
                        self.skip_balanced_to_fi(&name, tok.span);
                    }
                }
                _ => unreachable!("handle_stray_or_else_fi is only called with Fi/Else/Or"),
            },
            None => {
                self.err(format!("Extra {}.", self.cs_display(&tok)), tok.span);
            }
        }
    }

    fn skip_balanced_to_fi(&mut self, if_name: &str, at: Span) {
        let saved = std::mem::replace(
            &mut self.st.scanner_status,
            ScannerStatus::Skipping { if_name: if_name.to_string(), at },
        );
        let mut depth = 0i32;
        loop {
            let tok = match self.next_raw_token() {
                Some(t) => t,
                None => break,
            };
            let m = match &tok.kind {
                TokenKind::ControlSequence(name) => self.st.scopes.meaning_ref(name).cloned(),
                TokenKind::ActiveChar(c) => Some(self.st.scopes.active_meaning(*c)),
                _ => None,
            };
            match m.map(strip_let) {
                Some(Meaning::Primitive(p)) if is_if_primitive(p) => depth += 1,
                Some(Meaning::Primitive(Primitive::Fi)) => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        if matches!(self.st.scanner_status, ScannerStatus::Skipping { .. }) {
            self.st.scanner_status = saved;
        }
    }

    // ---- \newif -------------------------------------------------------

    fn do_newif(&mut self) {
        let (global, _) = self.take_prefixes();
        let name_tok = match self.next_raw_token() {
            Some(t) => t,
            None => return,
        };
        let base = match &name_tok.kind {
            TokenKind::ControlSequence(n) => n.strip_prefix("if").unwrap_or(n).to_string(),
            _ => {
                self.err("\\newif requires a control sequence starting with \\if", name_tok.span);
                return;
            }
        };
        let if_name = format!("if{base}");
        let true_name = format!("{base}true");
        let false_name = format!("{base}false");
        // \iffoo := \iffalse initially; \footrue/\foofalse re-\let it
        // globally, exactly like plain.tex's \newif.
        self.st.scopes.assign_cs(&if_name, Meaning::Primitive(Primitive::Iffalse), global);
        let mk = |target: &str| {
            vec![
                Token::synthetic(TokenKind::ControlSequence("global".into())),
                Token::synthetic(TokenKind::ControlSequence("let".into())),
                Token::synthetic(TokenKind::ControlSequence(if_name.clone())),
                Token::synthetic(TokenKind::ControlSequence(target.into())),
            ]
        };
        self.st.scopes.assign_cs(&true_name, Meaning::Macro(Rc::new(MacroDef::simple(mk("iftrue")))), global);
        self.st.scopes.assign_cs(&false_name, Meaning::Macro(Rc::new(MacroDef::simple(mk("iffalse")))), global);
    }

    // ---- \string / \meaning helpers ------------------------------------

    fn string_of(&self, tok: &Token) -> String {
        match &tok.kind {
            TokenKind::ControlSequence(name) => format!("{}{name}", self.esc()),
            TokenKind::ActiveChar(c) => c.to_string(),
            TokenKind::Char(c, _) => c.to_string(),
            TokenKind::Param(n) => format!("#{n}"),
            _ => String::new(),
        }
    }

    /// `\meaning` text exactly as TeX's `print_meaning` produces it.
    pub fn meaning_string(&self, tok: &Token) -> String {
        match &tok.kind {
            TokenKind::Char(c, cat) => char_meaning(*c, *cat),
            TokenKind::Param(_) => "macro parameter character #".into(),
            _ => {
                let m = self.meaning_of_token(tok);
                self.meaning_to_string(&m)
            }
        }
    }

    fn meaning_to_string(&self, m: &Meaning) -> String {
        let esc = self.esc();
        match m {
            Meaning::Undefined => "undefined".to_string(),
            Meaning::Primitive(p) => format!("{esc}{}", primitive_name(*p)),
            Meaning::CharLike(t) => match t.kind {
                TokenKind::Char(c, cat) => char_meaning(c, cat),
                _ => "undefined".to_string(),
            },
            Meaning::RegisterAlias(k, idx) => {
                let kw = match k {
                    RegisterKind::Count => "count",
                    RegisterKind::Dimen => "dimen",
                    RegisterKind::Skip => "skip",
                    RegisterKind::Toks => "toks",
                };
                match (*idx as usize).checked_sub(TEX_PARAM_BASE as usize).and_then(|i| TEX_PARAMS.get(i)) {
                    Some((name, _)) => format!("{esc}{name}"),
                    None => format!("{esc}{kw}{idx}"),
                }
            }
            Meaning::CharDef(n) => format!("{esc}char\"{:X}", n),
            Meaning::MathCharDef(n) => format!("{esc}mathchar\"{:X}", n),
            Meaning::Macro(def) => self.macro_meaning(def),
            Meaning::Let(inner) => self.meaning_to_string(inner),
        }
    }

    fn macro_meaning(&self, def: &MacroDef) -> String {
        let esc = self.esc();
        let mut s = String::new();
        if def.flags.protected {
            s.push_str(&format!("{esc}protected"));
        }
        if def.flags.long {
            s.push_str(&format!("{esc}long"));
        }
        if def.flags.outer {
            s.push_str(&format!("{esc}outer"));
        }
        if def.flags.protected || def.flags.long || def.flags.outer {
            s.push(' ');
        }
        s.push_str("macro:");
        for p in &def.params {
            match p {
                ParamPart::Literal(t) => s.push_str(&self.token_meaning_text(t)),
                ParamPart::Param(n) => s.push_str(&format!("#{n}")),
            }
        }
        if def.flags.brace_delimited_last {
            s.push('{');
        }
        s.push_str("->");
        for p in &def.body {
            match p {
                BodyPart::Literal(t) => s.push_str(&self.token_meaning_text(t)),
                BodyPart::Param(n) => s.push_str(&format!("#{n}")),
            }
        }
        if def.flags.brace_delimited_last {
            s.push('{');
        }
        s
    }

    /// One token as `show_token_list` prints it inside a macro body: a
    /// catcode-6 `#` is doubled, control words get a trailing space.
    fn token_meaning_text(&self, t: &Token) -> String {
        match &t.kind {
            TokenKind::Char(c, CatCode::Param) => format!("{c}{c}"),
            TokenKind::Char(c, _) => c.to_string(),
            TokenKind::ControlSequence(name) => self.print_cs(name),
            TokenKind::ActiveChar(c) => c.to_string(),
            TokenKind::Param(n) => format!("#{n}"),
            TokenKind::Eof => String::new(),
        }
    }
}

// ---- free helpers ------------------------------------------------------

/// A grouping token as emitted into the output stream: a catcode-1/2
/// character or `\begingroup`/`\endgroup`.
pub fn is_group_token(t: &Token) -> bool {
    match &t.kind {
        TokenKind::Char(_, CatCode::BeginGroup | CatCode::EndGroup) => true,
        TokenKind::ControlSequence(n) => n == "begingroup" || n == "endgroup",
        _ => false,
    }
}

fn meaning_is_outer(m: &Meaning) -> bool {
    match m {
        Meaning::Macro(d) => d.flags.outer,
        Meaning::Let(inner) => meaning_is_outer(inner),
        _ => false,
    }
}

fn meaning_is_expandable(m: &Meaning, expand_only: bool) -> bool {
    use Primitive::*;
    match m {
        Meaning::Macro(d) => !(expand_only && d.flags.protected),
        Meaning::Let(inner) => meaning_is_expandable(inner, expand_only),
        Meaning::Primitive(p) => matches!(
            p,
            Expandafter
                | Noexpand
                | Csname
                | String
                | Number
                | Romannumeral
                | MeaningOf
                | The
                | Unexpanded
                | Detokenize
                | Expanded
                | ETeXRevision
                | InputFile
                | Jobname
                | Pdfstrcmp
                | Scantokens
                | If
                | Ifcat
                | Ifx
                | Ifnum
                | Ifdim
                | Ifodd
                | Ifvmode
                | Ifhmode
                | Ifmmode
                | Ifinner
                | Ifcase
                | Iftrue
                | Iffalse
                | Ifdefined
                | Ifcsname
                | Ifhbox
                | Ifvbox
                | Ifvoid
                | Ifeof
                | Ifincsname
                | Or
                | Else
                | Fi
                | Unless
                | Value
                | Arabic
                | RomanLower
                | RomanUpper
                | AlphLower
                | AlphUpper
                | Fnsymbol
        ),
        _ => false,
    }
}

fn strip_let(m: Meaning) -> Meaning {
    match m {
        Meaning::Let(inner) => strip_let(*inner),
        other => other,
    }
}

/// Does the first `{` of `toks` match the last `}` (so the whole list is
/// one brace group)?
fn encloses_whole(toks: &[Token]) -> bool {
    if !matches!(toks.first().map(|t| &t.kind), Some(TokenKind::Char(_, CatCode::BeginGroup)))
        || !matches!(toks.last().map(|t| &t.kind), Some(TokenKind::Char(_, CatCode::EndGroup)))
    {
        return false;
    }
    let mut depth = 0i32;
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            TokenKind::Char(_, CatCode::BeginGroup) => depth += 1,
            TokenKind::Char(_, CatCode::EndGroup) => {
                depth -= 1;
                if depth == 0 && i != toks.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn split_top_level(toks: &[Token], sep: char) -> Vec<Vec<Token>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut depth = 0i32;
    for t in toks {
        match t.kind {
            TokenKind::Char(_, CatCode::BeginGroup) => depth += 1,
            TokenKind::Char(_, CatCode::EndGroup) => depth -= 1,
            TokenKind::Char(c, CatCode::Other) if c == sep && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(t.clone());
    }
    out.push(cur);
    out
}

fn trim_spaces(mut toks: Vec<Token>) -> Vec<Token> {
    while matches!(toks.first().map(|t| &t.kind), Some(TokenKind::Char(_, CatCode::Space))) {
        toks.remove(0);
    }
    while matches!(toks.last().map(|t| &t.kind), Some(TokenKind::Char(_, CatCode::Space))) {
        toks.pop();
    }
    toks
}

fn arabic_call_tokens(name: &str) -> Vec<Token> {
    let mut v = vec![
        Token::synthetic(TokenKind::ControlSequence("arabic".into())),
        Token::synthetic(TokenKind::Char('{', CatCode::BeginGroup)),
    ];
    v.extend(chars_as_other(name, Span::synthetic()));
    v.push(Token::synthetic(TokenKind::Char('}', CatCode::EndGroup)));
    v
}

fn char_meaning(c: char, cat: CatCode) -> String {
    match cat {
        CatCode::Letter => format!("the letter {c}"),
        CatCode::Other => format!("the character {c}"),
        CatCode::BeginGroup => format!("begin-group character {c}"),
        CatCode::EndGroup => format!("end-group character {c}"),
        CatCode::MathShift => format!("math shift character {c}"),
        CatCode::AlignTab => format!("alignment tab character {c}"),
        CatCode::Param => format!("macro parameter character {c}"),
        CatCode::Superscript => format!("superscript character {c}"),
        CatCode::Subscript => format!("subscript character {c}"),
        CatCode::Space => format!("blank space {c}"),
        _ => format!("the character {c}"),
    }
}

/// TeX's `print_cmd_chr` name for each primitive we model.
fn primitive_name(p: Primitive) -> &'static str {
    use Primitive::*;
    match p {
        Relax => "relax",
        Par => "par",
        Def => "def",
        Edef => "edef",
        Gdef => "gdef",
        Xdef => "xdef",
        Let => "let",
        Futurelet => "futurelet",
        Global => "global",
        Long => "long",
        Outer => "outer",
        Protected => "protected",
        Expandafter => "expandafter",
        Noexpand => "noexpand",
        Csname => "csname",
        Endcsname => "endcsname",
        String => "string",
        Number => "number",
        Romannumeral => "romannumeral",
        MeaningOf => "meaning",
        The => "the",
        Unexpanded => "unexpanded",
        Detokenize => "detokenize",
        Expanded => "expanded",
        ETeXRevision => "eTeXrevision",
        InputFile => "input",
        Immediate => "immediate",
        Write => "write",
        Openout => "openout",
        Closeout => "closeout",
        Openin => "openin",
        Closein => "closein",
        Read => "read",
        Message => "message",
        Errmessage => "errmessage",
        Jobname => "jobname",
        IntPar(IntParam::ETeXVersion) => "eTeXversion",
        Pdfstrcmp => "pdfstrcmp",
        Scantokens => "scantokens",
        Afterassignment => "afterassignment",
        Uppercase => "uppercase",
        Lowercase => "lowercase",
        Uccode => "uccode",
        Lccode => "lccode",
        Chardef => "chardef",
        Mathchardef => "mathchardef",
        IntPar(IntParam::Escapechar) => "escapechar",
        IntPar(IntParam::Endlinechar) => "endlinechar",
        IntPar(IntParam::Newlinechar) => "newlinechar",
        Begingroup => "begingroup",
        Endgroup => "endgroup",
        Aftergroup => "aftergroup",
        Catcode => "catcode",
        Ignorespaces => "ignorespaces",
        Endinput => "endinput",
        If => "if",
        Ifcat => "ifcat",
        Ifx => "ifx",
        Ifnum => "ifnum",
        Ifdim => "ifdim",
        Ifodd => "ifodd",
        Ifvmode => "ifvmode",
        Ifhmode => "ifhmode",
        Ifmmode => "ifmmode",
        Ifinner => "ifinner",
        Ifcase => "ifcase",
        Iftrue => "iftrue",
        Iffalse => "iffalse",
        Ifdefined => "ifdefined",
        Ifcsname => "ifcsname",
        Ifhbox => "ifhbox",
        Ifvbox => "ifvbox",
        Ifvoid => "ifvoid",
        Ifeof => "ifeof",
        Ifincsname => "ifincsname",
        Or => "or",
        Else => "else",
        Fi => "fi",
        Newif => "newif",
        Unless => "unless",
        Count => "count",
        Dimen => "dimen",
        Skip => "skip",
        Toks => "toks",
        Countdef => "countdef",
        Dimendef => "dimendef",
        Skipdef => "skipdef",
        Toksdef => "toksdef",
        Newcount => "newcount",
        Newdimen => "newdimen",
        Newskip => "newskip",
        Newtoks => "newtoks",
        Advance => "advance",
        Multiply => "multiply",
        Divide => "divide",
        Numexpr => "numexpr",
        Dimexpr => "dimexpr",
        NewCommand => "newcommand",
        RenewCommand => "renewcommand",
        ProvideCommand => "providecommand",
        DeclareRobustCommand => "DeclareRobustCommand",
        NewEnvironment => "newenvironment",
        RenewEnvironment => "renewenvironment",
        Begin => "begin",
        End => "end",
        NewCounter => "newcounter",
        SetCounter => "setcounter",
        AddToCounter => "addtocounter",
        StepCounter => "stepcounter",
        RefStepCounter => "refstepcounter",
        AddToReset => "@addtoreset",
        RemoveFromReset => "@removefromreset",
        CounterWithin => "counterwithin",
        CounterWithout => "counterwithout",
        Label => "label",
        Value => "value",
        Arabic => "arabic",
        RomanLower => "roman",
        RomanUpper => "Roman",
        AlphLower => "alph",
        AlphUpper => "Alph",
        Fnsymbol => "fnsymbol",
        NewLength => "newlength",
        SetToWidth => "settowidth",
        SetToHeight => "settoheight",
        SetToDepth => "settodepth",
        DefineKey => "define@key",
        SetKeys => "setkeys",
        Verb => "verb",
        StopInput => "flashtex@stop",
        Host => "flashtex@host",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relation {
    Lt,
    Eq,
    Gt,
}

fn apply_relation(a: i64, b: i64, rel: Relation) -> bool {
    match rel {
        Relation::Lt => a < b,
        Relation::Eq => a == b,
        Relation::Gt => a > b,
    }
}

enum BranchEnd {
    Or,
    Else,
    Fi,
}

/// tex.web §102 `round_decimals`: the digits after a decimal point as a
/// fraction of 2^16, rounded.
fn round_decimals(digits: &str) -> i64 {
    let mut a: i64 = 0;
    for d in digits.bytes().take(17).rev() {
        a = (a + (d - b'0') as i64 * 131072) / 10;
    }
    (a + 1) / 2
}

/// tex.web §107 `xn_over_d`: x*n/d truncated toward zero.
fn xn_over_d(x: i64, n: i64, d: i64) -> i64 {
    let r = (x.abs() as i128 * n as i128) / d as i128;
    if x < 0 {
        -(r as i64)
    } else {
        r as i64
    }
}

/// `value` when it exists and its magnitude is at most `limit`.
fn in_range(value: Option<i64>, limit: i64) -> Option<i64> {
    value.filter(|v| v.abs() <= limit)
}

/// tex.web §1238-1239: `\advance` adds with the host's 32-bit integer
/// arithmetic and no range check, so it wraps silently (pdfTeX:
/// `\count0=2147483647 \advance\count0 by 1` gives -2147483648; dimens and
/// glue components wrap the same way in scaled points).
fn tex_wrapping_add(a: i64, b: i64) -> i64 {
    i64::from((a as i32).wrapping_add(b as i32))
}

/// e-TeX's `\numexpr`/`\dimexpr` division rounds to the nearest integer,
/// ties away from zero (not truncating like `\divide`).

fn rounded_div(a: i64, d: i64) -> i64 {
    if d == 0 {
        return 0;
    }
    let sign: i128 = if (a < 0) != (d < 0) { -1 } else { 1 };
    let a_abs = a.unsigned_abs() as i128;
    let d_abs = d.unsigned_abs() as i128;
    let q = sign * ((2 * a_abs + d_abs) / (2 * d_abs));
    q.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn is_if_primitive(p: Primitive) -> bool {
    use Primitive::*;
    matches!(
        p,
        If | Ifcat
            | Ifx
            | Ifnum
            | Ifdim
            | Ifodd
            | Ifvmode
            | Ifhmode
            | Ifmmode
            | Ifinner
            | Ifcase
            | Iftrue
            | Iffalse
            | Ifdefined
            | Ifcsname
            | Ifhbox
            | Ifvbox
            | Ifvoid
            | Ifeof
            | Ifincsname
    )
}

fn meanings_equal(a: &Meaning, b: &Meaning) -> bool {
    match (a, b) {
        (Meaning::Undefined, Meaning::Undefined) => true,
        (Meaning::Primitive(p1), Meaning::Primitive(p2)) => p1 == p2,
        (Meaning::CharLike(t1), Meaning::CharLike(t2)) => t1.kind == t2.kind,
        (Meaning::RegisterAlias(k1, i1), Meaning::RegisterAlias(k2, i2)) => k1 == k2 && i1 == i2,
        (Meaning::CharDef(a), Meaning::CharDef(b)) => a == b,
        (Meaning::MathCharDef(a), Meaning::MathCharDef(b)) => a == b,
        (Meaning::Macro(m1), Meaning::Macro(m2)) => {
            params_equal(&m1.params, &m2.params) && body_equal(&m1.body, &m2.body) && m1.flags == m2.flags
        }
        (Meaning::Let(l1), other) => meanings_equal(l1, other),
        (other, Meaning::Let(l2)) => meanings_equal(other, l2),
        _ => false,
    }
}

/// Meaning-comparison (`\ifx`) equality of macro param/body lists must
/// ignore source spans -- two macros defined identically at different
/// source locations are still "the same" per TeXbook rule 5.
fn params_equal(a: &[ParamPart], b: &[ParamPart]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(x, y)| match (x, y) {
            (ParamPart::Literal(t1), ParamPart::Literal(t2)) => t1.kind == t2.kind,
            (ParamPart::Param(n1), ParamPart::Param(n2)) => n1 == n2,
            _ => false,
        })
}

fn body_equal(a: &[BodyPart], b: &[BodyPart]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(x, y)| match (x, y) {
            (BodyPart::Literal(t1), BodyPart::Literal(t2)) => t1.kind == t2.kind,
            (BodyPart::Param(n1), BodyPart::Param(n2)) => n1 == n2,
            _ => false,
        })
}

/// Fold `#` (catcode 6) characters in a raw macro-body token list into
/// `TokenKind::Param` placeholders: `#` followed by a digit 1-9 becomes
/// that parameter slot, `##` becomes one literal `#` (TeXbook p.204).
/// Frozen tokens (from `\the\toks`/`\unexpanded` inside an `\edef`) are
/// never folded: TeX inserts them verbatim.
fn fold_param_tokens(tokens: Vec<Pending>) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0usize;
    while i < tokens.len() {
        let t = &tokens[i];
        if !t.frozen {
            if let TokenKind::Char(_, CatCode::Param) = t.tok.kind {
                if let Some(next) = tokens.get(i + 1) {
                    if let TokenKind::Char(d, _) = next.tok.kind {
                        if let Some(n) = d.to_digit(10) {
                            if (1..=9).contains(&n) {
                                out.push(Token::new(TokenKind::Param(n as u8), t.tok.span));
                                i += 2;
                                continue;
                            }
                        }
                    }
                    if matches!(next.tok.kind, TokenKind::Char(_, CatCode::Param)) {
                        out.push(Token::new(TokenKind::Char('#', CatCode::Param), t.tok.span));
                        i += 2;
                        continue;
                    }
                }
            }
        }
        out.push(t.tok.clone());
        i += 1;
    }
    out
}

fn substitute_body(body: &[BodyPart], args: &HashMap<u8, Vec<Token>>) -> Vec<Token> {
    let mut expansion = Vec::with_capacity(body.len());
    for part in body {
        match part {
            BodyPart::Literal(t) => expansion.push(t.clone()),
            BodyPart::Param(n) => {
                if let Some(a) = args.get(n) {
                    expansion.extend(a.iter().cloned());
                }
            }
        }
    }
    expansion
}

/// Characters of an environment name shown in a diagnostic. TeX has no
/// limit on names; only the message is shortened.
const SHOWN_NAME_CHARS: usize = 100;

/// `name`, cut to [`SHOWN_NAME_CHARS`] characters plus "..." when longer.
fn shown_name(name: &str) -> std::borrow::Cow<'_, str> {
    match name.char_indices().nth(SHOWN_NAME_CHARS) {
        Some((cut, _)) => std::borrow::Cow::Owned(format!("{}...", &name[..cut])),
        None => std::borrow::Cow::Borrowed(name),
    }
}

/// Does `body`, printed token by token (`Token::display_name`, `#n` for a
/// parameter), spell exactly `name`?
fn body_spells(body: &[BodyPart], name: &str) -> bool {
    let mut rest = name;
    for part in body {
        let next = match part {
            BodyPart::Literal(t) => match &t.kind {
                TokenKind::ControlSequence(cs) => rest.strip_prefix('\\').and_then(|r| r.strip_prefix(cs.as_str())),
                TokenKind::ActiveChar(c) | TokenKind::Char(c, _) => rest.strip_prefix(*c),
                TokenKind::Param(n) => rest.strip_prefix('#').and_then(|r| r.strip_prefix(n.to_string().as_str())),
                TokenKind::Eof => Some(rest),
            },
            BodyPart::Param(n) => rest.strip_prefix('#').and_then(|r| r.strip_prefix(n.to_string().as_str())),
        };
        match next {
            Some(r) => rest = r,
            None => return false,
        }
    }
    rest.is_empty()
}

/// `body` printed token by token, stopping once it has `max_chars`
/// characters.
fn body_display(body: &[BodyPart], max_chars: usize) -> String {
    let mut out = String::new();
    let mut chars = 0;
    for part in body {
        if chars >= max_chars {
            break;
        }
        let piece = match part {
            BodyPart::Literal(t) => t.display_name(),
            BodyPart::Param(n) => format!("#{n}"),
        };
        chars += piece.chars().count();
        out.push_str(&piece);
    }
    out
}

pub(crate) fn chars_as_other(s: &str, span: Span) -> Vec<Token> {
    s.chars()
        .map(|c| {
            let cat = if c == ' ' { CatCode::Space } else { CatCode::Other };
            Token::new(TokenKind::Char(c, cat), span)
        })
        .collect()
}

/// `\alph`/`\Alph`: 1->a, 2->b, ..., 26->z; anything else is LaTeX's
/// "Counter too large" error (`None`).
fn to_alph(n: i64, upper: bool) -> Option<String> {
    if !(1..=26).contains(&n) {
        return None;
    }
    let base = if upper { b'A' } else { b'a' };
    Some(((base + (n as u8 - 1)) as char).to_string())
}

/// `\fnsymbol`: LaTeX's nine footnote symbols (ltcounts.dtx `\@fnsymbol`)
/// as Unicode: 1 ∗ (U+2217 ASTERISK OPERATOR, `\ast`), 2 † (U+2020,
/// `\dagger`), 3 ‡ (U+2021, `\ddagger`), 4 § (U+00A7, `\S`), 5 ¶
/// (U+00B6, `\P`), 6 ‖ (U+2016, `\|`), 7 ∗∗, 8 ††, 9 ‡‡. Outside 1..9
/// LaTeX raises "Counter too large" (`None`).
pub fn to_fnsymbol(n: i64) -> Option<String> {
    const SYMS: &[&str] = &["\u{2217}", "\u{2020}", "\u{2021}", "\u{00A7}", "\u{00B6}", "\u{2016}", "\u{2217}\u{2217}", "\u{2020}\u{2020}", "\u{2021}\u{2021}"];
    if n >= 1 && (n as usize) <= SYMS.len() {
        Some(SYMS[n as usize - 1].to_string())
    } else {
        None
    }
}

/// TeX's `print_scaled` (tex.web @<Print the scaled dimension@>): renders
/// a scaled-point integer as the shortest decimal that round-trips to the
/// same sp value under TeX's own rounding, e.g. 4736287sp -> "72.26999".
pub(crate) fn print_scaled(sp: i64) -> String {
    let neg = sp < 0;
    let mut s = sp.unsigned_abs() as i64;
    let unity = 65536i64;
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push_str(&(s / unity).to_string());
    out.push('.');
    s = 10 * (s % unity) + 5;
    let mut delta = 10i64;
    loop {
        if delta > unity {
            s += 32768 - delta / 2;
        }
        out.push(std::char::from_digit((s / unity) as u32, 10).unwrap());
        s = 10 * (s % unity);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

fn glue_to_string(g: Glue) -> String {
    let mut s = format!("{}pt", print_scaled(g.value));
    if g.stretch != 0 {
        s.push_str(&format!(" plus {}{}", print_scaled(g.stretch), fil_unit(g.stretch_fil)));
    }
    if g.shrink != 0 {
        s.push_str(&format!(" minus {}{}", print_scaled(g.shrink), fil_unit(g.shrink_fil)));
    }
    s
}

fn fil_unit(fil: u8) -> &'static str {
    match fil {
        1 => "fil",
        2 => "fill",
        3 => "filll",
        _ => "pt",
    }
}

fn to_roman(mut n: i64) -> String {
    if n <= 0 {
        return String::new();
    }
    const VALUES: &[(i64, &str)] = &[
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (v, s) in VALUES {
        while n >= *v {
            out.push_str(s);
            n -= *v;
        }
    }
    out
}

/// The crate's LaTeX-layer pseudo-primitives (commands real LaTeX/plain
/// define as macros), excluded from the INITEX state.
fn is_format_level(p: Primitive) -> bool {
    use Primitive::*;
    matches!(
        p,
        Newif | Newcount | Newdimen | Newskip | Newtoks | NewCommand | RenewCommand | ProvideCommand | DeclareRobustCommand
            | NewEnvironment | RenewEnvironment | Begin | End | NewCounter | SetCounter | AddToCounter | StepCounter
            | RefStepCounter | AddToReset | RemoveFromReset | CounterWithin | CounterWithout | Label | Value | Arabic
            | RomanLower | RomanUpper | AlphLower | AlphUpper | Fnsymbol | NewLength | SetToWidth | SetToHeight
            | SetToDepth | DefineKey | SetKeys | Verb | StopInput
    )
}

impl Engine {
    /// An engine in (approximately) INITEX state: only the TeX/e-TeX/
    /// pdfTeX primitives this crate models are defined, INITEX catcodes
    /// (TeXbook p. 343: `\` escape, `%` comment, letters, space, end of
    /// line; everything else "other", including `{`/`}`), no LaTeX
    /// prelude. `\end` stops input. Used by the kernel feasibility probe
    /// (`examples/kernel_probe.rs`) to run `latex.ltx` itself.
    pub fn new_initex(source: &str, limits: Limits) -> Self {
        let mut st = base_state(true);
        for code in 0u32..256 {
            if let Some(c) = char::from_u32(code) {
                let cat = match c {
                    '\\' => CatCode::Escape,
                    '%' => CatCode::Comment,
                    ' ' => CatCode::Space,
                    '\r' => CatCode::EndLine,
                    '\0' => CatCode::Ignored,
                    '\u{7f}' => CatCode::Invalid,
                    c if c.is_ascii_alphabetic() => CatCode::Letter,
                    _ => CatCode::Other,
                };
                st.scopes.set_catcode(c, cat, true);
            }
        }
        st.next_free_register = 0;
        for (name, prim) in [
            ("immediate", Primitive::Immediate),
            ("write", Primitive::Write),
            ("openout", Primitive::Openout),
            ("closeout", Primitive::Closeout),
            ("openin", Primitive::Openin),
            ("closein", Primitive::Closein),
            ("read", Primitive::Read),
            ("message", Primitive::Message),
            ("errmessage", Primitive::Errmessage),
        ] {
            st.scopes.assign_cs(name, Meaning::Primitive(prim), true);
        }
        let (year, month, day, minutes) = civil_now();
        for (i, (name, kind)) in TEX_PARAMS.iter().enumerate() {
            let idx = TEX_PARAM_BASE + i as u16;
            st.scopes.assign_cs(name, Meaning::RegisterAlias(*kind, idx), true);
            let initial = match *name {
                "tolerance" => 10000,
                "mag" => 1000,
                "maxdeadcycles" => 25,
                "hangafter" => 1,
                "year" => year,
                "month" => month,
                "day" => day,
                "time" => minutes,
                _ => 0,
            };
            if *kind == RegisterKind::Count && initial != 0 {
                st.scopes.set_count(idx, initial, true);
            }
        }
        Self::from_parts(Rc::from(source), 0, LexState::NewLine, st, limits)
    }
}

/// TeX's (and e-TeX's/pdfTeX's commonly used) internal parameters, modelled
/// in INITEX mode as registers at reserved indices `TEX_PARAM_BASE..`
/// (printed by name in `\meaning`). In the default LaTeX-mode engine they
/// stay undefined and pass through to the typesetter, which owns them.
const TEX_PARAM_BASE: u16 = 60000;
const TEX_PARAMS: &[(&str, RegisterKind)] = {
    use RegisterKind::*;
    &[
        ("pretolerance", Count), ("tolerance", Count), ("linepenalty", Count), ("hyphenpenalty", Count),
        ("exhyphenpenalty", Count), ("clubpenalty", Count), ("widowpenalty", Count), ("displaywidowpenalty", Count),
        ("brokenpenalty", Count), ("binoppenalty", Count), ("relpenalty", Count), ("predisplaypenalty", Count),
        ("postdisplaypenalty", Count), ("interlinepenalty", Count), ("doublehyphendemerits", Count),
        ("finalhyphendemerits", Count), ("adjdemerits", Count), ("mag", Count), ("delimiterfactor", Count),
        ("looseness", Count), ("time", Count), ("day", Count), ("month", Count), ("year", Count),
        ("showboxbreadth", Count), ("showboxdepth", Count), ("hbadness", Count), ("vbadness", Count), ("pausing", Count),
        ("tracingonline", Count), ("tracingmacros", Count), ("tracingstats", Count), ("tracingparagraphs", Count),
        ("tracingpages", Count), ("tracingoutput", Count), ("tracinglostchars", Count), ("tracingcommands", Count),
        ("tracingrestores", Count), ("uchyph", Count), ("outputpenalty", Count), ("maxdeadcycles", Count),
        ("hangafter", Count), ("floatingpenalty", Count), ("globaldefs", Count), ("fam", Count),
        ("defaulthyphenchar", Count), ("defaultskewchar", Count), ("language", Count), ("lefthyphenmin", Count),
        ("righthyphenmin", Count), ("holdinginserts", Count), ("errorcontextlines", Count), ("tracingassigns", Count),
        ("tracinggroups", Count), ("tracingifs", Count), ("tracingscantokens", Count), ("tracingnesting", Count),
        ("predisplaydirection", Count), ("lastlinefit", Count), ("savingvdiscards", Count), ("savinghyphcodes", Count),
        ("pdfoutput", Count), ("pdfcompresslevel", Count), ("pdfobjcompresslevel", Count), ("pdfdecimaldigits", Count),
        ("pdfpkresolution", Count), ("pdfminorversion", Count), ("pdfmajorversion", Count),
        ("parindent", Dimen), ("mathsurround", Dimen), ("lineskiplimit", Dimen), ("hsize", Dimen), ("vsize", Dimen),
        ("maxdepth", Dimen), ("splitmaxdepth", Dimen), ("boxmaxdepth", Dimen), ("hfuzz", Dimen), ("vfuzz", Dimen),
        ("delimitershortfall", Dimen), ("nulldelimiterspace", Dimen), ("scriptspace", Dimen),
        ("predisplaysize", Dimen), ("displaywidth", Dimen), ("displayindent", Dimen), ("overfullrule", Dimen),
        ("hangindent", Dimen), ("hoffset", Dimen), ("voffset", Dimen), ("emergencystretch", Dimen),
        ("pdfpagewidth", Dimen), ("pdfpageheight", Dimen), ("pdfhorigin", Dimen), ("pdfvorigin", Dimen),
        ("baselineskip", Skip), ("lineskip", Skip), ("parskip", Skip), ("abovedisplayskip", Skip),
        ("belowdisplayskip", Skip), ("abovedisplayshortskip", Skip), ("belowdisplayshortskip", Skip),
        ("leftskip", Skip), ("rightskip", Skip), ("topskip", Skip), ("splittopskip", Skip), ("tabskip", Skip),
        ("spaceskip", Skip), ("xspaceskip", Skip), ("parfillskip", Skip), ("thinmuskip", Skip),
        ("medmuskip", Skip), ("thickmuskip", Skip),
        ("output", Toks), ("everypar", Toks), ("everymath", Toks), ("everydisplay", Toks), ("everyhbox", Toks),
        ("everyvbox", Toks), ("everyjob", Toks), ("everycr", Toks), ("errhelp", Toks), ("everyeof", Toks),
    ]
};

/// (year, month, day, minutes since midnight) in UTC, for INITEX's
/// `\year`/`\month`/`\day`/`\time` (Hinnant's civil-from-days).
fn civil_now() -> (i64, i64, i64, i64) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let days = secs.div_euclid(86400);
    let minutes = secs.rem_euclid(86400) / 60;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d, minutes)
}

/// Primitives bound (all of them, or with `tex_only` just the real TeX/
/// e-TeX/pdfTeX ones plus `\end` = stop), no macros.
fn base_state(tex_only: bool) -> State {
    let mut scopes = Scopes::new();
    for (name, prim) in PRIMITIVE_TABLE {
        if tex_only && is_format_level(*prim) {
            continue;
        }
        scopes.assign_cs(name, Meaning::Primitive(*prim), true);
    }
    if tex_only {
        scopes.assign_cs("end", Meaning::Primitive(Primitive::StopInput), true);
    }
    State {
        scopes,
        conditionals: ConditionalStack::default(),
        pending_global: false,
        pending_long: false,
        pending_outer: false,
        pending_protected: false,
        after_assignment: None,
        mode: Mode::Vertical,
        counter_children: Rc::new(HashMap::new()),
        next_free_register: 256,
        next_source_id: 1,
        prelude_source_end: 1,
        scanner_status: ScannerStatus::Normal,
        matching_long: false,
        runaway_par: false,
        runaway_par_silent: false,
        edef_depth: 0,
        in_csname: 0,
        emit_unbalanced_close: false,
        group_limit_reported: false,
        conditional_limit_reported: false,
    }
}

/// Build the state every document starts from: primitives bound, then the
/// LaTeX-kernel prelude (`prelude.rs`) executed once.
fn build_initial_state() -> State {
    let mut st = base_state(false);
    // The kernel prelude gets a source id of its own, like a host prelude:
    // with id 0 its macros' spans would alias bytes of the user's document,
    // so an incremental edit before byte ~4 KB "shifted" them and a re-run
    // could never converge with the previous run.
    let id = st.next_source_id;
    st.next_source_id += 1;
    st.prelude_source_end = st.next_source_id;
    let mut engine = Engine::from_parts(Rc::from(PRELUDE), 0, LexState::NewLine, st, Limits::default());
    engine.sources[0] = Input::Text(Lexer::new(Rc::from(PRELUDE), id));
    let out = engine.run();
    // End-of-line spaces after `}` are the only legitimate output (TeX's
    // vertical mode drops them; we have no modes).
    let stray: Vec<&Token> = out.iter().filter(|t| !matches!(t.kind, TokenKind::Char(' ', CatCode::Space))).collect();
    debug_assert!(stray.is_empty(), "prelude produced output tokens: {:?}", stray);
    debug_assert!(engine.diagnostics.is_empty(), "prelude produced diagnostics: {:?}", engine.diagnostics);
    engine.st
}
