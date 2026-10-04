//! Raw FFI to the vendored HarfBuzz 12.3.2: every function, type and constant
//! XeTeX's C/C++ layer uses (XeTeXFontInst.cpp, XeTeXLayoutInterface.cpp,
//! XeTeXOTMath.cpp, XeTeXFontMgr.cpp, XeTeX_ext.c), plus `hb_version`,
//! `hb_blob_destroy`, `hb_font_funcs_destroy`, `hb_font_funcs_make_immutable`
//! and `hb_unicode_funcs_get_default`. Names are HarfBuzz's, verbatim.
//!
//! Not declared: `hb_graphite2_face_get_gr_face` and
//! `hb_graphite2_font_get_gr_font` (Graphite2, phase S2: they are compiled
//! only with `HAVE_GRAPHITE2`), and `hb_icu_get_unicode_funcs`, which XeTeX
//! calls only `#if !HB_VERSION_ATLEAST(2,5,0)`, dead code against 12.3.2
//! (HarfBuzz's own UCD tables serve the Unicode functions).
//!
//! Layouts are transcribed from `src/hb-*.h` and checked against the C
//! compiler by `csrc/layout_probe.c`.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::os::raw::{c_char, c_int, c_uint, c_void};

pub type hb_bool_t = c_int;
pub type hb_codepoint_t = u32;
pub type hb_position_t = i32;
pub type hb_mask_t = u32;
pub type hb_tag_t = u32;
pub type hb_ot_name_id_t = c_uint;

/// C enums (int-sized; the probe checks `sizeof`).
pub type hb_direction_t = c_uint;
pub type hb_script_t = c_uint;
pub type hb_memory_mode_t = c_uint;
pub type hb_buffer_content_type_t = c_uint;
pub type hb_buffer_serialize_format_t = c_uint;
pub type hb_buffer_serialize_flags_t = c_uint;
pub type hb_ot_math_constant_t = c_uint;
pub type hb_ot_math_kern_t = c_uint;
pub type hb_ot_math_glyph_part_flags_t = c_uint;

macro_rules! opaque {
    ($($name:ident),* $(,)?) => {$(
        #[repr(C)]
        pub struct $name {
            _private: [u8; 0],
            _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
        }
    )*};
}
opaque!(
    hb_blob_t,
    hb_buffer_t,
    hb_face_t,
    hb_font_t,
    hb_font_funcs_t,
    hb_shape_plan_t,
    hb_unicode_funcs_t,
    hb_language_impl_t,
);

pub type hb_language_t = *const hb_language_impl_t;

