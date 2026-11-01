//! `flashtex-pdf-exact`: exercise and audit the exact export route.
//!
//! ```text
//! flashtex-pdf-exact reemit REF.pdf OUT.pdf      # rebuild REF through the exact API
//! flashtex-pdf-exact classify A.pdf B.pdf        # classify every difference
//! flashtex-pdf-exact dump X.pdf                  # pages, fonts, content operators
//! flashtex-pdf-exact from-v2 LIST.json --out OUT.pdf [--font-dir DIR]... [--project-root DIR]
//!                                                # rendering-v2 display list -> exact PDF
//! ```
//!
//! `from-v2` consumes the `display_list` envelope of `flashtex-render --v2`
//! (see `crate::v2`): glyph runs by original glyph id at exact tick
//! positions, fonts resolved by content hash and embedded as GID-preserving
//! subsets, typed rules. It prints one `note:` line per font and a summary,
//! runs the structural self-check, and exits 1 on any unsupported input.
//!
//! `reemit` reads a finished PDF (this crate's or pdfTeX's), carries its
//! page sizes, decoded content streams, and font resources into an
//! `ExactDocument`, renders it through `render_exact`, runs the structural
//! self-check, and writes the result. `classify` prints one line per
//! finding with a category tag and a final category summary; exit status
//! is 0 when the content operators and font programs are identical on
//! every page, 3 otherwise, so scripts can gate on it.

use flashtex_pdf::compare;
use flashtex_pdf::reader::PdfFile;
use std::process::ExitCode;

const USAGE: &str = "usage: flashtex-pdf-exact reemit REF.pdf OUT.pdf | classify A.pdf B.pdf | dump X.pdf | from-v2 LIST.json --out OUT.pdf [--font-dir DIR]... [--project-root DIR]";

fn read_pdf(path: &str) -> Result<PdfFile, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    PdfFile::parse(&bytes).map_err(|e| format!("{path}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["reemit", reference, out] => reemit(reference, out),
        ["classify", a, b] => classify(a, b),
        ["dump", x] => dump(x),
        ["from-v2", rest @ ..] => from_v2(rest),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn from_v2(args: &[&str]) -> Result<u8, String> {
    let mut input = None;
    let mut out = None;
    let mut options = flashtex_pdf::v2::V2Options::default();
    let mut project_root: Option<std::path::PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--out" => {
                out = args.get(i + 1).map(|s| s.to_string());
                i += 2;
            }
            "--project-root" => {
                project_root = Some(std::path::PathBuf::from(
                    args.get(i + 1).ok_or("--project-root needs a directory")?,
                ));
                i += 2;
            }
            "--font-dir" => {
                options.font_dirs.push(std::path::PathBuf::from(
                    args.get(i + 1).ok_or("--font-dir needs a path")?,
                ));
                i += 2;
            }
            a if a.starts_with('-') => return Err(format!("unknown option {a}\n{USAGE}")),
            a => {
                if input.replace(a.to_string()).is_some() {
                    return Err(format!("only one display list is accepted\n{USAGE}"));
                }
                i += 1;
            }
        }
    }
    let (Some(input), Some(out)) = (input, out) else {
        return Err(format!(
            "from-v2 needs LIST.json and --out OUT.pdf\n{USAGE}"
        ));
    };
    let (doc, report) = flashtex_pdf::v2::from_v2_file_rooted(
        std::path::Path::new(&input),
        &options,
        project_root.as_deref(),
    )?;
    let rendered = flashtex_pdf::exact::render_exact(&doc).map_err(|e| e.to_string())?;
    flashtex_pdf::verify::check_structure(&rendered.bytes)
        .map_err(|e| format!("generated PDF failed self-check: {e}"))?;
    std::fs::write(&out, &rendered.bytes).map_err(|e| format!("{out}: {e}"))?;
    for fnote in &report.fonts {
        eprintln!(
            "note: /{} {} from {}{}: {} glyph(s), {:?}, program {} bytes{}",
            fnote.resource,
            fnote.postscript_name,
            fnote.path.display(),
            if fnote.face_index == 0 {
                String::new()
            } else {
                format!(" face {}", fnote.face_index)
            },
            fnote.glyphs,
            fnote.outcome,
            fnote.program_bytes,
            fnote
                .note
                .as_deref()
                .map_or(String::new(), |n| format!(" ({n})"))
        );
    }
    for n in &report.notes {
        eprintln!("note: {n}");
    }
    for d in &report.diagnostics {
        eprintln!("diagnostic (from the display list): {d}");
    }
    eprintln!(
        "note: {} page(s), {} glyph run(s), {} glyph(s) ({} continued at the natural advance, {} with an exact TJ kern), {} rule(s), {} image(s) ({} XObject(s)), {} bytes -> {out}",
        report.pages,
        report.runs,
        report.glyphs,
        report.joined_glyphs,
        report.kerned_glyphs,
        report.rules,
        report.images,
        report.image_resources,
        rendered.bytes.len()
    );
    eprintln!(
        "note: searchable text: {} word gap(s) of >= {}/1000 em, {} ambiguous gap(s) in [{}, {})/1000 em (docs/proposals/pdf-searchable-text.md)",
        report.word_gaps,
        flashtex_pdf::v2::WORD_GAP_EM,
        report.ambiguous_gaps,
        flashtex_pdf::v2::CHAR_GAP_EM,
        flashtex_pdf::v2::WORD_GAP_EM
    );
    Ok(0)
}

