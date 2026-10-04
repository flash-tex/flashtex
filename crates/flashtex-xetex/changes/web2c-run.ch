% changes/web2c-run.ch -- how a run is set up and reports itself: web2c's
% command-line options and texmf.cnf settings, as tex.ch, xetex.ch and
% texmfmp.c implement them for TeX Live's XeTeX.
%
% The XeTeX counterpart of crates/flashtex-engine/changes/web2c-run.ch.
% XeTeX differs from pdfTeX here in that it ignores a TCX file (xetex.ch
% [5.61], [29.536]: "WARNING: translate-file ... ignored"), so a format
% carries no |xord|, |xchr| or |xprn| (xetex.ch [50.1307]); its error
% context buffer holds Unicode scalars (xetex.ch [5.54]); \write18's
% command is the string's UTF-16 made UTF-8 (xetex.ch [53.1370]); and there
% is no pdfTeX banner or `-output-format'.
%
% texmfmp.c reads the command line and texmf.cnf before the WEB program
% starts; the program sees the result in a few global variables. Here the
% driver (src/main.rs) and src/system.rs (|configure|) do texmfmp.c's part,
% and this file re-specifies tex.ch's part, per DESIGN.md section 4.1:
%
%   * |error_line|, |half_error_line| and |max_print_line| are texmf.cnf
%     values (tex.ch [1.11], [51.1332] |setup_bound_var|), not constants,
%     so the environment or `-cnf-line' can raise them; so is |expand_depth|;
%   * `-interaction' (tex.ch [6.73], [6.74]);
%   * `-file-line-error' messages (tex.ch [6.73], |print_file_line|);
%   * `-halt-on-error' (tex.ch [6.82]);
%   * web2c's version string and the status lines after the banner, on the
%     terminal and in the log (tex.ch [5.61], [29.536]): \.{\\write18},
%     file:line:error messages, %&-line parsing and the TCX file; and the
%     complete banner |pdftex_banner| (\.{\\pdftexbanner}, the PDF's
%     \.{/PTEX.Fullbanner}), made where pdftex.ch makes it;
%   * a `%&format' first line (tex.ch [51.1337]);
%   * `-jobname' and the recorder's file name (tex.ch [29.534], [29.537]);
%   * \.{\\write18} and \.{\\eof18} (tex.ch [53.1350], [53.1370], [28.501]),
%     with the shell escape itself in system.rs (texmfmp.c's |runsystem|);
%   * texmf.cnf's |openin_any| and |openout_any| (tex.ch [29.537],
%     [49.1275], [53.1374]: |kpse_in_name_ok|, |kpse_out_name_ok|);
%   * the TCX file's |xord|, |xchr| and |xprn|, which a format carries
%     (tex.ch [2.24], [50.1307]);
%   * `-output-format' and `-draftmode' (texmfmp.c, pdftex.web's
%     |pdf_output_option|);
%   * tex.ch's fixes for fatal errors on the terminal ([5.71], [27.484],
%     [23.331]) and the file-name stacks they keep ([23.328], [23.331]).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.565 - tex.ch [1.11]: three texmf.cnf values are variables
@!error_line=72; {width of context lines on terminal error messages}
@!half_error_line=42; {width of first lines of contexts in terminal
  error messages; should be between 30 and |error_line-15|}
@!max_print_line=79; {width of longest text lines output; should be at least 60}
@y
@z

