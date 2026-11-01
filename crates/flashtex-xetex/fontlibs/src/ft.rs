//! Raw FFI to the vendored FreeType 2.14.1: every function, type and constant
//! XeTeX's C/C++ layer uses (XeTeXFontInst.cpp, XeTeXFontMgr.cpp,
//! XeTeXFontMgr_FC.cpp, XeTeX_ext.c), plus `FT_Done_FreeType`,
//! `FT_New_Memory_Face` and the variation-font calls (`FT_MM_Var`) that
//! FlashTeX's font loading needs. Names are FreeType's, verbatim.
//!
//! Layouts are transcribed from `include/freetype/*.h` and checked against
//! the C compiler by `csrc/layout_probe.c` (crates/flashtex-xetex/tests/
//! fontlibs_layout.rs). `FT_Long`, `FT_Pos` and `FT_Fixed` are C `long`
//! (32 bits on Windows, 64 on LP64 hosts).
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::os::raw::{c_char, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_ushort, c_void};

pub type FT_Long = c_long;
pub type FT_ULong = c_ulong;
pub type FT_Int32 = c_int;
pub type FT_UInt32 = c_uint;
pub type FT_Pos = c_long;
pub type FT_Fixed = c_long;
pub type FT_Error = c_int;
pub type FT_Int = c_int;
pub type FT_UInt = c_uint;
pub type FT_Short = c_short;
pub type FT_UShort = c_ushort;
pub type FT_Byte = c_uchar;
pub type FT_Char = i8;
pub type FT_String = c_char;
pub type FT_Bool = c_uchar;
pub type FT_Tag = FT_UInt32;
pub type FT_Pointer = *mut c_void;

/// C enums (int-sized; the probe checks `sizeof`).
pub type FT_Encoding = c_uint;
pub type FT_Glyph_Format = c_uint;
pub type FT_Sfnt_Tag = c_uint;

/// Opaque records reached only through pointers.
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
    FT_LibraryRec,
    FT_SizeRec,
    FT_DriverRec,
    FT_MemoryRec,
    FT_StreamRec,
    FT_ListNodeRec,
    FT_Face_InternalRec,
    FT_SubGlyphRec,
    FT_Slot_InternalRec,
    FT_Glyph_Class,
);

pub type FT_Library = *mut FT_LibraryRec;
pub type FT_Face = *mut FT_FaceRec;
pub type FT_GlyphSlot = *mut FT_GlyphSlotRec;
pub type FT_CharMap = *mut FT_CharMapRec;
pub type FT_Size = *mut FT_SizeRec;
pub type FT_Driver = *mut FT_DriverRec;
pub type FT_Memory = *mut FT_MemoryRec;
pub type FT_Stream = *mut FT_StreamRec;
pub type FT_ListNode = *mut FT_ListNodeRec;
pub type FT_Face_Internal = *mut FT_Face_InternalRec;
pub type FT_SubGlyph = *mut FT_SubGlyphRec;
pub type FT_Slot_Internal = *mut FT_Slot_InternalRec;
pub type FT_Glyph = *mut FT_GlyphRec;

