//! Which `.tfm` file LaTeX's font definition files load for an NFSS text
//! shape at a size: `t1cmr.fd`/`ot1cmr.fd` and siblings (EC and Knuth CM),
//! `t1lm*.fd` (Latin Modern) and the TS1 companions. Moved from
//! `render-pipeline/src/fonts.rs` (PLAN3 S1), keyed by [`FontKey`] instead
//! of the pipeline's `Role`; the tables are unchanged.

use crate::nfss::{FamilyKind, FontKey, Series, Shape};

/// The `ec-lm*` TFM that `t1lm*.fd` pairs with a Latin Modern text file:
/// `lmroman12-regular` → `ec-lmr12`, `-bold` → `ec-lmbx12`, `-italic` →
/// `ec-lmri12`, `-bolditalic` → `ec-lmbxi10`. `None` for the math face and
/// for names this table does not know.
///
/// The other Latin Modern designs pair the same way (`t1lmr.fd`,
/// `t1lmss.fd`, `t1lmtt.fd`): `lmromanslant` → `ec-lmro`/`ec-lmbxo`,
/// `lmromancaps` → `ec-lmcsc`/`ec-lmcsco`, `lmromandemi` → `ec-lmb`/
/// `ec-lmbo`, `lmromanunsl` → `ec-lmu`, `lmsans` → `ec-lmss`/`ec-lmsso`/
/// `ec-lmssbx`/`ec-lmssbo`, `lmsansdemicond` → `ec-lmssdc`/`ec-lmssdo`,
/// `lmmono` → `ec-lmtt`/`ec-lmtti`, `lmmonoslant` → `ec-lmtto`,
/// `lmmonocaps` → `ec-lmtcsc`/`ec-lmtcso`, `lmmonolt` bold → `ec-lmtk`/
/// `ec-lmtko`.
pub fn latin_modern_tfm(otf_stem: &str) -> Option<String> {
    let digits_at = otf_stem.find(|c: char| c.is_ascii_digit())?;
    let (base, rest) = otf_stem.split_at(digits_at);
    let (digits, style) = rest.split_once('-')?;
    let d: u32 = digits.parse().ok()?;
    let series = match (base, style) {
        ("lmroman", "regular") => "r",
        ("lmroman", "bold") => "bx",
        ("lmroman", "italic") => "ri",
        ("lmroman", "bolditalic") => "bxi",
        ("lmromanslant", "regular") => "ro",
        ("lmromanslant", "bold") => "bxo",
        ("lmromancaps", "regular") => "csc",
        ("lmromancaps", "oblique") => "csco",
        ("lmromandemi", "regular") => "b",
        ("lmromandemi", "oblique") => "bo",
        ("lmromanunsl", "regular") => "u",
        ("lmsans", "regular") => "ss",
        ("lmsans", "oblique") => "sso",
        ("lmsans", "bold") => "ssbx",
        ("lmsans", "boldoblique") => "ssbo",
        ("lmsansdemicond", "regular") => "ssdc",
        ("lmsansdemicond", "oblique") => "ssdo",
        ("lmmono", "regular") => "tt",
        ("lmmono", "italic") => "tti",
        ("lmmonoslant", "regular") => "tto",
        ("lmmonocaps", "regular") => "tcsc",
        ("lmmonocaps", "oblique") => "tcso",
        ("lmmonolt", "bold") => "tk",
        ("lmmonolt", "boldoblique") => "tko",
        _ => return None,
    };
    Some(format!("ec-lm{series}{d}.tfm"))
}

