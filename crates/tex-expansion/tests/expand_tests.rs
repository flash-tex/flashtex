use flashtex_tex_expansion::{expand_str, is_group_token, tokens_to_display_string, Token};

/// Content text with grouping tokens removed (they are emitted for the
/// typesetter; see `grouping_tokens_are_emitted_with_spans`).
fn text(tokens: &[Token]) -> String {
    let content: Vec<Token> = tokens.iter().filter(|t| !is_group_token(t)).cloned().collect();
    tokens_to_display_string(&content)
}

fn run(src: &str) -> String {
    let r = expand_str(src);
    assert!(
        r.diagnostics.is_empty(),
        "unexpected diagnostics for {src:?}: {:?}",
        r.diagnostics
    );
    text(&r.tokens)
}

fn run_allow_diag(src: &str) -> (String, usize) {
    let r = expand_str(src);
    (text(&r.tokens), r.diagnostics.len())
}

#[test]
fn simple_def_and_call() {
    assert_eq!(run(r"\def\a{hello}\a"), "hello");
}

#[test]
fn def_with_parameters() {
    assert_eq!(run(r"\def\greet#1{Hello, #1!}\greet{world}"), "Hello, world!");
}

#[test]
fn def_delimited_parameter() {
    assert_eq!(run(r"\def\a#1;{[#1]}\a hello;"), "[hello]");
}

#[test]
fn let_copies_meaning() {
    assert_eq!(run(r"\def\a{X}\let\b=\a\b"), "X");
}

#[test]
fn let_to_character() {
    assert_eq!(run(r"\let\a=b\a"), "b");
}

#[test]
fn expandafter_swaps_order() {
    // \def\a{AA} \def\b{\a} -- \expandafter\a\b would just leave \a (not
    // expandable further meaningfully); use the classic pattern instead:
    // build \ifx comparisons via expandafter chains.
    assert_eq!(run(r"\def\a{X}\def\b{\a}\expandafter\def\expandafter\c\expandafter{\b}\c"), "X");
}

#[test]
fn noexpand_freezes_one_token() {
    assert_eq!(run(r"\def\a{X}\edef\b{\noexpand\a}\b"), "X");
    // \b's body is literally the token \a (unexpanded at edef time), and
    // when \b is *called*, its body (just \a) gets reinserted and then IS
    // expanded normally by the outer loop, yielding X. This matches real
    // TeX: \noexpand only suppresses expansion during the edef scan.
}

#[test]
fn csname_builds_control_sequence() {
    assert_eq!(run(r"\def\foo{bar}\csname foo\endcsname"), "bar");
}

#[test]
fn csname_undefined_becomes_relax() {
    let (_out, diags) = run_allow_diag(r"\csname zzzundefined\endcsname");
    assert_eq!(diags, 0);
}

#[test]
fn string_of_control_sequence() {
    assert_eq!(run(r"\string\foo"), "\\foo");
}

#[test]
fn number_primitive() {
    assert_eq!(run(r"\number 42"), "42");
}

#[test]
fn romannumeral_primitive() {
    assert_eq!(run(r"\romannumeral 1994"), "mcmxciv");
}

#[test]
fn the_of_count_register() {
    assert_eq!(run(r"\count0=5 \the\count0"), "5");
}

#[test]
fn advance_multiply_divide() {
    assert_eq!(run(r"\count0=5 \advance\count0 by 3 \the\count0"), "8");
    assert_eq!(run(r"\count0=5 \multiply\count0 by 3 \the\count0"), "15");
    assert_eq!(run(r"\count0=15 \divide\count0 by 3 \the\count0"), "5");
}

#[test]
fn countdef_alias() {
    assert_eq!(run(r"\countdef\mycount=5 \mycount=7 \the\mycount"), "7");
}

#[test]
fn grouping_restores_local_def() {
    // Bare grouping braces are pure bookkeeping (verified against real TeX
    // via the oracle corpus: they never appear in the observable output).
    assert_eq!(run(r"\def\a{outer}{\def\a{inner}\a}\a"), "innerouter");
}

