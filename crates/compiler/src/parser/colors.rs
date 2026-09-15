//! Colour commands of `color.sty` and `xcolor.sty` in the text parser.
//!
//! The colour model and pdfTeX-exact arithmetic live in `crate::color`; this
//! module reads the arguments and scopes the result: `\color` changes
//! `TextStyle::color` until the group ends (the style stack already restores
//! it, as TeX's `\aftergroup\reset@color` pops pdfTeX's colour stack),
//! `\textcolor` is `{\color{..}text}`, `\colorbox`/`\fcolorbox` build an
//! `Inline::ColorBox`, and colours inside math become
//! `Inline::Math::color_ranges`.

use super::{token_text, Block, ColorBox, Inline, InputToken, TextStyle, P};
use crate::color::{ColorError, Colors, DeviceColor};
use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::Span;

impl P<'_> {
    /// `\usepackage[options]{xcolor}` / `{color}`. Unreplayed options are
    /// reported by `package_matches_layout`'s package warning.
    pub(super) fn load_color_package(&mut self, package: &str, options: &str) {
        match package {
            "xcolor" => {
                let previous = self.colors.as_ref().filter(|c| !c.is_xcolor());
                self.colors = Some(Colors::xcolor(options, previous).0);
            }
            // After xcolor, color.sty is already "loaded" (`ver@color.sty`).
            "color" if self.colors.is_none() => self.colors = Some(Colors::color_sty(options).0),
            _ => {}
        }
    }

    /// The loaded colour package; without one the command is an error in
    /// LaTeX, and xcolor's defaults are applied so the output stays useful.
    fn colors_for(&mut self, name: &str, span: Span) -> &mut Colors {
        if self.colors.is_none() {
            self.diags.push(Diagnostic::error(
                format!("\\{name} needs \\usepackage{{xcolor}} or \\usepackage{{color}}"),
                Some(span),
                Some("used xcolor's default colours".into()),
            ));
            self.colors = Some(Colors::xcolor("", None).0);
        }
        self.colors.get_or_insert_with(|| Colors::xcolor("", None).0)
    }

    fn color_error(&mut self, name: &str, error: ColorError, span: Span, recovery: &str) {
        let message = format!("\\{name}: {error}");
        self.diags.push(match error {
            ColorError::Unsupported(_) => Diagnostic::warning(message, Some(span), Some(recovery.into())),
            _ => Diagnostic::error(message, Some(span), Some(recovery.into())),
        });
    }

    /// One required group as text.
    fn text_group(&mut self, name: &str, span: Span) -> (String, Span) {
        let (tokens, group_span) = self.required_group(name, span);
        (token_text(&tokens), group_span)
    }

    /// A colour argument resolved against `current` (`.`). An undefined
    /// colour is black, as xcolor recovers; other errors give `None`.
    fn resolve_color(
        &mut self,
        name: &str,
        model: Option<&str>,
        expression: &str,
        span: Span,
        current: Option<DeviceColor>,
    ) -> Option<DeviceColor> {
        match self.colors_for(name, span).resolve(model, expression, current) {
            Ok(color) => Some(color),
            Err(error) => {
                let undefined = matches!(error, ColorError::UndefinedColor(_));
                let recovery = if undefined { "used black" } else { "kept the current colour" };
                self.color_error(name, error, span, recovery);
                if undefined {
                    self.colors_for(name, span).resolve(None, "black", None).ok()
                } else {
                    None
                }
            }
        }
    }

    /// `\definecolor[class]{name}{model}{spec}`, `\providecolor`,
    /// `\xdefinecolor`, `\colorlet[class]{name}[model]{expression}`,
    /// `\definecolorset[class]{models}{head}{tail}{set}` and
    /// `\DefineNamedColor{class}{name}{model}{spec}`.
    pub(super) fn define_color(&mut self, name: &str, span: Span) {
        let class = if name == "DefineNamedColor" {
            None
        } else {
            self.optional_bracket_argument().map(|(class, _)| class)
        };
        let class = class.unwrap_or_default();
        let current = self.style.color;
        let (result, end) = match name {
            "colorlet" => {
                let (target, _) = self.text_group(name, span);
                let model = self.optional_bracket_argument().map(|(m, _)| m).unwrap_or_default();
                let (expression, end) = self.text_group(name, span);
                (self.colors_for(name, span).colorlet(&class, &target, &model, &expression, current), end)
            }
            "definecolorset" => {
                let (models, _) = self.text_group(name, span);
                let (head, _) = self.text_group(name, span);
                let (tail, _) = self.text_group(name, span);
                let (set, end) = self.text_group(name, span);
                (self.colors_for(name, span).define_set(&class, &models, &head, &tail, &set), end)
            }
            "DefineNamedColor" => {
                let (class, _) = self.text_group(name, span);
                let (colour, _) = self.text_group(name, span);
                let (model, _) = self.text_group(name, span);
                let (spec, end) = self.text_group(name, span);
                (self.colors_for(name, span).define(&class, &colour, &model, &spec), end)
            }
            _ => {
                let (colour, _) = self.text_group(name, span);
                let (model, _) = self.text_group(name, span);
                let (spec, end) = self.text_group(name, span);
                let colors = self.colors_for(name, span);
                let result = if name == "providecolor" {
                    colors.provide(&class, &colour, &model, &spec)
                } else {
                    colors.define(&class, &colour, &model, &spec)
                };
                (result, end)
            }
        };
        if let Err(error) = result {
            self.color_error(name, error, span.merge(end), "the colour was not defined");
        }
    }

    /// `\selectcolormodel{model}`.
    pub(super) fn select_color_model(&mut self, span: Span) {
        let (model, end) = self.text_group("selectcolormodel", span);
        if let Err(error) = self.colors_for("selectcolormodel", span).select_target(&model) {
            self.color_error("selectcolormodel", error, span.merge(end), "kept the target model");
        }
    }

    /// `[model]{colour}` of `\color`, `\textcolor`, `\pagecolor`.
    fn color_argument(&mut self, name: &str, span: Span) -> Option<DeviceColor> {
        let model = self.optional_bracket_argument().map(|(model, _)| model);
        let (expression, end) = self.text_group(name, span);
        let current = self.style.color;
        self.resolve_color(name, model.as_deref(), &expression, span.merge(end), current)
    }

    /// `\color[model]{colour}`: the rest of the group.
    pub(super) fn color_declaration(&mut self, span: Span) {
        if let Some(color) = self.color_argument("color", span) {
            self.style.color = Some(color);
        }
    }

    /// `\textcolor[model]{colour}{text}` = `{\color[model]{colour}text}`.
    pub(super) fn text_color(&mut self, span: Span, para: &mut Vec<Inline>) {
        let color = self.color_argument("textcolor", span).or(self.style.color);
        let next = TextStyle { color, ..self.style };
        self.skip_spaces();
        if let Some(open) = self.closed_group_start() {
            // Re-enter the argument as an ordinary group, like `\textbf`.
            self.i += 1;
            self.open_group(open);
            self.style = next;
        } else {
            let (tokens, _) = self.required_group("textcolor", span);
            para.extend(self.inlines_from_tokens(tokens, next));
        }
    }

    /// `\pagecolor[model]{colour}` / `\nopagecolor`, applied to every page.
    pub(super) fn page_color_command(&mut self, name: &str, span: Span) {
        if name == "nopagecolor" {
            self.page_color = None;
            return;
        }
        let Some(color) = self.color_argument(name, span) else { return };
        if self.page_color.is_some_and(|previous| previous != color) {
            self.diags.push(Diagnostic::warning(
                "\\pagecolor changed after a page colour was set",
                Some(span),
                Some("the last page colour is used for every page".into()),
            ));
        }
        self.page_color = Some(color);
    }

    /// `\colorbox[model]{fill}{text}`, `\fcolorbox[model]{frame}[model]{fill}{text}`.
    pub(super) fn color_box(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        let current = self.style.color;
        let model = self.optional_bracket_argument().map(|(model, _)| model);
        let frame = if name == "fcolorbox" {
            let (expression, end) = self.text_group(name, span);
            let frame = self.resolve_color(name, model.as_deref(), &expression, span.merge(end), current);
            Some(frame.unwrap_or(DeviceColor::BLACK))
        } else {
            None
        };
        // xcolor's `\color@fb@x`: the fill's own `[model]` defaults to the frame's.
        let fill_model = if name == "fcolorbox" {
            self.optional_bracket_argument().map(|(model, _)| model).or(model)
        } else {
            model
        };
        let (expression, end) = self.text_group(name, span);
        let fill = self
            .resolve_color(name, fill_model.as_deref(), &expression, span.merge(end), current)
            .unwrap_or(DeviceColor::BLACK);
        let (tokens, body) = self.required_group(name, span);
        let content = self.box_inlines(tokens);
        para.push(Inline::ColorBox(Box::new(ColorBox {
            fill,
            frame,
            content,
            fboxsep_pt: self.fboxsep_pt,
            fboxrule_pt: self.fboxrule_pt,
            span: span.merge(body),
            space_before,
        })));
    }

    /// A box argument parsed with the ordinary dispatch as one group in the
    /// current style (`\hbox`: restricted horizontal mode ignores `\par`).
    pub(super) fn box_inlines(&mut self, tokens: Vec<InputToken>) -> Vec<Inline> {
        let outer_tokens = std::mem::replace(&mut self.t, std::rc::Rc::new(tokens));
        let outer_index = std::mem::replace(&mut self.i, 0);
        let outer_style = self.style;
        let outer_label = self.pending_item_label.take();
        let outer_dependency_blocks = self.block_dependencies.len();
        let outer_par_leading_blocks = self.block_par_leading.len();
        let mut blocks = Vec::new();
        let mut para = Vec::new();
        self.parse_stream(&mut blocks, &mut para);
        self.flush_paragraph(&mut blocks, &mut para);
        self.block_dependencies.truncate(outer_dependency_blocks);
        self.block_par_leading.truncate(outer_par_leading_blocks);
        self.t = outer_tokens;
        self.i = outer_index;
        self.style = outer_style;
        self.pending_item_label = outer_label;
        blocks
            .into_iter()
            .flat_map(|block| match block {
                Block::Paragraph(inlines)
                | Block::Styled { content: inlines, .. }
                | Block::ListItem { content: inlines, .. } => inlines,
                _ => Vec::new(),
            })
            .collect()
    }

    /// `\color`/`\textcolor` at `index` of a flat token run: the index after
    /// the `[model]{colour}` argument and the resolved colour.
    pub(super) fn flat_color(
        &mut self,
        tokens: &[InputToken],
        index: usize,
        current: Option<DeviceColor>,
    ) -> (usize, Option<DeviceColor>) {
        let plain: Vec<Token> = tokens.iter().map(|t| t.token.clone()).collect();
        self.token_color(&plain, index, current)
    }

    fn token_color(&mut self, tokens: &[Token], index: usize, current: Option<DeviceColor>) -> (usize, Option<DeviceColor>) {
        let name = match &tokens[index].kind {
            TokenKind::Command(name) => name.clone(),
            _ => return (index + 1, None),
        };
        let span = tokens[index].span;
        let Some((next, model, expression)) = color_argument_tokens(tokens, index + 1) else {
            return (index + 1, None);
        };
        (next, self.resolve_color(&name, model.as_deref(), &expression, span, current))
    }

    /// The colour of every atom token that `\color`/`\textcolor` changed from
    /// the formula's starting colour, merged into ranges.
    pub(super) fn math_color_ranges(&mut self, raw: &[Token]) -> Vec<(Span, DeviceColor)> {
        let paints = |t: &Token| matches!(&t.kind, TokenKind::Command(n) if n == "color" || n == "textcolor");
        if !raw.iter().any(paints) {
            return Vec::new();
        }
        let base = self.style.color;
        let mut current = base;
        let mut saved: Vec<Option<DeviceColor>> = Vec::new();
        let mut pending: Option<DeviceColor> = None;
        let mut out: Vec<(Span, DeviceColor)> = Vec::new();
        let mut extend = false;
        let mut i = 0;
        while i < raw.len() {
            let token = &raw[i];
            match &token.kind {
                TokenKind::LBrace => {
                    saved.push(current);
                    if let Some(color) = pending.take() {
                        current = Some(color);
                    }
                    i += 1;
                }
                TokenKind::RBrace => {
                    if let Some(previous) = saved.pop() {
                        current = previous;
                    }
                    i += 1;
                }
                TokenKind::Command(name) if name == "color" || name == "textcolor" => {
                    let is_color = name == "color";
                    let (next, color) = self.token_color(raw, i, current);
                    i = next;
                    match color {
                        Some(color) if is_color => current = Some(color),
                        Some(color) => pending = Some(color),
                        None => {}
                    }
                }
                TokenKind::Space | TokenKind::ParBreak | TokenKind::Comment => i += 1,
                _ => {
                    match current.filter(|_| current != base) {
                        Some(color) => {
                            match out.last_mut() {
                                Some((range, c))
                                    if extend && *c == color && range.document == token.span.document =>
                                {
                                    range.end = range.end.max(token.span.end);
                                }
                                _ => out.push((token.span, color)),
                            }
                            extend = true;
                        }
                        None => extend = false,
                    }
                    i += 1;
                }
            }
        }
        out
    }
}

