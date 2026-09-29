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
    pub xord: Vec<ASCII_code>,
    // §20
    pub xchr: [u8; 256],
    // §20
    pub xprn: Vec<bool>,
    // §26
    pub name_of_file: [u8; 1024],
    // §26
    pub name_length: i32,
    // §30
    pub buffer: Vec<ASCII_code>,
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
    pub str_pool: Vec<packed_ASCII_code>,
    // §39
    pub str_start: Vec<pool_pointer>,
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
    pub dig: Vec<i32>,
    // §54
    pub tally: i32,
    // §54
    pub term_offset: i32,
    // §54
    pub file_offset: i32,
    // §54
    pub trick_buf: Vec<ASCII_code>,
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
    pub help_line: Vec<str_number>,
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
    pub randoms: Vec<i32>,
    // §110
    pub j_random: i32,
    // §110
    pub random_seed: scaled,
    // §117
    pub two_to_the: Vec<i32>,
    // §117
    pub spec_log: Vec<i32>,
    // §133
    pub temp_ptr: halfword,
    // §134
    pub mem: Vec<memory_word>,
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
    pub nest: Vec<list_state_record>,
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
    pub eqtb: Vec<memory_word>,
    // §271
    pub xeq_level: Vec<quarterword>,
    // §274
    pub hash: Vec<two_halves>,
    // §274
    pub hash_used: halfword,
    // §274
    pub no_new_control_sequence: bool,
    // §274
    pub cs_count: i32,
    // §275
    pub prim: Vec<two_halves>,
    // §275
    pub prim_used: halfword,
    // §293
    pub save_stack: Vec<memory_word>,
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
    pub input_stack: Vec<in_state_record>,
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
    pub line_stack: Vec<i32>,
    // §327
    pub scanner_status: i32,
    // §327
    pub warning_index: halfword,
    // §327
    pub def_ref: halfword,
    // §330
    pub param_stack: Vec<halfword>,
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
    pub cur_mark: Vec<halfword>,
    // §413
    pub long_state: i32,
    // §414
    pub pstack: Vec<halfword>,
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
    pub read_open: Vec<i32>,
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
    pub full_source_filename_stack: Vec<str_number>,
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
    pub font_info: Vec<memory_word>,
    // §575
    pub fmem_ptr: font_index,
    // §575
    pub font_ptr: internal_font_number,
    // §575
    pub font_check: Vec<four_quarters>,
    // §575
    pub font_size: Vec<scaled>,
    // §575
    pub font_dsize: Vec<scaled>,
    // §575
    pub font_params: Vec<font_index>,
    // §575
    pub font_name: Vec<str_number>,
    // §575
    pub font_area: Vec<str_number>,
    // §575
    pub font_bc: Vec<eight_bits>,
    // §575
    pub font_ec: Vec<eight_bits>,
    // §575
    pub font_glue: Vec<halfword>,
    // §575
    pub font_used: Vec<bool>,
    // §575
    pub hyphen_char: Vec<i32>,
    // §575
    pub skew_char: Vec<i32>,
    // §575
    pub bchar_label: Vec<font_index>,
    // §575
    pub font_bchar: Vec<i32>,
    // §575
    pub font_false_bchar: Vec<i32>,
    // §576
    pub char_base: Vec<i32>,
    // §576
    pub width_base: Vec<i32>,
    // §576
    pub height_base: Vec<i32>,
    // §576
    pub depth_base: Vec<i32>,
    // §576
    pub italic_base: Vec<i32>,
    // §576
    pub lig_kern_base: Vec<i32>,
    // §576
    pub kern_base: Vec<i32>,
    // §576
    pub exten_base: Vec<i32>,
    // §576
    pub param_base: Vec<i32>,
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
    pub dvi_buf: Vec<eight_bits>,
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
    pub pdf_mem: Vec<i32>,
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
    pub pdf_op_buf: Vec<eight_bits>,
    // §680
    pub pdf_os_buf: Vec<eight_bits>,
    // §680
    pub pdf_os_buf_size: i32,
    // §680
    pub pdf_os_objnum: Vec<i32>,
    // §680
    pub pdf_os_objoff: Vec<i32>,
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
    pub ten_pow: Vec<i32>,
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
    pub obj_tab: Vec<obj_entry>,
    // §696
    pub head_tab: Vec<i32>,
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
    pub pdf_font_type: Vec<eight_bits>,
    // §704
    pub pdf_font_attr: Vec<str_number>,
    // §704
    pub pdf_font_nobuiltin_tounicode: Vec<bool>,
    // §708
    pub pdf_char_used: Vec<char_used_array>,
    // §708
    pub pdf_font_size: Vec<scaled>,
    // §708
    pub pdf_font_num: Vec<i32>,
    // §708
    pub pdf_font_map: Vec<fm_entry_ptr>,
    // §708
    pub pdf_font_list: halfword,
    // §708
    pub pdf_resname_prefix: str_number,
    // §708
    pub last_tokens_string: str_number,
    // §710
    pub vf_packet_base: Vec<i32>,
    // §710
    pub vf_default_font: Vec<internal_font_number>,
    // §710
    pub vf_local_font_num: Vec<internal_font_number>,
    // §710
    pub vf_packet_length: i32,
    // §710
    pub vf_file: crate::system::ByteFile,
    // §710
    pub vf_nf: internal_font_number,
    // §710
    pub vf_e_fnts: Vec<i32>,
    // §710
    pub vf_i_fnts: Vec<internal_font_number>,
    // §710
    pub tmp_w: memory_word,
    // §723
    pub vf_cur_s: i32,
    // §723
    pub vf_stack: Vec<vf_stack_record>,
    // §723
    pub vf_stack_ptr: vf_stack_index,
    // §774
    pub saved_pdf_cur_form: i32,
    // §811
    pub pdftex_banner: str_number,
    // §818
    pub total_stretch: Vec<scaled>,
    // §818
    pub total_shrink: Vec<scaled>,
    // §818
    pub last_badness: i32,
    // §819
    pub adjust_tail: halfword,
    // §821
    pub pdf_font_blink: Vec<internal_font_number>,
    // §821
    pub pdf_font_elink: Vec<internal_font_number>,
    // §821
    pub pdf_font_has_space_char: Vec<bool>,
    // §821
    pub pdf_font_stretch: Vec<i32>,
    // §821
    pub pdf_font_shrink: Vec<i32>,
    // §821
    pub pdf_font_step: Vec<i32>,
    // §821
    pub pdf_font_expand_ratio: Vec<i32>,
    // §821
    pub pdf_font_auto_expand: Vec<bool>,
    // §821
    pub pdf_font_lp_base: Vec<i32>,
    // §821
    pub pdf_font_rp_base: Vec<i32>,
    // §821
    pub pdf_font_ef_base: Vec<i32>,
    // §821
    pub pdf_font_kn_bs_base: Vec<i32>,
    // §821
    pub pdf_font_st_bs_base: Vec<i32>,
    // §821
    pub pdf_font_sh_bs_base: Vec<i32>,
    // §821
    pub pdf_font_kn_bc_base: Vec<i32>,
    // §821
    pub pdf_font_kn_ac_base: Vec<i32>,
    // §821
    pub font_expand_ratio: i32,
    // §821
    pub last_leftmost_char: halfword,
    // §821
    pub last_rightmost_char: halfword,
    // §821
    pub hlist_stack: Vec<halfword>,
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
    pub active_width: Vec<scaled>,
    // §999
    pub cur_active_width: Vec<scaled>,
    // §999
    pub background: Vec<scaled>,
    // §999
    pub break_width: Vec<scaled>,
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
    pub prev_active_width: Vec<scaled>,
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
    pub minimal_demerits: Vec<i32>,
    // §1009
    pub minimum_demerits: i32,
    // §1009
    pub best_place: Vec<halfword>,
    // §1009
    pub best_pl_line: Vec<halfword>,
    // §1015
    pub disc_width: Vec<scaled>,
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
    pub hc: Vec<i32>,
    // §1069
    pub hn: i32,
    // §1069
    pub ha: halfword,
    // §1069
    pub hb: halfword,
    // §1069
    pub hf: internal_font_number,
    // §1069
    pub hu: Vec<i32>,
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
    pub hyf: Vec<i32>,
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
    pub trie: Vec<two_halves>,
    // §1098
    pub hyf_distance: Vec<small_number>,
    // §1098
    pub hyf_num: Vec<small_number>,
    // §1098
    pub hyf_next: Vec<quarterword>,
    // §1098
    pub op_start: Vec<i32>,
    // §1103
    pub hyph_word: Vec<str_number>,
    // §1103
    pub hyph_list: Vec<halfword>,
    // §1103
    pub hyph_count: hyph_pointer,
    // §1120
    pub trie_op_hash: Vec<i32>,
    // §1120
    pub trie_used: Vec<quarterword>,
    // §1120
    pub trie_op_lang: Vec<ASCII_code>,
    // §1120
    pub trie_op_val: Vec<quarterword>,
    // §1120
    pub trie_op_ptr: i32,
    // §1124
    pub trie_c: Vec<packed_ASCII_code>,
    // §1124
    pub trie_o: Vec<quarterword>,
    // §1124
    pub trie_l: Vec<trie_pointer>,
    // §1124
    pub trie_r: Vec<trie_pointer>,
    // §1124
    pub trie_ptr: trie_pointer,
    // §1124
    pub trie_hash: Vec<trie_pointer>,
    // §1127
    pub trie_taken: Vec<bool>,
    // §1127
    pub trie_min: Vec<trie_pointer>,
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
    pub page_so_far: Vec<scaled>,
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
    pub write_open: Vec<bool>,
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
    pub dest_names: Vec<dest_name_entry>,
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
    pub pdf_link_stack: Vec<pdf_link_stack_record>,
    // §1633
    pub pdf_link_stack_ptr: small_number,
    // §1640
    pub is_shipping_page: bool,
    // §1652
    pub eTeX_mode: i32,
    // §1660
    pub eof_seen: Vec<bool>,
    // §1705
    pub LR_ptr: halfword,
    // §1705
    pub LR_problems: i32,
    // §1705
    pub cur_dir: small_number,
    // §1750
    pub pseudo_files: halfword,
    // §1773
    pub grp_stack: Vec<save_pointer>,
    // §1773
    pub if_stack: Vec<halfword>,
    // §1814
    pub max_reg_num: halfword,
    // §1814
    pub max_reg_help_line: str_number,
    // §1816
    pub sa_root: Vec<halfword>,
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
    pub fill_width: Vec<scaled>,
    // §1842
    pub best_pl_short: Vec<scaled>,
    // §1842
    pub best_pl_glue: Vec<scaled>,
    // §1858
    pub hyph_start: trie_pointer,
    // §1858
    pub hyph_index: trie_pointer,
    // §1859
    pub disc_ptr: Vec<halfword>,
    // §1870
    pub expand_depth: i32,
    // §1870
    pub expand_depth_count: i32,
    // §1870
    pub shellenabledp: bool,
    // §1870
    pub restrictedshell: bool,
    // §1879
    pub mltex_p: bool,
    // §1879
    pub mltex_enabled_p: bool,
    // §1884
    pub error_line: i32,
    // §1884
    pub half_error_line: i32,
    // §1884
    pub max_print_line: i32,
    // §1884
    pub file_line_error_style_p: bool,
    // §1884
    pub halt_on_error_p: bool,
    // §1884
    pub halting_on_error_p: bool,
    // §1884
    pub parse_first_line_p: bool,
    // §1884
    pub dump_line: bool,
    // §1884
    pub eight_bit_p: bool,
    // §1884
    pub translate_filename_p: bool,
}

