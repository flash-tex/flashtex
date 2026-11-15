% changes/pdftex-mltex.ch -- pdftex.ch's ML\TeX\ hunks.
%
% TeX Live's pdftex.ch (pdftexdir/pdftex.ch, TeX Live 2026) carries tex.ch's
% ML\TeX\ (changes/tex-mltex.ch) into pdfTeX's own code: the character
% tests of pdfTeX's routines (|char_tag| in the font expansion code,
% |is_valid_char|, |get_charwidth| and its kin) read |orig_char_info|, and
% |pdf_hlist_out| outputs a substitution as |hlist_out| does in DVI mode.
% It also drops |pdf_hlist_out|'s call of |char_warning|: a character that
% is not in its font is output only as a substitution (ML\TeX\ on), and
% otherwise not at all, without a message, as in TeX Live's pdfTeX.
% |effective_char| moves among the procedures declared before
% |read_font_info|'s callers.
%
% These are those hunks, text and all, in pdftex.ch's order, applied where
% pdftex.ch is in pdftexdir/am/pdftex.am: after the SyncTeX files
% (changes/synctex.ch) and before char-warning-pdftex.ch.
%
% MLTeX is copyright (C) 1990-92 by Michael J. Ferguson; all rights reserved.
% MLTeX Version 2.2 is copyright (C) 1995 by B. Raichle; all rights reserved.
% (changes/tex-mltex.ch has ML\TeX's terms.) The rest of crates/flashtex-engine
% is GPL-2.0-or-later.

@x (MLTeX) l.12945
      begin i := char_tag(char_info(f)(c));
@y
      begin i := char_tag(orig_char_info(f)(c));
@z

@x (MLTeX) l.14640
@d is_valid_char(#)==((font_bc[f] <= #) and (# <= font_ec[f]) and
                      char_exists(char_info(f)(#)))
@y
@d is_valid_char(#)==((font_bc[f] <= #) and (# <= font_ec[f]) and
                      char_exists(orig_char_info(f)(#)))
@z

@x (MLTeX) l.14678
function get_charwidth(f: internal_font_number; c: eight_bits): scaled;
begin
    if is_valid_char(c) then
        get_charwidth := char_width(f)(char_info(f)(c))
    else
        get_charwidth := 0;
end;

function get_charheight(f: internal_font_number; c: eight_bits): scaled;
begin
    if is_valid_char(c) then
        get_charheight := char_height(f)(height_depth(char_info(f)(c)))
    else
        get_charheight := 0;
end;

function get_chardepth(f: internal_font_number; c: eight_bits): scaled;
begin
    if is_valid_char(c) then
        get_chardepth := char_depth(f)(height_depth(char_info(f)(c)))
    else
        get_chardepth := 0;
end;
@y
function get_charwidth(f: internal_font_number; c: eight_bits): scaled;
begin
    if is_valid_char(c) then
        get_charwidth := char_width(f)(orig_char_info(f)(c))
    else
        get_charwidth := 0;
end;

function get_charheight(f: internal_font_number; c: eight_bits): scaled;
begin
    if is_valid_char(c) then
        get_charheight := char_height(f)(height_depth(orig_char_info(f)(c)))
    else
        get_charheight := 0;
end;

function get_chardepth(f: internal_font_number; c: eight_bits): scaled;
begin
    if is_valid_char(c) then
        get_chardepth := char_depth(f)(height_depth(orig_char_info(f)(c)))
    else
        get_chardepth := 0;
end;
@z

@x (MLTeX) l.17413
label reswitch, move_past, fin_rule, next_p;
@y
label reswitch, move_past, fin_rule, next_p, found, continue;
@z

@x (MLTeX) l.17462
  if is_valid_char(c) then
      output_one_char(c)
  else
      char_warning(f, c);
  cur_h:=cur_h+char_width(f)(char_info(f)(c));
@y
  if is_valid_char(c) then begin
      output_one_char(c);
      cur_h:=cur_h+char_width(f)(char_info(f)(c));
      goto continue;
  end;
  if mltex_enabled_p then
    @<(\pdfTeX) Output a substitution, |goto continue| if not possible@>;
continue:
@z

@x tex.ch l.4858 - effective_char function needed earlier
replacement, but always existing character |font_bc[f]|.
@^inner loop@>

@<Declare additional functions for ML\TeX@>=
function effective_char(@!err_p:boolean;
@y
replacement, but always existing character |font_bc[f]|.
@^inner loop@>

@<Declare \eTeX\ procedures for sc...@>=
function effective_char(@!err_p:boolean;
@z

@x tex.ch l.4958 - pdftex character substitutions
@ The global variables for the code to substitute a virtual character
@y
@ \pdfTeX's |pdf_hlist_out| uses a similar, but slightly modified code
section of the (almost) same name.

@<(\pdfTeX) Output a substitution, |goto continue| if not possible@>=
  begin
  @<Get substitution information, check it, goto |found|
  if all is ok, otherwise goto |continue|@>;
found: @<Print character substitution tracing log@>;
  @<(\pdfTeX) Rebuild character using substitution information@>;
  end

@ The global variables for the code to substitute a virtual character
@z

@x tex.ch l.5071 - more pdftex substitution
@ Dumping ML\TeX-related material.  This is just the flag in the
@y
@ \pdfTeX's |pdf_hlist_out| uses a similar, but slightly modified code
section of the (almost) same name.

@<(\pdfTeX) Rebuild character using substitution information@>=
  base_x_height:=x_height(f);
  base_slant:=slant(f)/float_constant(65536);
@^real division@>
  accent_slant:=base_slant; {slant of accent character font}
  base_width:=char_width(f)(ib_c);
  base_height:=char_height(f)(height_depth(ib_c));
  accent_width:=char_width(f)(ia_c);
  accent_height:=char_height(f)(height_depth(ia_c));
  @/{compute necessary horizontal shift (don't forget slant)}@/
  delta:=round((base_width-accent_width)/float_constant(2)+
            base_height*base_slant-base_x_height*accent_slant);
@^real multiplication@>
@^real addition@>
  @/{1. For centering/horizontal shifting insert a kern node.}@/
  cur_h:=cur_h+delta;
  @/{2. Then insert the accent character possibly shifted up or down.}@/
  if ((base_height<>base_x_height) and (accent_height>0)) then
    begin {the accent must be shifted up or down}
    cur_v:=base_line+(base_x_height-base_height);
    output_one_char(accent_c);
    cur_v:=base_line;
    end
  else begin
    output_one_char(accent_c);
    end;
  cur_h:=cur_h+accent_width;
  @/{3. For centering/horizontal shifting insert another kern node.}@/
  cur_h:=cur_h+(-accent_width-delta);
  @/{4. Output the base character.}@/
  output_one_char(base_c);
  cur_h:=cur_h+base_width;

@ Dumping ML\TeX-related material.  This is just the flag in the
@z
