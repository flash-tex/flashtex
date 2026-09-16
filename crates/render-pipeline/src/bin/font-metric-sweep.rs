//! Compare the shipped Latin Modern glyph metrics with local Computer Modern
//! 10pt TFM metrics. This is an evidence tool, not a rendering path.
//!
//! ```text
//! export CARGO_TARGET_DIR=/Users/dqi26/flashtex/target-metricsweep
//! export CARGO_BUILD_JOBS=4
//! cargo run --bin font-metric-sweep -- --output docs/evidence/font-metric-sweep-710.md
//! ```

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use flashtex_compiler::math::COMMAND_GLYPHS;
use flashtex_font_engine::GlyphId;
use flashtex_math_layout::cm;
use flashtex_math_layout::FontId as MathFontId;

use flashtex_render_pipeline::fonts::{FontSet, LoadedFace};
use flashtex_render_pipeline::mathfont::{MathFonts, MathSizes};
use flashtex_render_pipeline::mathtex::TexMathMetrics;
use flashtex_render_pipeline::tfm::{CharMetrics, Tfm};

const SIZE_PT: f64 = 10.0;
const PT_TO_BP: f64 = 72.27 / 72.0;
const POSITION_GATE_BP: f64 = 0.5;
const RULE_GATE_BP: f64 = 0.1;
const APPROACH_FACTOR: f64 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CmFamily {
    Roman,
    Italic,
    Symbol,
    Extension,
}

impl CmFamily {
    fn name(self) -> &'static str {
        match self {
            Self::Roman => "cmr10",
            Self::Italic => "cmmi10",
            Self::Symbol => "cmsy10",
            Self::Extension => "cmex10",
        }
    }

    fn font_id(self) -> MathFontId {
        MathFontId(match self {
            Self::Roman => 0,
            Self::Italic => 3,
            Self::Symbol => 6,
            Self::Extension => 9,
        })
    }

    fn tfm_file(self) -> &'static str {
        match self {
            Self::Roman => "cmr10.tfm",
            Self::Italic => "cmmi10.tfm",
            Self::Symbol => "cmsy10.tfm",
            Self::Extension => "cmex10.tfm",
        }
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    family: CmFamily,
    code: u8,
    ch: char,
    labels: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct Measurement {
    candidate: Candidate,
    gid: u16,
    face_name: String,
    cm: [f64; 4],
    lm: [f64; 4],
}

#[derive(Debug, Clone)]
struct Unmeasured {
    subject: String,
    detail: String,
}

