//! beamer's blocks, columns and in-flow floats (issue #944, Tier 3):
//! `block`/`alertblock`/`exampleblock`, `columns` + `\column`/`column`,
//! and `figure`/`table` with `\caption` under `\documentclass{beamer}`.
//!
//! Everything here reads the environments' arguments and pushes marker
//! blocks ([`Block::BeamerBlockBegin`], [`Block::BeamerColumnsBegin`], ...)
//! around the ordinary blocks of their bodies; the geometry (the default
//! inner theme's block templates, the paper-wide `\hbox` a `columns` row
//! is, `\vcenter`/`\vtop` column boxes, the caption skips) lives in the
//! render pipeline (`typeset/beamer_blocks.rs`), transcribed from
//! `beamerinnerthemedefault.sty` 385-440, `beamerbaseframecomponents.sty`
//! 212-291 and `beamerbaselocalstructure.sty` 550-601.

use super::{
    beamer_columns_options, token_text, BeamerBlockKind, BeamerFloatKind,
    Block, FontSizeLevel, Inline, ParagraphStyle, TextStyle, P,
};
use crate::diagnostics::Diagnostic;
use crate::lexer::TokenKind;
use crate::Span;

/// `\abovecaptionskip` = `\belowcaptionskip` = 7pt
/// (`beamerbaselocalstructure.sty` 567-568).
pub const BEAMER_CAPTION_SKIP_PT: f64 = 7.0;

/// The environments this module owns (all beamer-only).
pub const BEAMER_ENVIRONMENTS: &[&str] = &["block", "alertblock", "exampleblock", "columns", "column"];

