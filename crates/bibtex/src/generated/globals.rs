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
    // §2
    pub standard_input: crate::system::AlphaFile,
    // §2
    pub standard_output: crate::system::AlphaFile,
    // §2
    pub lab31: bool,
    // §2
    pub aux_arg: i32,
    // §12
    pub pool_size: i32,
    // §12
    pub max_print_line: i32,
    // §12
    pub max_bib_files: i32,
    // §12
    pub max_cites: i32,
    // §12
    pub wiz_fn_space: i32,
    // §12
    pub ent_str_size: i32,
    // §12
    pub glob_str_size: i32,
    // §12
    pub max_glob_strs: i32,
    // §12
    pub max_fields: i32,
    // §12
    pub lit_stk_size: i32,
    // §12
    pub max_strings: i32,
    // §12
    pub hash_size: i32,
    // §12
    pub hash_prime: i32,
    // §12
    pub hash_max: i32,
    // §12
    pub end_of_def: i32,
    // §12
    pub undefined: i32,
    // §12
    pub bad: i32,
    // §15
    pub history: i32,
    // §15
    pub err_count: i32,
    // §19
    pub xord: crate::arena::Arr<ASCII_code>,
    // §19
    pub xchr: [u8; 256],
    // §25
    pub lex_class: crate::arena::Arr<lex_type>,
    // §25
    pub id_class: crate::arena::Arr<id_type>,
    // §29
    pub char_width: crate::arena::Arr<i32>,
    // §29
    pub string_width: i32,
    // §32
    pub name_of_file: crate::arena::Arr<u8>,
    // §32
    pub name_length: i32,
    // §32
    pub name_ptr: i32,
    // §34
    pub buf_size: i32,
    // §34
    pub buffer: crate::arena::Arr<ASCII_code>,
    // §34
    pub last: buf_pointer,
    // §36
    pub sv_buffer: crate::arena::Arr<ASCII_code>,
    // §36
    pub sv_ptr1: buf_pointer,
    // §36
    pub sv_ptr2: buf_pointer,
    // §36
    pub tmp_ptr: i32,
    // §36
    pub tmp_end_ptr: i32,
    // §41
    pub str_pool: crate::arena::Arr<ASCII_code>,
    // §41
    pub str_start: crate::arena::Arr<pool_pointer>,
    // §41
    pub pool_ptr: pool_pointer,
    // §41
    pub str_ptr: str_number,
    // §41
    pub str_num: str_number,
    // §41
    pub p_ptr1: pool_pointer,
    // §41
    pub p_ptr2: pool_pointer,
    // §58
    pub hash_next: crate::arena::Arr<hash_pointer>,
    // §58
    pub hash_text: crate::arena::Arr<str_number>,
    // §58
    pub hash_ilk: crate::arena::Arr<str_ilk>,
    // §58
    pub ilk_info: crate::arena::Arr<i32>,
    // §58
    pub hash_used: i32,
    // §58
    pub hash_found: bool,
    // §58
    pub dummy_loc: hash_loc,
    // §67
    pub s_aux_extension: str_number,
    // §67
    pub s_log_extension: str_number,
    // §67
    pub s_bbl_extension: str_number,
    // §67
    pub s_bst_extension: str_number,
    // §67
    pub s_bib_extension: str_number,
    // §67
    pub s_bst_area: str_number,
    // §67
    pub s_bib_area: str_number,
    // §69
    pub pre_def_loc: hash_loc,
    // §71
    pub command_num: i32,
    // §73
    pub buf_ptr1: buf_pointer,
    // §73
    pub buf_ptr2: buf_pointer,
    // §82
    pub scan_result: i32,
    // §84
    pub token_value: i32,
    // §90
    pub aux_name_length: i32,
    // §96
    pub aux_file: Vec<crate::system::AlphaFile>,
    // §96
    pub aux_list: crate::arena::Arr<str_number>,
    // §96
    pub aux_ptr: aux_number,
    // §96
    pub aux_ln_stack: crate::arena::Arr<i32>,
    // §96
    pub top_lev_str: str_number,
    // §96
    pub log_file: crate::system::AlphaFile,
    // §96
    pub bbl_file: crate::system::AlphaFile,
    // §109
    pub bib_list: crate::arena::Arr<str_number>,
    // §109
    pub bib_ptr: bib_number,
    // §109
    pub num_bib_files: bib_number,
    // §109
    pub bib_seen: bool,
    // §109
    pub bib_file: Vec<crate::system::AlphaFile>,
    // §116
    pub bst_seen: bool,
    // §116
    pub bst_str: str_number,
    // §116
    pub bst_file: crate::system::AlphaFile,
    // §121
    pub cite_list: crate::arena::Arr<str_number>,
    // §121
    pub cite_ptr: cite_number,
    // §121
    pub entry_cite_ptr: cite_number,
    // §121
    pub num_cites: cite_number,
    // §121
    pub old_num_cites: cite_number,
    // §121
    pub citation_seen: bool,
    // §121
    pub cite_loc: hash_loc,
    // §121
    pub lc_cite_loc: hash_loc,
    // §121
    pub lc_xcite_loc: hash_loc,
    // §121
    pub cite_found: bool,
    // §121
    pub all_entries: bool,
    // §121
    pub all_marker: cite_number,
    // §139
    pub bbl_line_num: i32,
    // §139
    pub bst_line_num: i32,
    // §153
    pub fn_loc: hash_loc,
    // §153
    pub wiz_loc: hash_loc,
    // §153
    pub literal_loc: hash_loc,
    // §153
    pub macro_name_loc: hash_loc,
    // §153
    pub macro_def_loc: hash_loc,
    // §153
    pub fn_type: crate::arena::Arr<fn_class>,
    // §153
    pub wiz_def_ptr: wiz_fn_loc,
    // §153
    pub wiz_fn_ptr: wiz_fn_loc,
    // §153
    pub wiz_functions: crate::arena::Arr<hash_ptr2>,
    // §153
    pub int_ent_ptr: int_ent_loc,
    // §153
    pub entry_ints: crate::arena::Arr<i32>,
    // §153
    pub num_ent_ints: int_ent_loc,
    // §153
    pub str_ent_ptr: str_ent_loc,
    // §153
    pub entry_strs: crate::arena::Arr<ASCII_code>,
    // §153
    pub num_ent_strs: str_ent_loc,
    // §153
    pub str_glb_ptr: i32,
    // §153
    pub glb_str_ptr: crate::arena::Arr<str_number>,
    // §153
    pub global_strs: crate::arena::Arr<ASCII_code>,
    // §153
    pub glb_str_end: crate::arena::Arr<i32>,
    // §153
    pub num_glb_strs: i32,
    // §153
    pub field_ptr: field_loc,
    // §153
    pub field_parent_ptr: field_loc,
    // §153
    pub field_end_ptr: field_loc,
    // §153
    pub cite_parent_ptr: cite_number,
    // §153
    pub cite_xptr: cite_number,
    // §153
    pub field_info: crate::arena::Arr<str_number>,
    // §153
    pub num_fields: field_loc,
    // §153
    pub num_pre_defined_fields: field_loc,
    // §153
    pub crossref_num: field_loc,
    // §153
    pub no_fields: bool,
    // §155
    pub entry_seen: bool,
    // §155
    pub read_seen: bool,
    // §155
    pub read_performed: bool,
    // §155
    pub reading_completed: bool,
    // §155
    pub read_completed: bool,
    // §187
    pub impl_fn_num: i32,
    // §210
    pub bib_line_num: i32,
    // §210
    pub entry_type_loc: hash_loc,
    // §210
    pub type_list: crate::arena::Arr<hash_ptr2>,
    // §210
    pub type_exists: bool,
    // §210
    pub entry_exists: crate::arena::Arr<bool>,
    // §210
    pub store_entry: bool,
    // §210
    pub field_name_loc: hash_loc,
    // §210
    pub field_val_loc: hash_loc,
    // §210
    pub store_field: bool,
    // §210
    pub store_token: bool,
    // §210
    pub right_outer_delim: ASCII_code,
    // §210
    pub right_str_delim: ASCII_code,
    // §210
    pub at_bib_command: bool,
    // §210
    pub cur_macro_loc: hash_loc,
    // §210
    pub cite_info: crate::arena::Arr<str_number>,
    // §210
    pub cite_hash_found: bool,
    // §210
    pub preamble_ptr: bib_number,
    // §210
    pub num_preamble_strings: bib_number,
    // §238
    pub bib_brace_level: i32,
    // §281
    pub lit_stack: crate::arena::Arr<i32>,
    // §281
    pub lit_stk_type: crate::arena::Arr<stk_type>,
    // §281
    pub lit_stk_ptr: lit_stk_loc,
    // §281
    pub cmd_str_ptr: str_number,
    // §281
    pub ent_chr_ptr: i32,
    // §281
    pub glob_chr_ptr: i32,
    // §281
    pub ex_buf: crate::arena::Arr<ASCII_code>,
    // §281
    pub ex_buf_ptr: buf_pointer,
    // §281
    pub ex_buf_length: buf_pointer,
    // §281
    pub out_buf: crate::arena::Arr<ASCII_code>,
    // §281
    pub out_buf_ptr: buf_pointer,
    // §281
    pub out_buf_length: buf_pointer,
    // §281
    pub mess_with_entries: bool,
    // §281
    pub sort_cite_ptr: cite_number,
    // §281
    pub sort_key_num: str_ent_loc,
    // §281
    pub brace_level: i32,
    // §322
    pub b_equals: hash_loc,
    // §322
    pub b_greater_than: hash_loc,
    // §322
    pub b_less_than: hash_loc,
    // §322
    pub b_plus: hash_loc,
    // §322
    pub b_minus: hash_loc,
    // §322
    pub b_concatenate: hash_loc,
    // §322
    pub b_gets: hash_loc,
    // §322
    pub b_add_period: hash_loc,
    // §322
    pub b_call_type: hash_loc,
    // §322
    pub b_change_case: hash_loc,
    // §322
    pub b_chr_to_int: hash_loc,
    // §322
    pub b_cite: hash_loc,
    // §322
    pub b_duplicate: hash_loc,
    // §322
    pub b_empty: hash_loc,
    // §322
    pub b_format_name: hash_loc,
    // §322
    pub b_if: hash_loc,
    // §322
    pub b_int_to_chr: hash_loc,
    // §322
    pub b_int_to_str: hash_loc,
    // §322
    pub b_missing: hash_loc,
    // §322
    pub b_newline: hash_loc,
    // §322
    pub b_num_names: hash_loc,
    // §322
    pub b_pop: hash_loc,
    // §322
    pub b_preamble: hash_loc,
    // §322
    pub b_purify: hash_loc,
    // §322
    pub b_quote: hash_loc,
    // §322
    pub b_skip: hash_loc,
    // §322
    pub b_stack: hash_loc,
    // §322
    pub b_substring: hash_loc,
    // §322
    pub b_swap: hash_loc,
    // §322
    pub b_text_length: hash_loc,
    // §322
    pub b_text_prefix: hash_loc,
    // §322
    pub b_top_stack: hash_loc,
    // §322
    pub b_type: hash_loc,
    // §322
    pub b_warning: hash_loc,
    // §322
    pub b_while: hash_loc,
    // §322
    pub b_width: hash_loc,
    // §322
    pub b_write: hash_loc,
    // §322
    pub b_default: hash_loc,
    // §322
    pub blt_in_loc: crate::arena::Arr<hash_loc>,
    // §322
    pub execution_count: crate::arena::Arr<i32>,
    // §322
    pub total_ex_count: i32,
    // §322
    pub blt_in_ptr: blt_in_range,
    // §328
    pub s_null: str_number,
    // §328
    pub s_default: str_number,
    // §328
    pub s_t: str_number,
    // §328
    pub s_l: str_number,
    // §328
    pub s_u: str_number,
    // §328
    pub s_preamble: crate::arena::Arr<str_number>,
    // §335
    pub pop_lit1: i32,
    // §335
    pub pop_lit2: i32,
    // §335
    pub pop_lit3: i32,
    // §335
    pub pop_typ1: stk_type,
    // §335
    pub pop_typ2: stk_type,
    // §335
    pub pop_typ3: stk_type,
    // §335
    pub sp_ptr: pool_pointer,
    // §335
    pub sp_xptr1: pool_pointer,
    // §335
    pub sp_xptr2: pool_pointer,
    // §335
    pub sp_end: pool_pointer,
    // §335
    pub sp_length: pool_pointer,
    // §335
    pub sp2_length: pool_pointer,
    // §335
    pub sp_brace_level: i32,
    // §335
    pub ex_buf_xptr: buf_pointer,
    // §335
    pub ex_buf_yptr: buf_pointer,
    // §335
    pub control_seq_loc: hash_loc,
    // §335
    pub preceding_white: bool,
    // §335
    pub and_found: bool,
    // §335
    pub num_names: i32,
    // §335
    pub name_bf_ptr: buf_pointer,
    // §335
    pub name_bf_xptr: buf_pointer,
    // §335
    pub name_bf_yptr: buf_pointer,
    // §335
    pub nm_brace_level: i32,
    // §335
    pub name_tok: crate::arena::Arr<buf_pointer>,
    // §335
    pub name_sep_char: crate::arena::Arr<ASCII_code>,
    // §335
    pub num_tokens: buf_pointer,
    // §335
    pub token_starting: bool,
    // §335
    pub alpha_found: bool,
    // §335
    pub double_letter: bool,
    // §335
    pub end_of_group: bool,
    // §335
    pub to_be_written: bool,
    // §335
    pub first_start: buf_pointer,
    // §335
    pub first_end: buf_pointer,
    // §335
    pub last_end: buf_pointer,
    // §335
    pub von_start: buf_pointer,
    // §335
    pub von_end: buf_pointer,
    // §335
    pub jr_end: buf_pointer,
    // §335
    pub cur_token: buf_pointer,
    // §335
    pub last_token: buf_pointer,
    // §335
    pub use_default: bool,
    // §335
    pub num_commas: buf_pointer,
    // §335
    pub comma1: buf_pointer,
    // §335
    pub comma2: buf_pointer,
    // §335
    pub num_text_chars: buf_pointer,
    // §356
    pub conversion_type: i32,
    // §356
    pub prev_colon: bool,
    // §460
    pub verbose: bool,
    // §463
    pub min_crossrefs: i32,
    /// The word space every `Arr` above lives in (crates/flashtex-engine/src/arena.rs).
    pub arena: crate::arena::Arena,
    /// The engine's state outside the word space (`--host-state`).
    pub host: crate::system::State,
}

