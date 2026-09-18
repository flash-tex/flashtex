//! Colour commands of `color.sty` and `xcolor.sty` in the text parser.
//!
//! The colour model and pdfTeX-exact arithmetic live in `crate::color`; this
//! module reads the arguments and scopes the result: `\color` changes
//! `TextStyle::color` until the group ends (the style stack already restores
//! it, as TeX's `\aftergroup\reset@color` pops pdfTeX's colour stack),
//! `\textcolor` is `{\color{..}text}`, `\colorbox`/`\fcolorbox` build an
//! `Inline::ColorBox`, and colours inside math become
//! `Inline::Math::color_ranges`.

use super::{environment_end_at, environment_name_at, token_text, Block, ColorBox, Inline, InputToken, TextStyle, P};
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
            // tcolorbox.sty requires xcolor itself, so without an explicit
            // colour package the xcolor defaults apply: `colback=yellow!10`
            // resolves exactly as it does under real tcolorbox.
            "tcolorbox" if self.colors.is_none() => self.colors = Some(Colors::xcolor("", None).0),
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
            para.extend(self.inlines_from_tokens(tokens, next, false));
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
            highlight: None,
        })));
    }

    /// tcolorbox.sty's own geometry defaults (TeX Live 2026, `size=normal`,
    /// the reset value every box starts from): `boxrule=0.5mm` and
    /// `boxsep=1mm`, in TeX points. The `size=normal` extras
    /// (`left=4mm`, `right=4mm`, `top=2mm`, `bottom=2mm`), the full
    /// `\linewidth` width and the rounded corners belong to the block-level
    /// follow-up: this slice's box carries `boxsep` alone as its padding, so
    /// its content sits that much closer to the frame than real tcolorbox
    /// puts it (4mm horizontally, 2mm vertically — see `tcolorbox_environment`).
    const TCB_BOXRULE_PT: f64 = 0.5 * 72.27 / 25.4;
    /// See [`P::TCB_BOXRULE_PT`].
    const TCB_BOXSEP_PT: f64 = 72.27 / 25.4;

    /// `\begin{tcolorbox}[key=value,...] body \end{tcolorbox}` (slice 1): an
    /// `\fcolorbox` in environment form with tcolorbox's own defaults. Only
    /// `colback`/`colframe` are honoured, resolved through
    /// `self.resolve_color` exactly like `\colorbox`'s colours (tcolorbox
    /// declares both as `.colorlet`, i.e. xcolor expressions); every other
    /// key — `title` (a second region, deferred), `boxrule`, `sharp corners`,
    /// watermarks, libraries — warns once and is ignored. The box is flushed
    /// onto its own paragraph, but its content is `box_inlines`' flattened
    /// single-line run: bodies longer than one line over- rather than
    /// re-flow (the block-level follow-up), exactly like `\colorbox`.
    pub(super) fn tcolorbox_environment(
        &mut self,
        open: Span,
        argument_span: Span,
        space_before: bool,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        if !self.packages.iter().any(|package| package == "tcolorbox") {
            self.diags.push(Diagnostic::environment_warning(
                "tcolorbox",
                "\\begin{tcolorbox} needs \\usepackage{tcolorbox}",
                Some(open.merge(argument_span)),
                Some("rendered the box anyway".into()),
            ));
        }
        let begin_span = open.merge(argument_span);
        let (fill, frame) = self.tcolorbox_options(begin_span);
        // The body runs to the matching `\end{tcolorbox}`; nested boxes nest,
        // exactly like the bordered-box `frame` environment.
        let mut depth = 1usize;
        let mut cursor = self.i;
        let mut end = None;
        while cursor < self.t.len() {
            let is_begin =
                matches!(&self.t[cursor].token.kind, TokenKind::Command(name) if name == "begin");
            let is_end = !is_begin
                && matches!(&self.t[cursor].token.kind, TokenKind::Command(name) if name == "end");
            if (is_begin || is_end) && environment_name_at(&self.t, cursor) == Some("tcolorbox") {
                if is_end {
                    if depth == 1 {
                        if let Some(found) = environment_end_at(&self.t, cursor, "tcolorbox") {
                            end = Some(found);
                            break;
                        }
                    } else {
                        depth -= 1;
                    }
                } else {
                    depth += 1;
                }
            }
            cursor += 1;
        }
        let (body, end_span, after) = match end {
            Some((after, end_span)) => (self.t[self.i..cursor].to_vec(), end_span, after),
            None => {
                self.diags.push(Diagnostic::error(
                    "unterminated environment 'tcolorbox' — no matching \\end",
                    Some(open),
                    Some("boxed the rest of the input".into()),
                ));
                (self.t[self.i..].to_vec(), open, self.t.len())
            }
        };
        self.i = after;
        let content = self.box_inlines(body);
        // A display box, not an in-paragraph one: the paragraphs around it
        // close before and open after, so the box is its own paragraph.
        self.flush_paragraph(blocks, para);
        para.push(Inline::ColorBox(Box::new(ColorBox {
            fill,
            frame: Some(frame),
            content,
            fboxsep_pt: Self::TCB_BOXSEP_PT,
            fboxrule_pt: Self::TCB_BOXRULE_PT,
            span: begin_span.merge(end_span),
            space_before,
            highlight: None,
        })));
        self.flush_paragraph(blocks, para);
    }

    /// The `[key=value,...]` of `\begin{tcolorbox}`: `(colback, colframe)`.
    /// Defaults are tcolorbox.sty's own reset values (`colback=black!5!white`,
    /// `colframe=black!75!white`); anything unresolvable falls back to plain
    /// white/black, and any other key warns once and is ignored.
    fn tcolorbox_options(&mut self, span: Span) -> (DeviceColor, DeviceColor) {
        let current = self.style.color;
        let mut fill = self
            .resolve_color("tcolorbox", None, "black!5!white", span, current)
            .unwrap_or(DeviceColor::WHITE);
        let mut frame = self
            .resolve_color("tcolorbox", None, "black!75!white", span, current)
            .unwrap_or(DeviceColor::BLACK);
        let Some((options, options_span)) = self.optional_bracket_argument() else {
            return (fill, frame);
        };
        let mut unknown = Vec::new();
        for (key, value) in tcolorbox_option_pairs(&options) {
            let Some(value) = value else {
                unknown.push(key);
                continue;
            };
            match key.as_str() {
                // No `[model]`: `.colorlet` takes a bare xcolor expression.
                "colback" => {
                    fill = self
                        .resolve_color("tcolorbox", None, &value, options_span, current)
                        .unwrap_or(DeviceColor::BLACK)
                }
                "colframe" => {
                    frame = self
                        .resolve_color("tcolorbox", None, &value, options_span, current)
                        .unwrap_or(DeviceColor::BLACK)
                }
                _ => unknown.push(key),
            }
        }
        if !unknown.is_empty() {
            self.diags.push(Diagnostic::warning(
                format!(
                    "tcolorbox keys {} are not implemented; rendered the box with colback/colframe only",
                    unknown.join(", ")
                ),
                Some(options_span),
                Some("ignored the other keys".into()),
            ));
        }
        (fill, frame)
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
        // The box's paragraphs never reach `blocks`: their leadings must not
        // reach `block_par_leading` either, which carries exactly one entry
        // per pushed block (see `argument_inlines`).
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

/// The `key=value` pairs of a `\begin{tcolorbox}[...]` option list: entries
/// split at top-level commas, each split at its first top-level `=`. Braces
/// and brackets nest, so `colback=[rgb]{1,0,0}` and `title={a, b}` stay one
/// entry each, and a backslash skips the character after it — the same shape
/// as `listings_key_names` (parser.rs), which keeps names only.
fn tcolorbox_option_pairs(list: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let bytes = list.as_bytes();
    let (mut start, mut depth, mut i) = (0usize, 0i32, 0usize);
    // `entry` runs `start..i` split out of the loop's tail copy below.
    let entry = |out: &mut Vec<(String, Option<String>)>, piece: &str| {
        let piece = piece.trim();
        if piece.is_empty() {
            return;
        }
        match piece.split_once('=') {
            Some((key, value)) => {
                out.push((key.trim().to_string(), Some(value.trim().to_string())))
            }
            None => out.push((piece.to_string(), None)),
        }
    };
    while i <= bytes.len() {
        let end = i == bytes.len();
        match if end { b',' } else { bytes[i] } {
            b'{' | b'[' => depth += 1,
            b'}' | b']' if depth > 0 => depth -= 1,
            b'\\' => i += 1,
            b',' if depth == 0 => {
                entry(&mut out, &list[start..i]);
                start = i + 1;
            }
            b'=' if depth == 0 => {
                // Split the entry at its FIRST top-level `=`: the value runs
                // to the entry's end, so swallow to the closing comma here.
                let key = list[start..i].trim().to_string();
                let mut j = i + 1;
                let mut inner = depth;
                while j <= bytes.len() {
                    let done = j == bytes.len();
                    match if done { b',' } else { bytes[j] } {
                        b'{' | b'[' => inner += 1,
                        b'}' | b']' if inner > depth => inner -= 1,
                        b'\\' => j += 1,
                        b',' if inner == depth => break,
                        _ => {}
                    }
                    j += 1;
                }
                let value = list[i + 1..j].trim().to_string();
                if !key.is_empty() {
                    out.push((key, Some(value)));
                }
                start = j + 1;
                i = j;
            }
            _ => {}
        }
        i += 1;
    }
    // `start` can sit one past the end when the last entry carried a value
    // (the `=` arm consumes through the closing comma, real or implied).
    entry(&mut out, list.get(start..).unwrap_or(""));
    out
}