#[derive(Debug, Clone)]
struct Args {
    font_dir: PathBuf,
    fontmath: Option<PathBuf>,
    tfm_dir: Option<PathBuf>,
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
enum Metric {
    Width,
    Height,
    Depth,
    Italic,
}

impl Metric {
    fn name(self) -> &'static str {
        match self {
            Self::Width => "advance width",
            Self::Height => "height",
            Self::Depth => "depth",
            Self::Italic => "italic correction",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Width => 0,
            Self::Height => 1,
            Self::Depth => 2,
            Self::Italic => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct Declaration {
    name: String,
    family: Option<CmFamily>,
    code: u8,
    large_family: Option<CmFamily>,
    large_code: Option<u8>,
    kind: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("font-metric-sweep: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let fontmath = match args.fontmath.as_ref() {
        Some(path) => path.clone(),
        None => kpsewhich("fontmath.ltx")?,
    };
    let declarations = parse_declarations(
        &fs::read_to_string(&fontmath).map_err(|e| format!("{}: {e}", fontmath.display()))?,
    );

    let mut tfm_paths = BTreeMap::new();
    for family in [
        CmFamily::Roman,
        CmFamily::Italic,
        CmFamily::Symbol,
        CmFamily::Extension,
    ] {
        let path = match &args.tfm_dir {
            Some(dir) => dir.join(family.tfm_file()),
            None => kpsewhich(family.tfm_file())?,
        };
        if !path.is_file() {
            return Err(format!("{} is not a file", path.display()));
        }
        tfm_paths.insert(family, path);
    }
    let mut tfms = BTreeMap::new();
    for (family, path) in &tfm_paths {
        let tfm = Tfm::load(path).map_err(|e| format!("{}: {e}", path.display()))?;
        tfms.insert(*family, tfm);
    }

    let fonts = FontSet::with_dirs(
        vec![args.font_dir.clone()],
        vec![args.tfm_dir.clone().unwrap_or_else(|| {
            tfm_paths
                .get(&CmFamily::Roman)
                .and_then(|path| path.parent().map(Path::to_path_buf))
                .expect("a TFM path has a parent")
        })],
    );
    let math_face = fonts
        .otf("latinmodern-math.otf")
        .map_err(|e| format!("latinmodern-math.otf: {e}"))?;
    let roman_face = fonts
        .otf("lmroman10-regular.otf")
        .map_err(|e| format!("lmroman10-regular.otf: {e}"))?;
    let math_fonts = MathFonts::new(
        math_face,
        MathSizes {
            text: SIZE_PT,
            script: 7.0,
            script_script: 5.0,
        },
    )
    .ok_or_else(|| "latinmodern-math.otf has no usable MATH table".to_string())?;
    let tex = TexMathMetrics::at_text_size(SIZE_PT, false, std::rc::Rc::new(math_fonts), &fonts)
        .ok_or_else(|| "the render-pipeline CM size table has no 10pt entry".to_string())?;

    let candidates = build_candidates(&declarations);
    let mut measurements = Vec::new();
    let mut unmeasured = Vec::new();
    for candidate in candidates.values() {
        let tfm = tfms.get(&candidate.family).expect("all families loaded");
        let Some(cm_metrics) = tfm.metrics(candidate.code) else {
            unmeasured.push(Unmeasured {
                subject: candidate_subject(candidate),
                detail: format!(
                    "{} has no character metric at slot 0x{:02X}",
                    candidate.family.name(),
                    candidate.code
                ),
            });
            continue;
        };
        let Some((face, gid)) =
            tex.otf_glyph(candidate.family.font_id(), candidate.code, candidate.ch)
        else {
            unmeasured.push(Unmeasured {
                subject: candidate_subject(candidate),
                detail: "the existing engine slot-to-Latin-Modern mapping returned no glyph"
                    .to_string(),
            });
            continue;
        };
        if gid == 0 {
            let detail = if candidate.family == CmFamily::Extension
                && matches!(candidate.ch, '\u{23DE}' | '\u{23DF}')
            {
                "the existing engine maps this brace assembly position to its empty placeholder (gid 0), not a font glyph"
            } else {
                "the existing engine mapping returned gid 0 (.notdef/empty placeholder), not a font glyph"
            };
            unmeasured.push(Unmeasured {
                subject: candidate_subject(candidate),
                detail: detail.to_string(),
            });
            continue;
        }
        let lm = loaded_metrics(&face, gid)?;
        let cm = tfm_metrics(cm_metrics);
        measurements.push(Measurement {
            candidate: candidate.clone(),
            gid,
            face_name: face.name.clone(),
            cm,
            lm,
        });
    }

    for (name, glyph) in COMMAND_GLYPHS {
        if *name == "varnothing" {
            unmeasured.push(Unmeasured {
                subject: format!("\\{name} {glyph}"),
                detail: "the engine gives this command the msbm/New Computer Modern Math sentinel path, not one of the four CM families".to_string(),
            });
            continue;
        }
        let command_label = format!("\\{name}");
        if matches!(*name, "hbar" | "angle" | "not" | "mapsto") {
            continue;
        }
        if glyph.chars().count() != 1 {
            unmeasured.push(Unmeasured {
                subject: format!("{command_label} {glyph}"),
                detail: "the compiler emits more than one Unicode scalar, so it is a composite rather than one glyph".to_string(),
            });
            continue;
        }
        if !measurements.iter().any(|m| {
            m.candidate
                .labels
                .iter()
                .any(|label| label.starts_with(&command_label))
        }) {
            unmeasured.push(Unmeasured {
                subject: format!("{command_label} {glyph}"),
                detail: "no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path".to_string(),
            });
        }
    }
    add_known_unmeasured(&mut unmeasured);
    unmeasured.sort_by(|a, b| a.subject.cmp(&b.subject).then(a.detail.cmp(&b.detail)));
    unmeasured.dedup_by(|a, b| a.subject == b.subject && a.detail == b.detail);

    let report = render_report(
        &args,
        &fontmath,
        &tfm_paths,
        &tfms,
        &fonts,
        &roman_face,
        &measurements,
        &unmeasured,
    )?;
    if let Some(path) = args.output {
        fs::write(&path, report).map_err(|e| format!("{}: {e}", path.display()))?;
        println!(
            "wrote {} ({} compared, {} unmeasured)",
            path.display(),
            measurements.len(),
            unmeasured.len()
        );
    } else {
        print!("{report}");
    }
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let repo_font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
    let mut args = env::args().skip(1);
    let mut result = Args {
        font_dir: repo_font_dir,
        fontmath: None,
        tfm_dir: None,
        output: None,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--font-dir" => result.font_dir = next_path(&mut args, "--font-dir")?,
            "--fontmath" => result.fontmath = Some(next_path(&mut args, "--fontmath")?),
            "--tfm-dir" => result.tfm_dir = Some(next_path(&mut args, "--tfm-dir")?),
            "--output" => result.output = Some(next_path(&mut args, "--output")?),
            "-h" | "--help" => {
                println!("usage: font-metric-sweep [--font-dir DIR] [--tfm-dir DIR] [--fontmath PATH] [--output PATH]");
                println!(
                    "defaults: shipped apps/mac/Fonts, kpsewhich for four CM TFMs and fontmath.ltx"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other:?}; use --help")),
        }
    }
    if !result.font_dir.is_dir() {
        return Err(format!(
            "font directory does not exist: {}",
            result.font_dir.display()
        ));
    }
    Ok(result)
}

fn next_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn kpsewhich(name: &str) -> Result<PathBuf, String> {
    let output = Command::new("kpsewhich")
        .arg(name)
        .output()
        .map_err(|e| format!("kpsewhich {name}: {e}"))?;
    if !output.status.success() {
        return Err(format!("kpsewhich {name} exited with {}", output.status));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err(format!("kpsewhich {name} returned no path"));
    }
    Ok(PathBuf::from(path))
}

fn build_candidates(declarations: &[Declaration]) -> BTreeMap<(CmFamily, u8, char), Candidate> {
    let command_chars: BTreeMap<String, char> = COMMAND_GLYPHS
        .iter()
        .filter_map(|(name, glyph)| {
            (glyph.chars().count() == 1).then(|| (name.to_string(), glyph.chars().next().unwrap()))
        })
        .collect();
    let mut out = BTreeMap::new();

    for ch in "!*+,-./:;<=>?@[]|\\()0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
        .chars()
    {
        add_char_candidates(&mut out, ch, "ASCII math".to_string());
    }
    for (name, glyph) in COMMAND_GLYPHS {
        if glyph.chars().count() == 1 {
            add_char_candidates(&mut out, glyph.chars().next().unwrap(), format!("\\{name}"));
        }
    }

    // These are compiler-supported structural delimiter names, not all of
    // them entries in COMMAND_GLYPHS.
    for (name, ch) in [
        ("lbrace", '{'),
        ("rbrace", '}'),
        ("backslash", '\\'),
        ("vert", '|'),
        ("Vert", '\u{2016}'),
        ("langle", '\u{27E8}'),
        ("rangle", '\u{27E9}'),
        ("lfloor", '\u{230A}'),
        ("rfloor", '\u{230B}'),
        ("lceil", '\u{2308}'),
        ("rceil", '\u{2309}'),
        ("mid", '\u{2223}'),
        ("parallel", '\u{2225}'),
    ] {
        add_char_candidates(&mut out, ch, format!("\\{name}"));
    }

    for declaration in declarations {
        let Some(ch) = command_char(&declaration.name, &command_chars) else {
            continue;
        };
        let label = format!("\\{} (fontmath.ltx)", declaration.name);
        if declaration.kind == "delimiter" {
            if let (Some(family), Some(code)) = (declaration.family, Some(declaration.code)) {
                add_candidate(&mut out, family, code, ch, label.clone());
            }
            if let (Some(family), Some(code)) = (declaration.large_family, declaration.large_code) {
                add_candidate(&mut out, family, code, ch, label);
            }
        } else if declaration.kind == "symbol" {
            if let Some(family) = declaration.family {
                add_candidate(&mut out, family, declaration.code, ch, label);
            }
        } else if declaration.kind == "accent" {
            if let Some(family) = declaration.family {
                add_candidate(&mut out, family, declaration.code, ch, label);
            }
        }
    }

    // fontmath.ltx declares these as private cmex pieces. The existing
    // pipeline maps their two brace orientations to the LM brace assembly.
    for code in 0x7Au8..=0x7Du8 {
        for ch in ['\u{23DE}', '\u{23DF}'] {
            add_candidate(
                &mut out,
                CmFamily::Extension,
                code,
                ch,
                "private cmex brace piece".to_string(),
            );
        }
    }
    add_candidate(
        &mut out,
        CmFamily::Extension,
        0x70,
        '\u{221A}',
        "fontmath.ltx radical".to_string(),
    );
    add_candidate(
        &mut out,
        CmFamily::Symbol,
        0x70,
        '\u{221A}',
        "fontmath.ltx radical".to_string(),
    );

    out
}

fn command_char(name: &str, command_chars: &BTreeMap<String, char>) -> Option<char> {
    command_chars.get(name).copied().or(match name {
        "imath" => Some('\u{0131}'),
        "jmath" => Some('\u{0237}'),
        "neg" | "lnot" => Some('\u{00AC}'),
        "hat" => Some('\u{02C6}'),
        "bar" => Some('\u{00AF}'),
        "vec" => Some('\u{20D7}'),
        "tilde" => Some('\u{02DC}'),
        "dot" => Some('\u{02D9}'),
        "ddot" => Some('\u{00A8}'),
        "check" => Some('\u{02C7}'),
        "breve" => Some('\u{02D8}'),
        "acute" => Some('\u{00B4}'),
        "grave" => Some('`'),
        "widehat" => Some('\u{0302}'),
        "widetilde" => Some('\u{0303}'),
        "flat" => Some('\u{266D}'),
        "natural" => Some('\u{266E}'),
        "sharp" => Some('\u{266F}'),
        "clubsuit" => Some('\u{2663}'),
        "diamondsuit" => Some('\u{2662}'),
        "heartsuit" => Some('\u{2661}'),
        "spadesuit" => Some('\u{2660}'),
        "lbrace" => Some('{'),
        "rbrace" => Some('}'),
        "backslash" => Some('\\'),
        _ => None,
    })
}

fn add_char_candidates(
    out: &mut BTreeMap<(CmFamily, u8, char), Candidate>,
    ch: char,
    label: String,
) {
    if let Some((family, code)) = cm::symbol_slot(ch) {
        add_candidate(out, cm_family(family), code, ch, label.clone());
    }
    if let Some(((family, code), large)) = cm::delimiter_slot(ch) {
        add_candidate(out, cm_family(family), code, ch, label.clone());
        add_candidate(out, CmFamily::Extension, large, ch, label);
    }
}

fn add_candidate(
    out: &mut BTreeMap<(CmFamily, u8, char), Candidate>,
    family: CmFamily,
    code: u8,
    ch: char,
    label: String,
) {
    out.entry((family, code, ch))
        .or_insert_with(|| Candidate {
            family,
            code,
            ch,
            labels: BTreeSet::new(),
        })
        .labels
        .insert(label);
}

fn cm_family(family: cm::Family) -> CmFamily {
    match family {
        cm::Family::Roman => CmFamily::Roman,
        cm::Family::Italic => CmFamily::Italic,
        cm::Family::Symbol => CmFamily::Symbol,
        cm::Family::Extension => CmFamily::Extension,
    }
}

fn parse_declarations(source: &str) -> Vec<Declaration> {
    let source: String = source
        .lines()
        .map(|line| line.split('%').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut result = Vec::new();
    for (macro_name, kind, count) in [
        ("\\DeclareMathSymbol", "symbol", 4usize),
        ("\\DeclareMathDelimiter", "delimiter", 6),
        ("\\DeclareMathAccent", "accent", 4),
    ] {
        let mut offset = 0;
        while let Some(relative) = source[offset..].find(macro_name) {
            let start = offset + relative + macro_name.len();
            let Some((args, _)) = braced_args(&source, start, count) else {
                offset = start;
                continue;
            };
            let Some(name) = args
                .first()
                .and_then(|a| a.trim().strip_prefix('\\'))
                .map(str::to_string)
            else {
                offset = start;
                continue;
            };
            let family = args.get(2).and_then(|a| parse_family(a));
            let Some(code) = args.get(3).and_then(|a| parse_slot(a)) else {
                offset = start;
                continue;
            };
            let (large_family, large_code) = if kind == "delimiter" {
                (
                    args.get(4).and_then(|a| parse_family(a)),
                    args.get(5).and_then(|a| parse_slot(a)),
                )
            } else {
                (None, None)
            };
            result.push(Declaration {
                name,
                family,
                code,
                large_family,
                large_code,
                kind,
            });
            offset = start;
        }
    }
    result
}

fn braced_args(source: &str, mut offset: usize, count: usize) -> Option<(Vec<String>, usize)> {
    let mut args = Vec::with_capacity(count);
    for _ in 0..count {
        while source
            .as_bytes()
            .get(offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            offset += 1;
        }
        if source.as_bytes().get(offset) != Some(&b'{') {
            return None;
        }
        let start = offset + 1;
        let mut depth = 1usize;
        let mut cursor = start;
        while let Some(&byte) = source.as_bytes().get(cursor) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        args.push(source[start..cursor].to_string());
                        offset = cursor + 1;
                        break;
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        if depth != 0 {
            return None;
        }
    }
    Some((args, offset))
}

fn parse_family(value: &str) -> Option<CmFamily> {
    match value.trim() {
        "operators" => Some(CmFamily::Roman),
        "letters" => Some(CmFamily::Italic),
        "symbols" => Some(CmFamily::Symbol),
        "largesymbols" => Some(CmFamily::Extension),
        _ => None,
    }
}

fn parse_slot(value: &str) -> Option<u8> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('"') {
        return u8::from_str_radix(hex.trim(), 16).ok();
    }
    if let Some(rest) = value.strip_prefix('`') {
        return rest
            .trim_start_matches('\\')
            .chars()
            .next()
            .map(|c| c as u32 as u8);
    }
    value.parse().ok()
}

fn tfm_metrics(metrics: CharMetrics) -> [f64; 4] {
    [metrics.width, metrics.height, metrics.depth, metrics.italic]
        .map(|value| Tfm::pt(value, SIZE_PT))
}

fn loaded_metrics(face: &LoadedFace, gid: u16) -> Result<[f64; 4], String> {
    let gid = GlyphId(gid);
    let advance = face
        .face()
        .advance(gid)
        .map_err(|e| format!("{} gid {} advance: {e}", face.name, gid.0))?;
    let bounds = face.bounds(gid, None);
    let height = if bounds.empty {
        0.0
    } else {
        face.pt(i64::from(bounds.y_max.max(0)), SIZE_PT)
    };
    let depth = if bounds.empty {
        0.0
    } else {
        face.pt(i64::from((-bounds.y_min).max(0)), SIZE_PT)
    };
    let italic = face
        .math()
        .map(|math| face.pt(i64::from(math.italics_correction(gid)), SIZE_PT))
        .unwrap_or(0.0);
    Ok([face.pt(i64::from(advance), SIZE_PT), height, depth, italic])
}

fn candidate_subject(candidate: &Candidate) -> String {
    format!(
        "{} 0x{:02X} {}",
        candidate.family.name(),
        candidate.code,
        display_char(candidate.ch)
    )
}

fn display_char(ch: char) -> String {
    match ch {
        ' ' => "U+0020 SPACE".to_string(),
        '`' => "U+0060 GRAVE ACCENT".to_string(),
        '\u{0302}' => "U+0302 COMBINING CIRCUMFLEX".to_string(),
        '\u{0303}' => "U+0303 COMBINING TILDE".to_string(),
        _ if ch.is_control() => format!("U+{:04X}", ch as u32),
        _ => format!("U+{:04X} `{ch}`", ch as u32),
    }
}

fn add_known_unmeasured(unmeasured: &mut Vec<Unmeasured>) {
    for (subject, detail) in [
        ("\\hbar ℏ", "fontmath.ltx defines a composite overprint, not one CM TFM glyph"),
        ("\\angle ∠", "fontmath.ltx defines a constructed box, not one CM TFM glyph"),
        ("\\not U+0338", "the engine uses a zero-width overprint before the following relation"),
        ("\\mapsto ↦", "plain TeX composes a zero-width mapstochar with an arrow; no single four-family CM glyph is declared"),
    ] {
        unmeasured.push(Unmeasured {
            subject: subject.to_string(),
            detail: detail.to_string(),
        });
    }
}

fn render_report(
    args: &Args,
    fontmath: &Path,
    tfm_paths: &BTreeMap<CmFamily, PathBuf>,
    tfms: &BTreeMap<CmFamily, Tfm>,
    fonts: &FontSet,
    roman_face: &std::rc::Rc<LoadedFace>,
    measurements: &[Measurement],
    unmeasured: &[Unmeasured],
) -> Result<String, String> {
    let math_face = fonts
        .by_name("latinmodern-math")
        .ok_or_else(|| "Latin Modern Math was loaded but not retained by FontSet".to_string())?;
    let mut report = String::new();
    writeln!(
        report,
        "# Issue #710: Latin Modern versus Computer Modern glyph metrics"
    )
    .unwrap();
    writeln!(report, "").unwrap();
    writeln!(
        report,
        "Status: measurement only. No layout or rendering code was changed."
    )
    .unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "This sweep compares the actual 10pt Latin Modern faces loaded by FlashTeX with the local pdfTeX Computer Modern TFM files. Every reported delta is `Latin Modern - Computer Modern`. Values are points at 10pt design size. Relative values use the Computer Modern advance as the denominator; a zero Computer Modern advance is reported as not applicable rather than divided or estimated.").unwrap();
    writeln!(report, "").unwrap();

    writeln!(report, "## Result at a glance").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "- Compared glyph rows: **{}**.", measurements.len()).unwrap();
    let family_count = |family| {
        measurements
            .iter()
            .filter(|row| row.candidate.family == family)
            .count()
    };
    writeln!(
        report,
        "- Rows by family: **cmr10 {}**, **cmmi10 {}**, **cmsy10 {}**, **cmex10 {}**.",
        family_count(CmFamily::Roman),
        family_count(CmFamily::Italic),
        family_count(CmFamily::Symbol),
        family_count(CmFamily::Extension)
    )
    .unwrap();
    writeln!(
        report,
        "- Unmeasured or out-of-scope entries: **{}**.",
        unmeasured.len()
    )
    .unwrap();
    writeln!(
        report,
        "- Rule gate: **{:.1} bp**; position gate: **{:.1} bp**.",
        RULE_GATE_BP, POSITION_GATE_BP
    )
    .unwrap();
    writeln!(report, "- “Approaches” means at least **{}%** of a gate: {:.2} bp for position and {:.2} bp for rules.", APPROACH_FACTOR * 100.0, POSITION_GATE_BP * APPROACH_FACTOR, RULE_GATE_BP * APPROACH_FACTOR).unwrap();
    if let Some(row) = measurements.iter().find(|row| {
        row.candidate
            .labels
            .iter()
            .any(|label| label.starts_with("\\ell"))
    }) {
        let delta = row.lm[0] - row.cm[0];
        let box_delta = effective_delta(row);
        writeln!(report, "- `\\ell` is present: raw advance CM {:.5} pt, LM {:.5} pt, delta {:+.5} pt; italic correction CM {:.5} pt, LM {:.5} pt; effective math char-box delta **{:+.5} pt = {:+.5} bp**.", row.cm[0], row.lm[0], delta, row.cm[3], row.lm[3], box_delta, box_delta * PT_TO_BP).unwrap();
    } else {
        writeln!(
            report,
            "- `\\ell` was not measured; see the unmeasured section."
        )
        .unwrap();
    }
    writeln!(report, "").unwrap();

