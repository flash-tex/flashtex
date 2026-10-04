% changes/ext.ch -- the interface between xetex.web and XeTeX's C parts.
%
% In TeX Live, xetex.web calls C and C++ routines (XeTeX_ext.c,
% XeTeX_pic.c, XeTeXOTMath.cpp, XeTeXLayoutInterface.cpp, trans.c, and the
% macros of xetex.h) that web2c declares through xetexdir/xetex.defines.
% Here they are declared as Pascal `external' routines, so that
% tools/web2rust translates every call with its real types, and the bodies
% are hand-written Rust in crates/flashtex-xetex/src/xetex_ext.rs (XeTeX's
% own code is MIT-licensed, so its C may be ported freely).
%
% xetex.web is closer to C than pdftex.web: it keeps C pointers in |mem|
% and in globals, and passes addresses. Each such construct becomes an
% integer handle or a `var' parameter here (docs/design/xetex/PLAN.md):
%
%   * |void_pointer| is an integer, a handle into a table of the Rust side,
%     0 (|nil|, |null_ptr|) for none: a font's layout engine and TECkit
%     mapping, an OpenType assembly, a node's glyph-info array (in the
%     |int| half of the word that holds the C pointer in TeX Live);
%   * |addressof(x)| as an argument is |x| passed to a `var' parameter
%     (tools/web2rust accepts the form);
%   * C string arguments (|name_of_file+1|, |native_text+s|) become the
%     global they point into, plus an offset where there is one;
%   * |sizeof(UTF16_code)| and |sizeof(memory_word)| are 2 and 8;
%   * a picture's path is copied into |mem| by the Rust side.
%
% Phase S0 (TFM fonts only) stubs every routine that serves native fonts,
% graphics and TECkit: no installed font is ever found, so no native font,
% node or mapping exists, and no picture file is ever found.
%
% New sections are added at the end of part 54, as tex.web asks.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.431 - the C routines are declared before everything else
@t\4@>@<Basic printing procedures@>@/
@y
@t\4@>@<Declare the routines of \XeTeX's C parts@>@/
@t\4@>@<Basic printing procedures@>@/
@z

@x xetex.web l.1735 - |native_text| is a growable array of the word space
    native_text:=xrealloc(native_text, native_text_size * sizeof(UTF16_code));
@y
    native_text:=xrealloc_array(native_text, UTF16_code, native_text_size);
@z

@x xetex.web l.1749 - |native_text| is a growable array of the word space
native_text:=xmalloc(native_text_size * sizeof(UTF16_code));
@y
native_text:=xmalloc_array(UTF16_code, native_text_size);
@z

@x xetex.web l.3726 - a glyph-info array is a handle (xetex.h's |native_glyph_info_ptr|)
@d native_glyph_info_ptr(#)==mem[#+5].ptr
@y
@d native_glyph_info_ptr(#)==mem[#+5].int {a handle, 0 for none}
@z

@x xetex.web l.3733 - a glyph-info array is a handle
      libc_free(native_glyph_info_ptr(#));
@y
      free_glyph_info(native_glyph_info_ptr(#));
@z

@x xetex.web l.3744 - a glyph-info array is a handle
    native_glyph_info_ptr(dest):=xmalloc_array(char, glyph_count * native_glyph_info_size);
    memcpy(native_glyph_info_ptr(dest), native_glyph_info_ptr(src), glyph_count * native_glyph_info_size);
@y
    native_glyph_info_ptr(dest):=copy_glyph_info(native_glyph_info_ptr(src));
@z

@x xetex.web l.12911 - xetex.ch [30.549]: character codes of native fonts need 16 bits
@!font_bc:array[internal_font_number] of eight_bits;
  {beginning (smallest) character code}
@!font_ec:array[internal_font_number] of eight_bits;
  {ending (largest) character code}
@y
@!font_bc:array[internal_font_number] of UTF16_code;
  {beginning (smallest) character code}
@!font_ec:array[internal_font_number] of UTF16_code;
  {ending (largest) character code}
@z

@x xetex.web l.12928 - xetex.ch [30.549]: the font arrays of native fonts
@!font_false_bchar:array[internal_font_number] of min_quarterword..non_char;
  {|font_bchar| if it doesn't exist in the font, otherwise |non_char|}
@y
@!font_false_bchar:array[internal_font_number] of min_quarterword..non_char;
  {|font_bchar| if it doesn't exist in the font, otherwise |non_char|}
@#
@!font_layout_engine: array[internal_font_number] of void_pointer;
  {a handle to a layout engine, 0 for a \.{TFM} font}
@!font_mapping: array[internal_font_number] of void_pointer;
  {a handle to a \.{TECkit} mapping, or 0}
@!font_flags: array[internal_font_number] of integer; { flags:
  0x01: |font_colored|
  0x02: |font_vertical| }
@!font_letter_space: array[internal_font_number] of scaled;
  { letterspacing to be applied to the font }
@!loaded_font_mapping: void_pointer; { used by |load_native_font| to return mapping, if any }
@!loaded_font_flags: integer; { used by |load_native_font| to return flags }
@!loaded_font_letter_space: scaled;
@!loaded_font_design_size: scaled;
@!mapped_text: ^UTF16_code; { scratch buffer used while applying font mappings }
@!xdv_buffer: ^eight_bits; { scratch buffer used in generating XDV output }
@z

@x xetex.web l.13116 - |print_c_string(name_of_file+1)| is a routine
  print_c_string(stringcast(name_of_file+1));
@y
  print_name_of_file_c;
@z

@x xetex.web l.13157 - |print_c_string(name_of_file+1)| is a routine
    print_c_string(stringcast(name_of_file+1));
@y
    print_name_of_file_c;
@z

@x xetex.web l.16600 - |sizeof(UTF16_code)=2|, |sizeof(memory_word)=8|
  l:=native_node_size + (n * sizeof(UTF16_code) + sizeof(memory_word) - 1) div sizeof(memory_word);
@y
  l:=native_node_size + (n * 2 + 8 - 1) div 8;
@z

@x xetex.web l.16626 - a TECkit mapping is applied to a range of the pool
    len:=apply_mapping(font_mapping[f], addressof(str_pool[str_start_macro(str_ptr)]), cur_length);
@y
    len:=apply_mapping_pool(font_mapping[f], str_start_macro(str_ptr), cur_length);
@z

@x xetex.web l.16682 - a C string is a handle
procedure font_feature_warning(featureNameP:void_pointer; featLen:integer;
  settingNameP:void_pointer; setLen:integer);
@y
procedure font_feature_warning(@!featureNameP:void_pointer; @!featLen:integer;
  @!settingNameP:void_pointer; @!setLen:integer);
@z

@x xetex.web l.16765 - |find_native_font| reads |name_of_file| itself
  font_engine:=find_native_font(name_of_file + 1, s);
@y
  font_engine:=find_native_font(s);
@z

@x xetex.web l.16890 - the text to break is |native_text[s..s+len-1]|
    linebreak_start(main_f, XeTeX_linebreak_locale, native_text + s, len);
@y
    linebreak_start(main_f, XeTeX_linebreak_locale, s, len);
@z

@x xetex.web l.16927 - |eqtb| is always there
  if eqtb=nil then get_input_normalization_state:=0 { may be called before eqtb is initialized }
  else get_input_normalization_state:=XeTeX_input_normalization_state;
@y
  get_input_normalization_state:=XeTeX_input_normalization_state;
@z

@x xetex.web l.24432 - a TECkit mapping is applied to |native_text|
    main_k:=apply_mapping(font_mapping[main_f], native_text, native_len);
@y
    main_k:=apply_mapping_native(font_mapping[main_f], native_len);
@z

@x xetex.web l.29981 - |sizeof(memory_word)=8|
@d total_pic_node_size(#) == (pic_node_size + (pic_path_length(#) + sizeof(memory_word) - 1) div sizeof(memory_word))
@y
@d total_pic_node_size(#) == (pic_node_size + (pic_path_length(#) + 8 - 1) div 8)
@z

@x xetex.web l.30586 - a picture's path is a handle
  pic_path: ^char;
@y
  pic_path: integer; {a handle to the path |find_pic_file| found}
@z

@x xetex.web l.30703 - a picture's path is a handle; |sizeof(memory_word)=8|
    new_whatsit(pic_node, pic_node_size + (strlen(pic_path) + sizeof(memory_word) - 1) div sizeof(memory_word));
@y
    new_whatsit(pic_node, pic_node_size + (pic_path_len(pic_path) + 8 - 1) div 8);
@z

@x xetex.web l.30707 - a picture's path is a handle
    pic_path_length(tail):=strlen(pic_path);
@y
    pic_path_length(tail):=pic_path_len(pic_path);
@z

@x xetex.web l.30722 - a picture's path is copied into |mem| by the Rust side
    memcpy(addressof(mem[tail + pic_node_size]), pic_path, strlen(pic_path));
    libc_free(pic_path);
@y
    pic_path_to_mem(pic_path, tail + pic_node_size);
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ A C pointer is an integer handle here (see the head of \.{changes/ext.ch});
these are \.{xetex.h}'s and \.{xetex.defines}' names for it.

@d nil==0 {no handle}
@d null_ptr==0 {no handle}
@d kpse_tex_format=26 {kpathsea's |kpse_tex_format|, for |u_open_in|}

@<Types...@>=
@!void_pointer=integer; {a handle into a table of the Rust side, 0 for none}
@!unicode_file=packed file of text_char;
  {a \.{UFILE}: |input_ln| decodes it as |u_open_in| or
   |set_input_file_encoding| says (src/system.rs)}
@!real_point=record@;@/
  @!x,@!y:real;
  end;
@!real_rect=record@;@/
  @!x,@!y,@!wd,@!ht:real;
  end;
@!transform=record@;@/
  @!a,@!b,@!c,@!d,@!x,@!y:real;
  end;

@ The fields of \.{trans.h}'s types, which \.{xetex.h} reads with macros.

@d xCoord(#)==#.x
@d yCoord(#)==#.y
@d xField(#)==#.x
@d yField(#)==#.y
@d wdField(#)==#.wd
@d htField(#)==#.ht
@d aField(#)==#.a
@d bField(#)==#.b
@d cField(#)==#.c
@d dField(#)==#.d


@ Picture and \.{TECkit} scratch buffers, and the \.{XDV} buffer, are
allocated like the other arrays of web2c.

@<Allocate the arrays that web2c allocates@>=
mapped_text:=xmalloc_array(UTF16_code, mapped_text_size);
xdv_buffer:=xmalloc_array(eight_bits, xdv_buffer_size);

@ @d mapped_text_size=1048576 {the largest number of |mapped_text| entries}
@d xdv_buffer_size=1048576 {the largest number of |xdv_buffer| bytes}

@ These are the routines of \XeTeX's C parts (\.{xetex.defines},
\.{XeTeX\_ext.h}, \.{XeTeXLayoutInterface.h}, \.{XeTeXOTMath.h},
\.{trans.h}). Their bodies are in \.{src/xetex\_ext.rs}.

@<Declare the routines of \XeTeX's C parts@>=
{\.{XeTeX\_ext.c}: Unicode files}
function u_open_in(var f:unicode_file;@!filefmt,@!in_mode,@!encoding_data:integer):boolean;
  external;
procedure u_close(var f:unicode_file); external;
procedure set_input_file_encoding(var f:unicode_file;@!in_mode,@!encoding_data:integer);
  external;
function get_encoding_mode_and_info(var enc_info:integer):integer; external;
procedure make_utf16_name; external;
procedure print_name_of_file_c; external;
  {texmfmp.c's |printcstring(nameoffile+1)|}
procedure print_utf8_str(@!s:void_pointer;@!len:integer); external;
function load_pool_strings(@!spare_size:integer):str_number; external;
  {texmfmp.c's |loadpoolstrings| (tex-binpool.ch)}
{\.{XeTeX\_ext.c}: the \.{XDV} file}
function dvi_open_out(var f:byte_file):boolean; external;
function dvi_close(var f:byte_file):integer; external;
procedure print_strerror(@!k:integer); external;
procedure fflush(var f:byte_file); external; {C's |fflush|}
{\.{XeTeX\_ext.c}: native fonts}
function find_native_font(@!s:scaled):void_pointer; external;
procedure release_font_engine(@!engine:void_pointer;@!type_flag:integer); external;
procedure ot_get_font_metrics(@!engine:void_pointer;
  var f_ascent,@!f_descent,@!f_xheight,@!f_capheight,@!f_slant:scaled); external;
procedure aat_get_font_metrics(@!engine:void_pointer;
  var f_ascent,@!f_descent,@!f_xheight,@!f_capheight,@!f_slant:scaled); external;
function make_font_def(@!f:internal_font_number):integer; external;
function make_xdv_glyph_array_data(@!p:pointer):integer; external;
function xdv_buffer_byte(@!k:integer):integer; external;
function get_native_char(@!p:pointer;@!i:integer):integer; external;
function get_native_usv(@!p:pointer;@!i:integer):integer; external;
procedure set_native_char(@!p:pointer;@!i,@!c:integer); external;
function get_native_glyph(@!p:pointer;@!i:integer):integer; external;
procedure set_native_metrics(@!p:pointer;@!use_glyph_metrics:boolean); external;
procedure set_justified_native_glyphs(@!p:pointer); external;
procedure set_native_glyph_metrics(@!p:pointer;@!use_glyph_metrics:boolean); external;
function get_native_italic_correction(@!p:pointer):scaled; external;
function get_native_glyph_italic_correction(@!p:pointer):scaled; external;
procedure get_native_char_height_depth(@!f:internal_font_number;@!c:integer;
  var h,@!d:scaled); external;
procedure get_native_char_sidebearings(@!f:internal_font_number;@!c:integer;
  var lsb,@!rsb:scaled); external;
function getnativecharwd(@!f:internal_font_number;@!c:integer):scaled; external;
function getnativecharht(@!f:internal_font_number;@!c:integer):scaled; external;
function getnativechardp(@!f:internal_font_number;@!c:integer):scaled; external;
function getnativecharic(@!f:internal_font_number;@!c:integer):scaled; external;
function get_glyph_bounds(@!f:internal_font_number;@!edge,@!gid:integer):scaled;
  external;
function map_char_to_glyph(@!f:internal_font_number;@!c:integer):integer; external;
function map_glyph_to_index(@!f:internal_font_number):integer; external;
function get_font_char_range(@!f:internal_font_number;@!first:boolean):integer;
  external;
procedure print_glyph_name(@!f:internal_font_number;@!gid:integer); external;
function get_native_word_cp(@!p:pointer;@!side:integer):integer; external;
function get_cp_code(@!f:internal_font_number;@!c,@!side:integer):integer; external;
procedure set_cp_code(@!f:internal_font_number;@!c,@!side,@!v:integer); external;
function usingOpenType(@!engine:void_pointer):boolean; external;
function usingGraphite(@!engine:void_pointer):boolean; external;
function isOpenTypeMathFont(@!engine:void_pointer):boolean; external;
function aat_font_get(@!what:integer;@!engine:void_pointer):integer; external;
function aat_font_get_1(@!what:integer;@!engine:void_pointer;@!p:integer):integer;
  external;
function aat_font_get_2(@!what:integer;@!engine:void_pointer;@!p1,@!p2:integer):integer;
  external;
function aat_font_get_named(@!what:integer;@!engine:void_pointer):integer; external;
function aat_font_get_named_1(@!what:integer;@!engine:void_pointer;@!p:integer):integer;
  external;
procedure aat_print_font_name(@!what:integer;@!engine:void_pointer;@!p1,@!p2:integer);
  external;
function ot_font_get(@!what:integer;@!engine:void_pointer):integer; external;
function ot_font_get_1(@!what:integer;@!engine:void_pointer;@!p:integer):integer;
  external;
function ot_font_get_2(@!what:integer;@!engine:void_pointer;@!p1,@!p2:integer):integer;
  external;
function ot_font_get_3(@!what:integer;@!engine:void_pointer;@!p1,@!p2,@!p3:integer):integer;
  external;
function gr_font_get_named(@!what:integer;@!engine:void_pointer):integer; external;
function gr_font_get_named_1(@!what:integer;@!engine:void_pointer;@!p:integer):integer;
  external;
procedure gr_print_font_name(@!what:integer;@!engine:void_pointer;@!p1,@!p2:integer);
  external;
procedure linebreak_start(@!f:internal_font_number;@!locale_str_num:integer;
  @!s,@!len:integer); external;
  {break |native_text[s..s+len-1]|}
function linebreak_next:integer; external;
procedure terminate_font_manager; external;
{\.{XeTeX\_ext.c}: \.{TECkit} mappings}
procedure check_for_tfm_font_mapping; external;
function load_tfm_font_mapping:void_pointer; external;
function apply_tfm_font_mapping(@!m:void_pointer;@!c:integer):integer; external;
function apply_mapping_pool(@!m:void_pointer;@!s,@!len:integer):integer; external;
  {the mapping of |str_pool[s..s+len-1]| into |mapped_text|}
function apply_mapping_native(@!m:void_pointer;@!len:integer):integer; external;
  {the mapping of |native_text[0..len-1]| into |mapped_text|}
{glyph-info arrays}
procedure free_glyph_info(@!h:void_pointer); external;
function copy_glyph_info(@!h:void_pointer):void_pointer; external;
{\.{XeTeXOTMath.cpp}}
function get_native_mathsy_param(@!f:internal_font_number;@!n:integer):scaled;
  external;
function get_native_mathex_param(@!f:internal_font_number;@!n:integer):scaled;
  external;
function get_ot_math_constant(@!f:internal_font_number;@!n:integer):scaled; external;
function get_ot_math_variant(@!f:internal_font_number;@!g,@!v:integer;
  var adv:integer;@!horiz:integer):integer; external;
function get_ot_assembly_ptr(@!f:internal_font_number;@!g,@!horiz:integer):void_pointer;
  external;
procedure free_ot_assembly(@!a:void_pointer); external;
function get_ot_math_ital_corr(@!f:internal_font_number;@!g:integer):scaled;
  external;
function get_ot_math_accent_pos(@!f:internal_font_number;@!g:integer):scaled;
  external;
function get_ot_math_kern(@!f:internal_font_number;@!g:integer;
  @!sf:internal_font_number;@!sg,@!cmd,@!shift:integer):scaled; external;
function ot_part_count(@!a:void_pointer):integer; external;
function ot_part_glyph(@!a:void_pointer;@!i:integer):integer; external;
function ot_part_is_extender(@!a:void_pointer;@!i:integer):boolean; external;
function ot_part_start_connector(@!f:internal_font_number;@!a:void_pointer;
  @!i:integer):integer; external;
function ot_part_end_connector(@!f:internal_font_number;@!a:void_pointer;
  @!i:integer):integer; external;
function ot_part_full_advance(@!f:internal_font_number;@!a:void_pointer;
  @!i:integer):integer; external;
function ot_min_connector_overlap(@!f:internal_font_number):integer; external;
{\.{xetex.h}: the bit fields of a 32-bit math code, read as C reads an
 |unsigned|; C's |(unsigned short)|}
function math_fam_field(@!x:integer):integer; external;
function math_class_field(@!x:integer):integer; external;
function math_char_field(@!x:integer):integer; external;
function set_family_field(@!x:integer):integer; external;
function set_class_field(@!x:integer):integer; external;
function cast_to_ushort(@!x:integer):integer; external;
{\.{XeTeX\_pic.c} and \.{trans.c}}
procedure setPoint(var p:real_point;@!x,@!y:real); external;
function find_pic_file(var path:integer; var bounds:real_rect;
  @!pdf_box_type,@!page:integer):integer; external;
function pic_path_len(@!path:integer):integer; external;
procedure pic_path_to_mem(@!path:integer;@!p:pointer); external;
  {copy the path's bytes into |mem| from word |p| on, and forget it}
function pic_path_byte(@!p:pointer;@!i:integer):integer; external;
function count_pdf_file_pages:integer; external;
function D2Fix(@!d:real):scaled; external;
function Fix2D(@!f:scaled):real; external;
procedure make_identity(var t:transform); external;
procedure make_scale(var t:transform;@!xscale,@!yscale:real); external;
procedure make_translation(var t:transform;@!dx,@!dy:real); external;
procedure make_rotation(var t:transform;@!a:real); external;
procedure transform_point(var p:real_point; var t:transform); external;
procedure transform_concat(var t1:transform; var t2:transform); external;
{\.{texmfmp.c} and \.{utils.c}: time, files, \MD5}
procedure init_start_time; external;
procedure seconds_and_micros(var s,@!m:integer); external;
procedure date_and_time(var t,@!d,@!m,@!y:integer); external;
procedure getcreationdate; external;
procedure getfilemoddate(@!s:str_number); external;
procedure getfilesize(@!s:str_number); external;
procedure getfiledump(@!s:str_number;@!offset,@!len:integer); external;
procedure getmd5sum(@!s:str_number;@!is_file:boolean); external;

@* \[55] Index.
@z