pub type FT_Generic_Finalizer = Option<unsafe extern "C" fn(object: *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FT_Vector {
    pub x: FT_Pos,
    pub y: FT_Pos,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FT_BBox {
    pub xMin: FT_Pos,
    pub yMin: FT_Pos,
    pub xMax: FT_Pos,
    pub yMax: FT_Pos,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_Generic {
    pub data: *mut c_void,
    pub finalizer: FT_Generic_Finalizer,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_ListRec {
    pub head: FT_ListNode,
    pub tail: FT_ListNode,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_Bitmap_Size {
    pub height: FT_Short,
    pub width: FT_Short,
    pub size: FT_Pos,
    pub x_ppem: FT_Pos,
    pub y_ppem: FT_Pos,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_CharMapRec {
    pub face: FT_Face,
    pub encoding: FT_Encoding,
    pub platform_id: FT_UShort,
    pub encoding_id: FT_UShort,
}

#[repr(C)]
#[derive(Debug)]
pub struct FT_FaceRec {
    pub num_faces: FT_Long,
    pub face_index: FT_Long,
    pub face_flags: FT_Long,
    pub style_flags: FT_Long,
    pub num_glyphs: FT_Long,
    pub family_name: *mut FT_String,
    pub style_name: *mut FT_String,
    pub num_fixed_sizes: FT_Int,
    pub available_sizes: *mut FT_Bitmap_Size,
    pub num_charmaps: FT_Int,
    pub charmaps: *mut FT_CharMap,
    pub generic: FT_Generic,
    pub bbox: FT_BBox,
    pub units_per_EM: FT_UShort,
    pub ascender: FT_Short,
    pub descender: FT_Short,
    pub height: FT_Short,
    pub max_advance_width: FT_Short,
    pub max_advance_height: FT_Short,
    pub underline_position: FT_Short,
    pub underline_thickness: FT_Short,
    pub glyph: FT_GlyphSlot,
    pub size: FT_Size,
    pub charmap: FT_CharMap,
    pub driver: FT_Driver,
    pub memory: FT_Memory,
    pub stream: FT_Stream,
    pub sizes_list: FT_ListRec,
    pub autohint: FT_Generic,
    pub extensions: *mut c_void,
    pub internal: FT_Face_Internal,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FT_Glyph_Metrics {
    pub width: FT_Pos,
    pub height: FT_Pos,
    pub horiBearingX: FT_Pos,
    pub horiBearingY: FT_Pos,
    pub horiAdvance: FT_Pos,
    pub vertBearingX: FT_Pos,
    pub vertBearingY: FT_Pos,
    pub vertAdvance: FT_Pos,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_Bitmap {
    pub rows: c_uint,
    pub width: c_uint,
    pub pitch: c_int,
    pub buffer: *mut c_uchar,
    pub num_grays: c_ushort,
    pub pixel_mode: c_uchar,
    pub palette_mode: c_uchar,
    pub palette: *mut c_void,
}

/// FreeType 2.14's outline: `n_contours` and `n_points` are `unsigned short`
/// (they were `short` before 2.13.3).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_Outline {
    pub n_contours: c_ushort,
    pub n_points: c_ushort,
    pub points: *mut FT_Vector,
    pub tags: *mut c_uchar,
    pub contours: *mut c_ushort,
    pub flags: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct FT_GlyphSlotRec {
    pub library: FT_Library,
    pub face: FT_Face,
    pub next: FT_GlyphSlot,
    pub glyph_index: FT_UInt,
    pub generic: FT_Generic,
    pub metrics: FT_Glyph_Metrics,
    pub linearHoriAdvance: FT_Fixed,
    pub linearVertAdvance: FT_Fixed,
    pub advance: FT_Vector,
    pub format: FT_Glyph_Format,
    pub bitmap: FT_Bitmap,
    pub bitmap_left: FT_Int,
    pub bitmap_top: FT_Int,
    pub outline: FT_Outline,
    pub num_subglyphs: FT_UInt,
    pub subglyphs: FT_SubGlyph,
    pub control_data: *mut c_void,
    pub control_len: c_long,
    pub lsb_delta: FT_Pos,
    pub rsb_delta: FT_Pos,
    pub other: *mut c_void,
    pub internal: FT_Slot_Internal,
}

#[repr(C)]
#[derive(Debug)]
pub struct FT_GlyphRec {
    pub library: FT_Library,
    pub clazz: *const FT_Glyph_Class,
    pub format: FT_Glyph_Format,
    pub advance: FT_Vector,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_SfntName {
    pub platform_id: FT_UShort,
    pub encoding_id: FT_UShort,
    pub language_id: FT_UShort,
    pub name_id: FT_UShort,
    pub string: *mut FT_Byte,
    pub string_len: FT_UInt,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TT_Header {
    pub Table_Version: FT_Fixed,
    pub Font_Revision: FT_Fixed,
    pub CheckSum_Adjust: FT_Long,
    pub Magic_Number: FT_Long,
    pub Flags: FT_UShort,
    pub Units_Per_EM: FT_UShort,
    pub Created: [FT_ULong; 2],
    pub Modified: [FT_ULong; 2],
    pub xMin: FT_Short,
    pub yMin: FT_Short,
    pub xMax: FT_Short,
    pub yMax: FT_Short,
    pub Mac_Style: FT_UShort,
    pub Lowest_Rec_PPEM: FT_UShort,
    pub Font_Direction: FT_Short,
    pub Index_To_Loc_Format: FT_Short,
    pub Glyph_Data_Format: FT_Short,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TT_HoriHeader {
    pub Version: FT_Fixed,
    pub Ascender: FT_Short,
    pub Descender: FT_Short,
    pub Line_Gap: FT_Short,
    pub advance_Width_Max: FT_UShort,
    pub min_Left_Side_Bearing: FT_Short,
    pub min_Right_Side_Bearing: FT_Short,
    pub xMax_Extent: FT_Short,
    pub caret_Slope_Rise: FT_Short,
    pub caret_Slope_Run: FT_Short,
    pub caret_Offset: FT_Short,
    pub Reserved: [FT_Short; 4],
    pub metric_Data_Format: FT_Short,
    pub number_Of_HMetrics: FT_UShort,
    pub long_metrics: *mut c_void,
    pub short_metrics: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TT_OS2 {
    pub version: FT_UShort,
    pub xAvgCharWidth: FT_Short,
    pub usWeightClass: FT_UShort,
    pub usWidthClass: FT_UShort,
    pub fsType: FT_UShort,
    pub ySubscriptXSize: FT_Short,
    pub ySubscriptYSize: FT_Short,
    pub ySubscriptXOffset: FT_Short,
    pub ySubscriptYOffset: FT_Short,
    pub ySuperscriptXSize: FT_Short,
    pub ySuperscriptYSize: FT_Short,
    pub ySuperscriptXOffset: FT_Short,
    pub ySuperscriptYOffset: FT_Short,
    pub yStrikeoutSize: FT_Short,
    pub yStrikeoutPosition: FT_Short,
    pub sFamilyClass: FT_Short,
    pub panose: [FT_Byte; 10],
    pub ulUnicodeRange1: FT_ULong,
    pub ulUnicodeRange2: FT_ULong,
    pub ulUnicodeRange3: FT_ULong,
    pub ulUnicodeRange4: FT_ULong,
    pub achVendID: [FT_Char; 4],
    pub fsSelection: FT_UShort,
    pub usFirstCharIndex: FT_UShort,
    pub usLastCharIndex: FT_UShort,
    pub sTypoAscender: FT_Short,
    pub sTypoDescender: FT_Short,
    pub sTypoLineGap: FT_Short,
    pub usWinAscent: FT_UShort,
    pub usWinDescent: FT_UShort,
    pub ulCodePageRange1: FT_ULong,
    pub ulCodePageRange2: FT_ULong,
    pub sxHeight: FT_Short,
    pub sCapHeight: FT_Short,
    pub usDefaultChar: FT_UShort,
    pub usBreakChar: FT_UShort,
    pub usMaxContext: FT_UShort,
    pub usLowerOpticalPointSize: FT_UShort,
    pub usUpperOpticalPointSize: FT_UShort,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TT_Postscript {
    pub FormatType: FT_Fixed,
    pub italicAngle: FT_Fixed,
    pub underlinePosition: FT_Short,
    pub underlineThickness: FT_Short,
    pub isFixedPitch: FT_ULong,
    pub minMemType42: FT_ULong,
    pub maxMemType42: FT_ULong,
    pub minMemType1: FT_ULong,
    pub maxMemType1: FT_ULong,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_Var_Axis {
    pub name: *mut FT_String,
    pub minimum: FT_Fixed,
    pub def: FT_Fixed,
    pub maximum: FT_Fixed,
    pub tag: FT_ULong,
    pub strid: FT_UInt,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_Var_Named_Style {
    pub coords: *mut FT_Fixed,
    pub strid: FT_UInt,
    pub psid: FT_UInt,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FT_MM_Var {
    pub num_axis: FT_UInt,
    pub num_designs: FT_UInt,
    pub num_namedstyles: FT_UInt,
    pub axis: *mut FT_Var_Axis,
    pub namedstyle: *mut FT_Var_Named_Style,
}

const fn image_tag(a: u8, b: u8, c: u8, d: u8) -> c_uint {
    ((a as c_uint) << 24) | ((b as c_uint) << 16) | ((c as c_uint) << 8) | d as c_uint
}

// FT_LOAD_* (freetype.h)
pub const FT_LOAD_DEFAULT: FT_Int32 = 0x0;
pub const FT_LOAD_NO_SCALE: FT_Int32 = 1 << 0;
pub const FT_LOAD_NO_HINTING: FT_Int32 = 1 << 1;
pub const FT_LOAD_VERTICAL_LAYOUT: FT_Int32 = 1 << 4;

// FT_Kerning_Mode
pub const FT_KERNING_DEFAULT: FT_UInt = 0;
pub const FT_KERNING_UNFITTED: FT_UInt = 1;
pub const FT_KERNING_UNSCALED: FT_UInt = 2;

// FT_Glyph_BBox_Mode
pub const FT_GLYPH_BBOX_UNSCALED: FT_UInt = 0;
pub const FT_GLYPH_BBOX_SUBPIXELS: FT_UInt = 0;
pub const FT_GLYPH_BBOX_GRIDFIT: FT_UInt = 1;
pub const FT_GLYPH_BBOX_TRUNCATE: FT_UInt = 2;
pub const FT_GLYPH_BBOX_PIXELS: FT_UInt = 3;

// FT_Glyph_Format
pub const FT_GLYPH_FORMAT_NONE: FT_Glyph_Format = 0;
pub const FT_GLYPH_FORMAT_COMPOSITE: FT_Glyph_Format = image_tag(b'c', b'o', b'm', b'p');
pub const FT_GLYPH_FORMAT_BITMAP: FT_Glyph_Format = image_tag(b'b', b'i', b't', b's');
pub const FT_GLYPH_FORMAT_OUTLINE: FT_Glyph_Format = image_tag(b'o', b'u', b't', b'l');
pub const FT_GLYPH_FORMAT_PLOTTER: FT_Glyph_Format = image_tag(b'p', b'l', b'o', b't');
pub const FT_GLYPH_FORMAT_SVG: FT_Glyph_Format = image_tag(b'S', b'V', b'G', b' ');

// FT_FACE_FLAG_* and FT_STYLE_FLAG_* (freetype.h)
pub const FT_FACE_FLAG_SCALABLE: FT_Long = 1 << 0;
pub const FT_FACE_FLAG_FIXED_SIZES: FT_Long = 1 << 1;
pub const FT_FACE_FLAG_FIXED_WIDTH: FT_Long = 1 << 2;
pub const FT_FACE_FLAG_SFNT: FT_Long = 1 << 3;
pub const FT_FACE_FLAG_HORIZONTAL: FT_Long = 1 << 4;
pub const FT_FACE_FLAG_VERTICAL: FT_Long = 1 << 5;
pub const FT_FACE_FLAG_KERNING: FT_Long = 1 << 6;
pub const FT_FACE_FLAG_MULTIPLE_MASTERS: FT_Long = 1 << 8;
pub const FT_FACE_FLAG_GLYPH_NAMES: FT_Long = 1 << 9;
pub const FT_FACE_FLAG_CID_KEYED: FT_Long = 1 << 12;
pub const FT_FACE_FLAG_VARIATION: FT_Long = 1 << 15;
pub const FT_STYLE_FLAG_ITALIC: FT_Long = 1 << 0;
pub const FT_STYLE_FLAG_BOLD: FT_Long = 1 << 1;

// FT_Sfnt_Tag (tttables.h) and the deprecated lower-case names XeTeX uses.
pub const FT_SFNT_HEAD: FT_Sfnt_Tag = 0;
pub const FT_SFNT_MAXP: FT_Sfnt_Tag = 1;
pub const FT_SFNT_OS2: FT_Sfnt_Tag = 2;
pub const FT_SFNT_HHEA: FT_Sfnt_Tag = 3;
pub const FT_SFNT_VHEA: FT_Sfnt_Tag = 4;
pub const FT_SFNT_POST: FT_Sfnt_Tag = 5;
pub const FT_SFNT_PCLT: FT_Sfnt_Tag = 6;
pub const ft_sfnt_head: FT_Sfnt_Tag = FT_SFNT_HEAD;
pub const ft_sfnt_os2: FT_Sfnt_Tag = FT_SFNT_OS2;
pub const ft_sfnt_post: FT_Sfnt_Tag = FT_SFNT_POST;

// ttnameid.h
pub const TT_PLATFORM_APPLE_UNICODE: FT_UShort = 0;
pub const TT_PLATFORM_MACINTOSH: FT_UShort = 1;
pub const TT_PLATFORM_MICROSOFT: FT_UShort = 3;
pub const TT_MAC_ID_ROMAN: FT_UShort = 0;
pub const TT_MS_ID_UNICODE_CS: FT_UShort = 1;

// fterrdef.h (the ones FlashTeX's loader distinguishes)
pub const FT_Err_Ok: FT_Error = 0x00;
pub const FT_Err_Cannot_Open_Resource: FT_Error = 0x01;
pub const FT_Err_Unknown_File_Format: FT_Error = 0x02;
pub const FT_Err_Invalid_Argument: FT_Error = 0x06;
pub const FT_Err_Table_Missing: FT_Error = 0x8E;

// freetype.h: the version these headers describe.
pub const FREETYPE_MAJOR: FT_Int = 2;
pub const FREETYPE_MINOR: FT_Int = 14;
pub const FREETYPE_PATCH: FT_Int = 1;

/// `FT_IS_SCALABLE` (a macro in freetype.h).
///
/// # Safety
/// `face` must be a live face from `FT_New_Face`/`FT_New_Memory_Face`.
pub unsafe fn FT_IS_SCALABLE(face: FT_Face) -> bool {
    (*face).face_flags & FT_FACE_FLAG_SCALABLE != 0
}

/// `FT_IS_SFNT` (a macro in freetype.h).
///
/// # Safety
/// As [`FT_IS_SCALABLE`].
pub unsafe fn FT_IS_SFNT(face: FT_Face) -> bool {
    (*face).face_flags & FT_FACE_FLAG_SFNT != 0
}

/// `FT_HAS_GLYPH_NAMES` (a macro in freetype.h).
///
/// # Safety
/// As [`FT_IS_SCALABLE`].
pub unsafe fn FT_HAS_GLYPH_NAMES(face: FT_Face) -> bool {
    (*face).face_flags & FT_FACE_FLAG_GLYPH_NAMES != 0
}

crate::ffi_functions! {
    pub fn FT_Init_FreeType(alibrary: *mut FT_Library) -> FT_Error;
    pub fn FT_Done_FreeType(library: FT_Library) -> FT_Error;
    pub fn FT_Library_Version(library: FT_Library, amajor: *mut FT_Int, aminor: *mut FT_Int, apatch: *mut FT_Int);
    pub fn FT_New_Face(library: FT_Library, filepathname: *const c_char, face_index: FT_Long, aface: *mut FT_Face) -> FT_Error;
    pub fn FT_New_Memory_Face(library: FT_Library, file_base: *const FT_Byte, file_size: FT_Long, face_index: FT_Long, aface: *mut FT_Face) -> FT_Error;
    pub fn FT_Done_Face(face: FT_Face) -> FT_Error;
    pub fn FT_Attach_File(face: FT_Face, filepathname: *const c_char) -> FT_Error;
    pub fn FT_Load_Glyph(face: FT_Face, glyph_index: FT_UInt, load_flags: FT_Int32) -> FT_Error;
    pub fn FT_Get_Char_Index(face: FT_Face, charcode: FT_ULong) -> FT_UInt;
    pub fn FT_Face_GetCharVariantIndex(face: FT_Face, charcode: FT_ULong, variantSelector: FT_ULong) -> FT_UInt;
    pub fn FT_Get_First_Char(face: FT_Face, agindex: *mut FT_UInt) -> FT_ULong;
    pub fn FT_Get_Next_Char(face: FT_Face, char_code: FT_ULong, agindex: *mut FT_UInt) -> FT_ULong;
    pub fn FT_Get_Glyph_Name(face: FT_Face, glyph_index: FT_UInt, buffer: FT_Pointer, buffer_max: FT_UInt) -> FT_Error;
    pub fn FT_Get_Name_Index(face: FT_Face, glyph_name: *const FT_String) -> FT_UInt;
    pub fn FT_Get_Postscript_Name(face: FT_Face) -> *const c_char;
    pub fn FT_Get_Kerning(face: FT_Face, left_glyph: FT_UInt, right_glyph: FT_UInt, kern_mode: FT_UInt, akerning: *mut FT_Vector) -> FT_Error;
    pub fn FT_Get_Advance(face: FT_Face, gindex: FT_UInt, load_flags: FT_Int32, padvance: *mut FT_Fixed) -> FT_Error;
    pub fn FT_Get_Glyph(slot: FT_GlyphSlot, aglyph: *mut FT_Glyph) -> FT_Error;
    pub fn FT_Done_Glyph(glyph: FT_Glyph);
    pub fn FT_Glyph_Get_CBox(glyph: FT_Glyph, bbox_mode: FT_UInt, acbox: *mut FT_BBox);
    pub fn FT_Get_Sfnt_Table(face: FT_Face, tag: FT_Sfnt_Tag) -> *mut c_void;
    pub fn FT_Load_Sfnt_Table(face: FT_Face, tag: FT_ULong, offset: FT_Long, buffer: *mut FT_Byte, length: *mut FT_ULong) -> FT_Error;
    pub fn FT_Get_Sfnt_Name_Count(face: FT_Face) -> FT_UInt;
    pub fn FT_Get_Sfnt_Name(face: FT_Face, idx: FT_UInt, aname: *mut FT_SfntName) -> FT_Error;
    pub fn FT_Get_MM_Var(face: FT_Face, amaster: *mut *mut FT_MM_Var) -> FT_Error;
    pub fn FT_Done_MM_Var(library: FT_Library, amaster: *mut FT_MM_Var) -> FT_Error;
    pub fn FT_Set_Var_Design_Coordinates(face: FT_Face, num_coords: FT_UInt, coords: *mut FT_Fixed) -> FT_Error;
    pub fn FT_Get_Var_Design_Coordinates(face: FT_Face, num_coords: FT_UInt, coords: *mut FT_Fixed) -> FT_Error;
}
