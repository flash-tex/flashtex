//! What mode the document is in where a capture will land, and the wrapping
//! that makes an insertion there legal LaTeX.
//!
//! The destination pin already tells the bridge *where* a capture goes. It has
//! never said what *kind of place* that is, so a recogniser had to guess, and a
//! formula transcribed as `$x^2$` was inserted verbatim wherever the caret
//! happened to be. Inside an existing `$…$` that produces `$a + $x^2$ + b$`,
//! which pdflatex rejects outright ("Missing $ inserted"). The same applies to
//! `\[…\]` inside `equation`, and to display math in a `tabular` cell.
//!
//! Every rule below was checked against pdflatex, not assumed;
//! `scripts/caret_context_oracle.py` compiles each case in
//! `protocol/fixtures/caret-context-v1.json` twice and reports which rules turn
//! a real pdflatex error into a clean compile. Note that the FlashTeX engine is
//! currently *more permissive* than pdflatex on all of these — it reports no
//! diagnostic for `$a + $x^2$ + b$` — so the engine cannot be used to catch the
//! bug and pdflatex is the oracle.
use serde::{Deserialize, Serialize};

/// Math environments: a caret inside one is already in math mode. Kept in step
/// with `SyntaxHighlighter.mathEnvironments` in `apps/mac`; the fixture table
/// is what actually holds the two implementations together.
const MATH_ENVIRONMENTS: &[&str] = &[
    "math", "displaymath", "equation", "equation*", "align", "align*", "alignat", "alignat*",
    "gather", "gather*", "multline", "multline*", "flalign", "flalign*", "eqnarray", "eqnarray*",
    "split", "aligned", "gathered", "cases", "dcases", "matrix", "pmatrix", "bmatrix", "Bmatrix",
    "vmatrix", "Vmatrix", "smallmatrix", "array", "subequations", "empheq", "IEEEeqnarray",
    "IEEEeqnarray*",
];

/// Verbatim-like environments: their bodies are not LaTeX at all, so nothing in
/// them opens math and nothing inserted into them may be wrapped.
const VERBATIM_ENVIRONMENTS: &[&str] = &[
    "verbatim", "verbatim*", "Verbatim", "BVerbatim", "LVerbatim", "lstlisting", "minted", "alltt",
    "filecontents", "filecontents*",
];

/// Text-mode tabular cells. A cell is restricted horizontal mode: `\[…\]` in one
/// is a pdflatex error, so only inline math can be inserted there.
const TABULAR_ENVIRONMENTS: &[&str] = &[
    "tabular", "tabular*", "tabularx", "tabulary", "longtable", "supertabular", "xtabular",
];

/// The largest enclosing-environment stack reported. Deeper nesting is still
/// tracked for mode purposes; only the reported list is capped.
const MAX_REPORTED_ENVIRONMENTS: usize = 16;

/// How the document reads where the caret sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Text,
    InlineMath,
    DisplayMath,
    Verbatim,
    Comment,
}

/// What an insertion at this caret must do with recognised mathematics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrap {
    /// Text mode where a display block is legal: a formula of its own becomes `\[…\]`.
    Display,
    /// Text mode where only inline math is legal (mid-sentence, or a tabular cell).
    Inline,
    /// The caret is already inside math: emit bare math, never a delimiter.
    AlreadyMath,
    /// Verbatim or a comment: insert the transcription exactly, wrap nothing.
    Literal,
}

/// The caret's document context, sent to the recogniser as
/// `destination_context.caret_context` and used again when the approved
/// proposal is turned into an edit. One value decides both, so the prompt and
/// the insertion cannot disagree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaretContext {
    pub mode: Mode,
    /// The delimiter that opened the current math, for a human reading the
    /// request: `$`, `$$`, `\(`, `\[`, or `environment`.
    pub delimiter: Option<String>,
    /// Innermost open environment, if any.
    pub environment: Option<String>,
    /// Enclosing environments, outermost first, capped at 16.
    pub environments: Vec<String>,
    /// Whether amsmath is loaded, which decides `\text{…}` vs `\mbox{…}`.
    pub amsmath: bool,
    pub wrap: Wrap,
}

