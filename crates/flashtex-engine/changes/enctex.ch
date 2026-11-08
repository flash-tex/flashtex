% changes/enctex.ch -- what TeX Live's encTeX does in every run.
%
% TeX Live builds pdfTeX with encTeX (enctexdir/enctex1.ch,
% enctex-pdftex.ch, enctex2.ch, applied after showstream.ch, as
% pdftexdir/am/pdftex.am lists them). encTeX itself is not re-specified
% (`-enc` is refused), and without `-enc` pdfTeX defines none of its
% primitives. Three parts of it are there in every run all the same, and
% this file gives them, each from the hunk of enctex*.ch it names:
%
%   * its eqtb entries: three locations in region 4 (the chr codes of
%     \xordcode, \xchrcode and \xprncode) and four integer parameters
%     (\mubytein, \mubyteout, \mubytelog, \specialout). They move every
%     later eqtb location, so the layout is TeX Live's only with them
%     (eqtb_size 30192, as TeX Live's pdftex.fmt records it). Their names
%     are print_param's, as in enctex2.ch [17.237];
%   * how |print| prints a character: |xprn| decides when it is printed,
%     not when the string pool is made (enctex1.ch [5.59]). The first 256
%     strings are pdftex.web's (`^^A', `^^e9'), whatever the TCX file
%     says, as in TeX Live's formats, and a character |xprn| calls
%     printable is printed as itself: `-8bit' prints `^^A' as the byte 1;
%   * \message and \errmessage make their string with |message_printing|
%     set and print it with |print|, not |slow_print| (enctex2.ch
%     [49.1279]): a character |xprn| calls unprintable is in the string as
%     its `^^' form, which the line-length test counts.
%
% Without `-enc' the \mubyte tables are empty (|mubyte_write| is all
% zero), |mubyte_log| is 0 and nothing sets |special_printing|, so the
% branches that use them never run; they are kept as enctex1.ch has them.
% TeX Live's string pool also holds the 18 strings of encTeX's and ML\TeX's
% own code (\mubyte, \noconvert, \xordcode & co., \charsubdef and its
% substitution messages), which only `-enc' and `-mltex' reach; changes/pool.ch
% puts them in this engine's pool too, in TeX Live's places (#2208).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.729 - enctex1.ch [2.20]: encTeX's global declarations used here
@!xprn: array [ASCII_code] of boolean;
  {is the character printable? (tex.ch)}
@y
@!xprn: array [ASCII_code] of boolean;
  {is the character printable? (tex.ch)}
@!mubyte_write: array [ASCII_code] of str_number;
   { non zero iff character expands to multi bytes in log and write files }
@!no_convert: boolean; { conversion supressed by noconvert primitive }
@!active_noconvert: boolean; { true if noconvert primitive is active }
@!special_printing: boolean; { true only if we need converting in special }
@!message_printing: boolean; { true if message or errmessage prints to string }
@z

@x pdftex.web l.873 - enctex1.ch [2.23]: encTeX's data
for i:=@'177 to @'377 do xchr[i]:=i;
@y
for i:=@'177 to @'377 do xchr[i]:=i;
{Initialize enc\TeX\ data.}
for i:=0 to 255 do mubyte_write[i]:=0;
special_printing := false; message_printing := false;
no_convert := false; active_noconvert := false;
@z

@x pdftex.web l.1667 - enctex1.ch [5.59]: |xprn| decides at print time
  else begin if selector>pseudo then
      begin print_char(s); return; {internal strings are not expanded}
      end;
    if (@<Character |s| is the current new-line character@>) then
      if selector<pseudo then
        begin print_ln; return;
        end;
@y
  else begin if (selector>pseudo) and (not special_printing)
                 and (not message_printing) then
      begin print_char(s); return; {internal strings are not expanded}
      end;
    if (@<Character |s| is the current new-line character@>) then
      if selector<pseudo then
        begin print_ln; no_convert := false; return;
        end
      else if message_printing then
        begin print_char(s); no_convert := false; return;
        end;
    if (mubyte_log>0) and (not no_convert) and (mubyte_write[s]>0) then
      s := mubyte_write[s]
    else if xprn[s] or special_printing then
      begin print_char(s); no_convert := false; return; end;
    no_convert := false;
@z

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

@x web2c-hooks.ch's line - enctex2.ch [17.236]: \mubytein ... \specialout
@d tracing_char_sub_def==int_par(tracing_char_sub_def_code)
@y
@d tracing_char_sub_def==int_par(tracing_char_sub_def_code)
@d mubyte_in==int_par(mubyte_in_code)
@d mubyte_out==int_par(mubyte_out_code)
@d mubyte_log==int_par(mubyte_log_code)
@d spec_out==int_par(spec_out_code)
@z

@x web2c-hooks.ch's line - enctex2.ch [17.237]: \mubytein ... \specialout
tracing_char_sub_def_code:print_esc("tracingcharsubdef");
@y
tracing_char_sub_def_code:print_esc("tracingcharsubdef");
mubyte_in_code:print_esc("mubytein");
mubyte_out_code:print_esc("mubyteout");
mubyte_log_code:print_esc("mubytelog");
spec_out_code:print_esc("specialout");
@z

@x pdftex.web l.32554 - enctex2.ch [49.1279]: implement \noconvert
old_setting:=selector; selector:=new_string;
token_show(def_ref); selector:=old_setting;
@y
old_setting:=selector; selector:=new_string;
message_printing := true; active_noconvert := true;
token_show(def_ref);
message_printing := false; active_noconvert := false;
selector:=old_setting;
@z

@x pdftex.web l.32565 - enctex2.ch [49.1279]: \message goes directly to |print|
slow_print(s); update_terminal;
@y
print(s); update_terminal;
@z

@x pdftex.web l.32578 - enctex2.ch [49.1279]: \errmessage goes directly to |print|
begin print_err(""); slow_print(s);
@y
begin print_err(""); print(s);
@z
