% changes/xetex-web2c.ch -- TeX Live's xetexdir/xetex.ch, re-specified.
%
% TeX Live builds XeTeX from xetex.web, tex.ch and xetexdir/xetex.ch
% (xetexdir/am/xetex.am). xetex.ch is written against tex.ch's text, which
% this engine re-specifies in its own change files (web2c.ch, filenames.ch,
% virtex.ch, web2c-hooks.ch, web2c-run.ch), so the parts of xetex.ch that
% change tex.ch's text are folded into those files where they belong, each
% change naming its origin. This file holds the rest: what xetex.ch and
% TeX Live's tex-binpool.ch change in xetex.web itself.
%
%   * the banner is XeTeX's (xetex.ch [1]);
%   * the terminal and the input files are Unicode files (xetex.ch [3.32],
%     [22.304]);
%   * the string pool is loaded from the pool the program carries
%     (tex-binpool.ch, texmfmp.c's |loadpoolstrings|);
%   * |name_of_file| ends with a 0, as xetex.ch's C string does (the font
%     warnings of xetex.web scan it for the 0), and |TEX_format_default|
%     is used without |xord|;
%   * a fifth |history| value, |output_failure| (xetex.ch [6.76]);
%   * |new_character| makes a native character in a native font
%     (xetex.ch [30.582]);
%   * |hlist_out| has xetex.web's two extra labels and |vlist_out| an
%     upwards empty box (xetex.ch [32.619], [32.629]);
%   * the output file is closed with |dvi_close|, which reports an error
%     (xetex.ch [32.645]);
%   * tex.ch's bigtrie sizes the per-language arrays by |biggest_lang|
%     and an empty trie by |max_hyph_char| (xetex.ch [43.943], [43.946],
%     [43.958]);
%   * a format holds the starts of the pool strings only, not of the
%     65536 single-character ones (xetex.ch [50.1309], [50.1310]);
%   * a format with native fonts or font mappings is refused
%     (xetex.ch [50.1322]);
%   * |native_font_type_flag| and |xtx_ligature_present| (xetex.ch's part
%     [54/web2c]);
%   * the code that follows the main program in xetex.web comes before it,
%     as tex.ch's main procedure allows.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.328 - xetex.ch [1]: the banner is XeTeX's
@d banner=='This is TeX, Version 3.141592653' {printed when \TeX\ starts}
@y
@d banner==XeTeX_banner
@z

@x xetex.web l.904 - xetex.ch [3.26]: |name_of_file16| starts at 0 (a C array)
@!name_of_file16:array[1..file_name_size] of UTF16_code;@;@/
@y
@!name_of_file16:array[0..file_name_size] of UTF16_code;@;@/
@z

@x xetex.web l.1078 - xetex.ch [3.32]: the terminal is a Unicode file
@!term_in:alpha_file; {the terminal as an input file}
@y
@!term_in:unicode_file; {the terminal as an input file}
@z

@x xetex.web l.1417 - tex-binpool.ch: the pool is part of the program
@!m,@!n:text_char; {characters input from |pool_file|}
@!g:str_number; {garbage}
@!a:integer; {accumulator for check sum}
@!c:boolean; {check sum has been checked}
@y
@!g:str_number; {garbage}
@z

