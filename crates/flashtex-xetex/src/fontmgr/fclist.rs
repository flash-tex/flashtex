//! The order `FcFontList` lists faces in (fontconfig's `fclist.c`), which
//! is the order `XeTeXFontMgr_FC::searchForHostPlatformFonts` walks
//! `allFonts` in: when two installed faces share a name, `addToMaps` keeps
//! the first, so TeX Live's XeTeX on Linux selects the face this order puts
//! first (#2545; `FreeSerif`: TeX Live's `truetype/.../FreeSerif.ttf`, not
//! the `opentype/.../FreeSerif.otf` a directory walk meets first).
//!
//! `FcFontSetList` puts each listed pattern in bucket
//! `FcListPatternHash(pattern) % FC_LIST_HASH_SIZE` (4099), appended within
//! the bucket, and returns the buckets in index order. `FcListPatternHash`
//! XORs, over the object set's elements, the XOR of `FcListValueHash` of
//! each value: a string `FcStrHashIgnoreCase`, an integer its value, a
//! double `(int)` its value. XeTeX's object set is `family, style, file,
//! index, fullname, weight, width, slant, fontformat`.
//!
//! The values are fontconfig's own (`FcFreeTypeQueryFaceInternal`); they
//! are modelled here from the face's `name` table ([`PatternNames`]),
//! `OS/2`, `head` and table directory. Not modelled: variable fonts (their
//! weight is a range in fontconfig, and fontconfig lists their named
//! instances as faces of their own), `.woff` files, and fontconfig's
//! scan-time configuration rules. Only the Linux build applies this order
//! ([`fc_list_order`] from [`super::FontCatalog`]); TeX Live's macOS build
//! asks Core Text, where the question does not arise.

use super::catalog::PatternNames;
use super::sfnt::{Face, FileBytes};
use std::path::Path;

/// `FC_LIST_HASH_SIZE` (fclist.c).
pub const FC_LIST_HASH_SIZE: u32 = 4099;

/// The values of one face's pattern for XeTeX's `FcFontList` object set.
#[derive(Debug, Clone, PartialEq)]
pub struct FcListValues {
    pub families: Vec<String>,
    pub styles: Vec<String>,
    pub full_names: Vec<String>,
    pub file: String,
    pub index: u32,
    pub weight: f64,
    pub width: f64,
    pub slant: i32,
    pub fontformat: &'static str,
}

/// `FcStrCaseWalkerNext`'s folding of one character: ASCII and Unicode
/// lower-casing, with the case folds that differ from it (fontconfig folds
/// U+00B5 MICRO SIGN to U+03BC, final sigma to sigma, long s to s).
fn fold(c: char, out: &mut String) {
    match c {
        '\u{b5}' => out.push('\u{3bc}'),
        '\u{3c2}' => out.push('\u{3c3}'),
        '\u{17f}' => out.push('s'),
        _ => out.extend(c.to_lowercase()),
    }
}

/// `FcStrHashIgnoreCase`: `h = ((h << 3) ^ (h >> 3)) ^ c` over the
/// case-folded UTF-8 bytes, 32-bit.
pub fn str_hash_ignore_case(s: &str) -> u32 {
    let mut folded = String::with_capacity(s.len());
    for c in s.chars() {
        fold(c, &mut folded);
    }
    folded
        .bytes()
        .fold(0u32, |h, c| ((h << 3) ^ (h >> 3)) ^ u32::from(c))
}

/// `FcListValueHash` of a double: `(FcChar32) (int) v`.
fn double_hash(v: f64) -> u32 {
    (v as i32) as u32
}

/// `FcListPatternHash` over XeTeX's object set.
pub fn list_hash(v: &FcListValues) -> u32 {
    let strings = |l: &[String]| l.iter().fold(0u32, |h, s| h ^ str_hash_ignore_case(s));
    strings(&v.families)
        ^ strings(&v.styles)
        ^ str_hash_ignore_case(&v.file)
        ^ v.index
        ^ strings(&v.full_names)
        ^ double_hash(v.weight)
        ^ double_hash(v.width)
        ^ (v.slant as u32)
        ^ str_hash_ignore_case(v.fontformat)
}

/// The bucket `FcFontSetList` files the face under.
pub fn bucket(v: &FcListValues) -> u32 {
    list_hash(v) % FC_LIST_HASH_SIZE
}