@x xetex.web l.1584 - tex.ch [5.54]: the bounds are variables (C's |integer| in web2c)
@!term_offset : 0..max_print_line;
  {the number of characters on the current terminal line}
@!file_offset : 0..max_print_line;
  {the number of characters on the current file line}
@!trick_buf:array[0..error_line] of ASCII_code; {circular buffer for
@y
@!term_offset : integer;
  {the number of characters on the current terminal line}
@!file_offset : integer;
  {the number of characters on the current file line}
@!trick_buf:array[0..ssup_error_line] of UnicodeScalar; {circular buffer for
@z

@x virtex.ch's xetex.web l.1808 - tex.ch [5.61]: web2c's version, the status lines after the banner
wterm(banner);
if format_ident=0 then
  begin wterm(' (preloaded format='); wterm_dump_name; wterm_ln(')');
  end
else  begin slow_print(format_ident); print_ln;
  end;
@y
wterm(banner);
wterm_version_string;
if format_ident=0 then
  begin wterm(' (preloaded format='); wterm_dump_name; wterm_ln(')');
  end
else  begin slow_print(format_ident); print_ln;
  end;
if shellenabledp then begin
  wterm(' ');
  if restrictedshell then begin
    wterm('restricted ');
  end;
  wterm_ln('\write18 enabled.');
end;
if translate_filename_p then begin
  wterm(' (WARNING: translate-file "');
  wterm_translate_filename;
  wterm_ln('" ignored)');
end;
@z

@x xetex.web l.1947 - tex.ch [5.71]: set |limit| when |fatal_error|
if not input_ln(term_in,true) then fatal_error("End of file on the terminal!");
@y
if not input_ln(term_in,true) then begin
  limit:=0; fatal_error("End of file on the terminal!"); end;
@z

@x xetex.web l.1984 - tex.ch [6.73]: |unspecified_mode|; file:line:error messages
@d error_stop_mode=3 {stops at every opportunity to interact}
@d print_err(#)==begin if interaction=error_stop_mode then wake_up_terminal;
  print_nl("! "); print(#);
  end
@y
@d error_stop_mode=3 {stops at every opportunity to interact}
@d unspecified_mode=4 {extra value for command-line switch}
@d print_err(#)==begin if interaction=error_stop_mode then wake_up_terminal;
  if file_line_error_style_p then print_file_line
  else print_nl("! ");
  print(#);
  end
@z

@x xetex.web l.1998 - tex.ch [6.73]: |interaction_option|
@!interaction:batch_mode..error_stop_mode; {current level of interaction}

@ @<Set init...@>=interaction:=error_stop_mode;
@y
@!interaction:batch_mode..error_stop_mode; {current level of interaction}
@!interaction_option:batch_mode..unspecified_mode; {set from command line}

@ @<Set init...@>=if interaction_option=unspecified_mode then
  interaction:=error_stop_mode
else
  interaction:=interaction_option;
@z

@x xetex.web l.2126 - tex.ch [6.82]: halt on error?
print_char("."); show_context;
@y
print_char("."); show_context;
if (halt_on_error_p) then begin
  {If |close_files_and_terminate| generates an error, we'll end up back
   here; just give up in that case. If files are truncated, too bad.}
  if (halting_on_error_p) then do_final_end; {quit immediately}
  halting_on_error_p:=true;

  {This module is executed at the end of the |error| procedure in
   \.{tex.web}, but we'll never get there when |halt_on_error_p|, so the
   error help shouldn't get duplicated. It's potentially useful to see,
   especially if \.{\\errhelp} is being used. See thread at:
   \.{https://tug.org/pipermail/tex-live/2024-July/050741.html}.}
  @<Put help message on the transcript file@>;

  {Proceed with normal exit.}
  history:=fatal_error_stop;
  jump_out;
end;
@z

@x xetex.web l.8081 - the bounds are variables (C's |integer| in web2c)
@!l:0..half_error_line; {length of descriptive information on line 1}
@!m:integer; {context information gathered for line 2}
@!n:0..error_line; {length of line 1}
@y
@!l:integer; {length of descriptive information on line 1}
@!m:integer; {context information gathered for line 2}
@!n:integer; {length of line 1}
@z

@x xetex.web l.8274 - tex.ch [23.328]: keep the top of the file-name stack initialized
incr(in_open); push_input; index:=in_open;
@y
incr(in_open); push_input; index:=in_open;
full_source_filename_stack[index]:=0;
@z

@x xetex.web l.8305 - tex.ch [23.331]: the file-name stack at the bottom
begin input_ptr:=0; max_in_stack:=0;
@y
begin input_ptr:=0; max_in_stack:=0;
full_source_filename_stack[0]:=0;
@z

@x xetex.web l.8309 - tex.ch [23.331]: |buffer[0]| is initialized
first:=buf_size; repeat buffer[first]:=0; decr(first); until first=0;
@y
first:=buf_size; repeat buffer[first]:=0; decr(first); until first=0;
buffer[0]:=0;
@z

@x xetex.web l.11620 - tex.ch [27.484]: set |limit| when |fatal_error|
else fatal_error("*** (cannot \read from terminal in nonstop modes)")
@y
else begin
  limit:=0;
  fatal_error("*** (cannot \read from terminal in nonstop modes)");
  end
@z

@x xetex.web l.11907 - tex.ch [28.501]: \.{\\eof18} is true when \.{\\write18} is disabled
if_eof_code: begin scan_four_bit_int; b:=(read_open[cur_val]=closed);
  end;
@y
if_eof_code: begin scan_four_bit_int_or_18;
  if cur_val=18 then b:=not shellenabledp
  else b:=(read_open[cur_val]=closed);
  end;
@z

@x xetex.web l.12488 - tex.ch [29.534]: `-jobname'; the recorder's file name
if job_name=0 then job_name:="texput";
@.texput@>
@y
if job_name=0 then job_name:=get_job_name("texput");
@.texput@>
pack_job_name(".fls");
recorder_change_filename;
@z

@x xetex.web l.12525 - tex.ch [29.536]: web2c's version after the banner
begin wlog(banner);
slow_print(format_ident); print("  ");
@y
begin wlog(banner);
wlog_version_string;
slow_print(format_ident); print("  ");
@z

@x xetex.web l.12532 - tex.ch [29.536]: the status lines after the banner
if eTeX_ex then
  begin; wlog_cr; wlog('entering extended mode');
  end;
end
@y
if eTeX_ex then
  begin; wlog_cr; wlog('entering extended mode');
  end;
if shellenabledp then begin
  wlog_cr;
  wlog(' ');
  if restrictedshell then begin
    wlog('restricted ');
  end;
  wlog('\write18 enabled.')
  end;
if file_line_error_style_p then begin
  wlog_cr;
  wlog(' file:line:error style messages enabled.')
  end;
if parse_first_line_p then begin
  wlog_cr;
  wlog(' %&-line parsing enabled.');
  end;
if translate_filename_p then begin
  wlog_cr;
  wlog(' (WARNING: translate-file "');
  wlog_translate_filename;
  wlog('" ignored)');
  end;
end
@z

@x xetex.web l.12559 - tex.ch [29.537]: was |job_name| given on the command line?
  begin job_name:=cur_name; open_log_file;
@y
  begin job_name:=get_job_name(cur_name); open_log_file;
@z

@x xetex.web l.14618 - tex.ch [32.617]: `-output-comment' replaces the preamble's comment
  old_setting:=selector; selector:=new_string;
  print(" XeTeX output "); print_int(year); print_char(".");
  print_two(month); print_char("."); print_two(day);
  print_char(":"); print_two(time div 60);
  print_two(time mod 60);
  selector:=old_setting; dvi_out(cur_length);
  for s:=str_start_macro(str_ptr) to pool_ptr-1 do dvi_out(so(str_pool[s]));
  pool_ptr:=str_start_macro(str_ptr); {flush the current string}
@y
if output_comment_length>=0 then
  begin dvi_out(output_comment_length);
  for s:=0 to output_comment_length-1 do dvi_out(output_comment_byte(s));
  end
else begin {the default code is unchanged}
  old_setting:=selector; selector:=new_string;
  print(" XeTeX output "); print_int(year); print_char(".");
  print_two(month); print_char("."); print_two(day);
  print_char(":"); print_two(time div 60);
  print_two(time mod 60);
  selector:=old_setting; dvi_out(cur_length);
  for s:=str_start_macro(str_ptr) to pool_ptr-1 do dvi_out(so(str_pool[s]));
  pool_ptr:=str_start_macro(str_ptr); {flush the current string}
end;
@z

@x xetex.web l.29214 - tex.ch [50.1327]: `-interaction' overrides the format's mode
undump(batch_mode)(error_stop_mode)(interaction);
@y
undump(batch_mode)(error_stop_mode)(interaction);
if interaction_option<>unspecified_mode then interaction:=interaction_option;
@z

@x xetex.web l.29301 - tex.ch [51.1332]: the texmf.cnf values come first
history:=fatal_error_stop; {in case we quit during initialization}
@y
setup_bound_vars; {|error_line|, \dots, from texmf.cnf (system.rs)}
if error_line > ssup_error_line then error_line := ssup_error_line;
@<Copy the run's settings from system.rs@>;
history:=fatal_error_stop; {in case we quit during initialization}
@z

@x xetex.web l.29464 - tex.ch [51.1337]: a `%&format' line names the format
if (format_ident=0)or(buffer[loc]="&") then
@y
if (format_ident=0)or(buffer[loc]="&")or dump_line then
@z

@x xetex.web l.29770 - tex.ch [53.1350]: \.{\\write18}
  else if cur_val>15 then cur_val:=16;
@y
  else if (cur_val>15) and (cur_val <> 18) then cur_val:=16;
@z

@x xetex.web l.30339 - tex.ch [53.1370]: \.{\\write18}{\it command}
procedure write_out(@!p:pointer);
var old_setting:0..max_selector; {holds print |selector|}
@!old_mode:integer; {saved |mode|}
@!j:small_number; {write stream number}
@!k:integer;
@!q,@!r:pointer; {temporary variables for list manipulation}
begin @<Expand macros in the token list
@y
procedure write_out(@!p:pointer);
var old_setting:0..max_selector; {holds print |selector|}
@!old_mode:integer; {saved |mode|}
@!j:small_number; {write stream number}
@!k:integer;
@!q,@!r:pointer; {temporary variables for list manipulation}
@!d:integer; {number of characters in incomplete current string}
@!clobbered:boolean; {system string is ok?}
@!runsystem_ret:integer; {return value from |runsystem|}
begin @<Expand macros in the token list
@z

@x xetex.web l.30348 - tex.ch [53.1370]: \.{\\write18}{\it command} runs the command
if write_open[j] then selector:=j
@y
if j=18 then selector := new_string
else if write_open[j] then selector:=j
@z

@x xetex.web l.30354 - tex.ch [53.1370]: \.{\\write18}{\it command} runs the command
flush_list(def_ref); selector:=old_setting;
@y
flush_list(def_ref);
if j=18 then
  begin if (tracing_online<=0) then
    selector:=log_only  {Show what we're doing in the log file.}
  else selector:=term_and_log;  {Show what we're doing.}
  {If the log file isn't open yet, we can only send output to the terminal.
   Calling |open_log_file| from here seems to result in bad data in the log.}
  if not log_opened then selector:=term_only;
  print_nl("runsystem(");
  for d:=0 to cur_length-1 do
    begin {|print| gives up if passed |str_ptr|, so do it by hand.}
    print(so(str_pool[str_start_macro(str_ptr)+d])); {N.B.: not |print_char|}
    end;
  print(")...");
  if shellenabledp then begin
    str_room(1); append_char(0); {Append a null byte to the expansion.}
    clobbered:=false;
    for d:=0 to cur_length-1 do {Convert to external character set.}
      begin
      if (str_pool[str_start_macro(str_ptr)+d]=null_code)
           and (d<cur_length-1) then clobbered:=true;
        {minimal checking: NUL not allowed in argument string of |system|()}
      end;
    if clobbered then print("clobbered")
    else begin {We have the command.  See if we're allowed to execute it,
         and report in the log.  We don't check the actual exit status of
         the command, or do anything with the output.}
      {xetex.ch [53.1370]: the command is the string's UTF-16 converted to
       UTF-8, as |append_to_name| converts it (|runsystem|, system.rs)}
      runsystem_ret := runsystem(str_start_macro(str_ptr), cur_length-1);
      if runsystem_ret = -1 then print("quotation error in system command")
      else if runsystem_ret = 0 then print("disabled (restricted)")
      else if runsystem_ret = 1 then print("executed")
      else if runsystem_ret = 2 then print("executed safely (allowed)")
    end;
  end else begin
    print("disabled"); {|shellenabledp| false}
  end;
  print_char("."); print_nl(""); print_ln;
  pool_ptr:=str_start_macro(str_ptr);  {erase the string}
end;
selector:=old_setting;
@z

@x xetex.web l.30461 - tex.ch [53.1374]: texmf.cnf's |openout_any|
      while not a_open_out(write_file[j]) do
@y
      while not kpse_out_name_ok or not a_open_out(write_file[j]) do
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ The state texmfmp.c sets up from the command line and texmf.cnf before
the program starts (system.rs).

@d ssup_error_line = 255 {the largest |error_line| (tex.ch)}

@<Glob...@>=
@!error_line:integer; {width of context lines on terminal error messages}
@!half_error_line:integer; {width of first lines of contexts in terminal
  error messages; should be between 30 and |error_line-15|}
@!max_print_line:integer;
  {width of longest text lines output; should be at least 60}
@!file_line_error_style_p:boolean; {format messages as file:line:error}
@!halt_on_error_p:boolean; {stop at first error}
@!halting_on_error_p:boolean; {already trying to halt?}
@!parse_first_line_p:boolean; {parse the first line for options}
@!dump_line:boolean; {was a \.{\%\AM format} line seen?}
@!eight_bit_p:boolean; {make all characters printable by default}
  {(xetex.web compares it with 0, as C does a boolean)}
@!translate_filename_p:boolean; {was a TCX file given?}

@ The C code sets these before the program starts, so before |initialize|.

@<Copy the run's settings from system.rs@>=
interaction_option:=web2c_interaction_option;
file_line_error_style_p:=web2c_file_line_error_style_p;
halt_on_error_p:=web2c_halt_on_error_p;
parse_first_line_p:=web2c_parse_first_line_p;
dump_line:=web2c_dump_line;
eight_bit_p:=web2c_eight_bit_p;
translate_filename_p:=web2c_translate_filename_p;
shellenabledp:=web2c_shellenabledp; restrictedshell:=web2c_restrictedshell;
no_pdf_output:=web2c_no_pdf_output

@ @<Set init...@>=
halting_on_error_p:=false;

@ The routines of system.rs this needs.

@<Declare web2c's file-name procedures@>=
procedure setup_bound_vars; external;
  {|error_line|, |half_error_line|, |max_print_line| and |expand_depth|
   from texmf.cnf}
function web2c_interaction_option:integer; external;
function web2c_file_line_error_style_p:boolean; external;
function web2c_halt_on_error_p:boolean; external;
function web2c_parse_first_line_p:boolean; external;
function web2c_dump_line:boolean; external;
function web2c_eight_bit_p:boolean; external;
function web2c_translate_filename_p:boolean; external;
function web2c_shellenabledp:boolean; external;
function web2c_restrictedshell:boolean; external;
function web2c_no_pdf_output:boolean; external;
  {`\.{-no-pdf}' (xetexextra.c): write \.{XDV}, run no driver}
procedure wlog_translate_filename; external;
  {the TCX file's name into the log}
procedure wterm_translate_filename; external;
  {the TCX file's name on the terminal}
procedure wterm_version_string; external;
  {web2c's |versionstring| on the terminal}
procedure wlog_version_string; external;
  {web2c's |versionstring| into the log}
procedure do_final_end; external;
  {tex.ch's |do_final_end|: leave with the status |history| implies}
function get_job_name(@!s:str_number):str_number; external;
  {`\.{-jobname}', else |s|}
procedure recorder_change_filename; external;
  {the recorder's file becomes |name_of_file|}
procedure set_tex_input_type(@!b:boolean); external;
  {tex.ch's |tex_input_type|: \.{\\input} (kpathsea's |must_exist|)
   or \.{\\openin}}
function kpse_in_name_ok:boolean; external;
  {may |name_of_file| be read? (texmf.cnf's |openin_any|)}
function kpse_out_name_ok:boolean; external;
  {may |name_of_file| be written? (texmf.cnf's |openout_any|)}
function output_comment_length:integer; external;
  {the length of `\.{-output-comment}', at most 255, or -1 without one}
function output_comment_byte(@!k:integer):integer; external;
  {its byte |k|}
function runsystem(@!s:pool_pointer;@!l:integer):integer; external;
  {texmfmp.c's |runsystem| on |str_pool[s..s+l-1]|, UTF-16 made UTF-8}

@ A helper for printing file:line:error style messages.  Look for a
filename in |full_source_filename_stack|, and if we fail to find
one fall back on the non-file:line:error style.

@<Basic print...@>=
procedure print_file_line;
var level: 0..max_in_open;
begin
  level:=in_open;
  while (level>0) and (full_source_filename_stack[level]=0) do
    decr(level);
  if level=0 then
    print_nl("! ")
  else begin
    print_nl (""); print (full_source_filename_stack[level]); print (":");
    if level=in_open then print_int (line)
    else print_int (line_stack[level+1]);
    print (": ");
  end;
end;

@ To be able to determine whether \.{\\write18} is enabled from within
\TeX\ we also implement \.{\\eof18}.  We sort of cheat by having an
additional route |scan_four_bit_int_or_18| which is the same as
|scan_four_bit_int| except it also accepts the value 18.

@<Declare procedures that scan restricted classes of integers@>=
procedure scan_four_bit_int_or_18;
begin scan_int;
if (cur_val<0)or((cur_val>15)and(cur_val<>18)) then
  begin print_err("Bad number");
@.Bad number@>
  help2("Since I expected to read a number between 0 and 15,")@/
    ("I changed this one to zero."); int_error(cur_val); cur_val:=0;
  end;
end;

@* \[55] Index.
@z
