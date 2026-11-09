//! An image file renamed over or removed between `\pdfximage` and the
//! shipout that writes it (by `\write18`), against TeX Live's own `pdftex`.
//!
//! pdfTeX writes a JPEG, a PNG and a PDF through the file it opened at
//! `\pdfximage` (writejpg.c's `jpg_ptr(img)->file`, writepng.c's
//! `png_get_io_ptr`, pdftoepdf.cc's `PdfDocument`), so a file renamed over
//! or removed since is still the one written; a JBIG2 file is opened again
//! by name (writejbig2.c's `write_jbig2`). Both runs must give the same PDF
//! and the same log from the `**` line on.
//!
//! Not tested: a file rewritten in place (the same inode) while it is
//! open. What pdfTeX writes then depends on the C library's buffering, not
//! on pdfTeX: the same C program, linked against the macOS 11 SDK (as
//! MacTeX's pdftex is), reads the new bytes after an `fseek` and, linked
//! against the macOS 26 SDK, keeps the stale buffer.
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// (kind, file included, file that replaces it): every PNG writing path
/// (copied IDAT, several IDAT chunks, rows decoded by libpng).
const KINDS: &[(&str, &str, &str)] = &[
    ("png", "png-rgb8.png", "png-srgb.png"),
    ("pngm", "png-multi-idat.png", "png-rgb8.png"),
    ("pnga", "png-rgba8.png", "png-ga8.png"),
    ("jpg", "jpg-rgb.jpg", "jpg-gray.jpg"),
    ("jb2", "jbig2-sequential.jb2", "jbig2-random.jbig2"),
    ("pdf", "pdf-hand.pdf", "pdf-fonts.pdf"),
];

/// (name, document) pairs: the image read from `cur-*`, then `cur-*`
/// replaced (`mv`) or removed (`rm`), then the page shipped out.
fn cases() -> Vec<(String, String)> {
    let mut v = vec![];
    for &(kind, from, to) in KINDS {
        let ext = Path::new(from).extension().unwrap().to_str().unwrap();
        let cur = format!("cur-{kind}.{ext}");
        let mut hows = vec![
            (
                "mv",
                format!("cp {to} tmp-{kind}.{ext} && mv tmp-{kind}.{ext} {cur}"),
            ),
            ("rm", format!("rm {cur}")),
        ];
        if kind == "jb2" {
            // (opened again by name: the new bytes, as defined)
            hows.push(("inplace", format!("cat {to} > {cur}")));
        }
        for (how, cmd) in hows {
            let body = format!(
                "\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
                 \\pdfsuppressptexinfo=-1 \\pdfminorversion=7 \
                 \\immediate\\write18{{cp {from} {cur}}}\
                 \\pdfximage{{{cur}}}\\edef\\i{{\\the\\pdflastximage}}\
                 \\message{{[\\the\\pdflastximagepages]}}\
                 \\immediate\\write18{{{cmd}}}\
                 \\setbox0\\hbox{{\\pdfrefximage\\i}}\\message{{[\\the\\wd0 x \\the\\ht0]}}\
                 \\box0\\bye\n"
            );
            v.push((format!("{kind}-{how}"), body));
        }
    }
    v
}

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) {
    let mut c = Command::new(bin);
    c.args(args)
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap();
}

/// The log from the `**` line on, lines joined.
fn normalised_log(log: &str) -> String {
    let from = log.find("\n**").map_or(0, |i| i + 1);
    log[from..].lines().collect()
}

#[test]
fn replaced_images_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images");
    let base = common::fresh_dir("flashtex-imgrepl");
    let cases = cases();
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for &(_, from, to) in KINDS {
            for f in [from, to] {
                std::fs::copy(images.join(f), d.join(f)).unwrap();
            }
        }
        for (name, body) in &cases {
            std::fs::write(d.join(format!("{name}.tex")), body).unwrap();
        }
    }
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _) in &cases {
        let arg = format!("&plain {name}");
        run(ours, &a, &["-fmt=plain", "-shell-escape", &arg], true);
        run(&theirs, &b, &["-shell-escape", &arg], false);
        let pdf = format!("{name}.pdf");
        let (x, y) = (
            std::fs::read(a.join(&pdf)).ok(),
            std::fs::read(b.join(&pdf)).ok(),
        );
        if x != y {
            failures.push(format!(
                "{pdf}: ours {:?} bytes, TeX Live's {:?} bytes, not the same",
                x.map(|x| x.len()),
                y.map(|y| y.len())
            ));
        }
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || !ly.contains("runsystem(") {
            failures.push(format!("{log}:\n  ours {lx:?}\n  tex  {ly:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
