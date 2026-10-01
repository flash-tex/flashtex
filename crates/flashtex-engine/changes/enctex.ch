% changes/enctex.ch -- the eqtb entries of TeX Live's encTeX.
%
% TeX Live builds pdfTeX with encTeX (enctexdir/enctex1.ch,
% enctex-pdftex.ch, enctex2.ch, applied after showstream.ch, as
% pdftexdir/am/pdftex.am lists them). encTeX itself is not re-specified
% (`-enc` is refused), and without `-enc` pdfTeX defines none of its
% primitives, but its eqtb entries are there in every run: three
% locations in region 4 (the chr codes of \xordcode, \xchrcode and
% \xprncode) and four integer parameters (\mubytein, \mubyteout,
% \mubytelog, \specialout). They move every later eqtb location, so the
% layout is TeX Live's only with them (eqtb_size 30192, as TeX Live's
% pdftex.fmt records it).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.5426 - enctex1.ch [17.230]: xord_code_base, xchr_code_base, xprn_code_base
@d math_font_base=cur_font_loc+1 {table of 48 math font numbers}
@y
@d xord_code_base=cur_font_loc+1
@d xchr_code_base=xord_code_base+1
@d xprn_code_base=xchr_code_base+1
@d math_font_base=xprn_code_base+1 {table of 48 math font numbers}
@z

@x showstream.ch's line - enctex-pdftex.ch [17.236]: \mubytein ... \specialout
@d web2c_int_pars=web2c_int_base+6 {total number of web2c's integer parameters}
@y
@d mubyte_in_code=web2c_int_base+6{if positive then reading mubytes is active}
@d mubyte_out_code=web2c_int_base+7{if positive then printing mubytes is active}
@d mubyte_log_code=web2c_int_base+8{if positive then print mubytes to log and terminal}
@d spec_out_code=web2c_int_base+9 {if positive then print specials by mubytes}
@d web2c_int_pars=web2c_int_base+10 {total number of web2c's integer parameters}
@z
