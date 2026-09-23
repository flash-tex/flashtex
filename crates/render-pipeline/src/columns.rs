//! Two-column mode as *document state*: `\twocolumn` and `\onecolumn`.
//!
//! ```tex
//! \def\twocolumn{\@restonecolfalse\if@twocolumn\@restonecoltrue\fi
//!   \clearpage \@twocolumntrue \col@number \tw@
//!   \@ifnextchar [\@topnewpage\@floatplacement}
//! \def\onecolumn{\@restonecolfalse
//!   \clearpage \@twocolumnfalse \col@number \@ne ...}
//! ```
//! (latex.ltx lines 20256-20275, TeX Live 2026.)
//!
//! Before this module the pipeline had no model of the commands at all:
//! `\if@twocolumn` was read once from `\documentclass`'s option list, so a
//! document that asks for two columns with the *command* — the usual way
//! to do it when the class options are otherwise spoken for — was set in
//! one column from beginning to end (GH#743). A fix keyed on class options
//! cannot reach that case however many sites it patches, because the class
//! options are not where the answer is; what decides it is a command the
//! document may run, and run more than once.
//!
//! So the state lives here, as the class option plus every switch the
//! document performs, in order, and every `\if@twocolumn` question is asked
//! *at a position*. The switches are the compiler's
//! `Parsed::column_switches` (PLAN1 site 37): the ones the document
//! actually ran, whatever produced them. Until then this module scanned the
//! entry source for the commands' bytes and skipped macro-definition bodies
//! by hand so that an uncalled one did not count -- which could never see
//! one a macro or a project `.sty` *did* run, and is the shape
//! [`crate::adapter`]'s `body_commands` still has.
//!
//! **What the commands change, measured.** Only `\if@twocolumn`,
//! `\col@number` and `\columnwidth`. The `twocolumn` class *option* is read
//! by `size1<n>.clo` while the class is still loading, so it also doubles
//! `\textwidth` and sets `\parindent` to `1em`, `\marginparsep` to `10pt`
//! and `\leftmargini` to `2em`; the commands run long afterwards and leave
//! all of those alone. Against pdflatex (TeX Live 2026, `article`, 10pt,
//! letter paper), `\twocolumn` in the preamble keeps the one-column text
//! block — first column at x = 133.768 bp where the option gives 72.0 —
//! and keeps `\parindent` at 15pt where the option gives 1em (9.963 bp).
//! Its two columns are at 133.768 and 310.605 bp, a step of 176.837 bp =
//! `\columnwidth` + `\columnsep` of the *one-column* `\textwidth`. That is
//! exactly [`flashtex_class_geometry::ResolvedDocument::set_twocolumn`],
//! which is why the command feeds that and not the class options.
//!
//! **What is not here yet.** The page frame is still one frame for the
//! whole document, so a switch that comes after material — measured: it
//! starts a new page, and the columns change from that page on — sets the
//! state every `\if@twocolumn` test reads, but not the column count of the
//! pages. That case is reported, not silently approximated. So is
//! `\twocolumn[<material>]` on a command that is *not* the document's
//! first material, which `\@topnewpage` cannot set either (it opens with
//! `\@nodocument`). The argument of a command that is, this module finds
//! for the pipeline to box ([`ColumnMode::top_material`]).