/// Bytes of the scalar globals' region at the start of the word space.
pub const SCALAR_BYTES: usize = 0
    + crate::arena::slot::<bool>()
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
    + crate::arena::slot::<[u8; 256]>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<aux_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<bib_number>()
    + crate::arena::slot::<bib_number>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<wiz_fn_loc>()
    + crate::arena::slot::<wiz_fn_loc>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<int_ent_loc>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<int_ent_loc>()
    + crate::arena::slot::<str_ent_loc>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<str_ent_loc>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<field_loc>()
    + crate::arena::slot::<field_loc>()
    + crate::arena::slot::<field_loc>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<field_loc>()
    + crate::arena::slot::<field_loc>()
    + crate::arena::slot::<field_loc>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<ASCII_code>()
    + crate::arena::slot::<ASCII_code>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bib_number>()
    + crate::arena::slot::<bib_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<lit_stk_loc>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<cite_number>()
    + crate::arena::slot::<str_ent_loc>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<blt_in_range>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<str_number>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<stk_type>()
    + crate::arena::slot::<stk_type>()
    + crate::arena::slot::<stk_type>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<pool_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<hash_loc>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<usize>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<buf_pointer>()
    + crate::arena::slot::<i32>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<bool>()
    + crate::arena::slot::<i32>();

