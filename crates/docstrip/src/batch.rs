//! The batch-file interpreter (docstrip.dtx §"Batchfile commands", lines
//! 433–727 for the user interface and 2871–3735 for the implementation).
//!
//! A batch file is executed token by token the way TeX would, with a
//! deliberately small vocabulary: docstrip's own commands, the pre- and
//! postamble machinery, and enough of TeX's macro layer (`\def` with
//! parameters, `\let`, `\edef`, groups, `\iffalse`…`\fi`, `\string`)
//! for the helper macros real batch files define. Everything else is a
//! diagnostic naming the command and the line; execution continues, so
//! stray TeX after the `\generate` (as in `lipsum.ins`) costs nothing but
//! notes.
//!
//! Pre- and postambles are what docstrip makes them: macros whose
//! replacement text was built by `\edef` at `\declarepreamble` time
//! (line 3524), with `\outFileName`, `\inFileName` and `\ReferenceLines`
//! left unexpanded (they are `\relax` then, line 3529) and filled in when
//! the file is written (`\WritePreamble`, line 3697). `\MetaPrefix`
//! expands at declaration time unless a batch file has `\let` it to
//! `\relax` (microtype does), in which case it expands at write time —
//! the same token-level behaviour falls out of treating ambles as token
//! lists here.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use crate::lexer::{self, Cat, Catcodes, Lexer, Token};
use crate::{strip, Diagnostic, GeneratedFile, Options, Outcome, Sources, DOCSTRIP_VERSION};

/// A sentinel control sequence marking the end of a token list pushed
/// for execution or expansion; the lexer cannot produce it (a NUL is an
/// ignored character).
const END: &[u8] = b"\0end";

/// A batch file that programs TeX beyond what is interpreted here can
/// loop (a recursion guarded by an `\ifx` this interpreter takes as
/// true). Runs are bounded in tokens read and tokens waiting; a real
/// batch file needs a few thousand.
const STEP_LIMIT: u64 = 2_000_000;
const PENDING_LIMIT: usize = 1_000_000;

/// The batch-file commands this interpreter executes. Anything else is
/// looked up as a macro, else reported.
const PRIMITIVES: &[&str] = &[
    "input", "endbatchfile", "keepsilent", "showprogress", "askforoverwritetrue", "askforoverwritefalse", "askonceonly", "preamble", "postamble", "declarepreamble",
    "declarepostamble", "usepreamble", "usepostamble", "nopreamble", "nopostamble", "usedir", "BaseDirectory", "UseTDS", "DeclareDir", "generate", "file", "from", "needed",
    "generateFile", "include", "processFile", "Msg", "ifToplevel", "batchinput", "AddGenerationDate", "def", "gdef", "edef", "xdef", "long", "outer", "protected",
    "global", "let", "begingroup", "endgroup", "iffalse", "iftrue", "ifcase", "ifnum", "ifx", "if", "else", "or", "fi", "relax", "par", "obeyspaces", "makeatletter", "makeatother", "catcode", "newlinechar",
    "escapechar", "endlinechar", "lccode", "uccode", "string", "noexpand", "checkeoln", "endpreamble", "endpostamble", "batchfile", "endinput", "end", "@@end", "active",
];

#[derive(Clone, Debug)]
struct Macro {
    /// Tokens that must follow the name before the first parameter.
    prefix: Vec<Token>,
    /// For parameter `i+1`, the delimiter tokens that end it (empty:
    /// undelimited).
    delimiters: Vec<Vec<Token>>,
    /// Replacement text with `Token::Arg(n)` for `#n`.
    body: Vec<Token>,
}

#[derive(Clone, Debug)]
enum Meaning {
    Macro(Rc<Macro>),
    /// `\let` to `\relax` (or the docstrip placeholders before `\generate`
    /// defines them): unexpandable, does nothing.
    Relax,
    /// `\let\bgroup={`: the control sequence acts as that character.
    Char(Token),
    Primitive,
    Undefined,
}

/// Everything a group restores (`\begingroup`/`{`).
#[derive(Clone)]
struct Locals {
    macros: BTreeMap<Vec<u8>, Meaning>,
    /// `\catcode` assignments are local to the group too.
    catcodes: Catcodes,
    ask_for_overwrite: bool,
    current_preamble: Vec<u8>,
    current_postamble: Vec<u8>,
    dest_dir: Option<String>,
}

/// `\generate` in progress: the input files in reading order and the
/// `\file`s declared so far (docstrip.dtx lines 2871–2913).
#[derive(Default)]
struct Generate {
    /// Uniquized source names (a repeat within one `\file` gets a
    /// trailing space per repeat, `\@need@d`, line 3126), in the order
    /// they are read.
    inputs: Vec<String>,
    files: Vec<FileSpec>,
}

struct FileSpec {
    name: String,
    /// (uniquized source, options) in `\from` order.
    passes: Vec<(String, Vec<u8>)>,
    /// `\curinnames`: every `\from` name, repeats included.
    names: Vec<String>,
    preamble: Vec<Token>,
    postamble: Vec<Token>,
    /// `\ref@…`: the "original source files" lines, expanded at `\file` time.
    reference: Vec<Token>,
    dir: Option<String>,
}

/// `\file` in progress.
struct FileCtx {
    name: String,
    passes: Vec<(String, Vec<u8>)>,
    curinfiles: Vec<String>,
    names: Vec<String>,
    reference: Vec<Token>,
}

struct Input {
    name: String,
    lexer: Lexer,
    pending: VecDeque<Token>,
}

pub struct Interpreter<'a> {
    sources: &'a dyn Sources,
    options: &'a Options,
    inputs: Vec<Input>,
    locals: Locals,
    saved: Vec<Locals>,
    generate: Option<Generate>,
    file: Option<FileCtx>,
    out: Outcome,
    stopped: bool,
    aborted: bool,
    /// After `\input docstrip`: `\input` is then docstrip's no-op.
    docstrip_loaded: bool,
    steps: u64,
    /// Commands reported once each.
    reported: BTreeMap<Vec<u8>, ()>,
    stray_reported: BTreeMap<String, ()>,
}

