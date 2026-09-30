// GENERATED FILE -- DO NOT EDIT.
// All WEB globals in one arena struct (DESIGN.md §4.2).
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, unused_comparisons, clippy::all)]

use super::consts::*;
use super::types::*;

pub struct Globals {
    // §13
    pub bad: i32,
    // §20
    pub xord: crate::arena::Arr<ASCII_code>,
    // §20
    pub xchr: [u8; 256],
    // §20
    pub xprn: crate::arena::Arr<bool>,
    // §26
    pub name_of_file: [u8; 1024],
    // §26
    pub name_length: i32,
    // §30
    pub buffer: crate::arena::Arr<ASCII_code>,
    // §30
    pub first: i32,
    // §30
    pub last: i32,
    // §30
    pub max_buf_stack: i32,
    // §32
    pub term_in: crate::system::AlphaFile,
    // §32
    pub term_out: crate::system::AlphaFile,
    // §39
    pub str_pool: crate::arena::Arr<packed_ASCII_code>,
    // §39
    pub str_start: crate::arena::Arr<pool_pointer>,
    // §39
    pub pool_ptr: pool_pointer,
    // §39
    pub str_ptr: str_number,
    // §39
    pub init_pool_ptr: pool_pointer,
    // §39
    pub init_str_ptr: str_number,
    // §50
    pub pool_file: crate::system::AlphaFile,
    // §54
    pub log_file: crate::system::AlphaFile,
    // §54
    pub selector: i32,
    // §54
    pub dig: crate::arena::Arr<i32>,
    // §54
    pub tally: i32,
    // §54
    pub term_offset: i32,
    // §54
    pub file_offset: i32,
    // §54
    pub trick_buf: crate::arena::Arr<ASCII_code>,
    // §54
    pub trick_count: i32,
    // §54
    pub first_count: i32,
    // §73
    pub interaction: i32,
    // §73
    pub interaction_option: i32,
    // §76
    pub deletions_allowed: bool,
    // §76
    pub set_box_allowed: bool,
    // §76
    pub history: i32,
    // §76
    pub error_count: i32,
    // §79
    pub help_line: crate::arena::Arr<str_number>,
    // §79
    pub help_ptr: i32,
    // §79
    pub use_err_help: bool,
    // §96
    pub interrupt: i32,
    // §96
    pub OK_to_interrupt: bool,
    // §104
    pub arith_error: bool,
    // §104
    pub remainder: scaled,
    // §110
    pub randoms: crate::arena::Arr<i32>,
    // §110
    pub j_random: i32,
    // §110
    pub random_seed: scaled,
    // §117
    pub two_to_the: crate::arena::Arr<i32>,
    // §117
    pub spec_log: crate::arena::Arr<i32>,
    // §133
    pub temp_ptr: halfword,
    // §134
    pub mem: crate::arena::Arr<memory_word>,
    // §134
    pub lo_mem_max: halfword,
    // §134
    pub hi_mem_min: halfword,
    // §135
    pub var_used: i32,
    // §135
    pub dyn_used: i32,
    // §136
    pub avail: halfword,
    // §136
    pub mem_end: halfword,
    // §142
    pub rover: halfword,
    // §191
    pub font_in_short_display: i32,
    // §199
    pub depth_threshold: i32,
    // §199
    pub breadth_max: i32,
    // §231
    pub nest: crate::arena::Arr<list_state_record>,
    // §231
    pub nest_ptr: i32,
    // §231
    pub max_nest_stack: i32,
    // §231
    pub cur_list: list_state_record,
    // §231
    pub shown_mode: i32,
    // §231
    pub save_tail: halfword,
    // §231
    pub prev_tail: halfword,
    // §264
    pub old_setting: i32,
    // §264
    pub old_selector_ignored_err: i32,
    // §264
    pub sys_time: i32,
    // §264
    pub sys_day: i32,
    // §264
    pub sys_month: i32,
    // §264
    pub sys_year: i32,
    // §271
    pub eqtb: crate::arena::Arr<memory_word>,
    // §271
    pub xeq_level: crate::arena::Arr<quarterword>,
    // §274
    pub hash: crate::arena::Arr<two_halves>,
    // §274
    pub hash_used: halfword,
    // §274
    pub hash_high: halfword,
    // §274
    pub no_new_control_sequence: bool,
    // §274
    pub cs_count: i32,
    // §275
    pub prim: crate::arena::Arr<two_halves>,
    // §275
    pub prim_used: halfword,
    // §293
    pub save_stack: crate::arena::Arr<memory_word>,
    // §293
    pub save_ptr: i32,
    // §293
    pub max_save_stack: i32,
    // §293
    pub cur_level: quarterword,
    // §293
    pub cur_group: group_code,
    // §293
    pub cur_boundary: i32,
    // §308
    pub mag_set: i32,
    // §319
    pub cur_cmd: eight_bits,
    // §319
    pub cur_chr: halfword,
    // §319
    pub cur_cs: halfword,
    // §319
    pub cur_tok: halfword,
    // §323
    pub input_stack: crate::arena::Arr<in_state_record>,
    // §323
    pub input_ptr: i32,
    // §323
    pub max_in_stack: i32,
    // §323
    pub cur_input: in_state_record,
    // §326
    pub in_open: i32,
    // §326
    pub open_parens: i32,
    // §326
    pub input_file: Vec<crate::system::AlphaFile>,
    // §326
    pub line: i32,
    // §326
    pub line_stack: crate::arena::Arr<i32>,
    // §327
    pub scanner_status: i32,
    // §327
    pub warning_index: halfword,
    // §327
    pub def_ref: halfword,
    // §330
    pub param_stack: crate::arena::Arr<halfword>,
    // §330
    pub param_ptr: i32,
    // §330
    pub max_param_stack: i32,
    // §331
    pub align_state: i32,
    // §332
    pub base_ptr: i32,
    // §355
    pub par_loc: halfword,
    // §355
    pub par_token: halfword,
    // §383
    pub force_eof: bool,
    // §389
    pub is_in_csname: bool,
    // §408
    pub cur_mark: crate::arena::Arr<halfword>,
    // §413
    pub long_state: i32,
    // §414
    pub pstack: crate::arena::Arr<halfword>,
    // §436
    pub cur_val: i32,
    // §436
    pub cur_val_level: i32,
    // §464
    pub radix: small_number,
    // §473
    pub cur_order: glue_ord,
    // §506
    pub read_file: Vec<crate::system::AlphaFile>,
    // §506
    pub read_open: crate::arena::Arr<i32>,
    // §515
    pub cond_ptr: halfword,
    // §515
    pub if_limit: i32,
    // §515
    pub cur_if: small_number,
    // §515
    pub if_line: i32,
    // §519
    pub skip_line: i32,
    // §538
    pub cur_name: str_number,
    // §538
    pub cur_area: str_number,
    // §538
    pub cur_ext: str_number,
    // §539
    pub area_delimiter: pool_pointer,
    // §539
    pub ext_delimiter: pool_pointer,
    // §539
    pub quoted_filename: bool,
    // §539
    pub stop_at_space: bool,
    // §539
    pub full_source_filename_stack: crate::arena::Arr<str_number>,
    // §546
    pub TEX_format_default: [u8; 20],
    // §553
    pub name_in_progress: bool,
    // §553
    pub job_name: str_number,
    // §553
    pub log_opened: bool,
    // §558
    pub dvi_file: crate::system::ByteFile,
    // §558
    pub output_file_name: str_number,
    // §558
    pub log_name: str_number,
    // §565
    pub tfm_file: crate::system::ByteFile,
    // §575
    pub font_info: crate::arena::Arr<memory_word>,
    // §575
    pub fmem_ptr: font_index,
    // §575
    pub font_ptr: internal_font_number,
    // §575
    pub font_check: crate::arena::Arr<four_quarters>,
    // §575
    pub font_size: crate::arena::Arr<scaled>,
    // §575
    pub font_dsize: crate::arena::Arr<scaled>,
    // §575
    pub font_params: crate::arena::Arr<font_index>,
    // §575
    pub font_name: crate::arena::Arr<str_number>,
    // §575
    pub font_area: crate::arena::Arr<str_number>,
    // §575
    pub font_bc: crate::arena::Arr<eight_bits>,
    // §575
    pub font_ec: crate::arena::Arr<eight_bits>,
    // §575
    pub font_glue: crate::arena::Arr<halfword>,
    // §575
    pub font_used: crate::arena::Arr<bool>,
    // §575
    pub hyphen_char: crate::arena::Arr<i32>,
    // §575
    pub skew_char: crate::arena::Arr<i32>,
    // §575
    pub bchar_label: crate::arena::Arr<font_index>,
    // §575
    pub font_bchar: crate::arena::Arr<i32>,
    // §575
    pub font_false_bchar: crate::arena::Arr<i32>,
    // §576
    pub char_base: crate::arena::Arr<i32>,
    // §576
    pub width_base: crate::arena::Arr<i32>,
    // §576
    pub height_base: crate::arena::Arr<i32>,
    // §576
    pub depth_base: crate::arena::Arr<i32>,
    // §576
    pub italic_base: crate::arena::Arr<i32>,
    // §576
    pub lig_kern_base: crate::arena::Arr<i32>,
    // §576
    pub kern_base: crate::arena::Arr<i32>,
    // §576
    pub exten_base: crate::arena::Arr<i32>,
    // §576
    pub param_base: crate::arena::Arr<i32>,
    // §581
    pub null_character: four_quarters,
    // §619
    pub total_pages: i32,
    // §619
    pub max_v: scaled,
    // §619
    pub max_h: scaled,
    // §619
    pub max_push: i32,
    // §619
    pub last_bop: i32,
    // §619
    pub dead_cycles: i32,
    // §619
    pub doing_leaders: bool,
    // §619
    pub c: quarterword,
    // §619
    pub f: quarterword,
    // §619
    pub rule_ht: scaled,
    // §619
    pub rule_dp: scaled,
    // §619
    pub rule_wd: scaled,
    // §619
    pub g: halfword,
    // §619
    pub lq: i32,
    // §619
    pub lr: i32,
    // §622
    pub dvi_buf: crate::arena::Arr<eight_bits>,
    // §622
    pub half_buf: dvi_index,
    // §622
    pub dvi_limit: dvi_index,
    // §622
    pub dvi_ptr: dvi_index,
    // §622
    pub dvi_offset: i32,
    // §622
    pub dvi_gone: i32,
    // §632
    pub down_ptr: halfword,
    // §632
    pub right_ptr: halfword,
    // §643
    pub dvi_h: scaled,
    // §643
    pub dvi_v: scaled,
    // §643
    pub cur_h: scaled,
    // §643
    pub cur_v: scaled,
    // §643
    pub dvi_f: internal_font_number,
    // §643
    pub cur_s: i32,
    // §676
    pub pdf_mem_size: i32,
    // §676
    pub pdf_mem: crate::arena::Arr<i32>,
    // §676
    pub pdf_mem_ptr: i32,
    // §680
    pub pdf_file: crate::system::ByteFile,
    // §680
    pub pdf_buf_is_os: bool,
    // §680
    pub pdf_buf_size: i32,
    // §680
    pub pdf_ptr: i32,
    // §680
    pub pdf_op_buf: crate::arena::Arr<eight_bits>,
    // §680
    pub pdf_os_buf: crate::arena::Arr<eight_bits>,
    // §680
    pub pdf_os_buf_size: i32,
    // §680
    pub pdf_os_objnum: crate::arena::Arr<i32>,
    // §680
    pub pdf_os_objoff: crate::arena::Arr<i32>,
    // §680
    pub pdf_os_objidx: halfword,
    // §680
    pub pdf_os_cntr: i32,
    // §680
    pub pdf_op_ptr: i32,
    // §680
    pub pdf_os_ptr: i32,
    // §680
    pub pdf_os_mode: bool,
    // §680
    pub pdf_os_enable: bool,
    // §680
    pub pdf_os_cur_objnum: i32,
    // §680
    pub pdf_gone: longinteger,
    // §680
    pub pdf_save_offset: longinteger,
    // §680
    pub zip_write_state: i32,
    // §680
    pub fixed_pdf_major_version: i32,
    // §680
    pub fixed_pdf_minor_version: i32,
    // §680
    pub fixed_pdf_objcompresslevel: i32,
    // §680
    pub pdf_version_written: bool,
    // §680
    pub fixed_pdfoutput: i32,
    // §680
    pub fixed_pdfoutput_set: bool,
    // §680
    pub fixed_gamma: i32,
    // §680
    pub fixed_image_gamma: i32,
    // §680
    pub fixed_image_hicolor: bool,
    // §680
    pub fixed_image_apply_gamma: i32,
    // §680
    pub epochseconds: i32,
    // §680
    pub microseconds: i32,
    // §680
    pub fixed_pdf_draftmode: i32,
    // §680
    pub fixed_pdf_draftmode_set: bool,
    // §680
    pub pdf_page_group_val: i32,
    // §687
    pub one_bp: scaled,
    // §687
    pub one_hundred_bp: scaled,
    // §687
    pub one_hundred_inch: scaled,
    // §687
    pub one_inch: i32,
    // §687
    pub ten_pow: crate::arena::Arr<i32>,
    // §687
    pub scaled_out: i32,
    // §687
    pub init_pdf_output: bool,
    // §687
    pub adv_char_width_s: i32,
    // §687
    pub adv_char_width_s_out: scaled,
    // §691
    pub pdf_f: internal_font_number,
    // §691
    pub pdf_h: scaled,
    // §691
    pub pdf_v: scaled,
    // §691
    pub pdf_tj_start_h: scaled,
    // §691
    pub cur_delta_h: scaled,
    // §691
    pub pdf_delta_h: scaled,
    // §691
    pub pdf_origin_h: scaled,
    // §691
    pub pdf_origin_v: scaled,
    // §691
    pub pdf_doing_string: bool,
    // §691
    pub pdf_doing_text: bool,
    // §691
    pub min_bp_val: scaled,
    // §691
    pub min_font_val: scaled,
    // §691
    pub fixed_pk_resolution: i32,
    // §691
    pub fixed_decimal_digits: i32,
    // §691
    pub fixed_gen_tounicode: i32,
    // §691
    pub fixed_inclusion_copy_font: i32,
    // §691
    pub pk_scale_factor: i32,
    // §691
    pub pdf_output_option: i32,
    // §691
    pub pdf_output_value: i32,
    // §691
    pub pdf_draftmode_option: i32,
    // §691
    pub pdf_draftmode_value: i32,
    // §691
    pub pdf_cur_Tm_a: i32,
    // §691
    pub pdf_last_f: internal_font_number,
    // §691
    pub pdf_last_fs: internal_font_number,
    // §691
    pub pdf_dummy_font: internal_font_number,
    // §696
    pub obj_tab_size: i32,
    // §696
    pub obj_tab: crate::arena::Arr<obj_entry>,
    // §696
    pub head_tab: crate::arena::Arr<i32>,
    // §696
    pub pages_tail: i32,
    // §696
    pub obj_ptr: i32,
    // §696
    pub sys_obj_ptr: i32,
    // §696
    pub pdf_last_pages: i32,
    // §696
    pub pdf_last_page: i32,
    // §696
    pub pdf_last_stream: i32,
    // §696
    pub pdf_stream_length: longinteger,
    // §696
    pub pdf_stream_length_offset: longinteger,
    // §696
    pub pdf_seek_write_length: bool,
    // §696
    pub pdf_last_byte: eight_bits,
    // §696
    pub pdf_append_list_arg: i32,
    // §696
    pub ff: i32,
    // §696
    pub pdf_box_spec_media: i32,
    // §696
    pub pdf_box_spec_crop: i32,
    // §696
    pub pdf_box_spec_bleed: i32,
    // §696
    pub pdf_box_spec_trim: i32,
    // §696
    pub pdf_box_spec_art: i32,
    // §701
    pub pdf_image_procset: i32,
    // §701
    pub pdf_text_procset: bool,
    // §704
    pub pdf_font_type: crate::arena::Arr<eight_bits>,
    // §704
    pub pdf_font_attr: crate::arena::Arr<str_number>,
    // §704
    pub pdf_font_nobuiltin_tounicode: crate::arena::Arr<bool>,
    // §708
    pub pdf_char_used: crate::arena::Arr<char_used_array>,
    // §708
    pub pdf_font_size: crate::arena::Arr<scaled>,
    // §708
    pub pdf_font_num: crate::arena::Arr<i32>,
    // §708
    pub pdf_font_map: crate::arena::Arr<fm_entry_ptr>,
    // §708
    pub pdf_font_list: halfword,
    // §708
    pub pdf_resname_prefix: str_number,
    // §708
    pub last_tokens_string: str_number,
    // §710
    pub vf_packet_base: crate::arena::Arr<i32>,
    // §710
    pub vf_default_font: crate::arena::Arr<internal_font_number>,
    // §710
    pub vf_local_font_num: crate::arena::Arr<internal_font_number>,
    // §710
    pub vf_packet_length: i32,
    // §710
    pub vf_file: crate::system::ByteFile,
    // §710
    pub vf_nf: internal_font_number,
    // §710
    pub vf_e_fnts: crate::arena::Arr<i32>,
    // §710
    pub vf_i_fnts: crate::arena::Arr<internal_font_number>,
    // §710
    pub tmp_w: memory_word,
    // §723
    pub vf_cur_s: i32,
    // §723
    pub vf_stack: crate::arena::Arr<vf_stack_record>,
    // §723
    pub vf_stack_ptr: vf_stack_index,
    // §774
    pub saved_pdf_cur_form: i32,
    // §811
    pub pdftex_banner: str_number,
    // §818
    pub total_stretch: crate::arena::Arr<scaled>,
    // §818
    pub total_shrink: crate::arena::Arr<scaled>,
    // §818
    pub last_badness: i32,
    // §819
    pub adjust_tail: halfword,
    // §821
    pub pdf_font_blink: crate::arena::Arr<internal_font_number>,
    // §821
    pub pdf_font_elink: crate::arena::Arr<internal_font_number>,
    // §821
    pub pdf_font_has_space_char: crate::arena::Arr<bool>,
    // §821
    pub pdf_font_stretch: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_shrink: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_step: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_expand_ratio: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_auto_expand: crate::arena::Arr<bool>,
    // §821
    pub pdf_font_lp_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_rp_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_ef_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_kn_bs_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_st_bs_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_sh_bs_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_kn_bc_base: crate::arena::Arr<i32>,
    // §821
    pub pdf_font_kn_ac_base: crate::arena::Arr<i32>,
    // §821
    pub font_expand_ratio: i32,
    // §821
    pub last_leftmost_char: halfword,
    // §821
    pub last_rightmost_char: halfword,
    // §821
    pub hlist_stack: crate::arena::Arr<halfword>,
    // §821
    pub hlist_stack_level: i32,
    // §829
    pub pre_adjust_tail: halfword,
    // §837
    pub pack_begin_line: i32,
    // §860
    pub empty_field: two_halves,
    // §860
    pub null_delimiter: four_quarters,
    // §895
    pub cur_mlist: halfword,
    // §895
    pub cur_style: small_number,
    // §895
    pub cur_size: small_number,
    // §895
    pub cur_mu: scaled,
    // §895
    pub mlist_penalties: bool,
    // §900
    pub cur_f: internal_font_number,
    // §900
    pub cur_c: quarterword,
    // §900
    pub cur_i: four_quarters,
    // §940
    pub magic_offset: i32,
    // §946
    pub cur_align: halfword,
    // §946
    pub cur_span: halfword,
    // §946
    pub cur_loop: halfword,
    // §946
    pub align_ptr: halfword,
    // §946
    pub cur_head: halfword,
    // §946
    pub cur_tail: halfword,
    // §946
    pub cur_pre_head: halfword,
    // §946
    pub cur_pre_tail: halfword,
    // §990
    pub just_box: halfword,
    // §997
    pub passive: halfword,
    // §997
    pub printed_node: halfword,
    // §997
    pub pass_number: halfword,
    // §999
    pub active_width: crate::arena::Arr<scaled>,
    // §999
    pub cur_active_width: crate::arena::Arr<scaled>,
    // §999
    pub background: crate::arena::Arr<scaled>,
    // §999
    pub break_width: crate::arena::Arr<scaled>,
    // §999
    pub auto_breaking: bool,
    // §999
    pub prev_p: halfword,
    // §999
    pub first_p: halfword,
    // §999
    pub prev_char_p: halfword,
    // §999
    pub next_char_p: halfword,
    // §999
    pub try_prev_break: bool,
    // §999
    pub prev_legal: halfword,
    // §999
    pub prev_prev_legal: halfword,
    // §999
    pub prev_auto_breaking: bool,
    // §999
    pub prev_active_width: crate::arena::Arr<scaled>,
    // §999
    pub rejected_cur_p: halfword,
    // §999
    pub before_rejected_cur_p: bool,
    // §999
    pub max_stretch_ratio: i32,
    // §999
    pub max_shrink_ratio: i32,
    // §999
    pub cur_font_step: i32,
    // §1001
    pub no_shrink_error_yet: bool,
    // §1004
    pub cur_p: halfword,
    // §1004
    pub second_pass: bool,
    // §1004
    pub final_pass: bool,
    // §1004
    pub threshold: i32,
    // §1009
    pub minimal_demerits: crate::arena::Arr<i32>,
    // §1009
    pub minimum_demerits: i32,
    // §1009
    pub best_place: crate::arena::Arr<halfword>,
    // §1009
    pub best_pl_line: crate::arena::Arr<halfword>,
    // §1015
    pub disc_width: crate::arena::Arr<scaled>,
    // §1023
    pub easy_line: halfword,
    // §1023
    pub last_special_line: halfword,
    // §1023
    pub first_width: scaled,
    // §1023
    pub second_width: scaled,
    // §1023
    pub first_indent: scaled,
    // §1023
    pub second_indent: scaled,
    // §1048
    pub best_bet: halfword,
    // §1048
    pub fewest_demerits: i32,
    // §1048
    pub best_line: halfword,
    // §1048
    pub actual_looseness: i32,
    // §1048
    pub line_diff: i32,
    // §1069
    pub hc: crate::arena::Arr<i32>,
    // §1069
    pub hn: i32,
    // §1069
    pub ha: halfword,
    // §1069
    pub hb: halfword,
    // §1069
    pub hf: internal_font_number,
    // §1069
    pub hu: crate::arena::Arr<i32>,
    // §1069
    pub hyf_char: i32,
    // §1069
    pub cur_lang: ASCII_code,
    // §1069
    pub init_cur_lang: ASCII_code,
    // §1069
    pub l_hyf: i32,
    // §1069
    pub r_hyf: i32,
    // §1069
    pub init_l_hyf: i32,
    // §1069
    pub init_r_hyf: i32,
    // §1069
    pub hyf_bchar: halfword,
    // §1077
    pub hyf: crate::arena::Arr<i32>,
    // §1077
    pub init_list: halfword,
    // §1077
    pub init_lig: bool,
    // §1077
    pub init_lft: bool,
    // §1082
    pub hyphen_passed: small_number,
    // §1084
    pub cur_l: halfword,
    // §1084
    pub cur_r: halfword,
    // §1084
    pub cur_q: halfword,
    // §1084
    pub lig_stack: halfword,
    // §1084
    pub ligature_present: bool,
    // §1084
    pub lft_hit: bool,
    // §1084
    pub rt_hit: bool,
    // §1098
    pub trie: crate::arena::Arr<two_halves>,
    // §1098
    pub hyf_distance: crate::arena::Arr<small_number>,
    // §1098
    pub hyf_num: crate::arena::Arr<small_number>,
    // §1098
    pub hyf_next: crate::arena::Arr<quarterword>,
    // §1098
    pub op_start: crate::arena::Arr<i32>,
    // §1103
    pub hyph_word: crate::arena::Arr<str_number>,
    // §1103
    pub hyph_list: crate::arena::Arr<halfword>,
    // §1103
    pub hyph_count: hyph_pointer,
    // §1120
    pub trie_op_hash: crate::arena::Arr<i32>,
    // §1120
    pub trie_used: crate::arena::Arr<quarterword>,
    // §1120
    pub trie_op_lang: crate::arena::Arr<ASCII_code>,
    // §1120
    pub trie_op_val: crate::arena::Arr<quarterword>,
    // §1120
    pub trie_op_ptr: i32,
    // §1124
    pub trie_c: crate::arena::Arr<packed_ASCII_code>,
    // §1124
    pub trie_o: crate::arena::Arr<quarterword>,
    // §1124
    pub trie_l: crate::arena::Arr<trie_pointer>,
    // §1124
    pub trie_r: crate::arena::Arr<trie_pointer>,
    // §1124
    pub trie_ptr: trie_pointer,
    // §1124
    pub trie_hash: crate::arena::Arr<trie_pointer>,
    // §1127
    pub trie_taken: crate::arena::Arr<bool>,
    // §1127
    pub trie_min: crate::arena::Arr<trie_pointer>,
    // §1127
    pub trie_max: trie_pointer,
    // §1127
    pub trie_not_ready: bool,
    // §1148
    pub best_height_plus_depth: scaled,
    // §1157
    pub page_tail: halfword,
    // §1157
    pub page_contents: i32,
    // §1157
    pub page_max_depth: scaled,
    // §1157
    pub best_page_break: halfword,
    // §1157
    pub least_page_cost: i32,
    // §1157
    pub best_size: scaled,
    // §1159
    pub page_so_far: crate::arena::Arr<scaled>,
    // §1159
    pub last_glue: halfword,
    // §1159
    pub last_penalty: i32,
    // §1159
    pub last_kern: scaled,
    // §1159
    pub last_node_type: i32,
    // §1159
    pub insert_penalties: i32,
    // §1166
    pub output_active: bool,
    // §1166
    pub output_can_end: bool,
    // §1209
    pub main_f: internal_font_number,
    // §1209
    pub main_i: four_quarters,
    // §1209
    pub main_j: four_quarters,
    // §1209
    pub main_k: font_index,
    // §1209
    pub main_p: halfword,
    // §1209
    pub main_s: i32,
    // §1209
    pub bchar: halfword,
    // §1209
    pub false_bchar: halfword,
    // §1209
    pub cancel_boundary: bool,
    // §1209
    pub ins_disc: bool,
    // §1252
    pub cur_box: halfword,
    // §1444
    pub after_token: halfword,
    // §1459
    pub long_help_seen: bool,
    // §1477
    pub format_ident: str_number,
    // §1483
    pub fmt_file: crate::system::WordFile,
    // §1511
    pub ready_already: i32,
    // §1522
    pub write_file: Vec<crate::system::AlphaFile>,
    // §1522
    pub write_open: crate::arena::Arr<bool>,
    // §1525
    pub write_loc: halfword,
    // §1543
    pub pdf_last_obj: i32,
    // §1547
    pub pdf_last_xform: i32,
    // §1550
    pub pdf_last_ximage: i32,
    // §1550
    pub pdf_last_ximage_pages: i32,
    // §1550
    pub pdf_last_ximage_colordepth: i32,
    // §1550
    pub alt_rule: halfword,
    // §1550
    pub warn_pdfpagebox: bool,
    // §1557
    pub pdf_last_annot: i32,
    // §1559
    pub pdf_last_link: i32,
    // §1570
    pub pdf_last_x_pos: i32,
    // §1570
    pub pdf_last_y_pos: i32,
    // §1570
    pub pdf_snapx_refpos: i32,
    // §1570
    pub pdf_snapy_refpos: i32,
    // §1570
    pub count_do_snapy: i32,
    // §1583
    pub pdf_retval: i32,
    // §1628
    pub cur_page_width: scaled,
    // §1628
    pub cur_page_height: scaled,
    // §1628
    pub cur_h_offset: scaled,
    // §1628
    pub cur_v_offset: scaled,
    // §1628
    pub pdf_obj_list: halfword,
    // §1628
    pub pdf_xform_list: halfword,
    // §1628
    pub pdf_ximage_list: halfword,
    // §1628
    pub last_thread: halfword,
    // §1628
    pub pdf_thread_ht: scaled,
    // §1628
    pub pdf_thread_dp: scaled,
    // §1628
    pub pdf_thread_wd: scaled,
    // §1628
    pub pdf_last_thread_id: halfword,
    // §1628
    pub pdf_last_thread_named_id: bool,
    // §1628
    pub pdf_thread_level: i32,
    // §1628
    pub pdf_annot_list: halfword,
    // §1628
    pub pdf_link_list: halfword,
    // §1628
    pub pdf_dest_list: halfword,
    // §1628
    pub pdf_bead_list: halfword,
    // §1628
    pub pdf_obj_count: i32,
    // §1628
    pub pdf_xform_count: i32,
    // §1628
    pub pdf_ximage_count: i32,
    // §1628
    pub pdf_cur_form: i32,
    // §1628
    pub pdf_first_outline: i32,
    // §1628
    pub pdf_last_outline: i32,
    // §1628
    pub pdf_parent_outline: i32,
    // §1628
    pub pdf_xform_width: scaled,
    // §1628
    pub pdf_xform_height: scaled,
    // §1628
    pub pdf_xform_depth: scaled,
    // §1628
    pub pdf_info_toks: halfword,
    // §1628
    pub pdf_catalog_toks: halfword,
    // §1628
    pub pdf_catalog_openaction: i32,
    // §1628
    pub pdf_names_toks: halfword,
    // §1628
    pub pdf_dest_names_ptr: i32,
    // §1628
    pub dest_names_size: i32,
    // §1628
    pub dest_names: crate::arena::Arr<dest_name_entry>,
    // §1628
    pub pk_dpi: i32,
    // §1628
    pub image_orig_x: i32,
    // §1628
    pub image_orig_y: i32,
    // §1628
    pub pdf_trailer_toks: halfword,
    // §1628
    pub pdf_trailer_id_toks: halfword,
    // §1628
    pub gen_faked_interword_space: bool,
    // §1628
    pub gen_running_link: bool,
    // §1628
    pub pdf_space_font_name: str_number,
    // §1633
    pub pdf_link_stack: crate::arena::Arr<pdf_link_stack_record>,
    // §1633
    pub pdf_link_stack_ptr: small_number,
    // §1640
    pub is_shipping_page: bool,
    // §1652
    pub eTeX_mode: i32,
    // §1660
    pub eof_seen: crate::arena::Arr<bool>,
    // §1705
    pub LR_ptr: halfword,
    // §1705
    pub LR_problems: i32,
    // §1705
    pub cur_dir: small_number,
    // §1750
    pub pseudo_files: halfword,
    // §1773
    pub grp_stack: crate::arena::Arr<save_pointer>,
    // §1773
    pub if_stack: crate::arena::Arr<halfword>,
    // §1814
    pub max_reg_num: halfword,
    // §1814
    pub max_reg_help_line: str_number,
    // §1816
    pub sa_root: crate::arena::Arr<halfword>,
    // §1816
    pub cur_ptr: halfword,
    // §1816
    pub sa_null: memory_word,
    // §1835
    pub sa_chain: halfword,
    // §1835
    pub sa_level: quarterword,
    // §1842
    pub last_line_fill: halfword,
    // §1842
    pub do_last_line_fit: bool,
    // §1842
    pub active_node_size: small_number,
    // §1842
    pub fill_width: crate::arena::Arr<scaled>,
    // §1842
    pub best_pl_short: crate::arena::Arr<scaled>,
    // §1842
    pub best_pl_glue: crate::arena::Arr<scaled>,
    // §1858
    pub hyph_start: trie_pointer,
    // §1858
    pub hyph_index: trie_pointer,
    // §1859
    pub disc_ptr: crate::arena::Arr<halfword>,
    // §1871
    pub expand_depth: i32,
    // §1871
    pub expand_depth_count: i32,
    // §1871
    pub shellenabledp: bool,
    // §1871
    pub restrictedshell: bool,
    // §1880
    pub mltex_p: bool,
    // §1880
    pub mltex_enabled_p: bool,
    // §1885
    pub error_line: i32,
    // §1885
    pub half_error_line: i32,
    // §1885
    pub max_print_line: i32,
    // §1885
    pub file_line_error_style_p: bool,
    // §1885
    pub halt_on_error_p: bool,
    // §1885
    pub halting_on_error_p: bool,
    // §1885
    pub parse_first_line_p: bool,
    // §1885
    pub dump_line: bool,
    // §1885
    pub eight_bit_p: bool,
    // §1885
    pub translate_filename_p: bool,
    // §1893
    pub ckpt_request: i32,
    // §1893
    pub ckpt_arm_cs: halfword,
    // §1893
    pub ckpt_arm_level: i32,
    // §1893
    pub ckpt_resuming: bool,
    // §1893
    pub ckpt_on_shipout: i32,
    // §1893
    pub ckpt_on_segment: i32,
    // §1895
    pub rs_on: bool,
    // §1895
    pub rs_seen: crate::arena::Arr<bool>,
    // §1898
    pub macro_prof_on: bool,
    // §1899
    pub intr_on: bool,
    // §1899
    pub intr_at_switch: bool,
    // §1899
    pub intr_rec_on: bool,
    // §1899
    pub intr_all: bool,
    // §1899
    pub intr_weak: bool,
    // §1899
    pub intr_state: crate::arena::Arr<i32>,
    // §1899
    pub intr_cand: crate::arena::Arr<i32>,
    // §1899
    pub intr_watch: crate::arena::Arr<i32>,
    // §1899
    pub intr_seen: crate::arena::Arr<i32>,
    // §1899
    pub intr_pre: crate::arena::Arr<memory_word>,
    // §1899
    pub intr_data: crate::arena::Arr<i32>,
    /// The word space every `Arr` above lives in (crates/flashtex-engine/src/arena.rs).
    pub arena: crate::arena::Arena,
}

