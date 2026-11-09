//! PDF inclusion (pdftoepdf.cc's `write_epdf` and its helpers,
//! src/pdftex/pdftoepdf.rs) against TeX Live's pdftex, run side by side on
//! the same INITEX input: exit status, the log after its banner line and
//! the PDF, byte for byte. Both programs are started as `pdftex` from PATH,
//! so the messages that name the program agree.
//!
//! Each case includes page 1 of a small PDF file `k.pdf` written by the
//! test.
//!
//! * `copyOtherResources` warns about a resource that is not a dictionary
//!   with the key as `%s`: the bytes of a name like `/Sh#E9x` as they are,
//!   not as UTF-8.
//!
//! Skips where there is no TeX Live (e.g. CI without it).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// What one run left: exit status, the log after its first line, the PDF if
/// any.
#[derive(Debug, PartialEq)]
struct Outcome {
    code: Option<i32>,
    log: Vec<u8>,
    pdf: Option<Vec<u8>>,
}

const DOC: &[u8] = b"\\catcode`\\{=1 \\catcode`\\}=2\n\
\\pdfoutput=1 \\pdfpagewidth=200pt \\pdfpageheight=200pt\n\
\\pdfcompresslevel=0 \\pdfobjcompresslevel=0\n\
\\pdfximage{k.pdf}\\shipout\\hbox{\\pdfrefximage\\pdflastximage}\n\
\\end\n";

/// A PDF file with these objects (numbered from 1; object 1 is the
/// catalog) and a cross-reference table.
fn pdf(objs: &[&[u8]]) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offs = Vec::new();
    for (i, body) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offs {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// A one-page PDF whose page has these resources.
fn page_with_resources(resources: &[u8]) -> Vec<u8> {
    let mut page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] /Resources ".to_vec();
    page.extend_from_slice(resources);
    page.extend_from_slice(b" /Contents 4 0 R >>");
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &page,
        b"<< /Length 15 >>\nstream\n0 0 m 10 10 l S\nendstream",
    ])
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
    let log = std::fs::read(dir.join("doc.log")).unwrap_or_default();
    let after = log
        .iter()
        .position(|&b| b == b'\n')
        .map_or(log.len(), |i| i + 1);
    Outcome {
        code: out.status.code(),
        log: log[after..].to_vec(),
        pdf: std::fs::read(dir.join("doc.pdf")).ok(),
    }
}

/// pdftex's outcome and ours on `DOC` with `k.pdf`.
fn both(tag: &str, k: &[u8]) -> Option<(Outcome, Outcome)> {
    let Some(texlive) = find_texlive_bin() else {
        common::no_texlive();
        return None;
    };
    let files: &[(&str, &[u8])] = &[("doc.tex", DOC), ("k.pdf", k)];
    let base = common::fresh_dir(&format!("flashtex-epdf-{tag}"));
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

fn assert_same(tag: &str, o: &Outcome, f: &Outcome) {
    // escaped, so a byte that is not UTF-8 shows as itself
    assert_eq!(
        f.log.escape_ascii().to_string(),
        o.log.escape_ascii().to_string(),
        "{tag}: log, ours vs pdftex"
    );
    assert_eq!(f, o, "{tag}: ours vs pdftex");
}

#[test]
fn other_resource_warnings_print_the_key_bytes() {
    let k = page_with_resources(b"<< /Sh#E9x 5 /Subtype 7 /Pat#20A 3 /Subtype2 (s) >>");
    let Some((o, f)) = both("keys", &k) else {
        return;
    };
    // pdfTeX prints the name's byte 0xE9 itself
    let want: &[u8] = b"(key 'Sh\xe9x'";
    assert!(
        o.log.windows(want.len()).any(|w| w == want),
        "oracle log has no raw key byte"
    );
    assert_same("keys", &o, &f);
}