impl Default for CaretContext {
    fn default() -> Self {
        Self {
            mode: Mode::Text,
            delimiter: None,
            environment: None,
            environments: vec![],
            amsmath: false,
            wrap: Wrap::Display,
        }
    }
}

impl CaretContext {
    /// One sentence for the provider prompt and the Mac's destination row.
    pub fn describe(&self) -> String {
        let place = match (self.mode, self.environment.as_deref()) {
            (Mode::Verbatim, Some(env)) => format!("inside \\begin{{{env}}}, which is not LaTeX"),
            (Mode::Verbatim, None) => "inside a verbatim block, which is not LaTeX".into(),
            (Mode::Comment, _) => "inside a comment".into(),
            (Mode::InlineMath, _) => format!(
                "already inside inline math opened by {}",
                self.delimiter.as_deref().unwrap_or("$")
            ),
            (Mode::DisplayMath, Some(env)) => format!("already inside \\begin{{{env}}} math"),
            (Mode::DisplayMath, _) => format!(
                "already inside display math opened by {}",
                self.delimiter.as_deref().unwrap_or("\\[")
            ),
            (Mode::Text, Some(env)) if TABULAR_ENVIRONMENTS.contains(&env) => {
                format!("in a \\begin{{{env}}} cell")
            }
            (Mode::Text, Some(env)) => format!("in text mode inside \\begin{{{env}}}"),
            (Mode::Text, None) => "in text mode".into(),
        };
        let rule = match self.wrap {
            Wrap::Display => "Wrap a formula that stands on its own in \\[ … \\] and a formula inside a sentence in $ … $.",
            Wrap::Inline => "Wrap every formula in $ … $. Display math (\\[ … \\], equation) is NOT legal here.",
            Wrap::AlreadyMath => "Emit bare mathematics with NO delimiters: do not write $, $$, \\(, \\[ or a math environment, because the destination is already in math mode and a second delimiter would close it.",
            Wrap::Literal => "Emit the transcription literally with no LaTeX markup at all: the destination is not typeset.",
        };
        format!("The insertion point is {place}. {rule}")
    }

    /// The math text-box command available here: amsmath's `\text` when the
    /// preamble loads it, otherwise the kernel's `\mbox` (`\text` without
    /// amsmath is a pdflatex error, verified by the oracle).
    pub fn text_box_command(&self) -> &'static str {
        if self.amsmath {
            "\\text"
        } else {
            "\\mbox"
        }
    }
}

/// A single math opener still waiting to be closed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Open {
    Dollar,
    DoubleDollar,
    Paren,
    Bracket,
    Environment(String),
}

