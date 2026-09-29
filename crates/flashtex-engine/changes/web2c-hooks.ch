% changes/web2c-hooks.ch -- the text of tex.ch that TeX Live's feature
% change files are written against.
%
% TeX Live builds pdfTeX from pdftex.web, tex.ch and further change files
% (pdftexdir/am/pdftex.am): tracingstacklevels.ch, partoken-102.ch,
% partoken.ch, locnull-optimize.ch, showstream.ch, unbalanced-braces.ch and
% char-warning-pdftex.ch among them. Those are public domain and are applied
% here unmodified (third_party/pdftex/web2c/), after this file. Their `@x'
% lines match text that tex.ch introduces, so this file reproduces exactly
% that text, and nothing more:
%
%   * the block of web2c's integer parameters (tex.ch [17.236]-[17.240]),
%     which begins with ML\TeX's three (\charsubdefmin, \charsubdefmax,
%     \tracingcharsubdef). ML\TeX itself is not re-specified: |mltex_p| is
%     always false, so those three are never primitives, exactly as in
%     TeX Live without `-mltex';
%   * tex.ch's first line of |par_loc|'s undumping ([50.1314]).
%
% tex.ch's start_input text, which tracingstacklevels.ch also matches, is in
% changes/filenames.ch, where it belongs.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.5680 - tex.ch [17.236]: web2c's integer parameters come before pdfTeX's
@d tex_int_pars=55 {total number of \TeX's integer parameters}
@#
@d pdftex_first_integer_code = tex_int_pars {base for \pdfTeX's integer parameters}
@y
@d tex_int_pars=55 {total number of \TeX's integer parameters}
@#
@d web2c_int_base=tex_int_pars {base for web2c's integer parameters}
@d char_sub_def_min_code=web2c_int_base {smallest value in the charsubdef list}
@d char_sub_def_max_code=web2c_int_base+1 {largest value in the charsubdef list}
@d tracing_char_sub_def_code=web2c_int_base+2 {traces changes to a charsubdef def}
@d web2c_int_pars=web2c_int_base+3 {total number of web2c's integer parameters}
@#
@d pdftex_first_integer_code = web2c_int_pars {base for \pdfTeX's integer parameters}
@z

@x pdftex.web l.5798 - tex.ch [17.236]
@d error_context_lines==int_par(error_context_lines_code)
@y
@d error_context_lines==int_par(error_context_lines_code)
@#
@d char_sub_def_min==int_par(char_sub_def_min_code)
@d char_sub_def_max==int_par(char_sub_def_max_code)
@d tracing_char_sub_def==int_par(tracing_char_sub_def_code)
@z

@x pdftex.web l.5911 - tex.ch [17.237]
error_context_lines_code:print_esc("errorcontextlines");
@y
error_context_lines_code:print_esc("errorcontextlines");
char_sub_def_min_code:print_esc("charsubdefmin");
char_sub_def_max_code:print_esc("charsubdefmax");
tracing_char_sub_def_code:print_esc("tracingcharsubdef");
@z

@x pdftex.web l.6070 - tex.ch [17.238]; |mltex_p| is always false here
@!@:error_context_lines_}{\.{\\errorcontextlines} primitive@>
@y
@!@:error_context_lines_}{\.{\\errorcontextlines} primitive@>
if mltex_p then
  begin mltex_enabled_p:=true;  {enable character substitution}
  if false then {remove the if-clause to enable \.{\\charsubdefmin}}
  primitive("charsubdefmin",assign_int,int_base+char_sub_def_min_code);@/
@!@:char_sub_def_min_}{\.{\\charsubdefmin} primitive@>
  primitive("charsubdefmax",assign_int,int_base+char_sub_def_max_code);@/
@!@:char_sub_def_max_}{\.{\\charsubdefmax} primitive@>
  primitive("tracingcharsubdef",assign_int,int_base+tracing_char_sub_def_code);@/
@!@:tracing_char_sub_def_}{\.{\\tracingcharsubdef} primitive@>
  end;
@z

@x pdftex.web l.6168 - tex.ch [17.240]
for k:=int_base to del_code_base-1 do eqtb[k].int:=0;
@y
for k:=int_base to del_code_base-1 do eqtb[k].int:=0;
char_sub_def_min:=256; char_sub_def_max:=-1;
{allow \.{\\charsubdef} for char 0}@/
{|tracing_char_sub_def:=0| is already done}@/
@z

@x pdftex.web l.33001 - tex.ch [50.1314]: |hash_top| is |frozen_control_sequence| here
undump(hash_base)(frozen_control_sequence)(par_loc);
@y
undump(hash_base)(hash_top)(par_loc);
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ tex.ch's ML\TeX\ switches (its part \.{[54/ML\TeX]}). ML\TeX\ is not
re-specified, so both stay false.

@d hash_top==frozen_control_sequence {tex.ch's name, without |hash_extra|}

@<Glob...@>=
@!mltex_p: boolean; {was \.{-mltex} given? (never, here)}
@!mltex_enabled_p:boolean;  {enable character substitution}

@ @<Set init...@>=
mltex_p:=false;
mltex_enabled_p:=false;

@* \[55] Index.
@z
