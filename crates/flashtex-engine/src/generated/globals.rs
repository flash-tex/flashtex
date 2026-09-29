// GENERATED FILE -- DO NOT EDIT.
// All WEB globals in one arena struct (DESIGN.md §4.2).
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, clippy::all)]

use super::consts::*;
use super::types::*;

pub struct Globals {
    // §13
    pub bad: i32,
    // §20
    pub xord: Vec<ASCII_code>,
    // §20
    pub xchr: [u8; 256],
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
    // §115
    pub temp_ptr: halfword,
    // §116
    pub mem: Vec<memory_word>,
    // §116
    pub lo_mem_max: halfword,
    // §116
    pub hi_mem_min: halfword,
    // §117
    pub var_used: i32,
    // §117
    pub dyn_used: i32,
    // §118
    pub avail: halfword,
    // §118
    pub mem_end: halfword,
    // §124
    pub rover: halfword,
    // §173
    pub font_in_short_display: i32,
    // §181
    pub depth_threshold: i32,
    // §181
    pub breadth_max: i32,
    // §213
    pub nest: Vec<list_state_record>,
    // §213
    pub nest_ptr: i32,
    // §213
    pub max_nest_stack: i32,
    // §213
    pub cur_list: list_state_record,
    // §213
    pub shown_mode: i32,
    // §246
    pub old_setting: i32,
    // §246
    pub sys_time: i32,
    // §246
    pub sys_day: i32,
    // §246
    pub sys_month: i32,
    // §246
    pub sys_year: i32,
    // §253
    pub eqtb: Vec<memory_word>,
    // §253
    pub xeq_level: Vec<quarterword>,
    // §256
    pub hash: Vec<two_halves>,
    // §256
    pub hash_used: halfword,
    // §256
    pub no_new_control_sequence: bool,
    // §256
    pub cs_count: i32,
    // §271
    pub save_stack: Vec<memory_word>,
    // §271
    pub save_ptr: i32,
    // §271
    pub max_save_stack: i32,
    // §271
    pub cur_level: quarterword,
    // §271
    pub cur_group: group_code,
    // §271
    pub cur_boundary: i32,
    // §286
    pub mag_set: i32,
    // §297
    pub cur_cmd: eight_bits,
    // §297
    pub cur_chr: halfword,
    // §297
    pub cur_cs: halfword,
    // §297
    pub cur_tok: halfword,
    // §301
    pub input_stack: Vec<in_state_record>,
    // §301
    pub input_ptr: i32,
    // §301
    pub max_in_stack: i32,
    // §301
    pub cur_input: in_state_record,
    // §304
    pub in_open: i32,
    // §304
    pub open_parens: i32,
    // §304
    pub input_file: Vec<crate::system::AlphaFile>,
    // §304
    pub line: i32,
    // §304
    pub line_stack: Vec<i32>,
    // §305
    pub scanner_status: i32,
    // §305
    pub warning_index: halfword,
    // §305
    pub def_ref: halfword,
    // §308
    pub param_stack: Vec<halfword>,
    // §308
    pub param_ptr: i32,
    // §308
    pub max_param_stack: i32,
    // §309
    pub align_state: i32,
    // §310
    pub base_ptr: i32,
    // §333
    pub par_loc: halfword,
    // §333
    pub par_token: halfword,
    // §361
    pub force_eof: bool,
    // §382
    pub cur_mark: Vec<halfword>,
    // §387
    pub long_state: i32,
    // §388
    pub pstack: Vec<halfword>,
    // §410
    pub cur_val: i32,
    // §410
    pub cur_val_level: i32,
    // §438
    pub radix: small_number,
    // §447
    pub cur_order: glue_ord,
    // §480
    pub read_file: Vec<crate::system::AlphaFile>,
    // §480
    pub read_open: Vec<i32>,
    // §489
    pub cond_ptr: halfword,
    // §489
    pub if_limit: i32,
    // §489
    pub cur_if: small_number,
    // §489
    pub if_line: i32,
    // §493
    pub skip_line: i32,
    // §512
    pub cur_name: str_number,
    // §512
    pub cur_area: str_number,
    // §512
    pub cur_ext: str_number,
    // §513
    pub area_delimiter: pool_pointer,
    // §513
    pub ext_delimiter: pool_pointer,
    // §520
    pub TEX_format_default: [u8; 20],
    // §527
    pub name_in_progress: bool,
    // §527
    pub job_name: str_number,
    // §527
    pub log_opened: bool,
    // §532
    pub dvi_file: crate::system::ByteFile,
    // §532
    pub output_file_name: str_number,
    // §532
    pub log_name: str_number,
    // §539
    pub tfm_file: crate::system::ByteFile,
    // §549
    pub font_info: Vec<memory_word>,
    // §549
    pub fmem_ptr: font_index,
    // §549
    pub font_ptr: internal_font_number,
    // §549
    pub font_check: Vec<four_quarters>,
    // §549
    pub font_size: Vec<scaled>,
    // §549
    pub font_dsize: Vec<scaled>,
    // §549
    pub font_params: Vec<font_index>,
    // §549
    pub font_name: Vec<str_number>,
    // §549
    pub font_area: Vec<str_number>,
    // §549
    pub font_bc: Vec<eight_bits>,
    // §549
    pub font_ec: Vec<eight_bits>,
    // §549
    pub font_glue: Vec<halfword>,
    // §549
    pub font_used: Vec<bool>,
    // §549
    pub hyphen_char: Vec<i32>,
    // §549
    pub skew_char: Vec<i32>,
    // §549
    pub bchar_label: Vec<font_index>,
    // §549
    pub font_bchar: Vec<i32>,
    // §549
    pub font_false_bchar: Vec<i32>,
    // §550
    pub char_base: Vec<i32>,
    // §550
    pub width_base: Vec<i32>,
    // §550
    pub height_base: Vec<i32>,
    // §550
    pub depth_base: Vec<i32>,
    // §550
    pub italic_base: Vec<i32>,
    // §550
    pub lig_kern_base: Vec<i32>,
    // §550
    pub kern_base: Vec<i32>,
    // §550
    pub exten_base: Vec<i32>,
    // §550
    pub param_base: Vec<i32>,
    // §555
    pub null_character: four_quarters,
    // §592
    pub total_pages: i32,
    // §592
    pub max_v: scaled,
    // §592
    pub max_h: scaled,
    // §592
    pub max_push: i32,
    // §592
    pub last_bop: i32,
    // §592
    pub dead_cycles: i32,
    // §592
    pub doing_leaders: bool,
    // §592
    pub c: quarterword,
    // §592
    pub f: quarterword,
    // §592
    pub rule_ht: scaled,
    // §592
    pub rule_dp: scaled,
    // §592
    pub rule_wd: scaled,
    // §592
    pub g: halfword,
    // §592
    pub lq: i32,
    // §592
    pub lr: i32,
    // §595
    pub dvi_buf: Vec<eight_bits>,
    // §595
    pub half_buf: dvi_index,
    // §595
    pub dvi_limit: dvi_index,
    // §595
    pub dvi_ptr: dvi_index,
    // §595
    pub dvi_offset: i32,
    // §595
    pub dvi_gone: i32,
    // §605
    pub down_ptr: halfword,
    // §605
    pub right_ptr: halfword,
    // §616
    pub dvi_h: scaled,
    // §616
    pub dvi_v: scaled,
    // §616
    pub cur_h: scaled,
    // §616
    pub cur_v: scaled,
    // §616
    pub dvi_f: internal_font_number,
    // §616
    pub cur_s: i32,
    // §646
    pub total_stretch: Vec<scaled>,
    // §646
    pub total_shrink: Vec<scaled>,
    // §646
    pub last_badness: i32,
    // §647
    pub adjust_tail: halfword,
    // §661
    pub pack_begin_line: i32,
    // §684
    pub empty_field: two_halves,
    // §684
    pub null_delimiter: four_quarters,
    // §719
    pub cur_mlist: halfword,
    // §719
    pub cur_style: small_number,
    // §719
    pub cur_size: small_number,
    // §719
    pub cur_mu: scaled,
    // §719
    pub mlist_penalties: bool,
    // §724
    pub cur_f: internal_font_number,
    // §724
    pub cur_c: quarterword,
    // §724
    pub cur_i: four_quarters,
    // §764
    pub magic_offset: i32,
    // §770
    pub cur_align: halfword,
    // §770
    pub cur_span: halfword,
    // §770
    pub cur_loop: halfword,
    // §770
    pub align_ptr: halfword,
    // §770
    pub cur_head: halfword,
    // §770
    pub cur_tail: halfword,
    // §814
    pub just_box: halfword,
    // §821
    pub passive: halfword,
    // §821
    pub printed_node: halfword,
    // §821
    pub pass_number: halfword,
    // §823
    pub active_width: Vec<scaled>,
    // §823
    pub cur_active_width: Vec<scaled>,
    // §823
    pub background: Vec<scaled>,
    // §823
    pub break_width: Vec<scaled>,
    // §825
    pub no_shrink_error_yet: bool,
    // §828
    pub cur_p: halfword,
    // §828
    pub second_pass: bool,
    // §828
    pub final_pass: bool,
    // §828
    pub threshold: i32,
    // §833
    pub minimal_demerits: Vec<i32>,
    // §833
    pub minimum_demerits: i32,
    // §833
    pub best_place: Vec<halfword>,
    // §833
    pub best_pl_line: Vec<halfword>,
    // §839
    pub disc_width: scaled,
    // §847
    pub easy_line: halfword,
    // §847
    pub last_special_line: halfword,
    // §847
    pub first_width: scaled,
    // §847
    pub second_width: scaled,
    // §847
    pub first_indent: scaled,
    // §847
    pub second_indent: scaled,
    // §872
    pub best_bet: halfword,
    // §872
    pub fewest_demerits: i32,
    // §872
    pub best_line: halfword,
    // §872
    pub actual_looseness: i32,
    // §872
    pub line_diff: i32,
    // §892
    pub hc: Vec<i32>,
    // §892
    pub hn: i32,
    // §892
    pub ha: halfword,
    // §892
    pub hb: halfword,
    // §892
    pub hf: internal_font_number,
    // §892
    pub hu: Vec<i32>,
    // §892
    pub hyf_char: i32,
    // §892
    pub cur_lang: ASCII_code,
    // §892
    pub init_cur_lang: ASCII_code,
    // §892
    pub l_hyf: i32,
    // §892
    pub r_hyf: i32,
    // §892
    pub init_l_hyf: i32,
    // §892
    pub init_r_hyf: i32,
    // §892
    pub hyf_bchar: halfword,
    // §900
    pub hyf: Vec<i32>,
    // §900
    pub init_list: halfword,
    // §900
    pub init_lig: bool,
    // §900
    pub init_lft: bool,
    // §905
    pub hyphen_passed: small_number,
    // §907
    pub cur_l: halfword,
    // §907
    pub cur_r: halfword,
    // §907
    pub cur_q: halfword,
    // §907
    pub lig_stack: halfword,
    // §907
    pub ligature_present: bool,
    // §907
    pub lft_hit: bool,
    // §907
    pub rt_hit: bool,
    // §921
    pub trie: Vec<two_halves>,
    // §921
    pub hyf_distance: Vec<small_number>,
    // §921
    pub hyf_num: Vec<small_number>,
    // §921
    pub hyf_next: Vec<quarterword>,
    // §921
    pub op_start: Vec<i32>,
    // §926
    pub hyph_word: Vec<str_number>,
    // §926
    pub hyph_list: Vec<halfword>,
    // §926
    pub hyph_count: hyph_pointer,
    // §943
    pub trie_op_hash: Vec<i32>,
    // §943
    pub trie_used: Vec<quarterword>,
    // §943
    pub trie_op_lang: Vec<ASCII_code>,
    // §943
    pub trie_op_val: Vec<quarterword>,
    // §943
    pub trie_op_ptr: i32,
    // §947
    pub trie_c: Vec<packed_ASCII_code>,
    // §947
    pub trie_o: Vec<quarterword>,
    // §947
    pub trie_l: Vec<trie_pointer>,
    // §947
    pub trie_r: Vec<trie_pointer>,
    // §947
    pub trie_ptr: trie_pointer,
    // §947
    pub trie_hash: Vec<trie_pointer>,
    // §950
    pub trie_taken: Vec<bool>,
    // §950
    pub trie_min: Vec<trie_pointer>,
    // §950
    pub trie_max: trie_pointer,
    // §950
    pub trie_not_ready: bool,
    // §971
    pub best_height_plus_depth: scaled,
    // §980
    pub page_tail: halfword,
    // §980
    pub page_contents: i32,
    // §980
    pub page_max_depth: scaled,
    // §980
    pub best_page_break: halfword,
    // §980
    pub least_page_cost: i32,
    // §980
    pub best_size: scaled,
    // §982
    pub page_so_far: Vec<scaled>,
    // §982
    pub last_glue: halfword,
    // §982
    pub last_penalty: i32,
    // §982
    pub last_kern: scaled,
    // §982
    pub insert_penalties: i32,
    // §989
    pub output_active: bool,
    // §1032
    pub main_f: internal_font_number,
    // §1032
    pub main_i: four_quarters,
    // §1032
    pub main_j: four_quarters,
    // §1032
    pub main_k: font_index,
    // §1032
    pub main_p: halfword,
    // §1032
    pub main_s: i32,
    // §1032
    pub bchar: halfword,
    // §1032
    pub false_bchar: halfword,
    // §1032
    pub cancel_boundary: bool,
    // §1032
    pub ins_disc: bool,
    // §1074
    pub cur_box: halfword,
    // §1266
    pub after_token: halfword,
    // §1281
    pub long_help_seen: bool,
    // §1299
    pub format_ident: str_number,
    // §1305
    pub fmt_file: crate::system::WordFile,
    // §1331
    pub ready_already: i32,
    // §1342
    pub write_file: Vec<crate::system::AlphaFile>,
    // §1342
    pub write_open: Vec<bool>,
    // §1345
    pub write_loc: halfword,
}

