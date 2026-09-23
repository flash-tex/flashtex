#!/usr/bin/env python3
"""
Oracle corpus generator for crates/tex-expansion.

Each case is (id, setup, expr):
  - `setup` is executed as ordinary top-level TeX commands (definitions,
    assignments) -- it must produce no visible characters of its own.
  - `expr` is a purely *expandable* fragment (macro calls, \\the, \\number,
    \\romannumeral, \\string, \\csname, conditionals used as expressions,
    etc) with no bare assignments, since it is captured via a real TeX
    `\\write`, whose argument is scanned in "expand only" mode (exactly
    like `\\edef`'s body) -- assignments embedded there would NOT execute
    and would corrupt the captured output.

For each case this script:
  1. Writes tmp/<id>.tex running `setup` then `\\immediate\\write\\out{expr}`.
  2. Runs real `tex` (plain format directly on primitives -- no plain.tex
     macro dependency) in batchmode, oracle-only per KC-101 (never in the
     product path).
  3. Saves the captured line to tests/oracle/expected/<id>.txt.
  4. Appends a manifest entry (id -> full source `setup ++ expr`, which is
     exactly the string fed to the Rust engine's `expand_str`) to
     tests/oracle/manifest.json.

Cases needing the LaTeX kernel (`\newcommand`, counters, `\@ifnextchar`,
...) set `latex=True` and are run through `pdflatex` with a minimal
`article` preamble instead of bare `tex`.

Run: python3 tests/oracle/gen/cases.py   (from crates/tex-expansion/)
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ORACLE_DIR = os.path.dirname(HERE)
TMP_DIR = os.path.join(ORACLE_DIR, "tmp")
EXPECTED_DIR = os.path.join(ORACLE_DIR, "expected")
MANIFEST_PATH = os.path.join(ORACLE_DIR, "manifest.json")

os.makedirs(TMP_DIR, exist_ok=True)
os.makedirs(EXPECTED_DIR, exist_ok=True)

# (id, setup, expr, latex)
CASES = [
    # -- \def / \let / basic macro calling --------------------------------
    ("def_simple", r"\def\a{hello}", r"\a", "tex"),
    ("def_with_arg", r"\def\greet#1{Hello, #1!}", r"\greet{world}", "tex"),
    ("def_delimited", r"\def\a#1;{[#1]}", r"\a hello;", "tex"),
    ("def_two_delimited", r"\def\a#1,#2.{(#1)(#2)}", r"\a x,y.", "tex"),
    ("let_to_macro", r"\def\a{X}\let\b=\a", r"\b", "tex"),
    # NOTE: no `let_to_char` case here: `\let\a=b` makes `\a` a
    # non-expandable "let to character" token. Real TeX's `\write`
    # captures such tokens as the literal control sequence "\a " (since
    # `\write`'s scan only expands *expandable* tokens and copies
    # everything else verbatim) -- but our engine intentionally
    # substitutes the underlying character immediately, matching what
    # real TeX's *main control* does when such a token reaches
    # typesetting (append the character, exactly as if `b` had been
    # typed). Those are two different real-TeX subsystems with two
    # different observable behaviors for the same construct; since this
    # crate's output feeds typesetting (not `\write`), `\write` is the
    # wrong oracle for this one case. See CONTRACT.md "Known deviations".
    ("def_nested_call", r"\def\inner{IN}\def\outer{[\inner]}", r"\outer", "tex"),
    ("def_hash_hash", r"\def\a{\#}", r"\a", "tex"),
    # NOTE: no oracle case for `#{` (brace-delimited last parameter) here.
    # `#{` leaves the delimiting `{` unconsumed for whatever follows the
    # call (TeXbook p.205); under real *document* typesetting (main
    # control) that leftover `{...}` is a silent group (matching this
    # crate's design -- see CONTRACT.md "Known deviations"), but `\write`'s
    # argument scan is a `scan_toks` context that always preserves braces
    # literally, so it cannot validate this construct without producing a
    # misleading expectation. Covered instead by the Rust unit test
    # `def_brace_delim_last` in tests/expand_tests.rs.
    # -- \edef / \noexpand / \expandafter ----------------------------------
    ("edef_expands_body", r"\def\a{X}\edef\b{[\a]}", r"\b", "tex"),
    # \meaning rather than a bare \b: \write prints a let-to-\relax token
    # by its own name, while main control (what this crate feeds the
    # typesetter) emits it as \relax -- see CONTRACT.md.
    ("edef_noexpand", r"\def\a{X}\edef\b{\noexpand\a}\let\a=\relax", r"\meaning\b", "tex"),
    ("expandafter_basic", r"\def\a{X}\def\b{\a}\expandafter\def\expandafter\c\expandafter{\b}", r"\c", "tex"),
    ("expandafter_chain", r"\def\a{A}\def\b{B}\def\pick#1#2{#1}", r"\expandafter\pick\expandafter{\a}{\b}", "tex"),
    # -- \csname / \string / \number / \romannumeral ------------------------
    ("csname_builds", r"\def\foo{bar}", r"\csname foo\endcsname", "tex"),
    ("csname_undefined_relax", r"", r"\ifx\csname zzz\endcsname\relax DEF\else NOTDEF\fi", "tex"),
    ("string_of_cs", r"", r"\string\foo", "tex"),
    ("string_of_char", r"", r"\string a", "tex"),
    ("number_basic", r"", r"\number 42", "tex"),
    ("number_negative", r"", r"\number -7", "tex"),
    ("romannumeral_basic", r"", r"\romannumeral 1994", "tex"),
    ("romannumeral_zero", r"", r"\romannumeral 0", "tex"),
    # -- registers / \the / arithmetic --------------------------------------
    ("count_the", r"\count0=5 ", r"\the\count0", "tex"),
    ("count_advance", r"\count0=5 \advance\count0 by 3 ", r"\the\count0", "tex"),
    ("count_multiply", r"\count0=5 \multiply\count0 by 3 ", r"\the\count0", "tex"),
    ("count_divide", r"\count0=15 \divide\count0 by 3 ", r"\the\count0", "tex"),
    ("countdef_alias", r"\countdef\mycount=5 \mycount=7 ", r"\the\mycount", "tex"),
    ("dimen_pt", r"\dimen0=12pt ", r"\the\dimen0", "tex"),
    ("dimen_in", r"\dimen0=1in ", r"\the\dimen0", "tex"),
    ("numexpr_basic", r"\count0=\numexpr 2+3*4\relax ", r"\the\count0", "etex"),
    ("numexpr_parens", r"\count0=\numexpr (2+3)*4\relax ", r"\the\count0", "etex"),
    # -- grouping / \global / \aftergroup -----------------------------------
    ("group_restores_local", r"\def\a{outer}{\def\a{inner}}", r"\a", "tex"),
    ("group_global_survives", r"\def\a{outer}{\global\def\a{inner}}", r"\a", "tex"),
    ("begingroup_endgroup_count", r"\count0=1 \begingroup\count0=2 \endgroup", r"\the\count0", "tex"),
    # -- conditionals --------------------------------------------------------
    ("iftrue_basic", r"", r"\iftrue A\else B\fi", "tex"),
    ("iffalse_basic", r"", r"\iffalse A\else B\fi", "tex"),
    ("ifnum_lt", r"", r"\ifnum 3<5 yes\else no\fi", "tex"),
    ("ifnum_gt", r"", r"\ifnum 5<3 yes\else no\fi", "tex"),
    ("ifnum_eq", r"", r"\ifnum 5=5 yes\else no\fi", "tex"),
    ("ifdim_lt", r"", r"\ifdim 1pt<2pt yes\else no\fi", "tex"),
    ("ifodd_true", r"", r"\ifodd 3 odd\else even\fi", "tex"),
    ("ifodd_false", r"", r"\ifodd 4 odd\else even\fi", "tex"),
    ("ifx_same", r"\def\a{X}\def\b{X}", r"\ifx\a\b same\else different\fi", "tex"),
    ("ifx_diff", r"\def\a{X}\def\b{Y}", r"\ifx\a\b same\else different\fi", "tex"),
    ("ifcase_two", r"", r"\ifcase 2 zero\or one\or two\or three\fi", "tex"),
    ("ifcase_zero", r"", r"\ifcase 0 zero\or one\or two\fi", "tex"),
    ("if_char_eq", r"", r"\if ab same\else diff\fi", "tex"),
    ("if_char_same", r"", r"\if aa same\else diff\fi", "tex"),
    ("ifcat_letters", r"", r"\ifcat ab same\else diff\fi", "tex"),
    ("ifcat_diff", r"", r"\ifcat a1 same\else diff\fi", "tex"),
    ("nested_if_else", r"", r"\iftrue \iffalse T\else F\fi \else O\fi", "tex"),
    ("newif_true", r"\newif\ifmyflag \myflagtrue", r"\ifmyflag YES\else NO\fi", "tex"),
    ("newif_false", r"\newif\ifmyflag \myflagfalse", r"\ifmyflag YES\else NO\fi", "tex"),
    ("unless_iftrue", r"", r"\unless\iftrue A\else B\fi", "etex"),
    # -- LaTeX layer (rendered as document body, captured via pdftotext,
    # since these macros use \let/\def/\futurelet internally and so
    # cannot be captured through \write's edef-like restricted scan) ------
    ("newcommand_basic", r"\newcommand{\hi}{Hello}", r"\hi", "latex-render"),
    ("newcommand_arg", r"\newcommand{\greet}[1]{Hello #1}", r"\greet{World}", "latex-render"),
    ("newcommand_opt_default", r"\newcommand{\greet}[2][Hi]{#1, #2}", r"\greet{World}", "latex-render"),
    ("newcommand_opt_given", r"\newcommand{\greet}[2][Hi]{#1, #2}", r"\greet[Yo]{World}", "latex-render"),
    ("renewcommand_basic", r"\newcommand{\mytestcmd}{old}\renewcommand{\mytestcmd}{new}", r"\mytestcmd", "latex-render"),
    ("providecommand_keeps", r"\newcommand{\mytestcmd}{old}\providecommand{\mytestcmd}{new}", r"\mytestcmd", "latex-render"),
    ("newenvironment_basic", r"\newenvironment{myenv}{[BEGIN]}{[END]}", r"\begin{myenv}content\end{myenv}", "latex-render"),
    ("newcounter_setcounter", r"\newcounter{foo}\setcounter{foo}{5}", r"\arabic{foo}", "latex-render"),
    ("stepcounter_twice", r"\newcounter{foo}\stepcounter{foo}\stepcounter{foo}", r"\arabic{foo}", "latex-render"),
    ("addtocounter_basic", r"\newcounter{foo}\setcounter{foo}{3}\addtocounter{foo}{4}", r"\arabic{foo}", "latex-render"),
    ("roman_counter", r"\newcounter{foo}\setcounter{foo}{4}", r"\roman{foo}", "latex-render"),
    ("Roman_counter", r"\newcounter{foo}\setcounter{foo}{4}", r"\Roman{foo}", "latex-render"),
    ("alph_counter", r"\newcounter{foo}\setcounter{foo}{1}", r"\alph{foo}", "latex-render"),
    ("Alph_counter", r"\newcounter{foo}\setcounter{foo}{2}", r"\Alph{foo}", "latex-render"),
    (
        "ifnextchar_star",
        r"\makeatletter\def\test{\@ifnextchar*{\@teststar}{\@testnostar}}\def\@teststar*{STAR}\def\@testnostar{NOSTAR}\makeatother",
        r"\test*",
        "latex-render",
    ),
    (
        "ifnextchar_nostar",
        r"\makeatletter\def\test{\@ifnextchar*{\@teststar}{\@testnostar}}\def\@teststar*{STAR}\def\@testnostar{NOSTAR}\makeatother",
        r"\test ",
        "latex-render",
    ),
    ("ifstar_star", r"\makeatletter\def\test{\@ifstar{STARBRANCH}{NOSTARBRANCH}}\makeatother", r"\test*", "latex-render"),
    ("nameuse_namedef", r"\makeatletter\@namedef{foo}{bar}", r"\@nameuse{foo}\makeatother", "latex-render"),
    # -- additional coverage: more def/param/register/conditional variety ----
    ("def_no_args_reuse", r"\def\x{A}\def\y{\x\x\x}", r"\y", "tex"),
    ("def_param_repeated", r"\def\dup#1{#1#1}", r"\dup{ab}", "tex"),
    ("def_three_params", r"\def\three#1#2#3{#3#2#1}", r"\three{a}{b}{c}", "tex"),
    ("let_chain", r"\def\a{X}\let\b=\a\let\c=\b", r"\c", "tex"),
    ("edef_concat", r"\def\a{A}\def\b{B}\edef\c{\a\b}", r"\c", "tex"),
    ("edef_the_count", r"\count5=9 \edef\c{\the\count5}\count5=0 ", r"\c", "tex"),
    ("csname_expandafter", r"\def\foo{FOO}", r"\expandafter\csname foo\endcsname", "tex"),
    ("string_of_digit", r"", r"\string 5", "tex"),
    ("number_of_count", r"\count0=99 ", r"\number\count0", "tex"),
    ("romannumeral_small", r"", r"\romannumeral 9", "tex"),
    ("romannumeral_large", r"", r"\romannumeral 3888", "tex"),
    ("count_octal", r"\count0='17 ", r"\the\count0", "tex"),
    ("count_hex", r'\count0="FF ', r"\the\count0", "tex"),
    ("count_backtick", r"\count0=`A ", r"\the\count0", "tex"),
    ("count_negative_advance", r"\count0=5 \advance\count0 by -8 ", r"\the\count0", "tex"),
    ("dimen_cm", r"\dimen0=2.5cm ", r"\the\dimen0", "tex"),
    ("dimen_negative", r"\dimen0=-3pt ", r"\the\dimen0", "tex"),
    ("skip_the", r"\skip0=1pt plus 2pt minus 3pt ", r"\the\skip0", "tex"),
    ("group_nested_three_deep", r"\def\a{L0}{\def\a{L1}{\def\a{L2}}}", r"\a", "tex"),
    ("ifnum_ge_via_not_lt", r"", r"\ifnum 5<5 lt\else \ifnum 5>5 gt\else eq\fi\fi", "tex"),
    ("ifdim_gt", r"", r"\ifdim 3pt>1pt yes\else no\fi", "tex"),
    ("ifx_undefined_both", r"", r"\ifx\undefinedaaa\undefinedbbb same\else different\fi", "tex"),
    ("ifx_primitive_same", r"", r"\ifx\relax\relax same\else different\fi", "tex"),
    ("ifcase_negative_falls_to_else", r"", r"\ifcase -1 zero\or one\else other\fi", "tex"),
    ("ifcase_beyond_or_falls_to_else", r"", r"\ifcase 5 zero\or one\else other\fi", "tex"),
    ("nested_ifcase_in_iftrue", r"", r"\iftrue\ifcase 1 a\or b\or c\fi\fi", "tex"),
    ("newif_default_false", r"\newif\ifmyflag ", r"\ifmyflag YES\else NO\fi", "tex"),
    ("unless_iffalse", r"", r"\unless\iffalse A\else B\fi", "etex"),
    ("dimexpr_basic", r"\dimen0=\dimexpr 2pt+3pt\relax ", r"\the\dimen0", "etex"),
    ("numexpr_division", r"\count0=\numexpr 17/3\relax ", r"\the\count0", "etex"),
    ("numexpr_nested_parens", r"\count0=\numexpr (2+3)*(4-1)\relax ", r"\the\count0", "etex"),
    ("gdef_survives_extra_group", r"\makeatletter{\gdef\foo@bar{Y}}", r"\foo@bar", "tex"),
    ("catcode_tilde_as_active", r"\catcode`\~=13 \def~{TILDE}", r"~", "tex"),
    ("countdef_then_advance", r"\countdef\mycount=9 \mycount=1 \advance\mycount by 4 ", r"\the\mycount", "tex"),
    ("global_inside_nested_group", r"\count0=1 {{\global\count0=9 }}", r"\the\count0", "tex"),
    ("newcommand_two_args", r"\newcommand{\pair}[2]{(#1,#2)}", r"\pair{x}{y}", "latex-render"),
    ("newcounter_within_arabic", r"\newcounter{sec}\newcounter{sub}[sec]\setcounter{sub}{3}", r"\arabic{sub}", "latex-render"),
    ("refstepcounter_basic", r"\newcounter{foo}\refstepcounter{foo}\refstepcounter{foo}", r"\arabic{foo}", "latex-render"),
    # NOTE: no oracle case for `\fnsymbol` -- real LaTeX renders it as
    # math-mode footnote symbols (asterisk-operator, dagger, ...), which
    # are not representable as plain catcode-Other characters at all; our
    # engine deliberately substitutes ASCII approximations ("*", "**",
    # "#", ...), documented as a known limitation in CONTRACT.md. A
    # pdftotext-captured Unicode glyph could never equal that
    # approximation, so no oracle comparison is meaningful here.
    ("value_in_setcounter", r"\newcounter{foo}\setcounter{foo}{4}\newcounter{bar}\setcounter{bar}{\value{foo}}", r"\arabic{bar}", "latex-render"),
]

# ======================= round 2 (KC-101 r2) =============================
# Modes added in round 2:
#   latex-write   pdflatex; setup + \immediate\write{expr} in the document body
#   latex-doc     pdflatex; setup in the preamble, write after \begin{document}
#   tex-err / etex-err / latex-err
#                 run setup+expr; expected = the "! ..." error lines of the log
CASES += [
    # -- \@ifundefined -------------------------------------------------------
    ("ifundefined_undef", r"\makeatletter", r"\@ifundefined{zzq}{U}{D}", "latex-render"),
    ("ifundefined_def", r"\makeatletter\def\zzq{}", r"\@ifundefined{zzq}{U}{D}", "latex-render"),
    ("ifundefined_relax", r"\makeatletter\let\zzq\relax", r"\@ifundefined{zzq}{U}{D}", "latex-render"),
    ("ifundefined_after_group", r"\makeatletter{\def\zzq{}}", r"\@ifundefined{zzq}{U}{D}", "latex-render"),
    ("ifundefined_write", r"\makeatletter\def\zzq{}", r"\@ifundefined{zzq}{U}{D}\@ifundefined{zzr}{U}{D}", "latex-write"),
    # -- \DeclareRobustCommand / \protect / \protected@edef ------------------
    ("robust_render", r"\DeclareRobustCommand\zz[1]{(#1)}", r"\zz{a}\zz b", "latex-render"),
    ("robust_meaning", r"\DeclareRobustCommand\zz{R}", r"\meaning\zz", "latex-write"),
    ("robust_inner_meaning", r"\DeclareRobustCommand\zz[1]{(#1)}", r"\expandafter\meaning\csname zz \endcsname", "latex-write"),
    ("robust_opt_render", r"\DeclareRobustCommand\zz[1][d]{(#1)}", r"\zz\zz[x]", "latex-render"),
    ("robust_opt_inner_meaning", r"\DeclareRobustCommand\zz[1][d]{(#1)}", r"\expandafter\meaning\csname zz \endcsname", "latex-write"),
    ("protected_edef_meaning", r"\makeatletter\DeclareRobustCommand\zz{R}\protected@edef\zq{a\zz b}", r"\meaning\zq", "latex-write"),
    ("protected_edef_plain_macro", r"\makeatletter\def\zy{Y}\protected@edef\zq{a\zy b}", r"\meaning\zq", "latex-write"),
    ("makeuppercase_render", r"", r"\MakeUppercase{abc}", "latex-render"),
    ("makelowercase_render", r"", r"\MakeLowercase{AbC}", "latex-render"),
    # -- \unexpanded / \detokenize / \expanded -------------------------------
    ("unexpanded_in_edef", r"\def\a{A}\edef\x{\unexpanded{\a}\a}", r"\meaning\x", "etex"),
    # In main control (not an expand-only context) \unexpanded's tokens are
    # put back and expanded normally.
    ("unexpanded_main_control", r"\def\zq{A}", r"\unexpanded{\zq}\zq", "latex-render"),
    ("unexpanded_hash_in_edef", r"\toks0={#}\edef\x{\unexpanded{\the\toks0}}", r"\meaning\x", "etex"),
    ("detokenize_basic", r"", r"\detokenize{\foo bar}", "etex"),
    ("detokenize_hash", r"", r"\detokenize{#}", "etex"),
    ("detokenize_cs_spacing", r"", r"\detokenize{\a\b1\c}", "etex"),
    ("detokenize_control_symbol", r"", r"\detokenize{\%\ab}", "etex"),
    ("detokenize_atletter", r"\catcode`\@=11 ", r"\detokenize{\foo@}", "etex"),
    ("expanded_basic", r"\def\a{A}\def\b{\a\a}", r"\expanded{\b}", "etex"),
    ("expanded_noexpand", r"\def\a{A}\edef\x{\expandafter\noexpand\expanded{\noexpand\a}}", r"\meaning\x", "etex"),
    ("expanded_unexpanded", r"\def\a{A}", r"\expanded{\unexpanded{\a}}", "etex"),
    ("edef_detokenize", r"\edef\x{\detokenize{\a}}", r"\meaning\x", "etex"),
    ("expanded_in_edef", r"\def\a{A}\def\b{\a}\edef\x{\expanded{\noexpand\b}}", r"\meaning\x", "etex"),
    # -- \ifdefined / \ifcsname ----------------------------------------------
    ("ifdefined_undef", r"", r"\ifdefined\zzq Y\else N\fi", "etex"),
    ("ifdefined_relax", r"\let\zzq\relax", r"\ifdefined\zzq Y\else N\fi", "etex"),
    ("ifdefined_unless", r"", r"\unless\ifdefined\zzq Y\else N\fi", "etex"),
    ("ifcsname_undef", r"", r"\ifcsname zzq\endcsname Y\else N\fi", "etex"),
    ("ifcsname_defined", r"\def\zzq{}", r"\ifcsname zzq\endcsname Y\else N\fi", "etex"),
    ("ifcsname_after_csname", r"", r"\expandafter\ifx\csname zzq\endcsname\relax\fi\ifcsname zzq\endcsname Y\else N\fi", "etex"),
    ("ifcsname_group_restore", r"{\def\zzq{}}", r"\ifcsname zzq\endcsname Y\else N\fi", "etex"),
    ("ifdefined_csname_local", r"{\expandafter\ifx\csname zzq\endcsname\relax\fi}", r"\ifdefined\zzq Y\else N\fi", "etex"),
    ("ifcsname_expands", r"\def\n{zz}\def\zzq{}", r"\ifcsname\n q\endcsname Y\else N\fi", "etex"),
    # -- \scantokens ------------------------------------------------------------
    ("scantokens_basic", r"", r"\scantokens{xy}", "latex-render"),
    ("scantokens_defines", r"", r"\scantokens{\def\zq{SX}}\zq", "latex-render"),
    ("scantokens_retokenizes", r"\edef\zt{\detokenize{\def\zq{QQ}}}", r"\expandafter\scantokens\expandafter{\zt}\zq", "latex-render"),
    # -- \afterassignment -------------------------------------------------------
    ("afterassign_def", r"\def\b{\def\c{C}}\afterassignment\b\def\a{A}", r"\c", "tex"),
    ("afterassign_count", r"\def\b{\count1=7 }\afterassignment\b\count0=3 ", r"\the\count0,\the\count1", "tex"),
    ("afterassign_let", r"\def\b{\def\c{L}}\afterassignment\b\let\a\relax", r"\c", "tex"),
    ("afterassign_last_wins", r"\def\b{\def\c{1}}\def\d{\def\c{2}}\afterassignment\b\afterassignment\d\def\a{}", r"\c", "tex"),
    ("afterassign_chardef", r"\def\b{\def\c{\the\e}}\afterassignment\b\chardef\e=66 ", r"\c", "tex"),
    # -- \uppercase / \lowercase with \uccode / \lccode ----------------------
    ("uppercase_def", r"\uppercase{\def\x{abc}}", r"\x", "tex"),
    ("lowercase_def", r"\lowercase{\def\x{ABC}}", r"\x", "tex"),
    ("uppercase_cs_untouched", r"\def\ab{q}\uppercase{\def\x{\ab}}", r"\meaning\x", "tex"),
    ("uccode_custom", r"\uccode`\a=`\Z \uppercase{\def\x{ab}}", r"\x", "tex"),
    ("uccode_zero", r"\uccode`\a=0 \uppercase{\def\x{ab}}", r"\x", "tex"),
    ("lccode_custom", r"\lccode`\Q=`\z \lowercase{\def\x{QR}}", r"\x", "tex"),
    ("the_uccode_lccode", r"", r"\the\uccode`\a,\the\lccode`\A,\the\uccode`\1", "tex"),
    ("uppercase_nonletters", r"\uppercase{\def\x{a1-b}}", r"\x", "tex"),
    ("uccode_grouped", r"{\uccode`\a=`\Z }\uppercase{\def\x{a}}", r"\x", "tex"),
    ("uppercase_other_catcode", r"\edef\y{\string a}\expandafter\uppercase\expandafter{\expandafter\def\expandafter\x\expandafter{\y}}", r"\x", "tex"),
    # -- \chardef / \mathchardef ------------------------------------------------
    ("chardef_the", r"\chardef\c=65 ", r"\the\c", "tex"),
    ("chardef_meaning", r"\chardef\c=65 ", r"\meaning\c", "tex"),
    ("mathchardef_meaning", r'\mathchardef\m="1234 ', r"\meaning\m", "tex"),
    ("chardef_number", r"\chardef\c=65 ", r"\number\c", "tex"),
    ("chardef_in_count", r"\chardef\c=12 \count0=\c ", r"\the\count0", "tex"),
    ("chardef_ifnum", r"\chardef\c=65 ", r"\ifnum\c=65 Y\else N\fi", "tex"),
    ("chardef_group_local", r"{\chardef\zzq=1 }", r"\meaning\zzq", "tex"),
    ("mathchardef_the", r'\mathchardef\m="7161 ', r"\the\m", "tex"),
    # -- \futurelet idioms --------------------------------------------------------
    ("futurelet_brace", r"\def\zt{\futurelet\zn\ztt}\def\ztt{\ifx\zn\bgroup B\else N\fi}", r"\zt{x}\zt x", "latex-render"),
    ("futurelet_letter", r"\def\zt{\futurelet\zn\ztt}\def\ztt{\ifx\zn a A\else O\fi}", r"\zt a\zt b", "latex-render"),
    ("futurelet_relax", r"\def\zt{\futurelet\zn\ztt}\def\ztt{\ifx\zn\relax R\else O\fi}", r"\zt\relax x\zt y", "latex-render"),
    ("ifnextchar_bracket", r"\makeatletter\def\zt{\@ifnextchar[{O}{N}}", r"\zt[x]\zt x", "latex-render"),
    ("ifnextchar_skips_space", r"\makeatletter\def\zt{\@ifnextchar[{O}{N}}", r"\zt [x]", "latex-render"),
    ("futurelet_meaning_write", r"\futurelet\zn\def\zm{}", r"\meaning\zn", "tex"),
    # -- \@for / \@tfor -----------------------------------------------------------
    ("for_basic", r"\makeatletter", r"\@for\zi:=a,b,c\do{(\zi)}", "latex-render"),
    ("for_macro_list", r"\makeatletter\def\zl{x,yy,z}", r"\@for\zi:=\zl\do{(\zi)}", "latex-render"),
    ("for_single", r"\makeatletter", r"\@for\zi:=q\do{(\zi)}", "latex-render"),
    ("for_braced_item", r"\makeatletter", r"\@for\zi:={a,b},c\do{(\zi)}", "latex-render"),
    ("tfor_basic", r"\makeatletter", r"\@tfor\zi:=abc\do{(\zi)}", "latex-render"),
    ("tfor_groups", r"\makeatletter", r"\@tfor\zi:=a{bc}d\do{(\zi)}", "latex-render"),
    ("for_accumulate", r"\makeatletter\def\zacc{}\@for\zi:=1,2,3\do{\xdef\zacc{\zacc\zi}}", r"\zacc", "latex-write"),
    # -- \loop ... \repeat ----------------------------------------------------------
    ("loop_accumulate", r"\count1=0 \def\acc{}\loop\advance\count1 by 1 \edef\acc{\acc\the\count1}\ifnum\count1<5 \repeat", r"\acc", "tex"),
    ("loop_once", r"\count1=0 \def\acc{}\loop\advance\count1 by 1 \edef\acc{\acc\the\count1}\ifnum\count1<0 \repeat", r"\acc", "tex"),
    ("loop_countdown", r"\count1=3 \def\acc{}\loop\edef\acc{\acc\the\count1}\advance\count1 -1 \ifnum\count1>0 \repeat", r"\acc", "tex"),
    ("loop_latex", r"\newcount\zc \def\zacc{}\loop\advance\zc by 1 \edef\zacc{\zacc[\the\zc]}\ifnum\zc<3 \repeat", r"\zacc", "latex-write"),
    # -- \g@addto@macro -----------------------------------------------------------
    ("addto_basic", r"\makeatletter\def\zx{A}\g@addto@macro\zx{B}", r"\zx", "latex-write"),
    ("addto_in_group", r"\makeatletter\def\zx{A}{\g@addto@macro\zx{B}}", r"\zx", "latex-write"),
    ("addto_empty_meaning", r"\makeatletter\let\zx\@empty\g@addto@macro\zx{\foo}", r"\meaning\zx", "latex-write"),
    ("addto_twice", r"\makeatletter\def\zx{A}\g@addto@macro\zx{B}\g@addto@macro\zx{C}", r"\zx", "latex-write"),
    # -- \AtBeginDocument / \AtEndDocument ----------------------------------------
    ("atbegindocument_basic", r"\AtBeginDocument{\gdef\zx{hooked}}", r"\zx", "latex-doc"),
    ("atbegindocument_order", r"\def\zx{}\AtBeginDocument{\xdef\zx{\zx a}}\AtBeginDocument{\xdef\zx{\zx b}}", r"\zx", "latex-doc"),
    ("atbegindocument_in_body", r"\AtBeginDocument{BODY}", r"", "latex-render"),
    # -- keyval -------------------------------------------------------------------
    ("keyval_basic", r"\makeatletter\define@key{fam}{k}{(#1)}", r"\setkeys{fam}{k=v}", "latex-render"),
    ("keyval_two", r"\makeatletter\define@key{fam}{a}{(a#1)}\define@key{fam}{b}{(b#1)}", r"\setkeys{fam}{a=1,b=2}", "latex-render"),
    ("keyval_default", r"\makeatletter\define@key{fam}{k}[dflt]{(#1)}", r"\setkeys{fam}{k}", "latex-render"),
    ("keyval_spaces", r"\makeatletter\define@key{fam}{k}{(#1)}", r"\setkeys{fam}{ k = v , k=w }", "latex-render"),
    ("keyval_braces", r"\makeatletter\define@key{fam}{k}{(#1)}", r"\setkeys{fam}{k={a,b}}", "latex-render"),
    ("keyval_empty_items", r"\makeatletter\define@key{fam}{k}{(#1)}", r"\setkeys{fam}{,k=v,,}", "latex-render"),
    ("keyval_braced_equals", r"\makeatletter\define@key{fam}{k}{(#1)}", r"\setkeys{fam}{k={x=y}}", "latex-render"),
    ("keyval_store", r"\makeatletter\define@key{fam}{k}{\def\zv{#1}}\setkeys{fam}{k=stored}", r"\zv", "latex-write"),
    ("keyval_meaning", r"\makeatletter\define@key{fam}{k}{(#1)}", r"\expandafter\meaning\csname KV@fam@k\endcsname", "latex-write"),
    ("keyval_default_meaning", r"\makeatletter\define@key{fam}{k}[dd]{(#1)}", r"\expandafter\meaning\csname KV@fam@k@default\endcsname", "latex-write"),
    # -- \newlength / \setlength / \addtolength -----------------------------------
    ("newlength_zero", r"\newlength\lenA", r"\the\lenA", "latex-write"),
    ("setlength_basic", r"\newlength\lenA\setlength\lenA{3.5pt}", r"\the\lenA", "latex-write"),
    ("addtolength_basic", r"\newlength\lenA\setlength\lenA{3pt}\addtolength\lenA{2pt}", r"\the\lenA", "latex-write"),
    ("setlength_glue", r"\newlength\lenA\setlength\lenA{1pt plus 2pt minus 1fil}", r"\the\lenA", "latex-write"),
    ("setlength_braced_cm", r"\newlength{\lenA}\setlength{\lenA}{2cm}", r"\the\lenA", "latex-write"),
    ("setlength_from_length", r"\newlength\lenA\newlength\lenB\setlength\lenA{4pt}\setlength\lenB{\lenA}", r"\the\lenB", "latex-write"),
    ("setlength_negative_length", r"\newlength\lenA\newlength\lenB\setlength\lenA{4pt}\setlength\lenB{-\lenA}", r"\the\lenB", "latex-write"),
    ("setlength_factor", r"\newlength\lenA\newlength\lenB\setlength\lenA{10pt}\setlength\lenB{0.35\lenA}", r"\the\lenB", "latex-write"),
    ("setlength_in_group", r"\newlength\lenA\setlength\lenA{1pt}{\setlength\lenA{9pt}}", r"\the\lenA", "latex-write"),
    ("addtolength_glue", r"\newlength\lenA\setlength\lenA{1pt plus 1fil}\addtolength\lenA{2pt plus 3fil}", r"\the\lenA", "latex-write"),
    ("newlength_defined_err", r"\newlength\lenA", r"\newlength\lenA", "latex-err"),
    # -- \refstepcounter / \@currentlabel ------------------------------------------
    ("refstep_currentlabel", r"\makeatletter\newcounter{zf}\setcounter{zf}{4}\refstepcounter{zf}", r"\@currentlabel", "latex-write"),
    ("refstep_roman_label", r"\makeatletter\newcounter{zf}\renewcommand\thezf{\roman{zf}}\refstepcounter{zf}\refstepcounter{zf}", r"\@currentlabel", "latex-write"),
    ("refstep_prefix", r"\makeatletter\newcounter{zf}\def\p@zf{P-}\refstepcounter{zf}", r"\@currentlabel", "latex-write"),
    ("refstep_within", r"\makeatletter\newcounter{zp}\newcounter{zc}[zp]\renewcommand\thezc{\thezp.\arabic{zc}}\refstepcounter{zp}\refstepcounter{zc}\refstepcounter{zc}", r"\@currentlabel", "latex-write"),
    # -- \csname implicit \relax --------------------------------------------------
    ("csname_meaning_relax", r"", r"\expandafter\meaning\csname zzq\endcsname", "tex"),
    ("csname_defined_untouched", r"\def\zzq{Q}", r"\csname zzq\endcsname", "tex"),
    ("csname_empty_meaning", r"", r"\expandafter\meaning\csname\endcsname", "tex"),
    ("csname_with_expansion", r"\def\n{zz}\def\zzq{Q}", r"\csname\n q\endcsname", "tex"),
    ("csname_string_space", r"", r"\expandafter\string\csname a b\endcsname", "tex"),
    ("csname_let_relax_ifx", r"\expandafter\let\expandafter\x\csname zzq\endcsname", r"\ifx\x\relax R\else N\fi", "tex"),
    # -- \string with \escapechar ----------------------------------------------------
    ("string_escapechar_neg", r"\escapechar=-1 ", r"\string\foo", "tex"),
    ("string_escapechar_other", r"\escapechar=`\! ", r"\string\foo", "tex"),
    ("string_active", r"\catcode`\~=13 ", r"\string~", "tex"),
    ("string_control_space", r"", r"\string\ |", "tex"),
    ("meaning_escapechar", r"\escapechar=`\/ \def\a{\b}", r"\meaning\a", "tex"),
    ("string_single_letter", r"", r"\string\a|", "tex"),
    ("string_escapechar_in_group", r"{\escapechar=-1 }", r"\string\foo", "tex"),
    ("the_escapechar", r"", r"\the\escapechar", "tex"),
    # -- \meaning for every token kind -----------------------------------------------
    ("meaning_letter", r"", r"\meaning a", "tex"),
    ("meaning_other", r"", r"\meaning 1", "tex"),
    ("meaning_bgroup", r"", r"\meaning\bgroup", "tex"),
    ("meaning_egroup", r"", r"\meaning\egroup", "tex"),
    ("meaning_mathshift", r"", r"\meaning$", "tex"),
    ("meaning_aligntab", r"", r"\meaning&", "tex"),
    ("meaning_superscript", r"", r"\meaning^", "tex"),
    ("meaning_subscript", r"", r"\meaning_", "tex"),
    ("meaning_space", r"\def\:{\let\sp= }\: ", r"\meaning\sp", "tex"),
    ("meaning_active_undefined", r"\catcode`\Z=13 ", r"\meaning Z", "tex"),
    ("meaning_undefined", r"", r"\meaning\zzq", "tex"),
    ("meaning_long_macro", r"\long\def\a#1{x}", r"\meaning\a", "tex"),
    ("meaning_protected_macro", r"\protected\def\a{x}", r"\meaning\a", "etex"),
    ("meaning_protected_long", r"\protected\long\def\a{}", r"\meaning\a", "etex"),
    ("meaning_primitive_count", r"", r"\meaning\count", "tex"),
    ("meaning_primitive_ifx", r"", r"\meaning\ifx", "tex"),
    ("meaning_countdef", r"\countdef\c=5 ", r"\meaning\c", "tex"),
    ("meaning_dimendef", r"\dimendef\d=3 ", r"\meaning\d", "tex"),
    ("meaning_skipdef", r"\skipdef\s=2 ", r"\meaning\s", "tex"),
    ("meaning_toksdef", r"\toksdef\t=4 ", r"\meaning\t", "tex"),
    ("meaning_let_char", r"\let\x=a", r"\meaning\x", "tex"),
    ("meaning_delimited", r"\def\a#1.#2;{}", r"\meaning\a", "tex"),
    ("meaning_brace_delimited", r"\def\a#1#{x}", r"\meaning\a", "tex"),
    ("meaning_nested_hash", r"\def\a{\def\b##1{##1}}", r"\meaning\a", "tex"),
    ("meaning_let_primitive", r"\let\x\ifnum", r"\meaning\x", "tex"),
    ("meaning_par", r"", r"\meaning\par", "tex"),
    ("meaning_param_char", r"", r"\meaning#", "tex"),
    ("meaning_expandafter", r"\def\a{A}", r"\expandafter\meaning\a", "tex"),
    # -- \pdfstrcmp ------------------------------------------------------------------
    ("strcmp_less", r"", r"\pdfstrcmp{a}{b}", "etex"),
    ("strcmp_greater", r"", r"\pdfstrcmp{b}{a}", "etex"),
    ("strcmp_equal", r"", r"\pdfstrcmp{abc}{abc}", "etex"),
    ("strcmp_expands", r"\def\a{x}", r"\pdfstrcmp{\a}{x}", "etex"),
    ("strcmp_cs_text", r"", r"\pdfstrcmp{\relax}{\string\relax}", "etex"),
    ("strcmp_in_ifnum", r"", r"\ifnum\pdfstrcmp{a}{a}=0 Y\else N\fi", "etex"),
    ("strcmp_prefix", r"", r"\pdfstrcmp{ab}{abc}", "etex"),
    # -- \long / \outer enforcement and TeX error text ---------------------------------
    ("err_par_in_arg", r"\def\a#1{}", r"\a{x\par}", "tex-err"),
    ("err_long_par_ok", r"\long\def\a#1{}", r"\a{x\par}", "tex-err"),
    ("err_par_in_delimited", r"\def\a#1.{}", r"\a x\par.", "tex-err"),
    ("err_par_undelimited_bare", r"\def\a#1{}", r"\a\par", "tex-err"),
    ("err_outer_in_arg", r"\outer\def\o{}\def\a#1{}", r"\a{\o}", "tex-err"),
    ("err_outer_in_def", r"\outer\def\o{}", r"\def\a{\o}", "tex-err"),
    ("err_outer_in_skipped", r"\outer\def\o{}", r"\iffalse\o\fi", "tex-err"),
    ("err_outer_in_skipped_ifx", r"\outer\def\o{}", r"\ifx ab\o\fi", "tex-err"),
    ("err_outer_in_edef", r"\outer\def\o{}", r"\edef\a{\o}", "tex-err"),
    ("err_outer_in_uppercase", r"\outer\def\o{}", r"\uppercase{\o}", "tex-err"),
    ("err_outer_ok_toplevel", r"\outer\def\o{O}", r"\o", "tex-err"),
    ("err_outer_in_params", r"\outer\def\o{}", r"\def\a\o{}", "tex-err"),
    ("err_outer_in_else_skip", r"\outer\def\o{}", r"\iftrue\else\o\fi", "tex-err"),
    ("err_extra_brace_arg", r"\def\a#1{}", r"{\a}", "tex-err"),
    ("err_extra_brace_long", r"\long\def\a#1{}", r"{\a}", "tex-err"),
    ("err_doesnt_match", r"\def\a.{}", r"\a x", "tex-err"),
    ("err_prefix_letter", r"", r"\global x", "tex-err"),
    ("err_prefix_begingroup", r"", r"\global\begingroup\endgroup", "tex-err"),
    ("err_long_with_count", r"", r"\long\count0=1 ", "tex-err"),
    ("err_long_with_let", r"", r"\outer\let\a\relax", "tex-err"),
    ("err_extra_fi", r"", r"\fi", "tex-err"),
    ("err_extra_else", r"", r"\else", "tex-err"),
    ("err_extra_or", r"", r"\iftrue\or\fi", "tex-err"),
    ("err_too_many_braces", r"", r"}", "tex-err"),
    ("err_missing_number", r"", r"\count0=x ", "tex-err"),
    ("err_extra_endcsname", r"", r"\endcsname", "tex-err"),
    ("err_unless_ifcase", r"", r"\unless\ifcase0\fi", "etex-err"),
    ("err_newcommand_star_par", r"\newcommand*\zz[1]{}", r"\zz{x\par}", "latex-err"),
    ("err_newcommand_long_ok", r"\newcommand\zz[1]{}", r"\zz{x\par}", "latex-err"),
    ("err_newcommand_opt_par", r"\newcommand*\zz[2][d]{}", r"\zz[x]{y\par}", "latex-err"),
    ("err_newcommand_defined", r"\newcommand\zzq{}", r"\newcommand\zzq{}", "latex-err"),
    ("err_renewcommand_undefined", r"", r"\renewcommand\zzq{}", "latex-err"),
    ("err_no_counter", r"", r"\setcounter{zzq}{1}", "latex-err"),
    ("err_keyval_undefined", r"\makeatletter\define@key{fam}{a}{}", r"\setkeys{fam}{b=1}", "latex-err"),
    ("err_newenvironment_defined", r"\newenvironment{zze}{}{}", r"\newenvironment{zze}{}{}", "latex-err"),
    ("err_renewenvironment_undefined", r"", r"\renewenvironment{zze}{}{}", "latex-err"),
    ("err_newcounter_defined", r"\newcounter{zc}", r"\newcounter{zc}", "latex-err"),
    ("err_alph_too_large", r"\newcounter{zc}\setcounter{zc}{27}", r"\alph{zc}", "latex-err"),
    # `\newcounter` defines `\the<ctr>` (ltcounts.dtx), so a document's
    # `\newcommand{\theequation}` collides while `\renewcommand` is the
    # ordinary way to renumber (parity 2026-09-23 cause 4, 39 arXiv docs).
    ("err_newcommand_theequation", r"", r"\newcommand{\theequation}{A\arabic{equation}}", "latex-err"),
    ("err_newcommand_thetheorem", r"\newtheorem{theorem}{Theorem}", r"\newcommand{\thetheorem}{A\arabic{theorem}}", "latex-err"),
    ("err_renewcommand_theequation_ok", r"", r"\renewcommand{\theequation}{A\arabic{equation}}", "latex-err"),
    # -- counters: [within], \@addtoreset, \counterwithin/\counterwithout ------------
    ("counter_within_reset", r"\newcounter{zp}\newcounter{zc}[zp]\setcounter{zc}{5}\stepcounter{zp}", r"\arabic{zc}", "latex-write"),
    ("counter_within_setcounter_no_reset", r"\newcounter{zp}\newcounter{zc}[zp]\setcounter{zc}{5}\setcounter{zp}{3}", r"\arabic{zc}", "latex-write"),
    ("counter_addtoreset", r"\makeatletter\newcounter{zp}\newcounter{zc}\@addtoreset{zc}{zp}\setcounter{zc}{2}\stepcounter{zp}", r"\arabic{zc}", "latex-write"),
    ("counter_removefromreset", r"\makeatletter\newcounter{zp}\newcounter{zc}[zp]\@removefromreset{zc}{zp}\setcounter{zc}{2}\stepcounter{zp}", r"\arabic{zc}", "latex-write"),
    ("counterwithin_the", r"\newcounter{zp}\newcounter{zc}\counterwithin{zc}{zp}\stepcounter{zp}\stepcounter{zc}", r"\thezc", "latex-write"),
    ("counterwithin_resets", r"\newcounter{zp}\newcounter{zc}\counterwithin{zc}{zp}\setcounter{zc}{4}\stepcounter{zp}", r"\arabic{zc}", "latex-write"),
    ("counterwithin_star", r"\newcounter{zp}\newcounter{zc}\counterwithin*{zc}{zp}\stepcounter{zp}\stepcounter{zc}", r"\thezc", "latex-write"),
    ("counterwithout_no_reset", r"\newcounter{zp}\newcounter{zc}[zp]\counterwithout{zc}{zp}\setcounter{zc}{4}\stepcounter{zp}", r"\arabic{zc}", "latex-write"),
    ("counterwithout_the", r"\newcounter{zp}\newcounter{zc}\counterwithin{zc}{zp}\counterwithout{zc}{zp}\stepcounter{zc}", r"\thezc", "latex-write"),
    ("counter_reset_recursive", r"\newcounter{za}\newcounter{zb}[za]\newcounter{zd}[zb]\setcounter{zb}{4}\setcounter{zd}{7}\stepcounter{za}", r"\arabic{zb}\arabic{zd}", "latex-write"),
    ("counter_refstep_resets", r"\newcounter{zp}\newcounter{zc}[zp]\setcounter{zc}{3}\refstepcounter{zp}", r"\arabic{zc}", "latex-write"),
    ("counter_addto_negative", r"\newcounter{zp}\addtocounter{zp}{-3}", r"\arabic{zp}", "latex-write"),
    ("counter_step_in_group", r"\newcounter{zp}{\stepcounter{zp}}", r"\arabic{zp}", "latex-write"),
    ("counter_roman_large", r"\newcounter{zp}\setcounter{zp}{1994}", r"\roman{zp}\Roman{zp}", "latex-write"),
    ("counter_value_number", r"\newcounter{zp}\setcounter{zp}{12}", r"\number\value{zp}", "latex-write"),
    ("counter_the_default", r"\newcounter{zp}\setcounter{zp}{8}", r"\thezp", "latex-write"),
    ("counter_within_the_default", r"\newcounter{zp}\newcounter{zc}[zp]\setcounter{zc}{8}", r"\thezc", "latex-write"),
    ("counter_setcounter_expr", r"\newcounter{zp}\newcounter{zq}\setcounter{zq}{3}\setcounter{zp}{\value{zq}}\addtocounter{zp}{\value{zq}}", r"\arabic{zp}", "latex-write"),
    # -- \fnsymbol (text mode) ---------------------------------------------------------
    ("fnsymbol_1", r"\newcounter{zf}\setcounter{zf}{1}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_2", r"\newcounter{zf}\setcounter{zf}{2}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_3", r"\newcounter{zf}\setcounter{zf}{3}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_4", r"\newcounter{zf}\setcounter{zf}{4}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_5", r"\newcounter{zf}\setcounter{zf}{5}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_6", r"\newcounter{zf}\setcounter{zf}{6}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_7", r"\newcounter{zf}\setcounter{zf}{7}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_8", r"\newcounter{zf}\setcounter{zf}{8}", r"\fnsymbol{zf}", "latex-render"),
    ("fnsymbol_9", r"\newcounter{zf}\setcounter{zf}{9}", r"\fnsymbol{zf}", "latex-render"),
    # -- \newcommand / \newenvironment internals ----------------------------------------
    ("newcommand_meaning", r"\newcommand\zz[1]{(#1)}", r"\meaning\zz", "latex-write"),
    ("newcommand_star_meaning", r"\newcommand*\zz[1]{(#1)}", r"\meaning\zz", "latex-write"),
    ("newcommand_opt_meaning", r"\newcommand\zz[2][d]{(#1#2)}", r"\meaning\zz", "latex-write"),
    ("newcommand_opt_inner_meaning", r"\newcommand\zz[2][d]{(#1#2)}", r"\expandafter\meaning\csname\string\zz\endcsname", "latex-write"),
    ("newcommand_opt_star_inner", r"\newcommand*\zz[1][d]{(#1)}", r"\expandafter\meaning\csname\string\zz\endcsname", "latex-write"),
    ("newcommand_opt_empty_given", r"\newcommand\zz[2][d]{(#1#2)}", r"\zz[]{b}", "latex-render"),
    ("newcommand_opt_braced_bracket", r"\newcommand\zz[2][d]{(#1#2)}", r"\zz[{]}]{b}", "latex-render"),
    ("newcommand_opt_space_before", r"\newcommand\zz[2][d]{(#1#2)}", r"\zz [x]{b}", "latex-render"),
    ("newcommand_zero_meaning", r"\newcommand\zz{Z}", r"\meaning\zz", "latex-write"),
    ("newcommand_defined_keeps_old", r"\newcommand\zzq{old}\newcommand\zzq{new}", r"\zzq", "latex-render"),
    ("renewcommand_meaning", r"\newcommand\zz{a}\renewcommand*\zz[1]{b#1}", r"\meaning\zz", "latex-write"),
    ("providecommand_new", r"\providecommand\zz{new}", r"\zz", "latex-render"),
    ("newenvironment_args", r"\newenvironment{zze}[1]{(#1}{)}", r"\begin{zze}{x}y\end{zze}", "latex-render"),
    ("newenvironment_opt", r"\newenvironment{zze}[1][d]{(#1}{)}", r"\begin{zze}y\end{zze}\begin{zze}[z]w\end{zze}", "latex-render"),
    ("newenvironment_group_local", r"\newenvironment{zze}{\def\zq{in}}{}\def\zq{out}", r"\begin{zze}\zq\end{zze}\zq", "latex-render"),
    ("newenvironment_currenvir", r"\makeatletter\newenvironment{zze}{[\@currenvir]}{}\makeatother", r"\begin{zze}\end{zze}", "latex-render"),
    ("newenvironment_begin_meaning", r"\newenvironment{zze}[1]{(#1}{)}", r"\expandafter\meaning\csname zze\endcsname", "latex-write"),
    ("newenvironment_end_meaning", r"\newenvironment{zze}[1]{(#1}{)}", r"\expandafter\meaning\csname endzze\endcsname", "latex-write"),
    ("renewenvironment_render", r"\newenvironment{zze}{A}{B}\renewenvironment{zze}{C}{D}", r"\begin{zze}x\end{zze}", "latex-render"),
    ("newenvironment_nested", r"\newenvironment{zze}{(}{)}", r"\begin{zze}a\begin{zze}b\end{zze}c\end{zze}", "latex-render"),
    # -- core TeX corner cases -------------------------------------------------------------
    ("number_leading_zeros", r"", r"\number 007", "tex"),
    ("number_minus_zero", r"", r"\number -0", "tex"),
    ("romannumeral_negative", r"", r"[\romannumeral -5]", "tex"),
    ("romannumeral_4000", r"", r"\romannumeral 4000", "tex"),
    ("number_backtick_cs", r"", r"\number`\a", "tex"),
    ("number_hex_octal", r"", "\\number'777,\\number\"1F", "tex"),
    ("if_relax_relax", r"", r"\if\relax\relax Y\else N\fi", "tex"),
    ("ifx_let_char", r"\let\x=a", r"\ifx\x a Y\else N\fi", "tex"),
    ("ifx_long_vs_short", r"\def\a{x}\long\def\b{x}", r"\ifx\a\b Y\else N\fi", "tex"),
    ("ifx_same_params", r"\def\a#1{x}\def\b#1{x}", r"\ifx\a\b Y\else N\fi", "tex"),
    ("ifx_undefined_vs_relax", r"", r"\ifx\zzq\relax Y\else N\fi", "tex"),
    ("ifcat_active", r"\catcode`\~=13 \catcode`\Z=13 ", r"\ifcat\noexpand~\noexpand Z Y\else N\fi", "tex"),
    ("if_noexpand_cs", r"\def\a{x}", r"\if\noexpand\a\relax Y\else N\fi", "tex"),
    ("edef_the_toks", r"\toks0={\a}\edef\x{\the\toks0}", r"\meaning\x", "tex"),
    ("aftergroup_order", r"\def\x{}{\aftergroup\def\aftergroup\x\aftergroup{\aftergroup A\aftergroup}}", r"\x", "tex"),
    ("numexpr_unary_minus", r"", r"\the\numexpr -3*2\relax", "etex"),
    ("numexpr_round_half", r"", r"\the\numexpr 7/2\relax,\the\numexpr -7/2\relax", "etex"),
    ("dimexpr_scale", r"", r"\the\dimexpr 1pt*3/2\relax", "etex"),
    ("numexpr_in_ifnum", r"", r"\ifnum\numexpr 1+1=2 Y\else N\fi", "etex"),
    ("numexpr_count_update", r"\count0=4 \count0=\numexpr\count0+5\relax ", r"\the\count0", "etex"),
    ("dimexpr_parens", r"", r"\the\dimexpr(1pt+2pt)*2\relax", "etex"),
    ("numexpr_nested_mult", r"", r"\the\numexpr 2*(3+4)\relax", "etex"),
    ("ifdim_dimexpr", r"", r"\ifdim\dimexpr 1pt\relax<2pt Y\else N\fi", "etex"),
    ("the_catcode", r"", r"\the\catcode`\{,\the\catcode`\a,\the\catcode`\%", "tex"),
    ("the_endlinechar", r"", r"\the\endlinechar", "tex"),
    ("ifincsname", r"\def\a{A}\def\b{B}", r"\csname\ifincsname a\else b\fi\endcsname\ifincsname a\else b\fi", "etex"),
    ("unless_ifx", r"", r"\unless\ifx aa Y\else N\fi", "etex"),
    ("unless_ifdim", r"", r"\unless\ifdim 1pt>2pt Y\else N\fi", "etex"),
    ("ifcase_else", r"", r"\ifcase 3 a\or b\else c\fi", "tex"),
    ("let_undefined", r"\let\x\zzq", r"\meaning\x", "tex"),
    ("def_nested_params", r"\def\a#1{\def\b##1{#1##1}}\a x", r"\b y", "tex"),
    ("expandafter_else", r"\def\a{A}", r"\iftrue\expandafter\a\else B\fi", "tex"),
    ("string_backslash", r"", r"\string\\", "tex"),
    ("def_swap", r"\def\a#1#2{#2#1}", r"\a xy", "tex"),
    ("delimited_brace_strip", r"\def\a#1.{[#1]}", r"\a{x}.", "tex"),
    # Via \meaning: \write would print the argument's braces literally while
    # main control treats them as grouping (CONTRACT.md "Known deviations").
    ("delimited_two_groups", r"\def\a#1.{\def\r{#1}}\a{x}{y}.", r"\meaning\r", "tex"),
    ("delimited_space", r"\def\a#1#2.{[#1|#2]}", r"\a x yz.", "tex"),
    ("toks_concat", r"\toks0={a}\toks1=\expandafter{\the\toks0 b}", r"\the\toks1", "tex"),
    ("multiply_dimen", r"\dimen0=1.5pt \multiply\dimen0 by 3 ", r"\the\dimen0", "tex"),
    ("advance_skip_fil", r"\skip0=1pt plus 1fil \advance\skip0 by 1pt plus 1fil ", r"\the\skip0", "tex"),
    ("divide_negative", r"\count0=7 \divide\count0 by -2 ", r"\the\count0", "tex"),
    ("dimen_factor_register", r"\dimen1=2pt \dimen0=1.5\dimen1 ", r"\the\dimen0", "tex"),
    ("dimen_neg_decimal", r"\dimen0=-.5pt ", r"\the\dimen0", "tex"),
    ("count_from_dimen", r"\dimen1=2pt \count0=\dimen1 ", r"\the\count0", "tex"),
    ("dimen_factor_odd", r"\dimen1=3.3pt \dimen0=0.333\dimen1 ", r"\the\dimen0", "tex"),
    ("dimen_bp_mm", r"\dimen0=10bp \dimen1=3mm ", r"\the\dimen0,\the\dimen1", "tex"),
    ("dimen_dd_cc_pc", r"\dimen0=1dd \dimen1=1cc \dimen2=1pc ", r"\the\dimen0,\the\dimen1,\the\dimen2", "tex"),
    ("dimen_true", r"\dimen0=1truein ", r"\the\dimen0", "tex"),
    ("skip_minus_only", r"\skip0=3pt minus 1fill ", r"\the\skip0", "tex"),
    ("global_let", r"{\global\let\x=a}", r"\meaning\x", "tex"),
    ("edef_expands_conditional", r"\edef\x{\ifnum1<2 lt\else ge\fi}", r"\meaning\x", "tex"),
    ("gdef_then_local", r"\def\a{0}{\def\a{1}\gdef\a{2}}", r"\a", "tex"),
    ("local_after_global", r"\def\a{0}{\gdef\a{2}\def\a{1}}", r"\a", "tex"),
    ("count_global_then_local", r"\count0=0 {\global\count0=2 \count0=1 }", r"\the\count0", "tex"),
    ("catcode_group_local", r"{\catcode`\Z=13 }", r"\the\catcode`\Z", "tex"),
    ("noexpand_active", r"\catcode`\~=13 \def~{T}\edef\x{\noexpand~}", r"\meaning\x", "tex"),
    ("csname_in_edef", r"\def\zzq{Q}\edef\x{\csname zzq\endcsname}", r"\meaning\x", "tex"),
    ("expandafter_twice", r"\def\a{\b}\def\b{B}", r"\expandafter\expandafter\expandafter\meaning\a", "tex"),
    ("romannumeral_trick", r"\def\a{A}", r"\romannumeral-`0\a", "tex"),
    ("uppercase_expandafter_macro", r"\def\y{abc}\expandafter\uppercase\expandafter{\expandafter\def\expandafter\x\expandafter{\y}}", r"\x", "tex"),
    ("ifnum_char_const", r"", r"\ifnum`a=97 Y\else N\fi", "tex"),
    ("par_from_blank_line", "\\def\\a#1\\par{[#1]}\\a xy\n\nz", r"", "latex-render"),
]


# Long error messages must not be wrapped in the log.
TEX_ENV = dict(os.environ, max_print_line="10000", error_line="254", half_error_line="238")

LATEX_PREAMBLE = "\\documentclass{article}\n\\usepackage{keyval}\n"


def run_write_mode(tex_source: str, case_id: str, engine: str) -> str:
    src_path = os.path.join(TMP_DIR, f"{case_id}.tex")
    with open(src_path, "w") as f:
        f.write(tex_source)
    out_path = os.path.join(TMP_DIR, f"{case_id}.out")
    if os.path.exists(out_path):
        os.remove(out_path)
    cmd = [engine, "-interaction=batchmode", "-output-directory", TMP_DIR, src_path]
    result = subprocess.run(cmd, cwd=TMP_DIR, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30, env=TEX_ENV)
    out_path = os.path.join(TMP_DIR, f"{case_id}.out")
    if not os.path.exists(out_path) or os.path.getsize(out_path) == 0:
        log_path = os.path.join(TMP_DIR, f"{case_id}.log")
        log = open(log_path, errors="replace").read() if os.path.exists(log_path) else "(no log)"
        raise RuntimeError(
            f"{case_id}: no/empty output file.\n--- stdout ---\n{result.stdout.decode('utf-8', 'replace')}\n--- log tail ---\n{log[-1500:]}"
        )
    with open(out_path, "r") as f:
        return f.read().rstrip("\n")


def build_latex_write_source(setup: str, expr: str, case_id: str, setup_in_preamble: bool) -> str:
    opening = r"\newwrite\out" + "\n" + rf"\immediate\openout\out={case_id}.out" + "\n"
    write = r"\immediate\write\out{" + expr + "}\n" + r"\immediate\closeout\out" + "\n"
    if setup_in_preamble:
        return LATEX_PREAMBLE + opening + setup + "\n" + r"\begin{document}" + "\n" + write + r"\end{document}" + "\n"
    return LATEX_PREAMBLE + opening + r"\begin{document}" + "\n" + setup + "\n" + write + r"\end{document}" + "\n"


def run_err_mode(setup: str, expr: str, case_id: str, engine: str) -> str:
    """Run setup+expr and return the log's `! ...` error lines."""
    src_path = os.path.join(TMP_DIR, f"{case_id}.tex")
    if engine == "pdflatex":
        source = LATEX_PREAMBLE + r"\begin{document}" + "\n" + setup + expr + "\n" + r"\end{document}" + "\n"
    else:
        source = r"\catcode`\@=11" + "\n" + setup + expr + "\n" + r"\end" + "\n"
    with open(src_path, "w") as f:
        f.write(source)
    cmd = [engine, "-interaction=batchmode", "-output-directory", TMP_DIR, src_path]
    subprocess.run(cmd, cwd=TMP_DIR, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30, env=TEX_ENV)
    log_path = os.path.join(TMP_DIR, f"{case_id}.log")
    lines = open(log_path, errors="replace").read().splitlines()
    errs = [l[2:] for l in lines if l.startswith("! ") and "Emergency stop" not in l and "Fatal error" not in l]
    return "\n".join(errs)


