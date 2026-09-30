% changes/web2c.ch -- what pdftex.web takes for granted from web2c.
%
% pdftex.web is "pdftex without system-dependent changes"
% (pdftexdir/change-files.txt), but it is not a complete program on its own:
% it uses identifiers that only web2c's tex.ch defines, and pointer arrays
% that only web2c allocates. This change file supplies them, re-specified per
% docs/design/engine-v2/DESIGN.md section 4.1. Each change names the web2c
% original it stands in for.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.395 - web2c's run-time arrays are allocated before anything else
  begin @<Initialize whatever \TeX\ might access@>@;
@y
  begin @<Allocate the arrays that web2c allocates@>@;
  @<Initialize whatever \TeX\ might access@>@;
@z

@x pdftex.web l.729 - tex.ch [2.20]: which characters are printable
@!xchr: array [ASCII_code] of text_char;
  {specifies conversion of output characters}
@y
@!xchr: array [ASCII_code] of text_char;
  {specifies conversion of output characters}
@!xprn: array [ASCII_code] of boolean;
  {is the character printable? (tex.ch)}
@z

@x pdftex.web l.873 - tex.ch [2.23]: every character is itself, none is invalid
for i:=0 to @'37 do xchr[i]:=' ';
for i:=@'177 to @'377 do xchr[i]:=' ';
@y
{Initialize |xchr| to the identity mapping.}
for i:=0 to @'37 do xchr[i]:=i;
for i:=@'177 to @'377 do xchr[i]:=i;
@z

@x pdftex.web l.885 - tex.ch [2.24]: printable ASCII, \.{-8bit}, the TCX file
for i:=0 to @'176 do xord[xchr[i]]:=i;
@y
for i:=0 to @'176 do xord[xchr[i]]:=i;
{Set |xprn| for printable ASCII, unless |eight_bit_p| is set.}
for i:=0 to 255 do xprn[i]:=(eight_bit_p or ((i>=" ")and(i<="~")));

{The idea for this dynamic translation comes from the patch by
 Libor Skarvada \.{<libor@@informatics.muni.cz>}
 and Petr Sojka \.{<sojka@@informatics.muni.cz>}. I didn't use any of the
 actual code, though, preferring a more general approach.}

{This updates the |xchr|, |xord|, and |xprn| arrays from the provided
 |translate_filename|.  See the function definition in \.{texmfmp.c} for
 more comments.}
if translate_filename_p then read_tcx_file;
@z

@x pdftex.web l.1449 - tex.ch [4.49]: |xprn| says what is printable
@<Character |k| cannot be printed@>=
  (k<" ")or(k>"~")
@y
@<Character |k| cannot be printed@>=
  not xprn[k]
@z

@x pdftex.web l.2948 - tex.ch [8.111]: more than 256 fonts
if (font_base<min_quarterword)or(font_max>max_quarterword) then bad:=15;
if font_max>font_base+256 then bad:=16;
@y
if (max_font_max<min_halfword)or(max_font_max>max_halfword) then bad:=15;
if font_max>font_base+max_font_max then bad:=16;
@z

@x pdftex.web l.2987 - texmfmem.h: the |b0| and |b1| of a |two_halves| are C shorts
  2: (@!b0:quarterword; @!b1:quarterword);
@y
  2: (@!b1:min_quarterword..@"FFFF; @!b0:min_quarterword..@"FFFF);
    {16 bits, so that a |char_node| can hold a font number above 255;
     |b1| is the low half of |lh| and |b0| the high half, as texmfmem.h's
     \.{short B1, B0} lays them out on a little-endian machine}
@z

@x pdftex.web l.5200 - pdftex.ch: the primitives' own |eqtb| entries must fit
@d frozen_null_font=frozen_control_sequence+10
@y
@d frozen_null_font=frozen_control_sequence+12+prim_size
@z

@x pdftex.web l.5207 - tex.ch [17.222]: room for |max_font_max| font identifiers
@d undefined_control_sequence=frozen_null_font+257 {dummy location}
@y
@d max_font_max=9000 {the largest |font_max| (tex.ch)}
@d undefined_control_sequence=frozen_null_font+max_font_max+1 {dummy location}
@z