/// `FcWeightFromOpenTypeDouble` (fcweight.c).
pub fn weight_from_opentype(ot: f64) -> f64 {
    const MAP: [(f64, f64); 13] = [
        (0.0, 0.0),
        (100.0, 0.0),
        (200.0, 40.0),
        (300.0, 50.0),
        (350.0, 55.0),
        (380.0, 75.0),
        (400.0, 80.0),
        (500.0, 100.0),
        (600.0, 180.0),
        (700.0, 200.0),
        (800.0, 205.0),
        (900.0, 210.0),
        (1000.0, 215.0),
    ];
    if ot < 0.0 {
        return -1.0;
    }
    let ot = ot.min(MAP[MAP.len() - 1].0);
    let mut i = 1;
    while ot > MAP[i].0 {
        i += 1;
    }
    if ot == MAP[i].0 {
        return MAP[i].1;
    }
    let (o1, f1) = MAP[i - 1];
    let (o2, f2) = MAP[i];
    f1 + (f2 - f1) * (ot - o1) / (o2 - o1)
}

/// fcfreetype.c's `usWidthClass` switch (`None` for a class out of range).
fn width_from_class(class: u16) -> Option<f64> {
    Some(match class {
        1 => 50.0,
        2 => 63.0,
        3 => 75.0,
        4 => 87.0,
        5 => 100.0,
        6 => 113.0,
        7 => 125.0,
        8 => 150.0,
        9 => 200.0,
        _ => return None,
    })
}

/// fontconfig's names: surrounding spaces trimmed, an empty name left out
/// (measured: `LinLibertine_MBO.otf`'s family `Linux Libertine Mono O `
/// is `fc-query`'s `Linux Libertine Mono O`), and a name equal to an
/// earlier one ignoring case and blanks not added again
/// (`FcStringInPatternElement`).
fn dedup(names: &[String]) -> Vec<String> {
    let key = |s: &str| -> String {
        let mut k = String::new();
        for c in s.chars().filter(|c| *c != ' ') {
            fold(c, &mut k);
        }
        k
    };
    let mut seen = std::collections::HashSet::new();
    names
        .iter()
        .map(|s| s.trim_matches(' '))
        .filter(|s| !s.is_empty() && seen.insert(key(s)))
        .map(str::to_string)
        .collect()
}

/// fcfreetype.c's search of the style names for a slant (`FcContainsSlant`
/// over `FcSlantConsts`, ignoring case and blanks), which decides before
/// FreeType's italic flag: measured, `FiraMono-BoldOblique.otf` (italic
/// flag set) is `slant: 110`.
fn slant_of_styles(styles: &[String]) -> Option<i32> {
    styles.iter().find_map(|s| {
        let mut k = String::new();
        for c in s.chars().filter(|c| *c != ' ') {
            fold(c, &mut k);
        }
        [("italic", 100), ("kursiv", 100), ("oblique", 110)]
            .iter()
            .find(|(n, _)| k.contains(n))
            .map(|(_, v)| *v)
    })
}

/// The values fontconfig gives face `index` of `path`, from its pattern
/// names and the file's `OS/2`, `head` and table directory; `None` when
/// the face cannot be opened.
pub fn values_of(path: &Path, index: u32, names: &PatternNames) -> Option<FcListValues> {
    let src = FileBytes::open(path).ok()?;
    let face = Face::open(&src, index)?;
    let os2 = face.os2();
    let mac_style = face.mac_style().unwrap_or(0);
    let (weight, width, italic) = match os2 {
        Some((w, wd, sel)) => {
            let w = if (1..10).contains(&w) {
                f64::from(w) * 100.0
            } else {
                f64::from(w)
            };
            (
                weight_from_opentype(w),
                width_from_class(wd).unwrap_or(100.0),
                sel & 0x201 != 0,
            )
        }
        None => (100.0, 100.0, mac_style & 2 != 0),
    };
    let fontformat = if face.table(b"CFF ").is_some() || face.table(b"CFF2").is_some() {
        "CFF"
    } else {
        "TrueType"
    };
    Some(FcListValues {
        families: dedup(&names.families),
        styles: dedup(&names.styles),
        full_names: dedup(&names.full_names),
        file: path.to_string_lossy().into_owned(),
        index,
        weight,
        width,
        slant: slant_of_styles(&names.styles).unwrap_or(if italic { 100 } else { 0 }),
        fontformat,
    })
}