/// Bytes of the scalar globals' region at the start of the word space.
pub const SCALAR_BYTES: usize = 0
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<[u8; 256]>()
    + crate::arena::slot::<[u8; 1024]>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<list_state_record>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<quarterword>()
    + crate::arena::slot::<group_code>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<eight_bits>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<in_state_record>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<glue_ord>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<[u8; 20]>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<font_index>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<four_quarters>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<quarterword>()
    + crate::arena::slot::<quarterword>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<dvi_index>()
    + crate::arena::slot::<dvi_index>()
    + crate::arena::slot::<dvi_index>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<longinteger>()
    + crate::arena::slot::<longinteger>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<longinteger>()
    + crate::arena::slot::<longinteger>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<eight_bits>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<memory_word>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<vf_stack_index>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<two_halves>()
    + crate::arena::slot::<four_quarters>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<quarterword>()
    + crate::arena::slot::<four_quarters>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<ASCII_code>()
    + crate::arena::slot::<ASCII_code>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<hyph_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<trie_pointer>()
    + crate::arena::slot::<trie_pointer>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<four_quarters>()
    + crate::arena::slot::<four_quarters>()
    + crate::arena::slot::<font_index>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<memory_word>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<quarterword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<trie_pointer>()
    + crate::arena::slot::<trie_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>();