/// Two-column mode over one document's source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnMode {
    /// `\if@twocolumn` where the document's first material is set: the
    /// class option, then every switch that precedes that material.
    start: bool,
    /// `(byte offset of the command, `\if@twocolumn` after it)` for every
    /// switch that comes *after* the document's first material, in source
    /// order. These are the ones the page frame cannot follow yet.
    later: Vec<(usize, bool)>,
    /// Byte range of every `\twocolumn`/`\onecolumn` the entry document
    /// ran, including the ones folded into [`ColumnMode::start`]. For a
    /// command a macro produced this is the invocation, which is what the
    /// compiler's span is.
    spans: Vec<(usize, usize)>,
    /// Index into the pipeline's `texts` of the entry document: the one
    /// [`ColumnMode::later`], [`ColumnMode::spans`] and
    /// [`ColumnMode::top`]'s byte offsets index. A switch inside an
    /// `\input` file still sets [`ColumnMode::start`] when it is a preamble
    /// or first-material one, but is not laid out positionally.
    document: usize,
    /// `(byte of the `[`, byte of the `]`)` of the optional argument of a
    /// `\twocolumn` that *is* the document's first material — the only
    /// place `\@topnewpage` can run (it opens with `\@nodocument`, and a
    /// `\twocolumn` after material would have to change the column count
    /// of the pages, which is [`ColumnMode::unmodelled`]).
    top: Option<(usize, usize)>,
}

/// The `[` that `\twocolumn`'s `\@ifnextchar [` sees after the command
/// ends at `end`, if any.
///
/// `\@ifnextchar` skips space tokens, so spaces, tabs and one newline may
/// stand between; a *blank* line is a `\par`, which is not a space token
/// and does not reach the `[`.
///
/// `pub(crate)` so the `twocolumn_top_material` limitation in [`crate::adapter`]
/// applies exactly this rule instead of its own whitespace test.
pub(crate) fn optional_bracket(source: &str, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = end;
    let mut newlines = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                newlines += 1;
                if newlines > 1 {
                    return None;
                }
                i += 1;
            }
            b' ' | b'\t' | b'\r' => i += 1,
            b'[' => return Some(i),
            _ => return None,
        }
    }
    None
}

/// The `]` that ends `\long\def\@topnewpage[#1]`'s delimited argument: the
/// first one outside braces and outside comments. A nested `[` does not
/// count — TeX matches a delimited parameter against the delimiter text
/// alone, at brace level zero.
fn closing_bracket(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = open + 1;
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            // A control symbol is one character long, so `\]`, `\%`, `\{`
            // and `\\` never delimit, comment or nest.
            b'\\' => i += 2,
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            b']' if depth == 0 => return Some(i),
            _ => i += 1,
        }
    }
    None
}

impl ColumnMode {
    /// The mode of the document the compiler parsed, under a class whose
    /// `twocolumn` option is `class_option` (PLAN1 site 37).
    ///
    /// `switches` is `Parsed::column_switches`: every `\twocolumn` /
    /// `\onecolumn` the document actually *ran*, in execution order, with
    /// the compiler's own answers to the two questions this used to decide
    /// from bytes -- whether the command stood before `\begin{document}`,
    /// and whether it was the document's first material. This module used
    /// to scan the entry source for the commands and skip macro-definition
    /// bodies by hand so an uncalled one did not count; the node stream
    /// reports the opposite direction instead, which is the correct one: a
    /// switch a macro or a project `.sty` performed counts, and a body that
    /// never runs is simply absent.
    ///
    /// `document` is the entry document. Only its switches are positional
    /// here, because everything downstream (the block ranges the page
    /// builder matches, `optional_bracket`) is entry-relative; one inside
    /// an `\input` file still sets the document's starting state when it is
    /// a preamble or first-material switch, and is otherwise not laid out
    /// -- the same limitation the scan had, which never read those files at
    /// all.
    pub fn from_switches(
        texts: &[&str],
        switches: &[flashtex_compiler::parser::ColumnSwitch],
        document: usize,
        class_option: bool,
    ) -> ColumnMode {
        let mut mode = ColumnMode {
            start: class_option,
            later: Vec::new(),
            spans: Vec::new(),
            document,
            top: None,
        };
        for switch in switches {
            let here = (switch.span.document.0 == document).then(|| (switch.span.start, switch.span.end));
            if let Some(span) = here {
                mode.spans.push(span);
            }
            if switch.preamble || switch.first_material {
                mode.start = switch.two;
                // `\@topnewpage` runs `\@nodocument` first, so only a
                // `\twocolumn` that is itself the document's first material
                // can carry the box; one in the preamble is an error in
                // LaTeX, and one after material is `unmodelled`.
                //
                // The bracket is still read from the bytes after the
                // command, so it is found only for a `\twocolumn` written
                // literally: the compiler leaves `[<material>]`'s tokens in
                // the paragraph at the invocation's span, and for a
                // macro-produced command that span is the call, whose
                // following bytes are the call's own. Such a `\twocolumn`
                // keeps the mode switch and loses only the box, which this
                // pipeline reports (`twocolumn_top_material`). Moving the
                // material itself into the node stream is a separate site.
                if switch.two && switch.first_material {
                    mode.top = here
                        .filter(|&(start, end)| texts.get(document).and_then(|t| t.get(start..end)) == Some("\\twocolumn"))
                        .and_then(|(_, end)| {
                            let source = texts[document];
                            optional_bracket(source, end).and_then(|open| closing_bracket(source, open).map(|close| (open, close)))
                        });
                }
            } else if let Some((at, _)) = here {
                mode.later.push((at, switch.two));
            }
        }
        mode
    }