impl P<'_> {
    /// Whether `environment` is one of beamer's block/column environments,
    /// or its `figure`/`table` (in-flow `center` boxes under beamer).
    pub(super) fn is_beamer_block_environment(&self, environment: &str) -> bool {
        self.in_body
            && (BEAMER_ENVIRONMENTS.contains(&environment)
                || (self.is_beamer_class() && matches!(environment, "figure" | "table")))
    }

    /// `\begin{block}{title}` and friends, `\begin{columns}[opts]`,
    /// `\begin{column}[align]{width}`, and beamer's `\begin{figure}` /
    /// `\begin{table}`. Called from `begin_environment` before the
    /// environment stacks are pushed.
    pub(super) fn beamer_environment_begin(
        &mut self,
        environment: &str,
        span: Span,
        argument_span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        if !self.is_beamer_class() {
            // Like `beamer_command_available`: outside beamer these
            // environments are undefined ("Environment block undefined").
            let class = self
                .document_class
                .clone()
                .unwrap_or_else(|| "no \\documentclass".to_string());
            self.diags.push(
                Diagnostic::environment_warning(
                    environment,
                    format!(
                        "environment '{environment}' is defined by the beamer document class; this document is {class}"
                    ),
                    Some(span),
                    Some("typeset the body without the environment's formatting".into()),
                ),
            );
            // The `{title}`/`{width}` argument is left for the token loop.
            return;
        }
        let head = span.merge(argument_span);
        match environment {
            "block" | "alertblock" | "exampleblock" => {
                let kind = match environment {
                    "alertblock" => BeamerBlockKind::Alert,
                    "exampleblock" => BeamerBlockKind::Example,
                    _ => BeamerBlockKind::Plain,
                };
                // `\begin{block}<spec>{title}`: the block is covered on the
                // slides the specification excludes, like `uncoverenv`; the
                // marker rides with the title into the block's first
                // paragraph and its end closes at `\end{block}`.
                let overlay = self.take_beamer_overlay_spec();
                let (tokens, title_span) = self.required_group(environment, span);
                let has_overlay = overlay.is_some();
                if let Some((spec, spec_span)) = overlay {
                    para.push(Inline::OverlayBegin { spec, kind: crate::overlay::OverlayKind::Cover, span: spec_span });
                }
                self.beamer_block_overlays.push(has_overlay);
                self.flush_paragraph(blocks, para);
                let title = self.inlines_from_tokens(tokens, TextStyle::default());
                blocks.push(Block::BeamerBlockBegin {
                    kind,
                    title,
                    span: head.merge(title_span),
                });
                self.finish_block_dependencies();
            }
            "columns" => {
                self.skip_beamer_overlay_spec();
                let raw = self.optional_bracket_argument();
                let (options, _) = beamer_columns_options(raw.as_ref().map_or("", |(r, _)| r.as_str()));
                let span = raw.map_or(head, |(_, s)| head.merge(s));
                self.flush_paragraph(blocks, para);
                blocks.push(Block::BeamerColumnsBegin { options, span });
                self.finish_block_dependencies();
                self.beamer_columns_depth += 1;
            }
            "column" => self.beamer_column_marker("column", span, head, blocks, para),
            // beamer's `figure`/`table`: `\par\nobreak\begin{center}\nobreak`,
            // a `center` environment whose caption is `\beamer@makecaption`.
            _ => {
                let _ = self.optional_bracket_argument();
                self.flush_paragraph(blocks, para);
                // The `\par` leaves vertical mode for `center`'s `\@trivlist`.
                self.trivlist_pending = Some(super::TrivlistStart { vmode: true });
                self.paragraph_styles.push(ParagraphStyle::Center);
                self.declared_alignment = None;
            }
        }
    }

    /// `\end{...}` of one of the environments above.
    pub(super) fn beamer_environment_end(
        &mut self,
        environment: &str,
        span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        if !self.is_beamer_class() {
            return;
        }
        // `\caption{...}\label{...}\end{table}`: the `\label` is a whatsit
        // in vertical mode, not a paragraph. It joins the caption's content
        // (where `\refstepcounter` made its number current) instead of
        // flushing as a paragraph of its own after the caption's skip.
        if matches!(environment, "figure" | "table")
            && !para.is_empty()
            && para.iter().all(|i| matches!(i, Inline::Label { .. }))
        {
            let caption = blocks.iter_mut().rev().take(2).find_map(|b| match b {
                Block::BeamerCaption { content, .. } => Some(content),
                _ => None,
            });
            if let Some(content) = caption {
                content.append(para);
            }
        }
        self.flush_paragraph(blocks, para);
        match environment {
            "block" | "alertblock" | "exampleblock" => {
                blocks.push(Block::BeamerBlockEnd { span });
                self.finish_block_dependencies();
                if self.beamer_block_overlays.pop().unwrap_or(false) {
                    para.push(Inline::OverlayEnd { span });
                }
            }
            "columns" => {
                blocks.push(Block::BeamerColumnsEnd { span });
                self.finish_block_dependencies();
                self.beamer_columns_depth = self.beamer_columns_depth.saturating_sub(1);
            }
            // `\end{column}`: the next `\column`/`\begin{column}` or the
            // `\end{columns}` closes the box; nothing to mark.
            "column" => {}
            _ => {
                self.paragraph_styles.pop();
            }
        }
    }

    /// `\column<overlay>[align]{width}` (the command form) — the previous
    /// column closes here. Diagnosed outside `columns` (beamer:
    /// "\column outside columns environment").
    pub(super) fn beamer_column_command(
        &mut self,
        name: &str,
        span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        if !self.beamer_command_available(name, span) {
            return;
        }
        self.beamer_column_marker(name, span, span, blocks, para);
    }

    fn beamer_column_marker(
        &mut self,
        name: &str,
        span: Span,
        head: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        self.skip_beamer_overlay_spec();
        let align = self
            .optional_bracket_argument()
            .and_then(|(raw, _)| beamer_columns_options(&raw).1);
        self.skip_spaces();
        if !matches!(self.peek().map(|token| &token.kind), Some(TokenKind::LBrace)) {
            self.diags.push(Diagnostic::error(
                format!("\\{name} requires a braced width argument (e.g. \\column{{.5\\textwidth}})"),
                Some(span),
                Some("ignored the column and continued".into()),
            ));
            return;
        }
        let (tokens, width_span) = self.required_group(name, span);
        let width = self.argument_text(&tokens, width_span);
        let width = if width.is_empty() { token_text(&tokens).trim().to_string() } else { width };
        self.flush_paragraph(blocks, para);
        if self.beamer_columns_depth == 0 {
            self.diags.push(Diagnostic::warning(
                format!("\\{name} outside a columns environment (beamer: \"\\column outside columns environment\")"),
                Some(span),
                Some("typeset the column's text in the frame's flow".into()),
            ));
            return;
        }
        blocks.push(Block::BeamerColumn {
            width,
            align,
            span: head.merge(width_span),
        });
        self.finish_block_dependencies();
    }

    /// `\caption[short]{text}` under beamer (`beamerbaselocalstructure.sty`
    /// 570-601): steps the `figure`/`table` counter (`\refstepcounter`, so a
    /// `\label` still resolves), then `\vskip\abovecaptionskip`, the caption
    /// block, `\vskip\belowcaptionskip`. The default `caption` template
    /// shows `\insertcaptionname` + `: ` and the text, unnumbered, in
    /// `\small`. Returns `false` (and reads nothing) when no `figure`/`table`
    /// is open, so the caller's "outside figure or table" path runs.
    pub(super) fn beamer_caption(
        &mut self,
        span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) -> bool {
        let kind = self.env_stack.iter().rev().find_map(|(env, _)| match env.as_str() {
            "figure" => Some(BeamerFloatKind::Figure),
            "table" => Some(BeamerFloatKind::Table),
            _ => None,
        });
        let Some(kind) = kind else { return false };
        let _ = self.optional_bracket_argument();
        let (tokens, argument_span) = self.required_group("caption", span);
        let counter = match kind {
            BeamerFloatKind::Figure => "figure",
            BeamerFloatKind::Table => "table",
        };
        let number = self.counters.step(counter).unwrap_or_default();
        self.set_current_counter(counter, Some(number));
        self.flush_paragraph(blocks, para);
        blocks.push(Block::VSpace { pt: BEAMER_CAPTION_SKIP_PT, stretch_pt: 0.0, shrink_pt: 0.0 });
        self.finish_block_dependencies();
        // `\usebeamerfont{caption}` is `\small` for the whole template.
        let small = TextStyle { size: Some(FontSizeLevel::Small), ..self.style };
        let content = self.inlines_from_tokens(tokens, small);
        blocks.push(Block::BeamerCaption {
            kind,
            content,
            span: span.merge(argument_span),
        });
        // The `\par` of the template runs under `\small`.
        self.next_block_par_leading = Some(FontSizeLevel::Small);
        self.finish_block_dependencies();
        blocks.push(Block::VSpace { pt: BEAMER_CAPTION_SKIP_PT, stretch_pt: 0.0, shrink_pt: 0.0 });
        self.finish_block_dependencies();
        true
    }
}
