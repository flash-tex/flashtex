% flashtex.ch: TeX Live's bibtex.ch (third_party/bibtex/bibtex.ch) is
% Pascal with C inside, which web2c's `convert` (web2c, cvtbib.sed and
% fixwrites) and TeX Live's library (lib/*.c, cpascal.h) turn into the C
% program TeX Live ships. This change file, applied after bibtex.ch, says
% the same things in the Pascal tools/web2rust translates: each C construct
% becomes either plain Pascal or a call of an `external' routine, whose
% Rust body (src/system.rs) does what TeX Live's C does. Nothing here
% changes what bibtex.web and bibtex.ch do.
%
% * debug, trace: off; stat: on (TeX Live builds stat in).
% * The nonlocal gotos cvtbib.sed turns into setjmp/longjmp (|close_up_shop|
%   from anywhere, |bst_done| from |bst_err_print_and_look_for_blank_line|)
%   become `external' jumps that unwind to a catching `external' wrapper;
%   |aux_done| is cvtbib.sed's |lab31| flag.
% * XTALLOC(n,T) is xmalloc_array(T,n-1) (both allocate n elements);
%   BIB_XRETALLOC is a log line (cpascal.h's fprintf) and xrealloc_array.
% * |text_char| stays Pascal's |char|: fixwrites prints |xchr[...]| and
%   |name_of_file[...]| with %c, which is what web2rust does with a |char|.

@x [2] standard_input is never read.
standard_input, standard_output: text;
@y
standard_input, standard_output: alpha_file; {Pascal's |text|}
@!lab31: boolean; {cvtbib.sed's flag: the \.{.aux} files are done}
@!aux_arg: integer; {where the command line's file argument is, for the host}
@z

@x [4] debug and trace are off, stat is on (--stat).
@d debug == ifdef('TEXMF_DEBUG')
@d gubed == endif('TEXMF_DEBUG')
@y
@d debug == @{
@d gubed == @t@>@}
@z

@x
@d trace == ifdef@&('TRACE')
@d ecart == endif@&('TRACE')
@y
@d trace == @{
@d ecart == @t@>@}
@z

@x [9] Macros for the C parts.
@d close_up_shop=9998           {jump here after fatal errors}
@y
@d close_up_shop=9998           {jump here after fatal errors}
@d version_string==' (TeX Live 2026)'
@d kpse_bib_format=1 {for |a_open_in|: \.{BIBINPUTS}}
@d kpse_bst_format=2 {for |a_open_in|: \.{BSTINPUTS}}
@z

@x [10] The main program: no `hack0'; the work is in |main_part|.
label   close_up_shop @<Labels in the outer block@>;
@y
label   close_up_shop;
@z

@x
standard_input := stdin;
standard_output := stdout;
@y
open_standard_files;
@z

@x
bib_file   := XTALLOC (max_bib_files + 1, alpha_file);
bib_list   := XTALLOC (max_bib_files + 1, str_number);
entry_ints := nil;
entry_strs := nil;
wiz_functions := XTALLOC (wiz_fn_space + 1, hash_ptr2);
field_info := XTALLOC (max_fields + 1, str_number);
s_preamble := XTALLOC (max_bib_files + 1, str_number);
str_pool   := XTALLOC (pool_size + 1, ASCII_code);
buffer     := XTALLOC (buf_size + 1, ASCII_code);
sv_buffer  := XTALLOC (buf_size + 1, ASCII_code);
ex_buf     := XTALLOC (buf_size + 1, ASCII_code);
out_buf    := XTALLOC (buf_size + 1, ASCII_code);
name_tok   := XTALLOC (buf_size + 1, buf_pointer);
name_sep_char := XTALLOC (buf_size + 1, ASCII_code);
@#
glb_str_ptr := XTALLOC (max_glob_strs, str_number);
global_strs := XTALLOC (max_glob_strs * (glob_str_size + 1), ASCII_code);
glb_str_end := XTALLOC (max_glob_strs, integer);
@#
cite_list  := XTALLOC (max_cites + 1, str_number);
type_list  := XTALLOC (max_cites + 1, hash_ptr2);
entry_exists := XTALLOC (max_cites + 1, boolean);
cite_info  := XTALLOC (max_cites + 1, str_number);
@#
str_start  := XTALLOC (max_strings + 1, pool_pointer);
@#
hash_next  := XTALLOC (hash_max + 1, hash_pointer);
hash_text  := XTALLOC (hash_max + 1, str_number);
hash_ilk   := XTALLOC (hash_max + 1, str_ilk);
ilk_info   := XTALLOC (hash_max + 1, integer);
fn_type    := XTALLOC (hash_max + 1, fn_class);
@#
lit_stack  := XTALLOC (lit_stk_size + 1, integer);
lit_stk_type := XTALLOC (lit_stk_size + 1, stk_type);
@#
compute_hash_prime;
@#
initialize;
{This initializes the jmp9998 buffer, which can be used early}
hack0;
@y
bib_file   := xmalloc_array (alpha_file, max_bib_files);
bib_list   := xmalloc_array (str_number, max_bib_files);
wiz_functions := xmalloc_array (hash_ptr2, wiz_fn_space);
field_info := xmalloc_array (str_number, max_fields);
s_preamble := xmalloc_array (str_number, max_bib_files);
str_pool   := xmalloc_array (ASCII_code, pool_size);
buffer     := xmalloc_array (ASCII_code, buf_size);
sv_buffer  := xmalloc_array (ASCII_code, buf_size);
ex_buf     := xmalloc_array (ASCII_code, buf_size);
out_buf    := xmalloc_array (ASCII_code, buf_size);
name_tok   := xmalloc_array (buf_pointer, buf_size);
name_sep_char := xmalloc_array (ASCII_code, buf_size);
@#
glb_str_ptr := xmalloc_array (str_number, max_glob_strs - 1);
global_strs := xmalloc_array (ASCII_code,
                              max_glob_strs * (glob_str_size + 1) - 1);
glb_str_end := xmalloc_array (integer, max_glob_strs - 1);
@#
cite_list  := xmalloc_array (str_number, max_cites);
type_list  := xmalloc_array (hash_ptr2, max_cites);
entry_exists := xmalloc_array (boolean, max_cites);
cite_info  := xmalloc_array (str_number, max_cites);
@#
str_start  := xmalloc_array (pool_pointer, max_strings);
@#
hash_next  := xmalloc_array (hash_pointer, hash_max);
hash_text  := xmalloc_array (str_number, hash_max);
hash_ilk   := xmalloc_array (str_ilk, hash_max);
ilk_info   := xmalloc_array (integer, hash_max);
fn_type    := xmalloc_array (fn_class, hash_max);
@#
lit_stack  := xmalloc_array (integer, lit_stk_size);
lit_stk_type := xmalloc_array (stk_type, lit_stk_size);
@#
compute_hash_prime;
@#
initialize;
@z

@x
@<Read the \.{.aux} file@>;
@<Read and execute the \.{.bst} file@>;
close_up_shop:
@y
catch_close_up_shop; {|main_part|, which a fatal error leaves}
close_up_shop:
@z

@x [14] maxint is C's.
@d file_name_size==maxint {file names have no arbitrary maximum length}
@y
@d file_name_size==@'17777777777 {file names have no arbitrary maximum length}
@z

@x [23] text_char stays char (see above), with all 256 codes.
@d text_char == ASCII_code    {the data type of characters in text files}
@y
@d text_char == char    {the data type of characters in text files}
@z

@x [38] The C routines bibtex.ch uses, declared here.
@d no_file_path = -1

@<Procedures and functions for all file...@>=
function bib_makecstring(s:str_number):cstring;
var cstr:cstring;
    i:pool_pointer;
begin
  cstr := xmalloc_array (ASCII_code, length (s) + 1);
  for i := 0 to length(s) - 1 do begin
    cstr[i] := str_pool[str_start[s] + i];
  end;
  cstr[length(s)] := 0;
  bib_makecstring := cstr;
exit: end;
@y
@d no_file_path = -1

@<Procedures and functions for all file...@>=
procedure uexit(@!code:integer); external;
procedure open_standard_files; external;
procedure parse_arguments; external;
function setup_bound_value(@!name:const_cstring; @!dflt:integer):integer;
  external;
procedure log_realloc(@!name:const_cstring; @!elt_size, @!new_size,
  @!old_size:integer); external;
function c_char(@!s:const_cstring; @!i:integer):integer; external;
function getc(var f:alpha_file):integer; external;
procedure vgetc(var f:alpha_file); external;
function a_open_in(var f:alpha_file; @!path:integer):boolean; external;
function a_open_in_with_dirname(var f:alpha_file; @!path:integer;
  @!top:str_number):boolean; external;
function a_open_out(var f:alpha_file):boolean; external;
procedure a_close(var f:alpha_file); external;
function name_in_ok:boolean; external;
function name_out_ok:boolean; external;
procedure set_aux_name_from_command_line; external;
procedure catch_close_up_shop; external;
procedure catch_bst_done; external;
procedure jump_to_bst_done; external;
procedure check_deadline; external;
@z

@x [46] BIB_XRETALLOC.
BIB_XRETALLOC_NOSET ('buffer', buffer, ASCII_code,
                     buf_size, buf_size + BUF_SIZE);
BIB_XRETALLOC_NOSET ('sv_buffer', sv_buffer, ASCII_code,
                     buf_size, buf_size + BUF_SIZE);
BIB_XRETALLOC_NOSET ('ex_buf', ex_buf, ASCII_code,
                     buf_size, buf_size + BUF_SIZE);
BIB_XRETALLOC_NOSET ('out_buf', out_buf, ASCII_code,
                     buf_size, buf_size + BUF_SIZE);
BIB_XRETALLOC_NOSET ('name_tok', name_tok, buf_pointer,
                     buf_size, buf_size + BUF_SIZE);
BIB_XRETALLOC ('name_sep_char', name_sep_char, ASCII_code,
               buf_size, buf_size + BUF_SIZE);
@y
log_realloc ('buffer', 1, buf_size + BUF_SIZE, buf_size);
buffer := xrealloc_array (buffer, ASCII_code, buf_size + BUF_SIZE);
log_realloc ('sv_buffer', 1, buf_size + BUF_SIZE, buf_size);
sv_buffer := xrealloc_array (sv_buffer, ASCII_code, buf_size + BUF_SIZE);
log_realloc ('ex_buf', 1, buf_size + BUF_SIZE, buf_size);
ex_buf := xrealloc_array (ex_buf, ASCII_code, buf_size + BUF_SIZE);
log_realloc ('out_buf', 1, buf_size + BUF_SIZE, buf_size);
out_buf := xrealloc_array (out_buf, ASCII_code, buf_size + BUF_SIZE);
log_realloc ('name_tok', 4, buf_size + BUF_SIZE, buf_size);
name_tok := xrealloc_array (name_tok, buf_pointer, buf_size + BUF_SIZE);
log_realloc ('name_sep_char', 1, buf_size + BUF_SIZE, buf_size);
name_sep_char := xrealloc_array (name_sep_char, ASCII_code,
                                 buf_size + BUF_SIZE);
buf_size := buf_size + BUF_SIZE;
@z

@x [53] BIB_XRETALLOC.
BIB_XRETALLOC ('str_pool', str_pool, ASCII_code, pool_size,
               pool_size + POOL_SIZE);
@y
log_realloc ('str_pool', 1, pool_size + POOL_SIZE, pool_size);
str_pool := xrealloc_array (str_pool, ASCII_code, pool_size + POOL_SIZE);
pool_size := pool_size + POOL_SIZE;
@z

@x [58] name_of_file is reallocated (xmalloc_array frees nothing here).
free (name_of_file);
name_of_file := xmalloc_array (ASCII_code, length (file_name) + 1);
@y
name_of_file := xmalloc_array (text_char, length (file_name) + 1);
@z

@x [73] A C string literal.
@!pds_type = const_cstring;
@y
@!pds_type = const_cstring;
@!rust_dummy_type = 0..1; {so that the section is not empty after this}
@z

@x [77] C strings start at zero instead of one.
    buffer[i] := xord[ucharcast(pds[i-1])];
@y
    buffer[i] := xord[chr(c_char(pds,i-1))];
@z

@x [100] The command line's file argument.
  name_of_file := xmalloc_array (ASCII_code, strlen (cmdline (optind)) + 5);
  strcpy (stringcast(name_of_file + 1), cmdline (optind));
  aux_name_length := strlen (stringcast(name_of_file + 1));
@y
  set_aux_name_from_command_line;
@z

@x [106] strcmp and kpathsea's name checks.
if (name_length < 4) or
   (strcmp (stringcast(name_of_file + 1 + name_length - 4), '.aux') <> 0)
then
@y
if (name_length < 4) or
   (name_of_file[name_length - 3] <> '.') or
   (name_of_file[name_length - 2] <> 'a') or
   (name_of_file[name_length - 1] <> 'u') or
   (name_of_file[name_length] <> 'x')
then
@z

@x
if (not kpse_in_name_ok(stringcast(name_of_file+1)) or
    not a_open_in(cur_aux_file,no_file_path)) then
@y
if (not name_in_ok or
    not a_open_in(cur_aux_file,no_file_path)) then
@z

@x
if (not kpse_out_name_ok(stringcast(name_of_file+1)) or
    not a_open_out(log_file)) then
@y
if (not name_out_ok or
    not a_open_out(log_file)) then
@z

@x
if (not kpse_out_name_ok(stringcast(name_of_file+1)) or
    not a_open_out(bbl_file)) then
@y
if (not name_out_ok or
    not a_open_out(bbl_file)) then
@z

@x [146] The .aux loop runs until lab31 (cvtbib.sed's while (lab31==0)).
loop
    begin                       {|pop_the_aux_stack| will exit the loop}
@y
while not lab31 do
    begin                       {|pop_the_aux_stack| will exit the loop}
@z

@x [123] BIB_XRETALLOC; kpathsea's name check.
  BIB_XRETALLOC_NOSET ('bib_list', bib_list, str_number, max_bib_files,
                 max_bib_files + MAX_BIB_FILES);@/
  BIB_XRETALLOC_NOSET ('bib_file', bib_file, alpha_file, max_bib_files,
                 max_bib_files + MAX_BIB_FILES);@/
  BIB_XRETALLOC ('s_preamble', s_preamble, str_number, max_bib_files,
                 max_bib_files + MAX_BIB_FILES);
end;
@y
  log_realloc ('bib_list', 4, max_bib_files + MAX_BIB_FILES, max_bib_files);
  bib_list := xrealloc_array (bib_list, str_number,
                              max_bib_files + MAX_BIB_FILES);
  log_realloc ('bib_file', 8, max_bib_files + MAX_BIB_FILES, max_bib_files);
  bib_file := xrealloc_array (bib_file, alpha_file,
                              max_bib_files + MAX_BIB_FILES);
  log_realloc ('s_preamble', 4, max_bib_files + MAX_BIB_FILES, max_bib_files);
  s_preamble := xrealloc_array (s_preamble, str_number,
                                max_bib_files + MAX_BIB_FILES);
  max_bib_files := max_bib_files + MAX_BIB_FILES;
end;
@z

@x
if (not kpse_in_name_ok(stringcast(name_of_file+1)) or
    not a_open_in(cur_bib_file, kpse_bib_format)) then
@y
if (not name_in_ok or
    not a_open_in(cur_bib_file, kpse_bib_format)) then
@z

@x [127] kpathsea's name check.
if (not kpse_in_name_ok(stringcast(name_of_file+1)) or
    not a_open_in(bst_file, kpse_bst_format)) then
@y
if (not name_in_ok or
    not a_open_in(bst_file, kpse_bst_format)) then
@z

@x [138] BIB_XRETALLOC.
    BIB_XRETALLOC_NOSET ('cite_list', cite_list, str_number,
                         max_cites, max_cites + MAX_CITES);
    BIB_XRETALLOC_NOSET ('type_list', type_list, hash_ptr2,
                         max_cites, max_cites + MAX_CITES);
    BIB_XRETALLOC_NOSET ('entry_exists', entry_exists, boolean,
                         max_cites, max_cites + MAX_CITES);
    BIB_XRETALLOC ('cite_info', cite_info, str_number,
                   max_cites, max_cites + MAX_CITES);
@y
    log_realloc ('cite_list', 4, max_cites + MAX_CITES, max_cites);
    cite_list := xrealloc_array (cite_list, str_number, max_cites + MAX_CITES);
    log_realloc ('type_list', 4, max_cites + MAX_CITES, max_cites);
    type_list := xrealloc_array (type_list, hash_ptr2, max_cites + MAX_CITES);
    log_realloc ('entry_exists', 4, max_cites + MAX_CITES, max_cites);
    entry_exists := xrealloc_array (entry_exists, boolean,
                                    max_cites + MAX_CITES);
    log_realloc ('cite_info', 4, max_cites + MAX_CITES, max_cites);
    cite_info := xrealloc_array (cite_info, str_number, max_cites + MAX_CITES);
    max_cites := max_cites + MAX_CITES;
@z

@x [141] kpathsea's name check; the top-level name's directory.
if (not kpse_in_name_ok(stringcast(name_of_file+1))
    or (not a_open_in(cur_aux_file, no_file_path)
        and not a_open_in_with_dirname(cur_aux_file, no_file_path,
                                       bib_makecstring(top_lev_str)))
    ) then
@y
if (not name_in_ok
    or (not a_open_in(cur_aux_file, no_file_path)
        and not a_open_in_with_dirname(cur_aux_file, no_file_path,
                                       top_lev_str))
    ) then
@z

@x [142] cvtbib.sed: goto lab31 is {lab31=1; return;}
    goto aux_done
@y
    begin lab31 := true; return; end
@z

@x [150] cvtbib.sed: goto bst_done is longjmp(jmp32,1).
        goto bst_done
@y
        jump_to_bst_done
@z

@x [151] cvtbib.sed: hack1 is if(setjmp(jmp32)==0)for(;;), hack2 is break.
hack1;
    begin
    if (not eat_bst_white_space) then   {the end of the \.{.bst} file}
        hack2;
    get_bst_command_and_process;
    end;
bst_done: a_close (bst_file);
@y
catch_bst_done; {|bst_loop|, which |jump_to_bst_done| leaves}
bst_done: a_close (bst_file);
@z

@x [160] end_of_def is a variable: web2c makes the subrange an integer.
@!hash_ptr2 = quote_next_fn..end_of_def; {a special marker or a |hash_loc|}
@y
@!hash_ptr2 = integer; {a special marker or a |hash_loc|}
@!fn_def_loc = integer; {|scan_fn_def|'s local type, for web2rust}
@z

@x [187] A local type section is not web2rust's Pascal.
type @!fn_def_loc = integer; {for a single |wiz_defined|-function}
var singl_function : ^hash_ptr2;
@y
var singl_function : ^hash_ptr2;
@z

@x [187] Dynamically allocate singl_function.
singl_function := XTALLOC (single_fn_space + 1, hash_ptr2);
@y
singl_function := xmalloc_array (hash_ptr2, single_fn_space);
@z

@x
exit:
libc_free (singl_function);
end;
@y
exit:
end;
@z

@x [188] BIB_XRETALLOC.
                            BIB_XRETALLOC ('singl_function', singl_function, hash_ptr2,
                                           single_fn_space, single_fn_space + SINGLE_FN_SPACE);
@y
                            log_realloc ('singl_function', 4,
                                         single_fn_space + SINGLE_FN_SPACE,
                                         single_fn_space);
                            singl_function := xrealloc_array (singl_function,
                                hash_ptr2, single_fn_space + SINGLE_FN_SPACE);
                            single_fn_space := single_fn_space
                                               + SINGLE_FN_SPACE;
@z

@x [200] BIB_XRETALLOC.
    BIB_XRETALLOC ('wiz_functions', wiz_functions, hash_ptr2,
                    wiz_fn_space, wiz_fn_space + WIZ_FN_SPACE);
@y
    log_realloc ('wiz_functions', 4, wiz_fn_space + WIZ_FN_SPACE,
                 wiz_fn_space);
    wiz_functions := xrealloc_array (wiz_functions, hash_ptr2,
                                     wiz_fn_space + WIZ_FN_SPACE);
    wiz_fn_space := wiz_fn_space + WIZ_FN_SPACE;
@z

@x [216] BIB_XRETALLOC.
    BIB_XRETALLOC_NOSET ('glb_str_ptr', glb_str_ptr, str_number,
                         max_glob_strs, max_glob_strs + MAX_GLOB_STRS);@/
    BIB_XRETALLOC_STRING ('global_strs', global_strs, glob_str_size,
                          max_glob_strs, max_glob_strs + MAX_GLOB_STRS);@/
    BIB_XRETALLOC ('glb_str_end', glb_str_end, integer,
                   max_glob_strs, max_glob_strs + MAX_GLOB_STRS);@/
@y
    log_realloc ('glb_str_ptr', 4, max_glob_strs + MAX_GLOB_STRS,
                 max_glob_strs);
    glb_str_ptr := xrealloc_array (glb_str_ptr, str_number,
                                   max_glob_strs + MAX_GLOB_STRS);
    log_realloc ('global_strs', glob_str_size + 1,
                 max_glob_strs + MAX_GLOB_STRS, max_glob_strs);
    global_strs := xrealloc_array (global_strs, ASCII_code,
              (max_glob_strs + MAX_GLOB_STRS) * (glob_str_size + 1) - 1);
    log_realloc ('glb_str_end', 4, max_glob_strs + MAX_GLOB_STRS,
                 max_glob_strs);
    glb_str_end := xrealloc_array (glb_str_end, integer,
                                   max_glob_strs + MAX_GLOB_STRS);
    max_glob_strs := max_glob_strs + MAX_GLOB_STRS;
@z

@x [226] BIB_XRETALLOC.
    BIB_XRETALLOC ('field_info', field_info, str_number, max_fields,
                   total_fields + MAX_FIELDS);
@y
    log_realloc ('field_info', 4, total_fields + MAX_FIELDS, max_fields);
    field_info := xrealloc_array (field_info, str_number,
                                  total_fields + MAX_FIELDS);
    max_fields := total_fields + MAX_FIELDS;
@z

@x [242] BIB_XRETALLOC.
  BIB_XRETALLOC_NOSET ('bib_list', bib_list, str_number, max_bib_files,
                 max_bib_files + MAX_BIB_FILES);@/
  BIB_XRETALLOC_NOSET ('bib_file', bib_file, alpha_file, max_bib_files,
                 max_bib_files + MAX_BIB_FILES);@/
  BIB_XRETALLOC ('s_preamble', s_preamble, str_number, max_bib_files,
                 max_bib_files + MAX_BIB_FILES);@/
@y
  log_realloc ('bib_list', 4, max_bib_files + MAX_BIB_FILES, max_bib_files);
  bib_list := xrealloc_array (bib_list, str_number,
                              max_bib_files + MAX_BIB_FILES);
  log_realloc ('bib_file', 8, max_bib_files + MAX_BIB_FILES, max_bib_files);
  bib_file := xrealloc_array (bib_file, alpha_file,
                              max_bib_files + MAX_BIB_FILES);
  log_realloc ('s_preamble', 4, max_bib_files + MAX_BIB_FILES, max_bib_files);
  s_preamble := xrealloc_array (s_preamble, str_number,
                                max_bib_files + MAX_BIB_FILES);
  max_bib_files := max_bib_files + MAX_BIB_FILES;
@z

@x [287] XTALLOC.
entry_ints := XTALLOC ((num_ent_ints + 1) * (num_cites + 1), integer);
@y
entry_ints := xmalloc_array (integer, (num_ent_ints + 1) * (num_cites + 1) - 1);
@z

@x [288] XTALLOC.
entry_strs := XTALLOC ((num_ent_strs + 1) * (num_cites + 1) * (ent_str_size + 1), ASCII_code);
@y
entry_strs := xmalloc_array (ASCII_code,
              (num_ent_strs + 1) * (num_cites + 1) * (ent_str_size + 1) - 1);
@z

@x [290] Run-time bounds: web2c makes these subranges integers.
@!ent_chr_ptr : 0..ent_str_size; {points at a |str_entry_var| character}
@!glob_chr_ptr : 0..glob_str_size; {points at a |str_global_var| character}
@y
@!ent_chr_ptr : integer; {points at a |str_entry_var| character}
@!glob_chr_ptr : integer; {points at a |str_global_var| character}
@z

@x [301] Run-time bound.
var char_ptr : 0..ent_str_size;         {character index into compared strings}
@y
var char_ptr : integer;         {character index into compared strings}
@z

@x [307] BIB_XRETALLOC.
    BIB_XRETALLOC_NOSET ('lit_stack', lit_stack, integer,
                         lit_stk_size, lit_stk_size + LIT_STK_SIZE);
    BIB_XRETALLOC ('lit_stk_type', lit_stk_type, stk_type,
                   lit_stk_size, lit_stk_size + LIT_STK_SIZE);
@y
    log_realloc ('lit_stack', 4, lit_stk_size + LIT_STK_SIZE, lit_stk_size);
    lit_stack := xrealloc_array (lit_stack, integer,
                                 lit_stk_size + LIT_STK_SIZE);
    log_realloc ('lit_stk_type', 1, lit_stk_size + LIT_STK_SIZE,
                 lit_stk_size);
    lit_stk_type := xrealloc_array (lit_stk_type, stk_type,
                                    lit_stk_size + LIT_STK_SIZE);
    lit_stk_size := lit_stk_size + LIT_STK_SIZE;
@z

@x [325] The host's time limit: checked as each function starts.
  ecart@/
case (fn_type[ex_fn_loc]) of
    built_in : @<Execute a \(b)|built_in| function@>;
@y
  ecart@/
check_deadline; {an in-process run stops here once out of time}
case (fn_type[ex_fn_loc]) of
    built_in : @<Execute a \(b)|built_in| function@>;
@z

@x [467] parse_arguments is the external procedure (src/system.rs).
@d argument_is (#) == (strcmp (long_options[option_index].name, #) = 0)

@<Define \(p)|parse_arguments|@> =
procedure parse_arguments;
const n_options = 4; {Pascal won't count array lengths for us.}
var @!long_options: array[0..n_options] of getopt_struct;
    @!getopt_return_val: integer;
    @!option_index: c_int_type;
    @!current_option: 0..n_options;
begin
  @<Initialize the option variables@>;
  @<Define the option table@>;
  repeat
    getopt_return_val := getopt_long_only (argc, argv, '', long_options,
                                           address_of (option_index));
    if getopt_return_val = -1 then begin
      do_nothing; {End of arguments; we exit the loop below.}

    end else if getopt_return_val = "?" then begin
      usage (my_name);

    end else if argument_is ('min-crossrefs') then begin
      min_crossrefs := atoi (optarg);

    end else if argument_is ('help') then begin
      usage_help (BIBTEX_HELP, nil);

    end else if argument_is ('version') then begin
      print_version_and_exit (banner, 'Oren Patashnik', nil, nil);

    end; {Else it was a flag; |getopt| has already done the assignment.}
  until getopt_return_val = -1;

  {Now |optind| is the index of first non-option on the command line.
   We must have one remaining argument.}
  if (optind + 1 <> argc) then begin
    write_ln (stderr, my_name, ': Need exactly one file argument.');
    usage (my_name);
  end;
end;
@y
@<Define \(p)|parse_arguments|@> =
procedure init_option_variables; {called by |parse_arguments|}
begin
@<Initialize the option variables@>;
@<Define the option table@>;
end;
@z

@x
@<Define the option...@> =
current_option := 0;
long_options[0].name := 'terse';
long_options[0].has_arg := 0;
long_options[0].flag := address_of (verbose);
long_options[0].val := 0;
incr (current_option);
@y
@<Define the option...@> =
@z

@x
@!verbose: c_int_type;
@y
@!verbose: boolean;
@z

@x
@<Define the option...@> =
long_options[current_option].name := 'min-crossrefs';
long_options[current_option].has_arg := 1;
long_options[current_option].flag := 0;
long_options[current_option].val := 0;
incr (current_option);
@y
@<Define the option...@> =
@z

@x
@<Define the option...@> =
long_options[current_option].name := 'help';
long_options[current_option].has_arg := 0;
long_options[current_option].flag := 0;
long_options[current_option].val := 0;
incr (current_option);
@y
@<Define the option...@> =
@z

@x
@<Define the option...@> =
long_options[current_option].name := 'version';
long_options[current_option].has_arg := 0;
long_options[current_option].flag := 0;
long_options[current_option].val := 0;
incr (current_option);
@y
@<Define the option...@> =
@z

@x
@<Define the option...@> =
long_options[current_option].name := 0;
long_options[current_option].has_arg := 0;
long_options[current_option].flag := 0;
long_options[current_option].val := 0;
@y
@<Define the option...@> =
@z

@x setup_bound_variable is the external function |setup_bound_value|.
@d setup_bound_var_end_end(#)==
  setup_bound_variable(address_of(#), bound_name, bound_default);
  if # < bound_default then # := bound_default
@y
@d setup_bound_var_end_end(#)==
  #:=setup_bound_value(bound_name, bound_default);
  if # < bound_default then # := bound_default
@z

@x
begin kpse_set_program_name (argv[0], 'bibtex');
@y
begin
@z

@x [the end] The main program's work, as procedures; the nonlocal gotos.
  incr (k);
  hash_prime := j;
  primes[k] := hash_prime;
  end;
end;
@y
  incr (k);
  hash_prime := j;
  primes[k] := hash_prime;
  end;
end;

@ The \.{.bst} loop of |main_part|, which |catch_bst_done| runs: a return is
cvtbib.sed's |hack2| (|break|), |jump_to_bst_done| its |longjmp(jmp32,1)|.

@<Procedures and functions for about everything@>=
procedure bst_loop;
label exit;
begin
loop
    begin
    if (not eat_bst_white_space) then   {the end of the \.{.bst} file}
        return;
    get_bst_command_and_process;
    end;
exit:
end;

@ What the main program did between |hack0| and |close_up_shop|, which
|catch_close_up_shop| runs: a nonlocal |goto close_up_shop| is the
|longjmp(jmp9998,1)| that leaves it.

@<Procedures and functions for about everything@>=
procedure main_part;
label exit_program @<Labels in the outer block@>;
begin
@<Read the \.{.aux} file@>;
@<Read and execute the \.{.bst} file@>;
exit_program:
end;
@z