    writeln!(report, "## Sources and method").unwrap();
    writeln!(report, "Scope: CMR rows are the math family-0 inventory. OT1 text-only slots and ligature programs are not included because this engine's normal text path uses T1 Latin Modern metrics, not cmr10.").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "The OTF faces, advances, CFF bounds, and MATH italic corrections are loaded through the existing `render-pipeline` `FontSet`, `LoadedFace`, and `TexMathMetrics::otf_glyph` path. The CM metric numbers are parsed by the existing `render-pipeline::tfm::Tfm` wrapper, which delegates to the shared `font-resources` TFM reader. `fontmath.ltx` supplies TeX's family/slot declarations, including `\\ell`; it is not used to obtain metric numbers.").unwrap();
    writeln!(
        report,
        "Font directory supplied to `FontSet`: `{}`.",
        args.font_dir.display()
    )
    .unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "| side | source | SHA-256 | notes |").unwrap();
    writeln!(report, "| --- | --- | --- | --- |").unwrap();
    writeln!(
        report,
        "| Latin Modern CMR path | `{}` | `{}` | `lmroman10-regular.otf`, used for cmr10 math-family rows |",
        path_for_report(roman_face.path.as_deref()),
        hex(&roman_face.sha256)
    )
    .unwrap();
    writeln!(report, "| Latin Modern math | `{}` | `{}` | `latinmodern-math.otf`, used for cmmi10/cmsy10/cmex10 rows |", path_for_report(math_face.path.as_deref()), hex(&math_face.sha256)).unwrap();
    for family in [
        CmFamily::Roman,
        CmFamily::Italic,
        CmFamily::Symbol,
        CmFamily::Extension,
    ] {
        let tfm = tfms.get(&family).expect("all TFM sources loaded");
        let path = tfm_paths.get(&family).expect("all TFM paths loaded");
        writeln!(report, "| Computer Modern | `{}` | `{}` | `{}, design size {:.5} pt; TFM bytes are read through the shared parser |", path_for_report(Some(path)), tfm.sha256(), family.tfm_file(), tfm.design_size_pt).unwrap();
    }
    writeln!(
        report,
        "| TeX declarations | `{}` | n/a | `kpsewhich fontmath.ltx`; declarations only |",
        path_for_report(Some(fontmath))
    )
    .unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "The LM height is the loaded face's existing CFF glyph bound above the baseline; depth is the magnitude of its bound below the baseline. LM italic correction comes from the MATH table. The roman face has no MATH table, so its LM italic correction is zero; the CM value is still read from `cmr10.tfm`. Empty assembly placeholders are listed as unmeasured rather than treating `.notdef` as a glyph.").unwrap();
    writeln!(report, "").unwrap();

    write_top_tables(&mut report, measurements);
    write_gate_table(&mut report, measurements);
    write_full_table(&mut report, measurements);
    write_unmeasured(&mut report, unmeasured);
    Ok(report)
}