#[test]
fn global_def_survives_group() {
    assert_eq!(run(r"\def\a{outer}{\global\def\a{inner}\a}\a"), "innerinner");
}

#[test]
fn begingroup_endgroup_scopes_registers() {
    assert_eq!(run(r"\count0=1 \begingroup\count0=2 \the\count0\endgroup\the\count0"), "21");
}

#[test]
fn aftergroup_reinserts_after_close() {
    assert_eq!(run(r"{\aftergroup X}Y"), "XY");
}

#[test]
fn iftrue_iffalse() {
    assert_eq!(run(r"\iftrue A\else B\fi"), "A");
    assert_eq!(run(r"\iffalse A\else B\fi"), "B");
}

#[test]
fn ifnum_relations() {
    assert_eq!(run(r"\ifnum 3<5 yes\else no\fi"), "yes");
    assert_eq!(run(r"\ifnum 5<3 yes\else no\fi"), "no");
    assert_eq!(run(r"\ifnum 5=5 yes\else no\fi"), "yes");
}

#[test]
fn ifdim_relations() {
    assert_eq!(run(r"\ifdim 1pt<2pt yes\else no\fi"), "yes");
}

#[test]
fn ifodd_check() {
    assert_eq!(run(r"\ifodd 3 odd\else even\fi"), "odd");
    assert_eq!(run(r"\ifodd 4 odd\else even\fi"), "even");
}

#[test]
fn ifx_macro_equality() {
    assert_eq!(run(r"\def\a{X}\def\b{X}\ifx\a\b same\else different\fi"), "same");
    assert_eq!(run(r"\def\a{X}\def\b{Y}\ifx\a\b same\else different\fi"), "different");
}

#[test]
fn ifcase_selects_branch() {
    assert_eq!(run(r"\ifcase 2 zero\or one\or two\or three\fi"), "two");
    assert_eq!(run(r"\ifcase 0 zero\or one\or two\fi"), "zero");
}

#[test]
fn nested_conditionals_with_else_skipping() {
    assert_eq!(
        run(r"\iftrue \iffalse inner-true\else inner-false\fi \else outer-false\fi"),
        "inner-false"
    );
}

#[test]
fn newif_and_toggle() {
    assert_eq!(run(r"\newif\ifmyflag \myflagtrue\ifmyflag YES\else NO\fi"), "YES");
    assert_eq!(run(r"\newif\ifmyflag \myflagfalse\ifmyflag YES\else NO\fi"), "NO");
}

#[test]
fn unless_negates() {
    assert_eq!(run(r"\unless\iftrue A\else B\fi"), "B");
}

#[test]
fn catcode_and_makeatletter() {
    assert_eq!(run(r"\makeatletter\def\foo@bar{X}\foo@bar\makeatother"), "X");
}

#[test]
fn dimen_units() {
    assert_eq!(run(r"\dimen0=1in \the\dimen0"), "72.26999pt");
}

#[test]
fn numexpr_arithmetic() {
    assert_eq!(run(r"\count0=\numexpr 2+3*4\relax \the\count0"), "14");
}

#[test]
fn newcommand_no_args() {
    assert_eq!(run(r"\newcommand{\hi}{Hello}\hi"), "Hello");
}

#[test]
fn newcommand_with_args() {
    assert_eq!(run(r"\newcommand{\greet}[1]{Hello #1}\greet{World}"), "Hello World");
}

#[test]
fn newcommand_with_optional_arg_default() {
    assert_eq!(run(r"\newcommand{\greet}[2][Hi]{#1, #2}\greet{World}"), "Hi, World");
}

#[test]
fn newcommand_with_optional_arg_given() {
    assert_eq!(run(r"\newcommand{\greet}[2][Hi]{#1, #2}\greet[Yo]{World}"), "Yo, World");
}

#[test]
fn renewcommand_replaces() {
    assert_eq!(run(r"\newcommand{\a}{old}\renewcommand{\a}{new}\a"), "new");
}

#[test]
fn providecommand_keeps_existing() {
    assert_eq!(run(r"\newcommand{\a}{old}\providecommand{\a}{new}\a"), "old");
}

