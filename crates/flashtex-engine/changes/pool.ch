% changes/pool.ch -- the strings of encTeX's and ML\TeX's own code.
%
% TeX Live's pdfTeX is tangled with encTeX (enctexdir/enctex2.ch) and ML\TeX
% (tex.ch), and TANGLE puts every string constant of the program into the
% string pool, whether or not the code that uses it ever runs. Their
% primitives exist only with `-enc' and `-mltex', which this engine refuses
% (changes/enctex.ch), so their code is not re-specified; but INITEX's pool
% is TeX Live's only with their 18 strings (409 characters), and the pool's
% size shows: `strings out of', `string characters out of' in the log's
% statistics and the capacities in `TeX capacity exceeded, sorry [pool
% size=...]' and `[number of strings=...]' are counted from the strings a
% format holds (#2208).
%
% Each hunk puts the strings where TeX Live's change file has them, in a
% meta-comment (`@{...@}'): web2rust, like TANGLE, enters a string into the
% pool when it reads it, and a meta-comment is not part of the program. So
% the strings get TeX Live's numbers and nothing the program computes
% changes. `tests/pool_strings.rs' compares INITEX's string count and pool
% length after \dump with TeX Live's pdfTeX.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.6833 - enctex2.ch [18.265]: \endmubyte
primitive("endcsname",end_cs_name,0);@/
@!@:end_cs_name_}{\.{\\endcsname} primitive@>
@y
primitive("endcsname",end_cs_name,0);@/
@!@:end_cs_name_}{\.{\\endcsname} primitive@>
@{"endmubyte"@}
@z

@x pdftex.web l.30524 - enctex2.ch [48.1138]: \endmubyte's help
help1("I'm ignoring this, since I wasn't doing a \csname.");
@y
@{"I'm ignoring this, since I wasn't doing a \mubyte."@}
help1("I'm ignoring this, since I wasn't doing a \csname.");
@z

@x pdftex.web l.31739 - enctex2.ch [49.1219], [49.1221]: \mubyte, \noconvert
primitive("futurelet",let,normal+1);@/
@!@:future_let_}{\.{\\futurelet} primitive@>
@y
primitive("futurelet",let,normal+1);@/
@!@:future_let_}{\.{\\futurelet} primitive@>
@{"mubyte" "noconvert"
"not appear in <byte sequence> between \mubyte and \endmubyte."
"The empty <byte sequence>, " " ignored" "The <byte sequence> in"
"\mubyte <token> <byte sequence>\endmubyte should not be empty."@}
@z

@x pdftex.web l.31795 - tex.ch [49.1222], [49.1224]: ML\TeX's \charsubdef
@!@:toks_def_}{\.{\\toksdef} primitive@>
@y
@!@:toks_def_}{\.{\\toksdef} primitive@>
@{"charsubdef" "New character substitution: " " = "@}
@z

@x pdftex.web l.31944 - enctex2.ch [49.1230]: \xordcode, \xchrcode, \xprncode
primitive("catcode",def_code,cat_code_base);
@!@:cat_code_}{\.{\\catcode} primitive@>
@y
primitive("catcode",def_code,cat_code_base);
@!@:cat_code_}{\.{\\catcode} primitive@>
@{"xordcode" "xchrcode" "xprncode"@}
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ The messages of ML\TeX's character substitution (tex.ch [54/ML\TeX]),
which only `-mltex' prints; their strings, in TeX Live's places in the
pool (\.{changes/pool.ch}).

@<Constants in the outer block@>=
@{"substitution for " "Missing character: Incomplete substitution "
"Using character substitution: "@}

@* \[55] Index.
@z
