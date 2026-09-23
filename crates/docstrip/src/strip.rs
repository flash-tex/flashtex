//! Stripping one `.dtx` source into the output files that read it
//! (docstrip.dtx §"Processing the input lines", lines 2321–2495, §"The
//! handling of options", lines 2496–2870, and `\readsource`, lines
//! 3211–3380).
//!
//! docstrip reads a source with `\read` after making every special
//! character *other* and `\endlinechar=-1` (lines 3277–3281), so a line
//! is its bytes with two exceptions inherited from TeX's reader: trailing
//! spaces are gone, and a tab (still category 10 under plain TeX) is a
//! blank — dropped at the start of a line, one space in the middle of one
//! however many tabs there are (a trailing tab included). Then, per line (`\processLine`, lines
//! 2413–2495):
//!
//! - `%%…` is a meta-comment: `\MetaPrefix` + the rest, to every active
//!   output (`\putMetaComment`, line 2394);
//! - `%<…` is a guard line (`\checkOption`, line 2517): `%<*expr>` opens a
//!   block, `%</expr>` closes one, `%<expr>` and `%<+expr>` include the
//!   rest of the line where `expr` holds, `%<-expr>` where it does not,
//!   `%<<TAG` copies lines verbatim up to a line `%TAG`, `%<@@=name>`
//!   sets the module name that replaces `@@`;
//! - any other `%…` line is a comment and is dropped;
//! - everything else is code, copied to every active output after the
//!   `@@` replacement.
//!
//! A block that is off for an output leaves every nested guard
//! unevaluated (`\checkguard@do`, line 2637). At most one of a run of
//! empty lines is kept (`\emptyLines`, line 3326), a counter that
//! docstrip never resets between sources. A line that is exactly
//! `\endinput` ends the source (line 3305).

use crate::guard;
use crate::Diagnostic;

/// docstrip state that outlives one source: the module name
/// (`\replaceModuleInLine`, reset by `\generate`'s group), the block
/// nesting (`\blockLevel`, `\blockHead`, `\guardStack`) and the
/// empty-line counter (`\emptyLines`), which are count registers and
/// macros docstrip never resets.
#[derive(Default, Debug, Clone)]
pub struct State {
    pub module: Option<Vec<u8>>,
    pub block_level: i64,
    pub block_head: Vec<u8>,
    pub guard_stack: Vec<Vec<u8>>,
    pub empty_lines: u32,
}

/// One output reading this source: its index into the outputs and the
/// option list of its `\from`.
pub struct Consumer {
    pub output: usize,
    pub options: Vec<u8>,
}

