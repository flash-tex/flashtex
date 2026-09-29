//! Image and PDF inclusion (src/pdftex/: writeimg, writepng over libpng,
//! writejpg, writejbig2, pdftoepdf over xpdf) against TeX Live's own
//! `pdftex`: build plain.fmt with each, typeset documents that include the
//! images of tests/images/ (made by tests/images/generate.py), and require
//! byte-identical PDFs and identical logs from the `**` line on (lines
//! joined, the program name in pdfTeX's warnings and errors normalised: it
//! is argv[0], see docs/evidence/pdf-images-2026-09-29/).
//!
//! `\pdfsuppressptexinfo=-1` or `=1` leaves out `/PTEX.Fullbanner`, which
//! names web2c's version string. Both runs use `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

fn setup(minor: u32, extra: &str) -> String {
    format!(
        "\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfminorversion={minor} \
         \\pdfobjcompresslevel=2 \\pdfcompresslevel=9 \\pdfdecimaldigits=3 \
         \\pdfpagewidth=8.5truein \\pdfpageheight=11truein \
         \\pdfhorigin=1truein \\pdfvorigin=1truein {extra}\n\
         \\def\\img#1#2{{\\pdfximage width 1in #1{{#2}}\\pdfrefximage\\pdflastximage\\ }}\n"
    )
}

const PNGS: &[&str] = &[
    "png-rgb8.png",
    "png-gray8.png",
    "png-gray1.png",
    "png-gray4.png",
    "png-pal8.png",
    "png-pal4-trns.png",
    "png-rgba8.png",
    "png-ga8.png",
    "png-rgb16.png",
    "png-rgba16.png",
    "png-rgb8-interlaced.png",
    "png-gray16-interlaced.png",
    "png-gamma.png",
    "png-srgb.png",
    "png-multi-idat.png",
    "png-gray-trns.png",
];

const JPEGS: &[&str] = &[
    "jpg-gray.jpg",
    "jpg-rgb.jpg",
    "jpg-progressive.jpg",
    "jpg-cmyk.jpg",
    "jpg-exif.jpg",
];

/// `\img{}{file}` for each file, a page each, with the image's size and
/// resolution queries in the log.
fn each(files: &[&str]) -> String {
    files
        .iter()
        .map(|f| {
            format!(
                "\\img{{}}{{{f}}}\\message{{[{f}: \\the\\pdflastximagepages\\space \
                 \\the\\pdflastximagecolordepth]}}\\vfill\\eject\n"
            )
        })
        .collect()
}

