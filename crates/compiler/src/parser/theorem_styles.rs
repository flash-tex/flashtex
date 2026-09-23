//! Declared theorem styles and the theorem-declaring commands of thmtools
//! and mdframed, all reduced to [`CustomTheoremStyle`] + `define_theorem`.
//!
//! * amsthm `\newtheoremstyle{name}{above}{below}{body font}{indent}{head
//!   font}{head punct}{head sep}{head spec}` (amsthm.sty 236-270);
//! * thmtools `\declaretheoremstyle[keys]{name}` and `\declaretheorem[keys]
//!   {env}` (thmtools.sty/thm-kv.sty key names), whose styles are
//!   `\newtheoremstyle`s under the hood;
//! * mdframed `\newmdtheoremenv[opts]{env}[shared]{Title}[within]`, which is
//!   `\newtheorem` plus a frame. The frame is not drawn yet (reported once);
//!   the head, numbering and body fonts are the theorem's.
//!
//! The vertical skips (`above`/`below`, `spaceabove`/`spacebelow`) and the
//! head separator belong to the render pipeline, which reads them from the
//! same declarations.

use super::{apply_style, lists, parse_dimen_pt_current, token_text, Inline, InputToken, TextStyle, P};
use crate::diagnostics::Diagnostic;
use crate::lexer::TokenKind;
use crate::lexer::tokenize;
use crate::theorems::{CustomTheoremStyle, StyleRef, TheoremDef, TheoremStyle};
use crate::Span;

