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
    pub xchr: [u8; 256],
    // §26
    pub name_of_file: [u8; 1024],
    // §26
    pub name_of_file16: crate::arena::Arr<UTF16_code>,
    // §26
    pub name_length: i32,
    // §26
    pub name_length16: i32,
    // §30
    pub buffer: crate::arena::Arr<UTF16_code>,
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
    pub str_pool: crate::arena::Arr<packed_UTF16_code>,
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
    pub trick_buf: crate::arena::Arr<UnicodeScalar>,
    // §54
    pub trick_count: i32,
    // §54
    pub first_count: i32,
    // §61
    pub doing_special: bool,
    // §61
    pub native_text: crate::arena::Arr<UTF16_code>,
    // §61
    pub native_text_size: i32,
    // §61
    pub native_len: i32,
    // §61
    pub save_native_len: i32,
    // §77
    pub interaction: i32,
    // §77
    pub interaction_option: i32,
    // §80
    pub deletions_allowed: bool,
    // §80
    pub set_box_allowed: bool,
    // §80
    pub history: i32,
    // §80
    pub error_count: i32,
    // §83
    pub help_line: crate::arena::Arr<str_number>,
    // §83
    pub help_ptr: i32,
    // §83
    pub use_err_help: bool,
    // §100
    pub interrupt: i32,
    // §100
    pub OK_to_interrupt: bool,
    // §108
    pub arith_error: bool,
    // §108
    pub save_arith_error: bool,
    // §108
    pub remainder: scaled,
    // §114
    pub randoms: crate::arena::Arr<i32>,
    // §114
    pub j_random: i32,
    // §114
    pub random_seed: scaled,
    // §121
    pub two_to_the: crate::arena::Arr<i32>,
    // §121
    pub spec_log: crate::arena::Arr<i32>,
    // §137
    pub temp_ptr: halfword,
    // §138
    pub mem: crate::arena::Arr<memory_word>,
    // §138
    pub lo_mem_max: halfword,
    // §138
    pub hi_mem_min: halfword,
    // §139
    pub var_used: i32,
    // §139
    pub dyn_used: i32,
    // §140
    pub avail: halfword,
    // §140
    pub mem_end: halfword,
    // §146
    pub rover: halfword,
    // §181
    pub last_leftmost_char: halfword,
    // §181
    pub last_rightmost_char: halfword,
    // §181
    pub hlist_stack: crate::arena::Arr<halfword>,
    // §181
    pub hlist_stack_level: i32,
    // §181
    pub first_p: halfword,
    // §181
    pub global_prev_p: halfword,
    // §199
    pub font_in_short_display: i32,
    // §207
    pub depth_threshold: i32,
    // §207
    pub breadth_max: i32,
    // §239
    pub nest: crate::arena::Arr<list_state_record>,
    // §239
    pub nest_ptr: i32,
    // §239
    pub max_nest_stack: i32,
    // §239
    pub cur_list: list_state_record,
    // §239
    pub shown_mode: i32,
    // §272
    pub old_setting: i32,
    // §272
    pub old_selector_ignored_err: i32,
    // §272
    pub sys_time: i32,
    // §272
    pub sys_day: i32,
    // §272
    pub sys_month: i32,
    // §272
    pub sys_year: i32,
    // §279
    pub eqtb: crate::arena::Arr<memory_word>,
    // §279
    pub xeq_level: crate::arena::Arr<quarterword>,
    // §282
    pub hash: crate::arena::Arr<two_halves>,
    // §282
    pub hash_used: halfword,
    // §282
    pub hash_high: halfword,
    // §282
    pub no_new_control_sequence: bool,
    // §282
    pub cs_count: i32,
    // §283
    pub prim: crate::arena::Arr<two_halves>,
    // §283
    pub prim_used: halfword,
    // §301
    pub save_stack: crate::arena::Arr<memory_word>,
    // §301
    pub save_ptr: i32,
    // §301
    pub max_save_stack: i32,
    // §301
    pub cur_level: quarterword,
    // §301
    pub cur_group: group_code,
    // §301
    pub cur_boundary: i32,
    // §316
    pub mag_set: i32,
    // §327
    pub cur_cmd: eight_bits,
    // §327
    pub cur_chr: halfword,
    // §327
    pub cur_cs: halfword,
    // §327
    pub cur_tok: halfword,
    // §331
    pub input_stack: crate::arena::Arr<in_state_record>,
    // §331
    pub input_ptr: i32,
    // §331
    pub max_in_stack: i32,
    // §331
    pub cur_input: in_state_record,
    // §334
    pub in_open: i32,
    // §334
    pub open_parens: i32,
    // §334
    pub input_file: Vec<crate::system::AlphaFile>,
    // §334
    pub line: i32,
    // §334
    pub line_stack: crate::arena::Arr<i32>,
    // §335
    pub scanner_status: i32,
    // §335
    pub warning_index: halfword,
    // §335
    pub def_ref: halfword,
    // §338
    pub param_stack: crate::arena::Arr<halfword>,
    // §338
    pub param_ptr: i32,
    // §338
    pub max_param_stack: i32,
    // §339
    pub align_state: i32,
    // §340
    pub base_ptr: i32,
    // §363
    pub par_loc: halfword,
    // §363
    pub par_token: halfword,
    // §391
    pub force_eof: bool,
    // §397
    pub is_in_csname: bool,
    // §416
    pub cur_mark: crate::arena::Arr<halfword>,
    // §421
    pub long_state: i32,
    // §422
    pub pstack: crate::arena::Arr<halfword>,
    // §444
    pub cur_val: i32,
    // §444
    pub cur_val1: i32,
    // §444
    pub cur_val_level: i32,
    // §472
    pub radix: small_number,
    // §481
    pub cur_order: glue_ord,
    // §515
    pub read_file: Vec<crate::system::AlphaFile>,
    // §515
    pub read_open: crate::arena::Arr<i32>,
    // §524
    pub cond_ptr: halfword,
    // §524
    pub if_limit: i32,
    // §524
    pub cur_if: small_number,
    // §524
    pub if_line: i32,
    // §528
    pub skip_line: i32,
    // §547
    pub cur_name: str_number,
    // §547
    pub cur_area: str_number,
    // §547
    pub cur_ext: str_number,
    // §548
    pub area_delimiter: pool_pointer,
    // §548
    pub ext_delimiter: pool_pointer,
    // §548
    pub quoted_filename: bool,
    // §548
    pub stop_at_space: bool,
    // §548
    pub full_source_filename_stack: crate::arena::Arr<str_number>,
    // §548
    pub file_name_quote_char: UTF16_code,
    // §555
    pub TEX_format_default: [u8; 20],
    // §562
    pub name_in_progress: bool,
    // §562
    pub job_name: str_number,
    // §562
    pub log_opened: bool,
    // §567
    pub output_file_extension: str_number,
    // §567
    pub no_pdf_output: bool,
    // §567
    pub dvi_file: crate::system::ByteFile,
    // §567
    pub output_file_name: str_number,
    // §567
    pub log_name: str_number,
    // §574
    pub tfm_file: crate::system::ByteFile,
    // §584
    pub font_info: crate::arena::Arr<memory_word>,
    // §584
    pub fmem_ptr: font_index,
    // §584
    pub font_ptr: internal_font_number,
    // §584
    pub font_check: crate::arena::Arr<four_quarters>,
    // §584
    pub font_size: crate::arena::Arr<scaled>,
    // §584
    pub font_dsize: crate::arena::Arr<scaled>,
    // §584
    pub font_params: crate::arena::Arr<font_index>,
    // §584
    pub font_name: crate::arena::Arr<str_number>,
    // §584
    pub font_area: crate::arena::Arr<str_number>,
    // §584
    pub font_bc: crate::arena::Arr<UTF16_code>,
    // §584
    pub font_ec: crate::arena::Arr<UTF16_code>,
    // §584
    pub font_glue: crate::arena::Arr<halfword>,
    // §584
    pub font_used: crate::arena::Arr<bool>,
    // §584
    pub hyphen_char: crate::arena::Arr<i32>,
    // §584
    pub skew_char: crate::arena::Arr<i32>,
    // §584
    pub bchar_label: crate::arena::Arr<font_index>,
    // §584
    pub font_bchar: crate::arena::Arr<i32>,
    // §584
    pub font_false_bchar: crate::arena::Arr<i32>,
    // §584
    pub font_layout_engine: crate::arena::Arr<void_pointer>,
    // §584
    pub font_mapping: crate::arena::Arr<void_pointer>,
    // §584
    pub font_flags: crate::arena::Arr<i32>,
    // §584
    pub font_letter_space: crate::arena::Arr<scaled>,
    // §584
    pub loaded_font_mapping: void_pointer,
    // §584
    pub loaded_font_flags: i32,
    // §584
    pub loaded_font_letter_space: scaled,
    // §584
    pub loaded_font_design_size: scaled,
    // §584
    pub mapped_text: crate::arena::Arr<UTF16_code>,
    // §584
    pub xdv_buffer: crate::arena::Arr<eight_bits>,
    // §585
    pub char_base: crate::arena::Arr<i32>,
    // §585
    pub width_base: crate::arena::Arr<i32>,
    // §585
    pub height_base: crate::arena::Arr<i32>,
    // §585
    pub depth_base: crate::arena::Arr<i32>,
    // §585
    pub italic_base: crate::arena::Arr<i32>,
    // §585
    pub lig_kern_base: crate::arena::Arr<i32>,
    // §585
    pub kern_base: crate::arena::Arr<i32>,
    // §585
    pub exten_base: crate::arena::Arr<i32>,
    // §585
    pub param_base: crate::arena::Arr<i32>,
    // §590
    pub null_character: four_quarters,
    // §628
    pub total_pages: i32,
    // §628
    pub max_v: scaled,
    // §628
    pub max_h: scaled,
    // §628
    pub max_push: i32,
    // §628
    pub last_bop: i32,
    // §628
    pub dead_cycles: i32,
    // §628
    pub doing_leaders: bool,
    // §628
    pub c: quarterword,
    // §628
    pub f: quarterword,
    // §628
    pub rule_ht: scaled,
    // §628
    pub rule_dp: scaled,
    // §628
    pub rule_wd: scaled,
    // §628
    pub g: halfword,
    // §628
    pub lq: i32,
    // §628
    pub lr: i32,
    // §631
    pub dvi_buf: crate::arena::Arr<eight_bits>,
    // §631
    pub half_buf: dvi_index,
    // §631
    pub dvi_limit: dvi_index,
    // §631
    pub dvi_ptr: dvi_index,
    // §631
    pub dvi_offset: i32,
    // §631
    pub dvi_gone: i32,
    // §641
    pub down_ptr: halfword,
    // §641
    pub right_ptr: halfword,
    // §652
    pub dvi_h: scaled,
    // §652
    pub dvi_v: scaled,
    // §652
    pub cur_h: scaled,
    // §652
    pub cur_v: scaled,
    // §652
    pub dvi_f: internal_font_number,
    // §652
    pub cur_s: i32,
    // §682
    pub epochseconds: i32,
    // §682
    pub microseconds: i32,
    // §685
    pub total_stretch: crate::arena::Arr<scaled>,
    // §685
    pub total_shrink: crate::arena::Arr<scaled>,
    // §685
    pub last_badness: i32,
    // §686
    pub adjust_tail: halfword,
    // §695
    pub pre_adjust_tail: halfword,
    // §703
    pub pack_begin_line: i32,
    // §726
    pub empty_field: two_halves,
    // §726
    pub null_delimiter: four_quarters,
    // §762
    pub cur_mlist: halfword,
    // §762
    pub cur_style: small_number,
    // §762
    pub cur_size: i32,
    // §762
    pub cur_mu: scaled,
    // §762
    pub mlist_penalties: bool,
    // §767
    pub cur_f: internal_font_number,
    // §767
    pub cur_c: i32,
    // §767
    pub cur_i: four_quarters,
    // §812
    pub magic_offset: i32,
    // §818
    pub cur_align: halfword,
    // §818
    pub cur_span: halfword,
    // §818
    pub cur_loop: halfword,
    // §818
    pub align_ptr: halfword,
    // §818
    pub cur_head: halfword,
    // §818
    pub cur_tail: halfword,
    // §818
    pub cur_pre_head: halfword,
    // §818
    pub cur_pre_tail: halfword,
    // §862
    pub just_box: halfword,
    // §869
    pub passive: halfword,
    // §869
    pub printed_node: halfword,
    // §869
    pub pass_number: halfword,
    // §871
    pub active_width: crate::arena::Arr<scaled>,
    // §871
    pub cur_active_width: crate::arena::Arr<scaled>,
    // §871
    pub background: crate::arena::Arr<scaled>,
    // §871
    pub break_width: crate::arena::Arr<scaled>,
    // §873
    pub no_shrink_error_yet: bool,
    // §876
    pub cur_p: halfword,
    // §876
    pub second_pass: bool,
    // §876
    pub final_pass: bool,
    // §876
    pub threshold: i32,
    // §881
    pub minimal_demerits: crate::arena::Arr<i32>,
    // §881
    pub minimum_demerits: i32,
    // §881
    pub best_place: crate::arena::Arr<halfword>,
    // §881
    pub best_pl_line: crate::arena::Arr<halfword>,
    // §887
    pub disc_width: scaled,
    // §895
    pub easy_line: halfword,
    // §895
    pub last_special_line: halfword,
    // §895
    pub first_width: scaled,
    // §895
    pub second_width: scaled,
    // §895
    pub first_indent: scaled,
    // §895
    pub second_indent: scaled,
    // §920
    pub best_bet: halfword,
    // §920
    pub fewest_demerits: i32,
    // §920
    pub best_line: halfword,
    // §920
    pub actual_looseness: i32,
    // §920
    pub line_diff: i32,
    // §940
    pub hc: crate::arena::Arr<i32>,
    // §940
    pub hn: small_number,
    // §940
    pub ha: halfword,
    // §940
    pub hb: halfword,
    // §940
    pub hf: internal_font_number,
    // §940
    pub hu: crate::arena::Arr<i32>,
    // §940
    pub hyf_char: i32,
    // §940
    pub cur_lang: i32,
    // §940
    pub init_cur_lang: i32,
    // §940
    pub l_hyf: i32,
    // §940
    pub r_hyf: i32,
    // §940
    pub init_l_hyf: i32,
    // §940
    pub init_r_hyf: i32,
    // §940
    pub hyf_bchar: halfword,
    // §940
    pub max_hyph_char: i32,
    // §953
    pub hyf: crate::arena::Arr<i32>,
    // §953
    pub init_list: halfword,
    // §953
    pub init_lig: bool,
    // §953
    pub init_lft: bool,
    // §959
    pub hyphen_passed: small_number,
    // §961
    pub cur_l: halfword,
    // §961
    pub cur_r: halfword,
    // §961
    pub cur_q: halfword,
    // §961
    pub lig_stack: halfword,
    // §961
    pub ligature_present: bool,
    // §961
    pub lft_hit: bool,
    // §961
    pub rt_hit: bool,
    // §975
    pub trie: crate::arena::Arr<two_halves>,
    // §975
    pub hyf_distance: crate::arena::Arr<small_number>,
    // §975
    pub hyf_num: crate::arena::Arr<small_number>,
    // §975
    pub hyf_next: crate::arena::Arr<quarterword>,
    // §975
    pub op_start: crate::arena::Arr<i32>,
    // §980
    pub hyph_word: crate::arena::Arr<str_number>,
    // §980
    pub hyph_list: crate::arena::Arr<halfword>,
    // §980
    pub hyph_count: hyph_pointer,
    // §997
    pub trie_op_hash: crate::arena::Arr<i32>,
    // §997
    pub trie_used: crate::arena::Arr<quarterword>,
    // §997
    pub trie_op_lang: crate::arena::Arr<i32>,
    // §997
    pub trie_op_val: crate::arena::Arr<quarterword>,
    // §997
    pub trie_op_ptr: i32,
    // §1001
    pub trie_c: crate::arena::Arr<packed_UTF16_code>,
    // §1001
    pub trie_o: crate::arena::Arr<quarterword>,
    // §1001
    pub trie_l: crate::arena::Arr<trie_pointer>,
    // §1001
    pub trie_r: crate::arena::Arr<trie_pointer>,
    // §1001
    pub trie_ptr: trie_pointer,
    // §1001
    pub trie_hash: crate::arena::Arr<trie_pointer>,
    // §1004
    pub trie_taken: crate::arena::Arr<bool>,
    // §1004
    pub trie_min: crate::arena::Arr<trie_pointer>,
    // §1004
    pub trie_max: trie_pointer,
    // §1004
    pub trie_not_ready: bool,
    // §1025
    pub best_height_plus_depth: scaled,
    // §1034
    pub page_tail: halfword,
    // §1034
    pub page_contents: i32,
    // §1034
    pub page_max_depth: scaled,
    // §1034
    pub best_page_break: halfword,
    // §1034
    pub least_page_cost: i32,
    // §1034
    pub best_size: scaled,
    // §1036
    pub page_so_far: crate::arena::Arr<scaled>,
    // §1036
    pub last_glue: halfword,
    // §1036
    pub last_penalty: i32,
    // §1036
    pub last_kern: scaled,
    // §1036
    pub last_node_type: i32,
    // §1036
    pub insert_penalties: i32,
    // §1043
    pub output_active: bool,
    // §1043
    pub output_can_end: bool,
    // §1086
    pub main_f: internal_font_number,
    // §1086
    pub main_i: four_quarters,
    // §1086
    pub main_j: four_quarters,
    // §1086
    pub main_k: font_index,
    // §1086
    pub main_p: halfword,
    // §1086
    pub main_pp: halfword,
    // §1086
    pub main_ppp: halfword,
    // §1086
    pub main_h: halfword,
    // §1086
    pub is_hyph: bool,
    // §1086
    pub space_class: i32,
    // §1086
    pub prev_class: i32,
    // §1086
    pub main_s: i32,
    // §1086
    pub bchar: halfword,
    // §1086
    pub false_bchar: halfword,
    // §1086
    pub cancel_boundary: bool,
    // §1086
    pub ins_disc: bool,
    // §1128
    pub cur_box: halfword,
    // §1320
    pub after_token: halfword,
    // §1335
    pub long_help_seen: bool,
    // §1353
    pub format_ident: str_number,
    // §1359
    pub fmt_file: crate::system::WordFile,
    // §1385
    pub ready_already: i32,
    // §1396
    pub write_file: Vec<crate::system::AlphaFile>,
    // §1396
    pub write_open: crate::arena::Arr<bool>,
    // §1400
    pub write_loc: halfword,
    // §1429
    pub cur_page_width: scaled,
    // §1429
    pub cur_page_height: scaled,
    // §1429
    pub cur_h_offset: scaled,
    // §1429
    pub cur_v_offset: scaled,
    // §1449
    pub pdf_last_x_pos: i32,
    // §1449
    pub pdf_last_y_pos: i32,
    // §1462
    pub eTeX_mode: i32,
    // §1470
    pub eof_seen: crate::arena::Arr<bool>,
    // §1515
    pub LR_ptr: halfword,
    // §1515
    pub LR_problems: i32,
    // §1515
    pub cur_dir: small_number,
    // §1561
    pub pseudo_files: halfword,
    // §1584
    pub grp_stack: crate::arena::Arr<save_pointer>,
    // §1584
    pub if_stack: crate::arena::Arr<halfword>,
    // §1625
    pub max_reg_num: halfword,
    // §1625
    pub max_reg_help_line: str_number,
    // §1627
    pub sa_root: crate::arena::Arr<halfword>,
    // §1627
    pub cur_ptr: halfword,
    // §1627
    pub sa_null: memory_word,
    // §1646
    pub sa_chain: halfword,
    // §1646
    pub sa_level: quarterword,
    // §1653
    pub last_line_fill: halfword,
    // §1653
    pub do_last_line_fit: bool,
    // §1653
    pub active_node_size: small_number,
    // §1653
    pub fill_width: crate::arena::Arr<scaled>,
    // §1653
    pub best_pl_short: crate::arena::Arr<scaled>,
    // §1653
    pub best_pl_glue: crate::arena::Arr<scaled>,
    // §1669
    pub hyph_start: trie_pointer,
    // §1669
    pub hyph_index: trie_pointer,
    // §1670
    pub disc_ptr: crate::arena::Arr<halfword>,
    // §1683
    pub expand_depth: i32,
    // §1683
    pub expand_depth_count: i32,
    // §1683
    pub shellenabledp: bool,
    // §1683
    pub restrictedshell: bool,
    // §1685
    pub edit_name_start: pool_pointer,
    // §1685
    pub edit_name_length: i32,
    // §1685
    pub edit_line: i32,
    // §1700
    pub mltex_p: bool,
    // §1700
    pub mltex_enabled_p: bool,
    // §1708
    pub synctex_tag_counter: i32,
    // §1714
    pub native_font_type_flag: i32,
    // §1714
    pub xtx_ligature_present: bool,
    // §1715
    pub error_line: i32,
    // §1715
    pub half_error_line: i32,
    // §1715
    pub max_print_line: i32,
    // §1715
    pub file_line_error_style_p: bool,
    // §1715
    pub halt_on_error_p: bool,
    // §1715
    pub halting_on_error_p: bool,
    // §1715
    pub parse_first_line_p: bool,
    // §1715
    pub dump_line: bool,
    // §1715
    pub eight_bit_p: bool,
    // §1715
    pub translate_filename_p: bool,
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
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<usize>()
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
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<list_state_record>()
    + crate::arena::slot::<i32>()
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
    + crate::arena::slot::<UTF16_code>()
    + crate::arena::slot::<[u8; 20]>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<font_index>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<void_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
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
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<two_halves>()
    + crate::arena::slot::<four_quarters>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<i32>()
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
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<scaled>()
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
    + crate::arena::slot::<small_number>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<internal_font_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<i32>()
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
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<halfword>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
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
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<scaled>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
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
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
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
    + crate::arena::slot::<bool>();

