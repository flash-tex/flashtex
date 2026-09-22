use flashtex_tex_expansion::{
    expand_str, is_group_token, tokens_to_display_string, Engine, Limits, Token,
};

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
fn conditional_depth_limit_allows_exactly_n_open_conditionals() {
    let n: usize = 2;
    let limit_diagnostics = |depth| {
        let source = format!("{}X{}", r"\iftrue ".repeat(depth), r"\fi ".repeat(depth));
        let mut engine = Engine::with_limits(
            &source,
            Limits {
                max_conditional_depth: n as u32,
                ..Limits::default()
            },
        );
        engine.run();
        engine
            .take_diagnostics()
            .into_iter()
            .filter(|d| d.message == "conditional nesting limit exceeded")
            .count()
    };

    assert_eq!(limit_diagnostics(n), 0);
    assert_eq!(limit_diagnostics(n + 1), 1);
}

#[test]
fn group_depth_limit_allows_exactly_n_open_groups() {
    let n: usize = 2;
    let limit_diagnostics = |depth| {
        let source = format!("{}X{}", "{".repeat(depth), "}".repeat(depth));
        let mut engine = Engine::with_limits(
            &source,
            Limits {
                max_group_depth: n as u32,
                ..Limits::default()
            },
        );
        engine.run();
        engine
            .take_diagnostics()
            .into_iter()
            .filter(|d| d.message == "group nesting limit exceeded")
            .count()
    };

    assert_eq!(limit_diagnostics(n), 0);
    assert_eq!(limit_diagnostics(n + 1), 1);
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
fn ifthenelse_equal() {
    assert_eq!(run(r"\ifthenelse{\equal{a}{a}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\equal{a}{b}}{YES}{NO}"), "NO");
    // Arguments expand before comparison.
    assert_eq!(run(r"\def\who{Fred}\ifthenelse{\equal{\who}{Fred}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\def\who{Fred}\ifthenelse{\equal{\who}{Bob}}{YES}{NO}"), "NO");
}

#[test]
fn ifthenelse_not() {
    assert_eq!(run(r"\ifthenelse{\NOT{\equal{a}{b}}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\NOT{\equal{a}{a}}}{YES}{NO}"), "NO");
}

#[test]
fn ifthenelse_and_or() {
    assert_eq!(run(r"\ifthenelse{\AND{\equal{a}{a}}{\equal{b}{b}}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\AND{\equal{a}{a}}{\equal{b}{c}}}{YES}{NO}"), "NO");
    assert_eq!(run(r"\ifthenelse{\OR{\equal{a}{b}}{\equal{c}{d}}}{YES}{NO}"), "NO");
    // Combined nesting, as in real documents.
    assert_eq!(run(r"\ifthenelse{\OR{\equal{a}{b}}{\AND{\equal{x}{x}}{\NOT{\equal{y}{z}}}}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\OR{\equal{a}{b}}{\AND{\equal{x}{x}}{\NOT{\equal{x}{x}}}}}{YES}{NO}"), "NO");
}

#[test]
fn ifthenelse_numeric_tests() {
    assert_eq!(run(r"\ifthenelse{\isodd{3}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\isodd{4}}{YES}{NO}"), "NO");
    assert_eq!(run(r"\newcounter{sec}\setcounter{sec}{3}\ifthenelse{\isodd{\value{sec}}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\lengthtest{1pt<2pt}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\ifthenelse{\lengthtest{2pt<1pt}}{YES}{NO}"), "NO");
    assert_eq!(run(r"\ifthenelse{\lengthtest{12pt=12pt}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\def\foo{x}\ifthenelse{\isundefined{\foo}}{YES}{NO}"), "NO");
    assert_eq!(run(r"\ifthenelse{\isundefined{\nosuchcommand}}{YES}{NO}"), "YES");
}

#[test]
fn ifthenelse_boolean() {
    assert_eq!(run(r"\newboolean{draft}\ifthenelse{\boolean{draft}}{YES}{NO}"), "NO");
    assert_eq!(run(r"\newboolean{draft}\setboolean{draft}{true}\ifthenelse{\boolean{draft}}{YES}{NO}"), "YES");
    assert_eq!(
        run(r"\newboolean{draft}\setboolean{draft}{true}\setboolean{draft}{false}\ifthenelse{\boolean{draft}}{YES}{NO}"),
        "NO"
    );
    // Kernel flags share the representation, so \boolean sees them too.
    assert_eq!(run(r"\newif\ifmyflag\myflagtrue\ifthenelse{\boolean{myflag}}{YES}{NO}"), "YES");
    assert_eq!(run(r"\newif\ifmyflag\ifthenelse{\boolean{myflag}}{YES}{NO}"), "NO");
}

#[test]
fn ifthenelse_inside_macro_body() {
    assert_eq!(run(r"\def\check#1{\ifthenelse{\equal{#1}{x}}{YES}{NO}}\check{x}"), "YES");
    assert_eq!(run(r"\def\check#1{\ifthenelse{\equal{#1}{x}}{YES}{NO}}\check{y}"), "NO");
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
fn newenvironment_with_argument_substitutes() {
    assert_eq!(
        run(r"\newenvironment{greet}[1]{Hello #1: }{!}\begin{greet}{World}body\end{greet}"),
        "Hello World: body!"
    );
}

#[test]
fn renewenvironment_replaces_begin_and_end_code() {
    assert_eq!(
        run(r"\newenvironment{shout}{Hi }{!}\renewenvironment{shout}{Yo }{?}\begin{shout}Bob\end{shout}"),
        "Yo Bob?"
    );
}

#[test]
fn begin_end_emit_balanced_group_markers() {
    // `\end` already reaches the output as `\endgroup`; `\begin` must
    // emit the matching `\begingroup`, or an environment whose begin/end
    // code expands inline (every `\newenvironment`) leaves no scope
    // behind for the typesetter.
    let src = r"\newenvironment{myenv}{[BEGIN]}{[END]}\begin{myenv}content\end{myenv}";
    let r = expand_str(src);
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(
        tokens_to_display_string(&r.tokens),
        "\\begingroup [BEGIN]content[END]\\endgroup "
    );
    // The markers carry the `\begin`/`\end` spans, not the definition's.
    let span_text = |t: &Token| &src[t.span.start as usize..t.span.end as usize];
    assert_eq!(span_text(&r.tokens[0]), "\\begin");
    assert_eq!(span_text(&r.tokens[r.tokens.len() - 1]), "\\end");
}

#[test]
fn undefined_begin_end_emit_balanced_group_markers() {
    // Environments the engine passes through (`quote` here) get the same
    // pair: the opener is new, the closer was already emitted.
    let r = expand_str(r"\begin{quote}X\end{quote}");
    assert_eq!(tokens_to_display_string(&r.tokens), "\\begingroup \\quote X\\endquote \\endgroup ");
    assert!(
        r.diagnostics.iter().any(|d| d.message.contains("passed through")),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn newcounter_and_setcounter_stepcounter() {
    assert_eq!(run(r"\newcounter{foo}\setcounter{foo}{5}\arabic{foo}"), "5");
    assert_eq!(run(r"\newcounter{foo}\stepcounter{foo}\stepcounter{foo}\arabic{foo}"), "2");
}

#[test]
fn newcounter_within_kernel_counter_resets_when_parent_steps() {
    // Class counters (`section`, `chapter`, ...) exist without an explicit
    // `\newcounter`, as in any standard document class.
    assert_eq!(run(r"\newcounter{c}[section]\stepcounter{c}\arabic{c}"), "1");
    assert_eq!(run(r"\newcounter{c}[section]\stepcounter{c}\stepcounter{section}\arabic{c}"), "0");
    assert_eq!(run(r"\newcounter{c}[chapter]\stepcounter{c}\arabic{c}"), "1");
}

#[test]
fn counterwithout_kernel_parent_stops_resetting() {
    assert_eq!(
        run(r"\newcounter{c}[section]\stepcounter{c}\counterwithout{c}{section}\stepcounter{section}\arabic{c}"),
        "1"
    );
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
        (r"\count1=99999999999999999999 \advance\count1 by 1 \the\count1", "-2147483648", "Number too big."),
        (
            r"\count1=2147483647 \multiply\count1 by 2147483647 \multiply\count1 by 2147483647 \the\count1",
            "2147483647",
            "Arithmetic overflow.",
        ),
        (r"\dimen0=20000pt \the\dimen0", "16383.99998pt", "Dimension too large."),
        (r"\dimen0=99999999999999999999\dimen1 \the\dimen0", "0.0pt", "Number too big."),
        (r"\the\numexpr 2147483647+1\relax", "0", "Arithmetic overflow."),
        (r"\the\numexpr 2147483647*2147483647*2147483647*2147483647\relax", "0", "Arithmetic overflow."),
    ];
    for (src, value, message) in cases {
        let r = expand_str(src);
        assert_eq!(text(&r.tokens).trim(), *value, "{src}");
        assert!(r.diagnostics.iter().any(|d| d.message == *message), "{src}: {:?}", r.diagnostics);
    }
    // `\advance` has no range check: it wraps in 32-bit arithmetic without a
    // diagnostic. Values measured with pdfTeX 3.141592653-2.6-1.40.29 (TeX
    // Live 2026), plain and -etex alike, via \message{\the...}.
    // `\dimen9` is `\maxdimen` (2^30-1 sp), written in sp to avoid decimal rounding.
    let maxdimen = r"\dimen9=1073741823sp ";
    let wrapping: &[(&str, &str)] = &[
        (r"\count1=2147483647 \advance\count1 by 1 \the\count1", "-2147483648"),
        (r"\count1=-2147483647 \advance\count1 by -2 \the\count1", "2147483647"),
        (r"\dimen0=\dimen9 \advance\dimen0 by 1sp \the\dimen0", "16384.0pt"),
        (r"\dimen0=16383pt \advance\dimen0 by 16383pt \the\dimen0", "32766.0pt"),
        (r"\dimen0=\dimen9 \advance\dimen0 by \dimen9 \advance\dimen0 by \dimen9 \the\dimen0", "-16384.00005pt"),
        (
            r"\dimen0=\dimen9 \advance\dimen0 by \dimen9 \advance\dimen0 by \dimen9 \advance\dimen0 by \dimen9 \advance\dimen0 by 1sp \the\dimen0",
            "-0.00005pt",
        ),
        (
            r"\skip0=1073741823sp plus 1073741823sp minus 1pt \advance\skip0 by 1073741823sp plus 1073741823sp minus 2pt{}\the\skip0",
            "32767.99997pt plus 32767.99997pt minus 3.0pt",
        ),
        (
            r"\skip0=1073741823sp plus 1073741823sp \advance\skip0 by 1073741823sp plus 1073741823sp \advance\skip0 by 1073741823sp plus 1073741823sp \advance\skip0 by 1073741823sp plus 1073741823sp \advance\skip0 by 1sp plus 1sp{}\the\skip0",
            "-0.00005pt plus -0.00005pt",
        ),
    ];
    for (src, value) in wrapping {
        let r = expand_str(&format!("{maxdimen}{src}"));
        assert_eq!(text(&r.tokens).trim(), *value, "{src}");
        assert!(r.diagnostics.is_empty(), "{src}: {:?}", r.diagnostics);
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

fn limited_diagnostics(src: &str, limits: flashtex_tex_expansion::Limits) -> Vec<String> {
    let mut e = flashtex_tex_expansion::Engine::with_limits(src, limits);
    e.run();
    e.take_diagnostics().into_iter().map(|d| d.message).collect()
}

/// A `{` or `\if` past the nesting limit is dropped with an error. It used to
/// be reported once per dropped token (a runaway `\def\a{{\a}` reported it
/// until the step limit); now once per excursion past the limit.
#[test]
fn nesting_limits_are_reported_once_per_excursion() {
    use flashtex_tex_expansion::Limits;
    let limits = Limits { max_group_depth: 3, max_conditional_depth: 3, ..Limits::default() };
    let count = |messages: &[String], what: &str| messages.iter().filter(|m| *m == what).count();

    // Three `{` refused in a row, then (after a `}` and an accepted `{`) two more.
    let groups = limited_diagnostics("{{{{{{}{{{", limits);
    assert_eq!(count(&groups, "group nesting limit exceeded"), 2, "{groups:?}");

    let conditionals = limited_diagnostics(r"\iftrue\iftrue\iftrue\iftrue\iftrue\iftrue\fi\iftrue\iftrue", limits);
    assert_eq!(count(&conditionals, "conditional nesting limit exceeded"), 2, "{conditionals:?}");

    let steps = Limits { max_expansion_steps: 50_000, ..limits };
    let runaway = limited_diagnostics(r"\let\x={ \def\a{\x\a}\a", steps);
    assert_eq!(count(&runaway, "group nesting limit exceeded"), 1, "{runaway:?}");
    let runaway = limited_diagnostics(r"\def\b{\iftrue\b}\b", steps);
    assert_eq!(count(&runaway, "conditional nesting limit exceeded"), 1, "{runaway:?}");
}

/// A runaway loop never returns to a safe point, so each of its errors is
/// recorded once rather than once per iteration.
#[test]
fn a_runaway_loop_records_each_error_once() {
    use flashtex_tex_expansion::Limits;
    let limits = Limits { max_expansion_steps: 50_000, ..Limits::default() };
    let messages = limited_diagnostics(r"\def\a{\ifnum\relax<1 \fi\a}\a", limits);
    assert_eq!(
        messages,
        [
            "Missing number, treated as zero.",
            "Missing = inserted for \\ifnum.",
            "expansion step limit exceeded (possible infinite macro loop)"
        ],
    );
    // Separate lines are separate reports, even when identical.
    let r = expand_str("\\count1=\\relax\n\\count1=\\relax\n");
    assert_eq!(r.diagnostics.iter().filter(|d| d.message == "Missing number, treated as zero.").count(), 2, "{:?}", r.diagnostics);
}

#[test]
fn a_long_environment_name_is_shortened_only_in_messages() {
    let name = "x".repeat(100_000);
    let r = expand_str(&format!("\\begin{{a}}\\end{{{name}}}"));
    let mismatch = r.diagnostics.iter().find(|d| d.message.contains("ended by")).expect("mismatch reported");
    assert_eq!(mismatch.message, format!("LaTeX Error: \\begin{{a}} ended by \\end{{{}...}}.", "x".repeat(100)));
    // The comparison itself uses the whole name.
    let r = expand_str(&format!("\\begin{{{name}}}\\end{{{name}}}"));
    assert!(!r.diagnostics.iter().any(|d| d.message.contains("ended by")), "{:?}", r.diagnostics.len());
    let r = expand_str(&format!("\\begin{{{name}}}\\end{{{name}y}}"));
    assert!(r.diagnostics.iter().any(|d| d.message.contains("ended by")));
    let r = expand_str(r"\begin{foo}\end{bar}");
    assert!(r.diagnostics.iter().any(|d| d.message == "LaTeX Error: \\begin{foo} ended by \\end{bar}."), "{:?}", r.diagnostics);
}

/// What happens past a nesting limit, as the compiler's recovery notes
/// describe it: the extra `{` is dropped without opening a group, the extra
/// conditional is dropped without evaluating its test (what follows is read
/// as ordinary text), and expansion continues in both cases.
#[test]
fn past_a_nesting_limit_the_extra_group_or_conditional_is_ignored_and_expansion_continues() {
    use flashtex_tex_expansion::{Engine, Limits, TokenKind};
    let limits = Limits { max_group_depth: 1, max_conditional_depth: 1, ..Limits::default() };
    let run = |src: &str| {
        let mut e = Engine::with_limits(src, limits);
        let text: String = e
            .run()
            .iter()
            .map(|t| match &t.kind {
                TokenKind::Char(c, _) => c.to_string(),
                TokenKind::ControlSequence(cs) => format!("\\{cs}"),
                other => format!("{other:?}"),
            })
            .collect();
        let messages: Vec<String> = e.take_diagnostics().into_iter().map(|d| d.message).collect();
        (text, messages)
    };

    // The second `{` is dropped; its `}` closes the first group, and the last
    // `}` is then unbalanced. `\def` after the limit still takes effect.
    let (text, messages) = run(r"{{a}b}\def\m{M}\m");
    assert_eq!(text, "{a}bM");
    assert_eq!(messages, ["group nesting limit exceeded", "Too many }'s."]);

    // Two conditionals may be open; the third, `\ifnum`, is dropped
    // unevaluated: its test `1>2` is text, the `\else` belongs to the second
    // `\iftrue`, and the last `\fi` is extra.
    let (text, messages) = run(r"\iftrue\iftrue\ifnum1>2 X\else Y\fi\fi\fi Z");
    assert_eq!(text, "1>2 XZ");
    assert_eq!(messages[0], "conditional nesting limit exceeded");
    assert!(messages[1..].iter().any(|m| m.starts_with("Extra ")), "{messages:?}");
}

#[test]
fn newtheorem_reserved_name_reports_collision_not_missing_control_sequence() {
    // GitHub issue #700: `\newtheorem{def}` collides with the `\def`
    // primitive. Real pdflatex refuses the declaration ("LaTeX Error:
    // Command \def already defined."); without the check the bad name
    // reached `\begin{def}`, which executed `\def` and failed with a
    // generic "Missing control sequence inserted." instead.
    let r = expand_str(r"\newtheorem{def}{Definition}\begin{def}A test.\end{def}");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\def already defined."], "{messages:?}");
    assert!(!messages.iter().any(|m| m.contains("Missing control sequence")), "{messages:?}");
    // The rejected environment is skipped; its body still typesets.
    assert_eq!(text(&r.tokens), "A test.");
}

#[test]
fn newtheorem_free_name_passes_through_silently() {
    let r = expand_str(r"\newtheorem{defn}{Definition}\begin{defn}A test.\end{defn}");
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains("A test."), "{out:?}");
    // The declaration reaches the typesetter, which owns theorem counters.
    assert!(out.contains("\\newtheorem "), "{out:?}");
}

#[test]
fn newtheorem_rejection_leaves_the_shadowed_primitive_usable() {
    // Only the rejected `\begin{def}`/`\end{def}` are diverted; a later
    // `\def` still defines.
    let r = expand_str(r"\newtheorem{def}{Definition}\def\foo{hi}\foo");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\def already defined."], "{messages:?}");
    assert_eq!(text(&r.tokens), "hi");
}

#[test]
fn newtheorem_rejection_does_not_leak_a_pending_global() {
    // A rejected declaration returned early without clearing pending
    // prefixes, so `\global` (or `\long`/`\outer`/`\protected`, though none
    // apply here) leaked onto whatever command read prefixes next.
    let r = expand_str(r"{\global\newtheorem{def}{D}\def\foo{hi}}\ifdefined\foo Y\else N\fi");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\def already defined."], "{messages:?}");
    // `\foo` was defined inside the group without `\global` surviving onto
    // it, so it does not exist once the group closes.
    assert_eq!(text(&r.tokens), "N", "{:?}", r.tokens);
}

#[test]
fn newtheorem_name_is_fully_expanded_before_the_collision_check() {
    // The name argument is a `\csname`-equivalent context in real TeX: `\n`
    // expands to `def` before anything checks it, colliding with `\def`
    // exactly like a literal `\newtheorem{def}{D}` would (review finding
    // #1), not the unexpanded control sequence name "n".
    let src = r"\def\n{def}\newtheorem{\n}{D}";
    let r = expand_str(src);
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\def already defined."], "{messages:?}");
    // The diagnostic still points at `\newtheorem` in the real source, not
    // a synthetic span from the macro body that supplied its expanded name.
    let d = &r.diagnostics[0];
    assert_eq!(&src[d.span.start as usize..d.span.end as usize], r"\newtheorem", "{d:?}");
}

#[test]
fn newtheorem_rejection_is_undone_when_its_group_closes() {
    // "widget" only collided with a *local* `\def`; once that group closes,
    // a repeat `\newtheorem{widget}{...}` outside it must succeed cleanly,
    // and `\begin{widget}`/`\end{widget}` afterwards must actually reach
    // the typesetter rather than staying silently skipped by a stale
    // rejection marker (review finding #3: the marker must be
    // group-scoped, exactly like the local `\def` that caused it).
    let r = expand_str(
        r"{\def\widget{}\newtheorem{widget}{Widget}}\newtheorem{widget}{Widget}\begin{widget}\end{widget}",
    );
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\widget already defined."], "{messages:?}");
    let out = text(&r.tokens);
    assert!(out.contains(r"\widget "), "{out:?}");
    assert!(out.contains(r"\endwidget "), "{out:?}");
}

#[test]
fn newtheorem_stale_rejection_is_cleared_by_a_later_successful_claim() {
    // "foo" collides with a local `\def`, gets rejected, then that `\def` is
    // undone with `\let` (not a group close, so the rejection's own
    // group-scoped undo does not fire) before a fresh `\newtheorem{foo}`
    // succeeds. The stale rejection must not survive a later successful
    // claim of the same name (review finding #1's "related smaller gap").
    let r = expand_str(
        r"\def\foo{}\newtheorem{foo}{Foo}\let\foo\undefined\newtheorem{foo}{Foo}\begin{foo}\end{foo}",
    );
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\foo already defined."], "{messages:?}");
    let out = text(&r.tokens);
    assert!(out.contains(r"\foo "), "{out:?}");
    assert!(out.contains(r"\endfoo "), "{out:?}");
}

#[test]
fn newtheorem_ifx_csname_relax_guard_still_declares() {
    // The classic "define once" guard: `\csname thm\endcsname` on an
    // undefined name defines it as `\relax` (real TeX), and pdflatex's
    // `\@ifdefinable`-style check (`\ifx...\relax`) treats a `\relax`-valued
    // name as undefined, so the guard falls through to
    // `\newtheorem{thm}{Theorem}` with no real collision (review round 3,
    // finding #1).
    let r = expand_str(
        r"\expandafter\ifx\csname thm\endcsname\relax\newtheorem{thm}{Theorem}\fi\begin{thm}\end{thm}",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains(r"\thm "), "{out:?}");
    assert!(out.contains(r"\endthm "), "{out:?}");
}

#[test]
fn newtheorem_let_to_relax_guard_still_declares() {
    // Same idea, the other common spelling of the guard.
    let r = expand_str(r"\let\thm\relax\newtheorem{thm}{Theorem}\begin{thm}\end{thm}");
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains(r"\thm "), "{out:?}");
    assert!(out.contains(r"\endthm "), "{out:?}");
}

/// ltdefns.dtx `\@ifdefinable` goes through `\@ifundefined`, so a name
/// `\csname` has just made `\relax` is free: the
/// `\expandafter\newcommand\csname name\endcsname` idiom defines it, as
/// does `\newcommand` after `\let\name\relax`; `\relax` itself stays
/// refused (`\@qrelax`).
#[test]
fn newcommand_defines_a_relax_valued_name() {
    let r = expand_str(r"\expandafter\newcommand\csname foo\endcsname{F}\let\bar\relax\newcommand\bar{B}\foo\bar");
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(text(&r.tokens), "FB");
    let r = expand_str(r"\newcommand\relax{x}");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\relax already defined."]);
}

#[test]
fn newtheorem_relax_itself_is_never_definable() {
    // `\relax`'s own meaning is trivially `Relax`, the same value the
    // guard idiom above uses as a placeholder for "undefined" -- but
    // `\relax` is the primitive itself, not a placeholder, and pdflatex
    // never lets `\newtheorem` (or anything else) redefine it. Regression
    // for the review round-3 fix's own bug: `is_undefined_or_relax`
    // treated `\relax` as available for declaration too.
    let r = expand_str(r"\newtheorem{relax}{Relax}\begin{relax}\end{relax}");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\relax already defined."], "{messages:?}");
    // Rejected and swallowed: \begin{relax}/\end{relax} must not reach the
    // typesetter as \relax/\endrelax tokens (the bug this regresses would
    // have let the declaration through, so both would appear).
    let out = text(&r.tokens);
    assert!(!out.contains(r"\relax "), "{out:?}");
    assert!(!out.contains(r"\endrelax "), "{out:?}");
}

#[test]
fn newtheorem_successful_reclaim_inside_a_group_is_global() {
    // The successful branch claims `\name`/`\end<name>` globally
    // (`assign_cs(..., true)`), so its un-reject of a stale rejection must
    // be global too -- otherwise the group closing at the end of this
    // source resurrects the rejection over what is supposed to be a
    // permanent redeclaration (review round 3, finding #2).
    let r = expand_str(
        r"\def\foo{}\newtheorem{foo}{Foo}{\let\foo\undefined\newtheorem{foo}{Foo}}\begin{foo}\end{foo}",
    );
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\foo already defined."], "{messages:?}");
    let out = text(&r.tokens);
    assert!(out.contains(r"\foo "), "{out:?}");
    assert!(out.contains(r"\endfoo "), "{out:?}");
}

/// Issue #835: `\hspace`/`\vspace` routed through the host shims
/// (`\flashtexhspace`/`\flashtexvspace`, as the compiler's host prelude
/// wires them) splice a bare or factored length register to its value
/// text, so the register never reaches the stomach as an assignment.
const SPACE_SHIM: &str = "\\def\\hspace{\\flashtexhspace}\\def\\vspace{\\flashtexvspace}";

fn space_run(src: &str) -> String {
    run(&format!("{SPACE_SHIM}{src}"))
}

#[test]
fn space_shim_splices_bare_register_like_the() {
    let setup = "\\newlength{\\mylen}\\setlength{\\mylen}{1em}";
    // A bare register behaves exactly like the already-working `\the`
    // form, and the value is the register's fixed-point text.
    assert_eq!(
        space_run(&format!("{setup}\\hspace{{\\mylen}}y")),
        space_run(&format!("{setup}\\hspace{{\\the\\mylen}}y")),
    );
    assert_eq!(
        space_run(&format!("{setup}\\hspace{{\\mylen}}y")),
        "\\flashtexhspacedone 10.0pty",
    );
    // `\vspace` shares the shim.
    assert_eq!(
        space_run(&format!("{setup}\\vspace{{\\mylen}}y")),
        "\\flashtexvspacedone 10.0pty",
    );
}

#[test]
fn space_shim_splices_factor_times_register() {
    let setup = "\\newlength{\\mylen}\\setlength{\\mylen}{1em}\
         \\newlength{\\zerolen}\\setlength{\\zerolen}{0pt}";
    // TeX's `<factor><internal dimen>`: the fixed-point product.
    assert_eq!(
        space_run(&format!("{setup}\\hspace{{2\\mylen}}y")),
        "\\flashtexhspacedone 20.0pty",
    );
    // A leading `-` negates the whole value, as `scan_dimen` does.
    assert_eq!(
        space_run(&format!("{setup}\\hspace{{-\\mylen}}y")),
        "\\flashtexhspacedone -10.0pty",
    );
    // A factor times a register set to 0pt is 0pt, with no diagnostics.
    assert_eq!(
        space_run(&format!("{setup}\\hspace{{2\\zerolen}}y")),
        "\\flashtexhspacedone 0.0pty",
    );
}

#[test]
fn space_shim_leaves_other_arguments_untouched() {
    // A literal dimension passes through for the main loop exactly as
    // before (group tokens stripped by `text`).
    assert_eq!(space_run("\\hspace{1em}y"), "\\flashtexhspacedone 1emy");
    // The star is preserved.
    assert_eq!(
        space_run("\\newlength{\\mylen}\\setlength{\\mylen}{1em}\\hspace*{\\mylen}y"),
        "\\flashtexhspacedone *10.0pty",
    );
    // An undeclared register is not the shim's to report: it passes
    // through with no engine diagnostic (the host parser names it).
    assert_eq!(
        space_run("\\hspace{\\nosuchlen}y"),
        "\\flashtexhspacedone \\nosuchlen y",
    );
}

#[test]
fn newtheorem_second_declaration_of_the_same_name_errors() {
    // Like `\newenvironment`, a repeated declaration keeps the first
    // definition and reports the collision once per redeclaration.
    let r = expand_str(r"\newtheorem{thm}{Theorem}\newtheorem{thm}{Theorem}");
    let messages: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\thm already defined."], "{messages:?}");
    assert!(text(&r.tokens).contains("\\newtheorem "), "{:?}", r.tokens);
}

#[test]
fn newtheorem_failed_shared_counter_leaves_the_name_claimable() {
    // `\newtheorem{widget}[nonexistent]{Widget}` shares the counter of an
    // undeclared environment, so the compiler's `parser.rs::new_theorem`
    // rejects it -- but expansion must not claim `\widget`/`\endwidget`
    // first. The failed declaration burns nothing: a corrected retry on
    // the next line succeeds with no "already defined" error and the
    // environment is usable. Expansion itself stays silent here (no second
    // diagnostic); the one diagnostic for the bad `shared` counter comes
    // from the compiler, which owns that check.
    let r = expand_str(
        "\\newtheorem{widget}[nonexistent]{Widget}\n\\newtheorem{widget}{Widget}\\begin{widget}Hi\\end{widget}",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains("Hi"), "{out:?}");
    assert!(out.contains(r"\widget "), "{out:?}");
    assert!(out.contains(r"\endwidget "), "{out:?}");
}

#[test]
fn newtheorem_shared_counter_control_sequence_does_not_falsely_match_a_name() {
    // `bracket_arg_text` must not strip the backslash off a control
    // sequence in `[shared]`: `scan_through_bracket` reads with `next_raw`
    // (no expansion), so `[\base]` stays the literal token `\base`, and the
    // compiler's own `parser.rs::new_theorem` never re-expands its bracket
    // argument either -- both sides must compare the same raw
    // representation. Before this fix, stripping the backslash turned
    // `\base` into the string "base", which coincidentally matched the
    // *name* of an unrelated, already-declared theorem environment
    // (`\newtheorem{base}{Base}`), letting a declaration through that both
    // pdflatex and the compiler's own check reject. `\def\base{notcounter}`
    // is part of the reported repro; this check does not depend on macro
    // expansion of `\base` (neither side performs it), only on the two
    // sides agreeing about what `[\base]`'s raw text is.
    let r = expand_str(
        "\\newtheorem{base}{Base}\\def\\base{notcounter}\\newtheorem{alias}[\\base]{Alias}\n\\newtheorem{alias}{Alias}\\begin{alias}Hi\\end{alias}",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains("Hi"), "{out:?}");
    assert!(out.contains(r"\alias "), "{out:?}");
    assert!(out.contains(r"\endalias "), "{out:?}");
}

#[test]
fn newtheorem_shared_counter_macro_is_expanded_before_the_existence_check() {
    // The `[shared]` counter name is an expanded context like the `{name}`
    // argument (`scan_through_group(true)`): `\base`, a `\def`-defined
    // macro for the real counter `thm`, must expand to `thm` before the
    // existence check, so this declaration claims `\cor` exactly like a
    // literal `[thm]` would -- not the raw token text `\base`, which
    // matches no declared environment. The expanded name is also what is
    // handed downstream, so the compiler's own raw read of the bracket
    // agrees with this check instead of rejecting it.
    let r = expand_str(
        r"\newtheorem{thm}{Theorem}\def\base{thm}\newtheorem{cor}[\base]{Corollary}\begin{cor}Hi\end{cor}",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains("Hi"), "{out:?}");
    assert!(out.contains("[thm]"), "{out:?}");
    assert!(out.contains(r"\cor "), "{out:?}");
    assert!(out.contains(r"\endcor "), "{out:?}");
}

#[test]
fn newtheorem_valid_shared_counter_still_declares() {
    // A counter-sharing declaration that refers to an earlier, successful
    // `\newtheorem` keeps working exactly as before: both names are
    // claimed and the shared-counter environment is usable.
    let r = expand_str(
        r"\newtheorem{first}{First}\newtheorem{second}[first]{Second}\begin{second}Hi\end{second}",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains("Hi"), "{out:?}");
    assert!(out.contains(r"\second "), "{out:?}");
    assert!(out.contains(r"\endsecond "), "{out:?}");
}

#[test]
fn newtheorem_declaration_inside_a_group_is_globally_visible() {
    // `\newtheorem` is a global declaration in real LaTeX (like
    // `\newcommand`): a declaration inside a group stays visible and usable
    // after the group closes.
    let r = expand_str(r"{\newtheorem{x}{X}}\begin{x}Hi\end{x}");
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = text(&r.tokens);
    assert!(out.contains("Hi"), "{out:?}");
    assert!(out.contains(r"\x "), "{out:?}");
    assert!(out.contains(r"\endx "), "{out:?}");
}


// The LaTeX kernel's own `\newif` switches, font defaults, `\@setfontsize`
// and `\newinsert` allocation are in the prelude, so a project's `.cls`/
// `.sty` that tests or redefines them behaves as under pdflatex (oracle
// probes p1-p4 of lane PARITY-CLS-STY-READING, TeX Live 2026).

#[test]
fn kernel_switches_are_predefined() {
    // pdflatex: `\if@compatibility` false, `\if@filesw` true, the rest false.
    assert_eq!(
        run(r"\makeatletter\if@compatibility compat\else nocompat\fi\space\if@filesw fw\else nofw\fi\space\if@twoside two\else one\fi\space\@mparswitchtrue\if@mparswitch mp\else nomp\fi\makeatother"),
        "nocompat fw one mp"
    );
    // `\if@compatibility\else ... \fi` is the classes' guard around their
    // `\DeclareOption`s: no "Extra \else".
    assert_eq!(run(r"\makeatletter\if@compatibility\else B\fi\makeatother"), "B");
}

#[test]
fn setfontsize_takes_its_arguments_unexpanded() {
    // A class's `\def\normalsize{\@setfontsize\normalsize\@xpt\@xiipt}`:
    // `#1` is `\normalsize` itself, taken as a token, so calling
    // `\normalsize` must not recurse (it used to overflow the input stack).
    let (out, diags) = run_allow_diag(r"\makeatletter\def\normalsize{\@setfontsize\normalsize\@xpt\@xiipt}\normalsize\makeatother x");
    assert_eq!(diags, 0);
    assert_eq!(out, "\\relax \\fontsize 1012\\selectfont x");
    // pdflatex: \meaning\@setfontsize
    assert_eq!(
        run(r"\makeatletter\meaning\@setfontsize\makeatother"),
        "macro:#1#2#3->\\@nomath #1\\ifx \\protect \\@typeset@protect \\let \\@currsize #1\\fi \\fontsize {#2}{#3}\\selectfont "
    );
    // `\@currsize` remembers the size command.
    assert_eq!(
        run(r"\makeatletter\def\small{\@setfontsize\small\@ixpt{11}}\small\meaning\@currsize\makeatother"),
        "\\relax \\fontsize 911\\selectfont macro:->\\@setfontsize \\small \\@ixpt {11}"
    );
}

#[test]
fn kernel_font_defaults_are_defined_and_renewable() {
    // pdflatex: cmr/cmss/cmtt/cmr/m/n/b/OT1
    assert_eq!(
        run(r"\rmdefault/\sfdefault/\ttdefault/\familydefault/\seriesdefault/\shapedefault/\bfdefault/\encodingdefault"),
        "cmr/cmss/cmtt/cmr/m/n/b/OT1"
    );
    // The times.sty idiom every IEEEtran/neurips/spconf file uses.
    assert_eq!(
        run(r"\renewcommand{\sfdefault}{phv}\renewcommand{\rmdefault}{ptm}\renewcommand{\baselinestretch}{1.2}\rmdefault/\sfdefault/\familydefault/\baselinestretch"),
        "ptm/phv/ptm/1.2"
    );
}

#[test]
fn newinsert_allocates_downward_from_the_kernel_inserts() {
    // pdflatex: \footins is 253 (\@mpfootins took 254); the first
    // `\newinsert` of a class gets 252, and `\skip\footins` is skip 253.
    assert_eq!(run(r"\makeatletter\number\footins\makeatother"), "253");
    assert_eq!(run(r"\makeatletter\newinsert\myins\number\myins/\number\@mpfootins\makeatother"), "252/254");
    // (`\relax` ends the glue scan: a `\the` right after `plus 12pt` is
    // expanded while TeX looks for `minus`, before the value is stored.)
    assert_eq!(run(r"\makeatletter\skip\footins 12pt plus 12pt\relax\the\skip\footins\makeatother"), "\\relax 12.0pt plus 12.0pt");
}

