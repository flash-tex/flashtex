//! pdfTeX's C parts (`src/pdftex/`) against TeX Live's pdftex, run side by
//! side on the same INITEX input (lane WEB2RUST-AUDIT, audit of the C
//! ports): the log after its banner line, the terminal and the PDF, byte for
//! byte. Both programs are started as `pdftex` from PATH, so the messages
//! that name the program (`kpse_invocation_name`) agree.
//!
//! * A map file and an encoding file found in the current directory are
//!   shown as `{my.map}` and `{my.enc}`: texmfmp.c's `open_input` drops the
//!   `./` kpathsea puts in front of them, and writet1.c's `load_enc_file`
//!   and mapfile.c's `fm_read_info` print that name.
//! * `\pdfuniqueresname`'s prefix is the CRC-32 of utils.c's `setjobid`
//!   string, which ends with pdfTeX's banner, web2c's version string and
//!   kpathsea's.
//! * An included PDF with an object xpdf reads as an error stops the run
//!   with pdftoepdf.cc's `type <error> cannot be copied`, and the ` <raw.pdf`
//!   printed before it reaches the terminal before xpdf's own messages on
//!   stderr (utils.c's `tex_printf` flushes stdout).
//!
//! Skips where there is no TeX Live (e.g. CI without it).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// What one run left: exit status, terminal (stdout and stderr in one pipe,
/// as on a terminal), the log after its first line, the PDF if any.
#[derive(Debug, PartialEq)]
struct Outcome {
    code: Option<i32>,
    terminal: String,
    log: String,
    pdf: Option<Vec<u8>>,
}

