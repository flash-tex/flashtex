% changes/mltex.ch -- tex.ch's ML\TeX character access, through which
% xetexdir/xetex.ch applies a TFM font's TECkit mapping.
%
% TeX Live's tex.ch makes |char_info(f)(c)| read the information of the
% "effective character" |effective_char(true,f,c)| (ML\TeX, tex.ch
% [30.554]) and adds |orig_char_info|, the plain access, for the places that
% must not substitute: loading a font (tex.ch [30.570], [30.573], [30.576]),
% |new_character| ([30.582]), the characters |hlist_out| sends to the DVI
% file ([32.620]) and math mode ([35.708], [36.722], [36.740], [36.749]);
% the main loop checks and reads the effective character ([46.1036]).
% xetex.ch then makes |effective_char| and |effective_char_info| apply the
% font's TECkit mapping (a TFM font loaded as `name:mapping=...') to the
% character first, unless |xtx_ligature_present| (the character of a
% ligature, already mapped), which every call resets. So in XeTeX a TFM
% font with a mapping is measured and checked by its mapped characters,
% and its nodes keep the unmapped ones, which |hlist_out| maps again
% (xetex.web).
%
% ML\TeX's substitutions themselves are not re-specified: |mltex_enabled_p|
% is always false here (web2c-hooks.ch), so |effective_char| returns the
% (mapped) character and |effective_char_info| its |orig_char_info|, and
% |hlist_out| outputs a character only if it exists, as tex.ch's code does
% without ML\TeX. Applied after xetex-web2c.ch, as xetex.ch follows tex.ch.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.13016 - tex.ch [30.554]: |char_info| reads the effective character
@d char_info_end(#)==#].qqqq
@d char_info(#)==font_info[char_base[#]+char_info_end
@y
@d char_info_end(#)==#)].qqqq
@d char_info(#)==font_info[char_base[#]+effective_char(true,#,char_info_end
@#
@d orig_char_info_end(#)==#].qqqq
@d orig_char_info(#)==font_info[char_base[#]+orig_char_info_end
@z

@x xetex.web l.13093 - tex.ch [30.560]: ML\TeX's functions come before |read_font_info|
@p function read_font_info(@!u:pointer;@!nom,@!aire:str_number;
@y
@p @t\4@>@<Declare additional functions for ML\TeX@>@/

function read_font_info(@!u:pointer;@!nom,@!aire:str_number;
@z

@x xetex.web l.13332 - tex.ch [30.570]: no substitution while loading a font
  begin qw:=char_info(f)(d);
@y
  begin qw:=orig_char_info(f)(d);
@z

@x xetex.web l.13384 - tex.ch [30.573]: no substitution while loading a font
  qw:=char_info(f)(#); {N.B.: not |qi(#)|}
@y
  qw:=orig_char_info(f)(#); {N.B.: not |qi(#)|}
@z

@x xetex.web l.13448 - tex.ch [30.576]: no substitution while loading a font
  begin qw:=char_info(f)(bchar); {N.B.: not |qi(bchar)|}
@y
  begin qw:=orig_char_info(f)(bchar); {N.B.: not |qi(bchar)|}
@z

@x xetex.web l.13558 - tex.ch [30.582], xetex.ch: |new_character| checks the effective character
var p:pointer; {newly allocated node}
begin
if is_native_font(f) then
  begin new_character:=new_native_character(f,c); return;
  end;
if font_bc[f]<=c then if font_ec[f]>=c then
  if char_exists(char_info(f)(qi(c))) then
@y
var p:pointer; {newly allocated node}
@!ec:quarterword;  {effective character of |c|}
begin
if is_native_font(f) then
  begin new_character:=new_native_character(f,c); return;
  end;
ec:=effective_char(false,f,qi(c));
if font_bc[f]<=qo(ec) then if font_ec[f]>=qo(ec) then
  if char_exists(orig_char_info(f)(ec)) then  {N.B.: not |char_info|}
@z

@x xetex.web l.14850 - tex.ch [32.620]: |hlist_out| outputs a character that exists
  if c>=qi(128) then dvi_out(set1);
  dvi_out(qo(c));@/
  cur_h:=cur_h+char_width(f)(char_info(f)(c));
@y
  if font_ec[f]>=qo(c) then if font_bc[f]<=qo(c) then
    if char_exists(orig_char_info(f)(c)) then  {N.B.: not |char_info|}
      begin if c>=qi(128) then dvi_out(set1);
      dvi_out(qo(c));@/
      cur_h:=cur_h+char_width(f)(orig_char_info(f)(c));
      end;
@z

@x xetex.web l.17226 - tex.ch [35.708]: no substitution in |var_delimiter|
  begin continue: q:=char_info(g)(y);
@y
  begin continue: q:=orig_char_info(g)(y);
@z

@x xetex.web l.17509 - tex.ch [36.722]: no substitution in |fetch|
    cur_i:=char_info(cur_f)(cur_c)
@y
    cur_i:=orig_char_info(cur_f)(cur_c)
@z

@x xetex.web l.17933 - tex.ch [36.740]: no substitution in |make_math_accent|
  i:=char_info(f)(y);
@y
  i:=orig_char_info(f)(y);
@z

@x xetex.web l.18108 - tex.ch [36.749]: no substitution in |make_op|
      begin c:=rem_byte(cur_i); i:=char_info(cur_f)(c);
@y
      begin c:=rem_byte(cur_i); i:=orig_char_info(cur_f)(c);
@z

@x xetex.web l.24729 - tex.ch [46.1036]: the main loop checks the effective character
main_loop_move+2:if(cur_chr<font_bc[main_f])or(cur_chr>font_ec[main_f]) then
@y
main_loop_move+2:
if(qo(effective_char(false,main_f,qi(cur_chr)))>font_ec[main_f])or
  (qo(effective_char(false,main_f,qi(cur_chr)))<font_bc[main_f]) then
@z

@x xetex.web l.24732 - tex.ch [46.1036]
main_i:=char_info(main_f)(cur_l);
if not char_exists(main_i) then
@y
main_i:=effective_char_info(main_f,cur_l);
if not char_exists(main_i) then
@z

@x xetex.web l.34414 - tex.ch [54/web2c], xetex.ch [54/web2c]: |effective_char|
@* \[55] Index.
@y
@ The effective character of |c| in font |f| (tex.ch's ML\TeX, with
xetex.ch's font mappings): |c| through the font's \.{TECkit} mapping, if it
has one and |c| is not the character of a ligature, which is mapped
already. ML\TeX's substitution of a missing character is not
re-specified, because |mltex_enabled_p| is always false here; so |err_p|
is not used.

@<Declare additional functions for ML\TeX@>=
function effective_char(@!err_p:boolean;
                        @!f:internal_font_number;@!c:quarterword):integer;
begin if (not xtx_ligature_present) and (font_mapping[f]<>nil) then
  c:=apply_tfm_font_mapping(font_mapping[f],c);
xtx_ligature_present:=false;
effective_char:=c;
end;

@ |effective_char_info| is |char_info| of the effective character, without
a second mapping.

@<Declare additional functions for ML\TeX@>=
function effective_char_info(@!f:internal_font_number;
                             @!c:quarterword):four_quarters;
begin if (not xtx_ligature_present) and (font_mapping[f]<>nil) then
  c:=apply_tfm_font_mapping(font_mapping[f],c);
xtx_ligature_present:=false;
effective_char_info:=orig_char_info(f)(c);
end;

@* \[55] Index.
@z