/// Reorders `items` (each with its file, face index and pattern names) as
/// `FcFontList` lists them: by bucket, the given order kept within a bucket
/// (fontconfig's font set order: directories in configuration order, files
/// sorted). An item whose face cannot be opened goes into bucket 0.
pub fn fc_list_order<T>(items: Vec<T>, key: impl Fn(&T) -> (&Path, u32, &PatternNames)) -> Vec<T> {
    let mut keyed: Vec<(u32, T)> = items
        .into_iter()
        .map(|t| {
            let (p, i, n) = key(&t);
            (values_of(p, i, n).map_or(0, |v| bucket(&v)), t)
        })
        .collect();
    keyed.sort_by_key(|(b, _)| *b);
    keyed.into_iter().map(|(_, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fontconfig 2.18.3's values for TeX Live 2026's `gnu-freefont`
    /// `FreeSerif` (`fc-query`), at the paths of the NixOS PC of #2545.
    fn free_serif(file: &str, fontformat: &'static str) -> FcListValues {
        let styles = [
            "Regular",
            "нормален",
            "normal",
            "obyčejné",
            "Mittel",
            "µεσαία",
            "Normaali",
            "Normál",
            "Normale",
            "Gemiddeld",
            "odmiana zwykła",
            "Обычный",
            "Normálne",
            "ปกติ",
            "menengah",
            "прямій",
            "Navadno",
            "vidējs",
            "normalusis",
            "عادی",
            "vừa",
            "Arrunta",
            "सामान्य",
        ];
        FcListValues {
            families: vec!["FreeSerif".into()],
            styles: styles.iter().map(|s| s.to_string()).collect(),
            full_names: vec!["FreeSerif".into()],
            file: file.into(),
            index: 0,
            weight: 80.0,
            width: 100.0,
            slant: 0,
            fontformat,
        }
    }

    #[test]
    fn the_hash_is_fontconfigs() {
        assert_eq!(str_hash_ignore_case(""), 0);
        // h = ((h << 3) ^ (h >> 3)) ^ c, case folded.
        assert_eq!(str_hash_ignore_case("a"), 0x61);
        assert_eq!(str_hash_ignore_case("A"), 0x61);
        assert_eq!(str_hash_ignore_case("ab"), (0x61 << 3 ^ 0x61 >> 3) ^ 0x62);
        // U+00B5 folds to U+03BC.
        assert_eq!(str_hash_ignore_case("µ"), str_hash_ignore_case("μ"));
        assert_eq!(weight_from_opentype(400.0), 80.0);
        assert_eq!(weight_from_opentype(700.0), 200.0);
        assert_eq!(weight_from_opentype(450.0), 90.0);
        assert_eq!(weight_from_opentype(2000.0), 215.0);
    }

    /// #2545: on the NixOS PC, `FcFontList` lists TeX Live's
    /// `FreeSerif.ttf` (position 3208) before `FreeSerif.otf` (3811),
    /// though a directory walk meets `opentype` first.
    #[test]
    fn freeserif_ttf_comes_before_the_otf() {
        let root = "/home/kubar/texlive/2026/texmf-dist/fonts";
        let otf = free_serif(
            &format!("{root}/opentype/public/gnu-freefont/FreeSerif.otf"),
            "CFF",
        );
        let ttf = free_serif(
            &format!("{root}/truetype/public/gnu-freefont/FreeSerif.ttf"),
            "TrueType",
        );
        assert!(
            bucket(&ttf) < bucket(&otf),
            "{} {}",
            bucket(&ttf),
            bucket(&otf)
        );
        // Directory order in (opentype first), fontconfig's out.
        let order = fc_list_order(vec![otf.clone(), ttf], |_| {
            (Path::new("/nonexistent"), 0, &EMPTY)
        });
        // Faces that cannot be opened keep their order (bucket 0 each)...
        assert_eq!(order[0].file, otf.file);
        // ...and the bucket order is fontconfig's.
        let mut by_bucket = order;
        by_bucket.sort_by_key(bucket);
        assert!(by_bucket[0].file.ends_with("FreeSerif.ttf"));
    }

    /// fontconfig keeps the first of two names equal but for case and
    /// blanks, in its own order (typographic family before family): TeX
    /// Live's `TeXGyreScholaX-Bold.otf` is family `TeX Gyre ScholaX`
    /// (`fc-query`), its name id 1 `TeXGyreScholaX` left out.
    #[test]
    fn names_are_kept_in_fontconfigs_order() {
        use super::super::sfnt::NameRecord;
        let rec = |name_id: u16, s: &str| NameRecord {
            platform: 3,
            encoding: 1,
            language: 0x409,
            name_id,
            bytes: s.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        };
        let p = super::super::catalog::pattern_names(&[
            rec(1, "TeXGyreScholaX"),
            rec(16, "TeX Gyre ScholaX"),
        ]);
        assert_eq!(dedup(&p.families), vec!["TeX Gyre ScholaX".to_string()]);
    }

    /// The model against fontconfig (`cargo test --lib against_fc_list --
    /// --ignored --nocapture`): `FCLIST_ORACLE` is the output of
    /// `fc-list -f '%{file}\t%{index}\n' ':outline=true' family style file
    /// index fullname weight width slant fontformat` over the directories in
    /// `FCLIST_DIRS` (`:`-separated). Prints, over the faces both list, the
    /// pairs of faces sharing a full name that are listed in another order.
    #[test]
    #[ignore]
    fn against_fc_list() {
        let (Ok(oracle), Ok(dirs)) = (std::env::var("FCLIST_ORACLE"), std::env::var("FCLIST_DIRS"))
        else {
            return;
        };
        let text = std::fs::read_to_string(oracle).unwrap();
        let pos: std::collections::HashMap<(String, u32), usize> = text
            .lines()
            .enumerate()
            .filter_map(|(n, l)| {
                let mut it = l.split('\t');
                let (f, i) = (it.next()?, it.next()?);
                Some(((f.to_string(), i.parse().ok()?), n))
            })
            .collect();
        let dirs: Vec<std::path::PathBuf> = dirs.split(':').map(Into::into).collect();
        let cat = if std::env::var_os("FCLIST_UNORDERED").is_some() {
            super::super::FontCatalog::from_files(
                flashtex_font_discovery::FontIndex::scan(&dirs)
                    .files()
                    .iter()
                    .map(|f| (f.path.clone(), f.face_index)),
            )
        } else {
            super::super::FontCatalog::scan(&dirs).fc_list_ordered()
        };
        let ours: Vec<_> = cat
            .faces()
            .iter()
            .filter_map(|f| {
                let p = pos.get(&(f.path.to_string_lossy().into_owned(), f.index))?;
                Some((
                    *p,
                    f.pattern.full_names.first().cloned().unwrap_or_default(),
                    f,
                ))
            })
            .collect();
        if let Ok(show) = std::env::var("FCLIST_SHOW") {
            for f in cat.faces().iter().filter(|f| f.path.ends_with(&show)) {
                println!("SHOW {:?}", values_of(&f.path, f.index, &f.pattern));
            }
        }
        let descents = ours.windows(2).filter(|w| w[1].0 < w[0].0).count();
        if std::env::var_os("FCLIST_VERBOSE").is_some() {
            for w in ours.windows(2).filter(|w| w[1].0 < w[0].0) {
                println!(
                    "DESCENT {} {} -> {} {}",
                    w[0].0,
                    w[0].2.path.display(),
                    w[1].0,
                    w[1].2.path.display()
                );
            }
        }
        let (mut pairs, mut wrong) = (0, 0);
        for (a, x) in ours.iter().enumerate() {
            for y in &ours[a + 1..] {
                if x.1.is_empty() || x.1 != y.1 {
                    continue;
                }
                pairs += 1;
                if y.0 < x.0 {
                    wrong += 1;
                    println!(
                        "WRONG {:?}: {} before {}",
                        x.1,
                        x.2.path.display(),
                        y.2.path.display()
                    );
                }
            }
        }
        println!(
            "faces {} (oracle {}), descents {}, same-full-name pairs {}, in another order {}",
            ours.len(),
            pos.len(),
            descents,
            pairs,
            wrong
        );
    }

    static EMPTY: PatternNames = PatternNames {
        full_names: Vec::new(),
        families: Vec::new(),
        styles: Vec::new(),
    };
}
