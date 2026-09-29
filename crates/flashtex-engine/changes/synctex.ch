% changes/synctex.ch -- the \synctex parameter, without SyncTeX.
%
% TeX Live's pdfTeX includes SyncTeX (synctexdir/synctex-*.ch and
% synctex.c), which adds the integer parameter \synctex (after e-TeX's, as
% synctex-e-mem.ch1 places it) and two words of source position to several
% kinds of node. Only the parameter is re-specified here, so that the
% primitive exists and documents may set it; nothing reads it yet, so no
% .synctex file is written and nodes keep pdftex.web's sizes (which only
% shows in \tracingstats memory accounting, DESIGN.md section 1.1).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.5736 - synctex-e-mem.ch1: \synctex is the last integer parameter
@d int_pars=etex_int_pars {total number of integer parameters}
@y
@d synctex_code=etex_int_pars
@d int_pars=synctex_code+1 {total number of integer parameters}
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ synctex-mem.ch0's parameter \.{\\synctex}.

@d synctex==int_par(synctex_code)

@<Put each of \TeX's primitives into the hash table@>=
primitive("synctex",assign_int,int_base+synctex_code);@/
@!@:synctex_}{\.{\\synctex} primitive@>

@ @<Cases for |print_param|@>=
synctex_code:    print_esc("synctex");

@* \[55] Index.
@z
