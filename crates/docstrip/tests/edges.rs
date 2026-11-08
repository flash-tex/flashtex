//! Error paths and edge cases of the public API that the in-crate unit
//! tests and the TeX Live oracle (`tests/texlive.rs`) do not cover.

use std::collections::BTreeMap;

use flashtex_docstrip::strip::{process_source, Consumer, State};
use flashtex_docstrip::{Diagnostic, Outcome};

fn sources(files: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    files
        .iter()
        .map(|(n, t)| (n.to_string(), t.as_bytes().to_vec()))
        .collect()
}

fn run(ins: &str, files: &[(&str, &str)]) -> Outcome {
    flashtex_docstrip::run("t.ins", ins.as_bytes(), &sources(files))
}

fn strip(src: &str, options: &[&str]) -> (Vec<String>, Vec<Diagnostic>, Vec<String>) {
    let mut consumers = Vec::new();
    for (i, o) in options.iter().enumerate() {
        let options = o.as_bytes().to_vec();
        consumers.push(Consumer { output: i, options });
    }
    let mut outputs = vec![Vec::new(); options.len()];
    let mut state = State::default();
    let (mut diagnostics, mut messages) = (Vec::new(), Vec::new());
    process_source(
        "t.dtx",
        src.as_bytes(),
        &consumers,
        &mut outputs,
        b"%%",
        &mut state,
        &mut diagnostics,
        &mut messages,
    );
    let mut outs = Vec::new();
    for o in outputs {
        outs.push(String::from_utf8(o).unwrap());
    }
    (outs, diagnostics, messages)
}

#[test]
fn jobname_is_the_batch_file_stem() {
    // \jobname is the batch file's stem: directories and `.ins` are stripped.
    let ins = "\\input docstrip\n\\Msg{\\jobname}\n";
    let empty: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let o = flashtex_docstrip::run("a/b/c.ins", ins.as_bytes(), &empty);
    assert_eq!(o.messages, ["c"]);
    let o = flashtex_docstrip::run("plain", ins.as_bytes(), &empty);
    assert_eq!(o.messages, ["plain"]);
}

#[test]
fn strip_guard_diagnostics() {
    for src in ["%<*abc\nx\n", "%</abc\nx\n", "%<+abc\nx\n", "%<abc\nx\n"] {
        let (outs, diags, _) = strip(src, &["abc"]);
        assert_eq!(outs, ["x\n"]);
        assert_eq!(diags.len(), 1, "{src}: {diags:?}");
        assert_eq!(diags[0].message, "guard line without a closing `>`");
    }
    for (src, want) in [
        ("%<+bogus&>x\n", "in `%<+bogus&>`"),
        ("%<-bogus&>x\n", "in `%<-bogus&>`"),
        ("%<>x\n", "in `%<>`"),
    ] {
        let (outs, diags, _) = strip(src, &["bogus"]);
        assert!(outs[0].is_empty(), "{src}");
        assert!(diags[0].message.contains(want), "{src}: {diags:?}");
    }
    let (outs, diags, _) = strip("%<@foo>\nx\n", &["a"]);
    assert_eq!(outs, ["x\n"]);
    assert!(diags[0].message.contains("only valid as"), "{diags:?}");
    // The guard ends at the first `>`.
    let (outs, diags, _) = strip("%<a>b>c\n", &["a"]);
    assert_eq!(outs, ["b>c\n"]);
    assert!(diags.is_empty());
}

#[test]
fn strip_off_blocks_hide_guarded_lines() {
    let src = "%<*a>\n%<b>X\n%<+b>Y\n%<-b>Z\n%% keep\ncode\n%</a>\n";
    let (outs, diags, _) = strip(src, &["b"]);
    assert!(outs[0].is_empty());
    assert!(diags.is_empty());
    let (outs, _, _) = strip(src, &["a,b"]);
    assert_eq!(outs, ["X\nY\n%% keep\ncode\n"]);
}

#[test]
fn strip_verbatim_and_endinput_edges() {
    // Verbatim copies nothing while its block is off.
    let (outs, diags, _) = strip("%<*a>\n%<<END\ncode\n%END\n%</a>\n", &["b"]);
    assert!(outs[0].is_empty());
    assert!(diags.is_empty());
    // \endinput ends the source even inside an off block, with a message.
    let (outs, _, msgs) = strip("%<*a>\n\\endinput\ncode\n", &["b"]);
    assert!(outs[0].is_empty());
    assert_eq!(msgs, ["File t.dtx ended by \\endinput."]);
}

#[test]
fn input_before_and_after_docstrip_loads() {
    let o = run("\\input defs.tex\n\\input docstrip\n", &[]);
    assert!(o.diagnostics[0].message.contains("cannot find file"));
    let files = [("defs.tex", "\\def\\greet{hi}")];
    let ins = "\\input defs.tex\n\\input docstrip\n\\Msg{\\greet}\n";
    let o = run(ins, &files);
    assert!(o.diagnostics.is_empty());
    assert_eq!(o.messages, ["hi"]);
    let o = run("\\input l3docstrip\n\\input foo\n", &[]);
    assert!(o.diagnostics[0].message.contains("is ignored"));
    let o = run("\\input docstrip\n\\input foo\n", &[]);
    assert!(o.diagnostics[0].message.contains("is ignored"));
}

#[test]
fn nested_input_edges() {
    let files = [("defs", "\\def\\a{A}\n\\endinput\n\\def\\a{B}\n")];
    let o = run("\\input defs\n\\input docstrip\n\\Msg{\\a}\n", &files);
    assert!(o.diagnostics.is_empty());
    assert_eq!(o.messages, ["A"]);
    let files = [("mid", "\\input mid\n\\Msg{hi}\n")];
    let o = run("\\input mid\n\\input docstrip\n", &files);
    assert!(o.diagnostics[0].message.contains("already being read"));
    assert_eq!(o.messages, ["hi"]);
    assert!(o.completed);
    let o = run("\\input docstrip\n\\batchinput{nope.ins}\n", &[]);
    assert!(o.diagnostics[0].message.contains("cannot find batch file"));
}

#[test]
fn macro_definition_forms() {
    // \let with `=` or spaces.
    let ins = "\\def\\x{X}\\let\\a=\\x\\let\\b \\x\\ifx\\a\\b\\Msg{ok}\\fi";
    let o = run(ins, &[]);
    assert!(o.diagnostics.is_empty());
    assert_eq!(o.messages, ["ok"]);
    // \gdef survives the group; \def does not.
    let o = run("{\\def\\l{L}\\gdef\\g{G}}\\Msg{\\g}\\l", &[]);
    assert_eq!(o.messages, ["G"]);
    assert_eq!(o.diagnostics.len(), 1);
    assert!(o.diagnostics[0].message.contains("not a docstrip command"));
}

#[test]
fn runaway_expansion_aborts_the_run() {
    let ins = format!("\\def\\a{{{}}}\\a", "\\a".repeat(8));
    let o = run(&ins, &[]);
    assert!(!o.completed);
    assert_eq!(o.diagnostics.len(), 1);
    assert!(o.diagnostics[0].message.contains("giving up after"));
}