#[test]
fn newenvironment_expands_begin_end() {
    assert_eq!(
        run(r"\newenvironment{myenv}{[BEGIN]}{[END]}\begin{myenv}content\end{myenv}"),
        "[BEGIN]content[END]"
    );
}

#[test]
fn newcounter_and_setcounter_stepcounter() {
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{5}\arabic{foo}"), "5");
    assert_eq!(run(r"\newcounter{foo}\stepcounter{foo}\stepcounter{foo}\arabic{foo}"), "2");
}

#[test]
fn addtocounter_and_value() {
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{3}\addtocounter{foo}{4}\arabic{foo}"), "7");
}

#[test]
fn roman_and_alph_counters() {
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{4}\roman{foo}"), "iv");
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{4}\Roman{foo}"), "IV");
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{1}\alph{foo}"), "a");
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{2}\Alph{foo}"), "B");
}

#[test]
fn at_ifnextchar_and_ifstar() {
    assert_eq!(
        run(r"\makeatletter\def\test{\@ifnextchar*{\@teststar}{\@testnostar}}\def\@teststar*{STAR}\def\@testnostar{NOSTAR}\test*\makeatother"),
        "STAR"
    );
    assert_eq!(
        run(r"\makeatletter\def\test{\@ifstar{STARBRANCH}{NOSTARBRANCH}}\test*\makeatother"),
        "STARBRANCH"
    );
}

#[test]
fn nameuse_and_namedef() {
    assert_eq!(run(r"\makeatletter\@namedef{foo}{bar}\@nameuse{foo}\makeatother"), "bar");
}

#[test]
fn long_macro_flag_parses() {
    // \long is accepted (allowing \par in arguments); our engine does not
    // yet special-case \par-forbidding for non-long macros (documented gap),
    // but the flag must at least parse without breaking the definition.
    assert_eq!(run(r"\long\def\a#1{[#1]}\a{x}"), "[x]");
}

#[test]
fn def_brace_delim_last() {
    // `#{` (TeXbook p.205): the last parameter is delimited by the
    // upcoming `{`, which is left unconsumed. That leftover `{y}` is then
    // an ordinary top-level group processed by normal (main-control-like)
    // execution, which is silent for bare grouping braces (see
    // CONTRACT.md "Known deviations" -- this differs from what `\write`'s
    // scan_toks-based argument scanning would show, which is why this
    // case isn't in the oracle corpus).
    assert_eq!(run(r"\def\a#1#{[#1]}\a x{y}"), "[x]y");
}

#[test]
fn infinite_macro_loop_terminates_with_diagnostic() {
    // \loop directly recurses with no base case: real TeX would run out
    // of memory / hang. The engine must instead hit its step limit and
    // return a diagnostic rather than looping or panicking.
    let r = expand_str(r"\def\loop{\loop}\loop");
    assert!(r.tokens.is_empty());
    assert_eq!(r.diagnostics.len(), 1);
    assert!(r.diagnostics[0].message.contains("expansion step limit"));
}

#[test]
fn futurelet_peeks_without_consuming() {
    // \futurelet\next X\a sets \next's meaning to \a's meaning (macro ->
    // Z), then reinserts "X\a" unchanged so \a still expands normally;
    // calling \next afterward shares that same meaning.
    assert_eq!(run(r"\def\a{Z}\futurelet\next X\a\next"), "XZZ");
}


#[test]
fn grouping_tokens_are_emitted_with_spans() {
    let src = r"a{b}\begingroup c\endgroup\bgroup d\egroup";
    let r = expand_str(src);
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let shown: Vec<String> = r.tokens.iter().map(|t| tokens_to_display_string(std::slice::from_ref(t))).collect();
    assert_eq!(shown, ["a", "{", "b", "}", "\\begingroup ", "c", "\\endgroup ", "{", "d", "}"]);
    // Explicit braces keep their exact source spans...
    assert_eq!((r.tokens[1].span.start, r.tokens[1].span.end), (1, 2));
    assert_eq!((r.tokens[3].span.start, r.tokens[3].span.end), (3, 4));
    // ...and \bgroup/\egroup emit the implicit brace with the control
    // sequence's span; \begingroup keeps its own.
    let span_text = |i: usize| &src[r.tokens[i].span.start as usize..r.tokens[i].span.end as usize];
    assert_eq!(span_text(4), "\\begingroup");
    assert_eq!(span_text(7), "\\bgroup");
    assert_eq!(span_text(9), "\\egroup");
}