/// Derive the caret context from the document prefix ending at `caret`
/// (a byte offset, which must be a character boundary).
///
/// This is a lexical scan, not a TeX interpreter: it does not expand macros, so
/// a `$` produced by `\newcommand` is invisible to it, exactly as it is to the
/// Mac's syntax highlighter. That is the same bounded-lexical-context promise
/// the bridge already makes for `definitions`.
pub fn derive(text: &str, caret: usize) -> CaretContext {
    let caret = caret.min(text.len());
    let prefix = &text[..caret];
    let bytes = prefix.as_bytes();

    let mut math: Vec<Open> = vec![];
    let mut environments: Vec<String> = vec![];
    let mut verbatim: Option<String> = None;
    let mut comment_environment = 0usize;
    let mut in_line_comment = false;
    let mut amsmath = false;

    let mut i = 0usize;
    while i < bytes.len() {
        // A verbatim body is inert: only its own \end matters.
        if let Some(name) = verbatim.clone() {
            let marker = format!("\\end{{{name}}}");
            match prefix[i..].find(&marker) {
                Some(at) => {
                    i += at + marker.len();
                    verbatim = None;
                    pop_environment(&mut environments, &name);
                    continue;
                }
                None => break,
            }
        }
        match bytes[i] {
            b'%' => {
                // Comment to end of line. If no newline follows, the caret is in it.
                match prefix[i..].find('\n') {
                    Some(at) => i += at + 1,
                    None => {
                        in_line_comment = true;
                        i = bytes.len();
                    }
                }
            }
            b'\\' => {
                let name_end = control_sequence_end(bytes, i);
                let name = &prefix[i + 1..name_end];
                match name {
                    "[" => {
                        math.push(Open::Bracket);
                        i = name_end;
                    }
                    "]" => {
                        pop_math(&mut math, &Open::Bracket);
                        i = name_end;
                    }
                    "(" => {
                        math.push(Open::Paren);
                        i = name_end;
                    }
                    ")" => {
                        pop_math(&mut math, &Open::Paren);
                        i = name_end;
                    }
                    "verb" => i = skip_verb(bytes, name_end),
                    "begin" | "end" => {
                        let Some((env, after)) = braced_name(prefix, name_end) else {
                            i = name_end;
                            continue;
                        };
                        if name == "begin" {
                            environments.push(env.clone());
                            if VERBATIM_ENVIRONMENTS.contains(&env.as_str()) {
                                verbatim = Some(env);
                            } else if env == "comment" {
                                comment_environment += 1;
                            } else if MATH_ENVIRONMENTS.contains(&env.as_str()) {
                                math.push(Open::Environment(env));
                            }
                        } else {
                            pop_environment(&mut environments, &env);
                            if env == "comment" {
                                comment_environment = comment_environment.saturating_sub(1);
                            } else if MATH_ENVIRONMENTS.contains(&env.as_str()) {
                                pop_math(&mut math, &Open::Environment(env));
                            }
                        }
                        i = after;
                    }
                    "usepackage" | "RequirePackage" => {
                        if let Some((list, after)) = braced_name(prefix, name_end) {
                            amsmath |= list.split(',').any(|p| p.trim() == "amsmath");
                            i = after;
                        } else {
                            i = name_end;
                        }
                    }
                    // Any other control sequence, `\$` and `\%` included, is
                    // consumed whole: an escaped dollar never opens math.
                    _ => i = name_end,
                }
            }
            b'$' => {
                let double = bytes.get(i + 1) == Some(&b'$');
                let opener = if double { Open::DoubleDollar } else { Open::Dollar };
                // `$` does not nest: it closes the matching opener, or opens one.
                if math.last() == Some(&opener) {
                    math.pop();
                } else {
                    math.push(opener);
                }
                i += if double { 2 } else { 1 };
            }
            _ => i += 1,
        }
    }

    let environment = environments.last().cloned();
    let mode = if verbatim.is_some() {
        Mode::Verbatim
    } else if comment_environment > 0 || in_line_comment {
        Mode::Comment
    } else {
        match math.last() {
            Some(Open::Dollar) | Some(Open::Paren) => Mode::InlineMath,
            Some(_) => Mode::DisplayMath,
            None => Mode::Text,
        }
    };
    let delimiter = if matches!(mode, Mode::InlineMath | Mode::DisplayMath) {
        Some(
            match math.last() {
                Some(Open::Dollar) => "$",
                Some(Open::DoubleDollar) => "$$",
                Some(Open::Paren) => "\\(",
                Some(Open::Bracket) => "\\[",
                _ => "environment",
            }
            .to_owned(),
        )
    } else {
        None
    };
    let wrap = match mode {
        Mode::Verbatim | Mode::Comment => Wrap::Literal,
        Mode::InlineMath | Mode::DisplayMath => Wrap::AlreadyMath,
        Mode::Text => {
            if environments
                .iter()
                .any(|e| TABULAR_ENVIRONMENTS.contains(&e.as_str()))
                || !at_paragraph_position(text, caret)
            {
                Wrap::Inline
            } else {
                Wrap::Display
            }
        }
    };
    let skip = environments.len().saturating_sub(MAX_REPORTED_ENVIRONMENTS);
    CaretContext {
        mode,
        delimiter,
        environment,
        environments: environments[skip..].to_vec(),
        amsmath,
        wrap,
    }
}