fn write_top_tables(report: &mut String, measurements: &[Measurement]) {
    writeln!(report, "## Largest divergences").unwrap();
    writeln!(report, "Assembly-piece note: private cmex brace-piece rows are source-piece comparisons; Latin Modern Math's horizontal assemblies are not independent glyph metrics for those pieces.").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "Raw advance width is ranked first, followed by the effective math char-box width (advance plus italic correction), because that is the width used by clean character boxes such as fraction numerators and denominators. The component tables then rank height, depth, and italic correction too.").unwrap();
    writeln!(report, "").unwrap();
    write_ranked(
        report,
        "Largest absolute advance differences",
        measurements,
        Metric::Width,
        false,
        20,
        false,
    );
    write_ranked(
        report,
        "Largest relative advance differences",
        measurements,
        Metric::Width,
        true,
        20,
        false,
    );
    write_ranked(
        report,
        "Largest absolute effective char-box width differences",
        measurements,
        Metric::Width,
        false,
        20,
        true,
    );
    write_ranked(
        report,
        "Largest relative effective char-box width differences",
        measurements,
        Metric::Width,
        true,
        20,
        true,
    );
    write_ranked(
        report,
        "Largest absolute component differences",
        measurements,
        Metric::Width,
        false,
        0,
        false,
    );
    write_ranked(
        report,
        "Largest relative component differences",
        measurements,
        Metric::Width,
        true,
        0,
        false,
    );
}