impl Globals {
    pub fn new() -> Box<Globals> {
        let mut __plan = crate::arena::Plan::new(SCALAR_BYTES);
        let __r_xord = __plan.reserve::<ASCII_code>("xord", 256);
        let __r_lex_class = __plan.reserve::<lex_type>("lex_class", 256);
        let __r_id_class = __plan.reserve::<id_type>("id_class", 256);
        let __r_char_width = __plan.reserve::<i32>("char_width", 256);
        let __r_name_of_file = __plan.reserve::<u8>("name_of_file", ((0) as usize) + 1);
        let __r_buffer = __plan.reserve::<ASCII_code>("buffer", ((0) as usize) + 1);
        let __r_sv_buffer = __plan.reserve::<ASCII_code>("sv_buffer", ((0) as usize) + 1);
        let __r_str_pool = __plan.reserve::<ASCII_code>("str_pool", ((0) as usize) + 1);
        let __r_str_start = __plan.reserve::<pool_pointer>("str_start", ((0) as usize) + 1);
        let __r_hash_next = __plan.reserve::<hash_pointer>("hash_next", ((0) as usize) + 1);
        let __r_hash_text = __plan.reserve::<str_number>("hash_text", ((0) as usize) + 1);
        let __r_hash_ilk = __plan.reserve::<str_ilk>("hash_ilk", ((0) as usize) + 1);
        let __r_ilk_info = __plan.reserve::<i32>("ilk_info", ((0) as usize) + 1);
        let __r_aux_list = __plan.reserve::<str_number>("aux_list", 21);
        let __r_aux_ln_stack = __plan.reserve::<i32>("aux_ln_stack", 21);
        let __r_bib_list = __plan.reserve::<str_number>("bib_list", ((0) as usize) + 1);
        let __r_cite_list = __plan.reserve::<str_number>("cite_list", ((0) as usize) + 1);
        let __r_fn_type = __plan.reserve::<fn_class>("fn_type", ((0) as usize) + 1);
        let __r_wiz_functions = __plan.reserve::<hash_ptr2>("wiz_functions", ((0) as usize) + 1);
        let __r_entry_ints = __plan.reserve::<i32>("entry_ints", ((0) as usize) + 1);
        let __r_entry_strs = __plan.reserve::<ASCII_code>("entry_strs", ((0) as usize) + 1);
        let __r_glb_str_ptr = __plan.reserve::<str_number>("glb_str_ptr", ((0) as usize) + 1);
        let __r_global_strs = __plan.reserve::<ASCII_code>("global_strs", ((0) as usize) + 1);
        let __r_glb_str_end = __plan.reserve::<i32>("glb_str_end", ((0) as usize) + 1);
        let __r_field_info = __plan.reserve::<str_number>("field_info", ((0) as usize) + 1);
        let __r_type_list = __plan.reserve::<hash_ptr2>("type_list", ((0) as usize) + 1);
        let __r_entry_exists = __plan.reserve::<bool>("entry_exists", ((0) as usize) + 1);
        let __r_cite_info = __plan.reserve::<str_number>("cite_info", ((0) as usize) + 1);
        let __r_lit_stack = __plan.reserve::<i32>("lit_stack", ((0) as usize) + 1);
        let __r_lit_stk_type = __plan.reserve::<stk_type>("lit_stk_type", ((0) as usize) + 1);
        let __r_ex_buf = __plan.reserve::<ASCII_code>("ex_buf", ((0) as usize) + 1);
        let __r_out_buf = __plan.reserve::<ASCII_code>("out_buf", ((0) as usize) + 1);
        let __r_blt_in_loc = __plan.reserve::<hash_loc>("blt_in_loc", 38);
        let __r_execution_count = __plan.reserve::<i32>("execution_count", 38);
        let __r_s_preamble = __plan.reserve::<str_number>("s_preamble", ((0) as usize) + 1);
        let __r_name_tok = __plan.reserve::<buf_pointer>("name_tok", ((0) as usize) + 1);
        let __r_name_sep_char = __plan.reserve::<ASCII_code>("name_sep_char", ((0) as usize) + 1);
        let __arena = __plan.build();
        Box::new(Globals {
            standard_input: Default::default(),
            standard_output: Default::default(),
            lab31: false,
            aux_arg: 0,
            pool_size: 0,
            max_print_line: 0,
            max_bib_files: 0,
            max_cites: 0,
            wiz_fn_space: 0,
            ent_str_size: 0,
            glob_str_size: 0,
            max_glob_strs: 0,
            max_fields: 0,
            lit_stk_size: 0,
            max_strings: 0,
            hash_size: 0,
            hash_prime: 0,
            hash_max: 0,
            end_of_def: 0,
            undefined: 0,
            bad: 0,
            history: 0,
            err_count: 0,
            xord: __arena.arr(__r_xord, 256),
            xchr: [0u8; 256],
            lex_class: __arena.arr(__r_lex_class, 256),
            id_class: __arena.arr(__r_id_class, 256),
            char_width: __arena.arr(__r_char_width, 256),
            string_width: 0,
            name_of_file: __arena.arr(__r_name_of_file, 0),
            name_length: 0,
            name_ptr: 0,
            buf_size: 0,
            buffer: __arena.arr(__r_buffer, 0),
            last: 0,
            sv_buffer: __arena.arr(__r_sv_buffer, 0),
            sv_ptr1: 0,
            sv_ptr2: 0,
            tmp_ptr: 0,
            tmp_end_ptr: 0,
            str_pool: __arena.arr(__r_str_pool, 0),
            str_start: __arena.arr(__r_str_start, 0),
            pool_ptr: 0,
            str_ptr: 0,
            str_num: 0,
            p_ptr1: 0,
            p_ptr2: 0,
            hash_next: __arena.arr(__r_hash_next, 0),
            hash_text: __arena.arr(__r_hash_text, 0),
            hash_ilk: __arena.arr(__r_hash_ilk, 0),
            ilk_info: __arena.arr(__r_ilk_info, 0),
            hash_used: 0,
            hash_found: false,
            dummy_loc: 0,
            s_aux_extension: 0,
            s_log_extension: 0,
            s_bbl_extension: 0,
            s_bst_extension: 0,
            s_bib_extension: 0,
            s_bst_area: 0,
            s_bib_area: 0,
            pre_def_loc: 0,
            command_num: 0,
            buf_ptr1: 0,
            buf_ptr2: 0,
            scan_result: 0,
            token_value: 0,
            aux_name_length: 0,
            aux_file: (0..21).map(|_| Default::default()).collect::<Vec<_>>(),
            aux_list: __arena.arr(__r_aux_list, 21),
            aux_ptr: 0,
            aux_ln_stack: __arena.arr(__r_aux_ln_stack, 21),
            top_lev_str: 0,
            log_file: Default::default(),
            bbl_file: Default::default(),
            bib_list: __arena.arr(__r_bib_list, 0),
            bib_ptr: 0,
            num_bib_files: 0,
            bib_seen: false,
            bib_file: Vec::new(),
            bst_seen: false,
            bst_str: 0,
            bst_file: Default::default(),
            cite_list: __arena.arr(__r_cite_list, 0),
            cite_ptr: 0,
            entry_cite_ptr: 0,
            num_cites: 0,
            old_num_cites: 0,
            citation_seen: false,
            cite_loc: 0,
            lc_cite_loc: 0,
            lc_xcite_loc: 0,
            cite_found: false,
            all_entries: false,
            all_marker: 0,
            bbl_line_num: 0,
            bst_line_num: 0,
            fn_loc: 0,
            wiz_loc: 0,
            literal_loc: 0,
            macro_name_loc: 0,
            macro_def_loc: 0,
            fn_type: __arena.arr(__r_fn_type, 0),
            wiz_def_ptr: 0,
            wiz_fn_ptr: 0,
            wiz_functions: __arena.arr(__r_wiz_functions, 0),
            int_ent_ptr: 0,
            entry_ints: __arena.arr(__r_entry_ints, 0),
            num_ent_ints: 0,
            str_ent_ptr: 0,
            entry_strs: __arena.arr(__r_entry_strs, 0),
            num_ent_strs: 0,
            str_glb_ptr: 0,
            glb_str_ptr: __arena.arr(__r_glb_str_ptr, 0),
            global_strs: __arena.arr(__r_global_strs, 0),
            glb_str_end: __arena.arr(__r_glb_str_end, 0),
            num_glb_strs: 0,
            field_ptr: 0,
            field_parent_ptr: 0,
            field_end_ptr: 0,
            cite_parent_ptr: 0,
            cite_xptr: 0,
            field_info: __arena.arr(__r_field_info, 0),
            num_fields: 0,
            num_pre_defined_fields: 0,
            crossref_num: 0,
            no_fields: false,
            entry_seen: false,
            read_seen: false,
            read_performed: false,
            reading_completed: false,
            read_completed: false,
            impl_fn_num: 0,
            bib_line_num: 0,
            entry_type_loc: 0,
            type_list: __arena.arr(__r_type_list, 0),
            type_exists: false,
            entry_exists: __arena.arr(__r_entry_exists, 0),
            store_entry: false,
            field_name_loc: 0,
            field_val_loc: 0,
            store_field: false,
            store_token: false,
            right_outer_delim: 0,
            right_str_delim: 0,
            at_bib_command: false,
            cur_macro_loc: 0,
            cite_info: __arena.arr(__r_cite_info, 0),
            cite_hash_found: false,
            preamble_ptr: 0,
            num_preamble_strings: 0,
            bib_brace_level: 0,
            lit_stack: __arena.arr(__r_lit_stack, 0),
            lit_stk_type: __arena.arr(__r_lit_stk_type, 0),
            lit_stk_ptr: 0,
            cmd_str_ptr: 0,
            ent_chr_ptr: 0,
            glob_chr_ptr: 0,
            ex_buf: __arena.arr(__r_ex_buf, 0),
            ex_buf_ptr: 0,
            ex_buf_length: 0,
            out_buf: __arena.arr(__r_out_buf, 0),
            out_buf_ptr: 0,
            out_buf_length: 0,
            mess_with_entries: false,
            sort_cite_ptr: 0,
            sort_key_num: 0,
            brace_level: 0,
            b_equals: 0,
            b_greater_than: 0,
            b_less_than: 0,
            b_plus: 0,
            b_minus: 0,
            b_concatenate: 0,
            b_gets: 0,
            b_add_period: 0,
            b_call_type: 0,
            b_change_case: 0,
            b_chr_to_int: 0,
            b_cite: 0,
            b_duplicate: 0,
            b_empty: 0,
            b_format_name: 0,
            b_if: 0,
            b_int_to_chr: 0,
            b_int_to_str: 0,
            b_missing: 0,
            b_newline: 0,
            b_num_names: 0,
            b_pop: 0,
            b_preamble: 0,
            b_purify: 0,
            b_quote: 0,
            b_skip: 0,
            b_stack: 0,
            b_substring: 0,
            b_swap: 0,
            b_text_length: 0,
            b_text_prefix: 0,
            b_top_stack: 0,
            b_type: 0,
            b_warning: 0,
            b_while: 0,
            b_width: 0,
            b_write: 0,
            b_default: 0,
            blt_in_loc: __arena.arr(__r_blt_in_loc, 38),
            execution_count: __arena.arr(__r_execution_count, 38),
            total_ex_count: 0,
            blt_in_ptr: 0,
            s_null: 0,
            s_default: 0,
            s_t: 0,
            s_l: 0,
            s_u: 0,
            s_preamble: __arena.arr(__r_s_preamble, 0),
            pop_lit1: 0,
            pop_lit2: 0,
            pop_lit3: 0,
            pop_typ1: 0,
            pop_typ2: 0,
            pop_typ3: 0,
            sp_ptr: 0,
            sp_xptr1: 0,
            sp_xptr2: 0,
            sp_end: 0,
            sp_length: 0,
            sp2_length: 0,
            sp_brace_level: 0,
            ex_buf_xptr: 0,
            ex_buf_yptr: 0,
            control_seq_loc: 0,
            preceding_white: false,
            and_found: false,
            num_names: 0,
            name_bf_ptr: 0,
            name_bf_xptr: 0,
            name_bf_yptr: 0,
            nm_brace_level: 0,
            name_tok: __arena.arr(__r_name_tok, 0),
            name_sep_char: __arena.arr(__r_name_sep_char, 0),
            num_tokens: 0,
            token_starting: false,
            alpha_found: false,
            double_letter: false,
            end_of_group: false,
            to_be_written: false,
            first_start: 0,
            first_end: 0,
            last_end: 0,
            von_start: 0,
            von_end: 0,
            jr_end: 0,
            cur_token: 0,
            last_token: 0,
            use_default: false,
            num_commas: 0,
            comma1: 0,
            comma2: 0,
            num_text_chars: 0,
            conversion_type: 0,
            prev_colon: false,
            verbose: false,
            min_crossrefs: 0,
            arena: __arena,
            host: Default::default(),
        })
    }