/// Whether a display block would stand on its own here: only whitespace between
/// the caret and a blank line (or the document start) behind it, and the same
/// ahead. Mid-sentence, a display block would break the sentence in two.
fn at_paragraph_position(text: &str, caret: usize) -> bool {
    let before = text[..caret].trim_end_matches([' ', '\t']);
    let starts = before.is_empty() || before.ends_with("\n\n") || before.ends_with('\n') && before.trim_end().is_empty();
    let after = text[caret..].trim_start_matches([' ', '\t']);
    let ends = after.is_empty() || after.starts_with("\n\n") || after.starts_with('\n') && after.trim_start().is_empty();
    starts && ends
}

/// End offset of the control sequence starting at `i` (`bytes[i] == b'\\'`).
/// A control word runs over letters; a control symbol is exactly one character.
fn control_sequence_end(bytes: &[u8], i: usize) -> usize {
    let mut end = i + 1;
    if bytes.get(end).is_some_and(u8::is_ascii_alphabetic) {
        while bytes.get(end).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'*') {
            end += 1;
        }
        // `\begin*` is not a thing; only keep a trailing `*` for starred names.
        return end;
    }
    if end < bytes.len() {
        end += 1;
        while bytes.get(end).is_some_and(|c| c & 0xc0 == 0x80) {
            end += 1;
        }
    }
    end
}

/// `\verb<delim>…<delim>`: its body is inert.
fn skip_verb(bytes: &[u8], mut i: usize) -> usize {
    if bytes.get(i) == Some(&b'*') {
        i += 1;
    }
    let Some(&delimiter) = bytes.get(i) else {
        return i;
    };
    i += 1;
    while i < bytes.len() && bytes[i] != delimiter && bytes[i] != b'\n' {
        i += 1;
    }
    (i + 1).min(bytes.len())
}

/// `{name}` immediately after `at` (optional spaces), returning the name and the
/// offset just past the closing brace.
fn braced_name(text: &str, at: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let mut i = at;
    while bytes.get(i).is_some_and(|c| *c == b' ' || *c == b'\t') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'{') {
        return None;
    }
    let close = text[i..].find('}')? + i;
    Some((text[i + 1..close].trim().to_owned(), close + 1))
}

/// Close the innermost matching opener. An unmatched `\]` or `\end{equation}`
/// is ignored rather than corrupting the stack.
fn pop_math(math: &mut Vec<Open>, opener: &Open) {
    if let Some(at) = math.iter().rposition(|o| o == opener) {
        math.truncate(at);
    }
}

fn pop_environment(environments: &mut Vec<String>, name: &str) {
    if let Some(at) = environments.iter().rposition(|e| e == name) {
        environments.truncate(at);
    }
}

// ---------------------------------------------------------------------------
// Insertion normalisation
// ---------------------------------------------------------------------------

/// Structural math commands. Their presence is the *only* signal that turns
/// undelimited output into mathematics worth wrapping; see `shape`.
const MATH_MARKERS: &[&str] = &[
    "frac", "sqrt", "left", "right", "sum", "int", "prod", "lim", "cdot", "times", "div", "pm",
    "leq", "geq", "neq", "approx", "equiv", "infty", "partial", "nabla", "forall", "exists", "in",
    "subset", "cup", "cap", "to", "rightarrow", "alpha", "beta", "gamma", "delta", "theta",
    "lambda", "mu", "sigma", "phi", "omega", "pi", "binom", "overline", "underline", "hat", "vec",
];

/// Text-structure commands. Their presence means the transcription is a
/// paragraph, not a formula, whatever else it contains.
const TEXT_MARKERS: &[&str] = &[
    "section", "subsection", "subsubsection", "paragraph", "item", "textbf", "emph", "textit",
    "caption", "footnote", "par", "begin",
];

/// What a transcription looks like once its delimiters are accounted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Undelimited mathematics: it needs whatever wrapping the caret calls for.
    BareMath,
    /// Prose with no mathematics in it.
    Prose,
    /// Already carries its own math delimiters, or mixes prose and formulas.
    Delimited,
}