fn write_ranked(
    report: &mut String,
    title: &str,
    measurements: &[Measurement],
    focus: Metric,
    relative: bool,
    limit: usize,
    effective: bool,
) {
    let mut rows: Vec<(f64, usize, Metric)> = Vec::new();
    let metrics = if limit == 0 {
        [Metric::Width, Metric::Height, Metric::Depth, Metric::Italic].as_slice()
    } else {
        std::slice::from_ref(&focus)
    };
    for (index, row) in measurements.iter().enumerate() {
        for &metric in metrics {
            let delta = metric_delta(row, metric, effective);
            let value = if relative {
                if row.cm[0] == 0.0 {
                    continue;
                } else {
                    delta / row.cm[0]
                }
            } else {
                delta
            };
            rows.push((value.abs(), index, metric));
        }
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    let take = if limit == 0 { 20 } else { limit };
    writeln!(report, "### {title}").unwrap();
    writeln!(report, "").unwrap();
    writeln!(
        report,
        "| rank | metric | family/slot | glyph | label | delta | denominator fraction |"
    )
    .unwrap();
    writeln!(report, "| ---: | --- | --- | --- | --- | ---: | ---: |").unwrap();
    for (rank, &(_, index, metric)) in rows.iter().take(take).enumerate() {
        let row = &measurements[index];
        let delta = metric_delta(row, metric, effective);
        let fraction = if row.cm[0] == 0.0 {
            None
        } else {
            Some(delta / row.cm[0])
        };
        let delta_display = if relative {
            format!("{delta:+.5} pt")
        } else {
            format!("{delta:+.5} pt ({:+.5} bp)", delta * PT_TO_BP)
        };
        writeln!(
            report,
            "| {} | {} | {} 0x{:02X} | {} | {} | {} | {} |",
            rank + 1,
            metric_name(metric, effective),
            row.candidate.family.name(),
            row.candidate.code,
            display_char(row.candidate.ch),
            labels(&row.candidate),
            delta_display,
            format_fraction(fraction)
        )
        .unwrap();
    }
    writeln!(report, "").unwrap();
}

fn write_gate_table(report: &mut String, measurements: &[Measurement]) {
    let mut rows: Vec<&Measurement> = measurements
        .iter()
        .filter(|row| {
            let bp = effective_delta(row).abs() * PT_TO_BP;
            bp >= RULE_GATE_BP * APPROACH_FACTOR
        })
        .collect();
    rows.sort_by(|a, b| {
        effective_delta(b)
            .abs()
            .partial_cmp(&effective_delta(a).abs())
            .unwrap_or(Ordering::Equal)
    });
    let rule_exceeds = rows
        .iter()
        .filter(|row| gate_status(effective_delta(row) * PT_TO_BP, RULE_GATE_BP) == "EXCEEDS")
        .count();
    let rule_approaches = rows
        .iter()
        .filter(|row| gate_status(effective_delta(row) * PT_TO_BP, RULE_GATE_BP) == "approaches")
        .count();
    let position_exceeds = rows
        .iter()
        .filter(|row| gate_status(effective_delta(row) * PT_TO_BP, POSITION_GATE_BP) == "EXCEEDS")
        .count();
    let position_approaches = rows
        .iter()
        .filter(|row| {
            gate_status(effective_delta(row) * PT_TO_BP, POSITION_GATE_BP) == "approaches"
        })
        .count();
    writeln!(report, "## Gate watch list").unwrap();
    writeln!(report, "Private cmex brace-piece rows are source-piece evidence, not a claim that an LM assembly piece is painted as an independent glyph.").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "This list is based on the effective math char-box width (advance plus italic correction), because that is the width used for clean character boxes such as fraction numerators and denominators. Raw advance and italic correction remain separate in the complete table. `approaches` is the sweep's explicit 80% convention; it is not a project gate. A row can be below the position threshold while already exceeding the stricter rule threshold.").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "**What `EXCEEDS` means here.** The {RULE_GATE_BP:.1} bp / {POSITION_GATE_BP:.1} bp thresholds are the same generic acceptance tolerances this codebase already applies when comparing rendered rule widths and box positions against pdfTeX output (see `crates/render-pipeline/tests/*_oracle.rs`); this sweep reuses them as a screening cutoff on each glyph's raw metric delta, in isolation. An `EXCEEDS` mark is not a claim that this glyph draws a mis-sized TeX `\\rule`, and not a claim that any test currently fails — it means this glyph's own LM-versus-CM divergence, by itself, is larger than the margin those tests require. Whether that divergence ever becomes a visible or gate-breaking difference depends on whether, and how, this specific glyph's metric feeds a rendered rule or box width in some construct, the way `\\ell`'s italic correction feeds the vinculum width in `\\sqrt{{\\ell/\\ell}}` (issue #710). Most rows below are ordinary letters and symbols (`\\forall`, `\\Im`, `\\prime`, the cmex10 arrow and brace-piece slots) that are never used to draw a rule; for those, `EXCEEDS` flags a candidate worth checking against a specific construct, not a known rendering regression. Of the {} rows below, {rule_exceeds} exceed the rule threshold ({rule_approaches} more approach it) and {position_exceeds} exceed the position threshold ({position_approaches} more approach it); those counts describe this screening pass, not confirmed defects.", rows.len()).unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "| family/slot | glyph | labels | raw Δw pt | raw Δw bp | effective Δ char-box pt | effective Δ bp | effective fraction of CM advance | position 0.5 bp | rule 0.1 bp |").unwrap();
    writeln!(
        report,
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- | --- |"
    )
    .unwrap();
    for row in rows {
        let delta = row.lm[0] - row.cm[0];
        let box_delta = effective_delta(row);
        let bp = box_delta * PT_TO_BP;
        writeln!(
            report,
            "| {} 0x{:02X} | {} | {} | {delta:+.5} | {:+.5} | {box_delta:+.5} | {bp:+.5} | {} | {} | {} |",
            row.candidate.family.name(),
            row.candidate.code,
            display_char(row.candidate.ch),
            labels(&row.candidate),
            delta * PT_TO_BP,
            format_fraction((row.cm[0] != 0.0).then_some(box_delta / row.cm[0])),
            gate_status(bp, POSITION_GATE_BP),
            gate_status(bp, RULE_GATE_BP)
        )
        .unwrap();
    }
    writeln!(report, "").unwrap();
}

