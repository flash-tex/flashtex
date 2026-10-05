/* Layout probe for the FFI of src/ft.rs and src/hb.rs (FlashTeX, GPL-2.0-or-later).
 *
 * Compiled by build.rs with the same headers and defines as the vendored
 * FreeType and HarfBuzz. It reports sizeof, alignof and offsetof of every
 * struct the Rust FFI declares, and the value of every enum constant it
 * copies, so that crates/flashtex-xetex/tests/fontlibs_layout.rs can compare
 * them with the Rust definitions: a wrong layout is silent memory corruption.
 */
#include <stddef.h>
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_TRUETYPE_TABLES_H
#include FT_TRUETYPE_IDS_H
#include FT_GLYPH_H
#include FT_ADVANCES_H
#include FT_SFNT_NAMES_H
#include FT_MULTIPLE_MASTERS_H
#include <hb.h>
#include <hb-ot.h>

typedef struct {
  const char *name;
  long long value;
} flashtex_fontlibs_probe_entry;

#define SIZE(T) {"sizeof " #T, (long long)sizeof(T)}, {"alignof " #T, (long long)_Alignof(T)}
#define OFF(T, f) {#T "." #f, (long long)offsetof(T, f)}
#define VAL(c) {#c, (long long)(c)}

static const flashtex_fontlibs_probe_entry entries[] = {
  /* FreeType scalar types */
  SIZE(FT_Long), SIZE(FT_ULong), SIZE(FT_Int32), SIZE(FT_Pos), SIZE(FT_Fixed),
  SIZE(FT_Error), SIZE(FT_UInt), SIZE(FT_Int), SIZE(FT_Short), SIZE(FT_UShort),
  SIZE(FT_Byte), SIZE(FT_Char), SIZE(FT_Bool), SIZE(FT_Tag), SIZE(FT_Encoding),
  SIZE(FT_Glyph_Format), SIZE(FT_Sfnt_Tag),

  SIZE(FT_Vector), OFF(FT_Vector, x), OFF(FT_Vector, y),
  SIZE(FT_BBox), OFF(FT_BBox, xMin), OFF(FT_BBox, yMin), OFF(FT_BBox, xMax), OFF(FT_BBox, yMax),
  SIZE(FT_Generic), OFF(FT_Generic, data), OFF(FT_Generic, finalizer),
  SIZE(FT_ListRec), OFF(FT_ListRec, head), OFF(FT_ListRec, tail),

  SIZE(FT_Bitmap_Size), OFF(FT_Bitmap_Size, height), OFF(FT_Bitmap_Size, width),
  OFF(FT_Bitmap_Size, size), OFF(FT_Bitmap_Size, x_ppem), OFF(FT_Bitmap_Size, y_ppem),

  SIZE(FT_CharMapRec), OFF(FT_CharMapRec, face), OFF(FT_CharMapRec, encoding),
  OFF(FT_CharMapRec, platform_id), OFF(FT_CharMapRec, encoding_id),

  SIZE(FT_FaceRec),
  OFF(FT_FaceRec, num_faces), OFF(FT_FaceRec, face_index), OFF(FT_FaceRec, face_flags),
  OFF(FT_FaceRec, style_flags), OFF(FT_FaceRec, num_glyphs), OFF(FT_FaceRec, family_name),
  OFF(FT_FaceRec, style_name), OFF(FT_FaceRec, num_fixed_sizes), OFF(FT_FaceRec, available_sizes),
  OFF(FT_FaceRec, num_charmaps), OFF(FT_FaceRec, charmaps), OFF(FT_FaceRec, generic),
  OFF(FT_FaceRec, bbox), OFF(FT_FaceRec, units_per_EM), OFF(FT_FaceRec, ascender),
  OFF(FT_FaceRec, descender), OFF(FT_FaceRec, height), OFF(FT_FaceRec, max_advance_width),
  OFF(FT_FaceRec, max_advance_height), OFF(FT_FaceRec, underline_position),
  OFF(FT_FaceRec, underline_thickness), OFF(FT_FaceRec, glyph), OFF(FT_FaceRec, size),
  OFF(FT_FaceRec, charmap), OFF(FT_FaceRec, driver), OFF(FT_FaceRec, memory),
  OFF(FT_FaceRec, stream), OFF(FT_FaceRec, sizes_list), OFF(FT_FaceRec, autohint),
  OFF(FT_FaceRec, extensions), OFF(FT_FaceRec, internal),

  SIZE(FT_Glyph_Metrics),
  OFF(FT_Glyph_Metrics, width), OFF(FT_Glyph_Metrics, height),
  OFF(FT_Glyph_Metrics, horiBearingX), OFF(FT_Glyph_Metrics, horiBearingY),
  OFF(FT_Glyph_Metrics, horiAdvance), OFF(FT_Glyph_Metrics, vertBearingX),
  OFF(FT_Glyph_Metrics, vertBearingY), OFF(FT_Glyph_Metrics, vertAdvance),

  SIZE(FT_Bitmap),
  OFF(FT_Bitmap, rows), OFF(FT_Bitmap, width), OFF(FT_Bitmap, pitch), OFF(FT_Bitmap, buffer),
  OFF(FT_Bitmap, num_grays), OFF(FT_Bitmap, pixel_mode), OFF(FT_Bitmap, palette_mode),
  OFF(FT_Bitmap, palette),

  SIZE(FT_Outline),
  OFF(FT_Outline, n_contours), OFF(FT_Outline, n_points), OFF(FT_Outline, points),
  OFF(FT_Outline, tags), OFF(FT_Outline, contours), OFF(FT_Outline, flags),

  SIZE(FT_GlyphSlotRec),
  OFF(FT_GlyphSlotRec, library), OFF(FT_GlyphSlotRec, face), OFF(FT_GlyphSlotRec, next),
  OFF(FT_GlyphSlotRec, glyph_index), OFF(FT_GlyphSlotRec, generic), OFF(FT_GlyphSlotRec, metrics),
  OFF(FT_GlyphSlotRec, linearHoriAdvance), OFF(FT_GlyphSlotRec, linearVertAdvance),
  OFF(FT_GlyphSlotRec, advance), OFF(FT_GlyphSlotRec, format), OFF(FT_GlyphSlotRec, bitmap),
  OFF(FT_GlyphSlotRec, bitmap_left), OFF(FT_GlyphSlotRec, bitmap_top), OFF(FT_GlyphSlotRec, outline),
  OFF(FT_GlyphSlotRec, num_subglyphs), OFF(FT_GlyphSlotRec, subglyphs),
  OFF(FT_GlyphSlotRec, control_data), OFF(FT_GlyphSlotRec, control_len),
  OFF(FT_GlyphSlotRec, lsb_delta), OFF(FT_GlyphSlotRec, rsb_delta), OFF(FT_GlyphSlotRec, other),
  OFF(FT_GlyphSlotRec, internal),

  SIZE(FT_GlyphRec), OFF(FT_GlyphRec, library), OFF(FT_GlyphRec, clazz),
  OFF(FT_GlyphRec, format), OFF(FT_GlyphRec, advance),

  SIZE(FT_SfntName), OFF(FT_SfntName, platform_id), OFF(FT_SfntName, encoding_id),
  OFF(FT_SfntName, language_id), OFF(FT_SfntName, name_id), OFF(FT_SfntName, string),
  OFF(FT_SfntName, string_len),

  SIZE(TT_Header),
  OFF(TT_Header, Table_Version), OFF(TT_Header, Font_Revision), OFF(TT_Header, CheckSum_Adjust),
  OFF(TT_Header, Magic_Number), OFF(TT_Header, Flags), OFF(TT_Header, Units_Per_EM),
  OFF(TT_Header, Created), OFF(TT_Header, Modified), OFF(TT_Header, xMin), OFF(TT_Header, yMin),
  OFF(TT_Header, xMax), OFF(TT_Header, yMax), OFF(TT_Header, Mac_Style),
  OFF(TT_Header, Lowest_Rec_PPEM), OFF(TT_Header, Font_Direction),
  OFF(TT_Header, Index_To_Loc_Format), OFF(TT_Header, Glyph_Data_Format),

  SIZE(TT_HoriHeader),
  OFF(TT_HoriHeader, Version), OFF(TT_HoriHeader, Ascender), OFF(TT_HoriHeader, Descender),
  OFF(TT_HoriHeader, Line_Gap), OFF(TT_HoriHeader, advance_Width_Max),
  OFF(TT_HoriHeader, min_Left_Side_Bearing), OFF(TT_HoriHeader, min_Right_Side_Bearing),
  OFF(TT_HoriHeader, xMax_Extent), OFF(TT_HoriHeader, caret_Slope_Rise),
  OFF(TT_HoriHeader, caret_Slope_Run), OFF(TT_HoriHeader, caret_Offset),
  OFF(TT_HoriHeader, Reserved), OFF(TT_HoriHeader, metric_Data_Format),
  OFF(TT_HoriHeader, number_Of_HMetrics), OFF(TT_HoriHeader, long_metrics),
  OFF(TT_HoriHeader, short_metrics),

  SIZE(TT_OS2),
  OFF(TT_OS2, version), OFF(TT_OS2, xAvgCharWidth), OFF(TT_OS2, usWeightClass),
  OFF(TT_OS2, usWidthClass), OFF(TT_OS2, fsType), OFF(TT_OS2, ySubscriptXSize),
  OFF(TT_OS2, ySubscriptYSize), OFF(TT_OS2, ySubscriptXOffset), OFF(TT_OS2, ySubscriptYOffset),
  OFF(TT_OS2, ySuperscriptXSize), OFF(TT_OS2, ySuperscriptYSize),
  OFF(TT_OS2, ySuperscriptXOffset), OFF(TT_OS2, ySuperscriptYOffset),
  OFF(TT_OS2, yStrikeoutSize), OFF(TT_OS2, yStrikeoutPosition), OFF(TT_OS2, sFamilyClass),
  OFF(TT_OS2, panose), OFF(TT_OS2, ulUnicodeRange1), OFF(TT_OS2, ulUnicodeRange2),
  OFF(TT_OS2, ulUnicodeRange3), OFF(TT_OS2, ulUnicodeRange4), OFF(TT_OS2, achVendID),
  OFF(TT_OS2, fsSelection), OFF(TT_OS2, usFirstCharIndex), OFF(TT_OS2, usLastCharIndex),
  OFF(TT_OS2, sTypoAscender), OFF(TT_OS2, sTypoDescender), OFF(TT_OS2, sTypoLineGap),
  OFF(TT_OS2, usWinAscent), OFF(TT_OS2, usWinDescent), OFF(TT_OS2, ulCodePageRange1),
  OFF(TT_OS2, ulCodePageRange2), OFF(TT_OS2, sxHeight), OFF(TT_OS2, sCapHeight),
  OFF(TT_OS2, usDefaultChar), OFF(TT_OS2, usBreakChar), OFF(TT_OS2, usMaxContext),
  OFF(TT_OS2, usLowerOpticalPointSize), OFF(TT_OS2, usUpperOpticalPointSize),

  SIZE(TT_Postscript),
  OFF(TT_Postscript, FormatType), OFF(TT_Postscript, italicAngle),
  OFF(TT_Postscript, underlinePosition), OFF(TT_Postscript, underlineThickness),
  OFF(TT_Postscript, isFixedPitch), OFF(TT_Postscript, minMemType42),
  OFF(TT_Postscript, maxMemType42), OFF(TT_Postscript, minMemType1),
  OFF(TT_Postscript, maxMemType1),

  SIZE(FT_Var_Axis), OFF(FT_Var_Axis, name), OFF(FT_Var_Axis, minimum), OFF(FT_Var_Axis, def),
  OFF(FT_Var_Axis, maximum), OFF(FT_Var_Axis, tag), OFF(FT_Var_Axis, strid),
  SIZE(FT_Var_Named_Style), OFF(FT_Var_Named_Style, coords), OFF(FT_Var_Named_Style, strid),
  OFF(FT_Var_Named_Style, psid),
  SIZE(FT_MM_Var), OFF(FT_MM_Var, num_axis), OFF(FT_MM_Var, num_designs),
  OFF(FT_MM_Var, num_namedstyles), OFF(FT_MM_Var, axis), OFF(FT_MM_Var, namedstyle),

  /* FreeType constants */
  VAL(FT_LOAD_DEFAULT), VAL(FT_LOAD_NO_SCALE), VAL(FT_LOAD_NO_HINTING), VAL(FT_LOAD_VERTICAL_LAYOUT),
  VAL(FT_KERNING_DEFAULT), VAL(FT_KERNING_UNFITTED), VAL(FT_KERNING_UNSCALED),
  VAL(FT_GLYPH_BBOX_UNSCALED), VAL(FT_GLYPH_BBOX_GRIDFIT), VAL(FT_GLYPH_BBOX_TRUNCATE),
  VAL(FT_GLYPH_BBOX_PIXELS),
  VAL(FT_GLYPH_FORMAT_NONE), VAL(FT_GLYPH_FORMAT_COMPOSITE), VAL(FT_GLYPH_FORMAT_BITMAP),
  VAL(FT_GLYPH_FORMAT_OUTLINE), VAL(FT_GLYPH_FORMAT_PLOTTER), VAL(FT_GLYPH_FORMAT_SVG),
  VAL(FT_FACE_FLAG_SCALABLE), VAL(FT_FACE_FLAG_FIXED_SIZES), VAL(FT_FACE_FLAG_FIXED_WIDTH),
  VAL(FT_FACE_FLAG_SFNT), VAL(FT_FACE_FLAG_HORIZONTAL), VAL(FT_FACE_FLAG_VERTICAL),
  VAL(FT_FACE_FLAG_KERNING), VAL(FT_FACE_FLAG_MULTIPLE_MASTERS), VAL(FT_FACE_FLAG_GLYPH_NAMES),
  VAL(FT_FACE_FLAG_CID_KEYED), VAL(FT_FACE_FLAG_VARIATION),
  VAL(FT_STYLE_FLAG_ITALIC), VAL(FT_STYLE_FLAG_BOLD),
  VAL(FT_SFNT_HEAD), VAL(FT_SFNT_MAXP), VAL(FT_SFNT_OS2), VAL(FT_SFNT_HHEA), VAL(FT_SFNT_VHEA),
  VAL(FT_SFNT_POST), VAL(FT_SFNT_PCLT),
  VAL(ft_sfnt_head), VAL(ft_sfnt_os2), VAL(ft_sfnt_post),
  VAL(TT_PLATFORM_APPLE_UNICODE), VAL(TT_PLATFORM_MACINTOSH), VAL(TT_PLATFORM_MICROSOFT),
  VAL(TT_MAC_ID_ROMAN), VAL(TT_MS_ID_UNICODE_CS),
  VAL(FT_Err_Ok), VAL(FT_Err_Cannot_Open_Resource), VAL(FT_Err_Unknown_File_Format),
  VAL(FT_Err_Invalid_Argument), VAL(FT_Err_Table_Missing),
  VAL(FREETYPE_MAJOR), VAL(FREETYPE_MINOR), VAL(FREETYPE_PATCH),

  /* HarfBuzz */
  SIZE(hb_codepoint_t), SIZE(hb_position_t), SIZE(hb_mask_t), SIZE(hb_tag_t), SIZE(hb_bool_t),
  SIZE(hb_direction_t), SIZE(hb_script_t), SIZE(hb_language_t), SIZE(hb_memory_mode_t),
  SIZE(hb_buffer_content_type_t), SIZE(hb_ot_math_constant_t), SIZE(hb_ot_math_kern_t),
  SIZE(hb_ot_math_glyph_part_flags_t), SIZE(hb_buffer_serialize_format_t),
  SIZE(hb_buffer_serialize_flags_t), SIZE(hb_ot_name_id_t),
  SIZE(hb_var_int_t),
  SIZE(hb_glyph_info_t), OFF(hb_glyph_info_t, codepoint), OFF(hb_glyph_info_t, mask),
  OFF(hb_glyph_info_t, cluster), OFF(hb_glyph_info_t, var1), OFF(hb_glyph_info_t, var2),
  SIZE(hb_glyph_position_t), OFF(hb_glyph_position_t, x_advance),
  OFF(hb_glyph_position_t, y_advance), OFF(hb_glyph_position_t, x_offset),
  OFF(hb_glyph_position_t, y_offset), OFF(hb_glyph_position_t, var),
  SIZE(hb_feature_t), OFF(hb_feature_t, tag), OFF(hb_feature_t, value),
  OFF(hb_feature_t, start), OFF(hb_feature_t, end),
  SIZE(hb_segment_properties_t), OFF(hb_segment_properties_t, direction),
  OFF(hb_segment_properties_t, script), OFF(hb_segment_properties_t, language),
  OFF(hb_segment_properties_t, reserved1), OFF(hb_segment_properties_t, reserved2),
  SIZE(hb_glyph_extents_t), OFF(hb_glyph_extents_t, x_bearing),
  OFF(hb_glyph_extents_t, y_bearing), OFF(hb_glyph_extents_t, width),
  OFF(hb_glyph_extents_t, height),
  SIZE(hb_ot_math_glyph_variant_t), OFF(hb_ot_math_glyph_variant_t, glyph),
  OFF(hb_ot_math_glyph_variant_t, advance),
  SIZE(hb_ot_math_glyph_part_t), OFF(hb_ot_math_glyph_part_t, glyph),
  OFF(hb_ot_math_glyph_part_t, start_connector_length),
  OFF(hb_ot_math_glyph_part_t, end_connector_length),
  OFF(hb_ot_math_glyph_part_t, full_advance), OFF(hb_ot_math_glyph_part_t, flags),

  VAL(HB_DIRECTION_INVALID), VAL(HB_DIRECTION_LTR), VAL(HB_DIRECTION_RTL),
  VAL(HB_DIRECTION_TTB), VAL(HB_DIRECTION_BTT),
  VAL(HB_MEMORY_MODE_DUPLICATE), VAL(HB_MEMORY_MODE_READONLY), VAL(HB_MEMORY_MODE_WRITABLE),
  VAL(HB_MEMORY_MODE_READONLY_MAY_MAKE_WRITABLE),
  VAL(HB_BUFFER_CONTENT_TYPE_INVALID), VAL(HB_BUFFER_CONTENT_TYPE_UNICODE),
  VAL(HB_BUFFER_CONTENT_TYPE_GLYPHS),
  VAL(HB_BUFFER_SERIALIZE_FORMAT_TEXT), VAL(HB_BUFFER_SERIALIZE_FORMAT_JSON),
  VAL(HB_BUFFER_SERIALIZE_FORMAT_INVALID), VAL(HB_BUFFER_SERIALIZE_FLAG_DEFAULT),
  VAL(HB_BUFFER_SERIALIZE_FLAGS_DEFAULT),
  VAL(HB_SCRIPT_INVALID), VAL(HB_SCRIPT_COMMON), VAL(HB_SCRIPT_LATIN), VAL(HB_TAG_NONE),
  VAL(HB_OT_TAG_GSUB), VAL(HB_OT_TAG_GPOS), VAL(HB_OT_TAG_MATH),
  VAL(HB_OT_TAG_DEFAULT_SCRIPT), VAL(HB_OT_TAG_DEFAULT_LANGUAGE),
  VAL(HB_OT_LAYOUT_NO_SCRIPT_INDEX), VAL(HB_OT_LAYOUT_DEFAULT_LANGUAGE_INDEX),
  VAL(HB_OT_MATH_KERN_TOP_RIGHT), VAL(HB_OT_MATH_KERN_TOP_LEFT),
  VAL(HB_OT_MATH_KERN_BOTTOM_RIGHT), VAL(HB_OT_MATH_KERN_BOTTOM_LEFT),
  VAL(HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER),
  VAL(HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN),
  VAL(HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT),
  VAL(HB_VERSION_MAJOR), VAL(HB_VERSION_MINOR), VAL(HB_VERSION_MICRO),
};

const flashtex_fontlibs_probe_entry *
flashtex_fontlibs_layout_probe (size_t *count)
{
  *count = sizeof entries / sizeof entries[0];
  return entries;
}