/// The Latin Modern OpenType file drawing an NFSS shape at `size_pt`, with
/// the design sizes of `t1lmr.fd` (`m/n` `<-5.5>`5 ... `<11-15>`12 `<15->`17,
/// `bx/n` up to 12, `m/it` 7-12, `m/sl` 8-17), `t1lmss.fd` (`m/n`, `m/sl`:
/// `<-8.5>`8 `<8.5-9.5>`9 `<9.5-11>`10 `<11-15.5>`12 `<15.5->`17) and
/// `t1lmtt.fd` (`m/n`: 8, 9, 10, `<11->`12); every other shape has one
/// 10 pt design. The note is set when Latin Modern has no design for the
/// shape (the EC fonts' `bx/sc` `ecxc`, for example) and a neighbour is
/// drawn instead.
pub fn latin_modern_outline(key: FontKey, size_pt: f64) -> (String, Option<&'static str>) {
    use FamilyKind::{Rm, Sf, Tt};
    use Series::{Bx, Sbc, B, M};
    use Shape::{Ui, It, Sc, Scit, Scsl, Sl, N};
    const RM: [(f64, u32); 7] = [(5.5, 5), (6.5, 6), (7.5, 7), (8.5, 8), (9.5, 9), (11.0, 10), (15.0, 12)];
    const RM_IT: [(f64, u32); 4] = [(7.5, 7), (8.5, 8), (9.5, 9), (11.0, 10)];
    const RM_SL: [(f64, u32); 4] = [(8.5, 8), (9.5, 9), (11.0, 10), (15.0, 12)];
    const SS: [(f64, u32); 4] = [(8.5, 8), (9.5, 9), (11.0, 10), (15.5, 12)];
    const TT: [(f64, u32); 3] = [(8.5, 8), (9.5, 9), (11.0, 10)];
    const NO_BOLD_CAPS: &str = "Latin Modern has no bold small-caps design; the medium one is drawn";
    let pick = |bounds: &[(f64, u32)], last: u32| bounds.iter().find(|(b, _)| size_pt < *b).map_or(last, |(_, d)| *d);
    let exact = |file: String| (file, None);
    match (key.family, key.series, key.shape) {
        (Rm, M, N) => exact(format!("lmroman{}-regular.otf", pick(&RM, 17))),
        (Rm, Bx, N) => exact(format!("lmroman{}-bold.otf", pick(&RM[..6], 12))),
        (Rm, M, It) => exact(format!("lmroman{}-italic.otf", pick(&RM_IT, 12))),
        (Rm, Bx, It) => exact("lmroman10-bolditalic.otf".into()),
        (Rm, M, Sl) => exact(format!("lmromanslant{}-regular.otf", pick(&RM_SL, 17))),
        (Rm, Bx, Sl) => exact("lmromanslant10-bold.otf".into()),
        (Rm, M, Sc) => exact("lmromancaps10-regular.otf".into()),
        (Rm, M, Scsl) => exact("lmromancaps10-oblique.otf".into()),
        (Rm, M, Scit) => ("lmromancaps10-oblique.otf".into(), Some("Latin Modern has no italic small-caps design; the slanted one is drawn")),
        (Rm, M, Ui) => exact("lmromanunsl10-regular.otf".into()),
        (Rm, B, N) => exact("lmromandemi10-regular.otf".into()),
        (Rm, B, Sl | It) => exact("lmromandemi10-oblique.otf".into()),
        (Rm, Bx | B, Sc) => ("lmromancaps10-regular.otf".into(), Some(NO_BOLD_CAPS)),
        (Rm, Bx | B, Scsl | Scit) => ("lmromancaps10-oblique.otf".into(), Some(NO_BOLD_CAPS)),
        (Sf, M, N) => exact(format!("lmsans{}-regular.otf", pick(&SS, 17))),
        (Sf, M, Sl | It) => exact(format!("lmsans{}-oblique.otf", pick(&SS, 17))),
        (Sf, Bx | B, N) => exact("lmsans10-bold.otf".into()),
        (Sf, Bx | B, Sl | It) => exact("lmsans10-boldoblique.otf".into()),
        (Sf, Sbc, N) => exact("lmsansdemicond10-regular.otf".into()),
        (Sf, Sbc, Sl | It) => exact("lmsansdemicond10-oblique.otf".into()),
        (Tt, M, N) => exact(format!("lmmono{}-regular.otf", pick(&TT, 12))),
        (Tt, M, It) => exact("lmmono10-italic.otf".into()),
        (Tt, M, Sl) => exact("lmmonoslant10-regular.otf".into()),
        (Tt, M, Sc) => exact("lmmonocaps10-regular.otf".into()),
        (Tt, M, Scsl) => exact("lmmonocaps10-oblique.otf".into()),
        (Tt, B | Bx, N) => exact("lmmonolt10-bold.otf".into()),
        (Tt, B | Bx, Sl | It) => exact("lmmonolt10-boldoblique.otf".into()),
        _ => {
            let roman = FontKey::new(Rm, if key.bold() { Bx } else { M }, if key.slanted() { It } else { N });
            (latin_modern_outline(roman, size_pt).0, Some("no Latin Modern design for this font shape; the roman one of the same weight and slant is drawn"))
        }
    }
}