/// Runs one source through the line processor, appending to
/// `outputs[c.output]` for each consumer. `meta_prefix` is `\MetaPrefix`
/// as it expands while this `\generate` runs.
#[allow(clippy::too_many_arguments)]
pub fn process_source(
    source: &str,
    bytes: &[u8],
    consumers: &[Consumer],
    outputs: &mut [Vec<u8>],
    meta_prefix: &[u8],
    state: &mut State,
    diagnostics: &mut Vec<Diagnostic>,
    messages: &mut Vec<String>,
) {
    let lines = crate::lexer::split_lines(bytes);
    // Off-counters start at zero when the output is opened for this
    // source (`\showfiles@do`, line 3402): every consumer is active.
    let mut off: Vec<u32> = vec![0; consumers.len()];
    let mut i = 0;
    let diag = |diagnostics: &mut Vec<Diagnostic>, line: usize, message: String| diagnostics.push(Diagnostic { file: source.to_string(), line, message });
    while i < lines.len() {
        let line_no = i + 1;
        let line = read_line(lines[i]);
        i += 1;
        if line == b"\\endinput" {
            messages.push(format!("File {source} ended by \\endinput."));
            break;
        }
        if line.is_empty() {
            state.empty_lines += 1;
        } else {
            state.empty_lines = 0;
        }
        if state.empty_lines >= 2 {
            continue;
        }
        if line.first() != Some(&b'%') {
            let text = replace_module(&line, state.module.as_deref());
            put_active(consumers, &off, outputs, &text);
            continue;
        }
        match line.get(1) {
            Some(b'%') => {
                let mut text = meta_prefix.to_vec();
                text.extend_from_slice(&line[2..]);
                put_active(consumers, &off, outputs, &text);
            }
            Some(b'<') => {
                let rest = &line[2..];
                match rest.first() {
                    Some(b'*') => {
                        // \starOption (line 2591)
                        let Some((expr, _)) = split_guard(&rest[1..]) else {
                            diag(diagnostics, line_no, "guard line without a closing `>`".into());
                            continue;
                        };
                        let head = std::mem::replace(&mut state.block_head, expr.to_vec());
                        state.guard_stack.push(head);
                        state.block_level += 1;
                        let parsed = guard::parse(expr);
                        if let Err(e) = &parsed {
                            diag(diagnostics, line_no, format!("{e} in `%<*{}>`", String::from_utf8_lossy(expr)));
                        }
                        // \checkguard@do (line 2637): a block inside an off block
                        // is off without its guard being evaluated.
                        for (k, c) in consumers.iter().enumerate() {
                            let on = off[k] == 0 && parsed.as_ref().map(|e| guard::eval(e, &c.options)).unwrap_or(false);
                            if !on {
                                off[k] += 1;
                            }
                        }
                    }
                    Some(b'/') => {
                        // \slashOption (line 2679)
                        let Some((expr, _)) = split_guard(&rest[1..]) else {
                            diag(diagnostics, line_no, "guard line without a closing `>`".into());
                            continue;
                        };
                        if state.block_level < 1 {
                            diag(diagnostics, line_no, format!("Spurious end block </{}> ignored", String::from_utf8_lossy(expr)));
                            continue;
                        }
                        if expr == state.block_head.as_slice() {
                            state.block_head = state.guard_stack.pop().unwrap_or_default();
                        } else {
                            diag(diagnostics, line_no, format!("Found </{}> instead of </{}>", String::from_utf8_lossy(expr), String::from_utf8_lossy(&state.block_head)));
                        }
                        state.block_level -= 1;
                        for o in off.iter_mut() {
                            if *o > 0 {
                                *o -= 1;
                            }
                        }
                    }
                    Some(b'+') | Some(b'-') => {
                        // \plusOption / \minusOption (lines 2562, 2578)
                        let negate = rest[0] == b'-';
                        let Some((expr, text)) = split_guard(&rest[1..]) else {
                            diag(diagnostics, line_no, "guard line without a closing `>`".into());
                            continue;
                        };
                        match guard::parse(expr) {
                            Ok(e) => {
                                let text = replace_module(text, state.module.as_deref());
                                for (k, c) in consumers.iter().enumerate() {
                                    if off[k] == 0 && guard::eval(&e, &c.options) != negate {
                                        put(&mut outputs[c.output], &text);
                                    }
                                }
                            }
                            Err(e) => diag(diagnostics, line_no, format!("{e} in `%<{}{}>`", rest[0] as char, String::from_utf8_lossy(expr))),
                        }
                    }
                    Some(b'<') => {
                        // \verbOption (line 2740): copy up to a line `%TAG` unchanged.
                        let mut stop = vec![b'%'];
                        stop.extend_from_slice(&rest[1..]);
                        loop {
                            let Some(raw) = lines.get(i) else {
                                diag(diagnostics, line_no, "Source file ended while in verbatim mode!".into());
                                break;
                            };
                            let l = read_line(raw);
                            i += 1;
                            if l == stop {
                                break;
                            }
                            put_active(consumers, &off, outputs, &l);
                        }
                    }
                    Some(b'@') => {
                        // \moduleOption (line 2768): the syntax is fixed to `%<@@=name>`.
                        match rest.strip_prefix(b"@@=").and_then(split_guard) {
                            Some((name, _)) => state.module = if name.is_empty() { None } else { Some(name.to_vec()) },
                            None => diag(diagnostics, line_no, format!("`%<@…` is only valid as `%<@@=name>`: {}", String::from_utf8_lossy(&line))),
                        }
                    }
                    _ => {
                        // \doOption (line 2543)
                        let Some((expr, text)) = split_guard(rest) else {
                            diag(diagnostics, line_no, "guard line without a closing `>`".into());
                            continue;
                        };
                        match guard::parse(expr) {
                            Ok(e) => {
                                let text = replace_module(text, state.module.as_deref());
                                for (k, c) in consumers.iter().enumerate() {
                                    if off[k] == 0 && guard::eval(&e, &c.options) {
                                        put(&mut outputs[c.output], &text);
                                    }
                                }
                            }
                            Err(e) => diag(diagnostics, line_no, format!("{e} in `%<{}>`", String::from_utf8_lossy(expr))),
                        }
                    }
                }
            }
            _ => {} // \removeComment
        }
    }
}

/// `expr>rest` → `(expr, rest)` at the first `>`.
fn split_guard(text: &[u8]) -> Option<(&[u8], &[u8])> {
    let at = text.iter().position(|&b| b == b'>')?;
    Some((&text[..at], &text[at + 1..]))
}

fn put(out: &mut Vec<u8>, line: &[u8]) {
    out.extend_from_slice(line);
    out.push(b'\n');
}

fn put_active(consumers: &[Consumer], off: &[u32], outputs: &mut [Vec<u8>], line: &[u8]) {
    for (k, c) in consumers.iter().enumerate() {
        if off[k] == 0 {
            put(&mut outputs[c.output], line);
        }
    }
}

