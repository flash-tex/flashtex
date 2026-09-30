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
  2: (@!b0:min_quarterword..@"FFFF; @!b1:min_quarterword..@"FFFF);
    {16 bits, so that a |char_node| can hold a font number above 255}
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

@x pdftex.web l.5214 - tex.ch [17.222]: |hash_extra|
for k:=active_base to undefined_control_sequence-1 do
  eqtb[k]:=eqtb[undefined_control_sequence];
@y
for k:=active_base to eqtb_top do
  eqtb[k]:=eqtb[undefined_control_sequence];
@z

@x pdftex.web l.5433 - tex.ch [17.230]: ML\TeX's |char_sub_code_base|
@d int_base=math_code_base+256 {beginning of region 5}
@y
@d char_sub_code_base=math_code_base+256 {table of character substitutions}
@d int_base=char_sub_code_base+256 {beginning of region 5}
@z

@x pdftex.web l.6184 - tex.ch: the date (texmfmp.c's |get_date_and_time|)
begin sys_time:=12*60;
sys_day:=4; sys_month:=7; sys_year:=1776;  {self-evident truths}
@y
begin date_and_time(sys_time,sys_day,sys_month,sys_year);
@z

@x pdftex.web l.6450 - tex.ch [17.252]: |hash_extra|
else if n<glue_base then @<Show equivalent |n|, in region 1 or 2@>
@y
else if (n<glue_base) or ((n>eqtb_size)and(n<=eqtb_top)) then
  @<Show equivalent |n|, in region 1 or 2@>
@z

@x pdftex.web l.6465 - tex.ch [17.253]: |eqtb| goes up to |eqtb_top|
@!eqtb:array[active_base..eqtb_size] of memory_word;
@y
@!eqtb:array[active_base..eqtb_top] of memory_word;
@z

@x pdftex.web l.6513 - tex.ch [18.256]: |hash_extra|
@!hash: array[hash_base..undefined_control_sequence-1] of two_halves;
  {the hash table}
@!hash_used:pointer; {allocation pointer for |hash|}
@y
@!hash: array[hash_base..hash_top] of two_halves;
  {the hash table}
@!hash_used:pointer; {allocation pointer for |hash|}
@!hash_high:pointer; {pointer to next high hash location}
@z

@x pdftex.web l.6544 - tex.ch [18.257]: |hash_extra|
for k:=hash_base+1 to undefined_control_sequence-1 do hash[k]:=hash[hash_base];
@y
for k:=hash_base+1 to hash_top do hash[k]:=hash[hash_base];
@z

@x pdftex.web l.6548 - tex.ch [18.258]: |hash_extra|
hash_used:=frozen_control_sequence; {nothing is used}
@y
hash_used:=frozen_control_sequence; {nothing is used}
hash_high:=0;
@z

@x pdftex.web l.6590 - tex.ch [18.260]: |hash_extra|
begin if text(p)>0 then
  begin repeat if hash_is_full then overflow("hash size",hash_size);
@:TeX capacity exceeded hash size}{\quad hash size@>
  decr(hash_used);
  until text(hash_used)=0; {search for an empty location in |hash|}
  next(p):=hash_used; p:=hash_used;
  end;
@y
begin if text(p)>0 then
  begin if hash_high<hash_extra then
      begin incr(hash_high);
      next(p):=hash_high+eqtb_size; p:=hash_high+eqtb_size;
      end
    else begin
      repeat if hash_is_full then overflow("hash size",hash_size+hash_extra);
@:TeX capacity exceeded hash size}{\quad hash size@>
      decr(hash_used);
      until text(hash_used)=0; {search for an empty location in |hash|}
    next(p):=hash_used; p:=hash_used;
    end;
  end;
@z

@x pdftex.web l.6698 - tex.ch [18.262]: |hash_extra|
else if p>=undefined_control_sequence then print_esc("IMPOSSIBLE.")
@y
else if ((p>=undefined_control_sequence)and(p<=eqtb_size))or(p>eqtb_top) then
  print_esc("IMPOSSIBLE.")
@z

@x pdftex.web l.7294 - tex.ch [19.283]: |hash_extra|
if p<int_base then
  if eq_level(p)=level_one then
@y
if (p<int_base)or(p>eqtb_size) then
  if eq_level(p)=level_one then
@z

@x pdftex.web l.7403 - tex.ch [20.290]: |hash_extra|
if cs_token_flag+undefined_control_sequence>max_halfword then bad:=21;
@y
if cs_token_flag+eqtb_size+hash_extra>max_halfword then bad:=21;
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

@x pdftex.web l.31695 - tex.ch [49.1215]: |hash_extra|
if (cur_cs=0)or(cur_cs>frozen_control_sequence) then
@y
if (cur_cs=0)or(cur_cs>eqtb_top)or
  ((cur_cs>frozen_control_sequence)and(cur_cs<=eqtb_size)) then
@z

@x pdftex.web l.32880 - tex.ch [50.1307]: |hash_high| in the format
dump_int(eqtb_size);@/
@y
dump_int(eqtb_size);@/
dump_int(hash_high);@/
@z

@x pdftex.web l.32899 - tex.ch [50.1308]: |hash_high| in the format
if x<>eqtb_size then goto bad_fmt;
@y
if x<>eqtb_size then goto bad_fmt;
undump(0)(hash_extra)(hash_high);
@z

@x pdftex.web l.33003 - tex.ch [50.1314]: |hash_extra|
undump(hash_base)(frozen_control_sequence)(write_loc);@/
@y
undump(hash_base)(hash_top)(write_loc);@/
@z

@x pdftex.web l.33050 - tex.ch [50.1316]: dump the |hash_extra| part
until k>eqtb_size
@y
until k>eqtb_size;
if hash_high>0 then for k:=eqtb_size+1 to eqtb_size+hash_high do
  dump_wd(eqtb[k]); {dump |hash_extra| part}
@z

@x pdftex.web l.33062 - tex.ch [50.1308, 50.1317]: undump the |hash_extra| part
until k>eqtb_size
@y
until k>eqtb_size;
for j:=eqtb_size+1 to eqtb_top do eqtb[j]:=eqtb[undefined_control_sequence];
if hash_high>0 then for j:=eqtb_size+1 to eqtb_size+hash_high do
  undump_wd(eqtb[j]); {undump |hash_extra| part}
@z

@x pdftex.web l.33071 - tex.ch [50.1318]: |hash_extra|
dump_int(hash_used); cs_count:=frozen_control_sequence-1-hash_used;
@y
dump_int(hash_used); cs_count:=frozen_control_sequence-1-hash_used+hash_high;
@z

@x pdftex.web l.33075 - tex.ch [50.1318]: |hash_extra|
for p:=hash_used+1 to undefined_control_sequence-1 do dump_hh(hash[p]);
@y
for p:=hash_used+1 to undefined_control_sequence-1 do dump_hh(hash[p]);
if hash_high>0 then for p:=eqtb_size+1 to eqtb_size+hash_high do
  dump_hh(hash[p]);
@z

@x pdftex.web l.33084 - tex.ch [50.1319]: |hash_extra|
for p:=hash_used+1 to undefined_control_sequence-1 do undump_hh(hash[p]);
@y
for p:=hash_used+1 to undefined_control_sequence-1 do undump_hh(hash[p]);
if hash_high>0 then for p:=eqtb_size+1 to eqtb_size+hash_high do
  undump_hh(hash[p]);
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

@x pdftex.web l.33491 - tex.ch [51.1334]: |hash_extra|
  wlog_ln(' ',cs_count:1,' multiletter control sequences out of ',
    hash_size:1);@/
@y
  wlog_ln(' ',cs_count:1,' multiletter control sequences out of ',
    hash_size:1, '+', hash_extra:1);@/
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ tex.ch's |hash_extra|: |hash_size| control sequences hash into the table
below |frozen_control_sequence|, as in \.{tex.web}, and a name whose place
is taken goes to one of |hash_extra| more above |eqtb_size|, then (when
those are used up) below |hash_used|. |hash_extra| is texmf.cnf's value, a
constant of the configuration here.

@d hash_extra=0 {texmf.cnf's |hash_extra|; the configuration sets it}
@d eqtb_top==eqtb_size+hash_extra {the largest |eqtb| index}
@d hash_top==eqtb_top {the largest |hash| index}

@ The layout as constants of the outer block, so that the Rust parts
(\.{src/readset.rs}, \.{src/iso.rs}, \.{src/displaylist/}) read it from
\.{src/generated/consts.rs} instead of repeating the numbers.

@<Constants in the outer block@>=
@!layout_frozen_control_sequence=frozen_control_sequence;
@!layout_undefined_control_sequence=undefined_control_sequence;
@!layout_glue_base=glue_base;
@!layout_local_base=local_base;
@!layout_int_base=int_base;
@!layout_count_base=count_base;
@!layout_mag_loc=int_base+mag_code;
@!layout_eqtb_size=eqtb_size;
@!layout_eqtb_top=eqtb_top;
@!layout_hash_prime=hash_prime;
@!layout_etex_int_base=etex_int_base;

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