/// The sizes `t1cmr.fd` declares for every EC shape
/// (`<5><6><7><8><9><10><10.95><12><14.4><17.28><20.74><24.88><29.86><35.83>genb*ecrm`)
/// and the file-name suffix `genb*` builds from each.
pub const EC_SIZES: [(f64, &str); 14] = [
    (5.0, "0500"),
    (6.0, "0600"),
    (7.0, "0700"),
    (8.0, "0800"),
    (9.0, "0900"),
    (10.0, "1000"),
    (10.95, "1095"),
    (12.0, "1200"),
    (14.4, "1440"),
    (17.28, "1728"),
    (20.74, "2074"),
    (24.88, "2488"),
    (29.86, "2986"),
    (35.83, "3583"),
];

/// The EC metric file the T1 Computer Modern `.fd` files load for a text
/// role at `size_pt`, at the declared size nearest `size_pt` (an undeclared
/// size is a LaTeX size substitution to the nearest one):
///
/// * `t1cmr.fd`: `m/n` `ecrm`, `m/sl` `ecsl`, `m/it` `ecti`, `m/sc` `eccc`,
///   `bx/n` `ecbx`, `b/n` `ecrb`, `bx/it` `ecbi`, `bx/sl` `ecbl`, `bx/sc`
///   `ecxc`, `m/ui` `ecui`, `m/scsl` `ecsc`, `bx/scsl` and `b/scsl` `ecoc`;
/// * `t1cmss.fd`: `m/n` `ecss`, `m/sl` and `m/it` `ecsi`, `bx/n` `ecsx`,
///   `bx/it` and `bx/sl` `ecso`;
/// * `t1cmtt.fd`: `m/n` `ectt`, `m/sl` `ecst`, `m/it` `ecit`, `m/sc` `ectc`.
///
/// The sans and typewriter families declare `<5><6><7><8>#50800`: every
/// size up to 8 pt uses the 8 pt file. `None` for math and for shapes the
/// files do not declare (they are substituted before a font is loaded).
pub fn ec_tfm_file(key: FontKey, size_pt: f64) -> Option<String> {
    use FamilyKind::{Rm, Sf, Tt};
    use Series::{Bx, B, M};
    use Shape::{Ui, It, Sc, Scsl, Sl, N};
    let (prefix, small_sizes_share_0800) = match (key.family, key.series, key.shape) {
        (Rm, M, N) => ("ecrm", false),
        (Rm, M, Sl) => ("ecsl", false),
        (Rm, M, It) => ("ecti", false),
        (Rm, M, Sc) => ("eccc", false),
        (Rm, M, Ui) => ("ecui", false),
        (Rm, M, Scsl) => ("ecsc", false),
        (Rm, Bx, N) => ("ecbx", false),
        (Rm, B, N) => ("ecrb", false),
        (Rm, Bx, It) => ("ecbi", false),
        (Rm, Bx, Sl) => ("ecbl", false),
        (Rm, Bx, Sc) => ("ecxc", false),
        (Rm, Bx | B, Scsl) => ("ecoc", false),
        (Sf, M, N) => ("ecss", true),
        (Sf, M, Sl | It) => ("ecsi", true),
        (Sf, Bx, N) => ("ecsx", true),
        (Sf, Bx, Sl | It) => ("ecso", true),
        (Tt, M, N) => ("ectt", true),
        (Tt, M, Sl) => ("ecst", true),
        (Tt, M, It) => ("ecit", true),
        (Tt, M, Sc) => ("ectc", true),
        _ => return None,
    };
    let (size, suffix) = EC_SIZES
        .iter()
        .min_by(|a, b| (a.0 - size_pt).abs().total_cmp(&(b.0 - size_pt).abs()))?;
    let suffix = if small_sizes_share_0800 && *size <= 8.0 { "0800" } else { suffix };
    Some(format!("{prefix}{suffix}.tfm"))
}