/// A source line as `\read` tokenizes it under docstrip's category
/// codes: every byte is itself except a tab, which is a blank (category
/// 10 under plain TeX, not among the characters `\readsource` makes
/// other): ignored at the start of a line or after another blank, one
/// space otherwise (The TeXbook p. 47, states N/M/S). Ignored (0) and
/// invalid (127) characters vanish. `line` has no trailing spaces.
pub fn read_line(line: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len());
    // false: the start of the line or skipping blanks; true: mid-line.
    let mut mid = false;
    for &b in line {
        match b {
            b'\t' => {
                if mid {
                    out.push(b' ');
                    mid = false;
                }
            }
            0 | 127 => {}
            _ => {
                out.push(b);
                mid = true;
            }
        }
    }
    out
}

/// The `@@` → module replacement (docstrip.dtx lines 790–863 and
/// `\prepareActiveModule`, lines 2790–2807), in docstrip's order: `@@@@`
/// is set aside, then `__@@`, `_@@` and `@@` in turn become `__module`,
/// then the set-aside pairs become `@@`. Each pass replaces the leftmost
/// occurrence and continues after it (`\replaceAllIn`, line 2854). With
/// no module (never set, or `%<@@=>`) the line is untouched.
pub fn replace_module(line: &[u8], module: Option<&[u8]>) -> Vec<u8> {
    let Some(module) = module else { return line.to_vec() };
    let mut replacement = b"__".to_vec();
    replacement.extend_from_slice(module);
    // Pass 1: protect `@@@@` (`\string aa` in docstrip).
    let mut pieces: Vec<Option<Vec<u8>>> = Vec::new(); // None = a protected `@@`
    let mut text = Vec::new();
    let mut i = 0;
    while i < line.len() {
        if line[i..].starts_with(b"@@@@") {
            pieces.push(Some(std::mem::take(&mut text)));
            pieces.push(None);
            i += 4;
        } else {
            text.push(line[i]);
            i += 1;
        }
    }
    pieces.push(Some(text));
    let mut out = Vec::with_capacity(line.len());
    for piece in pieces {
        match piece {
            None => out.extend_from_slice(b"@@"),
            Some(t) => {
                let t = replace_all(&t, b"__@@", &replacement);
                let t = replace_all(&t, b"_@@", &replacement);
                let t = replace_all(&t, b"@@", &replacement);
                out.extend_from_slice(&t);
            }
        }
    }
    out
}