impl Globals {
    pub fn new() -> Box<Globals> {
        let mut __plan = crate::arena::Plan::new(SCALAR_BYTES);
        let __r_xord = __plan.reserve::<ASCII_code>("xord", 256);
        let __r_xprn = __plan.reserve::<bool>("xprn", 256);
        let __r_buffer = __plan.reserve::<ASCII_code>("buffer", 200001);
        let __r_str_pool = __plan.reserve::<packed_ASCII_code>("str_pool", 6250001);
        let __r_str_start = __plan.reserve::<pool_pointer>("str_start", 500001);
        let __r_dig = __plan.reserve::<i32>("dig", 23);
        let __r_trick_buf = __plan.reserve::<ASCII_code>("trick_buf", 256);
        let __r_help_line = __plan.reserve::<str_number>("help_line", 6);
        let __r_randoms = __plan.reserve::<i32>("randoms", 55);
        let __r_two_to_the = __plan.reserve::<i32>("two_to_the", 31);
        let __r_spec_log = __plan.reserve::<i32>("spec_log", 28);
        let __r_mem = __plan.reserve::<memory_word>("mem", 5000000);
        let __r_nest = __plan.reserve::<list_state_record>("nest", 1001);
        let __r_eqtb = __plan.reserve::<memory_word>("eqtb", 629929);
        let __r_xeq_level = __plan.reserve::<quarterword>("xeq_level", 912);
        let __r_hash = __plan.reserve::<two_halves>("hash", 629416);
        let __r_prim = __plan.reserve::<two_halves>("prim", 2101);
        let __r_save_stack = __plan.reserve::<memory_word>("save_stack", 200001);
        let __r_input_stack = __plan.reserve::<in_state_record>("input_stack", 10001);
        let __r_line_stack = __plan.reserve::<i32>("line_stack", 15);
        let __r_param_stack = __plan.reserve::<halfword>("param_stack", 20001);
        let __r_cur_mark = __plan.reserve::<halfword>("cur_mark", 5);
        let __r_pstack = __plan.reserve::<halfword>("pstack", 9);
        let __r_read_open = __plan.reserve::<i32>("read_open", 17);
        let __r_full_source_filename_stack =
            __plan.reserve::<str_number>("full_source_filename_stack", 16);
        let __r_font_info = __plan.reserve::<memory_word>("font_info", 8000001);
        let __r_font_check = __plan.reserve::<four_quarters>("font_check", 9001);
        let __r_font_size = __plan.reserve::<scaled>("font_size", 9001);
        let __r_font_dsize = __plan.reserve::<scaled>("font_dsize", 9001);
        let __r_font_params = __plan.reserve::<font_index>("font_params", 9001);
        let __r_font_name = __plan.reserve::<str_number>("font_name", 9001);
        let __r_font_area = __plan.reserve::<str_number>("font_area", 9001);
        let __r_font_bc = __plan.reserve::<eight_bits>("font_bc", 9001);
        let __r_font_ec = __plan.reserve::<eight_bits>("font_ec", 9001);
        let __r_font_glue = __plan.reserve::<halfword>("font_glue", 9001);
        let __r_font_used = __plan.reserve::<bool>("font_used", 9001);
        let __r_hyphen_char = __plan.reserve::<i32>("hyphen_char", 9001);
        let __r_skew_char = __plan.reserve::<i32>("skew_char", 9001);
        let __r_bchar_label = __plan.reserve::<font_index>("bchar_label", 9001);
        let __r_font_bchar = __plan.reserve::<i32>("font_bchar", 9001);
        let __r_font_false_bchar = __plan.reserve::<i32>("font_false_bchar", 9001);
        let __r_char_base = __plan.reserve::<i32>("char_base", 9001);
        let __r_width_base = __plan.reserve::<i32>("width_base", 9001);
        let __r_height_base = __plan.reserve::<i32>("height_base", 9001);
        let __r_depth_base = __plan.reserve::<i32>("depth_base", 9001);
        let __r_italic_base = __plan.reserve::<i32>("italic_base", 9001);
        let __r_lig_kern_base = __plan.reserve::<i32>("lig_kern_base", 9001);
        let __r_kern_base = __plan.reserve::<i32>("kern_base", 9001);
        let __r_exten_base = __plan.reserve::<i32>("exten_base", 9001);
        let __r_param_base = __plan.reserve::<i32>("param_base", 9001);
        let __r_dvi_buf = __plan.reserve::<eight_bits>("dvi_buf", 16385);
        let __r_pdf_mem = __plan.reserve::<i32>("pdf_mem", ((sup_pdf_mem_size) as usize) + 1);
        let __r_pdf_op_buf =
            __plan.reserve::<eight_bits>("pdf_op_buf", ((pdf_op_buf_size) as usize) + 1);
        let __r_pdf_os_buf =
            __plan.reserve::<eight_bits>("pdf_os_buf", ((sup_pdf_os_buf_size) as usize) + 1);
        let __r_pdf_os_objnum =
            __plan.reserve::<i32>("pdf_os_objnum", ((pdf_os_max_objs) as usize) + 1);
        let __r_pdf_os_objoff =
            __plan.reserve::<i32>("pdf_os_objoff", ((pdf_os_max_objs) as usize) + 1);
        let __r_ten_pow = __plan.reserve::<i32>("ten_pow", 10);
        let __r_obj_tab = __plan.reserve::<obj_entry>("obj_tab", ((sup_obj_tab_size) as usize) + 1);
        let __r_head_tab = __plan.reserve::<i32>("head_tab", 10);
        let __r_pdf_font_type =
            __plan.reserve::<eight_bits>("pdf_font_type", ((font_max) as usize) + 1);
        let __r_pdf_font_attr =
            __plan.reserve::<str_number>("pdf_font_attr", ((font_max) as usize) + 1);
        let __r_pdf_font_nobuiltin_tounicode =
            __plan.reserve::<bool>("pdf_font_nobuiltin_tounicode", ((font_max) as usize) + 1);
        let __r_pdf_char_used =
            __plan.reserve::<char_used_array>("pdf_char_used", ((font_max) as usize) + 1);
        let __r_pdf_font_size =
            __plan.reserve::<scaled>("pdf_font_size", ((font_max) as usize) + 1);
        let __r_pdf_font_num = __plan.reserve::<i32>("pdf_font_num", ((font_max) as usize) + 1);
        let __r_pdf_font_map =
            __plan.reserve::<fm_entry_ptr>("pdf_font_map", ((font_max) as usize) + 1);
        let __r_vf_packet_base = __plan.reserve::<i32>("vf_packet_base", ((font_max) as usize) + 1);
        let __r_vf_default_font =
            __plan.reserve::<internal_font_number>("vf_default_font", ((font_max) as usize) + 1);
        let __r_vf_local_font_num =
            __plan.reserve::<internal_font_number>("vf_local_font_num", ((font_max) as usize) + 1);
        let __r_vf_e_fnts = __plan.reserve::<i32>("vf_e_fnts", ((font_max) as usize) + 1);
        let __r_vf_i_fnts =
            __plan.reserve::<internal_font_number>("vf_i_fnts", ((font_max) as usize) + 1);
        let __r_vf_stack = __plan.reserve::<vf_stack_record>("vf_stack", 101);
        let __r_total_stretch = __plan.reserve::<scaled>("total_stretch", 4);
        let __r_total_shrink = __plan.reserve::<scaled>("total_shrink", 4);
        let __r_pdf_font_blink =
            __plan.reserve::<internal_font_number>("pdf_font_blink", ((font_max) as usize) + 1);
        let __r_pdf_font_elink =
            __plan.reserve::<internal_font_number>("pdf_font_elink", ((font_max) as usize) + 1);
        let __r_pdf_font_has_space_char =
            __plan.reserve::<bool>("pdf_font_has_space_char", ((font_max) as usize) + 1);
        let __r_pdf_font_stretch =
            __plan.reserve::<i32>("pdf_font_stretch", ((font_max) as usize) + 1);
        let __r_pdf_font_shrink =
            __plan.reserve::<i32>("pdf_font_shrink", ((font_max) as usize) + 1);
        let __r_pdf_font_step = __plan.reserve::<i32>("pdf_font_step", ((font_max) as usize) + 1);
        let __r_pdf_font_expand_ratio =
            __plan.reserve::<i32>("pdf_font_expand_ratio", ((font_max) as usize) + 1);
        let __r_pdf_font_auto_expand =
            __plan.reserve::<bool>("pdf_font_auto_expand", ((font_max) as usize) + 1);
        let __r_pdf_font_lp_base =
            __plan.reserve::<i32>("pdf_font_lp_base", ((font_max) as usize) + 1);
        let __r_pdf_font_rp_base =
            __plan.reserve::<i32>("pdf_font_rp_base", ((font_max) as usize) + 1);
        let __r_pdf_font_ef_base =
            __plan.reserve::<i32>("pdf_font_ef_base", ((font_max) as usize) + 1);
        let __r_pdf_font_kn_bs_base =
            __plan.reserve::<i32>("pdf_font_kn_bs_base", ((font_max) as usize) + 1);
        let __r_pdf_font_st_bs_base =
            __plan.reserve::<i32>("pdf_font_st_bs_base", ((font_max) as usize) + 1);
        let __r_pdf_font_sh_bs_base =
            __plan.reserve::<i32>("pdf_font_sh_bs_base", ((font_max) as usize) + 1);
        let __r_pdf_font_kn_bc_base =
            __plan.reserve::<i32>("pdf_font_kn_bc_base", ((font_max) as usize) + 1);
        let __r_pdf_font_kn_ac_base =
            __plan.reserve::<i32>("pdf_font_kn_ac_base", ((font_max) as usize) + 1);
        let __r_hlist_stack = __plan.reserve::<halfword>("hlist_stack", 513);
        let __r_active_width = __plan.reserve::<scaled>("active_width", 8);
        let __r_cur_active_width = __plan.reserve::<scaled>("cur_active_width", 8);
        let __r_background = __plan.reserve::<scaled>("background", 8);
        let __r_break_width = __plan.reserve::<scaled>("break_width", 8);
        let __r_prev_active_width = __plan.reserve::<scaled>("prev_active_width", 8);
        let __r_minimal_demerits = __plan.reserve::<i32>("minimal_demerits", 4);
        let __r_best_place = __plan.reserve::<halfword>("best_place", 4);
        let __r_best_pl_line = __plan.reserve::<halfword>("best_pl_line", 4);
        let __r_disc_width = __plan.reserve::<scaled>("disc_width", 8);
        let __r_hc = __plan.reserve::<i32>("hc", 66);
        let __r_hu = __plan.reserve::<i32>("hu", 64);
        let __r_hyf = __plan.reserve::<i32>("hyf", 65);
        let __r_trie = __plan.reserve::<two_halves>("trie", 1100001);
        let __r_hyf_distance = __plan.reserve::<small_number>("hyf_distance", 35111);
        let __r_hyf_num = __plan.reserve::<small_number>("hyf_num", 35111);
        let __r_hyf_next = __plan.reserve::<quarterword>("hyf_next", 35111);
        let __r_op_start = __plan.reserve::<i32>("op_start", 256);
        let __r_hyph_word = __plan.reserve::<str_number>("hyph_word", 8192);
        let __r_hyph_list = __plan.reserve::<halfword>("hyph_list", 8192);
        let __r_trie_op_hash = __plan.reserve::<i32>("trie_op_hash", 70223);
        let __r_trie_used = __plan.reserve::<quarterword>("trie_used", 256);
        let __r_trie_op_lang = __plan.reserve::<ASCII_code>("trie_op_lang", 35111);
        let __r_trie_op_val = __plan.reserve::<quarterword>("trie_op_val", 35111);
        let __r_trie_c = __plan.reserve::<packed_ASCII_code>("trie_c", 1100001);
        let __r_trie_o = __plan.reserve::<quarterword>("trie_o", 1100001);
        let __r_trie_l = __plan.reserve::<trie_pointer>("trie_l", 1100001);
        let __r_trie_r = __plan.reserve::<trie_pointer>("trie_r", 1100001);
        let __r_trie_hash = __plan.reserve::<trie_pointer>("trie_hash", 1100001);
        let __r_trie_taken = __plan.reserve::<bool>("trie_taken", 1100000);
        let __r_trie_min = __plan.reserve::<trie_pointer>("trie_min", 256);
        let __r_page_so_far = __plan.reserve::<scaled>("page_so_far", 8);
        let __r_write_open = __plan.reserve::<bool>("write_open", 18);
        let __r_dest_names =
            __plan.reserve::<dest_name_entry>("dest_names", ((sup_dest_names_size) as usize) + 1);
        let __r_pdf_link_stack = __plan.reserve::<pdf_link_stack_record>("pdf_link_stack", 10);
        let __r_eof_seen = __plan.reserve::<bool>("eof_seen", 15);
        let __r_grp_stack = __plan.reserve::<save_pointer>("grp_stack", 16);
        let __r_if_stack = __plan.reserve::<halfword>("if_stack", 16);
        let __r_sa_root = __plan.reserve::<halfword>("sa_root", 7);
        let __r_fill_width = __plan.reserve::<scaled>("fill_width", 3);
        let __r_best_pl_short = __plan.reserve::<scaled>("best_pl_short", 4);
        let __r_best_pl_glue = __plan.reserve::<scaled>("best_pl_glue", 4);
        let __r_disc_ptr = __plan.reserve::<halfword>("disc_ptr", 3);
        let __r_rs_seen = __plan.reserve::<bool>("rs_seen", 629930);
        let __r_intr_state = __plan.reserve::<i32>("intr_state", 4096);
        let __r_intr_cand = __plan.reserve::<i32>("intr_cand", 29930);
        let __r_intr_watch = __plan.reserve::<i32>("intr_watch", 29930);
        let __r_intr_seen = __plan.reserve::<i32>("intr_seen", 29930);
        let __r_intr_pre = __plan.reserve::<memory_word>("intr_pre", 29930);
        let __r_intr_data = __plan.reserve::<i32>("intr_data", 8388608);
        let __arena = __plan.build();
        Box::new(Globals {
            bad: 0,
            xord: __arena.arr(__r_xord, 256),
            xchr: [0u8; 256],
            xprn: __arena.arr(__r_xprn, 256),
            name_of_file: [0u8; 1024],
            name_length: 0,
            buffer: __arena.arr(__r_buffer, 200001),
            first: 0,
            last: 0,
            max_buf_stack: 0,
            term_in: Default::default(),
            term_out: Default::default(),
            str_pool: __arena.arr(__r_str_pool, 6250001),
            str_start: __arena.arr(__r_str_start, 500001),
            pool_ptr: 0,
            str_ptr: 0,
            init_pool_ptr: 0,
            init_str_ptr: 0,
            pool_file: Default::default(),
            log_file: Default::default(),
            selector: 0,
            dig: __arena.arr(__r_dig, 23),
            tally: 0,
            term_offset: 0,
            file_offset: 0,
            trick_buf: __arena.arr(__r_trick_buf, 256),
            trick_count: 0,
            first_count: 0,
            interaction: 0,
            interaction_option: 0,
            deletions_allowed: false,
            set_box_allowed: false,
            history: 0,
            error_count: 0,
            help_line: __arena.arr(__r_help_line, 6),
            help_ptr: 0,
            use_err_help: false,
            interrupt: 0,
            OK_to_interrupt: false,
            arith_error: false,
            remainder: 0,
            randoms: __arena.arr(__r_randoms, 55),
            j_random: 0,
            random_seed: 0,
            two_to_the: __arena.arr(__r_two_to_the, 31),
            spec_log: __arena.arr(__r_spec_log, 28),
            temp_ptr: 0,
            mem: __arena.arr(__r_mem, 5000000),
            lo_mem_max: 0,
            hi_mem_min: 0,
            var_used: 0,
            dyn_used: 0,
            avail: 0,
            mem_end: 0,
            rover: 0,
            font_in_short_display: 0,
            depth_threshold: 0,
            breadth_max: 0,
            nest: __arena.arr(__r_nest, 1001),
            nest_ptr: 0,
            max_nest_stack: 0,
            cur_list: list_state_record::default(),
            shown_mode: 0,
            save_tail: 0,
            prev_tail: 0,
            old_setting: 0,
            old_selector_ignored_err: 0,
            sys_time: 0,
            sys_day: 0,
            sys_month: 0,
            sys_year: 0,
            eqtb: __arena.arr(__r_eqtb, 629929),
            xeq_level: __arena.arr(__r_xeq_level, 912),
            hash: __arena.arr(__r_hash, 629416),
            hash_used: 0,
            hash_high: 0,
            no_new_control_sequence: false,
            cs_count: 0,
            prim: __arena.arr(__r_prim, 2101),
            prim_used: 0,
            save_stack: __arena.arr(__r_save_stack, 200001),
            save_ptr: 0,
            max_save_stack: 0,
            cur_level: 0,
            cur_group: 0,
            cur_boundary: 0,
            mag_set: 0,
            cur_cmd: 0,
            cur_chr: 0,
            cur_cs: 0,
            cur_tok: 0,
            input_stack: __arena.arr(__r_input_stack, 10001),
            input_ptr: 0,
            max_in_stack: 0,
            cur_input: in_state_record::default(),
            in_open: 0,
            open_parens: 0,
            input_file: (0..15).map(|_| Default::default()).collect::<Vec<_>>(),
            line: 0,
            line_stack: __arena.arr(__r_line_stack, 15),
            scanner_status: 0,
            warning_index: 0,
            def_ref: 0,
            param_stack: __arena.arr(__r_param_stack, 20001),
            param_ptr: 0,
            max_param_stack: 0,
            align_state: 0,
            base_ptr: 0,
            par_loc: 0,
            par_token: 0,
            force_eof: false,
            is_in_csname: false,
            cur_mark: __arena.arr(__r_cur_mark, 5),
            long_state: 0,
            pstack: __arena.arr(__r_pstack, 9),
            cur_val: 0,
            cur_val_level: 0,
            radix: 0,
            cur_order: 0,
            read_file: (0..16).map(|_| Default::default()).collect::<Vec<_>>(),
            read_open: __arena.arr(__r_read_open, 17),
            cond_ptr: 0,
            if_limit: 0,
            cur_if: 0,
            if_line: 0,
            skip_line: 0,
            cur_name: 0,
            cur_area: 0,
            cur_ext: 0,
            area_delimiter: 0,
            ext_delimiter: 0,
            quoted_filename: false,
            stop_at_space: false,
            full_source_filename_stack: __arena.arr(__r_full_source_filename_stack, 16),
            TEX_format_default: [0u8; 20],
            name_in_progress: false,
            job_name: 0,
            log_opened: false,
            dvi_file: Default::default(),
            output_file_name: 0,
            log_name: 0,
            tfm_file: Default::default(),
            font_info: __arena.arr(__r_font_info, 8000001),
            fmem_ptr: 0,
            font_ptr: 0,
            font_check: __arena.arr(__r_font_check, 9001),
            font_size: __arena.arr(__r_font_size, 9001),
            font_dsize: __arena.arr(__r_font_dsize, 9001),
            font_params: __arena.arr(__r_font_params, 9001),
            font_name: __arena.arr(__r_font_name, 9001),
            font_area: __arena.arr(__r_font_area, 9001),
            font_bc: __arena.arr(__r_font_bc, 9001),
            font_ec: __arena.arr(__r_font_ec, 9001),
            font_glue: __arena.arr(__r_font_glue, 9001),
            font_used: __arena.arr(__r_font_used, 9001),
            hyphen_char: __arena.arr(__r_hyphen_char, 9001),
            skew_char: __arena.arr(__r_skew_char, 9001),
            bchar_label: __arena.arr(__r_bchar_label, 9001),
            font_bchar: __arena.arr(__r_font_bchar, 9001),
            font_false_bchar: __arena.arr(__r_font_false_bchar, 9001),
            char_base: __arena.arr(__r_char_base, 9001),
            width_base: __arena.arr(__r_width_base, 9001),
            height_base: __arena.arr(__r_height_base, 9001),
            depth_base: __arena.arr(__r_depth_base, 9001),
            italic_base: __arena.arr(__r_italic_base, 9001),
            lig_kern_base: __arena.arr(__r_lig_kern_base, 9001),
            kern_base: __arena.arr(__r_kern_base, 9001),
            exten_base: __arena.arr(__r_exten_base, 9001),
            param_base: __arena.arr(__r_param_base, 9001),
            null_character: four_quarters::default(),
            total_pages: 0,
            max_v: 0,
            max_h: 0,
            max_push: 0,
            last_bop: 0,
            dead_cycles: 0,
            doing_leaders: false,
            c: 0,
            f: 0,
            rule_ht: 0,
            rule_dp: 0,
            rule_wd: 0,
            g: 0,
            lq: 0,
            lr: 0,
            dvi_buf: __arena.arr(__r_dvi_buf, 16385),
            half_buf: 0,
            dvi_limit: 0,
            dvi_ptr: 0,
            dvi_offset: 0,
            dvi_gone: 0,
            down_ptr: 0,
            right_ptr: 0,
            dvi_h: 0,
            dvi_v: 0,
            cur_h: 0,
            cur_v: 0,
            dvi_f: 0,
            cur_s: 0,
            pdf_mem_size: 0,
            pdf_mem: __arena.arr(__r_pdf_mem, 0),
            pdf_mem_ptr: 0,
            pdf_file: Default::default(),
            pdf_buf_is_os: false,
            pdf_buf_size: 0,
            pdf_ptr: 0,
            pdf_op_buf: __arena.arr(__r_pdf_op_buf, 0),
            pdf_os_buf: __arena.arr(__r_pdf_os_buf, 0),
            pdf_os_buf_size: 0,
            pdf_os_objnum: __arena.arr(__r_pdf_os_objnum, 0),
            pdf_os_objoff: __arena.arr(__r_pdf_os_objoff, 0),
            pdf_os_objidx: 0,
            pdf_os_cntr: 0,
            pdf_op_ptr: 0,
            pdf_os_ptr: 0,
            pdf_os_mode: false,
            pdf_os_enable: false,
            pdf_os_cur_objnum: 0,
            pdf_gone: 0,
            pdf_save_offset: 0,
            zip_write_state: 0,
            fixed_pdf_major_version: 0,
            fixed_pdf_minor_version: 0,
            fixed_pdf_objcompresslevel: 0,
            pdf_version_written: false,
            fixed_pdfoutput: 0,
            fixed_pdfoutput_set: false,
            fixed_gamma: 0,
            fixed_image_gamma: 0,
            fixed_image_hicolor: false,
            fixed_image_apply_gamma: 0,
            epochseconds: 0,
            microseconds: 0,
            fixed_pdf_draftmode: 0,
            fixed_pdf_draftmode_set: false,
            pdf_page_group_val: 0,
            one_bp: 0,
            one_hundred_bp: 0,
            one_hundred_inch: 0,
            one_inch: 0,
            ten_pow: __arena.arr(__r_ten_pow, 10),
            scaled_out: 0,
            init_pdf_output: false,
            adv_char_width_s: 0,
            adv_char_width_s_out: 0,
            pdf_f: 0,
            pdf_h: 0,
            pdf_v: 0,
            pdf_tj_start_h: 0,
            cur_delta_h: 0,
            pdf_delta_h: 0,
            pdf_origin_h: 0,
            pdf_origin_v: 0,
            pdf_doing_string: false,
            pdf_doing_text: false,
            min_bp_val: 0,
            min_font_val: 0,
            fixed_pk_resolution: 0,
            fixed_decimal_digits: 0,
            fixed_gen_tounicode: 0,
            fixed_inclusion_copy_font: 0,
            pk_scale_factor: 0,
            pdf_output_option: 0,
            pdf_output_value: 0,
            pdf_draftmode_option: 0,
            pdf_draftmode_value: 0,
            pdf_cur_Tm_a: 0,
            pdf_last_f: 0,
            pdf_last_fs: 0,
            pdf_dummy_font: 0,
            obj_tab_size: 0,
            obj_tab: __arena.arr(__r_obj_tab, 0),
            head_tab: __arena.arr(__r_head_tab, 10),
            pages_tail: 0,
            obj_ptr: 0,
            sys_obj_ptr: 0,
            pdf_last_pages: 0,
            pdf_last_page: 0,
            pdf_last_stream: 0,
            pdf_stream_length: 0,
            pdf_stream_length_offset: 0,
            pdf_seek_write_length: false,
            pdf_last_byte: 0,
            pdf_append_list_arg: 0,
            ff: 0,
            pdf_box_spec_media: 0,
            pdf_box_spec_crop: 0,
            pdf_box_spec_bleed: 0,
            pdf_box_spec_trim: 0,
            pdf_box_spec_art: 0,
            pdf_image_procset: 0,
            pdf_text_procset: false,
            pdf_font_type: __arena.arr(__r_pdf_font_type, 0),
            pdf_font_attr: __arena.arr(__r_pdf_font_attr, 0),
            pdf_font_nobuiltin_tounicode: __arena.arr(__r_pdf_font_nobuiltin_tounicode, 0),
            pdf_char_used: __arena.arr(__r_pdf_char_used, 0),
            pdf_font_size: __arena.arr(__r_pdf_font_size, 0),
            pdf_font_num: __arena.arr(__r_pdf_font_num, 0),
            pdf_font_map: __arena.arr(__r_pdf_font_map, 0),
            pdf_font_list: 0,
            pdf_resname_prefix: 0,
            last_tokens_string: 0,
            vf_packet_base: __arena.arr(__r_vf_packet_base, 0),
            vf_default_font: __arena.arr(__r_vf_default_font, 0),
            vf_local_font_num: __arena.arr(__r_vf_local_font_num, 0),
            vf_packet_length: 0,
            vf_file: Default::default(),
            vf_nf: 0,
            vf_e_fnts: __arena.arr(__r_vf_e_fnts, 0),
            vf_i_fnts: __arena.arr(__r_vf_i_fnts, 0),
            tmp_w: memory_word::default(),
            vf_cur_s: 0,
            vf_stack: __arena.arr(__r_vf_stack, 101),
            vf_stack_ptr: 0,
            saved_pdf_cur_form: 0,
            pdftex_banner: 0,
            total_stretch: __arena.arr(__r_total_stretch, 4),
            total_shrink: __arena.arr(__r_total_shrink, 4),
            last_badness: 0,
            adjust_tail: 0,
            pdf_font_blink: __arena.arr(__r_pdf_font_blink, 0),
            pdf_font_elink: __arena.arr(__r_pdf_font_elink, 0),
            pdf_font_has_space_char: __arena.arr(__r_pdf_font_has_space_char, 0),
            pdf_font_stretch: __arena.arr(__r_pdf_font_stretch, 0),
            pdf_font_shrink: __arena.arr(__r_pdf_font_shrink, 0),
            pdf_font_step: __arena.arr(__r_pdf_font_step, 0),
            pdf_font_expand_ratio: __arena.arr(__r_pdf_font_expand_ratio, 0),
            pdf_font_auto_expand: __arena.arr(__r_pdf_font_auto_expand, 0),
            pdf_font_lp_base: __arena.arr(__r_pdf_font_lp_base, 0),
            pdf_font_rp_base: __arena.arr(__r_pdf_font_rp_base, 0),
            pdf_font_ef_base: __arena.arr(__r_pdf_font_ef_base, 0),
            pdf_font_kn_bs_base: __arena.arr(__r_pdf_font_kn_bs_base, 0),
            pdf_font_st_bs_base: __arena.arr(__r_pdf_font_st_bs_base, 0),
            pdf_font_sh_bs_base: __arena.arr(__r_pdf_font_sh_bs_base, 0),
            pdf_font_kn_bc_base: __arena.arr(__r_pdf_font_kn_bc_base, 0),
            pdf_font_kn_ac_base: __arena.arr(__r_pdf_font_kn_ac_base, 0),
            font_expand_ratio: 0,
            last_leftmost_char: 0,
            last_rightmost_char: 0,
            hlist_stack: __arena.arr(__r_hlist_stack, 513),
            hlist_stack_level: 0,
            pre_adjust_tail: 0,
            pack_begin_line: 0,
            empty_field: two_halves::default(),
            null_delimiter: four_quarters::default(),
            cur_mlist: 0,
            cur_style: 0,
            cur_size: 0,
            cur_mu: 0,
            mlist_penalties: false,
            cur_f: 0,
            cur_c: 0,
            cur_i: four_quarters::default(),
            magic_offset: 0,
            cur_align: 0,
            cur_span: 0,
            cur_loop: 0,
            align_ptr: 0,
            cur_head: 0,
            cur_tail: 0,
            cur_pre_head: 0,
            cur_pre_tail: 0,
            just_box: 0,
            passive: 0,
            printed_node: 0,
            pass_number: 0,
            active_width: __arena.arr(__r_active_width, 8),
            cur_active_width: __arena.arr(__r_cur_active_width, 8),
            background: __arena.arr(__r_background, 8),
            break_width: __arena.arr(__r_break_width, 8),
            auto_breaking: false,
            prev_p: 0,
            first_p: 0,
            prev_char_p: 0,
            next_char_p: 0,
            try_prev_break: false,
            prev_legal: 0,
            prev_prev_legal: 0,
            prev_auto_breaking: false,
            prev_active_width: __arena.arr(__r_prev_active_width, 8),
            rejected_cur_p: 0,
            before_rejected_cur_p: false,
            max_stretch_ratio: 0,
            max_shrink_ratio: 0,
            cur_font_step: 0,
            no_shrink_error_yet: false,
            cur_p: 0,
            second_pass: false,
            final_pass: false,
            threshold: 0,
            minimal_demerits: __arena.arr(__r_minimal_demerits, 4),
            minimum_demerits: 0,
            best_place: __arena.arr(__r_best_place, 4),
            best_pl_line: __arena.arr(__r_best_pl_line, 4),
            disc_width: __arena.arr(__r_disc_width, 8),
            easy_line: 0,
            last_special_line: 0,
            first_width: 0,
            second_width: 0,
            first_indent: 0,
            second_indent: 0,
            best_bet: 0,
            fewest_demerits: 0,
            best_line: 0,
            actual_looseness: 0,
            line_diff: 0,
            hc: __arena.arr(__r_hc, 66),
            hn: 0,
            ha: 0,
            hb: 0,
            hf: 0,
            hu: __arena.arr(__r_hu, 64),
            hyf_char: 0,
            cur_lang: 0,
            init_cur_lang: 0,
            l_hyf: 0,
            r_hyf: 0,
            init_l_hyf: 0,
            init_r_hyf: 0,
            hyf_bchar: 0,
            hyf: __arena.arr(__r_hyf, 65),
            init_list: 0,
            init_lig: false,
            init_lft: false,
            hyphen_passed: 0,
            cur_l: 0,
            cur_r: 0,
            cur_q: 0,
            lig_stack: 0,
            ligature_present: false,
            lft_hit: false,
            rt_hit: false,
            trie: __arena.arr(__r_trie, 1100001),
            hyf_distance: __arena.arr(__r_hyf_distance, 35111),
            hyf_num: __arena.arr(__r_hyf_num, 35111),
            hyf_next: __arena.arr(__r_hyf_next, 35111),
            op_start: __arena.arr(__r_op_start, 256),
            hyph_word: __arena.arr(__r_hyph_word, 8192),
            hyph_list: __arena.arr(__r_hyph_list, 8192),
            hyph_count: 0,
            trie_op_hash: __arena.arr(__r_trie_op_hash, 70223),
            trie_used: __arena.arr(__r_trie_used, 256),
            trie_op_lang: __arena.arr(__r_trie_op_lang, 35111),
            trie_op_val: __arena.arr(__r_trie_op_val, 35111),
            trie_op_ptr: 0,
            trie_c: __arena.arr(__r_trie_c, 1100001),
            trie_o: __arena.arr(__r_trie_o, 1100001),
            trie_l: __arena.arr(__r_trie_l, 1100001),
            trie_r: __arena.arr(__r_trie_r, 1100001),
            trie_ptr: 0,
            trie_hash: __arena.arr(__r_trie_hash, 1100001),
            trie_taken: __arena.arr(__r_trie_taken, 1100000),
            trie_min: __arena.arr(__r_trie_min, 256),
            trie_max: 0,
            trie_not_ready: false,
            best_height_plus_depth: 0,
            page_tail: 0,
            page_contents: 0,
            page_max_depth: 0,
            best_page_break: 0,
            least_page_cost: 0,
            best_size: 0,
            page_so_far: __arena.arr(__r_page_so_far, 8),
            last_glue: 0,
            last_penalty: 0,
            last_kern: 0,
            last_node_type: 0,
            insert_penalties: 0,
            output_active: false,
            output_can_end: false,
            main_f: 0,
            main_i: four_quarters::default(),
            main_j: four_quarters::default(),
            main_k: 0,
            main_p: 0,
            main_s: 0,
            bchar: 0,
            false_bchar: 0,
            cancel_boundary: false,
            ins_disc: false,
            cur_box: 0,
            after_token: 0,
            long_help_seen: false,
            format_ident: 0,
            fmt_file: Default::default(),
            ready_already: 0,
            write_file: (0..16).map(|_| Default::default()).collect::<Vec<_>>(),
            write_open: __arena.arr(__r_write_open, 18),
            write_loc: 0,
            pdf_last_obj: 0,
            pdf_last_xform: 0,
            pdf_last_ximage: 0,
            pdf_last_ximage_pages: 0,
            pdf_last_ximage_colordepth: 0,
            alt_rule: 0,
            warn_pdfpagebox: false,
            pdf_last_annot: 0,
            pdf_last_link: 0,
            pdf_last_x_pos: 0,
            pdf_last_y_pos: 0,
            pdf_snapx_refpos: 0,
            pdf_snapy_refpos: 0,
            count_do_snapy: 0,
            pdf_retval: 0,
            cur_page_width: 0,
            cur_page_height: 0,
            cur_h_offset: 0,
            cur_v_offset: 0,
            pdf_obj_list: 0,
            pdf_xform_list: 0,
            pdf_ximage_list: 0,
            last_thread: 0,
            pdf_thread_ht: 0,
            pdf_thread_dp: 0,
            pdf_thread_wd: 0,
            pdf_last_thread_id: 0,
            pdf_last_thread_named_id: false,
            pdf_thread_level: 0,
            pdf_annot_list: 0,
            pdf_link_list: 0,
            pdf_dest_list: 0,
            pdf_bead_list: 0,
            pdf_obj_count: 0,
            pdf_xform_count: 0,
            pdf_ximage_count: 0,
            pdf_cur_form: 0,
            pdf_first_outline: 0,
            pdf_last_outline: 0,
            pdf_parent_outline: 0,
            pdf_xform_width: 0,
            pdf_xform_height: 0,
            pdf_xform_depth: 0,
            pdf_info_toks: 0,
            pdf_catalog_toks: 0,
            pdf_catalog_openaction: 0,
            pdf_names_toks: 0,
            pdf_dest_names_ptr: 0,
            dest_names_size: 0,
            dest_names: __arena.arr(__r_dest_names, 0),
            pk_dpi: 0,
            image_orig_x: 0,
            image_orig_y: 0,
            pdf_trailer_toks: 0,
            pdf_trailer_id_toks: 0,
            gen_faked_interword_space: false,
            gen_running_link: false,
            pdf_space_font_name: 0,
            pdf_link_stack: __arena.arr(__r_pdf_link_stack, 10),
            pdf_link_stack_ptr: 0,
            is_shipping_page: false,
            eTeX_mode: 0,
            eof_seen: __arena.arr(__r_eof_seen, 15),
            LR_ptr: 0,
            LR_problems: 0,
            cur_dir: 0,
            pseudo_files: 0,
            grp_stack: __arena.arr(__r_grp_stack, 16),
            if_stack: __arena.arr(__r_if_stack, 16),
            max_reg_num: 0,
            max_reg_help_line: 0,
            sa_root: __arena.arr(__r_sa_root, 7),
            cur_ptr: 0,
            sa_null: memory_word::default(),
            sa_chain: 0,
            sa_level: 0,
            last_line_fill: 0,
            do_last_line_fit: false,
            active_node_size: 0,
            fill_width: __arena.arr(__r_fill_width, 3),
            best_pl_short: __arena.arr(__r_best_pl_short, 4),
            best_pl_glue: __arena.arr(__r_best_pl_glue, 4),
            hyph_start: 0,
            hyph_index: 0,
            disc_ptr: __arena.arr(__r_disc_ptr, 3),
            expand_depth: 0,
            expand_depth_count: 0,
            shellenabledp: false,
            restrictedshell: false,
            mltex_p: false,
            mltex_enabled_p: false,
            error_line: 0,
            half_error_line: 0,
            max_print_line: 0,
            file_line_error_style_p: false,
            halt_on_error_p: false,
            halting_on_error_p: false,
            parse_first_line_p: false,
            dump_line: false,
            eight_bit_p: false,
            translate_filename_p: false,
            ckpt_request: 0,
            ckpt_arm_cs: 0,
            ckpt_arm_level: 0,
            ckpt_resuming: false,
            ckpt_on_shipout: 0,
            ckpt_on_segment: 0,
            rs_on: false,
            rs_seen: __arena.arr(__r_rs_seen, 629930),
            macro_prof_on: false,
            intr_on: false,
            intr_at_switch: false,
            intr_rec_on: false,
            intr_all: false,
            intr_weak: false,
            intr_state: __arena.arr(__r_intr_state, 4096),
            intr_cand: __arena.arr(__r_intr_cand, 29930),
            intr_watch: __arena.arr(__r_intr_watch, 29930),
            intr_seen: __arena.arr(__r_intr_seen, 29930),
            intr_pre: __arena.arr(__r_intr_pre, 29930),
            intr_data: __arena.arr(__r_intr_data, 8388608),
            arena: __arena,
        })
    }