impl<'a> Interpreter<'a> {
    pub fn new(sources: &'a dyn Sources, options: &'a Options) -> Interpreter<'a> {
        let mut macros = BTreeMap::new();
        let chars = |s: &str| s.bytes().map(|b| Token::Char(b, Cat::Other)).collect::<Vec<_>>();
        let simple = |body: Vec<Token>| Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body }));
        macros.insert(b"space".to_vec(), simple(vec![Token::Char(b' ', Cat::Space)]));
        macros.insert(b"empty".to_vec(), simple(vec![]));
        macros.insert(b"DoubleperCent".to_vec(), simple(chars("%%")));
        macros.insert(b"perCent".to_vec(), simple(chars("%")));
        macros.insert(b"MetaPrefix".to_vec(), simple(vec![Token::cs("DoubleperCent")]));
        macros.insert(b"outFileName".to_vec(), Meaning::Relax);
        macros.insert(b"inFileName".to_vec(), Meaning::Relax);
        macros.insert(b"ReferenceLines".to_vec(), Meaning::Relax);
        macros.insert(b"bgroup".to_vec(), Meaning::Char(Token::Char(b'{', Cat::BeginGroup)));
        macros.insert(b"egroup".to_vec(), Meaning::Char(Token::Char(b'}', Cat::EndGroup)));
        macros.insert(b"undefined".to_vec(), Meaning::Undefined);
        macros.insert(b"active".to_vec(), Meaning::Primitive);
        // plain.tex line 380: the active space (under \obeyspaces) is \space.
        macros.insert(active_key(b' '), simple(vec![Token::cs("space")]));
        // Batch files are run with plain TeX; a `.dtx` that is `\input`
        // stops itself with `\ifx\tmpa\fmtname\expandafter\endinput\fi`.
        macros.insert(b"fmtname".to_vec(), simple(b"plain".iter().map(|&b| Token::Char(b, Cat::Letter)).collect()));
        let locals = Locals { macros, catcodes: lexer::plain_catcodes(), ask_for_overwrite: true, current_preamble: b"defaultpreamble".to_vec(), current_postamble: b"defaultpostamble".to_vec(), dest_dir: None };
        let mut me = Interpreter { sources, options, inputs: Vec::new(), locals, saved: Vec::new(), generate: None, file: None, out: Outcome::default(), stopped: false, aborted: false, docstrip_loaded: false, steps: 0, reported: BTreeMap::new(), stray_reported: BTreeMap::new() };
        // docstrip's own defaults, declared the way docstrip.tex declares
        // them (docstrip.dtx lines 3596–3628 and 3650–3663).
        me.inputs.push(Input { name: "docstrip.tex".into(), lexer: Lexer::new(DEFAULTS.as_bytes()), pending: VecDeque::new() });
        me.execute();
        me.inputs.pop();
        debug_assert!(me.out.diagnostics.is_empty(), "{:?}", me.out.diagnostics);
        me
    }

    /// Executes `batch` to `\endbatchfile` or its end.
    pub fn run(mut self, batch_name: &str, batch: &[u8]) -> Outcome {
        let stem = batch_name.rsplit('/').next().unwrap_or(batch_name);
        let stem = stem.strip_suffix(".ins").unwrap_or(stem);
        self.define(b"jobname", Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body: stem.bytes().map(|b| Token::Char(b, Cat::Other)).collect() })), true);
        self.inputs.push(Input { name: batch_name.into(), lexer: Lexer::new(batch), pending: VecDeque::new() });
        self.execute();
        self.inputs.pop();
        self.out.completed = !self.aborted;
        self.out
    }

    // ----- tokens ---------------------------------------------------------

    fn diag(&mut self, message: impl Into<String>) {
        let (file, line) = self.loc();
        self.out.diagnostics.push(Diagnostic { file, line, message: message.into() });
    }

    fn loc(&self) -> (String, usize) {
        match self.inputs.last() {
            Some(i) => (i.name.clone(), i.lexer.last_line),
            None => ("docstrip".into(), 0),
        }
    }

    fn next_raw(&mut self) -> Option<Token> {
        if self.steps >= STEP_LIMIT {
            if !self.stopped {
                self.stopped = true;
                self.aborted = true;
                self.diag(format!("giving up after {STEP_LIMIT} tokens: the batch file loops in TeX the interpreter does not evaluate"));
            }
            return None;
        }
        self.steps += 1;
        let input = self.inputs.last_mut()?;
        if let Some(t) = input.pending.pop_front() {
            return Some(t);
        }
        input.lexer.next()
    }

    fn push_front(&mut self, tokens: Vec<Token>) {
        if let Some(input) = self.inputs.last_mut() {
            if input.pending.len() + tokens.len() > PENDING_LIMIT {
                self.steps = STEP_LIMIT;
                return;
            }
            for t in tokens.into_iter().rev() {
                input.pending.push_front(t);
            }
        }
    }

    /// A source or nested batch file: one this run generated (docstrip
    /// writes them to disk and reads them back; xcolor.ins generates
    /// `xcolor.lox` and then `\batchinput`s it), else from `sources`.
    fn read_source(&self, name: &str) -> Option<Vec<u8>> {
        if let Some(f) = self.out.files.iter().find(|f| f.name == name) {
            return Some(f.bytes.clone());
        }
        self.sources.read(name)
    }

    fn meaning(&self, name: &[u8]) -> Meaning {
        match self.locals.macros.get(name) {
            Some(m) => m.clone(),
            None if PRIMITIVES.iter().any(|p| p.as_bytes() == name) => Meaning::Primitive,
            None => Meaning::Undefined,
        }
    }

    fn define(&mut self, name: &[u8], meaning: Meaning, global: bool) {
        self.locals.macros.insert(name.to_vec(), meaning.clone());
        if global {
            for s in &mut self.saved {
                s.macros.insert(name.to_vec(), meaning.clone());
            }
        }
    }

    /// The next token with macros expanded: what `\edef` and the main
    /// loop see. `\string` and `\noexpand` are handled here (they are
    /// expansion-time in TeX).
    fn next_expanded(&mut self) -> Option<Token> {
        loop {
            let t = self.next_raw()?;
            let name: Vec<u8> = match &t {
                Token::Cs(n) if n == END => return Some(t),
                Token::Cs(n) => n.clone(),
                // An active character is looked up like a control sequence.
                Token::Char(b, Cat::Active) => active_key(*b),
                _ => return Some(t),
            };
            let name = name.as_slice();
            match self.meaning(name) {
                Meaning::Macro(m) => {
                    if !self.expand_macro(name, &m) {
                        return Some(t);
                    }
                }
                Meaning::Char(c) => return Some(c),
                Meaning::Primitive if name == b"string" => {
                    let next = self.next_raw()?;
                    let text: Vec<Token> = match next {
                        Token::Cs(n) if n == END => {
                            self.push_front(vec![Token::Cs(n)]);
                            vec![]
                        }
                        Token::Cs(n) => std::iter::once(b'\\').chain(n.iter().copied()).map(|b| Token::Char(b, Cat::Other)).collect(),
                        Token::Char(b, Cat::Space) => vec![Token::Char(b, Cat::Space)],
                        Token::Char(b, _) => vec![Token::Char(b, Cat::Other)],
                        Token::Arg(n) => vec![Token::Char(b'#', Cat::Other), Token::Char(b'0' + n, Cat::Other)],
                    };
                    self.push_front(text);
                }
                Meaning::Primitive if name == b"noexpand" => {
                    return self.next_raw();
                }
                // \checkeoln#1: nothing when #1 is the active line end, #1 otherwise (docstrip.dtx line 3537).
                Meaning::Primitive if name == b"checkeoln" => match self.next_raw() {
                    Some(n) if n.is_eol() => {}
                    Some(n) => self.push_front(vec![n]),
                    None => {}
                },
                // Conditionals are expandable: they act inside `\edef` and
                // file names (`\file{#2.\ifcase#1sty\or tex\fi}` in xcolor.lox).
                Meaning::Primitive if name == b"iffalse" => self.take_false_branch(),
                Meaning::Primitive if matches!(name, b"iftrue" | b"fi") => {}
                Meaning::Primitive if matches!(name, b"else" | b"or") => {
                    let _ = self.skip_conditional(false, false);
                }
                Meaning::Primitive if name == b"ifcase" => self.cmd_ifcase(),
                Meaning::Primitive if name == b"ifnum" => self.cmd_ifnum(),
                Meaning::Primitive if name == b"ifx" => self.cmd_ifx(),
                Meaning::Primitive if name == b"if" => self.cmd_if(),
                _ => return Some(t),
            }
        }
    }

    /// Fully expands `tokens` (as `\edef` would).
    fn expand_tokens(&mut self, tokens: Vec<Token>) -> Vec<Token> {
        let mut list = tokens;
        list.push(Token::Cs(END.to_vec()));
        self.push_front(list);
        let mut out = Vec::new();
        while let Some(t) = self.next_expanded() {
            if t.is_cs_bytes(END) {
                break;
            }
            out.push(t);
        }
        out
    }

    /// Reads the arguments of `m` and pushes its replacement text.
    /// `false` when the call does not match its definition (reported).
    fn expand_macro(&mut self, name: &[u8], m: &Macro) -> bool {
        for expected in &m.prefix {
            match self.next_raw() {
                Some(t) if &t == expected => {}
                other => {
                    if let Some(t) = other {
                        self.push_front(vec![t]);
                    }
                    self.diag(format!("Use of \\{} doesn't match its definition", show(name)));
                    return false;
                }
            }
        }
        let mut args: Vec<Vec<Token>> = Vec::with_capacity(m.delimiters.len());
        for delim in &m.delimiters {
            let arg = if delim.is_empty() { self.read_undelimited() } else { self.read_delimited(delim) };
            match arg {
                Some(a) => args.push(a),
                None => {
                    self.diag(format!("runaway argument of \\{}", show(name)));
                    return false;
                }
            }
        }
        let mut out = Vec::with_capacity(m.body.len());
        for t in &m.body {
            match t {
                Token::Arg(n) => out.extend(args.get(*n as usize - 1).cloned().unwrap_or_default()),
                _ => out.push(t.clone()),
            }
        }
        self.push_front(out);
        true
    }

    /// An undelimited argument: spaces skipped, one token or a braced
    /// group without its braces. `None` at the end of the input or a
    /// sentinel (which is put back).
    fn read_undelimited(&mut self) -> Option<Vec<Token>> {
        loop {
            let t = self.next_raw()?;
            match t {
                Token::Char(_, Cat::Space) => continue,
                Token::Char(_, Cat::BeginGroup) => return self.read_group_body(),
                Token::Char(_, Cat::EndGroup) => {
                    self.push_front(vec![t]);
                    return None;
                }
                Token::Cs(ref n) if n == END => {
                    self.push_front(vec![t]);
                    return None;
                }
                _ => return Some(vec![t]),
            }
        }
    }

    /// The body of a group whose `{` was consumed, without the braces.
    fn read_group_body(&mut self) -> Option<Vec<Token>> {
        let mut depth = 1usize;
        let mut out = Vec::new();
        loop {
            let t = self.next_raw()?;
            match &t {
                Token::Char(_, Cat::BeginGroup) => depth += 1,
                Token::Char(_, Cat::EndGroup) => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(out);
                    }
                }
                Token::Cs(n) if n == END => {
                    self.push_front(vec![t]);
                    return None;
                }
                _ => {}
            }
            out.push(t);
        }
    }

    /// `{…}` after optional spaces; `None` (reported) otherwise.
    fn read_group(&mut self, what: &str) -> Option<Vec<Token>> {
        loop {
            match self.next_raw() {
                Some(Token::Char(_, Cat::Space)) => continue,
                Some(Token::Char(_, Cat::BeginGroup)) => {
                    let body = self.read_group_body();
                    if body.is_none() {
                        self.diag(format!("{what}: argument not closed before the end of the input"));
                    }
                    return body;
                }
                Some(t) => {
                    self.push_front(vec![t]);
                    self.diag(format!("{what}: expected `{{`"));
                    return None;
                }
                None => {
                    self.diag(format!("{what}: expected `{{` but the input ended"));
                    return None;
                }
            }
        }
    }

    /// A delimited argument: everything up to `delim` at brace depth 0,
    /// braces stripped when the whole argument is one group.
    fn read_delimited(&mut self, delim: &[Token]) -> Option<Vec<Token>> {
        let mut acc: Vec<Token> = Vec::new();
        let mut depth = 0usize;
        loop {
            let t = self.next_raw()?;
            match &t {
                Token::Cs(n) if n == END => {
                    self.push_front(vec![t]);
                    return None;
                }
                Token::Char(_, Cat::BeginGroup) => depth += 1,
                Token::Char(_, Cat::EndGroup) => {
                    if depth == 0 {
                        self.push_front(vec![t]);
                        return None;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            acc.push(t);
            if depth == 0 && acc.len() >= delim.len() && acc[acc.len() - delim.len()..] == *delim {
                acc.truncate(acc.len() - delim.len());
                break;
            }
        }
        if acc.len() >= 2 && matches!(acc.first(), Some(Token::Char(_, Cat::BeginGroup))) && matches!(acc.last(), Some(Token::Char(_, Cat::EndGroup))) {
            // One group exactly?
            let mut d = 0i32;
            let mut closes_at_end = true;
            for (i, t) in acc.iter().enumerate() {
                match t {
                    Token::Char(_, Cat::BeginGroup) => d += 1,
                    Token::Char(_, Cat::EndGroup) => {
                        d -= 1;
                        if d == 0 && i + 1 != acc.len() {
                            closes_at_end = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if closes_at_end {
                acc.pop();
                acc.remove(0);
            }
        }
        Some(acc)
    }

    /// An argument, fully expanded, as text.
    fn read_text_arg(&mut self, what: &str) -> Option<Vec<u8>> {
        let raw = self.read_undelimited();
        let Some(raw) = raw else {
            self.diag(format!("{what}: missing argument"));
            return None;
        };
        let expanded = self.expand_tokens(raw);
        Some(self.write(&expanded))
    }

    /// A control-sequence argument (`\usepreamble\foo`), after spaces.
    fn read_cs_arg(&mut self, what: &str) -> Option<Vec<u8>> {
        loop {
            match self.next_raw() {
                Some(Token::Char(_, Cat::Space)) => continue,
                Some(Token::Cs(n)) if n != END => return Some(n),
                Some(t) => {
                    self.push_front(vec![t]);
                    self.diag(format!("{what}: expected a control sequence"));
                    return None;
                }
                None => {
                    self.diag(format!("{what}: expected a control sequence but the input ended"));
                    return None;
                }
            }
        }
    }

    /// Renders tokens the way `\write` prints them: characters as
    /// themselves, `^^J` (character 10, docstrip's `\newlinechar`, line
    /// 1107) as a line break, a control word as `\name` followed by a
    /// space, a control symbol as `\c`.
    fn write(&self, tokens: &[Token]) -> Vec<u8> {
        let mut out = Vec::with_capacity(tokens.len());
        for t in tokens {
            match t {
                Token::Char(_, Cat::Space) => out.push(b' '),
                Token::Char(10, _) => out.push(b'\n'),
                Token::Char(b, _) => out.push(*b),
                Token::Cs(n) => {
                    out.push(b'\\');
                    out.extend_from_slice(n);
                    if n.len() != 1 || n[0].is_ascii_alphabetic() || n[0] == b'@' {
                        out.push(b' ');
                    }
                }
                Token::Arg(n) => {
                    out.push(b'#');
                    out.push(b'0' + n);
                }
            }
        }
        out
    }

    // ----- execution ------------------------------------------------------

    fn push_group(&mut self) {
        self.saved.push(self.locals.clone());
    }

    fn pop_group(&mut self) {
        if let Some(l) = self.saved.pop() {
            self.locals = l;
            self.sync_catcodes();
        }
    }

    /// The lexer reads under the current group's category codes.
    fn sync_catcodes(&mut self) {
        if let Some(i) = self.inputs.last_mut() {
            i.lexer.catcodes = self.locals.catcodes;
        }
    }

    fn set_catcode(&mut self, byte: u8, code: u8) {
        self.locals.catcodes[byte as usize] = code;
        self.sync_catcodes();
    }

    /// Executes tokens from the current input until its end, a sentinel,
    /// or `\endbatchfile`.
    fn execute(&mut self) {
        let base = self.saved.len();
        loop {
            if self.stopped {
                break;
            }
            let Some(t) = self.next_expanded() else { break };
            match t {
                Token::Cs(name) if name == END => break,
                Token::Cs(name) => self.command(&name),
                Token::Char(_, Cat::BeginGroup) => self.push_group(),
                Token::Char(_, Cat::EndGroup) => {
                    if self.saved.len() > base {
                        self.pop_group();
                    } else if self.reported.insert(b"}".to_vec(), ()).is_none() {
                        self.diag("unmatched `}` (reported once)");
                    }
                }
                Token::Char(_, Cat::Space) => {}
                Token::Char(b, _) => self.stray(b),
                Token::Arg(_) => {}
            }
        }
        if self.saved.len() > base {
            self.diag(format!("{} group(s) not closed", self.saved.len() - base));
            while self.saved.len() > base {
                self.pop_group();
            }
        }
    }

    /// Executes a token list in place (with a sentinel after it).
    fn execute_tokens(&mut self, tokens: Vec<Token>) {
        let mut list = tokens;
        list.push(Token::Cs(END.to_vec()));
        self.push_front(list);
        self.execute();
    }

    fn stray(&mut self, byte: u8) {
        let (file, _) = self.loc();
        if self.stray_reported.insert(file, ()).is_none() {
            self.diag(format!("stray text starting with {:?} is ignored (only docstrip commands act here)", byte as char));
        }
    }

    fn command(&mut self, name: &[u8]) {
        match self.meaning(name) {
            Meaning::Relax => return,
            Meaning::Undefined => {
                if name.starts_with(b"if") && name != b"ifToplevel" {
                    self.note_once(name, &format!("\\{} is not evaluated; its true branch is taken", show(name)));
                } else if self.reported.insert(name.to_vec(), ()).is_none() {
                    self.diag(format!("\\{} is not a docstrip command; ignored here and wherever else it occurs", show(name)));
                }
                return;
            }
            Meaning::Macro(_) | Meaning::Char(_) => return, // expanded before reaching here
            Meaning::Primitive => {}
        }
        match name {
            b"input" => self.cmd_input(),
            b"endbatchfile" => {
                if self.inputs.len() <= 1 {
                    self.stopped = true;
                } else {
                    self.end_current_input();
                }
            }
            b"endinput" => self.end_current_input(),
            // TeX's `\end` (and LaTeX's saved copy `\@@end`) stops the job;
            // `\end{…}` is a LaTeX environment end and does nothing here.
            b"end" | b"@@end" => match self.next_raw() {
                Some(t) if t.cat() == Some(Cat::BeginGroup) => {
                    let _ = self.read_group_body();
                }
                Some(t) => {
                    self.push_front(vec![t]);
                    self.stopped = true;
                }
                None => self.stopped = true,
            },
            b"keepsilent" | b"showprogress" | b"relax" | b"par" | b"makeatletter" | b"makeatother" | b"fi" | b"iftrue" => {}
            // plain.tex line 380: `\catcode`\ =\active`, the active space being `\space`.
            b"obeyspaces" => self.set_catcode(b' ', lexer::ACTIVE),
            b"askforoverwritetrue" => self.locals.ask_for_overwrite = true,
            b"askforoverwritefalse" => self.locals.ask_for_overwrite = false,
            b"askonceonly" => {}
            b"AddGenerationDate" => self.cmd_add_generation_date(),
            b"preamble" => {
                self.locals.current_preamble = b"defaultpreamble".to_vec();
                self.declare_amble(b"defaultpreamble", true);
            }
            b"postamble" => {
                self.locals.current_postamble = b"defaultpostamble".to_vec();
                self.declare_amble(b"defaultpostamble", false);
            }
            b"declarepreamble" | b"declarepostamble" => {
                let pre = name == b"declarepreamble";
                if let Some(n) = self.read_cs_arg(&format!("\\{}", show(name))) {
                    self.declare_amble(&n, pre);
                }
            }
            b"usepreamble" | b"usepostamble" => {
                if let Some(n) = self.read_cs_arg(&format!("\\{}", show(name))) {
                    if name == b"usepreamble" {
                        self.locals.current_preamble = n;
                    } else {
                        self.locals.current_postamble = n;
                    }
                }
            }
            b"nopreamble" => self.locals.current_preamble = b"empty".to_vec(),
            b"nopostamble" => self.locals.current_postamble = b"empty".to_vec(),
            b"endpreamble" | b"endpostamble" => self.diag(format!("\\{} without a matching declaration", show(name))),
            b"usedir" => {
                if let Some(d) = self.read_text_arg("\\usedir") {
                    self.locals.dest_dir = Some(String::from_utf8_lossy(&d).into_owned());
                }
            }
            b"BaseDirectory" => {
                let _ = self.read_text_arg("\\BaseDirectory");
                self.note_once(name, "\\BaseDirectory is a docstrip.cfg command; ignored, files are written next to the batch file");
            }
            b"UseTDS" => self.note_once(name, "\\UseTDS is a docstrip.cfg command; ignored"),
            b"DeclareDir" => {
                // \DeclareDir[*]{label}{directory} (docstrip.dtx line 3856)
                match self.next_raw() {
                    Some(t) if t.is_char(b'*') => {}
                    Some(t) => self.push_front(vec![t]),
                    None => {}
                }
                let _ = self.read_text_arg("\\DeclareDir");
                let _ = self.read_text_arg("\\DeclareDir");
                self.note_once(name, "\\DeclareDir is a docstrip.cfg command; ignored");
            }
            b"generate" => self.cmd_generate(),
            b"file" => self.cmd_file(),
            b"from" => self.cmd_from(),
            b"needed" => {
                if let Some(n) = self.read_text_arg("\\needed") {
                    if self.file.is_some() {
                        self.needed(&String::from_utf8_lossy(&n));
                    } else {
                        self.diag("\\needed can only be used in argument to \\file");
                    }
                }
            }
            b"generateFile" => {
                // \generateFile{out}{t|f}{\from…} = a group setting the
                // overwrite switch around \generate{\file{out}{…}} (line 3869).
                let Some(out) = self.read_undelimited() else { return self.diag("\\generateFile: missing arguments") };
                let Some(ask) = self.read_text_arg("\\generateFile") else { return };
                let Some(spec) = self.read_undelimited() else { return self.diag("\\generateFile: missing arguments") };
                let mut list = vec![Token::Char(b'{', Cat::BeginGroup), Token::cs(if ask == b"t" { "askforoverwritetrue" } else { "askforoverwritefalse" }), Token::cs("generate"), Token::Char(b'{', Cat::BeginGroup), Token::cs("file"), Token::Char(b'{', Cat::BeginGroup)];
                list.extend(out);
                list.push(Token::Char(b'}', Cat::EndGroup));
                list.push(Token::Char(b'{', Cat::BeginGroup));
                list.extend(spec);
                list.extend([Token::Char(b'}', Cat::EndGroup), Token::Char(b'}', Cat::EndGroup), Token::Char(b'}', Cat::EndGroup)]);
                self.push_front(list);
            }
            b"include" => {
                // \include{options} defines \Options (line 3889).
                if let Some(opts) = self.read_undelimited() {
                    self.define(b"Options", Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body: opts })), false);
                }
            }
            b"processFile" => {
                // \processFile{name}{inext}{outext}{ask} =
                // \generateFile{name.outext}{ask}{\from{name.inext}{\Options}} (line 3913).
                let args: Vec<Option<Vec<Token>>> = (0..4).map(|_| self.read_undelimited()).collect();
                let [Some(n), Some(i), Some(o), Some(a)] = args.as_slice() else { return self.diag("\\processFile: missing arguments") };
                let mut list = vec![Token::cs("generateFile"), Token::Char(b'{', Cat::BeginGroup)];
                list.extend(n.clone());
                list.push(Token::Char(b'.', Cat::Other));
                list.extend(o.clone());
                list.extend([Token::Char(b'}', Cat::EndGroup), Token::Char(b'{', Cat::BeginGroup)]);
                list.extend(a.clone());
                list.extend([Token::Char(b'}', Cat::EndGroup), Token::Char(b'{', Cat::BeginGroup), Token::cs("from"), Token::Char(b'{', Cat::BeginGroup)]);
                list.extend(n.clone());
                list.push(Token::Char(b'.', Cat::Other));
                list.extend(i.clone());
                list.extend([Token::Char(b'}', Cat::EndGroup), Token::Char(b'{', Cat::BeginGroup), Token::cs("Options"), Token::Char(b'}', Cat::EndGroup), Token::Char(b'}', Cat::EndGroup)]);
                self.push_front(list);
            }
            b"Msg" => {
                if let Some(text) = self.read_text_arg("\\Msg") {
                    self.out.messages.push(String::from_utf8_lossy(&text).into_owned());
                }
            }
            b"ifToplevel" => {
                if let Some(body) = self.read_group("\\ifToplevel") {
                    if self.inputs.len() <= 1 {
                        self.push_front(body);
                    }
                }
            }
            b"batchinput" => self.cmd_batchinput(),
            b"def" | b"gdef" | b"edef" | b"xdef" | b"long" | b"outer" | b"protected" | b"global" | b"let" => {
                self.push_front(vec![Token::Cs(name.to_vec())]);
                self.cmd_definition();
            }
            b"begingroup" => self.push_group(),
            b"endgroup" => {
                if self.saved.is_empty() {
                    self.diag("\\endgroup without \\begingroup");
                } else {
                    self.pop_group();
                }
            }
            // Conditionals act at expansion time (`next_expanded`); they
            // never reach here.
            b"iffalse" | b"else" | b"or" | b"ifcase" | b"ifnum" | b"ifx" | b"if" => {}
            b"catcode" => self.cmd_catcode(),
            b"newlinechar" | b"escapechar" | b"endlinechar" | b"lccode" | b"uccode" => {
                self.skip_assignment();
                self.note_once(name, &format!("\\{} has no effect here", show(name)));
            }
            b"batchfile" | b"string" | b"noexpand" | b"checkeoln" | b"active" => {}
            _ => self.diag(format!("\\{} is listed as a command but has no implementation (a bug in this crate)", show(name))),
        }
    }

    fn note_once(&mut self, name: &[u8], message: &str) {
        if self.reported.insert(name.to_vec(), ()).is_none() {
            self.diag(message.to_string());
        }
    }

    /// `\input docstrip` (any spelling docstrip accepts) marks docstrip as
    /// loaded. Before that line, `\input` is TeX's own and reads the file
    /// in place (fontspec.ins inputs its `.dtx` first, whose `%<*dtx>`
    /// code defines the macro that lists its sources); after it, docstrip
    /// has redefined `\input` to ignore its argument (docstrip.dtx line
    /// 1310), and so does this.
    fn cmd_input(&mut self) {
        // The file name runs to a space (the end of the line is one) or
        // to a control sequence, which stays in the input.
        let mut name = Vec::new();
        loop {
            match self.next_raw() {
                Some(Token::Char(b, cat)) if cat != Cat::Space => name.push(b),
                Some(t @ Token::Cs(_)) => {
                    self.push_front(vec![t]);
                    break;
                }
                _ => break,
            }
        }
        let name = String::from_utf8_lossy(&name).into_owned();
        let stem = name.rsplit('/').next().unwrap_or(&name).to_string();
        let stem = stem.split('.').next().unwrap_or("").to_string();
        if matches!(stem.as_str(), "docstrip" | "l3docstrip") {
            self.docstrip_loaded = true;
            return;
        }
        if self.docstrip_loaded {
            return self.diag(format!("\\input {name} is ignored (docstrip only honours \\input docstrip; nested batch files use \\batchinput)"));
        }
        let Some(bytes) = self.read_source(&name) else {
            return self.diag(format!("\\input: cannot find file {name}"));
        };
        self.push_input(name, &bytes);
    }

    /// Reads `bytes` as a nested file to its end. A file that is already
    /// being read (a `.dtx` that `\input`s itself and relies on a
    /// conditional this interpreter cannot evaluate to stop) and nesting
    /// deeper than TeX's own input stack are refused.
    fn push_input(&mut self, name: String, bytes: &[u8]) {
        if self.inputs.iter().any(|i| i.name == name) {
            return self.diag(format!("{name} is already being read; not reading it again (the file expects TeX to stop before this point)"));
        }
        if self.inputs.len() >= 15 {
            return self.diag(format!("\\input {name}: files nested too deeply"));
        }
        self.inputs.push(Input { name, lexer: Lexer::new(bytes), pending: VecDeque::new() });
        self.sync_catcodes();
        self.execute();
        self.inputs.pop();
        self.sync_catcodes();
    }

    /// `\endinput`: the rest of the current file is not read.
    fn end_current_input(&mut self) {
        if let Some(i) = self.inputs.last_mut() {
            i.pending.clear();
            i.lexer.finish();
        }
    }

    fn cmd_batchinput(&mut self) {
        let Some(name) = self.read_text_arg("\\batchinput") else { return };
        let name = String::from_utf8_lossy(&name).into_owned();
        let Some(bytes) = self.read_source(&name) else {
            return self.diag(format!("\\batchinput: cannot find batch file {name}"));
        };
        // A nested batch file runs in a group with the original defaults
        // (docstrip.dtx lines 1323–1331).
        self.push_group();
        self.locals.current_preamble = b"org@preamble".to_vec();
        self.locals.current_postamble = b"org@postamble".to_vec();
        self.locals.dest_dir = None;
        self.push_input(name, &bytes);
        self.pop_group();
    }

    /// `\long`/`\outer`/`\protected`/`\global` prefixes, then
    /// `\def`/`\gdef`/`\edef`/`\xdef`/`\let`.
    fn cmd_definition(&mut self) {
        let mut global = false;
        loop {
            let Some(t) = self.next_raw() else { return };
            match &t {
                Token::Cs(n) if matches!(n.as_slice(), b"long" | b"outer" | b"protected") => {}
                Token::Cs(n) if n == b"global" => global = true,
                Token::Cs(n) if matches!(n.as_slice(), b"def" | b"edef") => return self.cmd_def(global, n == b"edef"),
                Token::Cs(n) if matches!(n.as_slice(), b"gdef" | b"xdef") => return self.cmd_def(true, n == b"xdef"),
                Token::Cs(n) if n == b"let" => return self.cmd_let(global),
                Token::Char(_, Cat::Space) => {}
                _ => {
                    self.push_front(vec![t]);
                    self.diag("a prefix (\\global, \\long, …) must be followed by \\def or \\let");
                    return;
                }
            }
        }
    }

    fn cmd_def(&mut self, global: bool, expand: bool) {
        let name = match self.next_raw() {
            Some(Token::Cs(n)) if n != END => n,
            Some(Token::Char(b, Cat::Active)) => active_key(b),
            Some(t) => {
                self.push_front(vec![t]);
                self.diag("\\def: expected a control sequence");
                return;
            }
            None => return self.diag("\\def: expected a control sequence but the input ended"),
        };
        // Parameter text up to `{`.
        let mut prefix = Vec::new();
        let mut delimiters: Vec<Vec<Token>> = Vec::new();
        loop {
            let Some(t) = self.next_raw() else { return self.diag("\\def: the input ended in the parameter text") };
            match t {
                Token::Char(_, Cat::BeginGroup) => break,
                Token::Char(_, Cat::Param) => match self.next_raw() {
                    Some(Token::Char(d, _)) if d.is_ascii_digit() && d != b'0' => {
                        if (d - b'0') as usize != delimiters.len() + 1 {
                            self.diag(format!("\\def \\{}: parameters must be numbered consecutively", show(&name)));
                        }
                        delimiters.push(Vec::new());
                    }
                    Some(Token::Char(_, Cat::BeginGroup)) => {
                        self.diag(format!("\\def \\{}: `#{{` (a brace-delimited last parameter) is not supported", show(&name)));
                        break;
                    }
                    Some(t) => {
                        self.diag(format!("\\def \\{}: `#` in the parameter text must be followed by a digit", show(&name)));
                        self.push_front(vec![t]);
                    }
                    None => return,
                },
                Token::Cs(ref n) if n == END => {
                    self.push_front(vec![t]);
                    self.diag(format!("\\def \\{}: no replacement text", show(&name)));
                    return;
                }
                t => match delimiters.last_mut() {
                    Some(d) => d.push(t),
                    None => prefix.push(t),
                },
            }
        }
        let Some(raw) = self.read_group_body() else { return self.diag(format!("\\def \\{}: replacement text not closed", show(&name))) };
        // `##` is `#`; `#n` is parameter n.
        let mut body = Vec::with_capacity(raw.len());
        let mut it = raw.into_iter().peekable();
        while let Some(t) = it.next() {
            if t.cat() == Some(Cat::Param) {
                match it.peek() {
                    Some(Token::Char(_, Cat::Param)) => {
                        it.next();
                        body.push(t);
                    }
                    Some(Token::Char(d, _)) if d.is_ascii_digit() && *d != b'0' && ((*d - b'0') as usize) <= delimiters.len() => {
                        let n = *d - b'0';
                        it.next();
                        body.push(Token::Arg(n));
                    }
                    _ => {
                        self.diag(format!("\\def \\{}: illegal parameter number in the replacement text", show(&name)));
                        body.push(t);
                    }
                }
            } else {
                body.push(t);
            }
        }
        let body = if expand { self.expand_tokens(body) } else { body };
        if is_internal(&name) {
            self.diag(format!("\\{} is docstrip's own; the redefinition is ignored, files are written with docstrip's behaviour", show(&name)));
            return;
        }
        self.define(&name, Meaning::Macro(Rc::new(Macro { prefix, delimiters, body })), global);
    }

    fn cmd_let(&mut self, global: bool) {
        let name = match self.next_raw() {
            Some(Token::Cs(n)) if n != END => n,
            Some(Token::Char(b, Cat::Active)) => active_key(b),
            Some(t) => {
                self.push_front(vec![t]);
                self.diag("\\let: expected a control sequence");
                return;
            }
            None => return self.diag("\\let: expected a control sequence but the input ended"),
        };
        // `\let\a=\b`, `\let\a \b`, `\let\a= \b` (The TeXbook p. 206).
        let mut t = self.next_raw();
        if matches!(&t, Some(x) if x.is_space()) {
            t = self.next_raw();
        }
        if matches!(&t, Some(x) if x.is_char(b'=')) {
            t = self.next_raw();
            if matches!(&t, Some(x) if x.is_space()) {
                t = self.next_raw();
            }
        }
        let meaning = match t {
            Some(Token::Cs(n)) if n == END => {
                self.push_front(vec![Token::Cs(n)]);
                return self.diag("\\let: missing the second control sequence");
            }
            Some(Token::Cs(n)) => self.meaning(&n),
            Some(t @ Token::Char(..)) => Meaning::Char(t),
            Some(Token::Arg(_)) | None => return self.diag("\\let: missing the second control sequence"),
        };
        if is_internal(&name) {
            self.diag(format!("\\{} is docstrip's own; the \\let is ignored", show(&name)));
            return;
        }
        self.define(&name, meaning, global);
    }

    /// Skips a conditional's text to its `\fi`, or to an `\else` (when
    /// `stop_at_else`) or `\or` (when `stop_at_or`) at the same level,
    /// counting nested conditionals by name (`\if…`, but not docstrip's
    /// `\ifToplevel`, which is an ordinary macro). Answers what stopped it.
    fn skip_conditional(&mut self, stop_at_else: bool, stop_at_or: bool) -> Stopped {
        let mut depth = 0usize;
        loop {
            let Some(t) = self.next_raw() else {
                self.diag("conditional not closed by \\fi before the end of the input");
                return Stopped::End;
            };
            let Token::Cs(n) = &t else { continue };
            if n == END {
                self.push_front(vec![t]);
                self.diag("conditional not closed by \\fi");
                return Stopped::End;
            }
            if n == b"fi" {
                if depth == 0 {
                    return Stopped::Fi;
                }
                depth -= 1;
            } else if n == b"else" && depth == 0 && stop_at_else {
                return Stopped::Else;
            } else if n == b"or" && depth == 0 && stop_at_or {
                return Stopped::Or;
            } else if n.starts_with(b"if") && n != b"ifToplevel" {
                depth += 1;
            }
        }
    }

    /// A conditional that came out false: skip to `\else` (its text then
    /// runs) or `\fi`.
    fn take_false_branch(&mut self) {
        let _ = self.skip_conditional(true, false);
    }

    /// `\ifcase <number> … \or … \else … \fi` (The TeXbook p. 210).
    fn cmd_ifcase(&mut self) {
        let Some(n) = self.read_number() else {
            self.diag("\\ifcase: expected a number; the first case is taken");
            return;
        };
        for _ in 0..n.max(0) {
            match self.skip_conditional(true, true) {
                Stopped::Or => {}
                Stopped::Else | Stopped::Fi | Stopped::End => return,
            }
        }
    }

    /// `\ifnum <number> <relation> <number>`: `<`, `=` or `>`.
    fn cmd_ifnum(&mut self) {
        let (Some(a), Some(rel), Some(b)) = (self.read_number(), self.next_expanded(), self.read_number()) else {
            self.diag("\\ifnum: expected <number> <relation> <number>; taken as true");
            return;
        };
        let holds = match rel {
            Token::Char(b'<', _) => a < b,
            Token::Char(b'=', _) => a == b,
            Token::Char(b'>', _) => a > b,
            _ => {
                self.diag("\\ifnum: expected <, = or >; taken as true");
                return;
            }
        };
        if !holds {
            self.take_false_branch();
        }
    }

    /// `\ifx <token> <token>`: equal when both are the same character (code
    /// and category), or control sequences with the same meaning.
    fn cmd_ifx(&mut self) {
        let (Some(a), Some(b)) = (self.next_raw(), self.next_raw()) else { return };
        let meaning_of = |me: &Self, t: &Token| -> Meaning {
            match t {
                Token::Cs(n) => me.meaning(n),
                Token::Char(c, Cat::Active) => me.meaning(&active_key(*c)),
                other => Meaning::Char(other.clone()),
            }
        };
        let same = match (meaning_of(self, &a), meaning_of(self, &b)) {
            (Meaning::Macro(x), Meaning::Macro(y)) => x.prefix == y.prefix && x.delimiters == y.delimiters && x.body == y.body,
            (Meaning::Relax, Meaning::Relax) | (Meaning::Undefined, Meaning::Undefined) => true,
            (Meaning::Char(x), Meaning::Char(y)) => x == y,
            (Meaning::Primitive, Meaning::Primitive) => a == b,
            _ => false,
        };
        if !same {
            self.take_false_branch();
        }
    }

    /// `\if <token> <token>` after expansion: equal character codes
    /// (control sequences all count as one code, as in TeX).
    fn cmd_if(&mut self) {
        let (Some(a), Some(b)) = (self.next_expanded(), self.next_expanded()) else { return };
        let code = |t: &Token| match t {
            Token::Char(b, _) => *b as i32,
            _ => 256,
        };
        if code(&a) != code(&b) {
            self.take_false_branch();
        }
    }

    /// A TeX <number> from expanded tokens: optional signs, then digits
    /// or `` `<char> ``; the one optional space after it is consumed.
    fn read_number(&mut self) -> Option<i64> {
        let mut t = self.next_expanded()?;
        let mut sign = 1i64;
        loop {
            match &t {
                Token::Char(_, Cat::Space) => {}
                Token::Char(b'-', _) => sign = -sign,
                Token::Char(b'+', _) => {}
                _ => break,
            }
            t = self.next_expanded()?;
        }
        let value = if t.is_cs("active") {
            13
        } else if t.is_char(b'`') {
            match self.next_raw()? {
                Token::Cs(n) if n.len() == 1 => n[0] as i64,
                Token::Char(b, _) => b as i64,
                other => {
                    self.push_front(vec![other]);
                    return None;
                }
            }
        } else {
            let mut v: i64 = 0;
            let mut digits = 0;
            loop {
                match &t {
                    Token::Char(d, _) if d.is_ascii_digit() => {
                        v = v.saturating_mul(10).saturating_add((d - b'0') as i64);
                        digits += 1;
                    }
                    _ => {
                        self.push_front(vec![t]);
                        break;
                    }
                }
                let Some(next) = self.next_expanded() else { break };
                t = next;
            }
            if digits == 0 {
                return None;
            }
            v
        };
        // One space ends the number.
        if let Some(next) = self.next_expanded() {
            if !next.is_space() {
                self.push_front(vec![next]);
            }
        }
        Some(sign * value)
    }

    /// `\catcode`\^^M=13 ` and friends: skips `` `<token> `` or a number,
    /// an optional `=` and the digits.
    fn skip_assignment(&mut self) {
        let mut seen_digit = false;
        loop {
            let Some(t) = self.next_raw() else { return };
            match &t {
                Token::Char(b'`', _) => {
                    let _ = self.next_raw();
                }
                Token::Char(b'=', _) | Token::Char(_, Cat::Space) if !seen_digit => {}
                Token::Char(b, _) if b.is_ascii_digit() => seen_digit = true,
                Token::Cs(n) if n == b"relax" => return,
                _ => {
                    self.push_front(vec![t]);
                    return;
                }
            }
        }
    }

    // ----- pre- and postambles --------------------------------------------

    /// `\declarepreamble\name … \endpreamble` (docstrip.dtx lines
    /// 3527–3538) and `\declarepostamble` (3551–3561). As in docstrip,
    /// `\declarepreamble` opens a group in which the end of a line is
    /// active and a space is other, then calls `\declarepreambleX`. A
    /// batch file may have redefined that (microtype does, to splice
    /// `\firstpreamblepart`/`\lastpreamblepart` around the text), in which
    /// case its macro runs; otherwise the built-in one does what
    /// docstrip's does: reads `#2` up to the line end + `\endpreamble`
    /// (the `^^M` is part of the delimiter, line 3531), closes the group,
    /// defines the active `^^M` as `^^J\MetaPrefix\space` and `\edef`s
    /// `\name` as heading + `\ReferenceLines` + `\MetaPrefix\space
    /// \checkeoln #2 \empty` (a postamble: `\MetaPrefix\space\checkeoln
    /// #2 \empty ^^J \MetaPrefix ^^J \MetaPrefix\space End of file
    /// `\outFileName'.`).
    fn declare_amble(&mut self, name: &[u8], preamble: bool) {
        self.push_group();
        self.set_catcode(13, lexer::ACTIVE);
        self.set_catcode(b' ', lexer::OTHER);
        let x: &[u8] = if preamble { b"declarepreambleX" } else { b"declarepostambleX" };
        if let Meaning::Macro(_) = self.meaning(x) {
            // The batch file's own; its `\endgroup` closes the group.
            self.push_front(vec![Token::Cs(x.to_vec()), Token::Cs(name.to_vec())]);
            return;
        }
        let end = if preamble { "endpreamble" } else { "endpostamble" };
        let text = self.read_delimited(&[Token::Char(13, Cat::Active), Token::cs(end)]);
        self.pop_group();
        let Some(text) = text else {
            return self.diag(format!("\\{} not closed by a line starting with \\{end} before the end of the input", if preamble { "declarepreamble" } else { "declarepostamble" }));
        };
        let eol_body = vec![Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::cs("space")];
        self.define(&active_key(13), Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body: eol_body })), false);
        let mut body = Vec::new();
        if preamble {
            body.extend(self.heading());
            body.push(Token::cs("ReferenceLines"));
        }
        body.extend([Token::cs("MetaPrefix"), Token::cs("space"), Token::cs("checkeoln")]);
        body.extend(text);
        body.push(Token::cs("empty"));
        if !preamble {
            body.extend([Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::cs("space")]);
            body.extend(chars("End of file `"));
            body.push(Token::cs("outFileName"));
            body.extend(chars("'."));
        }
        let body = self.expand_tokens(body);
        self.define(name, Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body })), false);
    }

    /// `\catcode`\c=n`: the one code assignment that matters to reading
    /// a batch file. The forms are `` `\c `` or `` `c ``, an optional `=`,
    /// digits; the space that ends the number is consumed under the old
    /// codes, as TeX consumes it before the assignment takes effect.
    fn cmd_catcode(&mut self) {
        let mut next = || self.next_raw();
        let _ = &mut next;
        let mut t = self.next_raw();
        while matches!(&t, Some(x) if x.is_space()) {
            t = self.next_raw();
        }
        let Some(tick) = t else { return };
        if !tick.is_char(b'`') {
            self.push_front(vec![tick]);
            self.skip_assignment();
            return self.note_once(b"catcode", "\\catcode with a numeric character code is not understood; ignored");
        }
        let byte = match self.next_raw() {
            Some(Token::Cs(n)) if n.len() == 1 => n[0],
            Some(Token::Char(b, _)) => b,
            Some(t) => {
                self.push_front(vec![t]);
                return self.note_once(b"catcode", "\\catcode`: expected a character; ignored");
            }
            None => return,
        };
        let mut t = self.next_raw();
        while matches!(&t, Some(x) if x.is_space()) {
            t = self.next_raw();
        }
        if matches!(&t, Some(x) if x.is_char(b'=')) {
            t = self.next_raw();
            while matches!(&t, Some(x) if x.is_space()) {
                t = self.next_raw();
            }
        }
        let mut code: u32 = 0;
        let mut digits = 0;
        // plain.tex's `\chardef\active=13`.
        if matches!(&t, Some(x) if x.is_cs("active")) {
            code = 13;
            digits = 1;
            t = self.next_raw();
        }
        while let Some(Token::Char(d, _)) = &t {
            if !d.is_ascii_digit() {
                break;
            }
            code = code * 10 + (d - b'0') as u32;
            digits += 1;
            t = self.next_raw();
        }
        // The token after the digits: a space ends the number and is
        // consumed; anything else is put back.
        if let Some(t) = t {
            if !t.is_space() {
                self.push_front(vec![t]);
            }
        }
        if digits == 0 || code > 15 {
            return self.note_once(b"catcode", "\\catcode: expected a category code 0–15; ignored");
        }
        self.set_catcode(byte, code as u8);
    }

    /// `\ds@heading` (docstrip.dtx line 3478): a macro, so a batch file
    /// may use it in a preamble of its own (ProjLib.ins does).
    fn heading(&mut self) -> Vec<Token> {
        vec![Token::cs("ds@heading")]
    }

    /// `\AddGenerationDate` (docstrip.dtx line 3496) redefines
    /// `\ds@heading` with the date and docstrip's version.
    fn cmd_add_generation_date(&mut self) {
        let (y, m, d) = match self.options.today {
            Some(d) => d,
            None => {
                self.diag("\\AddGenerationDate needs today's date, which was not supplied; writing <0/0/0>");
                (0, 0, 0)
            }
        };
        let mut h = vec![Token::cs("MetaPrefix"), Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::cs("space")];
        h.extend(chars("This is file `"));
        h.push(Token::cs("outFileName"));
        h.extend(chars(&format!("', generated on <{y}/{m}/{d}> ")));
        h.extend([Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::cs("space")]);
        h.extend(chars(&format!("with the docstrip utility ({DOCSTRIP_VERSION}).")));
        h.push(Token::Char(10, Cat::Other));
        self.define(b"ds@heading", Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body: h })), false);
    }

    // ----- \generate --------------------------------------------------------

    fn cmd_generate(&mut self) {
        let Some(body) = self.read_group("\\generate") else { return };
        self.push_group();
        let outer = self.generate.replace(Generate::default());
        self.execute_tokens(body);
        let generate = self.generate.take().unwrap_or_default();
        self.generate = outer;
        self.run_generate(generate);
        self.pop_group();
    }

    fn cmd_file(&mut self) {
        if self.generate.is_none() {
            self.diag("Command `\\file' only allowed in argument to `\\generate'");
            let _ = self.read_undelimited();
            let _ = self.read_undelimited();
            return;
        }
        let Some(name) = self.read_text_arg("\\file") else { return };
        let name = String::from_utf8_lossy(&name).into_owned();
        let Some(body) = self.read_undelimited() else { return self.diag("\\file: missing the \\from list") };
        // \curref (docstrip.dtx line 3040)
        let mut reference = vec![Token::cs("MetaPrefix"), Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::cs("space")];
        reference.extend(chars("The original source files were:"));
        reference.extend([Token::Char(10, Cat::Other), Token::cs("MetaPrefix"), Token::Char(10, Cat::Other)]);
        let outer = self.file.replace(FileCtx { name, passes: Vec::new(), curinfiles: Vec::new(), names: Vec::new(), reference });
        self.execute_tokens(body);
        let fc = self.file.take();
        self.file = outer;
        let Some(fc) = fc else { return };
        if fc.passes.is_empty() {
            self.diag(format!("\\file{{{}}} names no \\from source; nothing generated", fc.name));
            return;
        }
        let reference = self.expand_tokens(fc.reference);
        let amble = |me: &mut Self, which: &[u8], label: &str| -> Vec<Token> {
            match me.meaning(which) {
                Meaning::Macro(m) => m.body.clone(),
                _ => {
                    me.diag(format!("{label} \\{} is not declared; none written", show(which)));
                    Vec::new()
                }
            }
        };
        let (pre_name, post_name) = (self.locals.current_preamble.clone(), self.locals.current_postamble.clone());
        let preamble = amble(self, &pre_name, "preamble");
        let postamble = amble(self, &post_name, "postamble");
        let spec = FileSpec { name: fc.name, passes: fc.passes, names: fc.names, preamble, postamble, reference, dir: self.locals.dest_dir.clone() };
        if let Some(g) = self.generate.as_mut() {
            g.files.push(spec);
        }
    }

    fn cmd_from(&mut self) {
        if self.file.is_none() {
            self.diag("Command `\\from' only allowed in argument to `\\file'");
            let _ = self.read_undelimited();
            let _ = self.read_undelimited();
            return;
        }
        let Some(source) = self.read_text_arg("\\from") else { return };
        let Some(options) = self.read_text_arg("\\from") else { return };
        let source = String::from_utf8_lossy(&source).into_owned();
        // The reference line (docstrip.dtx line 3179): `\MetaPrefix\space
        // #1 ` then ` (with options: `#2')` unless the options are empty.
        let mut line = vec![Token::cs("MetaPrefix"), Token::cs("space")];
        line.extend(chars(&source));
        line.push(Token::Char(b' ', Cat::Space));
        if !options.is_empty() {
            line.push(Token::cs("space"));
            line.extend(chars("(with options: `"));
            line.extend(options.iter().map(|&b| Token::Char(b, Cat::Other)));
            line.extend(chars("')"));
        }
        line.push(Token::Char(10, Cat::Other));
        let line = self.expand_tokens(line);
        let key = self.needed(&source);
        if let Some(fc) = self.file.as_mut() {
            fc.reference.extend(line);
            fc.names.push(source);
            fc.passes.push((key, options));
        }
    }

    /// `\@need@d` (docstrip.dtx line 3121): the source's name, made
    /// unique within the current `\file` by trailing spaces, and added
    /// to the `\generate`'s reading order on first sight.
    fn needed(&mut self, source: &str) -> String {
        let mut key = source.to_string();
        if let Some(fc) = self.file.as_mut() {
            while fc.curinfiles.contains(&key) {
                key.push(' ');
            }
            fc.curinfiles.push(key.clone());
        }
        if let Some(g) = self.generate.as_mut() {
            if !g.inputs.contains(&key) {
                g.inputs.push(key.clone());
            }
        }
        key
    }

    /// Reads every source once in order and writes the files
    /// (`\processinputfiles`/`\readsource`, docstrip.dtx lines 2937 and
    /// 3211): each output opens (preamble) at its first source, receives
    /// the lines its options select from every source it names, and
    /// closes (postamble) after its last.
    fn run_generate(&mut self, g: Generate) {
        if g.files.is_empty() {
            return;
        }
        self.out.messages.push(format!("Generating file(s) {}", g.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(" ")));
        // \checkorder (line 3082): every file's own order must agree with
        // the global one; docstrip stops on a conflict, this run still
        // writes each file in its own order.
        for f in &g.files {
            let mut at = 0;
            let mut ok = true;
            for (key, _) in &f.passes {
                match g.inputs[at..].iter().position(|k| k == key) {
                    Some(p) => at += p + 1,
                    None => ok = false,
                }
            }
            if !ok {
                self.diag(format!("DOCSTRIP error: Incompatible order of input files specified for file `{}' (docstrip would stop here; the file is written in the order the sources are read)", f.name));
            }
        }
        let meta_prefix = {
            let t = self.expand_tokens(vec![Token::cs("MetaPrefix")]);
            self.write(&t)
        };
        let mut outputs: Vec<Vec<u8>> = vec![Vec::new(); g.files.len()];
        let mut state = strip::State::default();
        for key in &g.inputs {
            let source = key.trim_end_matches(' ');
            let consumers: Vec<strip::Consumer> = g.files.iter().enumerate().filter_map(|(i, f)| f.passes.iter().find(|(k, _)| k == key).map(|(_, o)| strip::Consumer { output: i, options: o.clone() })).collect();
            // docstrip reports a missing source and reads nothing from it;
            // here the file is still opened and closed around it, so its
            // preamble and postamble are written and the rest is usable.
            let bytes = self.read_source(source).unwrap_or_else(|| {
                self.out.diagnostics.push(Diagnostic { file: source.to_string(), line: 0, message: format!("Cannot find file {source}") });
                Vec::new()
            });
            for c in &consumers {
                let f = &g.files[c.output];
                self.out.messages.push(format!("Processing file {source}{} -> {}", if c.options.is_empty() { String::new() } else { format!(" ({})", String::from_utf8_lossy(&c.options)) }, f.name));
                if f.passes.first().is_some_and(|(k, _)| k == key) && !f.preamble.is_empty() {
                    let text = self.render_amble(f, &f.preamble);
                    outputs[c.output].extend_from_slice(&text);
                    outputs[c.output].push(b'\n');
                }
            }
            strip::process_source(source, &bytes, &consumers, &mut outputs, &meta_prefix, &mut state, &mut self.out.diagnostics, &mut self.out.messages);
            for c in &consumers {
                let f = &g.files[c.output];
                if f.passes.last().is_some_and(|(k, _)| k == key) && !f.postamble.is_empty() {
                    let text = self.render_amble(f, &f.postamble);
                    outputs[c.output].extend_from_slice(&text);
                    outputs[c.output].push(b'\n');
                }
            }
        }
        for (f, bytes) in g.files.into_iter().zip(outputs) {
            let mut sources: Vec<String> = Vec::new();
            for n in &f.names {
                if !sources.contains(n) {
                    sources.push(n.clone());
                }
            }
            let generated = GeneratedFile { name: f.name, bytes, sources, dir: f.dir };
            self.out.files.retain(|g| g.name != generated.name);
            self.out.files.push(generated);
        }
    }

    /// `\WritePreamble`/`\WritePostamble` (docstrip.dtx lines 3697 and
    /// 3720): the amble expanded with `\outFileName`, `\inFileName` and
    /// `\ReferenceLines` defined for this file.
    fn render_amble(&mut self, f: &FileSpec, amble: &[Token]) -> Vec<u8> {
        self.push_group();
        let simple = |body: Vec<Token>| Meaning::Macro(Rc::new(Macro { prefix: vec![], delimiters: vec![], body }));
        self.locals.macros.insert(b"outFileName".to_vec(), simple(chars(&f.name)));
        self.locals.macros.insert(b"inFileName".to_vec(), simple(chars(&f.names.join(" "))));
        self.locals.macros.insert(b"ReferenceLines".to_vec(), simple(f.reference.clone()));
        let expanded = self.expand_tokens(amble.to_vec());
        self.pop_group();
        self.write(&expanded)
    }
}