def build_write_source(setup: str, expr: str, engine: str, case_id: str) -> str:
    preamble = r"\catcode`\@=11" + "\n" if engine != "latex" else ""
    return (
        preamble
        + r"\newwrite\out" + "\n"
        + rf"\immediate\openout\out={case_id}.out" + "\n"
        + setup + "\n"
        + r"\immediate\write\out{" + expr + "}\n"
        + r"\immediate\closeout\out" + "\n"
        + r"\end" + "\n"
    )


def run_render_mode(setup: str, expr: str, case_id: str) -> str:
    """For LaTeX-layer macros whose internals (\\futurelet, nested \\def) do
    not survive \\write's edef-like restricted expansion: typeset `setup`
    then `expr` as ordinary document body content (full main-control
    execution, completely faithful), then extract text from the rendered
    PDF with `pdftotext`."""
    src_path = os.path.join(TMP_DIR, f"{case_id}.tex")
    source = (
        LATEX_PREAMBLE
        + "\\pagestyle{empty}\n\\begin{document}\n"
        + r"\newcommand{\oraclemarker}{}"
        + "\n\\noindent ORACLESTART\\oraclemarker\\ "
        + setup
        + "\n"
        + expr
        + "\n\\ ORACLEEND\\oraclemarker\n\\end{document}\n"
    )
    with open(src_path, "w") as f:
        f.write(source)
    pdf_path = os.path.join(TMP_DIR, f"{case_id}.pdf")
    if os.path.exists(pdf_path):
        os.remove(pdf_path)
    cmd = ["pdflatex", "-interaction=batchmode", "-output-directory", TMP_DIR, src_path]
    subprocess.run(cmd, cwd=TMP_DIR, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30, env=TEX_ENV)
    pdf_path = os.path.join(TMP_DIR, f"{case_id}.pdf")
    if not os.path.exists(pdf_path):
        log_path = os.path.join(TMP_DIR, f"{case_id}.log")
        log = open(log_path, errors="replace").read() if os.path.exists(log_path) else "(no log)"
        raise RuntimeError(f"{case_id}: pdflatex produced no PDF.\n--- log tail ---\n{log[-1500:]}")
    text = subprocess.run(["pdftotext", "-layout", pdf_path, "-"], cwd=TMP_DIR, stdout=subprocess.PIPE, timeout=30)
    rendered = text.stdout.decode("utf-8", "replace")
    # Extract exactly the text between our ORACLESTART/ORACLEEND markers
    # (unique tokens, so they can't collide with a case's own content).
    start = rendered.find("ORACLESTART")
    end = rendered.find("ORACLEEND", start if start >= 0 else 0)
    if start < 0 or end < 0:
        raise RuntimeError(f"{case_id}: ORACLESTART/ORACLEEND markers not found in rendered text:\n{rendered!r}")
    middle = rendered[start + len("ORACLESTART") : end]
    return middle.strip()


