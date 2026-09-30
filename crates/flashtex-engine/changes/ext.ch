% changes/ext.ch -- the interface between pdftex.web and pdfTeX's C parts.
%
% In TeX Live, pdftex.web calls C routines (utils.c, mapfile.c, vfpacket.c,
% writefont.c, writeimg.c, writezip.c, avlstuff.c, tounicode.c, writet3.c,
% and the macros of pdftex.h) that web2c declares through pdftex.defines.
% Here they are declared as Pascal `external` routines, so that
% tools/web2rust translates every call with its real types, and the bodies
% are hand-written Rust in crates/flashtex-engine/src/pdftex/ (one module per
% C file). The macros of pdftex.h are ordinary Pascal here.
%
% The PDF output buffer is the one construct of pdftex.web that is not
% Pascal: |pdf_buf| is a C pointer that aliases either |pdf_op_buf| or
% |pdf_os_buf|. It becomes an explicit choice, |pdf_buf_is_os|, read and
% written through |pdf_buf_get| and |pdf_buf_set|; every access goes to the
% same byte as before.
%
% Everything kpathsea did for PK fonts (`kpse_init_prog`,
% `kpse_set_program_enabled`) becomes one call, |pk_init|, and the default
% map file name moves from the call of |pdf_init_map_file| into its Rust
% body, because a Pascal string cannot be passed to a Rust routine.
%
% New sections are added at the end of part 54, as tex.web asks, so that
% only the index is renumbered.
%
% get_kn_bs_code, get_st_bs_code and get_sh_bs_code read 0 where
% pdftex.h's macros would index pdf_mem outside its pdf_mem_size+1 entries
% (#1219). adjust_interword_glue can ask for that: when the space is the
% first item of a list, tail is the list's head node, a one-word node from
% get_avail that is_char_node takes for a character, so font(tail) and
% character(tail) are the two halves of whatever that word held before (a
% token, say; web2c.ch gives b0 and b1 texmfmem.h's layout: the font is
% the high half of lh, the character the low half). The font number is
% then below 4096, a real index, but pdf_font_kn_bs_base[f]+c can be up to
% 65535 past the font's code block. Inside pdf_mem the port reads what
% pdfTeX reads; past it pdfTeX's C reads whatever the heap holds there,
% which has no value to port, and 0 is a font without codes.
% tools/lockstep/cases/261, 264 and 265 hold both engines to the same box dumps.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.397 - the C routines are declared before everything else
@t\4@>@<Basic printing procedures@>@/
@y
@t\4@>@<Declare the routines of pdf\TeX's C parts@>@/
@t\4@>@<Basic printing procedures@>@/
@z

@x pdftex.web l.15367 - |pdf_buf| is not a pointer
    pdf_buf[pdf_ptr] := #;
@y
    pdf_buf_set(pdf_ptr, #);
@z

@x pdftex.web l.15390 - |pdf_buf| is not a pointer
@!pdf_buf: ^eight_bits; {pointer to the PDF output buffer or PDF object stream buffer}
@y
@!pdf_buf_is_os: boolean; {is the PDF output buffer the object stream buffer?}
@z

@x pdftex.web l.15434 - |pdf_buf| is not a pointer
pdf_buf := pdf_op_buf;
@y
pdf_buf_is_os := false;
@z

@x pdftex.web l.15552 - |pdf_buf| is not a pointer
                pdf_last_byte := pdf_buf[pdf_ptr - 1];
@y
                pdf_last_byte := pdf_buf_get(pdf_ptr - 1);
@z

@x pdftex.web l.15673 - |pdf_buf| is not a pointer
        pdf_buf := pdf_os_buf;
        pdf_buf_size := pdf_os_buf_size;
    end;
end;

procedure remove_last_space;
begin
    if (pdf_ptr > 0) and (pdf_buf[pdf_ptr - 1] = 32) then
@y
        pdf_buf_is_os := true;
        pdf_buf_size := pdf_os_buf_size;
    end;
end;

procedure remove_last_space;
begin
    if (pdf_ptr > 0) and (pdf_buf_get(pdf_ptr - 1) = 32) then
@z

@x pdftex.web l.16866 - |pdf_buf| is not a pointer
            pdf_buf := pdf_os_buf;
@y
            pdf_buf_is_os := true;
@z

@x pdftex.web l.16874 - |pdf_buf| is not a pointer
            pdf_buf := pdf_op_buf;
@y
            pdf_buf_is_os := false;
@z

@x pdftex.web l.16996 - |pdf_buf| is not a pointer
    pdf_buf[pdf_ptr - 1] := pdf_new_line_char; {no risk of flush, as we are in |pdf_os_mode|}
@y
    pdf_buf_set(pdf_ptr - 1, pdf_new_line_char); {no risk of flush, as we are in |pdf_os_mode|}
@z

@x pdftex.web l.19481 - |pdf_buf| is not a pointer
            if (not obj_obj_is_stream(n)) and (pdf_ptr > 0) and (pdf_buf[pdf_ptr - 1] <> 10) then
@y
            if (not obj_obj_is_stream(n)) and (pdf_ptr > 0) and (pdf_buf_get(pdf_ptr - 1) <> 10) then
@z

@x pdftex.web l.19836 - kpathsea's PK set-up is one external call
if pdf_pk_mode <> null then begin
    kpse_init_prog('PDFTEX', fixed_pk_resolution,
                   make_cstring(tokens_to_string(pdf_pk_mode)), nil);
    flush_string;
end else
    kpse_init_prog('PDFTEX', fixed_pk_resolution, nil, nil);
kpse_set_program_enabled (kpse_pk_format, 1, kpse_src_compile);
@y
if pdf_pk_mode <> null then begin
    pk_init(fixed_pk_resolution, tokens_to_string(pdf_pk_mode));
    flush_string;
end else
    pk_init(fixed_pk_resolution, 0);
@z

@x pdftex.web l.33594 - the default map file name is the Rust routine's
pdf_init_map_file('pdftex.map');
@y
pdf_init_map_file; {\.{pdftex.map}}
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ web2c's \.{texmfmp.h} makes |longinteger| a 64-bit integer; pdf\TeX\
uses it for byte offsets in the \.{PDF} file and in |print_int|.
\.{tools/web2rust} is told so by \.{--scalar longinteger=i64}.

@<Types...@>=
@!longinteger=integer; {a 64-bit integer}

@ These are the routines of pdf\TeX's C parts (\.{pdftex.defines},
\.{ptexlib.h}). Their bodies are in \.{src/pdftex/} of the engine crate.

@<Declare the routines of pdf\TeX's C parts@>=
{\.{utils.c}}
procedure pdfassert(@!b:boolean); external;
function ext_xn_over_d(@!x,@!n,@!d:scaled):scaled; external;
procedure escapestring(@!p:pool_pointer); external;
procedure escapename(@!p:pool_pointer); external;
procedure escapehex(@!p:pool_pointer); external;
procedure unescapehex(@!p:pool_pointer); external;
procedure getcreationdate; external;
procedure getfilemoddate(@!s:str_number); external;
procedure getfilesize(@!s:str_number); external;
procedure getmd5sum(@!s:str_number;@!is_file:boolean); external;
procedure getfiledump(@!s:str_number;@!offset,@!len:integer); external;
procedure matchstrings(@!s,@!t:str_number;@!subcount:integer;@!icase:boolean);
  external;
procedure getmatch(@!i:integer); external;
function get_resname_prefix:str_number; external;
procedure init_start_time; external;
procedure seconds_and_micros(var s,@!m:integer); external;
procedure date_and_time(var t,@!d,@!m,@!y:integer); external;
procedure set_job_id(@!y,@!m,@!d,@!t:integer); external;
procedure print_creation_date; external;
procedure print_mod_date; external;
procedure print_ID(@!s:str_number); external;
procedure print_ID_alt(@!s:str_number); external;
procedure write_stream_length(@!len,@!offset:longinteger); external;
procedure remove_pdffile; external;
procedure garbage_warning; external;
procedure libpdffinish; external;
function newcolorstack(@!s:str_number;@!literal_mode:integer;
  @!page_start:boolean):integer; external;
function colorstackused:integer; external;
function colorstackset(@!colstack_no:integer;@!s:str_number):integer; external;
function colorstackpush(@!colstack_no:integer;@!s:str_number):integer; external;
function colorstackpop(@!colstack_no:integer):integer; external;
function colorstackcurrent(@!colstack_no:integer):integer; external;
function colorstackskippagestart(@!colstack_no:integer):integer; external;
procedure checkpdfsave(@!cur_h,@!cur_v:scaled); external;
procedure checkpdfrestore(@!cur_h,@!cur_v:scaled); external;
procedure pdfshipoutbegin(@!shipping_page:boolean); external;
procedure pdfshipoutend(@!shipping_page:boolean); external;
function pdfsetmatrix(@!p:pool_pointer;@!cur_h,@!cur_v:scaled):integer; external;
procedure matrixtransformrect(@!llx,@!lly,@!urx,@!ury:scaled); external;
function matrixused:boolean; external;
procedure matrixrecalculate(@!urx:scaled); external;
function getllx:scaled; external;
function getlly:scaled; external;
function geturx:scaled; external;
function getury:scaled; external;
procedure allocvffnts; external;
{\.{vfpacket.c}}
function new_vf_packet(@!f:internal_font_number):integer; external;
procedure storepacket(@!f:internal_font_number;@!c:integer;@!s:str_number);
  external;
procedure start_packet(@!f:internal_font_number;@!c:eight_bits); external;
function packet_byte:eight_bits; external;
procedure push_packet_state; external;
procedure pop_packet_state; external;
{\.{mapfile.c}}
function hasfmentry(@!f:internal_font_number):boolean; external;
function isscalable(@!f:internal_font_number):boolean; external;
function hasspacechar(@!f:internal_font_number):boolean; external;
procedure pdfmapfile(@!t:integer); external;
procedure pdfmapline(@!t:integer); external;
procedure pdfmaplinesp; external;
procedure pdf_init_map_file; external;
procedure pk_init(@!resolution:integer;@!pk_mode:str_number); external;
{\.{writefont.c}, \.{writet3.c}, \.{tounicode.c}}
procedure do_pdf_font(@!n:integer;@!f:internal_font_number); external;
procedure write_fontstuff; external;
function get_pk_char_width(@!f:internal_font_number;@!w:scaled):scaled; external;
procedure def_tounicode(@!glyph,@!unistr:str_number); external;
procedure dumptounicode; external;
procedure undumptounicode; external;
{\.{writeimg.c} and the image readers}
function check_image_b(@!procset:integer):boolean; external;
function check_image_c(@!procset:integer):boolean; external;
function check_image_i(@!procset:integer):boolean; external;
function is_pdf_image(@!img:integer):boolean; external;
function is_png_image(@!img:integer):boolean; external;
function epdf_orig_x(@!img:integer):integer; external;
function epdf_orig_y(@!img:integer):integer; external;
function image_width(@!img:integer):integer; external;
function image_height(@!img:integer):integer; external;
function image_rotate(@!img:integer):integer; external;
function image_pages(@!img:integer):integer; external;
function image_x_res(@!img:integer):integer; external;
function image_y_res(@!img:integer):integer; external;
function image_colordepth(@!img:integer):integer; external;
function get_image_group_ref(@!img:integer):integer; external;
procedure set_image_group_ref(@!img,@!ref:integer); external;
function read_image(@!s:str_number;@!page:integer;@!page_name:str_number;
  @!colorspace,@!page_box,@!major,@!minor,@!errorlevel:integer):integer;
  external;
procedure delete_image(@!img:integer); external;
procedure update_image_procset(@!img:integer); external;
procedure write_image(@!img:integer); external;
procedure dumpimagemeta; external;
procedure undumpimagemeta(@!major,@!minor,@!errorlevel:integer); external;
procedure flush_jbig2_page0_objects; external;
{\.{writezip.c}, \.{avlstuff.c}}
procedure write_zip(@!finish:boolean); external;
procedure avl_put_obj(@!objptr,@!t:integer); external;
function avl_find_obj(@!t,@!i,@!byname:integer):integer; external;
{\.{pdftex.h}: the output file and \.{kpathsea}}
procedure write_pdf(@!a,@!b:integer); external;
function getc(var f:byte_file):integer; external;
function tex_b_openin(var f:byte_file):boolean; external;
function vf_b_open_in(var f:byte_file):boolean; external;

@ The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
external declarations and before the basic printing procedures.

@<Declare the routines of pdf\TeX's C parts@>=
function pdf_char_bit(@!c:eight_bits):integer; {C's |1<<(c%8)|}
var k,@!b:integer;
begin b:=1; for k:=1 to c mod 8 do b:=b+b;
pdf_char_bit:=b;
end;
@#
function pdf_char_marked(@!f:internal_font_number;@!c:eight_bits):boolean;
begin pdf_char_marked:=odd(pdf_char_used[f,c div 8] div pdf_char_bit(c));
end;
@#
procedure pdf_mark_char(@!f:internal_font_number;@!c:eight_bits);
begin if not pdf_char_marked(f,c) then
  pdf_char_used[f,c div 8]:=pdf_char_used[f,c div 8]+pdf_char_bit(c);
end;
@#
function get_lp_code(@!f:internal_font_number;@!c:eight_bits):integer;
begin if pdf_font_lp_base[f]=0 then get_lp_code:=0
else get_lp_code:=pdf_mem[pdf_font_lp_base[f]+c];
end;
@#
function get_rp_code(@!f:internal_font_number;@!c:eight_bits):integer;
begin if pdf_font_rp_base[f]=0 then get_rp_code:=0
else get_rp_code:=pdf_mem[pdf_font_rp_base[f]+c];
end;
@#
function get_ef_code(@!f:internal_font_number;@!c:eight_bits):integer;
begin if pdf_font_ef_base[f]=0 then get_ef_code:=1000
else get_ef_code:=pdf_mem[pdf_font_ef_base[f]+c];
end;
@#
function get_kn_bs_code(@!f:internal_font_number;@!c:integer):integer;
var i:integer;
begin i:=pdf_font_kn_bs_base[f];
if i=0 then get_kn_bs_code:=0
else begin i:=i+c;
  if (i<0)or(i>pdf_mem_size) then get_kn_bs_code:=0
  else get_kn_bs_code:=pdf_mem[i];
  end;
end;
@#
function get_st_bs_code(@!f:internal_font_number;@!c:integer):integer;
var i:integer;
begin i:=pdf_font_st_bs_base[f];
if i=0 then get_st_bs_code:=0
else begin i:=i+c;
  if (i<0)or(i>pdf_mem_size) then get_st_bs_code:=0
  else get_st_bs_code:=pdf_mem[i];
  end;
end;
@#
function get_sh_bs_code(@!f:internal_font_number;@!c:integer):integer;
var i:integer;
begin i:=pdf_font_sh_bs_base[f];
if i=0 then get_sh_bs_code:=0
else begin i:=i+c;
  if (i<0)or(i>pdf_mem_size) then get_sh_bs_code:=0
  else get_sh_bs_code:=pdf_mem[i];
  end;
end;
@#
function get_kn_bc_code(@!f:internal_font_number;@!c:eight_bits):integer;
begin if pdf_font_kn_bc_base[f]=0 then get_kn_bc_code:=0
else get_kn_bc_code:=pdf_mem[pdf_font_kn_bc_base[f]+c];
end;
@#
function get_kn_ac_code(@!f:internal_font_number;@!c:eight_bits):integer;
begin if pdf_font_kn_ac_base[f]=0 then get_kn_ac_code:=0
else get_kn_ac_code:=pdf_mem[pdf_font_kn_ac_base[f]+c];
end;
@#
function pdf_buf_get(@!i:integer):eight_bits;
begin if pdf_buf_is_os then pdf_buf_get:=pdf_os_buf[i]
else pdf_buf_get:=pdf_op_buf[i];
end;
@#
procedure pdf_buf_set(@!i:integer;@!b:eight_bits);
begin if pdf_buf_is_os then pdf_os_buf[i]:=b
else pdf_op_buf[i]:=b;
end;

@* \[55] Index.
@z