@x pdftex.web l.6184 - tex.ch: the date (texmfmp.c's |get_date_and_time|)
begin sys_time:=12*60;
sys_day:=4; sys_month:=7; sys_year:=1776;  {self-evident truths}
@y
begin date_and_time(sys_time,sys_day,sys_month,sys_year);
@z

@x pdftex.web l.8959 - tex.ch [25.366]: expansion depth overflow
begin cv_backup:=cur_val; cvl_backup:=cur_val_level; radix_backup:=radix;
@y
begin
incr(expand_depth_count);
if expand_depth_count>=expand_depth then overflow("expansion depth",expand_depth);
cv_backup:=cur_val; cvl_backup:=cur_val_level; radix_backup:=radix;
@z

@x pdftex.web l.8966 - tex.ch [25.366]: expansion depth overflow
cur_order:=co_backup; link(backup_head):=backup_backup;
@y
cur_order:=co_backup; link(backup_head):=backup_backup;
decr(expand_depth_count);
@z

@x pdftex.web l.13015 - tex.ch's MLTeX |orig_char_info|; without MLTeX it is |char_info|
@d char_info(#)==font_info[char_base[#]+char_info_end
@y
@d char_info(#)==font_info[char_base[#]+char_info_end
@d orig_char_info_end(#)==#].qqqq
@d orig_char_info(#)==font_info[char_base[#]+orig_char_info_end
@z

@x pdftex.web l.14226 - tex.ch [32.602]: more than 256 fonts in the DVI file
begin dvi_out(fnt_def1);
dvi_out(f-font_base-1);@/
@y
begin if f<=256+font_base then
  begin dvi_out(fnt_def1);
  dvi_out(f-font_base-1);
  end
else begin dvi_out(fnt_def1+1);
  dvi_out((f-font_base-1) div @'400);
  dvi_out((f-font_base-1) mod @'400);
  end;
@z

@x pdftex.web l.14682 - tex.ch [32.621]: more than 256 fonts in the DVI file
else  begin dvi_out(fnt1); dvi_out(f-font_base-1);
  end;
@y
else if f<=256+font_base then
  begin dvi_out(fnt1); dvi_out(f-font_base-1);
  end
else begin dvi_out(fnt1+1);
  dvi_out((f-font_base-1) div @'400);
  dvi_out((f-font_base-1) mod @'400);
  end;
@z

@x pdftex.web l.17948 - a font map entry is a handle into the Rust font map
fm_entry_ptr = ^integer;
@y
fm_entry_ptr = integer; {0, or a handle into the font map of \.{src/pdftex/}}
@z

@x pdftex.web l.27075 - tex.ch [43.944]: more than 255 ops per language (bigtrie)
    if u=max_quarterword then
      overflow("pattern memory ops per language",
        max_quarterword-min_quarterword);
@y
    if u=max_trie_op then
      overflow("pattern memory ops per language",
      max_trie_op-min_quarterword);
@z

@x pdftex.web l.33205 - tex.ch [50.1325]: ops above 255 in a format (bigtrie)
  undump(min_quarterword)(max_quarterword)(hyf_next[k]);
@y
  undump(min_quarterword)(max_trie_op)(hyf_next[k]);
@z

@x pdftex.web l.33470 - tex.ch [51.1333]: a new line before termination
    slow_print(log_name); print_char(".");
    end;
  end;
end;
@y
    slow_print(log_name); print_char(".");
    end;
  end;
print_ln;
end;
@z

@x pdftex.web l.33491 - tex.ch [51.1334]: |hash_extra| (always 0 here)
  wlog_ln(' ',cs_count:1,' multiletter control sequences out of ',
    hash_size:1);@/
@y
  wlog_ln(' ',cs_count:1,' multiletter control sequences out of ',
    hash_size:1, '+0');@/
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ tex.ch's ``bigtrie'': a language may have up to |max_trie_op| hyphenation
ops (the German patterns need more than 255), which fits because the |b0|
field that holds |trie_op| is 16 bits wide here.

@d max_trie_op=65535 {largest possible trie opcode for any language (tex.ch)}

@ web2c allocates the following arrays at run time (\.{tex.ch},
\.{pdftex.ch}), sized by \.{texmf.cnf} or grown on demand, and gives each
font's entries their initial values twice, when \.{INITEX} starts and when a
format is loaded. Here all of that happens once, before anything else is
initialized, in \.{INITEX} and in production runs alike. |xmalloc_array(t,n)|
provides the elements |0..n|, as in web2c.

@<Allocate the arrays that web2c allocates@>=
obj_tab:=xmalloc_array(obj_entry,inf_obj_tab_size);
pdf_mem:=xmalloc_array(integer,inf_pdf_mem_size);
dest_names:=xmalloc_array(dest_name_entry,inf_dest_names_size);
pdf_op_buf:=xmalloc_array(eight_bits,pdf_op_buf_size);
pdf_os_buf:=xmalloc_array(eight_bits,inf_pdf_os_buf_size);
pdf_os_objnum:=xmalloc_array(integer,pdf_os_max_objs);
pdf_os_objoff:=xmalloc_array(integer,pdf_os_max_objs);
pdf_char_used:=xmalloc_array(char_used_array,font_max);
pdf_font_size:=xmalloc_array(scaled,font_max);
pdf_font_num:=xmalloc_array(integer,font_max);
pdf_font_map:=xmalloc_array(fm_entry_ptr,font_max);
pdf_font_type:=xmalloc_array(eight_bits,font_max);
pdf_font_attr:=xmalloc_array(str_number,font_max);
pdf_font_blink:=xmalloc_array(internal_font_number,font_max);
pdf_font_elink:=xmalloc_array(internal_font_number,font_max);
pdf_font_has_space_char:=xmalloc_array(boolean,font_max);
pdf_font_stretch:=xmalloc_array(integer,font_max);
pdf_font_shrink:=xmalloc_array(integer,font_max);
pdf_font_step:=xmalloc_array(integer,font_max);
pdf_font_expand_ratio:=xmalloc_array(integer,font_max);
pdf_font_auto_expand:=xmalloc_array(boolean,font_max);
pdf_font_lp_base:=xmalloc_array(integer,font_max);
pdf_font_rp_base:=xmalloc_array(integer,font_max);
pdf_font_ef_base:=xmalloc_array(integer,font_max);
pdf_font_kn_bs_base:=xmalloc_array(integer,font_max);
pdf_font_st_bs_base:=xmalloc_array(integer,font_max);
pdf_font_sh_bs_base:=xmalloc_array(integer,font_max);
pdf_font_kn_bc_base:=xmalloc_array(integer,font_max);
pdf_font_kn_ac_base:=xmalloc_array(integer,font_max);
vf_packet_base:=xmalloc_array(integer,font_max);
vf_default_font:=xmalloc_array(internal_font_number,font_max);
vf_local_font_num:=xmalloc_array(internal_font_number,font_max);
vf_e_fnts:=xmalloc_array(integer,font_max);
vf_i_fnts:=xmalloc_array(internal_font_number,font_max);
pdf_font_nobuiltin_tounicode:=xmalloc_array(boolean,font_max);
for i := font_base to font_max do begin
    for k := 0 to 31 do
        pdf_char_used[i, k] := 0;
    pdf_font_size[i] := 0;
    pdf_font_num[i] := 0;
    pdf_font_map[i] := 0;
    pdf_font_type[i] := new_font_type;
    pdf_font_attr[i] := "";
    pdf_font_blink[i] := null_font;
    pdf_font_elink[i] := null_font;
    pdf_font_has_space_char[i] := false;
    pdf_font_stretch[i] := null_font;
    pdf_font_shrink[i] := null_font;
    pdf_font_step[i] := 0;
    pdf_font_expand_ratio[i] := 0;
    pdf_font_auto_expand[i] := false;
    pdf_font_lp_base[i] := 0;
    pdf_font_rp_base[i] := 0;
    pdf_font_ef_base[i] := 0;
    pdf_font_kn_bs_base[i] := 0;
    pdf_font_st_bs_base[i] := 0;
    pdf_font_sh_bs_base[i] := 0;
    pdf_font_kn_bc_base[i] := 0;
    pdf_font_kn_ac_base[i] := 0;
    pdf_font_nobuiltin_tounicode[i] := false;
end;

@ web2c's \.{tex.ch} limits the recursion of |expand| (its part
\.{[54/web2c]}): a counter tracks the depth, and |expand_depth| (10000,
\.{texmf.cnf}'s default) bounds it. pdftex.web's |scan_expr| already uses
both. \.{texmf.cnf} also supplies |pk_dpi|, and \.{texmfmp.c} the state of
\.{\\write18}; both limits and the shell switches are set up in
\.{web2c-run.ch}.

@<Glob...@>=
@!expand_depth:integer; {limits recursive calls of |expand| and |scan_expr|}
@!expand_depth_count:integer; {current depth of those calls}
@!shellenabledp:boolean; {is \.{\\write18} enabled?}
@!restrictedshell:boolean; {is it restricted to a list of programs?}

@ @<Set init...@>=
expand_depth_count:=0;
pk_dpi:=72;

@* \[55] Index.
@z
