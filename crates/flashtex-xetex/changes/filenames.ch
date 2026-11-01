% changes/filenames.ch -- file names as web2c's tex.ch and xetex.ch treat them.
%
% The XeTeX counterpart of crates/flashtex-engine/changes/filenames.ch
% (which explains tex.ch's part: `/' areas, the last `.' starts the
% extension, `\input{...}', no forced `.tex', the name found shown in the
% log, prompt_file_name's help line, string recycling, \openout logged).
% On top of tex.ch, TeX Live's xetexdir/xetex.ch makes these differences,
% re-specified here:
%
%   * names are UTF-16 in the string pool and UTF-8 in |name_of_file|
%     (|append_to_name|; |make_utf16_name| converts back, ext.ch);
%   * a name may be quoted with `"' or `'' (|file_name_quote_char|), and
%     |print_file_name| quotes with whichever the name does not contain;
%   * |end_name| does not quote a part that contains a space;
%   * \input and \openin open a Unicode file (|u_open_in|, ext.ch), then
%     re-parse the name found into |cur_area|, |cur_name| and |cur_ext|;
%   * |scan_file_name| does not stop at a space at the end of a line;
%   * |search_string| skips the 65536 single-character strings.
%
% texmf.cnf's |openin_any| (|kpse_in_name_ok|, which web2c-run.ch declares)
% is checked here, before |u_open_in|, as TeX Live's xetex does.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x ext.ch's xetex.web l.431 - web2c's file-name procedures are declared early too
@t\4@>@<Declare the routines of \XeTeX's C parts@>@/
@y
@t\4@>@<Declare the routines of \XeTeX's C parts@>@/
@t\4@>@<Declare web2c's file-name procedures@>@/
@z

@x xetex.web l.1413 - tex.ch [4.47]: the string recycling routines
@p @!init function get_strings_started:boolean; {initializes the string pool,
@y
@p @t\4@>@<Declare additional routines for string recycling@>@/

@!init function get_strings_started:boolean; {initializes the string pool,
@z

@x xetex.web l.12148 - tex.ch [29.513]: `/' ends the area, the last `.' starts the extension
@!area_delimiter:pool_pointer; {the most recent `\.>' or `\.:', if any}
@!ext_delimiter:pool_pointer; {the relevant `\..', if any}
@y
@!area_delimiter:pool_pointer; {the most recent `\./', if any}
@!ext_delimiter:pool_pointer; {the most recent `\..', if any}
@!quoted_filename:boolean; {are we inside a quoted part of a file name?}
@!stop_at_space:boolean; {does a space end a file name?}
@!full_source_filename_stack:array[0..max_in_open] of str_number;
  {the name found for each input file (tex.ch)}
@z

@x xetex.web l.12168 - tex.ch [29.515]: quoted names
begin area_delimiter:=0; ext_delimiter:=0;
@y
begin area_delimiter:=0; ext_delimiter:=0; quoted_filename:=false;
@z

@x xetex.web l.12179 - tex.ch [29.516], xetex.ch [29.516]: quotes, `/', the last `.'
begin if c=" " then more_name:=false
else  begin
  if (c>@"FFFF) then str_room(2)
  else str_room(1);
  append_char(c); {contribute |c| to the current string}
  if (c=">")or(c=":") then
    begin area_delimiter:=cur_length; ext_delimiter:=0;
    end
  else if (c=".")and(ext_delimiter=0) then ext_delimiter:=cur_length;
@y
begin if stop_at_space and (c=" ") and (file_name_quote_char=0) then
  more_name:=false
else if stop_at_space and (file_name_quote_char<>0) and (c=file_name_quote_char) then begin
  file_name_quote_char:=0;
  more_name:=true;
  end
else if stop_at_space and (file_name_quote_char=0) and ((c="""") or (c="'")) then begin
  file_name_quote_char:=c;
  quoted_filename:=true;
  more_name:=true;
  end
else  begin
  if (c>@"FFFF) then str_room(2)
  else str_room(1);
  append_char(c); {contribute |c| to the current string}
  if c="/" then {|IS_DIR_SEP|}
    begin area_delimiter:=cur_length; ext_delimiter:=0;
    end
  else if c="." then ext_delimiter:=cur_length;
@z

@x xetex.web l.12195 - tex.ch [29.517]: string recycling (xetex.ch: no quotes added)
@p procedure end_name;
begin if str_ptr+3>max_strings then
@y
@p procedure end_name;
var temp_str: str_number; {result of file name cache lookups}
@!j: pool_pointer; {running index}
begin if str_ptr+3>max_strings then
@z

@x xetex.web l.12201 - tex.ch [29.517]: string recycling
  str_start_macro(str_ptr+1):=str_start_macro(str_ptr)+area_delimiter; incr(str_ptr);
  end;
if ext_delimiter=0 then
  begin cur_ext:=""; cur_name:=make_string;
  end
else  begin cur_name:=str_ptr;
  str_start_macro(str_ptr+1):=str_start_macro(str_ptr)+ext_delimiter-area_delimiter-1;
  incr(str_ptr); cur_ext:=make_string;
  end;
@y
  str_start_macro(str_ptr+1):=str_start_macro(str_ptr)+area_delimiter; incr(str_ptr);
  temp_str:=search_string(cur_area);
  if temp_str>0 then
    begin cur_area:=temp_str;
    decr(str_ptr);  {no |flush_string|, |pool_ptr| will be wrong!}
    for j:=str_start_macro(str_ptr+1) to pool_ptr-1 do
      begin str_pool[j-area_delimiter]:=str_pool[j];
      end;
    pool_ptr:=pool_ptr-area_delimiter; {update |pool_ptr|}
    end;
  end;
if ext_delimiter=0 then
  begin cur_ext:=""; cur_name:=slow_make_string;
  end
else  begin cur_name:=str_ptr;
  str_start_macro(str_ptr+1):=str_start_macro(str_ptr)+ext_delimiter-area_delimiter-1;
  incr(str_ptr); cur_ext:=make_string;
  decr(str_ptr); {undo extension string to look at name part}
  temp_str:=search_string(cur_name);
  if temp_str>0 then
    begin cur_name:=temp_str;
    decr(str_ptr);  {no |flush_string|, |pool_ptr| will be wrong!}
    for j:=str_start_macro(str_ptr+1) to pool_ptr-1 do
      begin str_pool[j-ext_delimiter+area_delimiter+1]:=str_pool[j];
      end;
    pool_ptr:=pool_ptr-ext_delimiter+area_delimiter+1;  {update |pool_ptr|}
    end;
  cur_ext:=slow_make_string;  {remake extension string}
  end;
@z

@x xetex.web l.12219 - tex.ch [29.518], xetex.ch [29.518]: print a quoted name, with the quote it lacks
begin slow_print(a); slow_print(n); slow_print(e);
@y
var must_quote: boolean; {whether to quote the filename}
@!quote_char: integer; {current quote char (single or double)}
@!j:pool_pointer; {index into |str_pool|}
begin
must_quote:=false;
quote_char:=0;
check_quoted(a); check_quoted(n); check_quoted(e);
if must_quote then begin
  if quote_char=0 then quote_char:="""";
  print_char(quote_char);
end;
print_quoted(a); print_quoted(n); print_quoted(e);
if quote_char<>0 then print_char(quote_char);
@z

@x xetex.web l.12228 - xetex.ch [29.519]: |name_of_file| is UTF-8
@d append_to_name(#)==begin c:=#; incr(k);
  if k<=file_name_size then name_of_file[k]:=xchr[c];
  end
@y
@d append_to_name(#)==begin c:=#; incr(k);
  if k<=file_name_size then begin
      if (c < 128) then name_of_file[k]:=c
      else if (c < @"800) then begin
        name_of_file[k]:=@"C0 + c div @"40; incr(k);
        name_of_file[k]:=@"80 + c mod @"40;
      end else if (c < @"D800) then begin
        name_of_file[k]:=@"E0 + c div @"1000; incr(k);
        name_of_file[k]:=@"80 + (c mod @"1000) div @"40; incr(k);
        name_of_file[k]:=@"80 + c mod @"40;
      end else if (c < @"DC00) and (k+3<file_name_size) then begin
        name_of_file[k]:=@"F0 + (c - @"D7C0) div @"1000; incr(k);
        name_of_file[k]:=@"80 + ((c - @"D7C0) mod @"1000) div @"4; incr(k);
        name_of_file[k]:=@"80 + (c - @"D7C0) mod @"4 * @"10; incr(k);
        name_of_file[k]:=@"80;
      end else if (c < @"E000) and (k>4) then begin
        decr(k);
        name_of_file[k-1]:=name_of_file[k-1] + (c - @"DC00) div @"40;
        name_of_file[k]  :=name_of_file[k]   + (c - @"DC00) mod @"40;
      end else if (c < @"10000) then begin
        name_of_file[k]:=@"E0 + c div @"1000; incr(k);
        name_of_file[k]:=@"80 + (c mod @"1000) div @"40; incr(k);
        name_of_file[k]:=@"80 + c mod @"40;
      end else begin { replacement character U+FFFD }
        name_of_file[k]:=@"EF; incr(k);
        name_of_file[k]:=@"BF; incr(k);
        name_of_file[k]:=@"BD;
      end
    end
  end
@z

@x xetex.web l.12341 - tex.ch [29.525], xetex.ch [29.525]: |cur_name| etc.\ describe the file found
var k:0..file_name_size; {index into |name_of_file|}
begin if (pool_ptr+name_length>pool_size)or(str_ptr=max_strings)or
 (cur_length>0) then
  make_name_string:="?"
else  begin
  make_utf16_name;
  for k:=0 to name_length16-1 do append_char(name_of_file16[k]);
  make_name_string:=make_string;
  end;
@y
var k:0..file_name_size; {index into |name_of_file|}
@!save_area_delimiter, @!save_ext_delimiter: pool_pointer;
@!save_name_in_progress, @!save_stop_at_space: boolean;
begin if (pool_ptr+name_length>pool_size)or(str_ptr=max_strings)or
 (cur_length>0) then
  make_name_string:="?"
else  begin
  make_utf16_name;
  for k:=0 to name_length16-1 do append_char(name_of_file16[k]);
  make_name_string:=make_string;
  {At this point we also set |cur_name|, |cur_ext|, and |cur_area| to
   match the contents of |name_of_file|.}
  save_area_delimiter:=area_delimiter; save_ext_delimiter:=ext_delimiter;
  save_name_in_progress:=name_in_progress; save_stop_at_space:=stop_at_space;
  name_in_progress:=true;
  begin_name;
  stop_at_space:=false;
  k:=0;
  while (k<name_length16)and(more_name(name_of_file16[k])) do
    incr(k);
  stop_at_space:=save_stop_at_space;
  end_name;
  name_in_progress:=save_name_in_progress;
  area_delimiter:=save_area_delimiter; ext_delimiter:=save_ext_delimiter;
  end;
@z

@x xetex.web l.12369 - tex.ch [29.526]: a braced name (xetex.ch drops tex.ch's end-of-line test)
@p procedure scan_file_name;
label done;
begin name_in_progress:=true; begin_name;
@<Get the next non-blank non-call...@>;
loop@+begin if (cur_cmd>other_char)or(cur_chr>biggest_usv) then
    {not a character}
    begin back_input; goto done;
    end;
  if not more_name(cur_chr) then goto done;
  get_x_token;
  end;
done: end_name; name_in_progress:=false;
end;
@y
@p procedure scan_file_name;
label done;
var
  @!save_warning_index: pointer;
begin
  save_warning_index := warning_index;
  warning_index := cur_cs; {store |cur_cs| here to remember until later}
  @<Get the next non-blank non-relax non-call...@>; {here the program expands
    tokens and removes spaces and \.{\\relax}es from the input. The \.{\\relax}
    removal follows LuaTeX''s implementation, and other cases of
    balanced text scanning.}
  back_input; {return the last token to be read by either code path}
  if cur_cmd=left_brace then
    scan_file_name_braced
  else
begin name_in_progress:=true; begin_name;
@<Get the next non-blank non-call...@>;
loop@+begin if (cur_cmd>other_char)or(cur_chr>biggest_usv) then
    {not a character}
    begin back_input; goto done;
    end;
  if not more_name(cur_chr) then goto done;
  get_x_token;
  end;
  end;
done: end_name; name_in_progress:=false;
warning_index := save_warning_index; {restore |warning_index|}
end;
@z

@x xetex.web l.12428 - tex.ch [29.530]: prompt_file_name: prevent empty filenames
var k:0..buf_size; {index into |buffer|}
@y
var k:0..buf_size; {index into |buffer|}
@!saved_cur_name:str_number; {to catch empty terminal input}
@!saved_cur_ext:str_number; {to catch empty terminal input}
@!saved_cur_area:str_number; {to catch empty terminal input}
@z

@x xetex.web l.12435 - tex.ch [29.530]: prompt_file_name: the help line; no default extension is an input file
if e=".tex" then show_context;
@y
if (e=".tex") or (e="") then show_context;
print_ln; print_prompt_file_name_help_msg;
if (e<>"") then
  begin
    print("; default file extension is `"); print(e); print("'");
  end;
print(")"); print_ln;
@z

@x xetex.web l.12441 - tex.ch [29.530]: prompt_file_name: an empty reply retries the same name
clear_terminal; prompt_input(": "); @<Scan file name in the buffer@>;
if cur_ext="" then cur_ext:=e;
@y
saved_cur_name:=cur_name;
saved_cur_ext:=cur_ext;
saved_cur_area:=cur_area;
clear_terminal; prompt_input(": "); @<Scan file name in the buffer@>;
if (length(cur_name)=0) and (cur_ext="") and (cur_area="") then
  begin
    cur_name:=saved_cur_name;
    cur_ext:=saved_cur_ext;
    cur_area:=saved_cur_area;
  end
else
  if cur_ext="" then cur_ext:=e;
@z

@x xetex.web l.12543 - tex.ch [29.537], xetex.ch [29.537]: a Unicode file, no forced `.tex', the name found
@p procedure start_input; {\TeX\ will \.{\\input} something}
label done;
begin scan_file_name; {set |cur_name| to desired file name}
if cur_ext="" then cur_ext:=".tex";
pack_cur_name;
loop@+  begin begin_file_reading; {set up |cur_file| and new level of input}
  if a_open_in(cur_file) then goto done;
  if cur_area="" then
    begin pack_file_name(cur_name,TEX_area,cur_ext);
    if a_open_in(cur_file) then goto done;
    end;
  end_file_reading; {remove the level that didn't work}
  prompt_file_name("input file name",".tex");
  end;
done: name:=a_make_name_string(cur_file);
@y
@p procedure start_input; {\TeX\ will \.{\\input} something}
label done;
var temp_str: str_number;
begin scan_file_name; {set |cur_name| to desired file name}
pack_cur_name;
loop@+  begin begin_file_reading; {set up |cur_file| and new level of input}
  {(|k|, the index into |name_of_file16|, is declared in xetex.ch, after
   tracingstacklevels.ch's |v|, as TeX Live declares it)}
  set_tex_input_type(true); {Tell |open_input| we are \.{\\input}.}
  {Kpathsea tries all the various ways to get the file.}
  if kpse_in_name_ok and u_open_in(cur_file, kpse_tex_format,
     XeTeX_default_input_mode, XeTeX_default_input_encoding) then
    {At this point |name_of_file| contains the actual name found, as a UTF8 string.
     We convert to UTF16, then extract the |cur_area|, |cur_name|, and |cur_ext| from it.}
    begin
    make_utf16_name;
    name_in_progress:=true;
    begin_name;
    stop_at_space:=false;
    k:=0;
    while (k<name_length16)and(more_name(name_of_file16[k])) do
      incr(k);
    stop_at_space:=true;
    end_name;
    name_in_progress:=false;
    goto done;
    end;
  end_file_reading; {remove the level that didn't work}
  prompt_file_name("input file name","");
  end;
done: name:=a_make_name_string(cur_file);
full_source_filename_stack[in_open]:=make_full_name_string;
if name=str_ptr-1 then {we can try to conserve string pool space now}
  begin temp_str:=search_string(name);
  if temp_str>0 then
    begin name:=temp_str; flush_string;
    end;
  end;
@z

@x xetex.web l.12562 - tex.ch [29.537]: print the name found; keep it
if term_offset+length(name)>max_print_line-2 then print_ln
else if (term_offset>0)or(file_offset>0) then print_char(" ");
print_char("("); incr(open_parens); slow_print(name); update_terminal;
state:=new_line;
if name=str_ptr-1 then {conserve string pool space (but see note above)}
  begin flush_string; name:=cur_name;
  end;
@y
if term_offset+length(full_source_filename_stack[in_open])>max_print_line-2
then print_ln
else if (term_offset>0)or(file_offset>0) then print_char(" ");
print_char("("); incr(open_parens);
slow_print(full_source_filename_stack[in_open]); update_terminal;
state:=new_line;
@z

@x xetex.web l.28327 - tex.ch [49.1257]: no flushable font name (|end_name| recycles)
@!flushable_string:str_number; {string not yet referenced}
@y
@z

@x xetex.web l.28509 - tex.ch [49.1275], xetex.ch [49.1275]: \openin opens a Unicode file
var c:0..1; {1 for \.{\\openin}, 0 for \.{\\closein}}
@!n:0..15; {stream number}
@y
var c:0..1; {1 for \.{\\openin}, 0 for \.{\\closein}}
@!n:0..15; {stream number}
@!k:0..file_name_size; {index into |name_of_file16|}
@z

@x xetex.web l.28517 - tex.ch [49.1275], xetex.ch [49.1275]: \openin adds no `.tex' either; texmf.cnf's |openin_any|
  if cur_ext="" then cur_ext:=".tex";
  pack_cur_name;
  if a_open_in(read_file[n]) then read_open[n]:=just_open;
@y
  pack_cur_name;
  set_tex_input_type(false); {Tell |open_input| we are \.{\\openin}.}
  if kpse_in_name_ok and u_open_in(read_file[n], kpse_tex_format,
     XeTeX_default_input_mode, XeTeX_default_input_encoding) then
    begin
    make_utf16_name;
    name_in_progress:=true;
    begin_name;
    stop_at_space:=false;
    k:=0;
    while (k<name_length16)and(more_name(name_of_file16[k])) do
      incr(k);
    stop_at_space:=true;
    end_name;
    name_in_progress:=false;
    read_open[n]:=just_open;
    end;
@z

@x xetex.web l.30433 - tex.ch [53.1373]: a new local for logging \openout
procedure out_what(@!p:pointer);
var j:small_number; {write stream number}
@y
procedure out_what(@!p:pointer);
var j:small_number; {write stream number}
    @!old_setting:0..max_selector;
@z

@x xetex.web l.30454 - tex.ch [53.1374]: a closed stream is closed; \openout is logged
  else  begin if write_open[j] then a_close(write_file[j]);
    if subtype(p)=close_node then write_open[j]:=false
@y
  else  begin if write_open[j] then begin a_close(write_file[j]);
                                          write_open[j]:=false; end;
    if subtype(p)=close_node then do_nothing {already closed}
@z

@x xetex.web l.30461 - tex.ch [53.1374]: log each \openout (texmf.cnf's |log_openout|)
      while not a_open_out(write_file[j]) do
        prompt_file_name("output file name",".tex");
      write_open[j]:=true;
@y
      while not a_open_out(write_file[j]) do
        prompt_file_name("output file name",".tex");
      write_open[j]:=true;
      {If on first line of input, log file is not ready yet, so don't log.}
      if log_opened and texmf_yesno_log_openout then begin
        old_setting:=selector;
        if (tracing_online<=0) then
          selector:=log_only  {Show what we're doing in the log file.}
        else selector:=term_and_log;  {Show what we're doing.}
        print_nl("\openout");
        print_int(j);
        print(" = `");
        print_file_name(cur_name,cur_area,cur_ext);
        print("'."); print_nl(""); print_ln;
        selector:=old_setting;
      end;
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ The macros of tex.ch's |print_file_name|, as xetex.ch makes them.

@d check_quoted(#) == {check if string |#| needs quoting}
if #<>0 then begin
  j:=str_start_macro(#);
  while ((not must_quote) or (quote_char=0)) and (j<str_start_macro(#+1)) do begin
    if str_pool[j]=" " then must_quote:=true
    else if (str_pool[j]="""") or (str_pool[j]="'") then begin
      must_quote:=true;
      quote_char:="""" + "'" - str_pool[j];
    end;
    incr(j);
  end;
end
@#
@d print_quoted(#) == {print string |#|, omitting quotes}
if #<>0 then
  for j:=str_start_macro(#) to str_start_macro(#+1)-1 do begin
    if str_pool[j]=quote_char then begin
      print(quote_char);
      quote_char:="""" + "'" - quote_char;
      print(quote_char);
    end;
    if (so(str_pool[j])>=@"D800) and (so(str_pool[j])<=@"DBFF)
      and (j+1<str_start_macro(#+1))
      and (so(str_pool[j+1])>=@"DC00) and (so(str_pool[j+1])<=@"DFFF) then
      begin print_char(@"10000 + (so(str_pool[j])-@"D800) * @"400
                     + so(str_pool[j+1])-@"DC00);
      incr(j);
      end
    else
      print(str_pool[j]);
  end

@ @<Set init...@>=
stop_at_space:=true;

@ tex.ch's |scan_file_name_braced| (its part \.{[54/web2c]}): when
|scan_file_name| finds a |left_brace|, the file name is a balanced token
list, expanded as it is read, converted into a string and fed to |more_name|
character by character, with spaces allowed.

@<Declare web2c's file-name procedures@>=
function texmf_yesno_log_openout:boolean; external;
  {texmf.cnf's |log_openout| (system.rs)}
@#
procedure print_prompt_file_name_help_msg; external;
  {tex.ch [29.530]'s |print_c_string(prompt_file_name_help_msg)|: cpascal.h's
   C string, printed character by character, not a pool string (system.rs)}
@#
function make_full_name_string:str_number; external;
  {texmfmp.c's |makefullnamestring|: the full name of the file opened last,
   before \.{./} is taken off |name_of_file| (system.rs)}
@#
procedure scan_file_name_braced;
var
  @!save_scanner_status: small_number; {|scanner_status| upon entry}
  @!save_def_ref: pointer; {|def_ref| upon entry, important if inside `\.{\\message}}
  @!save_cur_cs: pointer;
  @!s: str_number; {temp string}
  @!p: pointer; {temp pointer}
  @!i: integer; {loop tally}
  @!save_stop_at_space: boolean; {this should be in tex.ch}
  @!dummy: boolean;
    {Initializing}
begin save_scanner_status := scanner_status; {|scan_toks| sets |scanner_status| to |absorbing|}
  save_def_ref := def_ref; {|scan_toks| uses |def_ref| to point to the token list just read}
  save_cur_cs := cur_cs; {we set |cur_cs| back a few tokens to use in runaway errors}
    {Scanning a token list}
  cur_cs := warning_index; {for possible runaway error}
  {mimick |call_func| from pdfTeX}
  if scan_toks(false, true) <> 0 then do_nothing; {actually do the scanning}
  {|s := tokens_to_string(def_ref);|}
  old_setting := selector; selector:=new_string;
  show_token_list(link(def_ref),null,pool_size-pool_ptr);
  selector := old_setting;
  s := make_string;
  {turns the token list read in a string to input}
    {Restoring some variables}
  delete_token_ref(def_ref); {remove the token list from memory}
  def_ref := save_def_ref; {and restore |def_ref|}
  cur_cs := save_cur_cs; {restore |cur_cs|}
  scanner_status := save_scanner_status; {restore |scanner_status|}
    {Passing the read string to the input machinery}
  save_stop_at_space := stop_at_space; {save |stop_at_space|}
  stop_at_space := false; {set |stop_at_space| to false to allow spaces in file names}
  begin_name;
  for i:=str_start_macro(s) to str_start_macro(s+1)-1 do
    dummy := more_name(str_pool[i]); {add each read character to the current file name}
  stop_at_space := save_stop_at_space; {restore |stop_at_space|}
end;

@ tex.ch's string recycling routines (its part \.{[54/web2c-string]}).
\TeX{} uses 2 upto 4 {\it new\/} strings when scanning a filename in an
\.{\\input}, \.{\\openin}, or \.{\\openout} operation.  These strings are
normally lost because the reference to them are not saved after finishing
the operation.  |search_string| searches through the string pool for the
given string and returns either 0 or the found string number.

@<Declare additional routines for string recycling@>=
function search_string(@!search:str_number):str_number;
label found;
var result: str_number;
@!s: str_number; {running index}
@!len: integer; {length of searched string}
begin result:=0; len:=length(search);
if len=0 then  {trivial case}
  begin result:=""; goto found;
  end
else  begin s:=search-1;  {start search with newest string below |s|; |search>1|!}
  while s>65535 do  {first 64K strings don't really exist in the pool!}
    begin if length(s)=len then
      if str_eq_str(s,search) then
        begin result:=s; goto found;
        end;
    decr(s);
    end;
  end;
found:search_string:=result;
end;

@ The following routine is a variant of |make_string|.  It searches
the whole string pool for a string equal to the string currently built
and returns a found string.  Otherwise a new string is created and
returned.  Be cautious, you can not apply |flush_string| to a replaced
string!

@<Declare additional routines for string recycling@>=
function slow_make_string : str_number;
label exit;
var s: str_number; {result of |search_string|}
@!t: str_number; {new string}
begin t:=make_string; s:=search_string(t);
if s>0 then
  begin flush_string; slow_make_string:=s; return;
  end;
slow_make_string:=t;
exit:end;

@* \[55] Index.
@z
