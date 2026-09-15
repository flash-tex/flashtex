//! graphicx/graphics as structured inline nodes: `\includegraphics` in
//! running text and the box transforms `\scalebox`, `\resizebox`,
//! `\rotatebox` and `\reflectbox`.
//!
//! The parser records what the source asks for and never measures: an
//! image's natural size lives in the file and a transform's result depends
//! on the content box's font metrics, so a typesetter sets each node with
//! `graphics.sty`/`graphicx.sty`/`pdftex.def` semantics. Parameters keep the
//! exact source text (`0.5\textwidth`, `!`, `origin=c`, `trim=1 2 3 4`) so a
//! consumer converts dimensions exactly once, in its own context.
//!
//! `\graphicspath` is consumed without a diagnostic; its search list is
//! re-read from the source by a consumer that loads files (it also applies
//! to figures, which are not always parsed as inline nodes).

use crate::parser::Inline;
use crate::Span;

/// `\includegraphics[<keys>]{<file>}` (or the `graphics.sty` form
/// `\includegraphics[<llx>,<lly>][<urx>,<ury>]{<file>}`).
#[derive(Debug, Clone, PartialEq)]
pub struct Graphic {
    /// `\includegraphics*`: graphics.sty's clip form.
    pub starred: bool,
    /// The `graphicx` key list as written. The two-bracket bounding-box
    /// form is recorded as `viewport=<llx> <lly> <urx> <ury>`, which is what
    /// `pdftex.def`'s `\Gin@iii@vp` makes of it.
    pub options: String,
    /// The file argument as written (no extension search applied).
    pub path: String,
    /// The command through its file argument.
    pub span: Span,
    /// See `Inline::Text::space_before`.
    pub space_before: bool,
}

/// Which graphics transform a [`TransformBox`] is. Every parameter is the
/// argument's exact text.
#[derive(Debug, Clone, PartialEq)]
pub enum TransformKind {
    /// `\scalebox{<x>}[<y>]{..}` (`\Gscale@box`); `y` is `None` when the
    /// optional argument is absent (then it equals `x`).
    Scale { x: String, y: Option<String> },
    /// `\resizebox{<width>}{<height>}{..}` (`\Gscale@@box`); either may be
    /// `!`. `starred`: the height argument is the total height.
    Resize {
        starred: bool,
        width: String,
        height: String,
    },
    /// `\rotatebox[<keys>]{<angle>}{..}`: `options` is `None` without the
    /// optional argument (`\Grot@box@std`, rotation about the reference
    /// point), otherwise the `Grot` key list (`origin`, `x`, `y`, `units`).
    Rotate {
        options: Option<String>,
        angle: String,
    },
    /// `\reflectbox{..}` (`\Gscale@box-1[1]`).
    Reflect,
}

/// A graphics box transform around horizontal material.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformBox {
    pub kind: TransformKind,
    /// The material set in the inner `\hbox`.
    pub content: Vec<Inline>,
    /// The command through its last argument.
    pub span: Span,
    /// See `Inline::Text::space_before`.
    pub space_before: bool,
}

impl TransformBox {
    /// A copy with every span (and the content's) mapped; `None` when any
    /// mapping fails.
    pub fn try_map_spans(
        &self,
        span: &mut dyn FnMut(Span) -> Option<Span>,
        inlines: &mut dyn FnMut(&[Inline]) -> Option<Vec<Inline>>,
    ) -> Option<TransformBox> {
        Some(TransformBox {
            kind: self.kind.clone(),
            content: inlines(&self.content)?,
            span: span(self.span)?,
            space_before: self.space_before,
        })
    }
}

/// The directory list of a `\graphicspath{{dir1/}{dir2/}}` argument (the
/// braces' contents, in order). Text outside a brace pair is ignored, as
/// `\input@path` only ever holds brace groups.
pub fn graphics_path_entries(argument: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in argument.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    out.push(argument[start..i].to_string());
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphics_path_lists_each_group() {
        assert_eq!(
            graphics_path_entries("{figs/}{ images/png/}"),
            vec!["figs/", " images/png/"]
        );
        assert_eq!(graphics_path_entries("figs/"), Vec::<String>::new());
    }
}
