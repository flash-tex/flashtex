% changes/web2c.ch -- what xetex.web takes for granted from web2c.
%
% The XeTeX counterpart of crates/flashtex-engine/changes/web2c.ch, hunk for
% hunk where xetex.web has the same text (docs/design/xetex/PLAN.md). Like
% pdftex.web, xetex.web is not a complete program on its own: it uses
% identifiers that only web2c's tex.ch defines, and pointer arrays that only
% web2c allocates. This change file supplies them, re-specified per
% docs/design/engine-v2/DESIGN.md section 4.1, with TeX Live's
% xetexdir/xetex.ch applied on top of tex.ch where it changes tex.ch's text
% (each change names its origin). XeTeX's own C parts are in ext.ch, its
% other web2c changes in xetex.ch.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.429 - web2c's run-time arrays are allocated before anything else
  begin @<Initialize whatever \TeX\ might access@>@;
@y
  begin @<Allocate the arrays that web2c allocates@>@;
  @<Initialize whatever \TeX\ might access@>@;
@z

@x xetex.web l.791 - xetex.h: |xchr| is the identity (|Xchr(x)=x|), for the bytes |print_raw_char| writes
@!xchr: array [ASCII_code] of text_char;
  {specifies conversion of output characters}
@y
@!xchr: array [0..255] of text_char;
  {the identity: |print_raw_char| writes bytes (xetex.h's |Xchr|)}
@z

@x xetex.web l.838 - xetex.h: every byte is itself; there is no |xord| or |xprn|
for i:=0 to @'37 do xchr[i]:=' ';
for i:=@'177 to @'377 do xchr[i]:=' ';
@y
for i:=0 to 255 do xchr[i]:=i;
@z

@x xetex.web l.848 - xetex.ch [2.24]: |xord| is not used; no TCX file
for i:=0 to @'176 do xord[xchr[i]]:=i;
@y
@z

@x xetex.web l.2165 - tex.ch [6.84]: `\.E' switches to the editor
"E": if base_ptr>0 then if input_stack[base_ptr].name_field>=256 then
  begin print_nl("You want to edit file ");
@.You want to edit file x@>
  slow_print(input_stack[base_ptr].name_field);
  print(" at line "); print_int(line);
  interaction:=scroll_mode; jump_out;
@y
"E": if base_ptr>0 then if input_stack[base_ptr].name_field>=256 then
  begin edit_name_start:=str_start_macro(input_stack[base_ptr].name_field);
  edit_name_length:=length(input_stack[base_ptr].name_field);
  edit_line:=line;
  jump_out;
@z

@x xetex.web l.2493 - tex.ch [7.104]: |save_arith_error|
@!arith_error:boolean; {has arithmetic overflow occurred recently?}
@y
@!arith_error:boolean; {has arithmetic overflow occurred recently?}
@!save_arith_error:boolean; {for saving and restoring |arith_error| (tex.ch)}
@z

@x xetex.web l.3046 - tex.ch [8.111]: more than 256 fonts
if (font_base<min_quarterword)or(font_max>max_quarterword) then bad:=15;
if font_max>font_base+256 then bad:=16;
@y
if (max_font_max<min_halfword)or(max_font_max>max_halfword) then bad:=15;
if font_max>font_base+max_font_max then bad:=16;
@z

@x xetex.web l.3062 - tex.ch [8.112]: |qi|, |qo|, |hi| and |ho| are the identity
@d qi(#)==#+min_quarterword
  {to put an |eight_bits| item into a quarterword}
@d qo(#)==#-min_quarterword
  {to take an |eight_bits| item out of a quarterword}
@d hi(#)==#+min_halfword
  {to put a sixteen-bit item into a halfword}
@d ho(#)==#-min_halfword
  {to take a sixteen-bit item from a halfword}
@y
@d qi(#)==# {to put an |eight_bits| item into a quarterword}
@d qo(#)==# {to take an |eight_bits| item from a quarterword}
@d hi(#)==# {to put a sixteen-bit item into a halfword}
@d ho(#)==# {to take a sixteen-bit item from a halfword}
  {(tex.ch [8.112]: web2c's |hi| and |ho| do not add |min_halfword|, which
   matters here: XeTeX's |min_halfword| is negative)}
@z

@x xetex.web l.3085 - texmfmem.h: the |b0| and |b1| of a |two_halves| are C shorts
  2: (@!b0:quarterword; @!b1:quarterword);
@y
  2: (@!b1:min_quarterword..@"FFFF; @!b0:min_quarterword..@"FFFF);
    {16 bits, so that a |char_node| can hold a font number above 255;
     |b1| is the low half of |lh| and |b0| the high half, as texmfmem.h's
     \.{short B1, B0} lays them out on a little-endian machine}
@z

@x xetex.web l.4590 - tex.ch [12.186]: no |"?.?"| for a strange glue ratio
  if abs(mem[p+glue_offset].int)<@'4000000 then print("?.?")
  else if abs(g)>float_constant(20000) then
@y
  {tex.ch [12.186], ``Don't worry about strange floating point values'':
   web2c drops this test, so a box whose |glue_set| is a stale bit pattern
   (|hpack| with |cal_expand_ratio| and |font_expand_ratio=0| leaves it as
   |get_node| found it) prints its value as any other.
  |if abs(mem[p+glue_offset].int)<@'4000000 then print('?.?')|
  |else| }
  if abs(g)>float_constant(20000) then
@z

@x xetex.web l.5422 - pdftex.ch: the primitives' own |eqtb| entries must fit
@d frozen_null_font=frozen_control_sequence+10
@y
@d frozen_null_font=frozen_control_sequence+12+prim_size
@z

@x xetex.web l.5429 - tex.ch [17.222]: room for |max_font_max| font identifiers
@d undefined_control_sequence=frozen_null_font+257 {dummy location}
@y
@d max_font_max=9000 {the largest |font_max| (tex.ch)}
@d undefined_control_sequence=frozen_null_font+max_font_max+1 {dummy location}
@z

@x xetex.web l.5436 - tex.ch [17.222]: |hash_extra|
for k:=active_base to undefined_control_sequence-1 do
  eqtb[k]:=eqtb[undefined_control_sequence];
@y
for k:=active_base to eqtb_top do
  eqtb[k]:=eqtb[undefined_control_sequence];
@z

@x xetex.web l.5655 - tex.ch [17.230], xetex.ch [17.230]: ML\TeX's |char_sub_code_base|
@d int_base=math_code_base+number_usvs {beginning of region 5}
@y
@d char_sub_code_base=math_code_base+number_usvs {table of character substitutions}
@d int_base=char_sub_code_base+number_usvs {beginning of region 5}
@z

@x xetex.web l.6207 - tex.ch: the date (texmfmp.c's |get_date_and_time|)
begin sys_time:=12*60;
sys_day:=4; sys_month:=7; sys_year:=1776;  {self-evident truths}
@y
begin date_and_time(sys_time,sys_day,sys_month,sys_year);
@z

@x xetex.web l.6416 - tex.ch [17.252]: |hash_extra|
else if n<glue_base then @<Show equivalent |n|, in region 1 or 2@>
@y
else if (n<glue_base) or ((n>eqtb_size)and(n<=eqtb_top)) then
  @<Show equivalent |n|, in region 1 or 2@>
@z

@x xetex.web l.6431 - tex.ch [17.253]: |eqtb| goes up to |eqtb_top|
@!eqtb:array[active_base..eqtb_size] of memory_word;
@y
@!eqtb:array[active_base..eqtb_top] of memory_word;
@z

@x xetex.web l.6479 - tex.ch [18.256]: |hash_extra|
@!hash: array[hash_base..undefined_control_sequence-1] of two_halves;
  {the hash table}
@!hash_used:pointer; {allocation pointer for |hash|}
@y
@!hash: array[hash_base..hash_top] of two_halves;
  {the hash table}
@!hash_used:pointer; {allocation pointer for |hash|}
@!hash_high:pointer; {pointer to next high hash location}
@z

@x xetex.web l.6509 - tex.ch [18.257]: |hash_extra|
for k:=hash_base+1 to undefined_control_sequence-1 do hash[k]:=hash[hash_base];
@y
for k:=hash_base+1 to hash_top do hash[k]:=hash[hash_base];
@z

@x xetex.web l.6513 - tex.ch [18.258]: |hash_extra|
hash_used:=frozen_control_sequence; {nothing is used}
@y
hash_used:=frozen_control_sequence; {nothing is used}
hash_high:=0;
@z

@x xetex.web l.6557 - tex.ch [18.260]: |hash_extra|
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

@x xetex.web l.6672 - tex.ch [18.262]: |hash_extra|
else if p>=undefined_control_sequence then print_esc("IMPOSSIBLE.")
@y
else if ((p>=undefined_control_sequence)and(p<=eqtb_size))or(p>eqtb_top) then
  print_esc("IMPOSSIBLE.")
@z

@x xetex.web l.7233 - tex.ch [19.283]: |hash_extra|
if p<int_base then
  if eq_level(p)=level_one then
@y
if (p<int_base)or(p>eqtb_size) then
  if eq_level(p)=level_one then
@z

@x xetex.web l.7344 - tex.ch [20.290]: |hash_extra|
if cs_token_flag+undefined_control_sequence>max_halfword then bad:=21;
@y
if cs_token_flag+eqtb_size+hash_extra>max_halfword then bad:=21;
@z

@x xetex.web l.8975 - tex.ch [25.366]: expansion depth overflow
begin cv_backup:=cur_val; cvl_backup:=cur_val_level; radix_backup:=radix;
@y
begin
incr(expand_depth_count);
if expand_depth_count>=expand_depth then overflow("expansion depth",expand_depth);
cv_backup:=cur_val; cvl_backup:=cur_val_level; radix_backup:=radix;
@z

@x xetex.web l.8982 - tex.ch [25.366]: expansion depth overflow
cur_order:=co_backup; link(backup_head):=backup_backup;
@y
cur_order:=co_backup; link(backup_head):=backup_backup;
decr(expand_depth_count);
@z

@x xetex.web l.9029 - tex.ch [25.369]: disallow \.{\\noexpand\\endwrite}
if t>=cs_token_flag then
@y
if (t>=cs_token_flag)and(t<>end_write_token) then
@z

@x xetex.web l.10626 - tex.ch [26.449]: recover better from \.{\\mkern} <non-mu-dimen-or-skip>
  begin scan_something_internal(mu_val,false);
  @<Coerce glue to a dimension@>;
  if cur_val_level=mu_val then goto attach_sign;
  if cur_val_level<>int_val then mu_error;
@y
  begin scan_something_internal(mu_val,false);
  if cur_val_level<>int_val then
    begin
    @<Coerce glue to a dimension@>;
    if cur_val_level<>mu_val then mu_error;
    goto attach_sign;
    end;
@z

@x xetex.web l.13098 - tex.ch [30.560]: check the lengths of a TFM name
@!file_opened:boolean; {was |tfm_file| successfully opened?}
@y
@!name_too_long:boolean; {|nom| or |aire| exceeds 255 bytes?}
@!file_opened:boolean; {was |tfm_file| successfully opened?}
@z

@x xetex.web l.13187 - tex.ch [30.561], xetex.ch [30.561]: check the lengths of a TFM name
else print(" not loadable: Metric (TFM) file not found");
@y
else if name_too_long then print(" not loadable: Metric (TFM) file name too long")
else print(" not loadable: Metric (TFM) file or installed font not found");
@z

@x xetex.web l.13209 - tex.ch [30.563]: check lengths, don't use |TEX_font_area|
if aire="" then pack_file_name(nom,TEX_font_area,".tfm")
else pack_file_name(nom,aire,".tfm");
@y
name_too_long:=(length(nom)>255)or(length(aire)>255);
if name_too_long then abort;
{|kpse_find_file| will append the |".tfm"|, and avoid searching the disk
 before the font alias files as well.}
pack_file_name(nom,aire,"");
@z

@x xetex.web l.13304 - tex.ch [30.568]: avoid scaling fonts to 2048pt or more
  else z:=xn_over_d(z,-s,1000);
@y
  else begin
    save_arith_error:=arith_error;
    sw:=z; z:=xn_over_d(z,-s,1000);
    if arith_error or (z>=@'1000000000) then begin {web2c's C reading, made explicit}
       start_font_error_message; print(" scaled to 2048pt or higher");
       help1("I will ignore the scaling factor."); error; z:=sw;
       end;
    arith_error:=save_arith_error;
  end;
@z

@x xetex.web l.14202 - tex.ch [32.598]: dvi_swap: check dvi file size
begin if dvi_limit=dvi_buf_size then
@y
begin if dvi_ptr>(@"7FFFFFFF-dvi_offset) then
  begin cur_s:=-2; {the postamble is not written, tex.ch [32.642]}
  fatal_error("dvi length exceeds ""7FFFFFFF");
@.dvi length exceeds...@>
  end;
if dvi_limit=dvi_buf_size then
@z

@x xetex.web l.14216 - tex.ch [32.599]: empty the last bytes: check dvi file size
if dvi_ptr>0 then write_dvi(0,dvi_ptr-1)
@y
if dvi_ptr>(@"7FFFFFFF-dvi_offset) then
  begin cur_s:=-2;
  fatal_error("dvi length exceeds ""7FFFFFFF");
@.dvi length exceeds...@>
  end;
if dvi_ptr>0 then write_dvi(0,dvi_ptr-1)
@z

@x xetex.web l.14265 - tex.ch [32.602]: more than 256 fonts in the DVI file
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

@x xetex.web l.14867 - tex.ch [32.621]: more than 256 fonts in the DVI file
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

@x xetex.web l.15305 - tex.ch [32.642]: check dvi file size
else  begin dvi_out(post); {beginning of the postamble}
@y
else if cur_s<>-2 then
  begin dvi_out(post); {beginning of the postamble}
@z

@x xetex.web l.22380 - tex.ch [42.940]: a repeated exception replaces the old one
label reswitch, exit, found, not_found, not_found1;
@y
label reswitch, exit, found, found1, not_found, not_found1;
@z

@x xetex.web l.22472 - tex.ch [42.940]: a repeated exception replaces the old one
hyph_word[h]:=s; hyph_list[h]:=p
@y
found1: hyph_word[h]:=s; hyph_list[h]:=p
@z

@x xetex.web l.22482 - tex.ch [42.941]: a repeated exception replaces the old one
until u=str_start_macro(k+1);
@y
until u=str_start_macro(k+1);
{repeat hyphenation exception; flushing old data (tex.ch [42.941]). The
 table is ordered, so an equal word is met before any interchange, and |s|
 is still the string just made.}
flush_string; s:=hyph_word[h]; {avoid |slow_make_string|!}
decr(hyph_count);
{ We could also |flush_list(hyph_list[h]);|, but it interferes
  with \.{trip.log}. }
goto found1;
@z

@x xetex.web l.22559 - tex.ch [43.944]: more than 255 ops per language (bigtrie)
    if u=max_quarterword then
      overflow("pattern memory ops per language",
        max_quarterword-min_quarterword);
@y
    if u=max_trie_op then
      overflow("pattern memory ops per language",
      max_trie_op-min_quarterword);
@z

@x xetex.web l.27586 - tex.ch [49.1215]: |hash_extra|
if (cur_cs=0)or(cur_cs>frozen_control_sequence) then
@y
if (cur_cs=0)or(cur_cs>eqtb_top)or
  ((cur_cs>frozen_control_sequence)and(cur_cs<=eqtb_size)) then
@z

@x xetex.web l.28382 - tex.ch [49.1260]: no reuse after an overflow; tex.ch [49.1257]: no flushable font name (|end_name| recycles)
flushable_string:=str_ptr-1;
for f:=font_base+1 to font_ptr do begin
  if str_eq_str(font_name[f],cur_name) and
    (((cur_area = "") and is_native_font(f)) or str_eq_str(font_area[f],cur_area)) then
    begin if cur_name=flushable_string then
      begin flush_string; cur_name:=font_name[f];
      end;
    if s>0 then
      begin if s=font_size[f] then goto common_ending;
      end
    else if font_size[f]=xn_over_d(font_dsize[f],-s,1000) then
      goto common_ending;
    end;
@y
for f:=font_base+1 to font_ptr do begin
  if str_eq_str(font_name[f],cur_name) and
    (((cur_area = "") and is_native_font(f)) or str_eq_str(font_area[f],cur_area)) then
    begin if s>0 then
      begin if s=font_size[f] then goto common_ending;
      end
    else begin arith_error:=false; {tex.ch [49.1260]: avoid scaling fonts to 2048pt or more}
      if font_size[f]=xn_over_d(font_dsize[f],-s,1000)
      then if not arith_error
        then goto common_ending;
      end;
    end;
@z

@x xetex.web l.28869 - tex.ch [50.1307]: |hash_high| in the format
dump_int(eqtb_size);@/
@y
dump_int(eqtb_size);@/
dump_int(hash_high);@/
@z

@x xetex.web l.28888 - tex.ch [50.1308]: |hash_high| in the format
if x<>eqtb_size then goto bad_fmt;
@y
if x<>eqtb_size then goto bad_fmt;
undump(0)(hash_extra)(hash_high);
@z

@x xetex.web l.28992 - tex.ch [50.1314]: |hash_extra|
undump(hash_base)(frozen_control_sequence)(write_loc);@/
@y
undump(hash_base)(hash_top)(write_loc);@/
@z

@x xetex.web l.29039 - tex.ch [50.1316]: dump the |hash_extra| part
until k>eqtb_size
@y
until k>eqtb_size;
if hash_high>0 then for k:=eqtb_size+1 to eqtb_size+hash_high do
  dump_wd(eqtb[k]); {dump |hash_extra| part}
@z

@x xetex.web l.29051 - tex.ch [50.1308, 50.1317]: undump the |hash_extra| part
until k>eqtb_size
@y
until k>eqtb_size;
for j:=eqtb_size+1 to eqtb_top do eqtb[j]:=eqtb[undefined_control_sequence];
if hash_high>0 then for j:=eqtb_size+1 to eqtb_size+hash_high do
  undump_wd(eqtb[j]); {undump |hash_extra| part}
@z

@x xetex.web l.29060 - tex.ch [50.1318]: |hash_extra|
dump_int(hash_used); cs_count:=frozen_control_sequence-1-hash_used;
@y
dump_int(hash_used); cs_count:=frozen_control_sequence-1-hash_used+hash_high;
@z

@x xetex.web l.29064 - tex.ch [50.1318]: |hash_extra|
for p:=hash_used+1 to undefined_control_sequence-1 do dump_hh(hash[p]);
@y
for p:=hash_used+1 to undefined_control_sequence-1 do dump_hh(hash[p]);
if hash_high>0 then for p:=eqtb_size+1 to eqtb_size+hash_high do
  dump_hh(hash[p]);
@z

@x xetex.web l.29073 - tex.ch [50.1319]: |hash_extra|
for p:=hash_used+1 to undefined_control_sequence-1 do undump_hh(hash[p]);
@y
for p:=hash_used+1 to undefined_control_sequence-1 do undump_hh(hash[p]);
if hash_high>0 then for p:=eqtb_size+1 to eqtb_size+hash_high do
  undump_hh(hash[p]);
@z

@x xetex.web l.29196 - tex.ch [50.1325]: ops above 255 in a format (bigtrie)
  undump(min_quarterword)(max_quarterword)(hyf_next[k]);
@y
  undump(min_quarterword)(max_trie_op)(hyf_next[k]);
@z

@x xetex.web l.29351 - tex.ch [51.1333]: a new line before termination; switch to the editor
    slow_print(log_name); print_char(".");
    end;
  end;
end;
@y
    slow_print(log_name); print_char(".");
    end;
  end;
print_ln;
if (edit_name_start<>0) and (interaction>batch_mode) then
  call_edit(edit_name_start,edit_name_length,edit_line);
end;
@z

@x xetex.web l.29372 - tex.ch [51.1334]: |hash_extra|
  wlog_ln(' ',cs_count:1,' multiletter control sequences out of ',
    hash_size:1);@/
@y
  wlog_ln(' ',cs_count:1,' multiletter control sequences out of ',
    hash_size:1, '+', hash_extra:1);@/
@z

@x xetex.web l.34414 - new sections at the end of part 54
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
\.{xetex.ch}), sized by \.{texmf.cnf}, and gives each font's entries their
initial values when \.{INITEX} starts and when a format is loaded. Here they
are arrays of the word space (zero at the start), and the font entries
\.{xetex.ch} adds get their initial values once, before anything else is
initialized, in \.{INITEX} and in production runs alike. A C pointer of
\.{xetex.ch} (a layout engine, a \.{TECkit} mapping) is an integer handle
here, 0 for none (\.{changes/ext.ch}).

@<Allocate the arrays that web2c allocates@>=
for i := font_base to font_max do begin
    font_layout_engine[i] := 0;
    font_mapping[i] := 0;
    font_flags[i] := 0;
    font_letter_space[i] := 0;
end;

@ web2c's \.{tex.ch} limits the recursion of |expand| (its part
\.{[54/web2c]}): a counter tracks the depth, and |expand_depth| (10000,
\.{texmf.cnf}'s default) bounds it. \.{texmfmp.c} supplies the state of
\.{\\write18}; the limit and the shell switches are set up in
\.{web2c-run.ch}.

@<Glob...@>=
@!expand_depth:integer; {limits recursive calls of |expand| and |scan_expr|}
@!expand_depth_count:integer; {current depth of those calls}
@!shellenabledp:boolean; {is \.{\\write18} enabled?}
@!restrictedshell:boolean; {is it restricted to a list of programs?}

@ @<Set init...@>=
expand_depth_count:=0;

@ tex.ch [6.84] and [51.1333]: the `\.E' option of |error| remembers which
file and line to edit, and |close_files_and_terminate|, once \TeX\ has
closed its files, hands them to |call_edit| (\.{system.rs}, texmfmp.c's
|calledit|), which runs the editor command of \.{TEXEDIT} and ends the
program. |edit_name_start| is nonzero only when that is to happen.

@<Glob...@>=
@!edit_name_start: pool_pointer; {where the filename to switch to starts}
@!edit_name_length,@!edit_line: integer; {what line to start editing at}

@ @<Set init...@>=
edit_name_start:=0;

@ @<Declare web2c's file-name procedures@>=
procedure call_edit(@!s:pool_pointer;@!l,@!n:integer); external;
  {run the editor on |str_pool[s..s+l-1]| at line |n|, and stop}

@* \[55] Index.
@z
