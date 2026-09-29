% changes/filenames.ch -- file names as web2c's tex.ch treats them.
%
% tex.web's file names are those of Knuth's system: the area ends at `>' or
% `:', the extension starts at the first `.', a space ends the name, and
% `\input' and `\openin' add `.tex'. TeX Live's pdfTeX follows web2c's
% tex.ch part [29] instead, and LaTeX depends on it (it writes
% `\openin\@inputcheck"expl3.ltx"'). Re-specified here, per DESIGN.md
% section 4.1:
%
%   * the area ends at the last `/', the extension starts at the last `.';
%   * `"' quotes a name (the quotes are not part of it), inside which a
%     space does not end it; a name with a space is printed quoted;
%   * `\input{...}' and `\openin{...}' take a braced, expanded name
%     (scan_file_name_braced, tex.ch part [54/web2c]);
%   * at the end of a line a space ends the name, so none is added to it;
%   * `\input' and `\openin' do not add `.tex': the file resolver
%     (kpathsea) tries `name.tex' and then `name', and the name actually
%     found is what the log shows (system.rs writes it back into
%     |name_of_file|, and |make_name_string| re-parses it into |cur_area|,
%     |cur_name| and |cur_ext|, as tex.ch does);
%   * there is no `TeXinputs:' area to try second.
%
% Not re-specified: tex.ch's recycling of equal strings (search_string),
% which only saves pool space, and -output-directory.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x ext.ch's l.397 - web2c's file-name procedures are declared early too
@t\4@>@<Declare the routines of pdf\TeX's C parts@>@/
@y
@t\4@>@<Declare the routines of pdf\TeX's C parts@>@/
@t\4@>@<Declare web2c's file-name procedures@>@/
@z

@x pdftex.web l.12173 - tex.ch [29.513]: `/' ends the area, the last `.' starts the extension
@!area_delimiter:pool_pointer; {the most recent `\.>' or `\.:', if any}
@!ext_delimiter:pool_pointer; {the relevant `\..', if any}
@y
@!area_delimiter:pool_pointer; {the most recent `\./', if any}
@!ext_delimiter:pool_pointer; {the most recent `\..', if any}
@!quoted_filename:boolean; {are we inside a quoted part of a file name?}
@!stop_at_space:boolean; {does a space end a file name?}
@!full_source_filename_stack:array[1..max_in_open] of str_number;
  {the name found for each input file (tex.ch)}
@z

@x pdftex.web l.12192 - tex.ch [29.515]: quoted names
begin area_delimiter:=0; ext_delimiter:=0;
@y
begin area_delimiter:=0; ext_delimiter:=0; quoted_filename:=false;
@z

@x pdftex.web l.12202 - tex.ch [29.516]: quotes, `/', the last `.'
begin if c=" " then more_name:=false
else  begin str_room(1); append_char(c); {contribute |c| to the current string}
  if (c=">")or(c=":") then
    begin area_delimiter:=cur_length; ext_delimiter:=0;
    end
  else if (c=".")and(ext_delimiter=0) then ext_delimiter:=cur_length;
@y
begin if (c=" ") and stop_at_space and (not quoted_filename) then
  more_name:=false
else  if c="""" then begin
  quoted_filename:=not quoted_filename;
  more_name:=true;
  end
else  begin str_room(1); append_char(c); {contribute |c| to the current string}
  if c="/" then
    begin area_delimiter:=cur_length; ext_delimiter:=0;
    end
  else if c="." then ext_delimiter:=cur_length;
@z

@x pdftex.web l.12215 - tex.ch [29.517]: a part with a space is quoted again
@p procedure end_name;
begin if str_ptr+3>max_strings then
  overflow("number of strings",max_strings-init_str_ptr);
@:TeX capacity exceeded number of strings}{\quad number of strings@>
@y
@p procedure end_name;
var @!j,@!s,@!t: pool_pointer; {running indices}
@!must_quote:boolean; {whether we need to quote a string}
begin if str_ptr+3>max_strings then
  overflow("number of strings",max_strings-init_str_ptr);
@:TeX capacity exceeded number of strings}{\quad number of strings@>
str_room(6); {Room for quotes, if needed.}
{add quotes if needed}
if area_delimiter<>0 then begin
  {maybe quote |cur_area|}
  must_quote:=false;
  s:=str_start[str_ptr];
  t:=str_start[str_ptr]+area_delimiter;
  j:=s;
  while (not must_quote) and (j<t) do begin
    must_quote:=str_pool[j]=" "; incr(j);
    end;
  if must_quote then begin
    for j:=pool_ptr-1 downto t do str_pool[j+2]:=str_pool[j];
    str_pool[t+1]:="""";
    for j:=t-1 downto s do str_pool[j+1]:=str_pool[j];
    str_pool[s]:="""";
    if ext_delimiter<>0 then ext_delimiter:=ext_delimiter+2;
    area_delimiter:=area_delimiter+2;
    pool_ptr:=pool_ptr+2;
    end;
  end;
{maybe quote |cur_name|}
s:=str_start[str_ptr]+area_delimiter;
if ext_delimiter=0 then t:=pool_ptr else t:=str_start[str_ptr]+ext_delimiter-1;
must_quote:=false;
j:=s;
while (not must_quote) and (j<t) do begin
  must_quote:=str_pool[j]=" "; incr(j);
  end;
if must_quote then begin
  for j:=pool_ptr-1 downto t do str_pool[j+2]:=str_pool[j];
  str_pool[t+1]:="""";
  for j:=t-1 downto s do str_pool[j+1]:=str_pool[j];
  str_pool[s]:="""";
  if ext_delimiter<>0 then ext_delimiter:=ext_delimiter+2;
  pool_ptr:=pool_ptr+2;
  end;
if ext_delimiter<>0 then begin
  {maybe quote |cur_ext|}
  s:=str_start[str_ptr]+ext_delimiter-1;
  t:=pool_ptr;
  must_quote:=false;
  j:=s;
  while (not must_quote) and (j<t) do begin
    must_quote:=str_pool[j]=" "; incr(j);
    end;
  if must_quote then begin
    str_pool[t+1]:="""";
    for j:=t-1 downto s do str_pool[j+1]:=str_pool[j];
    str_pool[s]:="""";
    pool_ptr:=pool_ptr+2;
    end;
  end;
@z

@x pdftex.web l.12239 - tex.ch [29.518]: print a name with a space quoted, without its quotes
begin slow_print(a); slow_print(n); slow_print(e);
@y
var must_quote: boolean; {whether to quote the filename}
@!j:pool_pointer; {index into |str_pool|}
begin
must_quote:=false;
check_quoted(a); check_quoted(n); check_quoted(e);
if must_quote then print_char("""");
print_quoted(a); print_quoted(n); print_quoted(e);
if must_quote then print_char("""");
@z

@x pdftex.web l.12248 - tex.ch [29.519]: the quotes are not part of the name
@d append_to_name(#)==begin c:=#; incr(k);
  if k<=file_name_size then name_of_file[k]:=xchr[c];
  end
@y
@d append_to_name(#)==begin c:=#; if not (c="""") then begin incr(k);
  if k<=file_name_size then name_of_file[k]:=xchr[c];
  end end
@z

@x pdftex.web l.12362 - tex.ch [29.525]: |cur_name| etc.\ describe the file found
begin if (pool_ptr+name_length>pool_size)or(str_ptr=max_strings)or
 (cur_length>0) then
  make_name_string:="?"
else  begin for k:=1 to name_length do append_char(xord[name_of_file[k]]);
  make_name_string:=make_string;
  end;
@y
@!save_area_delimiter, @!save_ext_delimiter: pool_pointer;
@!save_name_in_progress, @!save_stop_at_space: boolean;
begin if (pool_ptr+name_length>pool_size)or(str_ptr=max_strings)or
 (cur_length>0) then
  make_name_string:="?"
else  begin for k:=1 to name_length do append_char(xord[name_of_file[k]]);
  make_name_string:=make_string;
  {At this point we also set |cur_name|, |cur_ext|, and |cur_area| to
   match the contents of |name_of_file|.}
  save_area_delimiter:=area_delimiter; save_ext_delimiter:=ext_delimiter;
  save_name_in_progress:=name_in_progress; save_stop_at_space:=stop_at_space;
  name_in_progress:=true;
  begin_name;
  stop_at_space:=false;
  k:=1;
  while (k<=name_length)and(more_name(name_of_file[k])) do
    incr(k);
  stop_at_space:=save_stop_at_space;
  end_name;
  name_in_progress:=save_name_in_progress;
  area_delimiter:=save_area_delimiter; ext_delimiter:=save_ext_delimiter;
  end;
@z

@x pdftex.web l.12384 - tex.ch [29.526]: a braced name; a space at the end of a line ends a name
@p procedure scan_file_name;
label done;
begin name_in_progress:=true; begin_name;
@<Get the next non-blank non-call...@>;
loop@+begin if (cur_cmd>other_char)or(cur_chr>255) then {not a character}
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
loop@+begin if (cur_cmd>other_char)or(cur_chr>255) then {not a character}
    begin back_input; goto done;
    end;
  {If |cur_chr| is a space and we're not scanning a token list, check
   whether we're at the end of the buffer. Otherwise we end up adding
   spurious spaces to file names in some cases.}
  if (cur_chr=" ") and (state<>token_list) and (loc>limit) then goto done;
  if not more_name(cur_chr) then goto done;
  get_x_token;
  end;
  end;
done: end_name; name_in_progress:=false;
warning_index := save_warning_index; {restore |warning_index|}
end;
@z

@x pdftex.web l.12552 - tex.ch [29.537]: no forced `.tex', no second area, the name found
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
  if a_open_in(cur_file) then goto done;
  end_file_reading; {remove the level that didn't work}
  prompt_file_name("input file name","");
  end;
done: name:=a_make_name_string(cur_file);
full_source_filename_stack[in_open]:=name;
@z

@x pdftex.web l.12570 - tex.ch [29.537]: print the name found; keep it
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

@x pdftex.web l.32525 - tex.ch [49.1275]: \openin adds no `.tex' either
  if cur_ext="" then cur_ext:=".tex";
  pack_cur_name;
  if a_open_in(read_file[n]) then read_open[n]:=just_open;
@y
  pack_cur_name;
  if a_open_in(read_file[n]) then read_open[n]:=just_open;
@z

@x pdftex.web l.36149 - tex.ch [53.1373]: a new local for logging \openout
procedure out_what(@!p:pointer);
var j:small_number; {write stream number}
@y
procedure out_what(@!p:pointer);
var j:small_number; {write stream number}
    @!old_setting:0..max_selector;
@z

@x pdftex.web l.36184 - tex.ch [53.1374]: a closed stream is closed; \openout is logged
  else  begin if write_open[j] then a_close(write_file[j]);
    if subtype(p)=close_node then write_open[j]:=false
@y
  else  begin if write_open[j] then begin a_close(write_file[j]);
                                          write_open[j]:=false; end;
    if subtype(p)=close_node then do_nothing {already closed}
@z

@x pdftex.web l.36191 - tex.ch [53.1374]: log each \openout (texmf.cnf's |log_openout|)
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

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ The macros of tex.ch's |print_file_name|.

@d check_quoted(#) == {check if string |#| needs quoting}
if #<>0 then begin
  j:=str_start[#];
  while (not must_quote) and (j<str_start[#+1]) do begin
    must_quote:=str_pool[j]=" "; incr(j);
  end;
end
@#
@d print_quoted(#) == {print string |#|, omitting quotes}
if #<>0 then
  for j:=str_start[#] to str_start[#+1]-1 do
    if so(str_pool[j])<>"""" then
      print(so(str_pool[j]))

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
  for i:=str_start[s] to str_start[s+1]-1 do
    dummy := more_name(str_pool[i]); {add each read character to the current file name}
  stop_at_space := save_stop_at_space; {restore |stop_at_space|}
end;

@* \[55] Index.
@z