/// The TS1 (text companion) metric file set with the text file `tfm`: the
/// font `\UseTextSymbol{TS1}{..}` switches to for a symbol T1 lacks
/// (`\textcopyright`, `\textdegree`, ...), which keeps the family, series,
/// shape and size and changes only the encoding.
///
/// * `ts1cmr.fd`: `tcrm`/`tcsl`/`tcti`/`tcbx`/`tcrb`/`tcbi`/`tcbl`/`tcui`
///   at the EC sizes -- `ec<shape><size>` → `tc<shape><size>`; the small-caps
///   shapes it does not declare (`eccc`, `ecsc`, `ecxc`, `ecoc`, `ectc`)
///   are NFSS-substituted by the upright of the same series (`m/sc` →
///   `m/n`, `bx/sc` → `bx/n`) before the font is loaded;
/// * `ts1cmss.fd`/`ts1cmtt.fd`: `tcss`/`tcsi`/`tcsx`/`tcso`, `tctt`/`tcst`/
///   `tcit` likewise;
/// * `ts1lm*.fd`: `ec-lm<face>` → `ts1-lm<face>`.
///
/// `None` for a file this table cannot pair. The companion is optional:
/// a face without one sets those symbols from its own program's advances,
/// as before.
pub fn ts1_companion_tfm(tfm: &str, size_pt: f64) -> Option<String> {
    let stem = tfm.strip_suffix(".tfm")?;
    if let Some(rest) = stem.strip_prefix("ec-lm") {
        return Some(format!("ts1-lm{rest}.tfm"));
    }
    // Knuth's OT1 files (`cmr10` at 10.95pt): `ts1cmr.fd` declares the
    // companions at the EC sizes (`genb*tcrm`), so the file is chosen by
    // the size, not the design.
    if let Some(cm) = stem.strip_prefix("cm").filter(|_| !stem.starts_with("cm-")) {
        let design = cm.trim_end_matches(|c: char| c.is_ascii_digit());
        let shape = match design {
            "r" | "csc" => "rm",
            "bx" | "b" => "bx",
            "ti" => "ti",
            "sl" => "sl",
            "bxti" => "bi",
            "bxsl" => "bl",
            "u" => "ui",
            "ss" | "ssdc" => "ss",
            "ssi" => "si",
            "ssbx" => "sx",
            "tt" | "tcsc" => "tt",
            "itt" => "it",
            "sltt" => "st",
            _ => return None,
        };
        let (_, suffix) = EC_SIZES.iter().min_by(|a, b| (a.0 - size_pt).abs().total_cmp(&(b.0 - size_pt).abs()))?;
        return Some(format!("tc{shape}{suffix}.tfm"));
    }
    let rest = stem.strip_prefix("ec")?;
    let (shape, size) = rest.split_at(rest.find(|c: char| c.is_ascii_digit())?);
    let shape = match shape {
        "cc" | "sc" => "rm",
        "xc" | "oc" => "bx",
        "tc" => "tt",
        other => other,
    };
    Some(format!("tc{shape}{size}.tfm"))
}