    /// The byte offsets of the `[` and `]` around `\twocolumn`'s optional
    /// argument, when the command is the document's first material.
    /// `\@topnewpage` sets what lies between them in a `\textwidth` box
    /// above both columns of the page the command starts.
    pub fn top_material(&self) -> Option<(usize, usize)> {
        self.top
    }

    /// Byte ranges of the `\twocolumn`/`\onecolumn` commands themselves.
    pub fn spans(&self) -> &[(usize, usize)] {
        &self.spans
    }

    /// `\if@twocolumn` where the document's first material is set. This is
    /// the one the page frame follows
    /// ([`flashtex_class_geometry::ResolvedDocument::set_twocolumn`]).
    pub fn start(&self) -> bool {
        self.start
    }

    /// The document the switches were read from.
    pub fn document(&self) -> usize {
        self.document
    }

    /// `\if@twocolumn` at byte offset `pos` of document `document`. A
    /// position in an `\input` file, whose switches are not scanned, reads
    /// the document's starting state.
    pub fn at(&self, document: usize, pos: usize) -> bool {
        if document != self.document {
            return self.start;
        }
        self.later
            .iter()
            .take_while(|(at, _)| *at < pos)
            .last()
            .map_or(self.start, |(_, on)| *on)
    }

    /// The switches the page frame cannot follow yet: those after the first
    /// material that actually change the column count from what the frame
    /// was built with.
    pub fn unmodelled(&self) -> Vec<(usize, bool)> {
        let mut state = self.start;
        let mut out = Vec::new();
        for &(at, on) in &self.later {
            if on != state {
                out.push((at, on));
            }
            state = on;
        }
        out
    }
}

/// A single post-material switch the page builder lays out: the index of
/// the first document block after the switch (`adapter`'s block list), the
/// switch's own byte offset (so the limitation pass can tell it apart from
/// the switches that are still reported), and `\if@twocolumn` after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnSwitch {
    /// Index into `adapter::Doc::blocks` of the first block after the
    /// switch.
    pub block: usize,
    /// Byte offset of the `\twocolumn`/`\onecolumn` command itself.
    pub at: usize,
    /// `\if@twocolumn` after the switch: the column state of every page
    /// from [`ColumnSwitch::block`] to the end of the document.
    pub on: bool,
}

