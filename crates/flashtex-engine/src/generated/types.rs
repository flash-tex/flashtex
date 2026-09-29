// GENERATED FILE -- DO NOT EDIT.
// Types from WEB's `@<Types in the outer block@>`.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, clippy::all)]

use super::consts::*;

// §18
pub type ASCII_code = i32;
// §25
pub type eight_bits = i32;
// §25
pub type alpha_file = crate::system::AlphaFile;
// §25
pub type byte_file = crate::system::ByteFile;
// §38
pub type pool_pointer = i32;
// §38
pub type str_number = i32;
// §38
pub type packed_ASCII_code = i32;
// §101
pub type scaled = i32;
// §101
pub type nonnegative_integer = i32;
// §101
pub type small_number = i32;
// §109
pub type glue_ratio = f32;
// §113
pub type quarterword = i32;
// §113
pub type halfword = i32;
// §113
pub type two_choices = i32;
// §113
pub type four_choices = i32;
// §113
/// Bit-packed Pascal record (64 bits). The variant part of the WEB
/// declaration is a real overlay, so the fields alias exactly as they do
/// in `tex.web`.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct two_halves(pub u64);
impl two_halves {
    #[inline(always)]
    pub fn to_bits(&self) -> u64 { self.0 as u64 }
    #[inline(always)]
    pub fn from_bits(v: u64) -> Self { two_halves(v as u64) }
    #[inline(always)]
    pub fn rh(&self) -> i32 { ((self.0 >> 0) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_rh(&mut self, v: i32) { self.0 = (self.0 & !((4294967295 as u64) << 0)) | (((v as u64) & (4294967295 as u64)) << 0); }
    #[inline(always)]
    pub fn lh(&self) -> i32 { ((self.0 >> 32) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_lh(&mut self, v: i32) { self.0 = (self.0 & !((4294967295 as u64) << 32)) | (((v as u64) & (4294967295 as u64)) << 32); }
    #[inline(always)]
    pub fn b0(&self) -> i32 { ((self.0 >> 32) & 255) as i32 }
    #[inline(always)]
    pub fn set_b0(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 32)) | (((v as u64) & (255 as u64)) << 32); }
    #[inline(always)]
    pub fn b1(&self) -> i32 { ((self.0 >> 40) & 255) as i32 }
    #[inline(always)]
    pub fn set_b1(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 40)) | (((v as u64) & (255 as u64)) << 40); }
}
// §113
/// Bit-packed Pascal record (32 bits). The variant part of the WEB
/// declaration is a real overlay, so the fields alias exactly as they do
/// in `tex.web`.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct four_quarters(pub u32);
impl four_quarters {
    #[inline(always)]
    pub fn to_bits(&self) -> u64 { self.0 as u64 }
    #[inline(always)]
    pub fn from_bits(v: u64) -> Self { four_quarters(v as u32) }
    #[inline(always)]
    pub fn b0(&self) -> i32 { ((self.0 >> 0) & 255) as i32 }
    #[inline(always)]
    pub fn set_b0(&mut self, v: i32) { self.0 = (self.0 & !((255 as u32) << 0)) | (((v as u32) & (255 as u32)) << 0); }
    #[inline(always)]
    pub fn b1(&self) -> i32 { ((self.0 >> 8) & 255) as i32 }
    #[inline(always)]
    pub fn set_b1(&mut self, v: i32) { self.0 = (self.0 & !((255 as u32) << 8)) | (((v as u32) & (255 as u32)) << 8); }
    #[inline(always)]
    pub fn b2(&self) -> i32 { ((self.0 >> 16) & 255) as i32 }
    #[inline(always)]
    pub fn set_b2(&mut self, v: i32) { self.0 = (self.0 & !((255 as u32) << 16)) | (((v as u32) & (255 as u32)) << 16); }
    #[inline(always)]
    pub fn b3(&self) -> i32 { ((self.0 >> 24) & 255) as i32 }
    #[inline(always)]
    pub fn set_b3(&mut self, v: i32) { self.0 = (self.0 & !((255 as u32) << 24)) | (((v as u32) & (255 as u32)) << 24); }
}
// §113
/// Bit-packed Pascal record (64 bits). The variant part of the WEB
/// declaration is a real overlay, so the fields alias exactly as they do
/// in `tex.web`.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct memory_word(pub u64);
impl memory_word {
    #[inline(always)]
    pub fn to_bits(&self) -> u64 { self.0 as u64 }
    #[inline(always)]
    pub fn from_bits(v: u64) -> Self { memory_word(v as u64) }
    #[inline(always)]
    pub fn int(&self) -> i32 { ((self.0 >> 0) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_int(&mut self, v: i32) { self.0 = (self.0 & !((4294967295 as u64) << 0)) | (((v as u64) & (4294967295 as u64)) << 0); }
    #[inline(always)]
    pub fn gr(&self) -> f32 { f32::from_bits(((self.0 >> 0) & 4294967295) as u32) }
    #[inline(always)]
    pub fn set_gr(&mut self, v: f32) { self.0 = (self.0 & !((4294967295 as u64) << 0)) | (((v.to_bits() as u64) & (4294967295 as u64)) << 0); }
    #[inline(always)]
    pub fn hh_rh(&self) -> i32 { ((self.0 >> 0) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_hh_rh(&mut self, v: i32) { self.0 = (self.0 & !((4294967295 as u64) << 0)) | (((v as u64) & (4294967295 as u64)) << 0); }
    #[inline(always)]
    pub fn hh_lh(&self) -> i32 { ((self.0 >> 32) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_hh_lh(&mut self, v: i32) { self.0 = (self.0 & !((4294967295 as u64) << 32)) | (((v as u64) & (4294967295 as u64)) << 32); }
    #[inline(always)]
    pub fn hh_b0(&self) -> i32 { ((self.0 >> 32) & 255) as i32 }
    #[inline(always)]
    pub fn set_hh_b0(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 32)) | (((v as u64) & (255 as u64)) << 32); }
    #[inline(always)]
    pub fn hh_b1(&self) -> i32 { ((self.0 >> 40) & 255) as i32 }
    #[inline(always)]
    pub fn set_hh_b1(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 40)) | (((v as u64) & (255 as u64)) << 40); }
    #[inline(always)]
    pub fn qqqq_b0(&self) -> i32 { ((self.0 >> 0) & 255) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b0(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 0)) | (((v as u64) & (255 as u64)) << 0); }
    #[inline(always)]
    pub fn qqqq_b1(&self) -> i32 { ((self.0 >> 8) & 255) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b1(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 8)) | (((v as u64) & (255 as u64)) << 8); }
    #[inline(always)]
    pub fn qqqq_b2(&self) -> i32 { ((self.0 >> 16) & 255) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b2(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 16)) | (((v as u64) & (255 as u64)) << 16); }
    #[inline(always)]
    pub fn qqqq_b3(&self) -> i32 { ((self.0 >> 24) & 255) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b3(&mut self, v: i32) { self.0 = (self.0 & !((255 as u64) << 24)) | (((v as u64) & (255 as u64)) << 24); }
    #[inline(always)]
    pub fn hh(&self) -> two_halves { two_halves(((self.0 >> 0) & (18446744073709551615 as u64)) as u64) }
    #[inline(always)]
    pub fn set_hh(&mut self, v: two_halves) { self.0 = (self.0 & !((18446744073709551615 as u64) << 0)) | (((v.0 as u64) & (18446744073709551615 as u64)) << 0); }
    #[inline(always)]
    pub fn qqqq(&self) -> four_quarters { four_quarters(((self.0 >> 0) & (4294967295 as u64)) as u32) }
    #[inline(always)]
    pub fn set_qqqq(&mut self, v: four_quarters) { self.0 = (self.0 & !((4294967295 as u64) << 0)) | (((v.0 as u64) & (4294967295 as u64)) << 0); }
}
// §113
pub type word_file = crate::system::WordFile;
// §150
pub type glue_ord = i32;
// §212
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct list_state_record {
    pub mode_field: i32,
    pub head_field: halfword,
    pub tail_field: halfword,
    pub pg_field: i32,
    pub ml_field: i32,
    pub aux_field: memory_word,
}
// §269
pub type group_code = i32;
// §300
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct in_state_record {
    pub state_field: quarterword,
    pub index_field: quarterword,
    pub start_field: halfword,
    pub loc_field: halfword,
    pub limit_field: halfword,
    pub name_field: halfword,
}
// §548
pub type internal_font_number = i32;
// §548
pub type font_index = i32;
// §594
pub type dvi_index = i32;
// §920
pub type trie_pointer = i32;
// §925
pub type hyph_pointer = i32;