impl P<'_> {
    /// The style `names` (control words) give over `\normalfont`, at the
    /// document's body size.
    fn font_declarations(&self, base: TextStyle, tokens: &[InputToken]) -> TextStyle {
        let body = self.body_size_pt();
        let scheme = self.nfss_scheme();
        tokens.iter().fold(base, |style, token| match &token.token.kind {
            TokenKind::Command(name) => apply_style(style, name, body, scheme),
            _ => style,
        })
    }

    /// Font declarations written as text (a thmtools key value).
    fn font_declarations_text(&self, base: TextStyle, text: &str) -> TextStyle {
        let body = self.body_size_pt();
        let scheme = self.nfss_scheme();
        text.split('\\')
            .skip(1)
            .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphabetic()))
            .map(|word| word.split(|c: char| !c.is_ascii_alphabetic()).next().unwrap_or(""))
            .filter(|word| !word.is_empty())
            .fold(base, |style, name| apply_style(style, name, body, scheme))
    }

    /// `\newtheoremstyle{name}{above}{below}{body}{indent}{head}{punct}
    /// {headsep}{headspec}`.
    pub(super) fn new_theorem_style(&mut self, span: Span) {
        let mut args: Vec<Vec<InputToken>> = Vec::with_capacity(9);
        for _ in 0..9 {
            args.push(self.required_group("newtheoremstyle", span).0);
        }
        let name = token_text(&args[0]).trim().to_string();
        if name.is_empty() {
            self.diags.push(Diagnostic::error(
                "\\newtheoremstyle was given an empty style name",
                Some(span),
                Some("ignored the declaration".into()),
            ));
            return;
        }
        let body = self.font_declarations(TextStyle::default(), &args[3]);
        let indent = token_text(&args[4]).trim().to_string();
        let head = self.font_declarations(TextStyle::default(), &args[5]);
        let punct = token_text(&args[6]).trim().to_string();
        let headsep = tokens_source(&args[7]).trim().to_string();
        let spec = tokens_source(&args[8]).trim().to_string();
        self.theorem_styles.insert(
            name,
            CustomTheoremStyle {
                head,
                body,
                note: None,
                punct,
                indent: nonzero_dimension(&indent),
                newline: headsep == "\\newline",
                head_spec: (!spec.is_empty()).then_some(spec),
                note_braces: ("(".into(), ")".into()),
            },
        );
    }

    /// thmtools `\declaretheoremstyle[keys]{name}`. Keys not listed here
    /// (`mdframed`, `shaded`, `thmbox`, `qed`, ...) are reported once.
    pub(super) fn declare_theorem_style(&mut self, span: Span) {
        let keys = self.optional_bracket_argument_braced().map(|(text, _)| text).unwrap_or_default();
        let (name_tokens, _) = self.required_group("declaretheoremstyle", span);
        let name = token_text(&name_tokens).trim().to_string();
        // thm-amsthm.sty `\thmt@declaretheoremstyle@setup`: bold head,
        // `\normalfont` body, `.`, an interword space, 3pt above/below.
        let base = CustomTheoremStyle { body: TextStyle::default(), ..builtin_as_custom(TheoremStyle::Plain) };
        let style = self.thmtools_style(&keys, span, Some(base));
        if !name.is_empty() {
            self.theorem_styles.insert(name, style);
        }
    }

    /// A thmtools key list's style, starting from `base` (`plain` if none).
    fn thmtools_style(&mut self, keys: &str, span: Span, base: Option<CustomTheoremStyle>) -> CustomTheoremStyle {
        let mut style = base.unwrap_or_else(|| builtin_as_custom(TheoremStyle::Plain));
        let mut unsupported: Vec<String> = Vec::new();
        for part in lists::split_top_level(keys) {
            let (key, value) = match part.split_once('=') {
                Some((k, v)) => (k.trim(), lists::strip_outer_braces(v.trim()).to_string()),
                None => (part.trim(), String::new()),
            };
            match key {
                "headfont" => style.head = self.font_declarations_text(TextStyle::default(), &value),
                "bodyfont" => style.body = self.font_declarations_text(TextStyle::default(), &value),
                "notefont" => style.note = Some(self.font_declarations_text(style.head, &value)),
                "headpunct" => style.punct = value,
                "headindent" => style.indent = nonzero_dimension(&value),
                "postheadspace" => style.newline = value.trim() == "\\newline",
                "headformat" => {
                    style.head_spec = match value.trim() {
                        "" | "margin" | "swapnumber" => style.head_spec.take(),
                        spec => Some(
                            spec.replace("\\NAME", "\\thmname{#1}")
                                .replace("\\NUMBER", "\\thmnumber{#2}")
                                .replace("\\NOTE", "\\thmnote{ (#3)}"),
                        ),
                    }
                }
                "notebraces" => {
                    let braces: Vec<String> = brace_groups(&value);
                    if braces.len() == 2 {
                        style.note_braces = (braces[0].clone(), braces[1].clone());
                    }
                }
                // Vertical skips: the render pipeline reads these.
                "spaceabove" | "spacebelow" | "style" | "name" | "numbered" | "numberwithin" | "within"
                | "sibling" | "numberlike" | "sharenumber" | "parent" | "title" | "heading" | "refname"
                | "Refname" | "preheadhook" | "postheadhook" | "prefoothook" | "postfoothook" => {}
                other => unsupported.push(other.to_string()),
            }
        }
        if !unsupported.is_empty() {
            self.diags.push(Diagnostic::warning(
                format!("thmtools keys {} are recognised but not implemented", unsupported.join(", ")),
                Some(span),
                Some("the theorem is set without them (an mdframed/shaded/thmbox frame is not drawn)".into()),
            ));
        }
        style
    }

    /// thmtools `\declaretheorem[keys]{env,env,...}`: a `\newtheorem` per
    /// name, titled `name=` (else the environment name with its first letter
    /// upper-cased), numbered `numberwithin=`/`sibling=`, unnumbered with
    /// `numbered=no`, in `style=` (else the `\theoremstyle` in force).
    pub(super) fn declare_theorem(&mut self, span: Span) {
        let keys = self.optional_bracket_argument_braced().map(|(text, _)| text).unwrap_or_default();
        let (names, names_span) = self.required_group("declaretheorem", span);
        // thmtools ≥ 2020 also takes the keys after the name.
        let keys = match self.optional_bracket_argument_braced() {
            Some((after, _)) if keys.is_empty() => after,
            Some((after, _)) => format!("{keys},{after}"),
            None => keys,
        };
        let mut title = None;
        let mut within = None;
        let mut shared = None;
        let mut numbered = true;
        let mut spec = self.theorem_spec.clone();
        let mut local = Vec::new();
        for part in lists::split_top_level(&keys) {
            let (key, value) = match part.split_once('=') {
                Some((k, v)) => (k.trim(), lists::strip_outer_braces(v.trim()).to_string()),
                None => (part.trim(), String::new()),
            };
            match key {
                "name" | "title" | "heading" => title = Some(value),
                "numberwithin" | "within" | "parent" => within = Some((value, span)),
                "sibling" | "numberlike" | "sharenumber" => shared = Some((value, span)),
                "numbered" => numbered = !matches!(value.trim(), "no" | "false"),
                "style" => {
                    spec = match self.theorem_styles.get(value.trim()) {
                        Some(custom) => StyleRef::Custom(Box::new(custom.clone())),
                        None => match TheoremStyle::from_name(value.trim()) {
                            Some(builtin) => StyleRef::Builtin(builtin),
                            None => {
                                self.diags.push(Diagnostic::warning(
                                    format!("\\declaretheorem style '{}' is not declared", value.trim()),
                                    Some(span),
                                    Some("kept the \\theoremstyle in force".into()),
                                ));
                                spec
                            }
                        },
                    }
                }
                "refname" | "Refname" => {}
                _ => local.push(part.to_string()),
            }
        }
        // Style keys given directly to `\declaretheorem` refine its style.
        if !local.is_empty() {
            let base = match &spec {
                StyleRef::Custom(custom) => (**custom).clone(),
                StyleRef::Builtin(builtin) => builtin_as_custom(*builtin),
            };
            spec = StyleRef::Custom(Box::new(self.thmtools_style(&local.join(","), span, Some(base))));
        }
        for name in token_text(&names).split(',').map(str::trim).filter(|n| !n.is_empty()) {
            let title = title.clone().unwrap_or_else(|| capitalize(name));
            self.define_theorem(name.to_string(), names_span, shared.clone(), title, within.clone(), !numbered, span, spec.clone());
        }
    }

    /// mdframed `\newmdtheoremenv[opts]{env}[shared]{Title}[within]`.
    pub(super) fn new_md_theorem_env(&mut self, span: Span) {
        let _options = self.optional_bracket_argument_braced();
        let (name_tokens, name_span) = self.required_group("newmdtheoremenv", span);
        let name = token_text(&name_tokens).trim().to_string();
        let shared = self.optional_bracket_argument();
        let (title_tokens, _) = self.required_group("newmdtheoremenv", span);
        let title = token_text(&title_tokens).trim().to_string();
        let within = if shared.is_none() { self.optional_bracket_argument() } else { None };
        self.mdframed_frame_not_drawn(span);
        let spec = self.theorem_spec.clone();
        self.define_theorem(name, name_span, shared, title, within, false, span, spec);
    }

    /// `\mdfdefinestyle{name}{keys}`, `\surroundwithmdframed[opts]{env}`:
    /// consumed; the frame they describe is not drawn.
    pub(super) fn mdframed_declaration(&mut self, name: &str, span: Span) {
        if name == "surroundwithmdframed" {
            let _ = self.optional_bracket_argument_braced();
            let _ = self.required_group(name, span);
            self.mdframed_frame_not_drawn(span);
        } else {
            let _ = self.required_group(name, span);
            let _ = self.required_group(name, span);
        }
    }

    fn mdframed_frame_not_drawn(&mut self, span: Span) {
        if !self.mdframed_reported {
            self.mdframed_reported = true;
            self.diags.push(Diagnostic::warning(
                "mdframed frames are recognised but not implemented",
                Some(span),
                Some("the environment is set without its frame and inner margins".into()),
            ));
        }
    }
}

