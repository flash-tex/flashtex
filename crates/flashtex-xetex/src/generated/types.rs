// GENERATED FILE -- DO NOT EDIT.
// Types from WEB's `@<Types in the outer block@>`.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, unused_comparisons, clippy::all)]

use super::consts::*;

// §18
pub type UTF16_code = i32;
// §18
pub type UTF8_code = i32;
// §18
pub type UnicodeScalar = i32;
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
pub type packed_UTF16_code = i32;
// §105
pub type scaled = i32;
// §105
pub type nonnegative_integer = i32;
// §105
pub type small_number = i32;
// §113
pub type glue_ratio = f64;
// §135
pub type quarterword = i32;
// §135
pub type halfword = i32;
// §135
pub type two_choices = i32;
// §135
pub type four_choices = i32;
// §135
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
    pub fn set_rh(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 0 } else { 4 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u32; // SAFETY: the field is bytes k..k+4 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u32) } }
    #[inline(always)]
    pub fn lh(&self) -> i32 { ((self.0 >> 32) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_lh(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 4 } else { 0 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u32; // SAFETY: the field is bytes k..k+4 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u32) } }
    #[inline(always)]
    pub fn b1(&self) -> i32 { ((self.0 >> 32) & 65535) as i32 }
    #[inline(always)]
    pub fn set_b1(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 4 } else { 2 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn b0(&self) -> i32 { ((self.0 >> 48) & 65535) as i32 }
    #[inline(always)]
    pub fn set_b0(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 6 } else { 0 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
}
// §135
/// Bit-packed Pascal record (64 bits). The variant part of the WEB
/// declaration is a real overlay, so the fields alias exactly as they do
/// in `tex.web`.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct four_quarters(pub u64);
impl four_quarters {
    #[inline(always)]
    pub fn to_bits(&self) -> u64 { self.0 as u64 }
    #[inline(always)]
    pub fn from_bits(v: u64) -> Self { four_quarters(v as u64) }
    #[inline(always)]
    pub fn b0(&self) -> i32 { ((self.0 >> 0) & 65535) as i32 }
    #[inline(always)]
    pub fn set_b0(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 0 } else { 6 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn b1(&self) -> i32 { ((self.0 >> 16) & 65535) as i32 }
    #[inline(always)]
    pub fn set_b1(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 2 } else { 4 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn b2(&self) -> i32 { ((self.0 >> 32) & 65535) as i32 }
    #[inline(always)]
    pub fn set_b2(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 4 } else { 2 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn b3(&self) -> i32 { ((self.0 >> 48) & 65535) as i32 }
    #[inline(always)]
    pub fn set_b3(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 6 } else { 0 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
}
// §135
/// Bit-packed Pascal record (64 bits). The variant part of the WEB
/// declaration is a real overlay, so the fields alias exactly as they do
/// in `tex.web`.
///
/// A 64-bit `real` member is stored with its bits rotated by 32, so that
/// its sign and exponent land where an overlapping `integer` member reads
/// them. tex.web §186 is marked `@^system dependencies@>` and assumes a
/// nonzero real has absolute value 2^20 or more when taken as an integer,
/// which holds for a 32-bit real; the rotation makes it hold for a 64-bit
/// one. Representation only (DESIGN.md §4.2). With
/// `--scalar glue_ratio=f32` the question does not arise.
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
    pub fn set_int(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 0 } else { 4 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u32; // SAFETY: the field is bytes k..k+4 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u32) } }
    #[inline(always)]
    pub fn gr(&self) -> f64 { f64::from_bits((self.0 as u64).rotate_left(32)) }
    #[inline(always)]
    pub fn set_gr(&mut self, v: f64) { self.0 = v.to_bits().rotate_right(32) as u64; }
    #[inline(always)]
    pub fn hh_rh(&self) -> i32 { ((self.0 >> 0) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_hh_rh(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 0 } else { 4 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u32; // SAFETY: the field is bytes k..k+4 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u32) } }
    #[inline(always)]
    pub fn hh_lh(&self) -> i32 { ((self.0 >> 32) & 4294967295) as u32 as i32 }
    #[inline(always)]
    pub fn set_hh_lh(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 4 } else { 0 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u32; // SAFETY: the field is bytes k..k+4 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u32) } }
    #[inline(always)]
    pub fn hh_b1(&self) -> i32 { ((self.0 >> 32) & 65535) as i32 }
    #[inline(always)]
    pub fn set_hh_b1(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 4 } else { 2 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn hh_b0(&self) -> i32 { ((self.0 >> 48) & 65535) as i32 }
    #[inline(always)]
    pub fn set_hh_b0(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 6 } else { 0 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn qqqq_b0(&self) -> i32 { ((self.0 >> 0) & 65535) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b0(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 0 } else { 6 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn qqqq_b1(&self) -> i32 { ((self.0 >> 16) & 65535) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b1(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 2 } else { 4 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn qqqq_b2(&self) -> i32 { ((self.0 >> 32) & 65535) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b2(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 4 } else { 2 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn qqqq_b3(&self) -> i32 { ((self.0 >> 48) & 65535) as i32 }
    #[inline(always)]
    pub fn set_qqqq_b3(&mut self, v: i32) { let k = if cfg!(target_endian = "little") { 6 } else { 0 }; let p = (&mut self.0 as *mut _ as *mut u8).wrapping_add(k) as *mut u16; // SAFETY: the field is bytes k..k+2 of this record, which `self` borrows mutably.
        unsafe { p.write_unaligned(v as u16) } }
    #[inline(always)]
    pub fn hh(&self) -> two_halves { two_halves(((self.0 >> 0) & (18446744073709551615 as u64)) as u64) }
    #[inline(always)]
    pub fn set_hh(&mut self, v: two_halves) { self.0 = (self.0 & !((18446744073709551615 as u64) << 0)) | (((v.0 as u64) & (18446744073709551615 as u64)) << 0); }
    #[inline(always)]
    pub fn qqqq(&self) -> four_quarters { four_quarters(((self.0 >> 0) & (18446744073709551615 as u64)) as u64) }
    #[inline(always)]
    pub fn set_qqqq(&mut self, v: four_quarters) { self.0 = (self.0 & !((18446744073709551615 as u64) << 0)) | (((v.0 as u64) & (18446744073709551615 as u64)) << 0); }
}
// §135
pub type word_file = crate::system::WordFile;
// §174
pub type glue_ord = i32;
// §238
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct list_state_record {
    pub mode_field: i32,
    pub head_field: halfword,
    pub tail_field: halfword,
    pub eTeX_aux_field: halfword,
    pub pg_field: i32,
    pub ml_field: i32,
    pub aux_field: memory_word,
}
// §299
pub type group_code = i32;
// §330
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct in_state_record {
    pub state_field: quarterword,
    pub index_field: quarterword,
    pub start_field: halfword,
    pub loc_field: halfword,
    pub limit_field: halfword,
    pub name_field: halfword,
    pub synctex_tag_field: i32,
}
// §583
pub type internal_font_number = i32;
// §583
pub type font_index = i32;
// §630
pub type dvi_index = i32;
// §974
pub type trie_pointer = i32;
// §979
pub type hyph_pointer = i32;
// §1488
pub type save_pointer = i32;
// §1689
pub type void_pointer = i32;
// §1689
pub type unicode_file = crate::system::AlphaFile;
// §1689
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct real_point {
    pub x: f64,
    pub y: f64,
}
// §1689
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct real_rect {
    pub x: f64,
    pub y: f64,
    pub wd: f64,
    pub ht: f64,
}
// §1689
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub x: f64,
    pub y: f64,
}