/// The result of making a proposal safe to insert at a caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalized {
    /// The exact text to insert, or `None` when no safe insertion exists.
    pub text: Option<String>,
    /// Reviewer-facing notes, in the `ambiguities` vocabulary the review sheet
    /// already renders. An `UNSUPPORTED: ` entry blocks direct insertion.
    pub advisories: Vec<String>,
}

/// Make `latex` legal at `context`.
///
/// The guarantee this function exists for: **the text it returns never adds a
/// math delimiter inside math and never leaves one unbalanced.** That is what
/// turns the owner's report into a fixed bug rather than a better prompt — a
/// recogniser that ignores its instructions still cannot produce
/// `$a + $x^2$ + b$`.
pub fn normalize(latex: &str, context: &CaretContext) -> Normalized {
    let body = latex.trim();
    let mut advisories = vec![];

    if context.wrap == Wrap::Literal {
        let place = if context.mode == Mode::Verbatim {
            "verbatim block"
        } else {
            "comment"
        };
        advisories.push(format!(
            "AMBIGUOUS: the destination is a {place}; the transcription was inserted literally \
             and is not typeset{}",
            if context.mode == Mode::Verbatim {
                " as LaTeX"
            } else {
                ""
            }
        ));
        return Normalized {
            text: Some(body.to_owned()),
            advisories,
        };
    }

    let scan = scan_math(body);
    if !scan.balanced {
        return refuse(
            "UNSUPPORTED: the proposal's math delimiters are unbalanced and it cannot be \
             inserted safely",
        );
    }

    match context.wrap {
        Wrap::Literal => unreachable!("handled above"),
        Wrap::AlreadyMath => {
            // Peel every layer the model wrapped around the formula. One `$…$`
            // inside an existing `$…$` is the reported bug; peeling is what
            // makes it impossible rather than unlikely.
            let mut inner = body;
            let mut peeled = false;
            loop {
                let scan = scan_math(inner);
                let Some(span) = scan.whole else { break };
                inner = inner[span.inner.clone()].trim();
                peeled = true;
            }
            if !scan_math(inner).spans.is_empty() {
                return refuse(
                    "UNSUPPORTED: text-mode content with its own math delimiters cannot be \
                     inserted at a math caret",
                );
            }
            let _ = peeled;
            // Already wrapped in a text box by an earlier pass: normalising a
            // second time must not nest \mbox inside \mbox.
            if shape(inner) == Shape::Prose && !inner.is_empty() && !is_text_box_group(inner) {
                let command = context.text_box_command();
                advisories.push(format!(
                    "AMBIGUOUS: prose recognised at a math caret was wrapped in {command}{{...}}"
                ));
                return gated(format!("{command}{{{inner}}}"), context, advisories);
            }
            gated(inner.to_owned(), context, advisories)
        }
        Wrap::Inline | Wrap::Display => {
            if scan.spans.is_empty() {
                // Undelimited. Wrap it if it is mathematics; leave prose alone.
                if shape(body) == Shape::BareMath && !body.is_empty() {
                    let text = if context.wrap == Wrap::Display {
                        format!("\\[ {body} \\]")
                    } else {
                        format!("${body}$")
                    };
                    return gated(text, context, advisories);
                }
                return gated(body.to_owned(), context, advisories);
            }
            // Already delimited. The only thing that can still be illegal is
            // display math where only inline math fits (a tabular cell).
            if context.wrap == Wrap::Inline {
                if let Some(rewritten) = display_to_inline(body, &scan) {
                    advisories.push(
                        "AMBIGUOUS: display math is not allowed in a tabular cell; inserted as \
                         inline math"
                            .to_owned(),
                    );
                    return gated(rewritten, context, advisories);
                }
            }
            gated(body.to_owned(), context, advisories)
        }
    }
}