impl P<'_> {
    /// The head of a theorem in a declared style or declared under
    /// `\swapnumbers` (amsthm.sty `\@begintheorem`): `\thm@indent`, the
    /// head spec (`\thmhead@plain`, `\swappedhead` or `#9`), the
    /// punctuation, and `\thmheadnl`. The separator glue after it is the
    /// render pipeline's. Every synthesised run carries the `\begin` span,
    /// which is how the pipeline recognises a theorem head.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn declared_theorem_head(
        &mut self,
        def: &TheoremDef,
        head: TextStyle,
        number_style: TextStyle,
        number: Option<String>,
        note: Option<(Vec<InputToken>, Span)>,
        span: Span,
        para: &mut Vec<Inline>,
    ) {
        let custom = def.spec.custom().cloned().unwrap_or_else(|| builtin_as_custom(def.style));
        let text = |text: String, style: TextStyle, space_before: bool| Inline::Text {
            text,
            span,
            style,
            space_before,
            boundary_before: false,
            glue_before: None,
        };
        let start = para.len();
        // `\thm@indent` = `\noindent\hbox to#5{}`, in the head font.
        if let Some(indent) = &custom.indent {
            let units = self.font_setup().em_ex_sp(head);
            if let Some(pt) = parse_dimen_pt_current(indent, units).filter(|pt| *pt != 0.0) {
                para.push(Inline::HSpace {
                    pt,
                    space_before_pt: 0.0,
                    space_after_pt: 0.0,
                    span,
                    stretch_pt: 0.0,
                    stretch_fil: 0,
                    shrink_pt: 0.0,
                    shrink_fil: 0,
                    style: head,
                });
            }
        }
        let note = note.filter(|(tokens, _)| {
            !tokens.iter().all(|t| matches!(t.token.kind, TokenKind::Space | TokenKind::Comment))
        });
        let name = def.title.clone();
        if def.swap {
            // `\swappedhead`: `\thmnumber{#2}\thmname{\@ifnotempty{#2}{~}#1}`
            // then the plain note.
            if let Some(number) = &number {
                para.push(text(number.clone(), number_style, true));
                para.push(text(" ".into(), head, false));
            }
            para.push(text(name, head, number.is_none()));
            self.plain_note(&custom, head, note, span, para);
        } else if let Some(spec) = &custom.head_spec {
            // The note's own tokens are spliced in where `#3` stands, so it
            // reads exactly as the plain head's note does.
            const NOTE: char = '\u{E000}';
            let placeholder = note.as_ref().map(|_| NOTE.to_string());
            let source = expand_head_spec(spec, &name, number.as_deref(), placeholder.as_deref());
            let mut tokens: Vec<InputToken> = Vec::new();
            for mut token in tokenize(&source) {
                token.span = span;
                let split = match &token.kind {
                    TokenKind::Word(w) => w.split_once(NOTE).map(|(a, b)| (a.to_string(), b.to_string())),
                    _ => None,
                };
                match &split {
                    Some((before, after)) => {
                        for (text, is_note) in [(before.clone(), false), (String::new(), true), (after.clone(), false)] {
                            if is_note {
                                tokens.extend(note.as_ref().map(|(t, _)| t.clone()).unwrap_or_default());
                            } else if !text.is_empty() {
                                let mut piece = token.clone();
                                piece.kind = TokenKind::Word(text);
                                tokens.push(InputToken { token: piece, definition: None, maps_to_invocation: false });
                            }
                        }
                    }
                    None => tokens.push(InputToken { token, definition: None, maps_to_invocation: false }),
                }
            }
            eprintln!("DBGTOK {:?}", tokens.iter().map(|t| format!("{:?}", t.token.kind)).collect::<Vec<_>>());
            let mut inner = self.inlines_from_tokens_reporting(tokens, head, true, false);
            eprintln!("DBGINL {:?}", inner.len());
            if let Some(Inline::Text { space_before, .. }) = inner.first_mut() {
                *space_before = true;
            }
            para.extend(inner);
        } else {
            para.push(text(name, head, true));
            if let Some(number) = number {
                para.push(text(" ".into(), head, false));
                para.push(text(number, number_style, false));
            }
            self.plain_note(&custom, head, note, span, para);
        }
        if !custom.punct.is_empty() {
            para.push(text(custom.punct.clone(), head, false));
        }
        if custom.newline {
            para.push(Inline::LineBreak { span, skip_pt: None });
        }
        // The head must open the paragraph with a `Text` on the `\begin`
        // span; an indent box goes after nothing else.
        debug_assert!(para.len() > start);
    }

    /// `\thmnote{ {\the\thm@notefont(#3)}}`: a head-font space, then the
    /// note in `\thm@notefont` between its braces.
    fn plain_note(
        &mut self,
        custom: &CustomTheoremStyle,
        head: TextStyle,
        note: Option<(Vec<InputToken>, Span)>,
        span: Span,
        para: &mut Vec<Inline>,
    ) {
        let Some((tokens, note_span)) = note else { return };
        let note_style = custom.note.unwrap_or_else(|| {
            let md = apply_style(head, "mdseries", self.body_size_pt(), self.nfss_scheme());
            apply_style(md, "upshape", self.body_size_pt(), self.nfss_scheme())
        });
        let text = |text: String, span: Span, style: TextStyle| Inline::Text {
            text,
            span,
            style,
            space_before: false,
            boundary_before: false,
            glue_before: None,
        };
        para.push(text(" ".into(), span, head));
        para.push(text(
            custom.note_braces.0.clone(),
            Span::in_document(note_span.document, note_span.start, note_span.start + 1),
            note_style,
        ));
        let mut inner = self.inlines_from_tokens_reporting(tokens, note_style, true, false);
        if let Some(Inline::Text { space_before, .. }) = inner.first_mut() {
            *space_before = false;
        }
        para.extend(inner);
        para.push(text(
            custom.note_braces.1.clone(),
            Span::in_document(note_span.document, note_span.end - 1, note_span.end),
            note_style,
        ));
    }
}