impl Globals {
    pub fn new() -> Box<Globals> {
        Box::new(Globals {
            bad: 0,
            xord: vec![0; 256],
            xchr: [0u8; 256],
            xprn: vec![false; 256],
            name_of_file: [0u8; 1024],
            name_length: 0,
            buffer: vec![0; 200001],
            first: 0,
            last: 0,
            max_buf_stack: 0,
            term_in: Default::default(),
            term_out: Default::default(),
            str_pool: vec![0; 6250001],
            str_start: vec![0; 500001],
            pool_ptr: 0,
            str_ptr: 0,
            init_pool_ptr: 0,
            init_str_ptr: 0,
            pool_file: Default::default(),
            log_file: Default::default(),
            selector: 0,
            dig: vec![0; 23],
            tally: 0,
            term_offset: 0,
            file_offset: 0,
            trick_buf: vec![0; 256],
            trick_count: 0,
            first_count: 0,
            interaction: 0,
            interaction_option: 0,
            deletions_allowed: false,
            set_box_allowed: false,
            history: 0,
            error_count: 0,
            help_line: vec![0; 6],
            help_ptr: 0,
            use_err_help: false,
            interrupt: 0,
            OK_to_interrupt: false,
            arith_error: false,
            remainder: 0,
            randoms: vec![0; 55],
            j_random: 0,
            random_seed: 0,
            two_to_the: vec![0; 31],
            spec_log: vec![0; 28],
            temp_ptr: 0,
            mem: vec![memory_word::default(); 5000000],
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
            nest: vec![list_state_record::default(); 1001],
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
            eqtb: vec![memory_word::default(); 629929],
            xeq_level: vec![0; 912],
            hash: vec![two_halves::default(); 626113],
            hash_used: 0,
            no_new_control_sequence: false,
            cs_count: 0,
            prim: vec![two_halves::default(); 2101],
            prim_used: 0,
            save_stack: vec![memory_word::default(); 200001],
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
            input_stack: vec![in_state_record::default(); 10001],
            input_ptr: 0,
            max_in_stack: 0,
            cur_input: in_state_record::default(),
            in_open: 0,
            open_parens: 0,
            input_file: (0..15).map(|_| Default::default()).collect::<Vec<_>>(),
            line: 0,
            line_stack: vec![0; 15],
            scanner_status: 0,
            warning_index: 0,
            def_ref: 0,
            param_stack: vec![0; 20001],
            param_ptr: 0,
            max_param_stack: 0,
            align_state: 0,
            base_ptr: 0,
            par_loc: 0,
            par_token: 0,
            force_eof: false,
            is_in_csname: false,
            cur_mark: vec![0; 5],
            long_state: 0,
            pstack: vec![0; 9],
            cur_val: 0,
            cur_val_level: 0,
            radix: 0,
            cur_order: 0,
            read_file: (0..16).map(|_| Default::default()).collect::<Vec<_>>(),
            read_open: vec![0; 17],
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
            full_source_filename_stack: vec![0; 16],
            TEX_format_default: [0u8; 20],
            name_in_progress: false,
            job_name: 0,
            log_opened: false,
            dvi_file: Default::default(),
            output_file_name: 0,
            log_name: 0,
            tfm_file: Default::default(),
            font_info: vec![memory_word::default(); 8000001],
            fmem_ptr: 0,
            font_ptr: 0,
            font_check: vec![four_quarters::default(); 9001],
            font_size: vec![0; 9001],
            font_dsize: vec![0; 9001],
            font_params: vec![0; 9001],
            font_name: vec![0; 9001],
            font_area: vec![0; 9001],
            font_bc: vec![0; 9001],
            font_ec: vec![0; 9001],
            font_glue: vec![0; 9001],
            font_used: vec![false; 9001],
            hyphen_char: vec![0; 9001],
            skew_char: vec![0; 9001],
            bchar_label: vec![0; 9001],
            font_bchar: vec![0; 9001],
            font_false_bchar: vec![0; 9001],
            char_base: vec![0; 9001],
            width_base: vec![0; 9001],
            height_base: vec![0; 9001],
            depth_base: vec![0; 9001],
            italic_base: vec![0; 9001],
            lig_kern_base: vec![0; 9001],
            kern_base: vec![0; 9001],
            exten_base: vec![0; 9001],
            param_base: vec![0; 9001],
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
            dvi_buf: vec![0; 16385],
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
            pdf_mem: Vec::new(),
            pdf_mem_ptr: 0,
            pdf_file: Default::default(),
            pdf_buf_is_os: false,
            pdf_buf_size: 0,
            pdf_ptr: 0,
            pdf_op_buf: Vec::new(),
            pdf_os_buf: Vec::new(),
            pdf_os_buf_size: 0,
            pdf_os_objnum: Vec::new(),
            pdf_os_objoff: Vec::new(),
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
            ten_pow: vec![0; 10],
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
            obj_tab: Vec::new(),
            head_tab: vec![0; 10],
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
            pdf_font_type: Vec::new(),
            pdf_font_attr: Vec::new(),
            pdf_font_nobuiltin_tounicode: Vec::new(),
            pdf_char_used: Vec::new(),
            pdf_font_size: Vec::new(),
            pdf_font_num: Vec::new(),
            pdf_font_map: Vec::new(),
            pdf_font_list: 0,
            pdf_resname_prefix: 0,
            last_tokens_string: 0,
            vf_packet_base: Vec::new(),
            vf_default_font: Vec::new(),
            vf_local_font_num: Vec::new(),
            vf_packet_length: 0,
            vf_file: Default::default(),
            vf_nf: 0,
            vf_e_fnts: Vec::new(),
            vf_i_fnts: Vec::new(),
            tmp_w: memory_word::default(),
            vf_cur_s: 0,
            vf_stack: vec![vf_stack_record::default(); 101],
            vf_stack_ptr: 0,
            saved_pdf_cur_form: 0,
            pdftex_banner: 0,
            total_stretch: vec![0; 4],
            total_shrink: vec![0; 4],
            last_badness: 0,
            adjust_tail: 0,
            pdf_font_blink: Vec::new(),
            pdf_font_elink: Vec::new(),
            pdf_font_has_space_char: Vec::new(),
            pdf_font_stretch: Vec::new(),
            pdf_font_shrink: Vec::new(),
            pdf_font_step: Vec::new(),
            pdf_font_expand_ratio: Vec::new(),
            pdf_font_auto_expand: Vec::new(),
            pdf_font_lp_base: Vec::new(),
            pdf_font_rp_base: Vec::new(),
            pdf_font_ef_base: Vec::new(),
            pdf_font_kn_bs_base: Vec::new(),
            pdf_font_st_bs_base: Vec::new(),
            pdf_font_sh_bs_base: Vec::new(),
            pdf_font_kn_bc_base: Vec::new(),
            pdf_font_kn_ac_base: Vec::new(),
            font_expand_ratio: 0,
            last_leftmost_char: 0,
            last_rightmost_char: 0,
            hlist_stack: vec![0; 513],
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
            active_width: vec![0; 8],
            cur_active_width: vec![0; 8],
            background: vec![0; 8],
            break_width: vec![0; 8],
            auto_breaking: false,
            prev_p: 0,
            first_p: 0,
            prev_char_p: 0,
            next_char_p: 0,
            try_prev_break: false,
            prev_legal: 0,
            prev_prev_legal: 0,
            prev_auto_breaking: false,
            prev_active_width: vec![0; 8],
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
            minimal_demerits: vec![0; 4],
            minimum_demerits: 0,
            best_place: vec![0; 4],
            best_pl_line: vec![0; 4],
            disc_width: vec![0; 8],
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
            hc: vec![0; 66],
            hn: 0,
            ha: 0,
            hb: 0,
            hf: 0,
            hu: vec![0; 64],
            hyf_char: 0,
            cur_lang: 0,
            init_cur_lang: 0,
            l_hyf: 0,
            r_hyf: 0,
            init_l_hyf: 0,
            init_r_hyf: 0,
            hyf_bchar: 0,
            hyf: vec![0; 65],
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
            trie: vec![two_halves::default(); 1100001],
            hyf_distance: vec![0; 35111],
            hyf_num: vec![0; 35111],
            hyf_next: vec![0; 35111],
            op_start: vec![0; 256],
            hyph_word: vec![0; 8192],
            hyph_list: vec![0; 8192],
            hyph_count: 0,
            trie_op_hash: vec![0; 70223],
            trie_used: vec![0; 256],
            trie_op_lang: vec![0; 35111],
            trie_op_val: vec![0; 35111],
            trie_op_ptr: 0,
            trie_c: vec![0; 1100001],
            trie_o: vec![0; 1100001],
            trie_l: vec![0; 1100001],
            trie_r: vec![0; 1100001],
            trie_ptr: 0,
            trie_hash: vec![0; 1100001],
            trie_taken: vec![false; 1100000],
            trie_min: vec![0; 256],
            trie_max: 0,
            trie_not_ready: false,
            best_height_plus_depth: 0,
            page_tail: 0,
            page_contents: 0,
            page_max_depth: 0,
            best_page_break: 0,
            least_page_cost: 0,
            best_size: 0,
            page_so_far: vec![0; 8],
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
            write_open: vec![false; 18],
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
            dest_names: Vec::new(),
            pk_dpi: 0,
            image_orig_x: 0,
            image_orig_y: 0,
            pdf_trailer_toks: 0,
            pdf_trailer_id_toks: 0,
            gen_faked_interword_space: false,
            gen_running_link: false,
            pdf_space_font_name: 0,
            pdf_link_stack: vec![pdf_link_stack_record::default(); 10],
            pdf_link_stack_ptr: 0,
            is_shipping_page: false,
            eTeX_mode: 0,
            eof_seen: vec![false; 15],
            LR_ptr: 0,
            LR_problems: 0,
            cur_dir: 0,
            pseudo_files: 0,
            grp_stack: vec![0; 16],
            if_stack: vec![0; 16],
            max_reg_num: 0,
            max_reg_help_line: 0,
            sa_root: vec![0; 7],
            cur_ptr: 0,
            sa_null: memory_word::default(),
            sa_chain: 0,
            sa_level: 0,
            last_line_fill: 0,
            do_last_line_fit: false,
            active_node_size: 0,
            fill_width: vec![0; 3],
            best_pl_short: vec![0; 4],
            best_pl_glue: vec![0; 4],
            hyph_start: 0,
            hyph_index: 0,
            disc_ptr: vec![0; 3],
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
        })
    }
}