/// The last line of defence: whatever route produced `text`, refuse it unless it
/// is actually legal at this caret. Every success path goes through here, so a
/// bug in the wrapping logic degrades to a refusal a reviewer sees rather than
/// to LaTeX that will not compile.
fn gated(text: String, context: &CaretContext, mut advisories: Vec<String>) -> Normalized {
    let scan = scan_math(&text);
    let unsafe_reason = if !scan.balanced {
        Some("its math delimiters are unbalanced")
    } else if context.wrap == Wrap::AlreadyMath && !scan.spans.is_empty() {
        Some("it carries math delimiters and the destination is already in math mode")
    } else if scan.max_depth > 1 {
        Some("it nests math inside math")
    } else if context.wrap == Wrap::Inline && scan.spans.iter().any(|s| s.display) {
        Some("it uses display math where only inline math is legal")
    } else {
        None
    };
    match unsafe_reason {
        None => Normalized {
            text: Some(text),
            advisories,
        },
        Some(reason) => {
            advisories.retain(|a| a.starts_with("UNSUPPORTED: "));
            advisories.push(format!(
                "UNSUPPORTED: the proposal cannot be inserted at this caret because {reason}"
            ));
            Normalized {
                text: None,
                advisories,
            }
        }
    }
}

/// Whether `content` is exactly one `\text{…}` or `\mbox{…}` group, which is
/// what a previous normalisation of prose at a math caret produces.
fn is_text_box_group(content: &str) -> bool {
    for command in ["\\text{", "\\mbox{"] {
        if let Some(rest) = content.strip_prefix(command) {
            let mut depth = 1usize;
            let bytes = rest.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' => i += 1,
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1 == rest.len();
                        }
                    }
                    _ => (),
                }
                i += 1;
            }
        }
    }
    false
}

fn refuse(message: &str) -> Normalized {
    Normalized {
        text: None,
        advisories: vec![message.to_owned()],
    }
}

/// Rewrite every top-level display group as inline math, or `None` when there
/// was no display group to rewrite.
fn display_to_inline(body: &str, scan: &MathScan) -> Option<String> {
    if !scan.spans.iter().any(|s| s.display) {
        return None;
    }
    let mut out = String::with_capacity(body.len());
    let mut at = 0;
    for span in &scan.spans {
        out.push_str(&body[at..span.outer.start]);
        if span.display {
            out.push('$');
            out.push_str(body[span.inner.clone()].trim());
            out.push('$');
        } else {
            out.push_str(&body[span.outer.clone()]);
        }
        at = span.outer.end;
    }
    out.push_str(&body[at..]);
    Some(out)
}

/// Is this transcription mathematics, prose, or already delimited?
fn shape(content: &str) -> Shape {
    if !scan_math(content).spans.is_empty() {
        return Shape::Delimited;
    }
    let mut has_math_marker = false;
    let mut has_text_marker = false;
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let end = control_sequence_end(bytes, i);
            let name = &content[i + 1..end];
            has_math_marker |= MATH_MARKERS.contains(&name);
            has_text_marker |= TEXT_MARKERS.contains(&name);
            i = end;
            continue;
        }
        if matches!(bytes[i], b'^' | b'_') {
            has_math_marker = true;
        }
        i += 1;
    }
    if has_text_marker || content.contains("\n\n") {
        return Shape::Prose;
    }
    if has_math_marker {
        return Shape::BareMath;
    }
    // Deliberately conservative. Without a structural signal there is no
    // reliable way to tell undelimited mathematics (`x = y`) from ordinary
    // prose (`<F>`, `see figure 2`), and guessing wrong corrupts the document
    // in the silent direction. Content with no math marker is left exactly as
    // it came, and the prompt carries the instruction to delimit it.
    Shape::Prose
}

/// One balanced math group inside a transcription.
#[derive(Debug, Clone)]
pub struct MathSpan {
    /// Byte range of the group including its delimiters.
    pub outer: std::ops::Range<usize>,
    /// Byte range of the group's contents.
    pub inner: std::ops::Range<usize>,
    /// Display math (`$$`, `\[`, a display environment) rather than inline.
    pub display: bool,
}

#[derive(Debug, Clone)]
pub struct MathScan {
    /// Top-level groups, in order.
    pub spans: Vec<MathSpan>,
    /// Deepest math nesting reached. Anything above 1 is a pdflatex error.
    pub max_depth: usize,
    /// Every delimiter closed.
    pub balanced: bool,
    /// The single group covering the whole string, when there is one.
    pub whole: Option<MathSpan>,
}