impl Token {
    fn is_cs_bytes(&self, name: &[u8]) -> bool {
        matches!(self, Token::Cs(n) if n == name)
    }
}

fn chars(s: &str) -> Vec<Token> {
    s.bytes().map(|b| Token::Char(b, Cat::Other)).collect()
}

/// What ended a skip over conditional text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stopped {
    Else,
    Or,
    Fi,
    End,
}

/// The macro-table key of an active character (no control sequence can
/// have this name: 0xff is never a letter).
fn active_key(byte: u8) -> Vec<u8> {
    vec![0xff, byte]
}

fn show(name: &[u8]) -> String {
    if name.first() == Some(&0xff) {
        let b = name.get(1).copied().unwrap_or(0);
        return if b < 32 { format!("^^{}", (b ^ 0x40) as char) } else { format!("active {:?}", b as char) };
    }
    String::from_utf8_lossy(name).into_owned()
}

/// docstrip internals a batch file cannot usefully redefine here: their
/// behaviour is built in, so a `\def` of one is reported rather than
/// silently accepted.
fn is_internal(name: &[u8]) -> bool {
    matches!(
        name,
        b"inFileName" | b"outFileName" | b"ReferenceLines" | b"processLine" | b"readsource" | b"makepathname" | b"WritePreamble" | b"WritePostamble" | b"StreamPut"
    )
}