#[test]
fn aftergroup_tokens_follow_the_closing_brace() {
    let r = expand_str(r"{\aftergroup Xy}z");
    let shown: Vec<String> = r.tokens.iter().map(|t| tokens_to_display_string(std::slice::from_ref(t))).collect();
    assert_eq!(shown, ["{", "y", "}", "X", "z"]);
}

#[test]
fn macro_arguments_keep_group_boundaries() {
    // The typesetter must see that `\textbf`'s argument is one group.
    let r = expand_str(r"\def\wrap#1{\textbf{#1}}\wrap{ab}");
    let shown: Vec<String> = r.tokens.iter().map(|t| tokens_to_display_string(std::slice::from_ref(t))).collect();
    assert_eq!(shown, ["\\textbf ", "{", "a", "b", "}"]);
}

#[test]
fn runaway_recursion_inside_edef_terminates() {
    use flashtex_tex_expansion::{Engine, Limits};
    let limits = Limits { max_expansion_steps: 20_000, ..Limits::default() };
    let mut e = Engine::with_limits(r"\def\a{x\a}\edef\b{\a}\b", limits);
    e.run();
    let d = e.take_diagnostics();
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].message.contains("step limit"));
}

#[test]
fn tail_recursive_loop_does_not_grow_the_input_stack() {
    // 50k iterations of LaTeX's \loop run within the default limits.
    let r = expand_str(r"\count1=0 \loop\advance\count1 by 1 \ifnum\count1<50000 \repeat\the\count1");
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert!(run_allow_diag(r"\count1=0 \loop\advance\count1 by 1 \ifnum\count1<50000 \repeat\the\count1").0.ends_with("50000"));
}


#[test]
fn crlf_line_endings_count_once() {
    // A blank CRLF line is one \par, not two; a CRLF mid-paragraph is a space.
    assert_eq!(run_allow_diag("a\r\nb\r\n\r\nc").0, "a b \\par c");
}

#[test]
fn trailing_spaces_are_stripped_before_endlinechar() {
    assert_eq!(run("\\endlinechar=-1 a   \nb"), "ab");
    assert_eq!(run("a   \nb"), "a b");
}

#[test]
fn active_endlinechar_under_obeylines_style_catcode() {
    assert_eq!(run("\\catcode`\\^^M=13 \\def^^M{|}%\na\nb"), "a|b");
}

/// Fuzz findings: TeX's integer and dimension ranges (tex.web §445, §448,
/// §1236-1240, e-TeX `\numexpr`). Out-of-range values are clamped or
/// rejected with TeX's error, never an i64 overflow panic.
#[test]
fn numeric_ranges_follow_tex_instead_of_overflowing() {
    let cases: &[(&str, &str, &str)] = &[
        (r"\count1=99999999999999999999 \the\count1", "2147483647", "Number too big."),
        (r#"\count1="FFFFFFFFFFFFFFFFFF \the\count1"#, "2147483647", "Number too big."),
        (r"\count1=99999999999999999999 \advance\count1 by 1 \the\count1", "2147483647", "Arithmetic overflow."),
        (r"\count1=2147483647 \advance\count1 by 1 \the\count1", "2147483647", "Arithmetic overflow."),
        (
            r"\count1=2147483647 \multiply\count1 by 2147483647 \multiply\count1 by 2147483647 \the\count1",
            "2147483647",
            "Arithmetic overflow.",
        ),
        (r"\dimen0=20000pt \the\dimen0", "16383.99998pt", "Dimension too large."),
        (r"\dimen0=16383pt \advance\dimen0 by 16383pt \the\dimen0", "16383.0pt", "Arithmetic overflow."),
        (r"\dimen0=99999999999999999999\dimen1 \the\dimen0", "0.0pt", "Number too big."),
        (r"\the\numexpr 2147483647+1\relax", "0", "Arithmetic overflow."),
        (r"\the\numexpr 2147483647*2147483647*2147483647*2147483647\relax", "0", "Arithmetic overflow."),
    ];
    for (src, value, message) in cases {
        let r = expand_str(src);
        assert_eq!(text(&r.tokens).trim(), *value, "{src}");
        assert!(r.diagnostics.iter().any(|d| d.message == *message), "{src}: {:?}", r.diagnostics);
    }
    let fil = format!(r"\skip0=0pt plus 1fi{} \the\skip0", "l".repeat(300));
    // 300 `l`s overflowed the u8 order counter.
    let r = expand_str(&fil);
    assert!(r.diagnostics.iter().any(|d| d.message == "Illegal unit of measure (replaced by filll)."));
}

/// Fuzz finding: `\loop` whose `\repeat` never comes. The file ends while
/// its argument is scanned; TeX aborts the call (§339) rather than running
/// `\iterate` on the partial body, which looped to the step limit and
/// flooded "Extra \fi." diagnostics.
#[test]
fn a_macro_call_cut_off_by_the_end_of_file_is_aborted() {
    let r = expand_str(r"\loop{x}");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["Runaway argument?\n! File ended while scanning use of \\loop."]);
    assert_eq!(text(&r.tokens).trim(), "");
}