/// The `twocolumn_mid_document` limitation text for a switch to `on` that
/// the frame (built with starting state `start_two`) cannot follow: the
/// rest of the document keeps whatever column count it already had.
///
/// One shared helper so the adapter pass (which reports every switch the
/// page builder does not lay out) and the page builder itself (which
/// reports the recorded switch back when it has to decline it) cannot
/// drift apart.
pub fn mid_document_message(on: bool, start_two: bool) -> String {
    format!(
        "\\{} after the first material starts a new page, but changing the number of \
         page columns during a document is not implemented: the rest of the document \
         keeps {} column(s)",
        if on { "twocolumn" } else { "onecolumn" },
        if start_two { 2 } else { 1 },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode the compiler's own switch list gives for `source`, as the
    /// adapter builds it.
    fn scan_test(source: &str, class_option: bool) -> ColumnMode {
        let parsed = flashtex_compiler::parser::parse(source);
        ColumnMode::from_switches(&[source], &parsed.column_switches, 0, class_option)
    }

    #[test]
    fn a_preamble_command_is_the_documents_column_state() {
        let m = scan_test("\\documentclass{article}\n\\twocolumn\n\\begin{document}x\\end{document}", false);
        assert!(m.start());
        assert!(m.unmodelled().is_empty());
    }

    #[test]
    fn a_command_before_any_material_is_too() {
        // pdflatex sets these two documents identically.
        let m = scan_test("\\documentclass{article}\n\\begin{document}\n% hi\n  \\twocolumn\nx\\end{document}", false);
        assert!(m.start());
        assert!(m.unmodelled().is_empty());
    }

    #[test]
    fn a_command_after_material_is_positional_and_reported() {
        let s = "\\documentclass{article}\n\\begin{document}\nA\n\\twocolumn\nB\\end{document}";
        let m = scan_test(s, false);
        assert!(!m.start());
        let at = s.find("\\twocolumn").unwrap();
        assert!(!m.at(0, at));
        assert!(m.at(0, at + 1));
        assert_eq!(m.unmodelled(), vec![(at, true)]);
    }

    #[test]
    fn onecolumn_undoes_the_class_option() {
        let m = scan_test("\\documentclass[twocolumn]{article}\n\\onecolumn\n\\begin{document}x\\end{document}", true);
        assert!(!m.start());
    }

    #[test]
    fn a_switch_that_changes_nothing_is_not_reported() {
        let s = "\\documentclass[twocolumn]{article}\n\\begin{document}\nA\n\\twocolumn\nB\\end{document}";
        assert!(scan_test(s, true).unmodelled().is_empty());
    }

    #[test]
    fn comments_and_longer_names_are_not_switches() {
        let m = scan_test("\\documentclass{article}\n% \\twocolumn\n\\twocolumnfoo\n\\begin{document}x\\end{document}", false);
        assert!(!m.start());
        assert!(m.unmodelled().is_empty());
    }

    #[test]
    fn the_first_materials_twocolumn_carries_the_topnewpage_box() {
        let src = "\\begin{document}\n\\twocolumn[Banner]\nAaa\n\\end{document}\n";
        let m = scan_test(src, false);
        let (open, close) = m.top_material().expect("an optional argument");
        assert_eq!(&src[open..=close], "[Banner]");
    }

    #[test]
    fn the_argument_ends_at_the_first_bracket_outside_braces() {
        // TeX matches `\@topnewpage`'s delimited parameter at brace level
        // zero, so the `]` inside the group does not end it.
        let src = "\\begin{document}\n\\twocolumn[{\\Large a]b} c]\nAaa\n\\end{document}\n";
        let m = scan_test(src, false);
        let (open, close) = m.top_material().unwrap();
        assert_eq!(&src[open..=close], "[{\\Large a]b} c]");
    }

    #[test]
    fn a_preamble_twocolumn_carries_no_box() {
        // `\@topnewpage` runs `\@nodocument` first: this is a LaTeX error,
        // not a banner.
        let m = scan_test("\\twocolumn[Banner]\n\\begin{document}\nAaa\n\\end{document}\n", false);
        assert!(m.start());
        assert_eq!(m.top_material(), None);
    }

    #[test]
    fn a_twocolumn_after_material_carries_no_box() {
        let m = scan_test("\\begin{document}\nAaa\n\n\\twocolumn[Banner]\nBbb\n\\end{document}\n", false);
        assert_eq!(m.top_material(), None);
        assert_eq!(m.unmodelled(), vec![(22, true)]);
    }

    #[test]
    fn a_blank_line_before_the_bracket_is_a_par_and_not_an_argument() {
        // `\@ifnextchar [` skips space tokens; a blank line is a `\par`.
        let m = scan_test("\\begin{document}\n\\twocolumn\n\n[Banner]\nAaa\n\\end{document}\n", false);
        assert_eq!(m.top_material(), None);
    }

    #[test]
    fn switches_inside_uninvoked_macro_bodies_do_not_move_the_mode() {
        // Neither macro is ever invoked: real pdflatex keeps the class's
        // own column count in every case, since the body never runs.
        for class_option in [false, true] {
            for (def, switch, word) in [
                ("newcommand", "onecolumn", "wide"),
                ("newcommand", "twocolumn", "narrow"),
                ("renewcommand", "onecolumn", "wide"),
                ("renewcommand", "twocolumn", "narrow"),
                ("providecommand", "onecolumn", "wide"),
                ("providecommand", "twocolumn", "narrow"),
            ] {
                let src = format!(
                    "\\documentclass{options}{{article}}\n\\{def}{{\\{word}}}{{\\{switch}}}\n\\begin{{document}}x\\end{{document}}",
                    options = if class_option { "[twocolumn]" } else { "" },
                );
                let m = scan_test(&src, class_option);
                assert_eq!(m.start(), class_option, "{src}");
                assert!(m.unmodelled().is_empty(), "{src}");
            }
        }
    }

    #[test]
    fn def_and_newenvironment_bodies_do_not_move_the_mode_either() {
        for (class_option, src) in [
            (false, "\\documentclass{article}\n\\def\\wide{\\onecolumn}\n\\begin{document}x\\end{document}"),
            (false, "\\documentclass{article}\n\\def\\narrow{\\twocolumn}\n\\begin{document}x\\end{document}"),
            (true, "\\documentclass[twocolumn]{article}\n\\newenvironment{wide}{\\onecolumn}{}\n\\begin{document}x\\end{document}"),
            (true, "\\documentclass[twocolumn]{article}\n\\renewenvironment{narrow}{\\twocolumn}{}\n\\begin{document}x\\end{document}"),
            (false, "\\documentclass{article}\n\\newenvironment{narrow}{\\twocolumn}{\\onecolumn}\n\\begin{document}x\\end{document}"),
        ] {
            let m = scan_test(src, class_option);
            assert_eq!(m.start(), class_option, "{src}");
            assert!(m.unmodelled().is_empty(), "{src}");
        }
    }

    #[test]
    fn let_and_gdef_family_bodies_do_not_move_the_mode() {
        for (class_option, src) in [
            // `\let\name=\target` and `\let\name\target`, both directions.
            (false, "\\documentclass{article}\n\\let\\oldtwocolumn=\\twocolumn\n\\begin{document}x\\end{document}"),
            (false, "\\documentclass{article}\n\\let\\oldtwocolumn\\twocolumn\n\\begin{document}x\\end{document}"),
            (true, "\\documentclass[twocolumn]{article}\n\\let\\oldonecolumn\\onecolumn\n\\begin{document}x\\end{document}"),
            // `\global\let`: `\global` itself is not a recognised name, so
            // the scan reaches the following `\let` on its own.
            (false, "\\documentclass{article}\n\\global\\let\\x\\twocolumn\n\\begin{document}x\\end{document}"),
            // `\gdef`/`\edef`/`\xdef` share `\def`'s shape.
            (false, "\\documentclass{article}\n\\gdef\\wide{\\onecolumn}\n\\begin{document}x\\end{document}"),
            (false, "\\documentclass{article}\n\\edef\\narrow{\\twocolumn}\n\\begin{document}x\\end{document}"),
            (true, "\\documentclass[twocolumn]{article}\n\\xdef\\one{\\onecolumn}\n\\begin{document}x\\end{document}"),
            // `\DeclareRobustCommand` takes `\newcommand`'s own shape.
            (false, "\\documentclass{article}\n\\DeclareRobustCommand{\\wide}{\\onecolumn}\n\\begin{document}x\\end{document}"),
            (false, "\\documentclass{article}\n\\DeclareRobustCommand\\narrow{\\twocolumn}\n\\begin{document}x\\end{document}"),
            // A brace-less `\newcommand` body: a single control sequence,
            // not a `{...}` group.
            (false, "\\documentclass{article}\n\\newcommand\\n\\twocolumn\n\\begin{document}x\\end{document}"),
        ] {
            let m = scan_test(src, class_option);
            assert_eq!(m.start(), class_option, "{src}");
            assert!(m.unmodelled().is_empty(), "{src}");
        }
    }

    #[test]
    fn at_names_in_makeatletter_definitions_do_not_switch() {
        // GH#801: under `\makeatletter`, `@` is a letter, so the defined
        // name is `\@oldtc` / `\@x` whole — the `\twocolumn` they alias is
        // never invoked and must not move the mode.
        for (class_option, src) in [
            (
                false,
                "\\documentclass{article}\n\\makeatletter\\let\\@oldtc\\twocolumn\\makeatother\n\\begin{document}x\\end{document}",
            ),
            (
                false,
                "\\documentclass{article}\n\\makeatletter\\newcommand\\@x\\twocolumn\\makeatother\n\\begin{document}x\\end{document}",
            ),
            (
                true,
                "\\documentclass[twocolumn]{article}\n\\makeatletter\\let\\@oldtc\\onecolumn\\makeatother\n\\begin{document}x\\end{document}",
            ),
            (
                false,
                "\\documentclass{article}\n\\makeatletter\\def\\@narrow{\\twocolumn}\\makeatother\n\\begin{document}x\\end{document}",
            ),
        ] {
            let m = scan_test(src, class_option);
            assert_eq!(m.start(), class_option, "{src}");
            assert!(m.unmodelled().is_empty(), "{src}");
            assert!(m.spans().is_empty(), "{src}");
        }
    }

    #[test]
    fn at_outside_makeatletter_does_not_swallow_a_switch() {
        // Negative control: outside `\makeatletter`, `@` is not a letter,
        // so `\let\@oldtc\twocolumn` is invalid TeX (pdflatex itself
        // errors) and the scanner must not treat `\@oldtc` as one defined
        // name that hides the `\twocolumn`. Existing behavior counts the
        // leaked switch; this test pins that it keeps doing so rather than
        // silently skipping a real invocation.
        let src = "\\documentclass{article}\n\\let\\@oldtc\\twocolumn\n\\begin{document}x\\end{document}";
        let m = scan_test(src, false);
        assert!(m.start(), "{src}");
        assert!(!m.spans().is_empty(), "{src}");
    }

    #[test]
    fn makeatother_closes_the_at_region() {
        // After `\makeatother`, `@` stops being a letter: a later invoked
        // `\twocolumn` still switches, and a later `\let\@x...` no longer
        // hides one.
        let src = "\\documentclass{article}\n\\makeatletter\\let\\@a\\onecolumn\\makeatother\n\\twocolumn\n\\begin{document}x\\end{document}";
        let m = scan_test(src, false);
        assert!(m.start(), "{src}");
        let src = "\\documentclass{article}\n\\makeatletter\\makeatother\\let\\@oldtc\\twocolumn\n\\begin{document}x\\end{document}";
        let m = scan_test(src, false);
        assert!(m.start(), "{src}");
    }

    #[test]
    fn onecolumn_takes_no_optional_argument() {
        let m = scan_test("\\begin{document}\n\\onecolumn[Banner]\nAaa\n\\end{document}\n", true);
        assert!(!m.start());
        assert_eq!(m.top_material(), None);
    }

}