fn reemit(reference: &str, out: &str) -> Result<u8, String> {
    let file = read_pdf(reference)?;
    let doc = compare::reemit(&file)?;
    let rendered = flashtex_pdf::exact::render_exact(&doc).map_err(|e| e.to_string())?;
    flashtex_pdf::verify::check_structure(&rendered.bytes)
        .map_err(|e| format!("generated PDF failed self-check: {e}"))?;
    std::fs::write(out, &rendered.bytes).map_err(|e| format!("{out}: {e}"))?;
    eprintln!(
        "note: re-emitted {} page(s), {} font resource(s), {} bytes -> {out}",
        doc.pages.len(),
        doc.fonts.len(),
        rendered.bytes.len()
    );
    for (name, font) in &doc.fonts {
        if let Some((len, sha)) = font.program_identity() {
            eprintln!("note: /{name} program {len} bytes sha256 {sha}");
        }
    }
    Ok(0)
}

fn classify(a: &str, b: &str) -> Result<u8, String> {
    let fa = read_pdf(a)?;
    let fb = read_pdf(b)?;
    let report = compare::classify(&fa, &fb, a, b);
    print!("{}", report.text());
    let gate = report.content_ops_identical.iter().all(|&x| x)
        && !report.content_ops_identical.is_empty()
        && report.font_program_identical.values().all(|&x| x);
    Ok(if gate { 0 } else { 3 })
}

fn dump(path: &str) -> Result<u8, String> {
    let f = read_pdf(path)?;
    let x = &f.features;
    println!(
        "version {} xref-stream {} object-streams {} objects {} filters {:?} id {}",
        x.version, x.xref_stream, x.object_streams, x.object_count, x.filters, x.has_id
    );
    if let Some(info) = f.info() {
        println!(
            "info {}",
            flashtex_pdf::reader::render(&flashtex_pdf::reader::Obj::Dict(info.clone()))
        );
    }
    for (i, page) in f.pages()?.iter().enumerate() {
        println!(
            "page {} MediaBox {}",
            i + 1,
            f.page_attr(page, "MediaBox")
                .map(flashtex_pdf::reader::render)
                .unwrap_or_default()
        );
        for (name, fd) in f.page_fonts(page) {
            match compare::font_from_dict(&f, fd) {
                Ok(font) => println!(
                    "  font /{name}: {}",
                    match &font {
                        flashtex_pdf::exact::ExactFont::Simple(s) => format!(
                            "{} {} program {:?} codes {}..{}",
                            s.subtype,
                            s.base_font,
                            font.program_identity(),
                            s.first_char,
                            s.first_char as usize + s.widths.len().max(1) - 1
                        ),
                        flashtex_pdf::exact::ExactFont::CidCff(c) => {
                            format!(
                                "Type0/CIDFontType0C {} program {:?} {} widths",
                                c.base_font,
                                font.program_identity(),
                                c.widths.len()
                            )
                        }
                        flashtex_pdf::exact::ExactFont::CidTrueType(c) => {
                            format!(
                                "Type0/CIDFontType2 {} program {:?} {} widths",
                                c.base_font,
                                font.program_identity(),
                                c.widths.len()
                            )
                        }
                    }
                ),
                Err(e) => println!("  font /{name}: unsupported: {e}"),
            }
        }
        let content = f.page_content(page)?;
        match flashtex_pdf::exact::parse(&content) {
            Ok(ops) => {
                println!("  content {} bytes, {} operators", content.len(), ops.len());
                for op in &ops {
                    let mut v = Vec::new();
                    op.write(&mut v);
                    print!("    {}", String::from_utf8_lossy(&v));
                }
            }
            Err(e) => println!("  content {} bytes: {e}", content.len()),
        }
    }
    Ok(0)
}
