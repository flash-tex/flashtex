% changes/pool.ch -- the string pool TeX Live's XeTeX has: tex.ch's strings.
%
% TANGLE puts every string constant of the program into the string pool, and
% INITEX's pool is part of what a run shows: `N strings of total length M'
% after \dump, the `strings out of' and `string characters out of' lines of
% the statistics, and the capacities in `TeX capacity exceeded, sorry [pool
% size=...]' and `[number of strings=...]', which are counted from the
% strings a format holds (and decide when a run overflows). TeX Live's
% xetex.pool (xetexdir/am/xetex.am: tie -c over xetex.web, tex.ch0, tex.ch,
% ..., xetex.ch, char-warning-xetex.ch, tex-binpool.ch; then tangle) has 15
% strings and 428 characters more than xetex.web with this crate's other
% change files (#2527), because those re-specify tex.ch's behaviour and keep
% xetex.web's strings where tex.ch has its own:
%
%   - tex.ch's i18n fixes, which print whole messages instead of pieces:
%     [16.211] |print_mode| and the new |print_in_mode| (used by [46.1049]),
%     [16.219] ` lines', [22.306] `Runaway ...', [24.338]/[24.339] ` while
%     scanning ...', and the statistics of [50.1320] and [50.1324]. Each hunk
%     is tex.ch's; what they print is what the pieces printed;
%   - [29.514]: tex.ch drops |TEX_area| and |TEX_font_area| (`TeXinputs:',
%     `TeXfonts:'), which no code of this crate uses (filenames.ch, web2c.ch);
%   - ML\TeX's strings ([49.1222], [49.1224] and part 54's three substitution
%     messages), which TANGLE pools though only `-mltex' reaches the code;
%     ML\TeX is not re-specified (web2c-hooks.ch, tex-mltex.ch), so these
%     stand in meta-comments (`@{...@}') where tex.ch has them: web2rust,
%     like TANGLE, enters a string into the pool when it reads it, and a
%     meta-comment is not part of the program (as the pdfTeX engine's
%     changes/pool.ch, #2208).
%
% With this file the regenerated xetex.pool is TeX Live's tangled xetex.pool
% line for line. Applied after web2c-hooks.ch and before synctex.ch, whose
% `synctex' is TeX Live's last string. tests/pool_strings.rs compares the
% pool counts with TeX Live's XeTeX.
%
% The hunks are from TeX Live's texk/web2c/tex.ch (texlive-2026.1).

@x xetex.web l.5120 - tex.ch [16.211]: i18n fix, whole mode names; |print_in_mode|
begin if m>0 then
  case m div (max_command+1) of
  0:print("vertical");
  1:print("horizontal");
  2:print("display math");
  end
else if m=0 then print("no")
else  case (-m) div (max_command+1) of
  0:print("internal vertical");
  1:print("restricted horizontal");
  2:print("math");
  end;
print(" mode");
end;
@y
begin if m>0 then
  case m div (max_command+1) of
  0:print("vertical mode");
  1:print("horizontal mode");
  2:print("display math mode");
  end
else if m=0 then print("no mode")
else  case (-m) div (max_command+1) of
  0:print("internal vertical mode");
  1:print("restricted horizontal mode");
  2:print("math mode");
  end;
end;

procedure print_in_mode(@!m:integer); {prints the mode represented by |m|}
begin if m>0 then
  case m div (max_command+1) of
  0:print("' in vertical mode");
  1:print("' in horizontal mode");
  2:print("' in display math mode");
  end
else if m=0 then print("' in no mode")
else  case (-m) div (max_command+1) of
  0:print("' in internal vertical mode");
  1:print("' in restricted horizontal mode");
  2:print("' in math mode");
  end;
end;
@z

@x xetex.web l.5305 - tex.ch [16.219]: i18n fix
    print_int(nest[p].pg_field); print(" line");
    if nest[p].pg_field<>1 then print_char("s");
@y
    print_int(nest[p].pg_field);
    if nest[p].pg_field<>1 then print(" lines")
    else print(" line");
@z

@x xetex.web l.7800 - tex.ch [22.306]: i18n fix
  begin print_nl("Runaway ");
@.Runaway...@>
  case scanner_status of
  defining: begin print("definition"); p:=def_ref;
    end;
  matching: begin print("argument"); p:=temp_head;
    end;
  aligning: begin print("preamble"); p:=hold_head;
    end;
  absorbing: begin print("text"); p:=def_ref;
    end;
  end; {there are no other cases}
@y
  begin
@.Runaway...@>
  case scanner_status of
  defining: begin print_nl("Runaway definition"); p:=def_ref;
    end;
  matching: begin print_nl("Runaway argument"); p:=temp_head;
    end;
  aligning: begin print_nl("Runaway preamble"); p:=hold_head;
    end;
  absorbing: begin print_nl("Runaway text"); p:=def_ref;
    end;
  end; {there are no other cases}
@z

@x xetex.web l.8405 - tex.ch [24.338]: i18n fix
print(" while scanning ");
@y
@z

@x xetex.web l.8427 - tex.ch [24.339]: i18n fix
defining:begin print("definition"); info(p):=right_brace_token+"}";
  end;
matching:begin print("use"); info(p):=par_token; long_state:=outer_call;
  end;
aligning:begin print("preamble"); info(p):=right_brace_token+"}"; q:=p;
  p:=get_avail; link(p):=q; info(p):=cs_token_flag+frozen_cr;
  align_state:=-1000000;
  end;
absorbing:begin print("text"); info(p):=right_brace_token+"}";
@y
defining:begin print(" while scanning definition"); info(p):=right_brace_token+"}";
  end;
matching:begin print(" while scanning use"); info(p):=par_token; long_state:=outer_call;
  end;
aligning:begin print(" while scanning preamble"); info(p):=right_brace_token+"}"; q:=p;
  p:=get_avail; link(p):=q; info(p):=cs_token_flag+frozen_cr;
  align_state:=-1000000;
  end;
absorbing:begin print(" while scanning text"); info(p):=right_brace_token+"}";
@z

@x xetex.web l.12159 - tex.ch [29.514]: TeX area directories
@d TEX_area=="TeXinputs:"
@.TeXinputs@>
@d TEX_font_area=="TeXfonts:"
@.TeXfonts@>
@y
In C, the default paths are specified separately.
@z

@x xetex.web l.24991 - tex.ch [46.1049]: i18n fix, see change to [16.211]
print("' in "); print_mode(mode);
@y
print_in_mode(mode);
@z

@x xetex.web l.27694 - tex.ch [49.1222], [49.1224]: ML\TeX's \charsubdef
@!@:toks_def_}{\.{\\toksdef} primitive@>
@y
@!@:toks_def_}{\.{\\toksdef} primitive@>
@{"charsubdef" "New character substitution: " " = "@}
@z

@x xetex.web l.29083 - tex.ch [50.1320]: i18n fix
print_int(font_ptr-font_base); print(" preloaded font");
if font_ptr<>font_base+1 then print_char("s")
@y
print_int(font_ptr-font_base);
if font_ptr<>font_base+1 then print(" preloaded fonts")
else print(" preloaded font")
@z

@x xetex.web l.29155 - tex.ch [50.1324]: i18n fix
print_ln; print_int(hyph_count); print(" hyphenation exception");
if hyph_count<>1 then print_char("s");
@y
print_ln; print_int(hyph_count);
if hyph_count<>1 then print(" hyphenation exceptions")
else print(" hyphenation exception");
@z

@x xetex.web l.29170 - tex.ch [50.1324]: i18n fix
print(" has "); print_int(trie_op_ptr); print(" op");
if trie_op_ptr<>1 then print_char("s");
@y
print(" has "); print_int(trie_op_ptr);
if trie_op_ptr<>1 then print(" ops")
else print(" op");
@z

@x xetex.web l.34397 - tex.ch [54/web2c]: ML\TeX's substitution messages
  cur_val:=penalty(equiv(m)+cur_val);
  end;
end

@* \[54] System-dependent changes.
@y
  cur_val:=penalty(equiv(m)+cur_val);
  end;
end
@{"substitution for " "Missing character: Incomplete substitution "
"Using character substitution: "@}

@* \[54] System-dependent changes.
@z