/// (name, document) pairs.
fn cases() -> Vec<(&'static str, String)> {
    let hand = "pdf-hand.pdf";
    vec![
        ("png", format!("{}{}\\bye\n", setup(7, "\\pdfimagehicolor=1"), each(PNGS))),
        ("png-nohicolor", format!("{}{}\\bye\n", setup(7, "\\pdfimagehicolor=0"), each(PNGS))),
        ("png-14", format!("{}{}\\bye\n", setup(4, "\\pdfimagehicolor=1"), each(PNGS))),
        ("png-13", format!("{}{}\\bye\n", setup(3, ""), each(PNGS))),
        (
            "png-gamma",
            format!(
                "{}{}\\bye\n",
                setup(7, "\\pdfimageapplygamma=1 \\pdfgamma=2200 \\pdfimagegamma=2200"),
                each(PNGS)
            ),
        ),
        (
            "png-plain-streams",
            format!(
                "{}{}\\bye\n",
                setup(7, "\\pdfcompresslevel=0 \\pdfobjcompresslevel=0"),
                each(&["png-rgba8.png", "png-pal8.png", "png-gray8.png", "png-rgb8-interlaced.png"])
            ),
        ),
        (
            "png-colorspace",
            format!(
                "{}\\immediate\\pdfobj{{[/ICCBased 0 R]}}\\edef\\cs{{\\the\\pdflastobj}}\
                 \\img{{colorspace \\cs}}{{png-pal8.png}}\\img{{colorspace \\cs}}{{png-rgb8.png}}\
                 \\img{{colorspace \\cs}}{{png-rgba8.png}}\\img{{colorspace \\cs}}{{jpg-cmyk.jpg}}\
                 \\img{{}}{{png-rgba8.png}}\\img{{}}{{png-ga8.png}}\\vfill\\eject\
                 \\img{{}}{{png-rgba16.png}}\\bye\n",
                setup(7, "\\pdfimagehicolor=1")
            ),
        ),
        ("jpeg", format!("{}{}\\bye\n", setup(7, ""), each(JPEGS))),
        (
            "jbig2",
            format!(
                "{}\\img{{page 1}}{{jbig2-sequential.jb2}}\\img{{page 2}}{{jbig2-sequential.jb2}}\
                 \\vfill\\eject\\img{{page 2}}{{jbig2-random.jbig2}}\\img{{page 1}}{{jbig2-random.jbig2}}\
                 \\message{{[\\the\\pdflastximagepages]}}\\bye\n",
                setup(5, "")
            ),
        ),
        (
            "pdf",
            format!(
                "{}Host text in cmr10 and {{\\bf bold}}.\\par\
                 \\img{{}}{{{hand}}}\\img{{page 2}}{{{hand}}}\\img{{page 3}}{{{hand}}}\\vfill\\eject\
                 \\img{{cropbox}}{{{hand}}}\\img{{bleedbox}}{{{hand}}}\\img{{trimbox}}{{{hand}}}\
                 \\img{{artbox}}{{{hand}}}\\img{{named {{pagetwo}}}}{{{hand}}}\\vfill\\eject\
                 \\img{{}}{{pdf-fonts.pdf}}\\img{{page 2}}{{pdf-fonts.pdf}}\
                 \\img{{}}{{pdf-fonts-whole.pdf}}\\vfill\\eject\
                 \\img{{page 2}}{{pdf-objstm.pdf}}\\img{{}}{{pdf-objstm.pdf}}\
                 \\message{{[\\the\\pdflastximagepages]}}\\bye\n",
                setup(7, "")
            ),
        ),
        (
            "pdf-ptexinfo",
            format!(
                "{}\\img{{}}{{{hand}}}\\img{{page 2}}{{pdf-fonts.pdf}}\\bye\n",
                setup(7, "\\pdfsuppressptexinfo=1")
            ),
        ),
        (
            "pdf-underscore",
            format!(
                "{}\\img{{}}{{{hand}}}\\bye\n",
                setup(7, "\\pdfsuppressptexinfo=1 \\pdfptexuseunderscore=1")
            ),
        ),
        (
            "pdf-copyfonts",
            format!(
                "{}Text.\\img{{}}{{pdf-fonts.pdf}}\\img{{}}{{pdf-fonts-whole.pdf}}\\bye\n",
                setup(7, "\\pdfinclusioncopyfonts=1")
            ),
        ),
        (
            "pdf-damaged",
            format!(
                "{}\\img{{page 2}}{{pdf-broken-xref.pdf}}\\img{{}}{{pdf-17.pdf}}\\bye\n",
                setup(4, "\\pdfinclusionerrorlevel=0")
            ),
        ),
        (
            "pdf-groups",
            format!(
                "{}\\img{{page 2}}{{{hand}}}\\img{{page 2}}{{{hand}}}\\img{{}}{{png-rgba8.png}}\
                 \\vfill\\eject\\img{{}}{{png-ga8.png}}\\img{{page 2}}{{{hand}}}\\bye\n",
                setup(7, "")
            ),
        ),
        (
            "immediate",
            format!(
                "{}\\immediate\\pdfximage{{png-rgb8.png}}\\edef\\a{{\\the\\pdflastximage}}\
                 \\pdfximage page 2 {{pdf-fonts.pdf}}\\edef\\b{{\\the\\pdflastximage}}\
                 \\pdfrefximage\\a\\pdfrefximage\\b\\vfill\\eject\
                 \\pdfrefximage\\b\\pdfrefximage\\a\\bye\n",
                setup(7, "")
            ),
        ),
    ]
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

/// The log from the `**` line on, lines joined, with the program name
/// after "pdfTeX warning: " and "!pdfTeX error: " replaced.
fn normalised_log(log: &str) -> String {
    let from = log.find("\n**").map_or(0, |i| i + 1);
    let mut s: String = log[from..].lines().collect();
    for key in ["pdfTeX warning: ", "!pdfTeX error: "] {
        let mut out = String::new();
        let mut rest = s.as_str();
        while let Some(i) = rest.find(key) {
            out.push_str(&rest[..i + key.len()]);
            rest = &rest[i + key.len()..];
            let end = rest.find([' ', ':']).unwrap_or(rest.len());
            out.push_str("PROG");
            rest = &rest[end..];
        }
        out.push_str(rest);
        s = out;
    }
    s
}

#[test]
fn images_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images");
    let base = std::env::temp_dir().join(format!("flashtex-img-{}", std::process::id()));
    let (a, b) = (base.join("ours"), base.join("tex"));
    let cases = cases();
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for e in std::fs::read_dir(&images).unwrap() {
            let e = e.unwrap();
            std::fs::copy(e.path(), d.join(e.file_name())).unwrap();
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
        run(ours, &a, &["-fmt=plain", &arg], true);
        run(&theirs, &b, &[&arg], false);
        let pdf = format!("{name}.pdf");
        match (std::fs::read(a.join(&pdf)), std::fs::read(b.join(&pdf))) {
            (Ok(x), Ok(y)) if x == y => {}
            (Ok(x), Ok(y)) => {
                let at = x
                    .iter()
                    .zip(&y)
                    .position(|(p, q)| p != q)
                    .unwrap_or(x.len().min(y.len()));
                failures.push(format!(
                    "{pdf}: differs at byte {at} ({} vs {} bytes)",
                    x.len(),
                    y.len()
                ));
            }
            (x, y) => failures.push(format!(
                "{pdf}: ours {}, TeX Live's {}",
                if x.is_ok() { "written" } else { "missing" },
                if y.is_ok() { "written" } else { "missing" }
            )),
        }
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            let at = lx
                .chars()
                .zip(ly.chars())
                .position(|(p, q)| p != q)
                .unwrap_or(lx.len().min(ly.len()));
            let ctx = |s: &str| {
                s.chars()
                    .skip(at.saturating_sub(40))
                    .take(120)
                    .collect::<String>()
            };
            failures.push(format!(
                "{log}: differs at {at}:\n  ours {:?}\n  tex  {:?}",
                ctx(&lx),
                ctx(&ly)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// pdfTeX's fatal image errors: the same message (and no PDF) from both.
#[test]
fn image_errors_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images");
    let base = std::env::temp_dir().join(format!("flashtex-imgerr-{}", std::process::id()));
    let cases: Vec<(&str, String)> = vec![
        (
            "progressive-12",
            format!("{}\\img{{}}{{jpg-progressive.jpg}}\\bye\n", setup(2, "")),
        ),
        (
            "jbig2-13",
            format!("{}\\img{{}}{{jbig2-sequential.jb2}}\\bye\n", setup(3, "")),
        ),
        (
            "jbig2-nopage",
            format!(
                "{}\\img{{page 7}}{{jbig2-random.jbig2}}\\bye\n",
                setup(5, "")
            ),
        ),
        (
            "pdf-nopage",
            format!("{}\\img{{page 4}}{{pdf-hand.pdf}}\\bye\n", setup(7, "")),
        ),
        (
            "pdf-nodest",
            format!(
                "{}\\img{{named {{nothere}}}}{{pdf-hand.pdf}}\\bye\n",
                setup(7, "")
            ),
        ),
        (
            "pdf-version",
            format!(
                "{}\\img{{}}{{pdf-17.pdf}}\\bye\n",
                setup(4, "\\pdfinclusionerrorlevel=1")
            ),
        ),
        (
            "no-file",
            format!("{}\\img{{}}{{nothere.png}}\\bye\n", setup(7, "")),
        ),
        (
            "not-image",
            format!("{}\\img{{}}{{generate.py}}\\bye\n", setup(7, "")),
        ),
    ];
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for e in std::fs::read_dir(&images).unwrap() {
            let e = e.unwrap();
            std::fs::copy(e.path(), d.join(e.file_name())).unwrap();
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
        run(ours, &a, &["-fmt=plain", &arg], true);
        run(&theirs, &b, &[&arg], false);
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || !ly.contains("pdfTeX") {
            failures.push(format!("{log}:\n  ours {lx:?}\n  tex  {ly:?}"));
        }
        let pdf = format!("{name}.pdf");
        if a.join(&pdf).exists() != b.join(&pdf).exists() {
            failures.push(format!("{pdf}: written by one engine only"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