#[repr(C)]
#[derive(Clone, Copy)]
pub union hb_var_int_t {
    pub u32_: u32,
    pub i32_: i32,
    pub u16_: [u16; 2],
    pub i16_: [i16; 2],
    pub u8_: [u8; 4],
    pub i8_: [i8; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hb_glyph_info_t {
    pub codepoint: hb_codepoint_t,
    pub mask: hb_mask_t,
    pub cluster: u32,
    pub var1: hb_var_int_t,
    pub var2: hb_var_int_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hb_glyph_position_t {
    pub x_advance: hb_position_t,
    pub y_advance: hb_position_t,
    pub x_offset: hb_position_t,
    pub y_offset: hb_position_t,
    pub var: hb_var_int_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct hb_feature_t {
    pub tag: hb_tag_t,
    pub value: u32,
    pub start: c_uint,
    pub end: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct hb_segment_properties_t {
    pub direction: hb_direction_t,
    pub script: hb_script_t,
    pub language: hb_language_t,
    pub reserved1: *mut c_void,
    pub reserved2: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct hb_glyph_extents_t {
    pub x_bearing: hb_position_t,
    pub y_bearing: hb_position_t,
    pub width: hb_position_t,
    pub height: hb_position_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct hb_ot_math_glyph_variant_t {
    pub glyph: hb_codepoint_t,
    pub advance: hb_position_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct hb_ot_math_glyph_part_t {
    pub glyph: hb_codepoint_t,
    pub start_connector_length: hb_position_t,
    pub end_connector_length: hb_position_t,
    pub full_advance: hb_position_t,
    pub flags: hb_ot_math_glyph_part_flags_t,
}

pub type hb_destroy_func_t = Option<unsafe extern "C" fn(user_data: *mut c_void)>;
pub type hb_reference_table_func_t = Option<
    unsafe extern "C" fn(
        face: *mut hb_face_t,
        tag: hb_tag_t,
        user_data: *mut c_void,
    ) -> *mut hb_blob_t,
>;
/// The deprecated `hb_font_get_glyph_func_t` XeTeX installs (nominal and
/// variation-selector lookup in one callback).
pub type hb_font_get_glyph_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        unicode: hb_codepoint_t,
        variation_selector: hb_codepoint_t,
        glyph: *mut hb_codepoint_t,
        user_data: *mut c_void,
    ) -> hb_bool_t,
>;
pub type hb_font_get_glyph_advance_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        glyph: hb_codepoint_t,
        user_data: *mut c_void,
    ) -> hb_position_t,
>;
pub type hb_font_get_glyph_h_advance_func_t = hb_font_get_glyph_advance_func_t;
pub type hb_font_get_glyph_v_advance_func_t = hb_font_get_glyph_advance_func_t;
pub type hb_font_get_glyph_origin_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        glyph: hb_codepoint_t,
        x: *mut hb_position_t,
        y: *mut hb_position_t,
        user_data: *mut c_void,
    ) -> hb_bool_t,
>;
pub type hb_font_get_glyph_h_origin_func_t = hb_font_get_glyph_origin_func_t;
pub type hb_font_get_glyph_v_origin_func_t = hb_font_get_glyph_origin_func_t;
pub type hb_font_get_glyph_kerning_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        first_glyph: hb_codepoint_t,
        second_glyph: hb_codepoint_t,
        user_data: *mut c_void,
    ) -> hb_position_t,
>;
pub type hb_font_get_glyph_h_kerning_func_t = hb_font_get_glyph_kerning_func_t;
pub type hb_font_get_glyph_v_kerning_func_t = hb_font_get_glyph_kerning_func_t;
pub type hb_font_get_glyph_extents_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        glyph: hb_codepoint_t,
        extents: *mut hb_glyph_extents_t,
        user_data: *mut c_void,
    ) -> hb_bool_t,
>;
pub type hb_font_get_glyph_contour_point_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        glyph: hb_codepoint_t,
        point_index: c_uint,
        x: *mut hb_position_t,
        y: *mut hb_position_t,
        user_data: *mut c_void,
    ) -> hb_bool_t,
>;
pub type hb_font_get_glyph_name_func_t = Option<
    unsafe extern "C" fn(
        font: *mut hb_font_t,
        font_data: *mut c_void,
        glyph: hb_codepoint_t,
        name: *mut c_char,
        size: c_uint,
        user_data: *mut c_void,
    ) -> hb_bool_t,
>;
pub type hb_unicode_decompose_compatibility_func_t = Option<
    unsafe extern "C" fn(
        ufuncs: *mut hb_unicode_funcs_t,
        u: hb_codepoint_t,
        decomposed: *mut hb_codepoint_t,
        user_data: *mut c_void,
    ) -> c_uint,
>;

/// `HB_TAG(a,b,c,d)` (hb-common.h).
pub const fn HB_TAG(a: u8, b: u8, c: u8, d: u8) -> hb_tag_t {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | d as u32
}
pub const HB_TAG_NONE: hb_tag_t = 0;

pub const HB_DIRECTION_INVALID: hb_direction_t = 0;
pub const HB_DIRECTION_LTR: hb_direction_t = 4;
pub const HB_DIRECTION_RTL: hb_direction_t = 5;
pub const HB_DIRECTION_TTB: hb_direction_t = 6;
pub const HB_DIRECTION_BTT: hb_direction_t = 7;

pub const HB_MEMORY_MODE_DUPLICATE: hb_memory_mode_t = 0;
pub const HB_MEMORY_MODE_READONLY: hb_memory_mode_t = 1;
pub const HB_MEMORY_MODE_WRITABLE: hb_memory_mode_t = 2;
pub const HB_MEMORY_MODE_READONLY_MAY_MAKE_WRITABLE: hb_memory_mode_t = 3;

pub const HB_BUFFER_CONTENT_TYPE_INVALID: hb_buffer_content_type_t = 0;
pub const HB_BUFFER_CONTENT_TYPE_UNICODE: hb_buffer_content_type_t = 1;
pub const HB_BUFFER_CONTENT_TYPE_GLYPHS: hb_buffer_content_type_t = 2;

pub const HB_BUFFER_SERIALIZE_FORMAT_TEXT: hb_buffer_serialize_format_t =
    HB_TAG(b'T', b'E', b'X', b'T');
pub const HB_BUFFER_SERIALIZE_FORMAT_JSON: hb_buffer_serialize_format_t =
    HB_TAG(b'J', b'S', b'O', b'N');
pub const HB_BUFFER_SERIALIZE_FORMAT_INVALID: hb_buffer_serialize_format_t = HB_TAG_NONE;
pub const HB_BUFFER_SERIALIZE_FLAG_DEFAULT: hb_buffer_serialize_flags_t = 0;
/// The deprecated spelling XeTeX uses (hb-deprecated.h).
pub const HB_BUFFER_SERIALIZE_FLAGS_DEFAULT: hb_buffer_serialize_flags_t =
    HB_BUFFER_SERIALIZE_FLAG_DEFAULT;

pub const HB_SCRIPT_INVALID: hb_script_t = HB_TAG_NONE;
pub const HB_SCRIPT_COMMON: hb_script_t = HB_TAG(b'Z', b'y', b'y', b'y');
pub const HB_SCRIPT_LATIN: hb_script_t = HB_TAG(b'L', b'a', b't', b'n');

pub const HB_OT_TAG_GSUB: hb_tag_t = HB_TAG(b'G', b'S', b'U', b'B');
pub const HB_OT_TAG_GPOS: hb_tag_t = HB_TAG(b'G', b'P', b'O', b'S');
pub const HB_OT_TAG_MATH: hb_tag_t = HB_TAG(b'M', b'A', b'T', b'H');
pub const HB_OT_TAG_DEFAULT_SCRIPT: hb_tag_t = HB_TAG(b'D', b'F', b'L', b'T');
pub const HB_OT_TAG_DEFAULT_LANGUAGE: hb_tag_t = HB_TAG(b'd', b'f', b'l', b't');
pub const HB_OT_LAYOUT_NO_SCRIPT_INDEX: c_uint = 0xFFFF;
pub const HB_OT_LAYOUT_DEFAULT_LANGUAGE_INDEX: c_uint = 0xFFFF;

pub const HB_OT_MATH_KERN_TOP_RIGHT: hb_ot_math_kern_t = 0;
pub const HB_OT_MATH_KERN_TOP_LEFT: hb_ot_math_kern_t = 1;
pub const HB_OT_MATH_KERN_BOTTOM_RIGHT: hb_ot_math_kern_t = 2;
pub const HB_OT_MATH_KERN_BOTTOM_LEFT: hb_ot_math_kern_t = 3;

pub const HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER: hb_ot_math_glyph_part_flags_t = 1;

// hb_ot_math_constant_t (hb-ot-math.h), in order.
pub const HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN: hb_ot_math_constant_t = 0;
pub const HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN: hb_ot_math_constant_t = 1;
pub const HB_OT_MATH_CONSTANT_DELIMITED_SUB_FORMULA_MIN_HEIGHT: hb_ot_math_constant_t = 2;
pub const HB_OT_MATH_CONSTANT_DISPLAY_OPERATOR_MIN_HEIGHT: hb_ot_math_constant_t = 3;
pub const HB_OT_MATH_CONSTANT_MATH_LEADING: hb_ot_math_constant_t = 4;
pub const HB_OT_MATH_CONSTANT_AXIS_HEIGHT: hb_ot_math_constant_t = 5;
pub const HB_OT_MATH_CONSTANT_ACCENT_BASE_HEIGHT: hb_ot_math_constant_t = 6;
pub const HB_OT_MATH_CONSTANT_FLATTENED_ACCENT_BASE_HEIGHT: hb_ot_math_constant_t = 7;
pub const HB_OT_MATH_CONSTANT_SUBSCRIPT_SHIFT_DOWN: hb_ot_math_constant_t = 8;
pub const HB_OT_MATH_CONSTANT_SUBSCRIPT_TOP_MAX: hb_ot_math_constant_t = 9;
pub const HB_OT_MATH_CONSTANT_SUBSCRIPT_BASELINE_DROP_MIN: hb_ot_math_constant_t = 10;
pub const HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP: hb_ot_math_constant_t = 11;
pub const HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP_CRAMPED: hb_ot_math_constant_t = 12;
pub const HB_OT_MATH_CONSTANT_SUPERSCRIPT_BOTTOM_MIN: hb_ot_math_constant_t = 13;
pub const HB_OT_MATH_CONSTANT_SUPERSCRIPT_BASELINE_DROP_MAX: hb_ot_math_constant_t = 14;
pub const HB_OT_MATH_CONSTANT_SUB_SUPERSCRIPT_GAP_MIN: hb_ot_math_constant_t = 15;
pub const HB_OT_MATH_CONSTANT_SUPERSCRIPT_BOTTOM_MAX_WITH_SUBSCRIPT: hb_ot_math_constant_t = 16;
pub const HB_OT_MATH_CONSTANT_SPACE_AFTER_SCRIPT: hb_ot_math_constant_t = 17;
pub const HB_OT_MATH_CONSTANT_UPPER_LIMIT_GAP_MIN: hb_ot_math_constant_t = 18;
pub const HB_OT_MATH_CONSTANT_UPPER_LIMIT_BASELINE_RISE_MIN: hb_ot_math_constant_t = 19;
pub const HB_OT_MATH_CONSTANT_LOWER_LIMIT_GAP_MIN: hb_ot_math_constant_t = 20;
pub const HB_OT_MATH_CONSTANT_LOWER_LIMIT_BASELINE_DROP_MIN: hb_ot_math_constant_t = 21;
pub const HB_OT_MATH_CONSTANT_STACK_TOP_SHIFT_UP: hb_ot_math_constant_t = 22;
pub const HB_OT_MATH_CONSTANT_STACK_TOP_DISPLAY_STYLE_SHIFT_UP: hb_ot_math_constant_t = 23;
pub const HB_OT_MATH_CONSTANT_STACK_BOTTOM_SHIFT_DOWN: hb_ot_math_constant_t = 24;
pub const HB_OT_MATH_CONSTANT_STACK_BOTTOM_DISPLAY_STYLE_SHIFT_DOWN: hb_ot_math_constant_t = 25;
pub const HB_OT_MATH_CONSTANT_STACK_GAP_MIN: hb_ot_math_constant_t = 26;
pub const HB_OT_MATH_CONSTANT_STACK_DISPLAY_STYLE_GAP_MIN: hb_ot_math_constant_t = 27;
pub const HB_OT_MATH_CONSTANT_STRETCH_STACK_TOP_SHIFT_UP: hb_ot_math_constant_t = 28;
pub const HB_OT_MATH_CONSTANT_STRETCH_STACK_BOTTOM_SHIFT_DOWN: hb_ot_math_constant_t = 29;
pub const HB_OT_MATH_CONSTANT_STRETCH_STACK_GAP_ABOVE_MIN: hb_ot_math_constant_t = 30;
pub const HB_OT_MATH_CONSTANT_STRETCH_STACK_GAP_BELOW_MIN: hb_ot_math_constant_t = 31;
pub const HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_SHIFT_UP: hb_ot_math_constant_t = 32;
pub const HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_DISPLAY_STYLE_SHIFT_UP: hb_ot_math_constant_t = 33;
pub const HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_SHIFT_DOWN: hb_ot_math_constant_t = 34;
pub const HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_DISPLAY_STYLE_SHIFT_DOWN: hb_ot_math_constant_t =
    35;
pub const HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_GAP_MIN: hb_ot_math_constant_t = 36;
pub const HB_OT_MATH_CONSTANT_FRACTION_NUM_DISPLAY_STYLE_GAP_MIN: hb_ot_math_constant_t = 37;
pub const HB_OT_MATH_CONSTANT_FRACTION_RULE_THICKNESS: hb_ot_math_constant_t = 38;
pub const HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_GAP_MIN: hb_ot_math_constant_t = 39;
pub const HB_OT_MATH_CONSTANT_FRACTION_DENOM_DISPLAY_STYLE_GAP_MIN: hb_ot_math_constant_t = 40;
pub const HB_OT_MATH_CONSTANT_SKEWED_FRACTION_HORIZONTAL_GAP: hb_ot_math_constant_t = 41;
pub const HB_OT_MATH_CONSTANT_SKEWED_FRACTION_VERTICAL_GAP: hb_ot_math_constant_t = 42;
pub const HB_OT_MATH_CONSTANT_OVERBAR_VERTICAL_GAP: hb_ot_math_constant_t = 43;
pub const HB_OT_MATH_CONSTANT_OVERBAR_RULE_THICKNESS: hb_ot_math_constant_t = 44;
pub const HB_OT_MATH_CONSTANT_OVERBAR_EXTRA_ASCENDER: hb_ot_math_constant_t = 45;
pub const HB_OT_MATH_CONSTANT_UNDERBAR_VERTICAL_GAP: hb_ot_math_constant_t = 46;
pub const HB_OT_MATH_CONSTANT_UNDERBAR_RULE_THICKNESS: hb_ot_math_constant_t = 47;
pub const HB_OT_MATH_CONSTANT_UNDERBAR_EXTRA_DESCENDER: hb_ot_math_constant_t = 48;
pub const HB_OT_MATH_CONSTANT_RADICAL_VERTICAL_GAP: hb_ot_math_constant_t = 49;
pub const HB_OT_MATH_CONSTANT_RADICAL_DISPLAY_STYLE_VERTICAL_GAP: hb_ot_math_constant_t = 50;
pub const HB_OT_MATH_CONSTANT_RADICAL_RULE_THICKNESS: hb_ot_math_constant_t = 51;
pub const HB_OT_MATH_CONSTANT_RADICAL_EXTRA_ASCENDER: hb_ot_math_constant_t = 52;
pub const HB_OT_MATH_CONSTANT_RADICAL_KERN_BEFORE_DEGREE: hb_ot_math_constant_t = 53;
pub const HB_OT_MATH_CONSTANT_RADICAL_KERN_AFTER_DEGREE: hb_ot_math_constant_t = 54;
pub const HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT: hb_ot_math_constant_t = 55;

crate::ffi_functions! {
    pub fn hb_version(major: *mut c_uint, minor: *mut c_uint, micro: *mut c_uint);
    pub fn hb_version_string() -> *const c_char;
    pub fn hb_tag_from_string(str: *const c_char, len: c_int) -> hb_tag_t;
    pub fn hb_language_from_string(str: *const c_char, len: c_int) -> hb_language_t;
    pub fn hb_language_to_string(language: hb_language_t) -> *const c_char;
    pub fn hb_script_get_horizontal_direction(script: hb_script_t) -> hb_direction_t;

    pub fn hb_blob_create(data: *const c_char, length: c_uint, mode: hb_memory_mode_t, user_data: *mut c_void, destroy: hb_destroy_func_t) -> *mut hb_blob_t;
    pub fn hb_blob_destroy(blob: *mut hb_blob_t);

    pub fn hb_buffer_create() -> *mut hb_buffer_t;
    pub fn hb_buffer_destroy(buffer: *mut hb_buffer_t);
    pub fn hb_buffer_reset(buffer: *mut hb_buffer_t);
    pub fn hb_buffer_add_utf16(buffer: *mut hb_buffer_t, text: *const u16, text_length: c_int, item_offset: c_uint, item_length: c_int);
    pub fn hb_buffer_set_direction(buffer: *mut hb_buffer_t, direction: hb_direction_t);
    pub fn hb_buffer_set_script(buffer: *mut hb_buffer_t, script: hb_script_t);
    pub fn hb_buffer_get_script(buffer: *const hb_buffer_t) -> hb_script_t;
    pub fn hb_buffer_set_language(buffer: *mut hb_buffer_t, language: hb_language_t);
    pub fn hb_buffer_set_unicode_funcs(buffer: *mut hb_buffer_t, unicode_funcs: *mut hb_unicode_funcs_t);
    pub fn hb_buffer_set_content_type(buffer: *mut hb_buffer_t, content_type: hb_buffer_content_type_t);
    pub fn hb_buffer_guess_segment_properties(buffer: *mut hb_buffer_t);
    pub fn hb_buffer_get_segment_properties(buffer: *const hb_buffer_t, props: *mut hb_segment_properties_t);
    pub fn hb_buffer_get_length(buffer: *const hb_buffer_t) -> c_uint;
    pub fn hb_buffer_get_glyph_infos(buffer: *mut hb_buffer_t, length: *mut c_uint) -> *mut hb_glyph_info_t;
    pub fn hb_buffer_get_glyph_positions(buffer: *mut hb_buffer_t, length: *mut c_uint) -> *mut hb_glyph_position_t;
    pub fn hb_buffer_serialize_glyphs(buffer: *mut hb_buffer_t, start: c_uint, end: c_uint, buf: *mut c_char, buf_size: c_uint, buf_consumed: *mut c_uint, font: *mut hb_font_t, format: hb_buffer_serialize_format_t, flags: hb_buffer_serialize_flags_t) -> c_uint;

    pub fn hb_face_create_for_tables(reference_table_func: hb_reference_table_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t) -> *mut hb_face_t;
    pub fn hb_face_destroy(face: *mut hb_face_t);
    pub fn hb_face_set_index(face: *mut hb_face_t, index: c_uint);
    pub fn hb_face_set_upem(face: *mut hb_face_t, upem: c_uint);

    pub fn hb_font_create(face: *mut hb_face_t) -> *mut hb_font_t;
    pub fn hb_font_destroy(font: *mut hb_font_t);
    pub fn hb_font_get_face(font: *mut hb_font_t) -> *mut hb_face_t;
    pub fn hb_font_set_funcs(font: *mut hb_font_t, klass: *mut hb_font_funcs_t, font_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_set_scale(font: *mut hb_font_t, x_scale: c_int, y_scale: c_int);
    pub fn hb_font_set_ppem(font: *mut hb_font_t, x_ppem: c_uint, y_ppem: c_uint);

    pub fn hb_font_funcs_create() -> *mut hb_font_funcs_t;
    pub fn hb_font_funcs_destroy(ffuncs: *mut hb_font_funcs_t);
    pub fn hb_font_funcs_make_immutable(ffuncs: *mut hb_font_funcs_t);
    pub fn hb_font_funcs_set_glyph_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_h_advance_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_h_advance_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_v_advance_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_v_advance_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_h_origin_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_h_origin_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_v_origin_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_v_origin_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_h_kerning_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_h_kerning_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_v_kerning_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_v_kerning_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_extents_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_extents_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_contour_point_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_contour_point_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);
    pub fn hb_font_funcs_set_glyph_name_func(ffuncs: *mut hb_font_funcs_t, func: hb_font_get_glyph_name_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);

    pub fn hb_unicode_funcs_get_default() -> *mut hb_unicode_funcs_t;
    pub fn hb_unicode_funcs_create(parent: *mut hb_unicode_funcs_t) -> *mut hb_unicode_funcs_t;
    pub fn hb_unicode_funcs_set_decompose_compatibility_func(ufuncs: *mut hb_unicode_funcs_t, func: hb_unicode_decompose_compatibility_func_t, user_data: *mut c_void, destroy: hb_destroy_func_t);

    pub fn hb_shape_plan_create(face: *mut hb_face_t, props: *const hb_segment_properties_t, user_features: *const hb_feature_t, num_user_features: c_uint, shaper_list: *const *const c_char) -> *mut hb_shape_plan_t;
    pub fn hb_shape_plan_create_cached(face: *mut hb_face_t, props: *const hb_segment_properties_t, user_features: *const hb_feature_t, num_user_features: c_uint, shaper_list: *const *const c_char) -> *mut hb_shape_plan_t;
    pub fn hb_shape_plan_destroy(shape_plan: *mut hb_shape_plan_t);
    pub fn hb_shape_plan_execute(shape_plan: *mut hb_shape_plan_t, font: *mut hb_font_t, buffer: *mut hb_buffer_t, features: *const hb_feature_t, num_features: c_uint) -> hb_bool_t;
    pub fn hb_shape_plan_get_shaper(shape_plan: *mut hb_shape_plan_t) -> *const c_char;

    pub fn hb_ot_tag_to_script(tag: hb_tag_t) -> hb_script_t;
    pub fn hb_ot_tag_to_language(tag: hb_tag_t) -> hb_language_t;
    pub fn hb_ot_layout_table_get_script_tags(face: *mut hb_face_t, table_tag: hb_tag_t, start_offset: c_uint, script_count: *mut c_uint, script_tags: *mut hb_tag_t) -> c_uint;
    pub fn hb_ot_layout_table_find_script(face: *mut hb_face_t, table_tag: hb_tag_t, script_tag: hb_tag_t, script_index: *mut c_uint) -> hb_bool_t;
    pub fn hb_ot_layout_script_get_language_tags(face: *mut hb_face_t, table_tag: hb_tag_t, script_index: c_uint, start_offset: c_uint, language_count: *mut c_uint, language_tags: *mut hb_tag_t) -> c_uint;
    pub fn hb_ot_layout_script_find_language(face: *mut hb_face_t, table_tag: hb_tag_t, script_index: c_uint, language_tag: hb_tag_t, language_index: *mut c_uint) -> hb_bool_t;
    pub fn hb_ot_layout_language_get_feature_tags(face: *mut hb_face_t, table_tag: hb_tag_t, script_index: c_uint, language_index: c_uint, start_offset: c_uint, feature_count: *mut c_uint, feature_tags: *mut hb_tag_t) -> c_uint;
    pub fn hb_ot_layout_get_size_params(face: *mut hb_face_t, design_size: *mut c_uint, subfamily_id: *mut c_uint, subfamily_name_id: *mut hb_ot_name_id_t, range_start: *mut c_uint, range_end: *mut c_uint) -> hb_bool_t;

    pub fn hb_ot_math_has_data(face: *mut hb_face_t) -> hb_bool_t;
    pub fn hb_ot_math_get_constant(font: *mut hb_font_t, constant: hb_ot_math_constant_t) -> hb_position_t;
    pub fn hb_ot_math_get_glyph_italics_correction(font: *mut hb_font_t, glyph: hb_codepoint_t) -> hb_position_t;
    pub fn hb_ot_math_get_glyph_top_accent_attachment(font: *mut hb_font_t, glyph: hb_codepoint_t) -> hb_position_t;
    pub fn hb_ot_math_get_glyph_kerning(font: *mut hb_font_t, glyph: hb_codepoint_t, kern: hb_ot_math_kern_t, correction_height: hb_position_t) -> hb_position_t;
    pub fn hb_ot_math_get_glyph_variants(font: *mut hb_font_t, glyph: hb_codepoint_t, direction: hb_direction_t, start_offset: c_uint, variants_count: *mut c_uint, variants: *mut hb_ot_math_glyph_variant_t) -> c_uint;
    pub fn hb_ot_math_get_min_connector_overlap(font: *mut hb_font_t, direction: hb_direction_t) -> hb_position_t;
    pub fn hb_ot_math_get_glyph_assembly(font: *mut hb_font_t, glyph: hb_codepoint_t, direction: hb_direction_t, start_offset: c_uint, parts_count: *mut c_uint, parts: *mut hb_ot_math_glyph_part_t, italics_correction: *mut hb_position_t) -> c_uint;
}