/// Run `pdftex` from `bin` on `doc.tex` in `dir`, with `files` written there
/// first.
fn run(bin: &Path, dir: &Path, files: &[(&str, &[u8])]) -> Outcome {
    std::fs::create_dir_all(dir).unwrap();
    for (name, data) in files {
        std::fs::write(dir.join(name), data).unwrap();
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs = vec![bin.to_path_buf()];
    dirs.extend(std::env::split_paths(&path));
    let out = Command::new("sh")
        .args([
            "-c",
            "exec pdftex -ini -interaction=nonstopmode doc.tex 2>&1",
        ])
        .current_dir(dir)
        .env("PATH", std::env::join_paths(dirs).unwrap())
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let after_first = |s: &str| s.split_once('\n').map_or("", |(_, t)| t).to_string();
    Outcome {
        code: out.status.code(),
        terminal: after_first(&String::from_utf8_lossy(&out.stdout)),
        log: after_first(&std::fs::read_to_string(dir.join("doc.log")).unwrap_or_default()),
        pdf: std::fs::read(dir.join("doc.pdf")).ok(),
    }
}

/// pdftex's outcome and ours on the same files, in a fresh directory.
fn both(tag: &str, files: &[(&str, &[u8])]) -> Option<(Outcome, Outcome)> {
    let Some(texlive) = find_texlive_bin() else {
        common::no_texlive();
        return None;
    };
    let base = common::fresh_dir(&format!("flashtex-pdftex-c-{tag}"));
    let ours = base.join("bin");
    std::fs::create_dir_all(&ours).unwrap();
    common::link_engine(
        Path::new(env!("CARGO_BIN_EXE_flashtex-initex")),
        &ours.join("pdftex"),
    );
    let o = run(&texlive, &base.join("o"), files);
    let f = run(&ours, &base.join("f"), files);
    let _ = std::fs::remove_dir_all(&base);
    Some((o, f))
}

#[test]
fn cwd_map_and_enc_names_and_unique_resource_names_are_pdftex_s() {
    let doc = b"\\catcode`\\{=1 \\catcode`\\}=2\n\
\\pdfoutput=1 \\pdfuniqueresname=1 \\pdfcompresslevel=0\n\
\\pdfmapfile{=my.map}\n\
\\font\\a=cmr10 \\a\n\
\\shipout\\hbox{AB}\n\
\\end\n";
    let mut enc = String::from("/MyEnc [\n");
    for c in 0..256 {
        enc.push_str(match c {
            65 => "/A\n",
            66 => "/B\n",
            _ => "/.notdef\n",
        });
    }
    enc.push_str("] def\n");
    let map = b"cmr10 CMR10 \" MyEnc ReEncodeFont \" <my.enc <cmr10.pfb\n";
    let Some((o, f)) = both(
        "map",
        &[
            ("doc.tex", doc),
            ("my.map", map),
            ("my.enc", enc.as_bytes()),
        ],
    ) else {
        return;
    };
    assert_eq!(o.code, Some(0), "{o:?}");
    assert!(
        o.log.contains("{my.map}") && o.log.contains("{my.enc}"),
        "{}",
        o.log
    );
    assert_eq!(
        (&f.code, &f.terminal, &f.log),
        (&o.code, &o.terminal, &o.log)
    );
    assert!(f.pdf == o.pdf, "the PDFs differ");
}

/// A one-page PDF whose page resources hold an array with `{}` in it, which
/// xpdf's parser reads as an error object.
fn raw_pdf() -> Vec<u8> {
    let objs: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
          /Resources << /Properties << /P1 5 0 R >> >> >>",
        b"<< /Length 12 >>\nstream\n0 0 1 1 re f\nendstream",
        b"<< /Arr [/a (c) [] {}] >>",
    ];
    let mut out = b"%PDF-1.5\n".to_vec();
    let mut offs = vec![];
    for (i, body) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let x = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offs {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[test]
fn an_object_xpdf_reads_as_an_error_cannot_be_copied_as_in_pdftex() {
    let doc = b"\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1\n\
\\pdfximage{raw.pdf}\\shipout\\hbox{\\pdfrefximage\\pdflastximage}\n\
\\end\n";
    let pdf = raw_pdf();
    let Some((o, f)) = both("incl", &[("doc.tex", doc), ("raw.pdf", &pdf)]) else {
        return;
    };
    assert!(
        o.terminal.contains("type <error> cannot be c"),
        "{}",
        o.terminal
    );
    assert_eq!(
        (&f.code, &f.terminal, &f.log),
        (&o.code, &o.terminal, &o.log)
    );
}

/// `raw_pdf`'s objects with a `startxref` that points nowhere: xpdf reports
/// the bad xref table on stderr and rebuilds it.
fn bad_xref_pdf() -> Vec<u8> {
    let mut pdf = raw_pdf();
    let at = pdf.windows(10).rposition(|w| w == b"startxref\n").unwrap() + 10;
    let end = at + pdf[at..].iter().position(|&b| b == b'\n').unwrap();
    pdf.splice(at..end, b"7".iter().copied());
    pdf
}

/// xpdf's messages at `\pdfximage` time (read_pdf_info, a rebuilt xref) and
/// the terminal around them, as in pdfTeX (REVIEW-A on #2098). The shim's
/// error callback flushes the engine's terminal before each `libxpdf:`
/// message, as xpdf's `error()` flushes pdfTeX's (C's stdout). Here the
/// partial line `(./doc.tex` is already out (TeX's `update_terminal` after
/// a file name), so this pins the order rather than failing without the
/// callback.
#[test]
fn xpdf_messages_come_after_a_partial_terminal_line_as_in_pdftex() {
    let doc = b"\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1\n\
\\pdfximage{xref.pdf}\n\
\\end\n";
    let pdf = bad_xref_pdf();
    let Some((o, f)) = both("xref", &[("doc.tex", doc), ("xref.pdf", &pdf)]) else {
        return;
    };
    assert!(o.terminal.contains("libxpdf:"), "{}", o.terminal);
    assert_eq!(
        (&f.code, &f.terminal, &f.log),
        (&o.code, &o.terminal, &o.log)
    );
}
