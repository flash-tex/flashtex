//! `ToUnicode` of an AAT Arabic font against xdvipdfmx's (#1787): Geeza Pro
//! (`/System/Library/Fonts/GeezaPro.ttc`, face 0) and Times New Roman's
//! Hebrew, the fonts of `tools/xetex-lockstep/latex-cases/l011-bidi-rtl.tex`,
//! in a plain INITEX job.
//!
//! The expected maps are xdvipdfmx's, from TeX Live's `xetex -ini` on
//! [`INPUT`] with the same font files. The test runs that oracle itself:
//! `FLASHTEX_XETEX_ORACLE`, else `/Library/TeX/texbin/xetex`. Without an
//! oracle it falls back to the maps pinned below (TeX Live 2026, MacTeX,
//! macOS 26.6.2 arm64), but only when both font files have the SHA-256
//! they were measured on. When the oracle runs on those files, its maps
//! must also equal the pinned ones.
//!
//! The fonts are macOS system fonts, so the test is `#[ignore]`d. Run it
//! with `cargo test --release --test tounicode_oracle -- --ignored`, as
//! `.github/workflows/unicode-tounicode-mac.yml` does on a hosted Mac with
//! BasicTeX. When run, a missing font, oracle or glyph list fails it rather
//! than passing. FlashTeX's resolver must find the glyph lists
//! (`texglyphlist.txt` and the rest, `out/agl.rs`) as xdvipdfmx does.