/// The OT1 metric file `ot1cmr.fd`/`ot1cmss.fd` (TeX Live 2026) load for
/// a text role at `size_pt`, at the declared size nearest `size_pt`. The
/// files are Knuth's design sizes scaled to the requested size (`cmr10 at
/// 10.95pt`), unlike the EC files which exist at every size:
///
/// * `ot1cmr.fd`: `m/n` `<5><6><7><8><9><10><12>gen*cmr <10.95>cmr10
///   <14.4>cmr12 <17.28><20.74><24.88>cmr17`; `m/sl` `<5><6><7>cmsl8
///   <8><9>gen*cmsl <10><10.95>cmsl10 <12>...cmsl12`; `m/it` `<5><6><7>cmti7
///   <8>cmti8 <9>cmti9 <10><10.95>cmti10 <12>...cmti12`; `m/sc` `cmcsc10`;
///   `m/ui` `cmu10`; `b/n` `cmb10`; `bx/n` `<5>...<9>gen*cmbx <10><10.95>cmbx10
///   <12>...cmbx12`; `bx/sl` `cmbxsl10`; `bx/it` `cmbxti10`;
/// * `ot1cmss.fd`: `m/n` `<5>...<8>cmss8 <9>cmss9 <10><10.95>cmss10
///   <12><14.4>cmss12 <17.28>...cmss17`; `m/sl` (and `m/it`, `ssub`) the
///   `cmssi` files at the same sizes; `bx/n` `cmssbx10`; `sbc/n` `cmssdc10`;
///   an undeclared `bx/it`/`bx/sl` is NFSS-substituted by `bx/n`.
///
/// `None` for math, for the typewriter family (see
/// render-pipeline's `Family::ComputerModernOt1`) and for shapes the files do not declare.
pub fn ot1_tfm_file(key: FontKey, size_pt: f64) -> Option<String> {
    use FamilyKind::{Rm, Sf};
    use Series::{Bx, Sbc, B, M};
    use Shape::{Ui, It, Sc, Sl, N};
    // The declared sizes of both files; an undeclared size is LaTeX's
    // substitution to the nearest one.
    const SIZES: [f64; 12] = [5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.95, 12.0, 14.4, 17.28, 20.74, 24.88];
    let size = SIZES.iter().copied().min_by(|a, b| (a - size_pt).abs().total_cmp(&(b - size_pt).abs()))?;
    let gen = |prefix: &str, own: &[u32], else_: &[(f64, u32)]| -> String {
        let d = size.round() as u32;
        if (size - f64::from(d)).abs() < 1e-9 && own.contains(&d) {
            return format!("{prefix}{d}.tfm");
        }
        let (_, d) = else_.iter().find(|(at, _)| (*at - size).abs() < 1e-9).copied().unwrap_or(*else_.last().unwrap());
        format!("{prefix}{d}.tfm")
    };
    // The `<a><b>file` runs of the declarations as (declared size, design).
    let table = |prefix: &str, runs: &[(&[f64], u32)]| -> String {
        let d = runs.iter().find(|(sizes, _)| sizes.iter().any(|at| (*at - size).abs() < 1e-9)).map_or(runs.last().unwrap().1, |(_, d)| *d);
        format!("{prefix}{d}.tfm")
    };
    let file = match (key.family, key.series, key.shape) {
        (Rm, M, N) => gen("cmr", &[5, 6, 7, 8, 9, 10, 12], &[(10.95, 10), (14.4, 12), (17.28, 17), (20.74, 17), (24.88, 17)]),
        (Rm, M, Sl) => table("cmsl", &[(&[5.0, 6.0, 7.0, 8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4, 17.28, 20.74, 24.88], 12)]),
        (Rm, M, It) => table("cmti", &[(&[5.0, 6.0, 7.0], 7), (&[8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4, 17.28, 20.74, 24.88], 12)]),
        (Rm, M, Sc) => "cmcsc10.tfm".to_string(),
        (Rm, M, Ui) => "cmu10.tfm".to_string(),
        (Rm, B, N) => "cmb10.tfm".to_string(),
        (Rm, Bx, N) => gen("cmbx", &[5, 6, 7, 8, 9], &[(10.0, 10), (10.95, 10), (12.0, 12), (14.4, 12), (17.28, 12), (20.74, 12), (24.88, 12)]),
        (Rm, Bx, Sl) => "cmbxsl10.tfm".to_string(),
        (Rm, Bx, It) => "cmbxti10.tfm".to_string(),
        (Sf, M, N) => table("cmss", &[(&[5.0, 6.0, 7.0, 8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4], 12), (&[17.28, 20.74, 24.88], 17)]),
        (Sf, M, Sl | It) => table("cmssi", &[(&[5.0, 6.0, 7.0, 8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4], 12), (&[17.28, 20.74, 24.88], 17)]),
        (Sf, Bx, N | Sl | It) => "cmssbx10.tfm".to_string(),
        (Sf, Sbc, N) => "cmssdc10.tfm".to_string(),
        _ => return None,
    };
    Some(file)
}