def main():
    manifest = {}
    failures = []
    for case_id, setup, expr, mode in CASES:
        try:
            if mode == "latex-render":
                out = run_render_mode(setup, expr, case_id)
            elif mode in ("latex-write", "latex-doc"):
                source = build_latex_write_source(setup, expr, case_id, mode == "latex-doc")
                out = run_write_mode(source, case_id, "pdflatex")
            elif mode.endswith("-err"):
                engine = {"tex-err": "tex", "etex-err": "etex", "latex-err": "pdflatex"}[mode]
                out = run_err_mode(setup, expr, case_id, engine)
            else:
                engine = "etex" if mode == "etex" else "tex"
                source = build_write_source(setup, expr, engine, case_id)
                out = run_write_mode(source, case_id, engine)
        except Exception as e:
            failures.append((case_id, str(e)))
            print(f"FAIL {case_id}: {e}", file=sys.stderr)
            continue
        with open(os.path.join(EXPECTED_DIR, f"{case_id}.txt"), "w") as f:
            f.write(out)
        manifest[case_id] = {"setup": setup, "expr": expr, "expected": out, "mode": mode}
        print(f"OK   {case_id}: {out!r}")
    with open(MANIFEST_PATH, "w") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
    print(f"\n{len(manifest)} cases captured, {len(failures)} failed.")
    if failures:
        sys.exit(1)


if __name__ == "__main__":
    main()