impl Globals {
    pub fn new() -> Box<Globals> {
        Box::new(Globals {
            bad: 0,
            xord: vec![0; 256],
            xchr: [0u8; 256],
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
            trick_buf: vec![0; 80],
            trick_count: 0,
            first_count: 0,
            interaction: 0,
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
            old_setting: 0,
            sys_time: 0,
            sys_day: 0,
            sys_month: 0,
            sys_year: 0,
            eqtb: vec![memory_word::default(); 619006],
            xeq_level: vec![0; 844],
            hash: vec![two_halves::default(); 615267],
            hash_used: 0,
            no_new_control_sequence: false,
            cs_count: 0,
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
            font_check: vec![four_quarters::default(); 256],
            font_size: vec![0; 256],
            font_dsize: vec![0; 256],
            font_params: vec![0; 256],
            font_name: vec![0; 256],
            font_area: vec![0; 256],
            font_bc: vec![0; 256],
            font_ec: vec![0; 256],
            font_glue: vec![0; 256],
            font_used: vec![false; 256],
            hyphen_char: vec![0; 256],
            skew_char: vec![0; 256],
            bchar_label: vec![0; 256],
            font_bchar: vec![0; 256],
            font_false_bchar: vec![0; 256],
            char_base: vec![0; 256],
            width_base: vec![0; 256],
            height_base: vec![0; 256],
            depth_base: vec![0; 256],
            italic_base: vec![0; 256],
            lig_kern_base: vec![0; 256],
            kern_base: vec![0; 256],
            exten_base: vec![0; 256],
            param_base: vec![0; 256],
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
            total_stretch: vec![0; 4],
            total_shrink: vec![0; 4],
            last_badness: 0,
            adjust_tail: 0,
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
            just_box: 0,
            passive: 0,
            printed_node: 0,
            pass_number: 0,
            active_width: vec![0; 6],
            cur_active_width: vec![0; 6],
            background: vec![0; 6],
            break_width: vec![0; 6],
            no_shrink_error_yet: false,
            cur_p: 0,
            second_pass: false,
            final_pass: false,
            threshold: 0,
            minimal_demerits: vec![0; 4],
            minimum_demerits: 0,
            best_place: vec![0; 4],
            best_pl_line: vec![0; 4],
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
            insert_penalties: 0,
            output_active: false,
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
        })
    }
}
