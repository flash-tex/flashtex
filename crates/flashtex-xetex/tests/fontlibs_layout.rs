//! The `#[repr(C)]` structs and copied constants of flashtex-xetex-fontlibs
//! against what the C compiler says, from the vendored headers
//! (fontlibs/csrc/layout_probe.c). A wrong field offset in an FFI struct is
//! silent memory corruption; this test makes it loud.

use std::collections::BTreeMap;
use std::mem::{align_of, offset_of, size_of};

use flashtex_xetex_fontlibs::{ft, hb};

fn probe() -> BTreeMap<String, i64> {
    flashtex_xetex_fontlibs::layout_probe()
        .into_iter()
        .collect()
}

macro_rules! check_struct {
    ($c:expr, $errs:expr, $m:ident :: $t:ident { $($f:ident),* $(,)? }) => {{
        let n = stringify!($t);
        $crate::cmp($c, $errs, &format!("sizeof {n}"), size_of::<$m::$t>() as i64);
        $crate::cmp($c, $errs, &format!("alignof {n}"), align_of::<$m::$t>() as i64);
        $( $crate::cmp($c, $errs, &format!("{n}.{}", stringify!($f)), offset_of!($m::$t, $f) as i64); )*
    }};
}

macro_rules! check_size {
    ($c:expr, $errs:expr, $m:ident :: $($t:ident),*) => {{
        $(
            $crate::cmp($c, $errs, &format!("sizeof {}", stringify!($t)), size_of::<$m::$t>() as i64);
            $crate::cmp($c, $errs, &format!("alignof {}", stringify!($t)), align_of::<$m::$t>() as i64);
        )*
    }};
}

macro_rules! check_values {
    ($c:expr, $errs:expr, $m:ident :: $($k:ident),* $(,)?) => {{
        $( $crate::cmp($c, $errs, stringify!($k), $m::$k as i64); )*
    }};
}

fn cmp(c: &mut BTreeMap<String, i64>, errs: &mut Vec<String>, key: &str, rust: i64) {
    match c.remove(key) {
        Some(v) if v == rust => {}
        Some(v) => errs.push(format!("{key}: C {v}, Rust {rust}")),
        None => errs.push(format!("{key}: not in the probe")),
    }
}