/// Find the top-level math groups in a transcription, honouring `\$`, comments
/// and `\verb` exactly as `derive` does.
fn scan_math(content: &str) -> MathScan {
    let bytes = content.as_bytes();
    let mut stack: Vec<(Open, usize, usize)> = vec![]; // opener, outer start, inner start
    let mut spans: Vec<MathSpan> = vec![];
    let mut max_depth = 0usize;
    let mut i = 0usize;
    let close = |stack: &mut Vec<(Open, usize, usize)>,
                     spans: &mut Vec<MathSpan>,
                     opener: &Open,
                     outer_end: usize,
                     inner_end: usize| {
        if let Some(at) = stack.iter().rposition(|(o, _, _)| o == opener) {
            let (open, outer_start, inner_start) = stack[at].clone();
            stack.truncate(at);
            if stack.is_empty() {
                spans.push(MathSpan {
                    outer: outer_start..outer_end,
                    inner: inner_start..inner_end,
                    display: !matches!(open, Open::Dollar | Open::Paren),
                });
            }
        }
    };
    while i < bytes.len() {
        match bytes[i] {
            b'%' => match content[i..].find('\n') {
                Some(at) => i += at + 1,
                None => break,
            },
            b'\\' => {
                let end = control_sequence_end(bytes, i);
                let name = &content[i + 1..end];
                match name {
                    "[" => {
                        stack.push((Open::Bracket, i, end));
                        max_depth = max_depth.max(stack.len());
                        i = end;
                    }
                    "]" => {
                        close(&mut stack, &mut spans, &Open::Bracket, end, i);
                        i = end;
                    }
                    "(" => {
                        stack.push((Open::Paren, i, end));
                        max_depth = max_depth.max(stack.len());
                        i = end;
                    }
                    ")" => {
                        close(&mut stack, &mut spans, &Open::Paren, end, i);
                        i = end;
                    }
                    "verb" => i = skip_verb(bytes, end),
                    "begin" | "end" => {
                        let Some((env, after)) = braced_name(content, end) else {
                            i = end;
                            continue;
                        };
                        if MATH_ENVIRONMENTS.contains(&env.as_str()) {
                            if name == "begin" {
                                stack.push((Open::Environment(env), i, after));
                                max_depth = max_depth.max(stack.len());
                            } else {
                                close(&mut stack, &mut spans, &Open::Environment(env), after, i);
                            }
                        }
                        i = after;
                    }
                    _ => i = end,
                }
            }
            b'$' => {
                let double = bytes.get(i + 1) == Some(&b'$');
                let width = if double { 2 } else { 1 };
                let opener = if double { Open::DoubleDollar } else { Open::Dollar };
                if stack.last().map(|(o, _, _)| o) == Some(&opener) {
                    close(&mut stack, &mut spans, &opener, i + width, i);
                } else {
                    stack.push((opener, i, i + width));
                    max_depth = max_depth.max(stack.len());
                }
                i += width;
            }
            _ => i += 1,
        }
    }
    let balanced = stack.is_empty();
    let whole = spans
        .first()
        .filter(|s| spans.len() == 1 && s.outer.start == 0 && s.outer.end == content.len())
        .cloned();
    MathScan {
        spans,
        max_depth,
        balanced,
        whole,
    }
}

/// The already-normalised `latex` if it is still legal at `context`, or `None`.
///
/// `normalize` runs once, at conversion, so the reviewer approves the exact
/// text that will be inserted. This is the check repeated at the moment the
/// edit is issued: it is a pure predicate, so running it twice changes nothing,
/// and it closes the gap where a stored proposal outlives the reasoning that
/// produced it.
pub fn insertable(latex: &str, context: &CaretContext) -> Option<String> {
    if context.wrap == Wrap::Literal {
        return Some(latex.to_owned());
    }
    gated(latex.to_owned(), context, vec![]).text
}