    /// Every scalar global, then every growable array's length, in the order
    /// of `SCALAR_BYTES` (the checkpoint spill and fill).
    pub fn visit_scalars<V: crate::arena::Visit>(&mut self, v: &mut V) {
        v.pod(&mut self.bad);
        v.pod(&mut self.xchr);
        v.pod(&mut self.name_of_file);
        v.pod(&mut self.name_length);
        v.pod(&mut self.first);
        v.pod(&mut self.last);
        v.pod(&mut self.max_buf_stack);
        v.pod(&mut self.pool_ptr);
        v.pod(&mut self.str_ptr);
        v.pod(&mut self.init_pool_ptr);
        v.pod(&mut self.init_str_ptr);
        v.pod(&mut self.selector);
        v.pod(&mut self.tally);
        v.pod(&mut self.term_offset);
        v.pod(&mut self.file_offset);
        v.pod(&mut self.trick_count);
        v.pod(&mut self.first_count);
        v.pod(&mut self.interaction);
        v.pod(&mut self.interaction_option);
        v.pod(&mut self.deletions_allowed);
        v.pod(&mut self.set_box_allowed);
        v.pod(&mut self.history);
        v.pod(&mut self.error_count);
        v.pod(&mut self.help_ptr);
        v.pod(&mut self.use_err_help);
        v.pod(&mut self.interrupt);
        v.pod(&mut self.OK_to_interrupt);
        v.pod(&mut self.arith_error);
        v.pod(&mut self.remainder);
        v.pod(&mut self.j_random);
        v.pod(&mut self.random_seed);
        v.pod(&mut self.temp_ptr);
        v.pod(&mut self.lo_mem_max);
        v.pod(&mut self.hi_mem_min);
        v.pod(&mut self.var_used);
        v.pod(&mut self.dyn_used);
        v.pod(&mut self.avail);
        v.pod(&mut self.mem_end);
        v.pod(&mut self.rover);
        v.pod(&mut self.font_in_short_display);
        v.pod(&mut self.depth_threshold);
        v.pod(&mut self.breadth_max);
        v.pod(&mut self.nest_ptr);
        v.pod(&mut self.max_nest_stack);
        v.pod(&mut self.cur_list);
        v.pod(&mut self.shown_mode);
        v.pod(&mut self.save_tail);
        v.pod(&mut self.prev_tail);
        v.pod(&mut self.old_setting);
        v.pod(&mut self.old_selector_ignored_err);
        v.pod(&mut self.sys_time);
        v.pod(&mut self.sys_day);
        v.pod(&mut self.sys_month);
        v.pod(&mut self.sys_year);
        v.pod(&mut self.hash_used);
        v.pod(&mut self.hash_high);
        v.pod(&mut self.no_new_control_sequence);
        v.pod(&mut self.cs_count);
        v.pod(&mut self.prim_used);
        v.pod(&mut self.save_ptr);
        v.pod(&mut self.max_save_stack);
        v.pod(&mut self.cur_level);
        v.pod(&mut self.cur_group);
        v.pod(&mut self.cur_boundary);
        v.pod(&mut self.mag_set);
        v.pod(&mut self.cur_cmd);
        v.pod(&mut self.cur_chr);
        v.pod(&mut self.cur_cs);
        v.pod(&mut self.cur_tok);
        v.pod(&mut self.input_ptr);
        v.pod(&mut self.max_in_stack);
        v.pod(&mut self.cur_input);
        v.pod(&mut self.in_open);
        v.pod(&mut self.open_parens);
        v.pod(&mut self.line);
        v.pod(&mut self.scanner_status);
        v.pod(&mut self.warning_index);
        v.pod(&mut self.def_ref);
        v.pod(&mut self.param_ptr);
        v.pod(&mut self.max_param_stack);
        v.pod(&mut self.align_state);
        v.pod(&mut self.base_ptr);
        v.pod(&mut self.par_loc);
        v.pod(&mut self.par_token);
        v.pod(&mut self.force_eof);
        v.pod(&mut self.is_in_csname);
        v.pod(&mut self.long_state);
        v.pod(&mut self.cur_val);
        v.pod(&mut self.cur_val_level);
        v.pod(&mut self.radix);
        v.pod(&mut self.cur_order);
        v.pod(&mut self.cond_ptr);
        v.pod(&mut self.if_limit);
        v.pod(&mut self.cur_if);
        v.pod(&mut self.if_line);
        v.pod(&mut self.skip_line);
        v.pod(&mut self.cur_name);
        v.pod(&mut self.cur_area);
        v.pod(&mut self.cur_ext);
        v.pod(&mut self.area_delimiter);
        v.pod(&mut self.ext_delimiter);
        v.pod(&mut self.quoted_filename);
        v.pod(&mut self.stop_at_space);
        v.pod(&mut self.TEX_format_default);
        v.pod(&mut self.name_in_progress);
        v.pod(&mut self.job_name);
        v.pod(&mut self.log_opened);
        v.pod(&mut self.output_file_name);
        v.pod(&mut self.log_name);
        v.pod(&mut self.fmem_ptr);
        v.pod(&mut self.font_ptr);
        v.pod(&mut self.null_character);
        v.pod(&mut self.total_pages);
        v.pod(&mut self.max_v);
        v.pod(&mut self.max_h);
        v.pod(&mut self.max_push);
        v.pod(&mut self.last_bop);
        v.pod(&mut self.dead_cycles);
        v.pod(&mut self.doing_leaders);
        v.pod(&mut self.c);
        v.pod(&mut self.f);
        v.pod(&mut self.rule_ht);
        v.pod(&mut self.rule_dp);
        v.pod(&mut self.rule_wd);
        v.pod(&mut self.g);
        v.pod(&mut self.lq);
        v.pod(&mut self.lr);
        v.pod(&mut self.half_buf);
        v.pod(&mut self.dvi_limit);
        v.pod(&mut self.dvi_ptr);
        v.pod(&mut self.dvi_offset);
        v.pod(&mut self.dvi_gone);
        v.pod(&mut self.down_ptr);
        v.pod(&mut self.right_ptr);
        v.pod(&mut self.dvi_h);
        v.pod(&mut self.dvi_v);
        v.pod(&mut self.cur_h);
        v.pod(&mut self.cur_v);
        v.pod(&mut self.dvi_f);
        v.pod(&mut self.cur_s);
        v.pod(&mut self.pdf_mem_size);
        v.arr_len(&mut self.pdf_mem);
        v.pod(&mut self.pdf_mem_ptr);
        v.pod(&mut self.pdf_buf_is_os);
        v.pod(&mut self.pdf_buf_size);
        v.pod(&mut self.pdf_ptr);
        v.arr_len(&mut self.pdf_op_buf);
        v.arr_len(&mut self.pdf_os_buf);
        v.pod(&mut self.pdf_os_buf_size);
        v.arr_len(&mut self.pdf_os_objnum);
        v.arr_len(&mut self.pdf_os_objoff);
        v.pod(&mut self.pdf_os_objidx);
        v.pod(&mut self.pdf_os_cntr);
        v.pod(&mut self.pdf_op_ptr);
        v.pod(&mut self.pdf_os_ptr);
        v.pod(&mut self.pdf_os_mode);
        v.pod(&mut self.pdf_os_enable);
        v.pod(&mut self.pdf_os_cur_objnum);
        v.pod(&mut self.pdf_gone);
        v.pod(&mut self.pdf_save_offset);
        v.pod(&mut self.zip_write_state);
        v.pod(&mut self.fixed_pdf_major_version);
        v.pod(&mut self.fixed_pdf_minor_version);
        v.pod(&mut self.fixed_pdf_objcompresslevel);
        v.pod(&mut self.pdf_version_written);
        v.pod(&mut self.fixed_pdfoutput);
        v.pod(&mut self.fixed_pdfoutput_set);
        v.pod(&mut self.fixed_gamma);
        v.pod(&mut self.fixed_image_gamma);
        v.pod(&mut self.fixed_image_hicolor);
        v.pod(&mut self.fixed_image_apply_gamma);
        v.pod(&mut self.epochseconds);
        v.pod(&mut self.microseconds);
        v.pod(&mut self.fixed_pdf_draftmode);
        v.pod(&mut self.fixed_pdf_draftmode_set);
        v.pod(&mut self.pdf_page_group_val);
        v.pod(&mut self.one_bp);
        v.pod(&mut self.one_hundred_bp);
        v.pod(&mut self.one_hundred_inch);
        v.pod(&mut self.one_inch);
        v.pod(&mut self.scaled_out);
        v.pod(&mut self.init_pdf_output);
        v.pod(&mut self.adv_char_width_s);
        v.pod(&mut self.adv_char_width_s_out);
        v.pod(&mut self.pdf_f);
        v.pod(&mut self.pdf_h);
        v.pod(&mut self.pdf_v);
        v.pod(&mut self.pdf_tj_start_h);
        v.pod(&mut self.cur_delta_h);
        v.pod(&mut self.pdf_delta_h);
        v.pod(&mut self.pdf_origin_h);
        v.pod(&mut self.pdf_origin_v);
        v.pod(&mut self.pdf_doing_string);
        v.pod(&mut self.pdf_doing_text);
        v.pod(&mut self.min_bp_val);
        v.pod(&mut self.min_font_val);
        v.pod(&mut self.fixed_pk_resolution);
        v.pod(&mut self.fixed_decimal_digits);
        v.pod(&mut self.fixed_gen_tounicode);
        v.pod(&mut self.fixed_inclusion_copy_font);
        v.pod(&mut self.pk_scale_factor);
        v.pod(&mut self.pdf_output_option);
        v.pod(&mut self.pdf_output_value);
        v.pod(&mut self.pdf_draftmode_option);
        v.pod(&mut self.pdf_draftmode_value);
        v.pod(&mut self.pdf_cur_Tm_a);
        v.pod(&mut self.pdf_last_f);
        v.pod(&mut self.pdf_last_fs);
        v.pod(&mut self.pdf_dummy_font);
        v.pod(&mut self.obj_tab_size);
        v.arr_len(&mut self.obj_tab);
        v.pod(&mut self.pages_tail);
        v.pod(&mut self.obj_ptr);
        v.pod(&mut self.sys_obj_ptr);
        v.pod(&mut self.pdf_last_pages);
        v.pod(&mut self.pdf_last_page);
        v.pod(&mut self.pdf_last_stream);
        v.pod(&mut self.pdf_stream_length);
        v.pod(&mut self.pdf_stream_length_offset);
        v.pod(&mut self.pdf_seek_write_length);
        v.pod(&mut self.pdf_last_byte);
        v.pod(&mut self.pdf_append_list_arg);
        v.pod(&mut self.ff);
        v.pod(&mut self.pdf_box_spec_media);
        v.pod(&mut self.pdf_box_spec_crop);
        v.pod(&mut self.pdf_box_spec_bleed);
        v.pod(&mut self.pdf_box_spec_trim);
        v.pod(&mut self.pdf_box_spec_art);
        v.pod(&mut self.pdf_image_procset);
        v.pod(&mut self.pdf_text_procset);
        v.arr_len(&mut self.pdf_font_type);
        v.arr_len(&mut self.pdf_font_attr);
        v.arr_len(&mut self.pdf_font_nobuiltin_tounicode);
        v.arr_len(&mut self.pdf_char_used);
        v.arr_len(&mut self.pdf_font_size);
        v.arr_len(&mut self.pdf_font_num);
        v.arr_len(&mut self.pdf_font_map);
        v.pod(&mut self.pdf_font_list);
        v.pod(&mut self.pdf_resname_prefix);
        v.pod(&mut self.last_tokens_string);
        v.arr_len(&mut self.vf_packet_base);
        v.arr_len(&mut self.vf_default_font);
        v.arr_len(&mut self.vf_local_font_num);
        v.pod(&mut self.vf_packet_length);
        v.pod(&mut self.vf_nf);
        v.arr_len(&mut self.vf_e_fnts);
        v.arr_len(&mut self.vf_i_fnts);
        v.pod(&mut self.tmp_w);
        v.pod(&mut self.vf_cur_s);
        v.pod(&mut self.vf_stack_ptr);
        v.pod(&mut self.saved_pdf_cur_form);
        v.pod(&mut self.pdftex_banner);
        v.pod(&mut self.last_badness);
        v.pod(&mut self.adjust_tail);
        v.arr_len(&mut self.pdf_font_blink);
        v.arr_len(&mut self.pdf_font_elink);
        v.arr_len(&mut self.pdf_font_has_space_char);
        v.arr_len(&mut self.pdf_font_stretch);
        v.arr_len(&mut self.pdf_font_shrink);
        v.arr_len(&mut self.pdf_font_step);
        v.arr_len(&mut self.pdf_font_expand_ratio);
        v.arr_len(&mut self.pdf_font_auto_expand);
        v.arr_len(&mut self.pdf_font_lp_base);
        v.arr_len(&mut self.pdf_font_rp_base);
        v.arr_len(&mut self.pdf_font_ef_base);
        v.arr_len(&mut self.pdf_font_kn_bs_base);
        v.arr_len(&mut self.pdf_font_st_bs_base);
        v.arr_len(&mut self.pdf_font_sh_bs_base);
        v.arr_len(&mut self.pdf_font_kn_bc_base);
        v.arr_len(&mut self.pdf_font_kn_ac_base);
        v.pod(&mut self.font_expand_ratio);
        v.pod(&mut self.last_leftmost_char);
        v.pod(&mut self.last_rightmost_char);
        v.pod(&mut self.hlist_stack_level);
        v.pod(&mut self.pre_adjust_tail);
        v.pod(&mut self.pack_begin_line);
        v.pod(&mut self.empty_field);
        v.pod(&mut self.null_delimiter);
        v.pod(&mut self.cur_mlist);
        v.pod(&mut self.cur_style);
        v.pod(&mut self.cur_size);
        v.pod(&mut self.cur_mu);
        v.pod(&mut self.mlist_penalties);
        v.pod(&mut self.cur_f);
        v.pod(&mut self.cur_c);
        v.pod(&mut self.cur_i);
        v.pod(&mut self.magic_offset);
        v.pod(&mut self.cur_align);
        v.pod(&mut self.cur_span);
        v.pod(&mut self.cur_loop);
        v.pod(&mut self.align_ptr);
        v.pod(&mut self.cur_head);
        v.pod(&mut self.cur_tail);
        v.pod(&mut self.cur_pre_head);
        v.pod(&mut self.cur_pre_tail);
        v.pod(&mut self.just_box);
        v.pod(&mut self.passive);
        v.pod(&mut self.printed_node);
        v.pod(&mut self.pass_number);
        v.pod(&mut self.auto_breaking);
        v.pod(&mut self.prev_p);
        v.pod(&mut self.first_p);
        v.pod(&mut self.prev_char_p);
        v.pod(&mut self.next_char_p);
        v.pod(&mut self.try_prev_break);
        v.pod(&mut self.prev_legal);
        v.pod(&mut self.prev_prev_legal);
        v.pod(&mut self.prev_auto_breaking);
        v.pod(&mut self.rejected_cur_p);
        v.pod(&mut self.before_rejected_cur_p);
        v.pod(&mut self.max_stretch_ratio);
        v.pod(&mut self.max_shrink_ratio);
        v.pod(&mut self.cur_font_step);
        v.pod(&mut self.no_shrink_error_yet);
        v.pod(&mut self.cur_p);
        v.pod(&mut self.second_pass);
        v.pod(&mut self.final_pass);
        v.pod(&mut self.threshold);
        v.pod(&mut self.minimum_demerits);
        v.pod(&mut self.easy_line);
        v.pod(&mut self.last_special_line);
        v.pod(&mut self.first_width);
        v.pod(&mut self.second_width);
        v.pod(&mut self.first_indent);
        v.pod(&mut self.second_indent);
        v.pod(&mut self.best_bet);
        v.pod(&mut self.fewest_demerits);
        v.pod(&mut self.best_line);
        v.pod(&mut self.actual_looseness);
        v.pod(&mut self.line_diff);
        v.pod(&mut self.hn);
        v.pod(&mut self.ha);
        v.pod(&mut self.hb);
        v.pod(&mut self.hf);
        v.pod(&mut self.hyf_char);
        v.pod(&mut self.cur_lang);
        v.pod(&mut self.init_cur_lang);
        v.pod(&mut self.l_hyf);
        v.pod(&mut self.r_hyf);
        v.pod(&mut self.init_l_hyf);
        v.pod(&mut self.init_r_hyf);
        v.pod(&mut self.hyf_bchar);
        v.pod(&mut self.init_list);
        v.pod(&mut self.init_lig);
        v.pod(&mut self.init_lft);
        v.pod(&mut self.hyphen_passed);
        v.pod(&mut self.cur_l);
        v.pod(&mut self.cur_r);
        v.pod(&mut self.cur_q);
        v.pod(&mut self.lig_stack);
        v.pod(&mut self.ligature_present);
        v.pod(&mut self.lft_hit);
        v.pod(&mut self.rt_hit);
        v.pod(&mut self.hyph_count);
        v.pod(&mut self.trie_op_ptr);
        v.pod(&mut self.trie_ptr);
        v.pod(&mut self.trie_max);
        v.pod(&mut self.trie_not_ready);
        v.pod(&mut self.best_height_plus_depth);
        v.pod(&mut self.page_tail);
        v.pod(&mut self.page_contents);
        v.pod(&mut self.page_max_depth);
        v.pod(&mut self.best_page_break);
        v.pod(&mut self.least_page_cost);
        v.pod(&mut self.best_size);
        v.pod(&mut self.last_glue);
        v.pod(&mut self.last_penalty);
        v.pod(&mut self.last_kern);
        v.pod(&mut self.last_node_type);
        v.pod(&mut self.insert_penalties);
        v.pod(&mut self.output_active);
        v.pod(&mut self.output_can_end);
        v.pod(&mut self.main_f);
        v.pod(&mut self.main_i);
        v.pod(&mut self.main_j);
        v.pod(&mut self.main_k);
        v.pod(&mut self.main_p);
        v.pod(&mut self.main_s);
        v.pod(&mut self.bchar);
        v.pod(&mut self.false_bchar);
        v.pod(&mut self.cancel_boundary);
        v.pod(&mut self.ins_disc);
        v.pod(&mut self.cur_box);
        v.pod(&mut self.after_token);
        v.pod(&mut self.long_help_seen);
        v.pod(&mut self.format_ident);
        v.pod(&mut self.ready_already);
        v.pod(&mut self.write_loc);
        v.pod(&mut self.pdf_last_obj);
        v.pod(&mut self.pdf_last_xform);
        v.pod(&mut self.pdf_last_ximage);
        v.pod(&mut self.pdf_last_ximage_pages);
        v.pod(&mut self.pdf_last_ximage_colordepth);
        v.pod(&mut self.alt_rule);
        v.pod(&mut self.warn_pdfpagebox);
        v.pod(&mut self.pdf_last_annot);
        v.pod(&mut self.pdf_last_link);
        v.pod(&mut self.pdf_last_x_pos);
        v.pod(&mut self.pdf_last_y_pos);
        v.pod(&mut self.pdf_snapx_refpos);
        v.pod(&mut self.pdf_snapy_refpos);
        v.pod(&mut self.count_do_snapy);
        v.pod(&mut self.pdf_retval);
        v.pod(&mut self.cur_page_width);
        v.pod(&mut self.cur_page_height);
        v.pod(&mut self.cur_h_offset);
        v.pod(&mut self.cur_v_offset);
        v.pod(&mut self.pdf_obj_list);
        v.pod(&mut self.pdf_xform_list);
        v.pod(&mut self.pdf_ximage_list);
        v.pod(&mut self.last_thread);
        v.pod(&mut self.pdf_thread_ht);
        v.pod(&mut self.pdf_thread_dp);
        v.pod(&mut self.pdf_thread_wd);
        v.pod(&mut self.pdf_last_thread_id);
        v.pod(&mut self.pdf_last_thread_named_id);
        v.pod(&mut self.pdf_thread_level);
        v.pod(&mut self.pdf_annot_list);
        v.pod(&mut self.pdf_link_list);
        v.pod(&mut self.pdf_dest_list);
        v.pod(&mut self.pdf_bead_list);
        v.pod(&mut self.pdf_obj_count);
        v.pod(&mut self.pdf_xform_count);
        v.pod(&mut self.pdf_ximage_count);
        v.pod(&mut self.pdf_cur_form);
        v.pod(&mut self.pdf_first_outline);
        v.pod(&mut self.pdf_last_outline);
        v.pod(&mut self.pdf_parent_outline);
        v.pod(&mut self.pdf_xform_width);
        v.pod(&mut self.pdf_xform_height);
        v.pod(&mut self.pdf_xform_depth);
        v.pod(&mut self.pdf_info_toks);
        v.pod(&mut self.pdf_catalog_toks);
        v.pod(&mut self.pdf_catalog_openaction);
        v.pod(&mut self.pdf_names_toks);
        v.pod(&mut self.pdf_dest_names_ptr);
        v.pod(&mut self.dest_names_size);
        v.arr_len(&mut self.dest_names);
        v.pod(&mut self.pk_dpi);
        v.pod(&mut self.image_orig_x);
        v.pod(&mut self.image_orig_y);
        v.pod(&mut self.pdf_trailer_toks);
        v.pod(&mut self.pdf_trailer_id_toks);
        v.pod(&mut self.gen_faked_interword_space);
        v.pod(&mut self.gen_running_link);
        v.pod(&mut self.pdf_space_font_name);
        v.pod(&mut self.pdf_link_stack_ptr);
        v.pod(&mut self.is_shipping_page);
        v.pod(&mut self.eTeX_mode);
        v.pod(&mut self.LR_ptr);
        v.pod(&mut self.LR_problems);
        v.pod(&mut self.cur_dir);
        v.pod(&mut self.pseudo_files);
        v.pod(&mut self.max_reg_num);
        v.pod(&mut self.max_reg_help_line);
        v.pod(&mut self.cur_ptr);
        v.pod(&mut self.sa_null);
        v.pod(&mut self.sa_chain);
        v.pod(&mut self.sa_level);
        v.pod(&mut self.last_line_fill);
        v.pod(&mut self.do_last_line_fit);
        v.pod(&mut self.active_node_size);
        v.pod(&mut self.hyph_start);
        v.pod(&mut self.hyph_index);
        v.pod(&mut self.expand_depth);
        v.pod(&mut self.expand_depth_count);
        v.pod(&mut self.shellenabledp);
        v.pod(&mut self.restrictedshell);
        v.pod(&mut self.mltex_p);
        v.pod(&mut self.mltex_enabled_p);
        v.pod(&mut self.error_line);
        v.pod(&mut self.half_error_line);
        v.pod(&mut self.max_print_line);
        v.pod(&mut self.file_line_error_style_p);
        v.pod(&mut self.halt_on_error_p);
        v.pod(&mut self.halting_on_error_p);
        v.pod(&mut self.parse_first_line_p);
        v.pod(&mut self.dump_line);
        v.pod(&mut self.eight_bit_p);
        v.pod(&mut self.translate_filename_p);
        v.pod(&mut self.ckpt_request);
        v.pod(&mut self.ckpt_arm_cs);
        v.pod(&mut self.ckpt_arm_level);
        v.pod(&mut self.ckpt_resuming);
        v.pod(&mut self.ckpt_on_shipout);
        v.pod(&mut self.ckpt_on_segment);
        v.pod(&mut self.rs_on);
        v.pod(&mut self.macro_prof_on);
        v.pod(&mut self.intr_on);
        v.pod(&mut self.intr_at_switch);
        v.pod(&mut self.intr_rec_on);
        v.pod(&mut self.intr_all);
        v.pod(&mut self.intr_weak);
    }

    /// Every file global, in declaration order (the checkpoint's host-state record).
    pub fn visit_files<V: crate::system::FileVisit>(&mut self, v: &mut V) {
        v.alpha(&mut self.term_in);
        v.alpha(&mut self.term_out);
        v.alpha(&mut self.pool_file);
        v.alpha(&mut self.log_file);
        for f in self.input_file.iter_mut() {
            v.alpha(f);
        }
        for f in self.read_file.iter_mut() {
            v.alpha(f);
        }
        v.byte(&mut self.dvi_file);
        v.byte(&mut self.tfm_file);
        v.byte(&mut self.pdf_file);
        v.byte(&mut self.vf_file);
        v.word(&mut self.fmt_file);
        for f in self.write_file.iter_mut() {
            v.alpha(f);
        }
    }
}