#[test]
fn ffi_layouts_match_the_c_headers() {
    let mut c = probe();
    let mut errs = Vec::new();
    let e = &mut errs;

    check_size!(
        &mut c,
        e,
        ft::FT_Long,
        FT_ULong,
        FT_Int32,
        FT_Pos,
        FT_Fixed,
        FT_Error,
        FT_UInt,
        FT_Int,
        FT_Short,
        FT_UShort,
        FT_Byte,
        FT_Char,
        FT_Bool,
        FT_Tag,
        FT_Encoding,
        FT_Glyph_Format,
        FT_Sfnt_Tag
    );
    check_struct!(&mut c, e, ft::FT_Vector { x, y });
    check_struct!(
        &mut c,
        e,
        ft::FT_BBox {
            xMin,
            yMin,
            xMax,
            yMax
        }
    );
    check_struct!(&mut c, e, ft::FT_Generic { data, finalizer });
    check_struct!(&mut c, e, ft::FT_ListRec { head, tail });
    check_struct!(
        &mut c,
        e,
        ft::FT_Bitmap_Size {
            height,
            width,
            size,
            x_ppem,
            y_ppem
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_CharMapRec {
            face,
            encoding,
            platform_id,
            encoding_id
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_FaceRec {
            num_faces,
            face_index,
            face_flags,
            style_flags,
            num_glyphs,
            family_name,
            style_name,
            num_fixed_sizes,
            available_sizes,
            num_charmaps,
            charmaps,
            generic,
            bbox,
            units_per_EM,
            ascender,
            descender,
            height,
            max_advance_width,
            max_advance_height,
            underline_position,
            underline_thickness,
            glyph,
            size,
            charmap,
            driver,
            memory,
            stream,
            sizes_list,
            autohint,
            extensions,
            internal
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_Glyph_Metrics {
            width,
            height,
            horiBearingX,
            horiBearingY,
            horiAdvance,
            vertBearingX,
            vertBearingY,
            vertAdvance
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_Bitmap {
            rows,
            width,
            pitch,
            buffer,
            num_grays,
            pixel_mode,
            palette_mode,
            palette
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_Outline {
            n_contours,
            n_points,
            points,
            tags,
            contours,
            flags
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_GlyphSlotRec {
            library,
            face,
            next,
            glyph_index,
            generic,
            metrics,
            linearHoriAdvance,
            linearVertAdvance,
            advance,
            format,
            bitmap,
            bitmap_left,
            bitmap_top,
            outline,
            num_subglyphs,
            subglyphs,
            control_data,
            control_len,
            lsb_delta,
            rsb_delta,
            other,
            internal
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_GlyphRec {
            library,
            clazz,
            format,
            advance
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_SfntName {
            platform_id,
            encoding_id,
            language_id,
            name_id,
            string,
            string_len
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::TT_Header {
            Table_Version,
            Font_Revision,
            CheckSum_Adjust,
            Magic_Number,
            Flags,
            Units_Per_EM,
            Created,
            Modified,
            xMin,
            yMin,
            xMax,
            yMax,
            Mac_Style,
            Lowest_Rec_PPEM,
            Font_Direction,
            Index_To_Loc_Format,
            Glyph_Data_Format
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::TT_HoriHeader {
            Version,
            Ascender,
            Descender,
            Line_Gap,
            advance_Width_Max,
            min_Left_Side_Bearing,
            min_Right_Side_Bearing,
            xMax_Extent,
            caret_Slope_Rise,
            caret_Slope_Run,
            caret_Offset,
            Reserved,
            metric_Data_Format,
            number_Of_HMetrics,
            long_metrics,
            short_metrics
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::TT_OS2 {
            version,
            xAvgCharWidth,
            usWeightClass,
            usWidthClass,
            fsType,
            ySubscriptXSize,
            ySubscriptYSize,
            ySubscriptXOffset,
            ySubscriptYOffset,
            ySuperscriptXSize,
            ySuperscriptYSize,
            ySuperscriptXOffset,
            ySuperscriptYOffset,
            yStrikeoutSize,
            yStrikeoutPosition,
            sFamilyClass,
            panose,
            ulUnicodeRange1,
            ulUnicodeRange2,
            ulUnicodeRange3,
            ulUnicodeRange4,
            achVendID,
            fsSelection,
            usFirstCharIndex,
            usLastCharIndex,
            sTypoAscender,
            sTypoDescender,
            sTypoLineGap,
            usWinAscent,
            usWinDescent,
            ulCodePageRange1,
            ulCodePageRange2,
            sxHeight,
            sCapHeight,
            usDefaultChar,
            usBreakChar,
            usMaxContext,
            usLowerOpticalPointSize,
            usUpperOpticalPointSize
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::TT_Postscript {
            FormatType,
            italicAngle,
            underlinePosition,
            underlineThickness,
            isFixedPitch,
            minMemType42,
            maxMemType42,
            minMemType1,
            maxMemType1
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_Var_Axis {
            name,
            minimum,
            def,
            maximum,
            tag,
            strid
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_Var_Named_Style {
            coords,
            strid,
            psid
        }
    );
    check_struct!(
        &mut c,
        e,
        ft::FT_MM_Var {
            num_axis,
            num_designs,
            num_namedstyles,
            axis,
            namedstyle
        }
    );

    check_values!(
        &mut c,
        e,
        ft::FT_LOAD_DEFAULT,
        FT_LOAD_NO_SCALE,
        FT_LOAD_NO_HINTING,
        FT_LOAD_VERTICAL_LAYOUT,
        FT_KERNING_DEFAULT,
        FT_KERNING_UNFITTED,
        FT_KERNING_UNSCALED,
        FT_GLYPH_BBOX_UNSCALED,
        FT_GLYPH_BBOX_GRIDFIT,
        FT_GLYPH_BBOX_TRUNCATE,
        FT_GLYPH_BBOX_PIXELS,
        FT_GLYPH_FORMAT_NONE,
        FT_GLYPH_FORMAT_COMPOSITE,
        FT_GLYPH_FORMAT_BITMAP,
        FT_GLYPH_FORMAT_OUTLINE,
        FT_GLYPH_FORMAT_PLOTTER,
        FT_GLYPH_FORMAT_SVG,
        FT_FACE_FLAG_SCALABLE,
        FT_FACE_FLAG_FIXED_SIZES,
        FT_FACE_FLAG_FIXED_WIDTH,
        FT_FACE_FLAG_SFNT,
        FT_FACE_FLAG_HORIZONTAL,
        FT_FACE_FLAG_VERTICAL,
        FT_FACE_FLAG_KERNING,
        FT_FACE_FLAG_MULTIPLE_MASTERS,
        FT_FACE_FLAG_GLYPH_NAMES,
        FT_FACE_FLAG_CID_KEYED,
        FT_FACE_FLAG_VARIATION,
        FT_STYLE_FLAG_ITALIC,
        FT_STYLE_FLAG_BOLD,
        FT_SFNT_HEAD,
        FT_SFNT_MAXP,
        FT_SFNT_OS2,
        FT_SFNT_HHEA,
        FT_SFNT_VHEA,
        FT_SFNT_POST,
        FT_SFNT_PCLT,
        ft_sfnt_head,
        ft_sfnt_os2,
        ft_sfnt_post,
        TT_PLATFORM_APPLE_UNICODE,
        TT_PLATFORM_MACINTOSH,
        TT_PLATFORM_MICROSOFT,
        TT_MAC_ID_ROMAN,
        TT_MS_ID_UNICODE_CS,
        FT_Err_Ok,
        FT_Err_Cannot_Open_Resource,
        FT_Err_Unknown_File_Format,
        FT_Err_Invalid_Argument,
        FT_Err_Table_Missing,
        FREETYPE_MAJOR,
        FREETYPE_MINOR,
        FREETYPE_PATCH
    );

    check_size!(
        &mut c,
        e,
        hb::hb_codepoint_t,
        hb_position_t,
        hb_mask_t,
        hb_tag_t,
        hb_bool_t,
        hb_direction_t,
        hb_script_t,
        hb_language_t,
        hb_memory_mode_t,
        hb_buffer_content_type_t,
        hb_ot_math_constant_t,
        hb_ot_math_kern_t,
        hb_ot_math_glyph_part_flags_t,
        hb_buffer_serialize_format_t,
        hb_buffer_serialize_flags_t,
        hb_ot_name_id_t,
        hb_var_int_t
    );
    check_struct!(
        &mut c,
        e,
        hb::hb_glyph_info_t {
            codepoint,
            mask,
            cluster,
            var1,
            var2
        }
    );
    check_struct!(
        &mut c,
        e,
        hb::hb_glyph_position_t {
            x_advance,
            y_advance,
            x_offset,
            y_offset,
            var
        }
    );
    check_struct!(
        &mut c,
        e,
        hb::hb_feature_t {
            tag,
            value,
            start,
            end
        }
    );
    check_struct!(
        &mut c,
        e,
        hb::hb_segment_properties_t {
            direction,
            script,
            language,
            reserved1,
            reserved2
        }
    );
    check_struct!(
        &mut c,
        e,
        hb::hb_glyph_extents_t {
            x_bearing,
            y_bearing,
            width,
            height
        }
    );
    check_struct!(&mut c, e, hb::hb_ot_math_glyph_variant_t { glyph, advance });
    check_struct!(
        &mut c,
        e,
        hb::hb_ot_math_glyph_part_t {
            glyph,
            start_connector_length,
            end_connector_length,
            full_advance,
            flags
        }
    );

    check_values!(
        &mut c,
        e,
        hb::HB_DIRECTION_INVALID,
        HB_DIRECTION_LTR,
        HB_DIRECTION_RTL,
        HB_DIRECTION_TTB,
        HB_DIRECTION_BTT,
        HB_MEMORY_MODE_DUPLICATE,
        HB_MEMORY_MODE_READONLY,
        HB_MEMORY_MODE_WRITABLE,
        HB_MEMORY_MODE_READONLY_MAY_MAKE_WRITABLE,
        HB_BUFFER_CONTENT_TYPE_INVALID,
        HB_BUFFER_CONTENT_TYPE_UNICODE,
        HB_BUFFER_CONTENT_TYPE_GLYPHS,
        HB_BUFFER_SERIALIZE_FORMAT_TEXT,
        HB_BUFFER_SERIALIZE_FORMAT_JSON,
        HB_BUFFER_SERIALIZE_FORMAT_INVALID,
        HB_BUFFER_SERIALIZE_FLAG_DEFAULT,
        HB_BUFFER_SERIALIZE_FLAGS_DEFAULT,
        HB_SCRIPT_INVALID,
        HB_SCRIPT_COMMON,
        HB_SCRIPT_LATIN,
        HB_TAG_NONE,
        HB_OT_TAG_GSUB,
        HB_OT_TAG_GPOS,
        HB_OT_TAG_MATH,
        HB_OT_TAG_DEFAULT_SCRIPT,
        HB_OT_TAG_DEFAULT_LANGUAGE,
        HB_OT_LAYOUT_NO_SCRIPT_INDEX,
        HB_OT_LAYOUT_DEFAULT_LANGUAGE_INDEX,
        HB_OT_MATH_KERN_TOP_RIGHT,
        HB_OT_MATH_KERN_TOP_LEFT,
        HB_OT_MATH_KERN_BOTTOM_RIGHT,
        HB_OT_MATH_KERN_BOTTOM_LEFT,
        HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER,
        HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN,
        HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT
    );

    // The compile-time version macros of the headers the probe was built with.
    cmp(&mut c, e, "HB_VERSION_MAJOR", 12);
    cmp(&mut c, e, "HB_VERSION_MINOR", 3);
    cmp(&mut c, e, "HB_VERSION_MICRO", 2);

    assert!(
        errs.is_empty(),
        "FFI layout differs from the C headers:\n{}",
        errs.join("\n")
    );
    assert!(
        c.is_empty(),
        "probe entries no Rust check covers: {:?}",
        c.keys().collect::<Vec<_>>()
    );
}