    /// Every scalar global, then every growable array's length, in the order
    /// of `SCALAR_BYTES` (the checkpoint spill and fill).
    pub fn visit_scalars<V: crate::arena::Visit>(&mut self, v: &mut V) {
        v.pod(&mut self.lab31);
        v.pod(&mut self.aux_arg);
        v.pod(&mut self.pool_size);
        v.pod(&mut self.max_print_line);
        v.pod(&mut self.max_bib_files);
        v.pod(&mut self.max_cites);
        v.pod(&mut self.wiz_fn_space);
        v.pod(&mut self.ent_str_size);
        v.pod(&mut self.glob_str_size);
        v.pod(&mut self.max_glob_strs);
        v.pod(&mut self.max_fields);
        v.pod(&mut self.lit_stk_size);
        v.pod(&mut self.max_strings);
        v.pod(&mut self.hash_size);
        v.pod(&mut self.hash_prime);
        v.pod(&mut self.hash_max);
        v.pod(&mut self.end_of_def);
        v.pod(&mut self.undefined);
        v.pod(&mut self.bad);
        v.pod(&mut self.history);
        v.pod(&mut self.err_count);
        v.pod(&mut self.xchr);
        v.pod(&mut self.string_width);
        v.arr_len(&mut self.name_of_file);
        v.pod(&mut self.name_length);
        v.pod(&mut self.name_ptr);
        v.pod(&mut self.buf_size);
        v.arr_len(&mut self.buffer);
        v.pod(&mut self.last);
        v.arr_len(&mut self.sv_buffer);
        v.pod(&mut self.sv_ptr1);
        v.pod(&mut self.sv_ptr2);
        v.pod(&mut self.tmp_ptr);
        v.pod(&mut self.tmp_end_ptr);
        v.arr_len(&mut self.str_pool);
        v.arr_len(&mut self.str_start);
        v.pod(&mut self.pool_ptr);
        v.pod(&mut self.str_ptr);
        v.pod(&mut self.str_num);
        v.pod(&mut self.p_ptr1);
        v.pod(&mut self.p_ptr2);
        v.arr_len(&mut self.hash_next);
        v.arr_len(&mut self.hash_text);
        v.arr_len(&mut self.hash_ilk);
        v.arr_len(&mut self.ilk_info);
        v.pod(&mut self.hash_used);
        v.pod(&mut self.hash_found);
        v.pod(&mut self.dummy_loc);
        v.pod(&mut self.s_aux_extension);
        v.pod(&mut self.s_log_extension);
        v.pod(&mut self.s_bbl_extension);
        v.pod(&mut self.s_bst_extension);
        v.pod(&mut self.s_bib_extension);
        v.pod(&mut self.s_bst_area);
        v.pod(&mut self.s_bib_area);
        v.pod(&mut self.pre_def_loc);
        v.pod(&mut self.command_num);
        v.pod(&mut self.buf_ptr1);
        v.pod(&mut self.buf_ptr2);
        v.pod(&mut self.scan_result);
        v.pod(&mut self.token_value);
        v.pod(&mut self.aux_name_length);
        v.pod(&mut self.aux_ptr);
        v.pod(&mut self.top_lev_str);
        v.arr_len(&mut self.bib_list);
        v.pod(&mut self.bib_ptr);
        v.pod(&mut self.num_bib_files);
        v.pod(&mut self.bib_seen);
        v.pod(&mut self.bst_seen);
        v.pod(&mut self.bst_str);
        v.arr_len(&mut self.cite_list);
        v.pod(&mut self.cite_ptr);
        v.pod(&mut self.entry_cite_ptr);
        v.pod(&mut self.num_cites);
        v.pod(&mut self.old_num_cites);
        v.pod(&mut self.citation_seen);
        v.pod(&mut self.cite_loc);
        v.pod(&mut self.lc_cite_loc);
        v.pod(&mut self.lc_xcite_loc);
        v.pod(&mut self.cite_found);
        v.pod(&mut self.all_entries);
        v.pod(&mut self.all_marker);
        v.pod(&mut self.bbl_line_num);
        v.pod(&mut self.bst_line_num);
        v.pod(&mut self.fn_loc);
        v.pod(&mut self.wiz_loc);
        v.pod(&mut self.literal_loc);
        v.pod(&mut self.macro_name_loc);
        v.pod(&mut self.macro_def_loc);
        v.arr_len(&mut self.fn_type);
        v.pod(&mut self.wiz_def_ptr);
        v.pod(&mut self.wiz_fn_ptr);
        v.arr_len(&mut self.wiz_functions);
        v.pod(&mut self.int_ent_ptr);
        v.arr_len(&mut self.entry_ints);
        v.pod(&mut self.num_ent_ints);
        v.pod(&mut self.str_ent_ptr);
        v.arr_len(&mut self.entry_strs);
        v.pod(&mut self.num_ent_strs);
        v.pod(&mut self.str_glb_ptr);
        v.arr_len(&mut self.glb_str_ptr);
        v.arr_len(&mut self.global_strs);
        v.arr_len(&mut self.glb_str_end);
        v.pod(&mut self.num_glb_strs);
        v.pod(&mut self.field_ptr);
        v.pod(&mut self.field_parent_ptr);
        v.pod(&mut self.field_end_ptr);
        v.pod(&mut self.cite_parent_ptr);
        v.pod(&mut self.cite_xptr);
        v.arr_len(&mut self.field_info);
        v.pod(&mut self.num_fields);
        v.pod(&mut self.num_pre_defined_fields);
        v.pod(&mut self.crossref_num);
        v.pod(&mut self.no_fields);
        v.pod(&mut self.entry_seen);
        v.pod(&mut self.read_seen);
        v.pod(&mut self.read_performed);
        v.pod(&mut self.reading_completed);
        v.pod(&mut self.read_completed);
        v.pod(&mut self.impl_fn_num);
        v.pod(&mut self.bib_line_num);
        v.pod(&mut self.entry_type_loc);
        v.arr_len(&mut self.type_list);
        v.pod(&mut self.type_exists);
        v.arr_len(&mut self.entry_exists);
        v.pod(&mut self.store_entry);
        v.pod(&mut self.field_name_loc);
        v.pod(&mut self.field_val_loc);
        v.pod(&mut self.store_field);
        v.pod(&mut self.store_token);
        v.pod(&mut self.right_outer_delim);
        v.pod(&mut self.right_str_delim);
        v.pod(&mut self.at_bib_command);
        v.pod(&mut self.cur_macro_loc);
        v.arr_len(&mut self.cite_info);
        v.pod(&mut self.cite_hash_found);
        v.pod(&mut self.preamble_ptr);
        v.pod(&mut self.num_preamble_strings);
        v.pod(&mut self.bib_brace_level);
        v.arr_len(&mut self.lit_stack);
        v.arr_len(&mut self.lit_stk_type);
        v.pod(&mut self.lit_stk_ptr);
        v.pod(&mut self.cmd_str_ptr);
        v.pod(&mut self.ent_chr_ptr);
        v.pod(&mut self.glob_chr_ptr);
        v.arr_len(&mut self.ex_buf);
        v.pod(&mut self.ex_buf_ptr);
        v.pod(&mut self.ex_buf_length);
        v.arr_len(&mut self.out_buf);
        v.pod(&mut self.out_buf_ptr);
        v.pod(&mut self.out_buf_length);
        v.pod(&mut self.mess_with_entries);
        v.pod(&mut self.sort_cite_ptr);
        v.pod(&mut self.sort_key_num);
        v.pod(&mut self.brace_level);
        v.pod(&mut self.b_equals);
        v.pod(&mut self.b_greater_than);
        v.pod(&mut self.b_less_than);
        v.pod(&mut self.b_plus);
        v.pod(&mut self.b_minus);
        v.pod(&mut self.b_concatenate);
        v.pod(&mut self.b_gets);
        v.pod(&mut self.b_add_period);
        v.pod(&mut self.b_call_type);
        v.pod(&mut self.b_change_case);
        v.pod(&mut self.b_chr_to_int);
        v.pod(&mut self.b_cite);
        v.pod(&mut self.b_duplicate);
        v.pod(&mut self.b_empty);
        v.pod(&mut self.b_format_name);
        v.pod(&mut self.b_if);
        v.pod(&mut self.b_int_to_chr);
        v.pod(&mut self.b_int_to_str);
        v.pod(&mut self.b_missing);
        v.pod(&mut self.b_newline);
        v.pod(&mut self.b_num_names);
        v.pod(&mut self.b_pop);
        v.pod(&mut self.b_preamble);
        v.pod(&mut self.b_purify);
        v.pod(&mut self.b_quote);
        v.pod(&mut self.b_skip);
        v.pod(&mut self.b_stack);
        v.pod(&mut self.b_substring);
        v.pod(&mut self.b_swap);
        v.pod(&mut self.b_text_length);
        v.pod(&mut self.b_text_prefix);
        v.pod(&mut self.b_top_stack);
        v.pod(&mut self.b_type);
        v.pod(&mut self.b_warning);
        v.pod(&mut self.b_while);
        v.pod(&mut self.b_width);
        v.pod(&mut self.b_write);
        v.pod(&mut self.b_default);
        v.pod(&mut self.total_ex_count);
        v.pod(&mut self.blt_in_ptr);
        v.pod(&mut self.s_null);
        v.pod(&mut self.s_default);
        v.pod(&mut self.s_t);
        v.pod(&mut self.s_l);
        v.pod(&mut self.s_u);
        v.arr_len(&mut self.s_preamble);
        v.pod(&mut self.pop_lit1);
        v.pod(&mut self.pop_lit2);
        v.pod(&mut self.pop_lit3);
        v.pod(&mut self.pop_typ1);
        v.pod(&mut self.pop_typ2);
        v.pod(&mut self.pop_typ3);
        v.pod(&mut self.sp_ptr);
        v.pod(&mut self.sp_xptr1);
        v.pod(&mut self.sp_xptr2);
        v.pod(&mut self.sp_end);
        v.pod(&mut self.sp_length);
        v.pod(&mut self.sp2_length);
        v.pod(&mut self.sp_brace_level);
        v.pod(&mut self.ex_buf_xptr);
        v.pod(&mut self.ex_buf_yptr);
        v.pod(&mut self.control_seq_loc);
        v.pod(&mut self.preceding_white);
        v.pod(&mut self.and_found);
        v.pod(&mut self.num_names);
        v.pod(&mut self.name_bf_ptr);
        v.pod(&mut self.name_bf_xptr);
        v.pod(&mut self.name_bf_yptr);
        v.pod(&mut self.nm_brace_level);
        v.arr_len(&mut self.name_tok);
        v.arr_len(&mut self.name_sep_char);
        v.pod(&mut self.num_tokens);
        v.pod(&mut self.token_starting);
        v.pod(&mut self.alpha_found);
        v.pod(&mut self.double_letter);
        v.pod(&mut self.end_of_group);
        v.pod(&mut self.to_be_written);
        v.pod(&mut self.first_start);
        v.pod(&mut self.first_end);
        v.pod(&mut self.last_end);
        v.pod(&mut self.von_start);
        v.pod(&mut self.von_end);
        v.pod(&mut self.jr_end);
        v.pod(&mut self.cur_token);
        v.pod(&mut self.last_token);
        v.pod(&mut self.use_default);
        v.pod(&mut self.num_commas);
        v.pod(&mut self.comma1);
        v.pod(&mut self.comma2);
        v.pod(&mut self.num_text_chars);
        v.pod(&mut self.conversion_type);
        v.pod(&mut self.prev_colon);
        v.pod(&mut self.verbose);
        v.pod(&mut self.min_crossrefs);
    }

    /// Every file global, in declaration order (the checkpoint's host-state record).
    pub fn visit_files<V: crate::system::FileVisit>(&mut self, v: &mut V) {
        v.alpha(&mut self.standard_input);
        v.alpha(&mut self.standard_output);
        for f in self.aux_file.iter_mut() {
            v.alpha(f);
        }
        v.alpha(&mut self.log_file);
        v.alpha(&mut self.bbl_file);
        for f in self.bib_file.iter_mut() {
            v.alpha(f);
        }
        v.alpha(&mut self.bst_file);
    }
}