@x xetex.web l.1476 - tex-binpool.ch: the pool is part of the program
@ @d bad_pool(#)==begin wake_up_terminal; write_ln(term_out,#);
  a_close(pool_file); get_strings_started:=false; return;
  end
@<Read the other strings...@>=
name_of_file:=pool_name; {we needn't set |name_length|}
if a_open_in(pool_file) then
  begin c:=false;
  repeat @<Read one string, but return |false| if the
    string memory space is getting too tight for comfort@>;
  until c;
  a_close(pool_file); get_strings_started:=true;
  end
else  bad_pool('! I can''t read TEX.POOL.')
@.I can't read TEX.POOL@>

@ @<Read one string...@>=
begin if eof(pool_file) then bad_pool('! TEX.POOL has no check sum.');
@.TEX.POOL has no check sum@>
read(pool_file,m,n); {read two digits of string length}
if m='*' then @<Check the pool check sum@>
else  begin if (xord[m]<"0")or(xord[m]>"9")or@|
      (xord[n]<"0")or(xord[n]>"9") then
    bad_pool('! TEX.POOL line doesn''t begin with two digits.');
@.TEX.POOL line doesn't...@>
  l:=xord[m]*10+xord[n]-"0"*11; {compute the length}
  if pool_ptr+l+string_vacancies>pool_size then
    bad_pool('! You have to increase POOLSIZE.');
@.You have to increase POOLSIZE@>
  for k:=1 to l do
    begin if eoln(pool_file) then m:=' '@+else read(pool_file,m);
    append_char(xord[m]);
    end;
  read_ln(pool_file); g:=make_string;
  end;
end

@ The \.{WEB} operation \.{@@\$} denotes the value that should be at the
end of this \.{TEX.POOL} file; any other value means that the wrong pool
file has been loaded.
@^check sum@>

@<Check the pool check sum@>=
begin a:=0; k:=1;
loop@+  begin if (xord[n]<"0")or(xord[n]>"9") then
  bad_pool('! TEX.POOL check sum doesn''t have nine digits.');
@.TEX.POOL check sum...@>
  a:=10*a+xord[n]-"0";
  if k=9 then goto done;
  incr(k); read(pool_file,n);
  end;
done: if a<>@$ then bad_pool('! TEX.POOL doesn''t match; TANGLE me again.');
@.TEX.POOL doesn't match@>
c:=true;
end
@y
@ @<Read the other strings...@>=
  g := load_pool_strings((pool_size-string_vacancies));
  if g=0 then begin
     wake_up_terminal; write_ln(term_out,'! You have to increase POOLSIZE.');
     get_strings_started:=false;
     return;
  end;
  get_strings_started:=true;

@ Empty module

@ Empty module
@z

@x xetex.web l.2025 - xetex.ch [6.76]: a |history| value for failure of the output driver
has been detected. It has four possible values: |spotless|, |warning_issued|,
|error_message_issued|, and |fatal_error_stop|.
@y
has been detected. It has five possible values: |spotless|, |warning_issued|,
|error_message_issued|, |fatal_error_stop|, and |output_failure|.
@z

@x xetex.web l.2036 - xetex.ch [6.76]
@d fatal_error_stop=3 {|history| value when termination was premature}
@y
@d fatal_error_stop=3 {|history| value when termination was premature}
@d output_failure=4 {|history| value when output driver returned an error}
@z

@x xetex.web l.2041 - xetex.ch [6.76]
@!history:spotless..fatal_error_stop; {has the source input been clean so far?}
@y
@!history:spotless..output_failure; {has the source input been clean so far?}
@z

@x xetex.web l.3100 - a format file is a file of memory words (TeX Live's is a |gzFile|; this engine's is not compressed)
@!word_file = gzFile;
@y
@!word_file = file of memory_word;
@z

@x xetex.web l.7748 - xetex.ch [22.304]: the input files are Unicode files
@!input_file : array[1..max_in_open] of alpha_file;
@y
@!input_file : array[0..max_in_open] of unicode_file;
  {(0: \.{\\XeTeXinputencoding} on the terminal level; texmfmp.c makes it
   |term_in|, which phase S0 does not)}
@z

@x xetex.web l.12241 - xetex.ch [29.519]: |name_of_file| ends with a 0
for k:=name_length+1 to file_name_size do name_of_file[k]:=' ';
end;
@y
if name_length<file_name_size then name_of_file[name_length+1]:=0;
end;
@z

@x xetex.web l.12285 - xetex.ch [29.523]: no |xord|
for j:=1 to n do append_to_name(xord[TEX_format_default[j]]);
@y
for j:=1 to n do append_to_name(TEX_format_default[j]);
@z

@x xetex.web l.12288 - xetex.ch [29.523]: no |xord|
  append_to_name(xord[TEX_format_default[j]]);
@y
  append_to_name(TEX_format_default[j]);
@z

@x xetex.web l.12290 - xetex.ch [29.523]: |name_of_file| ends with a 0
for k:=name_length+1 to file_name_size do name_of_file[k]:=' ';
@y
if name_length<file_name_size then name_of_file[name_length+1]:=0;
@z

@x xetex.web l.12545 - xetex.ch [29.537]: the index into |name_of_file16| (after tracingstacklevels.ch's |v|)
var temp_str: str_number;
v: pointer;
begin scan_file_name; {set |cur_name| to desired file name}
@y
var temp_str: str_number;
v: pointer;
@!k:0..file_name_size; {index into |name_of_file16|}
begin scan_file_name; {set |cur_name| to desired file name}
@z

@x xetex.web l.13558 - xetex.ch [30.582]: a native font makes a native character
@p function new_character(@!f:internal_font_number;@!c:eight_bits):pointer;
label exit;
var p:pointer; {newly allocated node}
begin if font_bc[f]<=c then if font_ec[f]>=c then
@y
@p function new_character(@!f:internal_font_number;@!c:ASCII_code):pointer;
label exit;
var p:pointer; {newly allocated node}
begin
if is_native_font(f) then
  begin new_character:=new_native_character(f,c); return;
  end;
if font_bc[f]<=c then if font_ec[f]>=c then
@z

@x xetex.web l.14655 - xetex.ch [32.619]: the labels xetex.web's |hlist_out| uses
label reswitch, move_past, fin_rule, next_p;
@y
label reswitch, move_past, fin_rule, next_p, check_next, end_node_run;
@z

@x xetex.web l.15082 - xetex.ch [32.629]: an empty box upwards
if list_ptr(p)=null then cur_v:=cur_v+height(p)+depth(p)
@y
if list_ptr(p)=null then begin
    if upwards then cur_v:=cur_v-depth(p)-height(p) else cur_v:=cur_v+height(p)+depth(p);
  end
@z

@x xetex.web l.15319 - xetex.ch [32.645]: |dvi_close| may report an error
  print_nl("Output written on "); slow_print(output_file_name);
@.Output written on x@>
  print(" ("); print_int(total_pages); print(" page");
  if total_pages<>1 then print_char("s");
  print(", "); print_int(dvi_offset+dvi_ptr); print(" bytes).");
  b_close(dvi_file);
@y
  k:=dvi_close(dvi_file);
  if k=0 then begin
    print_nl("Output written on "); print(output_file_name);
@.Output written on x@>
    print(" ("); print_int(total_pages);
    if total_pages<>1 then print(" pages")
    else print(" page");
    if no_pdf_output then begin
      print(", "); print_int(dvi_offset+dvi_ptr); print(" bytes).");
    end else print(").");
  end else begin
    print_nl("Error "); print_int(k); print(" (");
    if no_pdf_output then print_strerror(k)
    else print("driver return code");
    print(") generating output;");
    print_nl("file "); print(output_file_name); print(" may not be valid.");
    history:=output_failure;
    end;
@z

@x xetex.web l.22530 - xetex.ch [43.943]: bigtrie
@!trie_used:array[ASCII_code] of quarterword;
@y
@!trie_used:array[0..biggest_lang] of quarterword;
@z

@x xetex.web l.22598 - xetex.ch [43.946]: bigtrie
for k:=0 to 255 do trie_used[k]:=min_quarterword;
@y
for k:=0 to biggest_lang do trie_used[k]:=min_quarterword;
@z

@x xetex.web l.22822 - xetex.ch [43.958]: bigtrie: the trie's first |max_hyph_char| entries
  begin for r:=0 to 256 do trie[r]:=h;
  trie_max:=256;
@y
  begin for r:=0 to max_hyph_char do trie[r]:=h;
  trie_max:=max_hyph_char;
@z

@x xetex.web l.27724 - xetex.ch [49.1222]: a shorthand definition starts as |relax| with |too_big_usv|, as \.{\\csname} makes one
shorthand_def: begin n:=cur_chr; get_r_token; p:=cur_cs; define(p,relax,256);
@y
shorthand_def: begin n:=cur_chr; get_r_token; p:=cur_cs; define(p,relax,too_big_usv);
@z

@x xetex.web l.28902 - xetex.ch [50.1309]: only the pool strings' starts are dumped
for k:=0 to str_ptr do dump_int(str_start[k]);
@y
for k:=too_big_char to str_ptr do dump_int(str_start_macro(k));
@z

@x xetex.web l.28919 - xetex.ch [50.1310]: only the pool strings' starts are undumped
for k:=0 to str_ptr do undump(0)(pool_ptr)(str_start[k]);
@y
for k:=too_big_char to str_ptr do undump(0)(pool_ptr)(str_start_macro(k));
@z

@x xetex.web l.29118 - xetex.ch [50.1322]: no native fonts or font mappings in a format
print_file_name(font_name[k],font_area[k],"");
@y
if is_native_font(k) or (font_mapping[k]<>0) then
  begin print_file_name(font_name[k],"","");
  print_err("Can't \dump a format with native fonts or font-mappings");
  help3("You really, really don't want to do this.")@/
  ("It won't work, and only confuses me.")@/
  ("(Load them at runtime, not as part of the format file.)");
  error;
  end
else print_file_name(font_name[k],font_area[k],"");
@z

@x xetex.web l.29811 - the code after the main program comes before it (tex.ch makes the main program a procedure)
@p procedure flush_str(s: str_number); {flush a string if possible}
@y
@<Declare procedures that need to be declared forward for \pdfTeX@>=
procedure flush_str(s: str_number); {flush a string if possible}
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ \.{xetex.ch}'s variables of its part \.{[54/web2c]}.

@<Glob...@>=
@!native_font_type_flag:integer; {used by XeTeX font loading code to record which font technology was used}
@!xtx_ligature_present:boolean; {to suppress tfm font mapping of char codes from ligature nodes (already mapped)}

@* \[55] Index.
@z