/// The tokens written back as source (`token_text` drops backslashes and
/// braces, which a head spec and `\newline` need).
fn tokens_source(tokens: &[InputToken]) -> String {
    let mut out = String::new();
    for (i, input) in tokens.iter().enumerate() {
        match &input.token.kind {
            TokenKind::Word(w) => out.push_str(w),
            TokenKind::Command(name) => {
                out.push('\\');
                out.push_str(name);
                let letters = name.chars().all(|c| c.is_ascii_alphabetic());
                if letters && matches!(tokens.get(i + 1).map(|t| &t.token.kind), Some(TokenKind::Word(w)) if w.starts_with(|c: char| c.is_ascii_alphabetic())) {
                    out.push(' ');
                }
            }
            TokenKind::Space | TokenKind::ParBreak => out.push(' '),
            TokenKind::LBrace => out.push('{'),
            TokenKind::RBrace => out.push('}'),
            TokenKind::LineBreak => out.push_str("\\\\"),
            TokenKind::MathShift => out.push('$'),
            TokenKind::Superscript => out.push('^'),
            TokenKind::Subscript => out.push('_'),
            _ => {}
        }
    }
    out
}

/// `#9` with `\thmname{..}`, `\thmnumber{..}`, `\thmnote{..}` kept only
/// when the head has that part (`\@begintheorem`'s `\@ifempty` tests) and
/// `#1`/`#2`/`#3` replaced by the name, the number and the note's source.
pub(super) fn expand_head_spec(spec: &str, name: &str, number: Option<&str>, note: Option<&str>) -> String {
    let mut out = String::new();
    let mut rest = spec;
    while let Some(at) = rest.find("\\thm") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let word: String = after.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let tail = &after[word.len()..];
        let part = match word.as_str() {
            "thmname" => Some((!name.is_empty()).then_some(())),
            "thmnumber" => Some(number.map(|_| ())),
            "thmnote" => Some(note.map(|_| ())),
            _ => None,
        };
        match (part, tail.trim_start().strip_prefix('{')) {
            (Some(present), Some(group)) => {
                let mut depth = 1i32;
                let mut end = group.len();
                for (i, c) in group.char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if present.is_some() {
                    out.push_str(&group[..end]);
                }
                rest = &group[(end + 1).min(group.len())..];
            }
            _ => {
                out.push('\\');
                out.push_str(&word);
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out.replace("#1", name).replace("#2", number.unwrap_or("")).replace("#3", note.unwrap_or(""))
}

/// amsthm's builtin style as a declared one (for thmtools keys that refine
/// it).
pub(super) fn builtin_as_custom(style: TheoremStyle) -> CustomTheoremStyle {
    CustomTheoremStyle {
        head: style.head_style(),
        body: style.body_style(),
        note: None,
        punct: ".".into(),
        indent: None,
        newline: false,
        head_spec: None,
        note_braces: ("(".into(), ")".into()),
    }
}

/// `Some(text)` for an indent amount that is not empty or zero
/// (`\newtheoremstyle`'s `\ifdim\dimen@=\z@`).
fn nonzero_dimension(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | ',')).collect();
    match digits.replace(',', ".").trim_start_matches('+').parse::<f64>() {
        Ok(v) if v == 0.0 && !digits.is_empty() => None,
        _ => Some(text.to_string()),
    }
}

/// The top-level `{...}` groups of `text`, their contents.
fn brace_groups(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    out.push(text[start..i].to_string());
                }
            }
            _ => {}
        }
    }
    out
}

/// thmtools' default title: `\MakeUppercase` of the first letter.
fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