use flashtex_pdf::reader::{Obj, PdfFile};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const INPUT: &str = "\\catcode`\\{=1 \\catcode`\\}=2
\\font\\a=\"Geeza Pro:script=arab\" at 12pt
\\font\\h=\"Times New Roman:script=hebr\" at 12pt
\\shipout\\hbox{\\a النص (العربي) مع كلمات و ٣٫١٤ و ١٢٣ في [القوسين]، نهاية.}
\\shipout\\hbox{\\h שלום (עולם) [abc] 3.14 ו-2026, \"ציטוט\" ו-ABC 12\\char`\\%: סוף.}
\\end
";

const GEEZA: &str = "/System/Library/Fonts/GeezaPro.ttc";
const GEEZA_SHA256: &str = "5b4309f8d596abee520c0826885a39483a16fb244489d2c313322575df4b04a2";
const TIMES: &str = "/System/Library/Fonts/Supplemental/Times New Roman.ttf";
const TIMES_SHA256: &str = "f3b4ffff71c2a0c7227d37497683b2498fb2d0a4e8beae26f022e3ccfcaabfa3";

/// xdvipdfmx's Geeza Pro map: glyph id to UTF-16 text. `u064a`-style
/// post names read as U+FFFD, as `agl_name_is_unicode`/`xtol` read lower-case
/// hex; the `.notdef` glyph follows the cmap's 0xFFFF segment.
const GEEZA_REF: &[(u16, &str)] = &[
    (0, "FFFF"),
    (42, "002E"),
    (55, "060C"),
    (82, "066B"),
    (111, "0661"),
    (112, "0662"),
    (113, "0663"),
    (114, "0664"),
    (207, "FFFD 0646"),
    (240, "0627"),
    (241, "0627"),
    (243, "0628"),
    (247, "0629"),
    (248, "062A"),
    (273, "0631"),
    (277, "0633"),
    (287, "0635"),
    (302, "0639"),
    (329, "0641"),
    (334, "0642"),
    (337, "0643"),
    (341, "0644"),
    (342, "0644"),
    (345, "0645"),
    (346, "0645"),
    (349, "0646"),
    (350, "0646"),
    (354, "0647"),
    (356, "0648"),
    (357, "0648"),
    (363, "FFFD"),
    (904, "FFFD"),
    (925, "0639"),
];

/// xdvipdfmx's Times New Roman map (the Hebrew page). Glyph 16 is the
/// hyphen, U+002D by the cmap, not the soft hyphen's fallback.
const TIMES_REF: &[(u16, &str)] = &[
    (5, "0022"),
    (8, "0025"),
    (11, "0028"),
    (12, "0029"),
    (15, "002C"),
    (16, "002D"),
    (17, "002E"),
    (19, "0030"),
    (20, "0031"),
    (21, "0032"),
    (22, "0033"),
    (23, "0034"),
    (25, "0036"),
    (29, "003A"),
    (36, "0041"),
    (37, "0042"),
    (38, "0043"),
    (62, "005B"),
    (64, "005D"),
    (68, "0061"),
    (69, "0062"),
    (70, "0063"),
    (677, "05D5"),
    (680, "05D8"),
    (681, "05D9"),
    (684, "05DC"),
    (685, "05DD"),
    (689, "05E1"),
    (690, "05E2"),
    (691, "05E3"),
    (694, "05E6"),
    (697, "05E9"),
];

fn sha256(path: &str) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    Some(
        Sha256::digest(&data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

fn hex_u16(s: &str) -> Vec<u16> {
    s.as_bytes()
        .chunks(4)
        .map(|c| u16::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}

/// The `bfchar` and `bfrange` entries of a `ToUnicode` CMap: code to its
/// UTF-16 units.
fn read_cmap(text: &str) -> BTreeMap<u16, Vec<u16>> {
    let mut out = BTreeMap::new();
    let hexes = |s: &str| -> Vec<String> {
        s.split('<')
            .skip(1)
            .map(|p| p.split('>').next().unwrap().to_string())
            .collect()
    };
    for blk in text.split("beginbfchar").skip(1) {
        let body = blk.split("endbfchar").next().unwrap();
        for line in body.lines() {
            let h = hexes(line);
            if h.len() == 2 {
                out.insert(hex_u16(&h[0])[0], hex_u16(&h[1]));
            }
        }
    }
    for blk in text.split("beginbfrange").skip(1) {
        let body = blk.split("endbfrange").next().unwrap();
        for line in body.lines() {
            let h = hexes(line);
            if h.len() < 3 {
                continue;
            }
            let (lo, hi) = (hex_u16(&h[0])[0], hex_u16(&h[1])[0]);
            if line.contains('[') {
                for (i, t) in h[2..].iter().enumerate() {
                    out.insert(lo + i as u16, hex_u16(t));
                }
            } else {
                let base = hex_u16(&h[2]);
                for i in 0..=(hi - lo) {
                    let mut v = base.clone();
                    *v.last_mut().unwrap() += i;
                    out.insert(lo + i, v);
                }
            }
        }
    }
    out
}

fn expected(rows: &[(u16, &str)]) -> BTreeMap<u16, Vec<u16>> {
    rows.iter()
        .map(|&(g, t)| (g, t.split(' ').flat_map(hex_u16).collect()))
        .collect()
}

/// Runs `engine` on [`INPUT`] in a fresh `dir` and returns its PDF's
/// `ToUnicode` maps, one per page: the font's `BaseFont` and its map.
fn run_and_read(
    engine: &Path,
    dir: &Path,
    path_env: Option<String>,
) -> Vec<(String, BTreeMap<u16, Vec<u16>>)> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("ar.tex"), INPUT).unwrap();
    let mut cmd = Command::new(engine);
    cmd.current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("TZ", "UTC")
        .args(["-ini", "-interaction=nonstopmode", "ar.tex"]);
    if let Some(p) = path_env {
        cmd.env("PATH", p);
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("run {}: {e}", engine.display()));
    let log = std::fs::read_to_string(dir.join("ar.log")).unwrap_or_default();
    assert!(
        log.contains("Output written on ar.pdf (2 pages)."),
        "{}:\n{log}\n{}",
        engine.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(dir.join("ar.pdf")).unwrap();
    let pdf = PdfFile::parse(&bytes).unwrap();
    let pages = pdf.pages().unwrap();
    assert_eq!(pages.len(), 2, "{}", engine.display());
    pages
        .iter()
        .map(|&page| {
            let fonts = pdf.page_fonts(page);
            assert_eq!(fonts.len(), 1, "{}", engine.display());
            let f = *fonts.values().next().unwrap();
            let base = f
                .get("BaseFont")
                .and_then(Obj::as_name)
                .unwrap()
                .to_string();
            let tu = f
                .get("ToUnicode")
                .unwrap_or_else(|| panic!("{}: {base} has no ToUnicode", engine.display()));
            let cmap = pdf.decode_stream(pdf.resolve(tu)).unwrap();
            (base, read_cmap(&String::from_utf8_lossy(&cmap)))
        })
        .collect()
}

fn differences(want: &BTreeMap<u16, Vec<u16>>, got: &BTreeMap<u16, Vec<u16>>) -> Vec<String> {
    let fmt = |m: &BTreeMap<u16, Vec<u16>>, k: &u16| {
        m.get(k).map(|v| {
            v.iter()
                .map(|u| format!("{u:04X}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
    };
    want.keys()
        .chain(got.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|k| got.get(k) != want.get(k))
        .map(|k| {
            format!(
                "gid {k}: xdvipdfmx {:?}, FlashTeX {:?}",
                fmt(want, k),
                fmt(got, k)
            )
        })
        .collect()
}

#[test]
#[ignore = "needs macOS's Geeza Pro and Times New Roman and TeX Live's xetex: \
            cargo test --release --test tounicode_oracle -- --ignored \
            (CI: unicode-tounicode-mac.yml)"]
fn aat_arabic_tounicode_is_xdvipdfmx_s() {
    assert!(
        cfg!(target_os = "macos"),
        "Geeza Pro is a macOS system font: this test runs only on macOS"
    );
    let measured = [(GEEZA, GEEZA_SHA256), (TIMES, TIMES_SHA256)]
        .iter()
        .all(|&(path, want)| {
            let got = sha256(path).unwrap_or_else(|| panic!("{path}: not found"));
            if got != want {
                eprintln!("{path} is {got}, not the file the pinned maps were measured on");
            }
            got == want
        });
    let pinned = [
        ("+GeezaPro", expected(GEEZA_REF)),
        ("+TimesNewRomanPSMT", expected(TIMES_REF)),
    ];
    let base =
        std::env::temp_dir().join(format!("flashtex-xetex-tounicode-{}", std::process::id()));
    let oracle = std::env::var_os("FLASHTEX_XETEX_ORACLE")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let p = Path::new("/Library/TeX/texbin/xetex");
            p.exists().then(|| p.to_path_buf())
        });
    let want: Vec<(String, BTreeMap<u16, Vec<u16>>)> = match &oracle {
        Some(xetex) => {
            // xetex runs xdvipdfmx from its own directory
            let path = format!(
                "{}:{}",
                xetex.parent().unwrap().display(),
                std::env::var("PATH").unwrap_or_default()
            );
            eprintln!("oracle: {} -ini (xdvipdfmx)", xetex.display());
            let maps = run_and_read(xetex, &base.join("oracle"), Some(path));
            if measured {
                for ((b, m), (name, p)) in maps.iter().zip(&pinned) {
                    assert!(b.ends_with(name), "{b}");
                    let d = differences(p, m);
                    assert!(
                        d.is_empty(),
                        "the oracle's {b} map is not the pinned one on the same font file \
                         (a different xdvipdfmx?):\n{}",
                        d.join("\n")
                    );
                }
            }
            maps
        }
        None => {
            assert!(
                measured,
                "no oracle (set FLASHTEX_XETEX_ORACLE to TeX Live's xetex) and the font \
                 files are not the ones the pinned maps were measured on"
            );
            eprintln!("no oracle: the pinned maps (same font files)");
            pinned
                .iter()
                .map(|(n, m)| (n.to_string(), m.clone()))
                .collect()
        }
    };
    let got = run_and_read(
        Path::new(env!("CARGO_BIN_EXE_flashtex-xetex")),
        &base.join("flashtex"),
        None,
    );
    assert_eq!(got.len(), want.len());
    for ((wb, wm), (gb, gm)) in want.iter().zip(&got) {
        let name = &wb[wb.find('+').unwrap_or(0)..];
        assert!(gb.ends_with(name), "FlashTeX's font {gb}, xdvipdfmx's {wb}");
        assert!(!wm.is_empty(), "{wb}: the oracle's map is empty");
        let d = differences(wm, gm);
        assert!(d.is_empty(), "{gb}:\n{}", d.join("\n"));
        eprintln!("{gb}: {} ToUnicode entries, equal to xdvipdfmx's", gm.len());
    }
    let _ = std::fs::remove_dir_all(&base);
}