/// `[model]` (optional) and `{colour}` starting at `i`: the index after the
/// group, the model and the colour text.
fn color_argument_tokens(tokens: &[Token], mut i: usize) -> Option<(usize, Option<String>, String)> {
    let skip = |i: &mut usize| {
        while matches!(tokens.get(*i).map(|t| &t.kind), Some(TokenKind::Space)) {
            *i += 1;
        }
    };
    skip(&mut i);
    let mut model = None;
    if matches!(tokens.get(i).map(|t| &t.kind), Some(TokenKind::Word(w)) if w.starts_with('[')) {
        let mut raw = String::new();
        while let Some(token) = tokens.get(i) {
            i += 1;
            if let TokenKind::Word(w) = &token.kind {
                raw.push_str(w);
                if w.contains(']') {
                    break;
                }
            }
        }
        model = Some(raw.trim_start_matches('[').split(']').next().unwrap_or("").to_string());
        skip(&mut i);
    }
    if !matches!(tokens.get(i).map(|t| &t.kind), Some(TokenKind::LBrace)) {
        return None;
    }
    let mut depth = 0usize;
    let mut text = String::new();
    while let Some(token) = tokens.get(i) {
        i += 1;
        match &token.kind {
            TokenKind::LBrace => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            TokenKind::RBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, model, text));
                }
            }
            _ => {}
        }
        match &token.kind {
            TokenKind::Word(w) | TokenKind::Command(w) => text.push_str(w),
            TokenKind::Space => text.push(' '),
            _ => {}
        }
    }
    None
}
