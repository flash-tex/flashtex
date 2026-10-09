// GENERATED FILE -- DO NOT EDIT.
// Constants from WEB's `@<Constants in the outer block@>`, and the WEB macros
// that TANGLE writes as numbers.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, unused_comparisons, clippy::all)]

// §10
pub const hash_base: i32 = 1i32;
// §10
pub const quote_next_fn: i32 = (hash_base).wrapping_sub(1i32);
// §10
pub const BUF_SIZE: i32 = 20000i32;
// §10
pub const min_print_line: i32 = 3i32;
// §10
pub const MAX_PRINT_LINE: i32 = 60i32;
// §10
pub const aux_stack_size: i32 = 20i32;
// §10
pub const MAX_BIB_FILES: i32 = 20i32;
// §10
pub const POOL_SIZE: i32 = 65000i32;
// §10
pub const MAX_STRINGS: i32 = 4000i32;
// §10
pub const MAX_CITES: i32 = 750i32;
// §10
pub const WIZ_FN_SPACE: i32 = 3000i32;
// §10
pub const SINGLE_FN_SPACE: i32 = 50i32;
// §10
pub const ENT_STR_SIZE: i32 = 100i32;
// §10
pub const GLOB_STR_SIZE: i32 = 1000i32;
// §10
pub const MAX_GLOB_STRS: i32 = 10i32;
// §10
pub const MAX_FIELDS: i32 = 5000i32;
// §10
pub const LIT_STK_SIZE: i32 = 50i32;
// §324
pub const num_blt_in_fns: i32 = 37i32;

