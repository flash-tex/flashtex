//! `algorithm` floats and `algorithmic` environments in this compiler's own
//! parse tree. The package model (statements, nesting, numbering, keyword
//! texts) is `crate::algorithmic`, read from the source bytes; this module
//! lowers each statement line to a left-aligned paragraph for the compiler's
//! own layout: the printed line number or `\item[..]` label, glue for the
//! list indentation (`\labelwidth` + `\labelsep` + depth × `\algorithmicindent`,
//! in ems of the body font), then the statement's pieces with author material
//! parsed from its tokens. The render pipeline sets the exact list geometry
//! from `crate::algorithmic` itself.

use super::{Block, Inline, InputToken, ParagraphStyle, TextStyle, P};
use crate::algorithmic::{self as alg, Face, FloatStyle, LineKind, Piece};
use crate::diagnostics::Diagnostic;
use crate::lexer::TokenKind;
use crate::Span;

fn face_style(face: Face) -> TextStyle {
    match face {
        Face::Bold => TextStyle::BOLD,
        // No small-caps shape in this compiler's text style.
        Face::Roman | Face::SmallCaps => TextStyle::default(),
    }
}

fn set_space_before(inline: &mut Inline, value: bool) {
    match inline {
        Inline::Text { space_before, .. }
        | Inline::Math { space_before, .. }
        | Inline::Reference { space_before, .. }
        | Inline::Footnote { space_before, .. }
        | Inline::Logo { space_before, .. }
        | Inline::Rule { space_before, .. }
        | Inline::Verbatim { space_before, .. } => *space_before = value,
        _ => {}
    }
}

impl<'a> P<'a> {
    /// The project's pseudocode setup: the first document that loads a
    /// pseudocode package decides.
    fn algorithm_setup(&self) -> alg::Setup {
        self.documents
            .iter()
            .map(|d| alg::setup(d.text))
            .find(|s| s.dialect.is_some() || s.float_loaded)
            .unwrap_or_default()
    }

    /// `\begin{algorithmic}[n] ... \end{algorithmic}` read at `open` (the
    /// `\begin` token). Returns `false`, consuming nothing, when the source
    /// bytes there are not the environment (it came out of a macro).
    pub(super) fn algorithmic_environment(
        &mut self,
        open: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) -> bool {
        let document = open.document;
        let source = self.documents[document.0].text;
        if !source
            .get(open.start..)
            .is_some_and(|rest| rest.starts_with("\\begin{algorithmic}"))
        {
            return false;
        }
        let setup = self.algorithm_setup();
        let Some((algorithm, end)) = alg::parse_algorithmic(source, document, &setup, open.start)
        else {
            self.diags.push(Diagnostic::error(
                "unterminated environment 'algorithmic' — no matching \\end",
                Some(open),
                Some("typeset the body as ordinary text".into()),
            ));
            return false;
        };
        self.flush_paragraph(blocks, para);
        if setup.dialect.is_none() {
            self.diags.push(Diagnostic::warning(
                "algorithmic environment without \\usepackage{algorithmic} or {algpseudocode}; read with algorithmic's commands",
                Some(open),
                None,
            ));
        }
        for (span, message) in &algorithm.problems {
            self.diags
                .push(Diagnostic::warning(message.clone(), Some(*span), None));
        }
        let start = self.i;
        let mut stop = start;
        while stop < self.t.len()
            && self.t[stop].token.span.document == document
            && self.t[stop].token.span.start < end
        {
            stop += 1;
        }
        let tokens: Vec<InputToken> = self.t[start..stop].to_vec();
        self.i = stop;
        let indent_em = match setup.indent {
            alg::Dimen::Em(v) => v,
            // ex and pt against a 10pt body font's quad (a layout approximation).
            alg::Dimen::Ex(v) => v * 0.43,
            alg::Dimen::Pt(v) => v / 10.0,
        };
        let margin_em = algorithm.label_width_em() + alg::LABELSEP_EM;
        for line in &algorithm.lines {
            let mut content = Vec::new();
            match &line.kind {
                LineKind::NoText => continue,
                LineKind::Numbered if line.show_number => content.push(Inline::Text {
                    text: format!(
                        "{}{}{}",
                        setup.line_numbers.prefix, line.number, setup.line_numbers.suffix
                    ),
                    span: line.span,
                    style: TextStyle {
                        bold: setup.line_numbers.bold,
                        size: setup.line_numbers.size,
                        ..TextStyle::default()
                    },
                    space_before: false,
                }),
                LineKind::Labelled(label) => self.algorithm_pieces(label, &tokens, &mut content),
                LineKind::Numbered | LineKind::Unlabelled => {}
            }
            content.push(Inline::TextGlue {
                em: margin_em + indent_em * f64::from(line.depth),
                span: line.span,
            });
            self.algorithm_pieces(&line.pieces, &tokens, &mut content);
            blocks.push(Block::Styled {
                style: ParagraphStyle::FlushLeft,
                content,
                // #147's list structure: the open list frames, as for every
                // styled paragraph; an algorithm line has no pending `\\`.
                lists: self.list_frames.clone(),
                line_break_before: None,
            });
            self.finish_block_dependencies();
        }
        true
    }