fn gate_status(delta_bp: f64, gate_bp: f64) -> &'static str {
    let value = delta_bp.abs();
    if value >= gate_bp {
        "EXCEEDS"
    } else if value >= gate_bp * APPROACH_FACTOR {
        "approaches"
    } else {
        "below"
    }
}

fn write_full_table(report: &mut String, measurements: &[Measurement]) {
    writeln!(report, "## Complete measured table").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "All compared rows are included below, including rows with zero or near-zero deltas. `CM → LM` columns are points; delta fractions use the raw CM advance. Effective char-box width is raw advance plus italic correction.").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "| family/slot | glyph | labels | gid/face | width CM → LM pt | Δw pt | Δw bp | Δw/adv | effective char-box CM → LM pt | Δbox bp | Δbox/adv | height CM → LM pt | Δh pt | Δh/adv | depth CM → LM pt | Δd pt | Δd/adv | italic CM → LM pt | Δic pt | Δic/adv |").unwrap();
    writeln!(report, "| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |").unwrap();
    for row in measurements {
        let d = deltas(row);
        let cm_box = effective_width(row.cm);
        let lm_box = effective_width(row.lm);
        let box_delta = lm_box - cm_box;
        writeln!(report, "| {} 0x{:02X} | {} | {} | {} / {} | {} → {} | {:+.5} | {:+.5} | {} | {} → {} | {:+.5} | {} | {} → {} | {:+.5} | {} | {} → {} | {:+.5} | {} | {} → {} | {:+.5} | {} |", row.candidate.family.name(), row.candidate.code, display_char(row.candidate.ch), labels(&row.candidate), row.gid, row.face_name, fmt(row.cm[0]), fmt(row.lm[0]), d[0], d[0] * PT_TO_BP, format_fraction((row.cm[0] != 0.0).then_some(d[0] / row.cm[0])), fmt(cm_box), fmt(lm_box), box_delta * PT_TO_BP, format_fraction((row.cm[0] != 0.0).then_some(box_delta / row.cm[0])), fmt(row.cm[1]), fmt(row.lm[1]), d[1], format_fraction((row.cm[0] != 0.0).then_some(d[1] / row.cm[0])), fmt(row.cm[2]), fmt(row.lm[2]), d[2], format_fraction((row.cm[0] != 0.0).then_some(d[2] / row.cm[0])), fmt(row.cm[3]), fmt(row.lm[3]), d[3], format_fraction((row.cm[0] != 0.0).then_some(d[3] / row.cm[0]))).unwrap();
    }
    writeln!(report, "").unwrap();
}