// WEB macros whose expansion is an integer constant.
// §5
pub const any_value: i32 = 0i32;
// §5
pub const empty: i32 = 0i32;
// §6
pub const kpse_bib_format: i32 = 1i32;
// §6
pub const kpse_bst_format: i32 = 2i32;
// §11
pub const HASH_SIZE: i32 = 5000i32;
// §11
pub const file_name_size: i32 = 2147483647i32;
// §14
pub const error_message: i32 = 2i32;
// §14
pub const fatal_message: i32 = 3i32;
// §14
pub const spotless: i32 = 0i32;
// §14
pub const warning_message: i32 = 1i32;
// §18
pub const first_text_char: i32 = 0i32;
// §18
pub const last_text_char: i32 = 255i32;
// §21
pub const invalid_code: i32 = 127i32;
// §21
pub const space: i32 = 32i32;
// §21
pub const tab: i32 = 9i32;
// §24
pub const at_sign: i32 = 64i32;
// §24
pub const backslash: i32 = 92i32;
// §24
pub const colon: i32 = 58i32;
// §24
pub const comma: i32 = 44i32;
// §24
pub const comment: i32 = 37i32;
// §24
pub const concat_char: i32 = 35i32;
// §24
pub const double_quote: i32 = 34i32;
// §24
pub const equals_sign: i32 = 61i32;
// §24
pub const exclamation_mark: i32 = 33i32;
// §24
pub const hyphen: i32 = 45i32;
// §24
pub const left_brace: i32 = 123i32;
// §24
pub const left_paren: i32 = 40i32;
// §24
pub const minus_sign: i32 = 45i32;
// §24
pub const number_sign: i32 = 35i32;
// §24
pub const period: i32 = 46i32;
// §24
pub const question_mark: i32 = 63i32;
// §24
pub const right_brace: i32 = 125i32;
// §24
pub const right_paren: i32 = 41i32;
// §24
pub const single_quote: i32 = 39i32;
// §24
pub const star: i32 = 42i32;
// §24
pub const tie: i32 = 126i32;
// §26
pub const alpha: i32 = 2i32;
// §26
pub const illegal: i32 = 0i32;
// §26
pub const illegal_id_char: i32 = 0i32;
// §26
pub const legal_id_char: i32 = 1i32;
// §26
pub const numeric: i32 = 3i32;
// §26
pub const other_lex: i32 = 5i32;
// §26
pub const sep_char: i32 = 4i32;
// §26
pub const white_space: i32 = 1i32;
// §57
pub const aux_command_ilk: i32 = 2i32;
// §57
pub const aux_file_ilk: i32 = 3i32;
// §57
pub const bib_command_ilk: i32 = 12i32;
// §57
pub const bib_file_ilk: i32 = 6i32;
// §57
pub const bst_command_ilk: i32 = 4i32;
// §57
pub const bst_file_ilk: i32 = 5i32;
// §57
pub const bst_fn_ilk: i32 = 11i32;
// §57
pub const cite_ilk: i32 = 9i32;
// §57
pub const control_seq_ilk: i32 = 14i32;
// §57
pub const file_area_ilk: i32 = 8i32;
// §57
pub const file_ext_ilk: i32 = 7i32;
// §57
pub const integer_ilk: i32 = 1i32;
// §57
pub const lc_cite_ilk: i32 = 10i32;
// §57
pub const macro_ilk: i32 = 13i32;
// §57
pub const text_ilk: i32 = 0i32;
// §71
pub const n_aux_bibdata: i32 = 0i32;
// §71
pub const n_aux_bibstyle: i32 = 1i32;
// §71
pub const n_aux_citation: i32 = 2i32;
// §71
pub const n_aux_input: i32 = 3i32;
// §71
pub const n_bib_comment: i32 = 0i32;
// §71
pub const n_bib_preamble: i32 = 1i32;
// §71
pub const n_bib_string: i32 = 2i32;
// §71
pub const n_bst_entry: i32 = 0i32;
// §71
pub const n_bst_execute: i32 = 1i32;
// §71
pub const n_bst_function: i32 = 2i32;
// §71
pub const n_bst_integers: i32 = 3i32;
// §71
pub const n_bst_iterate: i32 = 4i32;
// §71
pub const n_bst_macro: i32 = 5i32;
// §71
pub const n_bst_read: i32 = 6i32;
// §71
pub const n_bst_reverse: i32 = 7i32;
// §71
pub const n_bst_sort: i32 = 8i32;
// §71
pub const n_bst_strings: i32 = 9i32;
// §82
pub const id_null: i32 = 0i32;
// §82
pub const other_char_adjacent: i32 = 2i32;
// §82
pub const specified_char_adjacent: i32 = 1i32;
// §82
pub const white_adjacent: i32 = 3i32;
// §148
pub const built_in: i32 = 0i32;
// §148
pub const field: i32 = 4i32;
// §148
pub const int_entry_var: i32 = 5i32;
// §148
pub const int_global_var: i32 = 7i32;
// §148
pub const int_literal: i32 = 2i32;
// §148
pub const str_entry_var: i32 = 6i32;
// §148
pub const str_global_var: i32 = 8i32;
// §148
pub const str_literal: i32 = 3i32;
// §148
pub const wiz_defined: i32 = 1i32;
// §153
pub const missing: i32 = 0i32;
// §208
pub const end_of_string: i32 = 127i32;
// §282
pub const stk_empty: i32 = 4i32;
// §282
pub const stk_field_missing: i32 = 3i32;
// §282
pub const stk_fn: i32 = 2i32;
// §282
pub const stk_int: i32 = 0i32;
// §282
pub const stk_str: i32 = 1i32;
// §293
pub const end_offset: i32 = 4i32;
// §293
pub const short_list: i32 = 10i32;
// §324
pub const n_add_period: i32 = 7i32;
// §324
pub const n_call_type: i32 = 8i32;
// §324
pub const n_change_case: i32 = 9i32;
// §324
pub const n_chr_to_int: i32 = 10i32;
// §324
pub const n_cite: i32 = 11i32;
// §324
pub const n_concatenate: i32 = 5i32;
// §324
pub const n_duplicate: i32 = 12i32;
// §324
pub const n_empty: i32 = 13i32;
// §324
pub const n_equals: i32 = 0i32;
// §324
pub const n_format_name: i32 = 14i32;
// §324
pub const n_gets: i32 = 6i32;
// §324
pub const n_greater_than: i32 = 1i32;
// §324
pub const n_if: i32 = 15i32;
// §324
pub const n_int_to_chr: i32 = 16i32;
// §324
pub const n_int_to_str: i32 = 17i32;
// §324
pub const n_less_than: i32 = 2i32;
// §324
pub const n_minus: i32 = 4i32;
// §324
pub const n_missing: i32 = 18i32;
// §324
pub const n_newline: i32 = 19i32;
// §324
pub const n_num_names: i32 = 20i32;
// §324
pub const n_plus: i32 = 3i32;
// §324
pub const n_pop: i32 = 21i32;
// §324
pub const n_preamble: i32 = 22i32;
// §324
pub const n_purify: i32 = 23i32;
// §324
pub const n_quote: i32 = 24i32;
// §324
pub const n_skip: i32 = 25i32;
// §324
pub const n_stack: i32 = 26i32;
// §324
pub const n_substring: i32 = 27i32;
// §324
pub const n_swap: i32 = 28i32;
// §324
pub const n_text_length: i32 = 29i32;
// §324
pub const n_text_prefix: i32 = 30i32;
// §324
pub const n_top_stack: i32 = 31i32;
// §324
pub const n_type: i32 = 32i32;
// §324
pub const n_warning: i32 = 33i32;
// §324
pub const n_while: i32 = 34i32;
// §324
pub const n_width: i32 = 35i32;
// §324
pub const n_write: i32 = 36i32;
// §329
pub const n_aa: i32 = 6i32;
// §329
pub const n_aa_upper: i32 = 7i32;
// §329
pub const n_ae: i32 = 4i32;
// §329
pub const n_ae_upper: i32 = 5i32;
// §329
pub const n_i: i32 = 0i32;
// §329
pub const n_j: i32 = 1i32;
// §329
pub const n_l: i32 = 10i32;
// §329
pub const n_l_upper: i32 = 11i32;
// §329
pub const n_o: i32 = 8i32;
// §329
pub const n_o_upper: i32 = 9i32;
// §329
pub const n_oe: i32 = 2i32;
// §329
pub const n_oe_upper: i32 = 3i32;
// §329
pub const n_ss: i32 = 12i32;
// §356
pub const all_lowers: i32 = 1i32;
// §356
pub const all_uppers: i32 = 2i32;
// §356
pub const bad_conversion: i32 = 3i32;
// §356
pub const title_lowers: i32 = 0i32;
// §408
pub const long_token: i32 = 3i32;
// §410
pub const long_name: i32 = 3i32;