fn replace_all(text: &[u8], pattern: &[u8], with: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(pattern) {
            out.extend_from_slice(with);
            i += pattern.len();
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(src: &str, options: &[&str]) -> (Vec<String>, Vec<Diagnostic>) {
        let consumers: Vec<Consumer> = options.iter().enumerate().map(|(i, o)| Consumer { output: i, options: o.as_bytes().to_vec() }).collect();
        let mut outputs = vec![Vec::new(); options.len()];
        let mut state = State::default();
        let mut diagnostics = Vec::new();
        let mut messages = Vec::new();
        process_source("t.dtx", src.as_bytes(), &consumers, &mut outputs, b"%%", &mut state, &mut diagnostics, &mut messages);
        (outputs.into_iter().map(|o| String::from_utf8(o).unwrap()).collect(), diagnostics)
    }

    fn one(src: &str, options: &str) -> String {
        strip(src, &[options]).0.remove(0)
    }

    #[test]
    fn comments_go_meta_comments_stay_code_passes() {
        let src = "% \\iffalse meta-comment\n%% keep me\n%    \\begin{macrocode}\n\\def\\x{1}\n%    \\end{macrocode}\n";
        assert_eq!(one(src, "package"), "%% keep me\n\\def\\x{1}\n");
    }

    #[test]
    fn blocks_single_lines_and_modifiers() {
        let src = "%<*package>\n\\a\n%<*ltx>\n\\b\n%</ltx>\n%<ltx>\\c\n%<+ltx>\\d\n%<-ltx>\\e\n%</package>\n%<driver>\\f\n\\g\n";
        assert_eq!(one(src, "package"), "\\a\n\\e\n\\g\n");
        assert_eq!(one(src, "package,ltx"), "\\a\n\\b\n\\c\n\\d\n\\g\n");
        assert_eq!(one(src, "driver"), "\\f\n\\g\n");
        let (outs, diags) = strip(src, &["package", "package,ltx", "driver"]);
        assert_eq!(outs, ["\\a\n\\e\n\\g\n", "\\a\n\\b\n\\c\n\\d\n\\g\n", "\\f\n\\g\n"]);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn guards_inside_an_off_block_are_not_evaluated() {
        // Inside an off block a nested block's guard is not evaluated
        // against the options; it is still parsed (`\Evaluate` runs
        // unconditionally in `\starOption`), so a malformed one is reported.
        let src = "%<*no>\n%<*yes>\nx\n%</yes>\n%</no>\ny\n";
        let (outs, diags) = strip(src, &["yes"]);
        assert_eq!(outs[0], "y\n");
        assert!(diags.is_empty(), "{diags:?}");
        let (_, diags) = strip("%<*no>\n%<*bogus&>\nx\n%</bogus&>\n%</no>\ny\n", &["yes"]);
        assert_eq!(diags.len(), 1, "{diags:?}");
        // At level zero a malformed guard is reported and counts as false.
        let (outs, diags) = strip("%<*bogus&>\nx\n%</bogus&>\ny\n", &["bogus"]);
        assert_eq!(outs[0], "y\n");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("empty terminal"), "{}", diags[0].message);
        assert_eq!((diags[0].file.as_str(), diags[0].line), ("t.dtx", 1));
    }

    #[test]
    fn mismatched_and_spurious_block_ends() {
        let (outs, diags) = strip("%<*a>\nx\n%</b>\ny\n%</a>\nz\n", &["a"]);
        assert_eq!(outs[0], "x\ny\nz\n");
        assert_eq!(diags.len(), 2, "{diags:?}");
        assert_eq!(diags[0].message, "Found </b> instead of </a>");
        assert_eq!(diags[1].message, "Spurious end block </a> ignored");
    }

    #[test]
    fn empty_lines_collapse_and_endinput_stops() {
        assert_eq!(one("a\n\n\n\nb\n\n", "x"), "a\n\nb\n\n");
        assert_eq!(one("a\n\\endinput\nb\n", "x"), "a\n");
        assert_eq!(one("a\n\\endinput  \nb\n", "x"), "a\n", "trailing spaces are stripped before the comparison");
        assert_eq!(one("a\n\\endinput b\nb\n", "x"), "a\n\\endinput b\nb\n");
        // The counter carries across sources.
        let consumers = [Consumer { output: 0, options: b"x".to_vec() }];
        let mut outputs = vec![Vec::new()];
        let mut state = State::default();
        let (mut d, mut m) = (Vec::new(), Vec::new());
        process_source("a.dtx", b"a\n\n", &consumers, &mut outputs, b"%%", &mut state, &mut d, &mut m);
        process_source("b.dtx", b"\nb\n", &consumers, &mut outputs, b"%%", &mut state, &mut d, &mut m);
        assert_eq!(String::from_utf8(outputs.remove(0)).unwrap(), "a\n\nb\n");
    }

    #[test]
    fn verbatim_mode_copies_percent_lines() {
        let src = "%<*m>\ncode\n%<<END\n% verbatim\n%% also\n%<not a guard>\n%END\nafter\n%</m>\n";
        assert_eq!(one(src, "m"), "code\n% verbatim\n%% also\n%<not a guard>\nafter\n");
        let (_, diags) = strip("%<<X\nnever closed\n", &["m"]);
        assert_eq!(diags[0].message, "Source file ended while in verbatim mode!");
    }

    #[test]
    fn meta_prefix_and_module_replacement() {
        let consumers = [Consumer { output: 0, options: b"p".to_vec() }];
        let mut outputs = vec![Vec::new()];
        let mut state = State::default();
        let (mut d, mut m) = (Vec::new(), Vec::new());
        process_source("a.dtx", b"%% x\n%<@@=foo>\n\\cs_new:Npn \\@@_f: { \\l__@@_tl \\l_@@_x @@@@ @@@@@ }\n%<p>\\__@@_g:\n%<@@=>\n\\@@_h:\n", &consumers, &mut outputs, b"--", &mut state, &mut d, &mut m);
        assert_eq!(String::from_utf8(outputs.remove(0)).unwrap(), "-- x\n\\cs_new:Npn \\__foo_f: { \\l__foo_tl \\l__foo_x @@ @@@ }\n\\__foo_g:\n\\@@_h:\n");
        assert_eq!(replace_module(b"a_@@b___@@c", Some(b"m")), b"a__mb___mc");
    }

    #[test]
    fn tabs_are_blanks() {
        assert_eq!(read_line(b"\t\tx\t\t y\tz"), b"x  y z");
        assert_eq!(read_line(b" \tx"), b"  x", "a space is other, so the tab after it is a blank in mid-line");
        assert_eq!(read_line(b"a\x00b\x7f"), b"ab");
        assert_eq!(one("\t\n\t\t\nx\n", "p"), "\nx\n", "a tab-only line is empty");
        // TeX Live 2026's tex on `a}<tab>` / `b}<tab>  <tab>` (probed): the
        // tabs are blanks, not trimmed.
        assert_eq!(one("a}\t\nb}\t  \t\nc} \n\t\td\n", "p"), "a} \nb}    \nc}\nd\n");
    }
}