    fn algorithm_pieces(&mut self, pieces: &[Piece], tokens: &[InputToken], out: &mut Vec<Inline>) {
        let mut space = false;
        for piece in pieces {
            let first = out.len();
            match piece {
                Piece::Text { text, face, span } => out.push(Inline::Text {
                    text: text.clone(),
                    span: *span,
                    style: face_style(*face),
                    space_before: false,
                }),
                Piece::Source(span) | Piece::StyledSource { span, .. } => {
                    let face = match piece {
                        Piece::StyledSource { face, .. } => *face,
                        _ => Face::Roman,
                    };
                    let source = self.documents[span.document.0].text;
                    let inner: Vec<InputToken> = tokens
                        .iter()
                        .filter_map(|t| {
                            let s = t.token.span;
                            if s.document != span.document
                                || s.end <= span.start
                                || s.start >= span.end
                            {
                                return None;
                            }
                            if s.start >= span.start && s.end <= span.end {
                                return Some(t.clone());
                            }
                            // A word token straddling the piece (`[odd]` of
                            // `\IF[odd]{..}`: brackets are word characters)
                            // keeps its part inside, when it is source text.
                            match &t.token.kind {
                                TokenKind::Word(word)
                                    if source.get(s.start..s.end) == Some(word.as_str()) =>
                                {
                                    let (a, b) = (s.start.max(span.start), s.end.min(span.end));
                                    let mut clipped = t.clone();
                                    clipped.token.kind =
                                        TokenKind::Word(source.get(a..b)?.to_string());
                                    clipped.token.span = Span::in_document(span.document, a, b);
                                    Some(clipped)
                                }
                                _ => None,
                            }
                        })
                        .collect();
                    let inlines = self.inlines_from_tokens(inner, face_style(face));
                    out.extend(inlines);
                }
                Piece::Space { .. } => {
                    space = true;
                    continue;
                }
                Piece::HFill { span } => out.push(Inline::HFill { span: *span }),
                Piece::MathSymbol { tex, span } => out.push(Inline::Text {
                    text: match *tex {
                        "\\triangleright" => "▷",
                        "\\{" => "{",
                        _ => "}",
                    }
                    .to_string(),
                    span: *span,
                    style: TextStyle::default(),
                    space_before: false,
                }),
            }
            if let Some(inline) = out.get_mut(first) {
                set_space_before(inline, space);
            }
            space = false;
        }
    }

    /// `\caption{..}` directly inside `algorithm`: float.sty's `\float@caption`
    /// (lines 118-126) with the float style's `\@fs@capt`: `ruled`/`boxed`
    /// set `{\bfseries Algorithm N} text` (lines 151, 157), `plain` sets
    /// `Algorithm N: text` (line 140).
    pub(super) fn algorithm_caption(
        &mut self,
        span: Span,
        tokens: Vec<InputToken>,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        self.flush_paragraph(blocks, para);
        let setup = self.algorithm_setup();
        self.algorithm_counter += 1;
        let number = self.algorithm_counter.to_string();
        self.current_counter = Some(number.clone());
        let (number, style) = match setup.float_style {
            FloatStyle::Plain => (format!("{number}:"), TextStyle::default()),
            FloatStyle::Ruled | FloatStyle::Boxed => (number, TextStyle::BOLD),
        };
        let mut content = vec![
            Inline::Text {
                text: setup.float_name.clone(),
                span,
                style,
                space_before: true,
            },
            Inline::Text {
                text: number,
                span,
                style,
                space_before: true,
            },
        ];
        let mut body = self.inlines_from_tokens(tokens, TextStyle::default());
        if let Some(first) = body.first_mut() {
            set_space_before(first, true);
        }
        content.extend(body);
        blocks.push(Block::FigureCaption { content });
        self.finish_block_dependencies();
    }

    /// `\algnewcommand\name[n][default]{body}` / `\algrenewcommand`
    /// (algorithmicx.sty 621-622): read by `crate::algorithmic::setup` from
    /// the source; the tokens are consumed here.
    pub(super) fn algorithm_definition(&mut self, name: &str, span: Span) {
        self.skip_spaces();
        match self.peek().map(|token| token.kind.clone()) {
            Some(TokenKind::LBrace) => {
                let _ = self.required_group(name, span);
            }
            Some(TokenKind::Command(_)) => self.i += 1,
            _ => {}
        }
        while self.optional_bracket_argument().is_some() {}
        // The body is read `\long`: `\item[..]` label commands would
        // otherwise end the argument at a paragraph boundary.
        let _ = self.required_group_bounded(name, span, true);
    }
}