/// docstrip.tex's default pre- and postambles (docstrip.dtx lines
/// 3596–3663), declared through the same code path as a batch file's.
const DEFAULTS: &str = "\\def\\ds@heading{%
  \\MetaPrefix ^^J%
  \\MetaPrefix\\space This is file `\\outFileName',^^J%
  \\MetaPrefix\\space  generated with the docstrip utility.^^J%
  }
\\declarepreamble\\org@preamble

IMPORTANT NOTICE:

For the copyright see the source file.

Any modified versions of this file must be renamed
with new filenames distinct from \\outFileName.

For distribution of the original source see the terms
for copying and modification in the file \\inFileName.

This generated file may be distributed as long as the
original source files, as listed above, are part of the
same distribution. (The sources need not necessarily be
in the same archive or directory.)
\\endpreamble
\\edef\\org@postamble{\\string\\endinput^^J%
  \\MetaPrefix ^^J%
  \\MetaPrefix\\space End of file `\\outFileName'.%
  }
\\let\\defaultpreamble\\org@preamble
\\let\\defaultpostamble\\org@postamble
\\declarepreamble\\originaldefault

IMPORTANT NOTICE:

For the copyright see the source file.

You are *not* allowed to modify this file.

You are *not* allowed to distribute this file.
For distribution of the original source see the terms
for copying and modification in the file \\inFileName.