impl Globals {
    pub fn new() -> Box<Globals> {
        let mut __plan = crate::arena::Plan::new(SCALAR_BYTES);
        let __r_name_of_file16 = __plan.reserve::<UTF16_code>("name_of_file16", 1025);
        let __r_buffer = __plan.reserve::<UTF16_code>("buffer", 200001);
        let __r_str_pool = __plan.reserve::<packed_UTF16_code>("str_pool", 6250001);
        let __r_str_start = __plan.reserve::<pool_pointer>("str_start", 565537);
        let __r_dig = __plan.reserve::<i32>("dig", 23);
        let __r_trick_buf = __plan.reserve::<UnicodeScalar>("trick_buf", 256);
        let __r_native_text = __plan.reserve::<UTF16_code>("native_text", ((1048576) as usize) + 1);
        let __r_help_line = __plan.reserve::<str_number>("help_line", 6);
        let __r_randoms = __plan.reserve::<i32>("randoms", 55);
        let __r_two_to_the = __plan.reserve::<i32>("two_to_the", 31);
        let __r_spec_log = __plan.reserve::<i32>("spec_log", 28);
        let __r_mem = __plan.reserve::<memory_word>("mem", 5000000);
        let __r_hlist_stack = __plan.reserve::<halfword>("hlist_stack", 513);
        let __r_nest = __plan.reserve::<list_state_record>("nest", 1001);
        let __r_eqtb = __plan.reserve::<memory_word>("eqtb", 9606998);
        let __r_xeq_level = __plan.reserve::<quarterword>("xeq_level", 1114735);
        let __r_hash = __plan.reserve::<two_halves>("hash", 8427349);
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
        let __r_font_bc = __plan.reserve::<UTF16_code>("font_bc", 9001);
        let __r_font_ec = __plan.reserve::<UTF16_code>("font_ec", 9001);
        let __r_font_glue = __plan.reserve::<halfword>("font_glue", 9001);
        let __r_font_used = __plan.reserve::<bool>("font_used", 9001);
        let __r_hyphen_char = __plan.reserve::<i32>("hyphen_char", 9001);
        let __r_skew_char = __plan.reserve::<i32>("skew_char", 9001);
        let __r_bchar_label = __plan.reserve::<font_index>("bchar_label", 9001);
        let __r_font_bchar = __plan.reserve::<i32>("font_bchar", 9001);
        let __r_font_false_bchar = __plan.reserve::<i32>("font_false_bchar", 9001);
        let __r_font_layout_engine = __plan.reserve::<void_pointer>("font_layout_engine", 9001);
        let __r_font_mapping = __plan.reserve::<void_pointer>("font_mapping", 9001);
        let __r_font_flags = __plan.reserve::<i32>("font_flags", 9001);
        let __r_font_letter_space = __plan.reserve::<scaled>("font_letter_space", 9001);
        let __r_mapped_text =
            __plan.reserve::<UTF16_code>("mapped_text", ((mapped_text_size) as usize) + 1);
        let __r_xdv_buffer =
            __plan.reserve::<eight_bits>("xdv_buffer", ((xdv_buffer_size) as usize) + 1);
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
        let __r_total_stretch = __plan.reserve::<scaled>("total_stretch", 4);
        let __r_total_shrink = __plan.reserve::<scaled>("total_shrink", 4);
        let __r_active_width = __plan.reserve::<scaled>("active_width", 6);
        let __r_cur_active_width = __plan.reserve::<scaled>("cur_active_width", 6);
        let __r_background = __plan.reserve::<scaled>("background", 6);
        let __r_break_width = __plan.reserve::<scaled>("break_width", 6);
        let __r_minimal_demerits = __plan.reserve::<i32>("minimal_demerits", 4);
        let __r_best_place = __plan.reserve::<halfword>("best_place", 4);
        let __r_best_pl_line = __plan.reserve::<halfword>("best_pl_line", 4);
        let __r_hc = __plan.reserve::<i32>("hc", 4099);
        let __r_hu = __plan.reserve::<i32>("hu", 4097);
        let __r_hyf = __plan.reserve::<i32>("hyf", 4097);
        let __r_trie = __plan.reserve::<two_halves>("trie", 1100001);
        let __r_hyf_distance = __plan.reserve::<small_number>("hyf_distance", 35111);
        let __r_hyf_num = __plan.reserve::<small_number>("hyf_num", 35111);
        let __r_hyf_next = __plan.reserve::<quarterword>("hyf_next", 35111);
        let __r_op_start = __plan.reserve::<i32>("op_start", 256);
        let __r_hyph_word = __plan.reserve::<str_number>("hyph_word", 8192);
        let __r_hyph_list = __plan.reserve::<halfword>("hyph_list", 8192);
        let __r_trie_op_hash = __plan.reserve::<i32>("trie_op_hash", 70223);
        let __r_trie_used = __plan.reserve::<quarterword>("trie_used", 256);
        let __r_trie_op_lang = __plan.reserve::<i32>("trie_op_lang", 35111);
        let __r_trie_op_val = __plan.reserve::<quarterword>("trie_op_val", 35111);
        let __r_trie_c = __plan.reserve::<packed_UTF16_code>("trie_c", 1100001);
        let __r_trie_o = __plan.reserve::<quarterword>("trie_o", 1100001);
        let __r_trie_l = __plan.reserve::<trie_pointer>("trie_l", 1100001);
        let __r_trie_r = __plan.reserve::<trie_pointer>("trie_r", 1100001);
        let __r_trie_hash = __plan.reserve::<trie_pointer>("trie_hash", 1100001);
        let __r_trie_taken = __plan.reserve::<bool>("trie_taken", 1100000);
        let __r_trie_min = __plan.reserve::<trie_pointer>("trie_min", 65536);
        let __r_page_so_far = __plan.reserve::<scaled>("page_so_far", 8);
        let __r_write_open = __plan.reserve::<bool>("write_open", 18);
        let __r_eof_seen = __plan.reserve::<bool>("eof_seen", 15);
        let __r_grp_stack = __plan.reserve::<save_pointer>("grp_stack", 16);
        let __r_if_stack = __plan.reserve::<halfword>("if_stack", 16);
        let __r_sa_root = __plan.reserve::<halfword>("sa_root", 8);
        let __r_fill_width = __plan.reserve::<scaled>("fill_width", 3);
        let __r_best_pl_short = __plan.reserve::<scaled>("best_pl_short", 4);
        let __r_best_pl_glue = __plan.reserve::<scaled>("best_pl_glue", 4);
        let __r_disc_ptr = __plan.reserve::<halfword>("disc_ptr", 3);
        let __arena = __plan.build();
        Box::new(Globals {
            bad: 0,
            xchr: [0u8; 256],
            name_of_file: [0u8; 1024],
            name_of_file16: __arena.arr(__r_name_of_file16, 1025),
            name_length: 0,
            name_length16: 0,
            buffer: __arena.arr(__r_buffer, 200001),
            first: 0,
            last: 0,
            max_buf_stack: 0,
            term_in: Default::default(),
            term_out: Default::default(),
            str_pool: __arena.arr(__r_str_pool, 6250001),
            str_start: __arena.arr(__r_str_start, 565537),
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
            doing_special: false,
            native_text: __arena.arr(__r_native_text, 0),
            native_text_size: 0,
            native_len: 0,
            save_native_len: 0,
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
            save_arith_error: false,
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
            last_leftmost_char: 0,
            last_rightmost_char: 0,
            hlist_stack: __arena.arr(__r_hlist_stack, 513),
            hlist_stack_level: 0,
            first_p: 0,
            global_prev_p: 0,
            font_in_short_display: 0,
            depth_threshold: 0,
            breadth_max: 0,
            nest: __arena.arr(__r_nest, 1001),
            nest_ptr: 0,
            max_nest_stack: 0,
            cur_list: list_state_record::default(),
            shown_mode: 0,
            old_setting: 0,
            old_selector_ignored_err: 0,
            sys_time: 0,
            sys_day: 0,
            sys_month: 0,
            sys_year: 0,
            eqtb: __arena.arr(__r_eqtb, 9606998),
            xeq_level: __arena.arr(__r_xeq_level, 1114735),
            hash: __arena.arr(__r_hash, 8427349),
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
            input_file: (0..16).map(|_| Default::default()).collect::<Vec<_>>(),
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
            cur_val1: 0,
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
            file_name_quote_char: 0,
            TEX_format_default: [0u8; 20],
            name_in_progress: false,
            job_name: 0,
            log_opened: false,
            output_file_extension: 0,
            no_pdf_output: false,
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
            font_layout_engine: __arena.arr(__r_font_layout_engine, 9001),
            font_mapping: __arena.arr(__r_font_mapping, 9001),
            font_flags: __arena.arr(__r_font_flags, 9001),
            font_letter_space: __arena.arr(__r_font_letter_space, 9001),
            loaded_font_mapping: 0,
            loaded_font_flags: 0,
            loaded_font_letter_space: 0,
            loaded_font_design_size: 0,
            mapped_text: __arena.arr(__r_mapped_text, 0),
            xdv_buffer: __arena.arr(__r_xdv_buffer, 0),
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
            epochseconds: 0,
            microseconds: 0,
            total_stretch: __arena.arr(__r_total_stretch, 4),
            total_shrink: __arena.arr(__r_total_shrink, 4),
            last_badness: 0,
            adjust_tail: 0,
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
            active_width: __arena.arr(__r_active_width, 6),
            cur_active_width: __arena.arr(__r_cur_active_width, 6),
            background: __arena.arr(__r_background, 6),
            break_width: __arena.arr(__r_break_width, 6),
            no_shrink_error_yet: false,
            cur_p: 0,
            second_pass: false,
            final_pass: false,
            threshold: 0,
            minimal_demerits: __arena.arr(__r_minimal_demerits, 4),
            minimum_demerits: 0,
            best_place: __arena.arr(__r_best_place, 4),
            best_pl_line: __arena.arr(__r_best_pl_line, 4),
            disc_width: 0,
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
            hc: __arena.arr(__r_hc, 4099),
            hn: 0,
            ha: 0,
            hb: 0,
            hf: 0,
            hu: __arena.arr(__r_hu, 4097),
            hyf_char: 0,
            cur_lang: 0,
            init_cur_lang: 0,
            l_hyf: 0,
            r_hyf: 0,
            init_l_hyf: 0,
            init_r_hyf: 0,
            hyf_bchar: 0,
            max_hyph_char: 0,
            hyf: __arena.arr(__r_hyf, 4097),
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
            trie_min: __arena.arr(__r_trie_min, 65536),
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
            main_pp: 0,
            main_ppp: 0,
            main_h: 0,
            is_hyph: false,
            space_class: 0,
            prev_class: 0,
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
            cur_page_width: 0,
            cur_page_height: 0,
            cur_h_offset: 0,
            cur_v_offset: 0,
            pdf_last_x_pos: 0,
            pdf_last_y_pos: 0,
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
            sa_root: __arena.arr(__r_sa_root, 8),
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
            edit_name_start: 0,
            edit_name_length: 0,
            edit_line: 0,
            mltex_p: false,
            mltex_enabled_p: false,
            synctex_tag_counter: 0,
            native_font_type_flag: 0,
            xtx_ligature_present: false,
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
        v.pod(&mut self.name_length16);
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
        v.pod(&mut self.doing_special);
        v.arr_len(&mut self.native_text);
        v.pod(&mut self.native_text_size);
        v.pod(&mut self.native_len);
        v.pod(&mut self.save_native_len);
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
        v.pod(&mut self.save_arith_error);
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
        v.pod(&mut self.last_leftmost_char);
        v.pod(&mut self.last_rightmost_char);
        v.pod(&mut self.hlist_stack_level);
        v.pod(&mut self.first_p);
        v.pod(&mut self.global_prev_p);
        v.pod(&mut self.font_in_short_display);
        v.pod(&mut self.depth_threshold);
        v.pod(&mut self.breadth_max);
        v.pod(&mut self.nest_ptr);
        v.pod(&mut self.max_nest_stack);
        v.pod(&mut self.cur_list);
        v.pod(&mut self.shown_mode);
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
        v.pod(&mut self.cur_val1);
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
        v.pod(&mut self.file_name_quote_char);
        v.pod(&mut self.TEX_format_default);
        v.pod(&mut self.name_in_progress);
        v.pod(&mut self.job_name);
        v.pod(&mut self.log_opened);
        v.pod(&mut self.output_file_extension);
        v.pod(&mut self.no_pdf_output);
        v.pod(&mut self.output_file_name);
        v.pod(&mut self.log_name);
        v.pod(&mut self.fmem_ptr);
        v.pod(&mut self.font_ptr);
        v.pod(&mut self.loaded_font_mapping);
        v.pod(&mut self.loaded_font_flags);
        v.pod(&mut self.loaded_font_letter_space);
        v.pod(&mut self.loaded_font_design_size);
        v.arr_len(&mut self.mapped_text);
        v.arr_len(&mut self.xdv_buffer);
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
        v.pod(&mut self.epochseconds);
        v.pod(&mut self.microseconds);
        v.pod(&mut self.last_badness);
        v.pod(&mut self.adjust_tail);
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
        v.pod(&mut self.no_shrink_error_yet);
        v.pod(&mut self.cur_p);
        v.pod(&mut self.second_pass);
        v.pod(&mut self.final_pass);
        v.pod(&mut self.threshold);
        v.pod(&mut self.minimum_demerits);
        v.pod(&mut self.disc_width);
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
        v.pod(&mut self.max_hyph_char);
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
        v.pod(&mut self.main_pp);
        v.pod(&mut self.main_ppp);
        v.pod(&mut self.main_h);
        v.pod(&mut self.is_hyph);
        v.pod(&mut self.space_class);
        v.pod(&mut self.prev_class);
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
        v.pod(&mut self.cur_page_width);
        v.pod(&mut self.cur_page_height);
        v.pod(&mut self.cur_h_offset);
        v.pod(&mut self.cur_v_offset);
        v.pod(&mut self.pdf_last_x_pos);
        v.pod(&mut self.pdf_last_y_pos);
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
        v.pod(&mut self.edit_name_start);
        v.pod(&mut self.edit_name_length);
        v.pod(&mut self.edit_line);
        v.pod(&mut self.mltex_p);
        v.pod(&mut self.mltex_enabled_p);
        v.pod(&mut self.synctex_tag_counter);
        v.pod(&mut self.native_font_type_flag);
        v.pod(&mut self.xtx_ligature_present);
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
        v.word(&mut self.fmt_file);
        for f in self.write_file.iter_mut() {
            v.alpha(f);
        }
    }
}