fn write_unmeasured(report: &mut String, unmeasured: &[Unmeasured]) {
    writeln!(report, "## Unmeasured or out-of-scope rows").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "These are not estimates. They are listed because a source or mapping needed for a four-family comparison was absent, or because the engine uses a composite/secondary-face path. No number was substituted.").unwrap();
    writeln!(report, "").unwrap();
    writeln!(report, "| subject | reason |").unwrap();
    writeln!(report, "| --- | --- |").unwrap();
    for item in unmeasured {
        writeln!(report, "| {} | {} |", item.subject, item.detail).unwrap();
    }
    writeln!(report, "").unwrap();
}

fn deltas(row: &Measurement) -> [f64; 4] {
    [0, 1, 2, 3].map(|index| row.lm[index] - row.cm[index])
}

fn effective_width(metrics: [f64; 4]) -> f64 {
    metrics[0] + metrics[3]
}

fn effective_delta(row: &Measurement) -> f64 {
    effective_width(row.lm) - effective_width(row.cm)
}

fn metric_delta(row: &Measurement, metric: Metric, effective: bool) -> f64 {
    if effective && matches!(metric, Metric::Width) {
        effective_delta(row)
    } else {
        row.lm[metric.index()] - row.cm[metric.index()]
    }
}

fn metric_name(metric: Metric, effective: bool) -> &'static str {
    if effective && matches!(metric, Metric::Width) {
        "effective char-box width"
    } else {
        metric.name()
    }
}

fn labels(candidate: &Candidate) -> String {
    candidate
        .labels
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

fn fmt(value: f64) -> String {
    format!("{value:.5}")
}

fn format_fraction(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |value| format!("{value:+.5}"))
}

fn path_for_report(path: Option<&Path>) -> String {
    path.map_or_else(
        || "unavailable".to_string(),
        |path| path.display().to_string(),
    )
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
