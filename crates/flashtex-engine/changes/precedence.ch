% changes/precedence.ch -- the operator precedence pdfTeX is compiled with.
%
% web2c prints Pascal's `and`, `or` and `not` as C's `&&`, `||` and `!`
% without adding parentheses (web2c/web2c-parser.y), and in C `&&` and `||`
% bind more loosely than comparisons, whereas in Pascal `and` binds like `*`
% and `or` like `+`. tex.web always parenthesises, so the two readings never
% differ there; in the four places below pdftex.web does not, and the program
% TeX Live ships is the C reading. tools/web2rust refuses such expressions
% (parse.rs, `precedence_clash`), and these changes add exactly the
% parentheses the C compiler implies, nothing else.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.16255 - C: (!g && pdf_f != f)
    if (not gen_faked_interword_space and pdf_f <> f)
@y
    if ((not gen_faked_interword_space) and (pdf_f <> f))
@z

@x pdftex.web l.16293 - C: ... || (ratio(f) != ratio(pdf_f))
          or get_font_auto_expand_ratio(f) <> get_font_auto_expand_ratio(pdf_f);
@y
          or (get_font_auto_expand_ratio(f) <> get_font_auto_expand_ratio(pdf_f));
@z

@x pdftex.web l.17337 - C: !(!is_char_node(g) && type(g) == glue_node)
    if not (not is_char_node(g) and type(g) = glue_node) then begin
@y
    if not ((not is_char_node(g)) and (type(g) = glue_node)) then begin
@z

@x pdftex.web l.19074 - C: set && (fixed_pdf_draftmode > 0)
    if fixed_pdf_draftmode_set and fixed_pdf_draftmode > 0 then begin
@y
    if fixed_pdf_draftmode_set and (fixed_pdf_draftmode > 0) then begin
@z