\\endpreamble
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sources(files: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
        files.iter().map(|(n, t)| (n.to_string(), t.as_bytes().to_vec())).collect()
    }

    fn run(ins: &str, files: &[(&str, &str)]) -> Outcome {
        crate::run("t.ins", ins.as_bytes(), &sources(files))
    }

    fn text(o: &Outcome, name: &str) -> String {
        String::from_utf8(o.file(name).unwrap_or_else(|| panic!("{name} not generated: {o:?}")).bytes.clone()).unwrap()
    }

    const DTX: &str = "% \\iffalse\n%<*driver>\n\\documentclass{ltxdoc}\n%</driver>\n% \\fi\n%    \\begin{macrocode}\n%<*package>\n\\ProvidesPackage{t}\n%<*extra>\n\\def\\extra{1}\n%</extra>\n%<-extra>\\def\\extra{0}\n%</package>\n%    \\end{macrocode}\n%%\n%% meta\n";

    #[test]
    fn the_default_preamble_and_postamble_are_docstrips() {
        let o = run("\\input docstrip.tex\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n\\endbatchfile\n", &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(
            text(&o, "t.sty"),
            "%%\n%% This is file `t.sty',\n%% generated with the docstrip utility.\n%%\n%% The original source files were:\n%%\n%% t.dtx  (with options: `package')\n%% \n%% IMPORTANT NOTICE:\n%% \n%% For the copyright see the source file.\n%% \n%% Any modified versions of this file must be renamed\n%% with new filenames distinct from t.sty.\n%% \n%% For distribution of the original source see the terms\n%% for copying and modification in the file t.dtx.\n%% \n%% This generated file may be distributed as long as the\n%% original source files, as listed above, are part of the\n%% same distribution. (The sources need not necessarily be\n%% in the same archive or directory.)\n\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n\\endinput\n%%\n%% End of file `t.sty'.\n"
        );
        assert_eq!(o.files[0].sources, ["t.dtx"]);
        assert!(o.completed);
        assert_eq!(o.messages[0], "Generating file(s) t.sty");
    }

    #[test]
    fn preamble_postamble_and_options() {
        // A `%` comment swallows the rest of its line *and* the line end
        // (so a comment-only line vanishes, and a trailing comment joins
        // the next line), as `tex` does.
        let ins = "\\input docstrip\n\\keepsilent\n\\askforoverwritefalse\n\\preamble\n\nCopyright  (c) me\n% a comment line\n  indented % joins\nnext\n\\endpreamble\n\\postamble\nbye\n\\endpostamble\n\\generate{\\file{t.sty}{\\from{t.dtx}{package,extra}}}\n\\endbatchfile\nignored";
        let o = run(ins, &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(
            text(&o, "t.sty"),
            "%%\n%% This is file `t.sty',\n%% generated with the docstrip utility.\n%%\n%% The original source files were:\n%%\n%% t.dtx  (with options: `package,extra')\n%% \n%% Copyright  (c) me\n%%   indented next\n\\ProvidesPackage{t}\n\\def\\extra{1}\n%%\n%% meta\n%% bye\n%%\n%% End of file `t.sty'.\n"
        );
    }

    #[test]
    fn empty_ambles_declared_and_no_ambles() {
        let o = run("\\input docstrip\n\\preamble\n\\endpreamble\n\\postamble\n\\endpostamble\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n", &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(text(&o, "t.sty"), "%%\n%% This is file `t.sty',\n%% generated with the docstrip utility.\n%%\n%% The original source files were:\n%%\n%% t.dtx  (with options: `package')\n%% \n\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n%% \n%%\n%% End of file `t.sty'.\n");
        let o = run("\\input docstrip\n\\generate{\\nopreamble\\nopostamble\\file{t.sty}{\\from{t.dtx}{package}}}\n", &[("t.dtx", DTX)]);
        assert_eq!(text(&o, "t.sty"), "\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n");
        let o = run("\\input docstrip\n\\generate{\\usepreamble\\empty\\usepostamble\\empty\\file{t.sty}{\\from{t.dtx}{package}}}\n", &[("t.dtx", DTX)]);
        assert_eq!(text(&o, "t.sty"), "\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n");
    }

    #[test]
    fn declared_ambles_are_selected_per_file_and_the_generate_group_is_local() {
        let ins = "\\input docstrip\n\\declarepreamble\\cfgpre\nA config file.\n\\endpreamble\n\\declarepostamble\\nothing\n\\endpostamble\n\\preamble\nmain\n\\endpreamble\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}\n  \\usepreamble\\cfgpre \\usepostamble\\nothing\n  \\file{t.cfg}{\\from{t.dtx}{driver}}}\n\\generate{\\file{u.sty}{\\from{t.dtx}{package}}}\n";
        let o = run(ins, &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert!(text(&o, "t.sty").contains("%% main\n\\ProvidesPackage"));
        assert!(text(&o, "t.sty").ends_with("\\endinput\n%%\n%% End of file `t.sty'.\n"));
        let cfg = text(&o, "t.cfg");
        assert!(cfg.contains("%% t.dtx  (with options: `driver')\n%% A config file.\n\\documentclass{ltxdoc}\n%%\n%% meta\n%% \n%%\n%% End of file `t.cfg'.\n"), "{cfg}");
        assert!(!cfg.contains("\\endinput"));
        assert!(text(&o, "u.sty").contains("%% main\n"), "\\usepreamble inside \\generate is undone by its group");
    }

    #[test]
    fn multiple_files_sources_and_repeated_reads() {
        let a = "%<*x>\na-x\n%</x>\n%<*y>\na-y\n%</y>\n";
        let b = "%<*x>\nb-x\n%</x>\n%<*y>\nb-y\n%</y>\n";
        let ins = "\\input docstrip\n\\nopreamble\\nopostamble\n\\generate{\\file{p1}{\\from{a.dtx}{x}\\from{b.dtx}{x}\\from{a.dtx}{y}}\n\\file{p2}{\\from{b.dtx}{y}}\n\\file{p3}{\\from{a.dtx}{x,y}\\from{b.dtx}{x}}}\n";
        let o = run(ins, &[("a.dtx", a), ("b.dtx", b)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(text(&o, "p1"), "a-x\nb-x\na-y\n");
        assert_eq!(text(&o, "p2"), "b-y\n");
        assert_eq!(text(&o, "p3"), "a-x\na-y\nb-x\n");
        assert_eq!(o.file("p1").unwrap().sources, ["a.dtx", "b.dtx"]);
        // The reference lines list every \from, repeats included, in order.
        let o = run("\\input docstrip\n\\preamble\n\\endpreamble\n\\generate{\\file{p1}{\\from{a.dtx}{x}\\from{b.dtx}{}\\from{a.dtx}{y}}}\n", &[("a.dtx", a), ("b.dtx", b)]);
        assert!(text(&o, "p1").contains("%% a.dtx  (with options: `x')\n%% b.dtx \n%% a.dtx  (with options: `y')\n%% \n"), "{}", text(&o, "p1"));
        // A conflicting order is reported and still written per file.
        let o = run("\\input docstrip\n\\nopreamble\\nopostamble\n\\generate{\\file{p1}{\\from{a.dtx}{x}\\from{b.dtx}{x}}\\file{p2}{\\from{b.dtx}{y}\\from{a.dtx}{y}}}\n", &[("a.dtx", a), ("b.dtx", b)]);
        assert!(o.diagnostics.iter().any(|d| d.message.contains("Incompatible order")), "{:?}", o.diagnostics);
        assert_eq!(text(&o, "p2"), "a-y\nb-y\n", "written in reading order, as the diagnostic says");
    }

    #[test]
    fn user_macros_let_and_edef() {
        let ins = "\\input docstrip\n\\nopreamble\\nopostamble\n\\def\\makefile#1#2{\\file{#1}{\\from{t.dtx}{#2}}}\n\\let\\DEBUG\\empty\n\\def\\pkg{package}\n\\edef\\opts{\\pkg,extra}\n\\generate{\\makefile{a.sty}{package\\DEBUG}\\makefile{b.sty}{\\opts}\\file{c.sty}{\\from{t.dtx}{\\jobname}}}\n";
        let o = run(ins, &[("t.dtx", DTX.replace("package", "t").as_str())]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(text(&o, "a.sty"), "%%\n%% meta\n", "the guard is `t`, not `package`");
        assert_eq!(text(&o, "c.sty"), "\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n", "\\jobname is the batch file's stem");
        let o = run("\\input docstrip\n\\nopreamble\\nopostamble\n\\def\\opts{package,extra}\n\\generate{\\file{b.sty}{\\from{t.dtx}{\\opts}}}\n", &[("t.dtx", DTX)]);
        assert_eq!(text(&o, "b.sty"), "\\ProvidesPackage{t}\n\\def\\extra{1}\n%%\n%% meta\n");
    }

    #[test]
    fn meta_prefix_is_deferred_when_relaxed() {
        // microtype's idiom: \let\MetaPrefix\relax before \preamble, then set it before \generate.
        let ins = "\\input docstrip\n\\let\\MetaPrefix\\relax\n\\preamble\nhead\n\\endpreamble\n\\let\\MetaPrefix\\DoubleperCent\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n\\def\\MetaPrefix{--}\n\\generate{\\file{t.lua}{\\from{t.dtx}{package}}}\n";
        let o = run(ins, &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert!(text(&o, "t.sty").starts_with("%%\n%% This is file `t.sty',\n%% generated with the docstrip utility.\n%%\n%% The original source files were:\n%%\n%% t.dtx  (with options: `package')\n%% head\n\\ProvidesPackage"), "{}", text(&o, "t.sty"));
        assert_eq!(text(&o, "t.lua"), "--\n-- This is file `t.lua',\n-- generated with the docstrip utility.\n--\n-- The original source files were:\n--\n-- t.dtx  (with options: `package')\n-- head\n\\ProvidesPackage{t}\n\\def\\extra{0}\n--\n-- meta\n\\endinput\n%%\n%% End of file `t.lua'.\n", "the default postamble baked `%%` in when docstrip.tex loaded (checked against tex)");
    }

    #[test]
    fn a_batch_file_may_redefine_declarepreamblex() {
        // microtype.ins's construction: a group making the line end active,
        // a \gdef of docstrip's \declarepreambleX whose parameter text ends
        // in that active ^^M, \firstpreamblepart/\lastpreamblepart spliced
        // around the text, and \MetaPrefix deferred with \let…\relax.
        let ins = "\\input docstrip\n\\keepsilent\n\\let\\MetaPrefix\\relax\n\\preamble\n\nmain text\n\n\\endpreamble\n{\\catcode`\\^^M=13 %\n \\gdef\\declarepreambleX#1#2\n\\endpreamble{\\endgroup%\n  \\def^^M{^^J\\MetaPrefix\\space}%\n  \\edef#1{\\firstpreamblepart\\checkeoln#2\\lastpreamblepart\\empty}}}\n\\def\\defaultlastpreamblepart{^^J\\MetaPrefix\\space\n----\n^^J\\MetaPrefix}\n\\let\\firstpreamblepart\\defaultpreamble\n\\let\\lastpreamblepart \\defaultlastpreamblepart\n\\declarepreamble\\defpreamble\n  This file contains definitions.\n\\endpreamble\n\\let\\lastpreamblepart\\empty\n\\declarepreamble\\contribpreamble^^M\n    Prepared by:\n\\endpreamble\n\\let\\MetaPrefix\\DoubleperCent\n\\generate{\\usepreamble\\defpreamble\\nopostamble\\file{t.def}{\\from{t.dtx}{package}}\\usepreamble\\contribpreamble\\file{t.cfg}{\\from{t.dtx}{package}}}\n\\endbatchfile\n";
        let o = run(ins, &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(
            text(&o, "t.def"),
            "%%\n%% This is file `t.def',\n%% generated with the docstrip utility.\n%%\n%% The original source files were:\n%%\n%% t.dtx  (with options: `package')\n%% \n%% main text\n%%   This file contains definitions.\n%% ---- \n%%\n\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n"
        );
        // `\declarepreamble\contribpreamble^^M`: the doubled-caret ^^M is an
        // active line end too, so the text starts with an empty line.
        assert_eq!(
            text(&o, "t.cfg"),
            "%%\n%% This is file `t.cfg',\n%% generated with the docstrip utility.\n%%\n%% The original source files were:\n%%\n%% t.dtx  (with options: `package')\n%% \n%% main text\n%% \n%%     Prepared by:\n\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n"
        );
    }

    #[test]
    fn old_interface_generatefile_include_processfile() {
        let o = run("\\input docstrip\n\\nopreamble\\nopostamble\n\\generateFile{t.sty}{f}{\\from{t.dtx}{package}}\n\\include{package,extra}\n\\processFile{t}{dtx}{cfg}{t}\n", &[("t.dtx", DTX)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert_eq!(text(&o, "t.sty"), "\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n");
        assert_eq!(text(&o, "t.cfg"), "\\ProvidesPackage{t}\n\\def\\extra{1}\n%%\n%% meta\n");
    }

    #[test]
    fn messages_iftoplevel_iffalse_and_batchinput() {
        let ins = "\\iffalse meta-comment\n\\generate{\\file{no}{\\from{t.dtx}{package}}}\n\\fi\n\\input docstrip\n\\nopreamble\\nopostamble\n\\ifToplevel{\\Msg{top^^Jlevel}}\n\\batchinput{inner.ins}\n\\Msg{*\\space done}\n\\endbatchfile\n\\Msg{never}\n";
        let inner = "\\usedir{tex/latex/t}\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n\\ifToplevel{\\Msg{not shown}}\n\\endbatchfile\n\\Msg{nor this}\n";
        let o = run(ins, &[("t.dtx", DTX), ("inner.ins", inner)]);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert!(o.file("no").is_none());
        // The nested file used the original defaults, not \nopreamble.
        assert!(text(&o, "t.sty").starts_with("%%\n%% This is file `t.sty',"));
        assert_eq!(o.file("t.sty").unwrap().dir.as_deref(), Some("tex/latex/t"));
        assert_eq!(o.messages, ["top\nlevel", "Generating file(s) t.sty", "Processing file t.dtx (package) -> t.sty", "* done"]);
    }

    #[test]
    fn unknown_commands_are_reported_once_and_never_stop_the_run() {
        // lipsum.ins's idiom for its terminal box: an active space defined
        // as \space keeps every space of the \Msg lines (TeX would collapse
        // them otherwise).
        let ins = "\\input docstrip\n\\nopreamble\\nopostamble\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n\\newread\\lipsread\n\\newread\\other\n\\def\\gen#1{\\openin\\lipsread=#1 }\n\\gen{x.txt}\n\\Msg{a   b}\n\\catcode`\\ =13 \\edef {\\space}\n\\Msg{done}\n\\Msg{*   x   *}\n{\\obeyspaces\\Msg{ spaced }}\n\\endbatchfile\n";
        let o = run(ins, &[("t.dtx", DTX)]);
        assert_eq!(text(&o, "t.sty"), "\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n");
        let messages: Vec<&str> = o.diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(o.diagnostics.iter().filter(|d| d.message.starts_with("\\newread")).count(), 1, "{messages:?}");
        assert!(messages.iter().any(|m| m.starts_with("\\openin is not a docstrip command")), "{messages:?}");
        assert!(!messages.iter().any(|m| m.contains("catcode")), "{messages:?}");
        assert_eq!(o.diagnostics[0].file, "t.ins");
        assert_eq!(o.diagnostics[0].line, 4);
        assert_eq!(&o.messages[2..], ["a b", "done", "*   x   *", " spaced "], "{:?}", o.messages);
        assert!(o.completed);
    }

    #[test]
    fn missing_sources_and_malformed_batch_files_are_diagnostics() {
        let o = run("\\input docstrip\n\\generate{\\file{t.sty}{\\from{missing.dtx}{package}}}\n", &[]);
        assert_eq!(o.diagnostics.len(), 1, "{:?}", o.diagnostics);
        assert_eq!(o.diagnostics[0].to_string(), "missing.dtx: Cannot find file missing.dtx");
        assert!(text(&o, "t.sty").starts_with("%%\n%% This is file `t.sty',"), "the preamble was written when the file opened");
        let o = run("\\input docstrip\n\\file{t.sty}{\\from{t.dtx}{package}}\n\\generate{\\from{t.dtx}{x}}\n\\generate{\\file{t.sty}}\n\\generate{\\file{t.sty}{}}\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}", &[("t.dtx", DTX)]);
        let messages: Vec<String> = o.diagnostics.iter().map(|d| d.to_string()).collect();
        assert!(messages[0].starts_with("t.ins:2: Command `\\file' only allowed"), "{messages:?}");
        assert!(messages[1].starts_with("t.ins:3: Command `\\from' only allowed"), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("names no \\from source")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("not closed")), "{messages:?}");
        assert!(o.completed);
        let o = run("\\input docstrip\n\\preamble\nnever closed\n", &[]);
        assert!(o.diagnostics[0].message.contains("not closed by a line starting with \\endpreamble"), "{:?}", o.diagnostics);
        let o = run("\\input docstrip\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n\\generate{\\file{t.sty}{\\from{t.dtx}{driver}}}\n", &[("t.dtx", DTX)]);
        assert_eq!(o.files.len(), 1, "a name generated twice keeps the last");
        assert!(text(&o, "t.sty").contains("\\documentclass{ltxdoc}"));
    }

    #[test]
    fn add_generation_date_and_usedir() {
        let opts = Options { today: Some((2026, 9, 19)) };
        let ins = "\\input docstrip\n\\AddGenerationDate\n\\preamble\nx\n\\endpreamble\n\\usedir{tex/latex/t}\n\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}\n";
        let o = crate::run_with("t.ins", ins.as_bytes(), &sources(&[("t.dtx", DTX)]), &opts);
        assert!(o.diagnostics.is_empty(), "{:?}", o.diagnostics);
        assert!(text(&o, "t.sty").starts_with("%%\n%% This is file `t.sty', generated on <2026/9/19> \n%% with the docstrip utility (v2.6c).\n%%\n%% The original source files were:\n"), "{}", text(&o, "t.sty"));
        assert_eq!(o.file("t.sty").unwrap().dir.as_deref(), Some("tex/latex/t"));
        let o = run(ins, &[("t.dtx", DTX)]);
        assert!(o.diagnostics[0].message.contains("today's date"));
        assert!(text(&o, "t.sty").contains("<0/0/0>"));
    }

    #[test]
    fn dir_sources_serve_plain_names_only() {
        let dir = std::env::temp_dir().join(format!("flashtex-docstrip-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.dtx"), DTX).unwrap();
        let s = crate::DirSources(dir.clone());
        assert!(s.read("t.dtx").is_some());
        assert!(s.read("../t.dtx").is_none());
        assert!(s.read(".t.dtx").is_none());
        assert!(s.read("nope.dtx").is_none());
        let o = crate::run("t.ins", b"\\input docstrip\n\\nopreamble\\nopostamble\\generate{\\file{t.sty}{\\from{t.dtx}{package}}}", &s);
        assert_eq!(text(&o, "t.sty"), "\\ProvidesPackage{t}\n\\def\\extra{0}\n%%\n%% meta\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