/// Fuzz finding (hang): a macro that doubles its argument on every call
/// exhausted memory long before the expansion-step limit.
#[test]
fn an_argument_that_doubles_every_call_exceeds_capacity() {
    let started = std::time::Instant::now();
    let r = expand_str(r"\def\a#1{\a{#1#1}}\a x");
    assert!(
        r.diagnostics.iter().any(|d| d.message == "TeX capacity exceeded, sorry [main memory size=5000000]."),
        "{:?}",
        r.diagnostics
    );
    assert!(started.elapsed().as_secs() < 60, "{:?}", started.elapsed());
}

/// Fuzz finding (36 GB resident): a self-invocation that is not a tail call
/// (`\csname a` re-enters `\a` before the rest of its body is read) adds an
/// input level holding the whole remaining body on every call.
#[test]
fn a_non_tail_self_call_exceeds_capacity_instead_of_memory() {
    let body = format!("{}x{}", "[".repeat(10_000), "]".repeat(10_000));
    let src = format!(r"\def\a{{\csname a\endcsname {body}}}\a");
    let started = std::time::Instant::now();
    let r = expand_str(&src);
    assert!(
        r.diagnostics.iter().any(|d| d.message.starts_with("TeX capacity exceeded, sorry [")),
        "{:?}",
        &r.diagnostics[..r.diagnostics.len().min(3)]
    );
    assert!(started.elapsed().as_secs() < 60, "{:?}", started.elapsed());
    // A plain non-tail recursion with a short body stops at the input stack.
    let r = expand_str(r"\def\b{\b x}\b");
    assert!(
        r.diagnostics.iter().any(|d| d.message == "TeX capacity exceeded, sorry [input stack size=10000]."),
        "{:?}",
        &r.diagnostics[..r.diagnostics.len().min(3)]
    );
}

/// Fuzz finding (22 s for a mutated oracle fixture): every `\if` counted the
/// newlines before it for a message only an unterminated conditional
/// prints, so a runaway loop of conditionals late in a long file was
/// quadratic.
#[test]
fn conditionals_in_a_runaway_loop_do_not_rescan_the_source() {
    let src = format!("{}\\def\\a{{\\ifnum1<2 \\fi\\a}}\\a", "% filler line\n".repeat(20_000));
    let started = std::time::Instant::now();
    let r = expand_str(&src);
    assert!(r.diagnostics.iter().any(|d| d.message.contains("step limit")), "{:?}", r.diagnostics);
    assert!(started.elapsed().as_secs() < 30, "{:?}", started.elapsed());
    // The line still appears where TeX prints it.
    let r = expand_str("\n\n\\iffalse never closed");
    assert!(
        r.diagnostics.iter().any(|d| d.message == "Incomplete \\iffalse; all text was ignored after line 3."),
        "{:?}",
        r.diagnostics
    );
}
