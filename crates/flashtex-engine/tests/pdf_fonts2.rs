//! The font paths lane P3-FONTS-2 ported (src/pdftex/writet3.rs,
//! writettf.rs, subfont.rs), against TeX Live's own `pdftex`: bitmap (PK)
//! fonts written as Type 3, including fonts mktexpk makes on the fly, other
//! resolutions (`\pdfpkresolution`), magnified fonts, pdfTeX's `.pgc` glyph
//! files, and TrueType and OpenType map entries. As in `pdf_backend.rs`,
//! each engine builds its own plain.fmt, the documents run in PDF mode with
//! `\pdfsuppressptexinfo=-1` and `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`,
//! and the PDFs must be byte-identical and the backend log lines the same.
//!
//! Each engine gets its own empty `TEXMFVAR`, so mktexpk runs in both and
//! the PK files are made the same way for both (the log lines name them;
//! the two directories are compared as one). Skips where there is no TeX
//! Live (e.g. CI) or a case's fonts are not installed.
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SETUP: &str = "\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfminorversion=7 \
    \\pdfobjcompresslevel=2 \\pdfcompresslevel=9 \\pdfdecimaldigits=3 \
    \\pdfpagewidth=8.5truein \\pdfpageheight=11truein \
    \\pdfhorigin=1truein \\pdfvorigin=1truein \\pdfpkresolution=600\n";

/// A `.pgc` file (pdfTeX's Type 3 glyph procedures) for `pgcfont`, whose
/// TFM is made by the case itself from bbm10's (see `pgc` below).
const PGC: &str = "% a pgc file\n\\pdffontscale 0.001\n\
    \\pdfglyph 65 0 700 0 0 0 600 700 =\n0 0 m 300 700 l 600 0 l f\n\\endglyph\n\
    \\pdfglyph 66 550 700 0 0 0 500 700 =\n\n0 0 500 700 re f\n\\endglyph\n";

/// (name, files needed (kpsewhich), body): each is typeset after `SETUP`
/// with plain TeX.
const CASES: &[(&str, &[&str], &str)] = &[
    (
        "pk-bbm",
        &["bbm10.mf"],
        "\\font\\b=bbm10 \\b ABC xyz 123 \\char0 \\char255\n\\bye\n",
    ),
    (
        "pk-sizes",
        &["bbm10.mf"],
        "\\font\\a=bbm10 scaled\\magstep2 \\a AB \\font\\c=bbm10 scaled 1440 \\c CD \
         \\font\\d=bbm10 at 7.3pt \\d EF \\font\\e=bbm12 \\e GH\n\\bye\n",
    ),
    (
        "pk-300dpi",
        &["bbm10.mf"],
        "\\pdfpkresolution=300 \\font\\b=bbm10 \\b Resolution\n\\bye\n",
    ),
    (
        "pk-cm",
        &["cmr10.mf"],
        "\\pdfmapline{-cmr10}\\pdfmapline{-cmmi10}\\pdfmapline{-cmsy10}\\pdfmapline{-cmex10}\
         Hello, world! \\char32\\char127 $x^2+\\alpha\\le\\sum_i y_i$ $$\\left(\\int_0^1\\right)$$\n\\bye\n",
    ),
    (
        "pk-enc",
        &["bbm10.mf"],
        "\\input glyphtounicode \\pdfgentounicode=1 \\pdfmapline{=bbm10 <8r.enc}\
         \\font\\b=bbm10 \\b ABC abc\n\\bye\n",
    ),
    (
        "pk-attr",
        &["bbm10.mf"],
        "\\font\\b=bbm10 \\pdffontattr\\b{/MyKey 1} \\b A \\pdfcompresslevel=0 \\pdfobjcompresslevel=0\n\\bye\n",
    ),
    (
        "pgc",
        &["bbm10.mf"],
        "\\font\\p=pgcfont \\p ABA\n\\bye\n",
    ),
    (
        "ttf-arvo",
        &["Arvo-Regular.ttf", "Arvo-tlf-ot1.tfm"],
        "\\font\\t=Arvo-tlf-ot1 \\t Hello, TrueType! fi {\\font\\v=Arvo-tlf-t1 \\v Virtual \\'e}\n\\bye\n",
    ),
    (
        "ttf-whole",
        &["Arvo-Regular.ttf", "Arvo-tlf-ot1.tfm"],
        "\\pdfmapline{=Arvo-tlf-ot1 Arvo-Regular <<Arvo-Regular.ttf}\
         \\font\\t=Arvo-tlf-ot1 \\t Whole TrueType\n\\bye\n",
    ),
    (
        "ttf-tounicode",
        &["ClearSans-Regular.ttf", "ClearSans-tlf-ot1.tfm"],
        "\\input glyphtounicode \\pdfgentounicode=1 \\font\\t=ClearSans-tlf-ot1 \\t Clear Sans\n\\bye\n",
    ),
    (
        "otf-bodoni",
        &["GFSBodoni.otf", "md-grbr7m.tfm"],
        "\\font\\o=md-grbr7m \\o Hello, OpenType!\n\\bye\n",
    ),
    (
        "pk-mode",
        &["bbm10.mf"],
        "\\pdfpkresolution=300 \\pdfpkmode={cx}\\font\\b=bbm10 \\b Mode cx\n\\bye\n",
    ),
    (
        // subfont entries: TFMs made by ttf2tfm from Arvo with Unicode.sfd
        // (see `SUBFONT_TFMS`)
        "ttf-subfont",
        &["Arvo-Regular.ttf", "Unicode.sfd"],
        "\\pdfmapline{+arvou@Unicode@ <Arvo-Regular.ttf}\
         \\font\\u=arvou00 \\u Subfont ABC \\font\\v=arvou20 \\v \\char\"13\\char\"14\\char\"AC\n\\bye\n",
    ),
];

/// The case whose TFMs ttf2tfm makes in each directory first.
const SUBFONT_TFMS: (&str, &[&str]) = ("ttf-subfont", &["-q", "-w", "arvou@Unicode@"]);

/// Kill a run after this long (mktexpk and METAFONT included).
const TIMEOUT: Duration = Duration::from_secs(180);

fn run(bin: &Path, dir: &Path, var: &Path, args: &[&str], ours: bool) {
    run_env(bin, dir, var, args, ours, &[]);
}

fn run_env(bin: &Path, dir: &Path, var: &Path, args: &[&str], ours: bool, env: &[(&str, &str)]) {
    let mut c = Command::new(bin);
    c.args(args)
        .envs(env.iter().copied())
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("TEXMFVAR", var)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    let mut child = c.spawn().unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return;
        }
        if start.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The backend's lines of a log (lines joined first: TeX breaks them at
/// 79), with the engine's own `TEXMFVAR` written as `$VAR`.
fn backend_lines(log: &str, var: &Path) -> Vec<String> {
    let text: String = log.lines().collect();
    let text = text.replace(&var.to_string_lossy().to_string(), "$VAR");
    let mut out = vec![];
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let rest = &text[i..];
        let (open, close) = match b[i] {
            b'{' => ('{', '}'),
            b'<' => ('<', '>'),
            _ => {
                if rest.starts_with("Output written on ") {
                    out.push(rest[..rest.find(')').map_or(rest.len(), |j| j + 1)].to_string());
                }
                if rest.starts_with("!pdfTeX error") {
                    out.push(rest[..rest.find('.').map_or(rest.len(), |j| j + 1)].to_string());
                }
                i += 1;
                continue;
            }
        };
        match rest[1..].find([close, open]) {
            Some(j) if rest.as_bytes()[j + 1] == close as u8 => {
                let item = &rest[..j + 2];
                let t = item.trim_end_matches('>');
                if item.ends_with(".map}")
                    || item.ends_with(".enc}")
                    || item.ends_with(".sfd}")
                    || t.ends_with(".pfb")
                    || t.ends_with("pk")
                    || t.ends_with(".pgc")
                    || t.ends_with(".ttf")
                    || t.ends_with(".ttc")
                    || t.ends_with(".otf")
                {
                    out.push(item.to_string());
                }
                i += j + 2;
            }
            _ => i += 1,
        }
    }
    out
}

fn installed(texbin: &Path, files: &[&str]) -> bool {
    files.iter().all(|f| {
        Command::new(texbin.join("kpsewhich"))
            .arg(f)
            .stderr(Stdio::null())
            .output()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false)
    })
}

#[test]
fn pdf_fonts2_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = std::env::temp_dir().join(format!("flashtex-pdf-fonts2-{}", std::process::id()));
    let (a, b) = (base.join("ours"), base.join("tex"));
    let (va, vb) = (base.join("ours-var"), base.join("tex-var"));
    let cases: Vec<_> = CASES
        .iter()
        .filter(|(name, files, _)| {
            let ok = installed(&texbin, files);
            if !ok {
                eprintln!("{name}: fonts not installed; skipping");
            }
            ok
        })
        .collect();
    for d in [&a, &b, &va, &vb] {
        std::fs::create_dir_all(d).unwrap();
    }
    for d in [&a, &b] {
        for (name, _, body) in &cases {
            std::fs::write(d.join(format!("{name}.tex")), format!("{SETUP}{body}")).unwrap();
        }
        // pgcfont: bbm10's metrics under another name, with a .pgc file
        let tfm = Command::new(texbin.join("kpsewhich"))
            .arg("bbm10.tfm")
            .output()
            .unwrap();
        let tfm = String::from_utf8_lossy(&tfm.stdout).trim().to_string();
        if !tfm.is_empty() {
            std::fs::copy(&tfm, d.join("pgcfont.tfm")).unwrap();
            std::fs::write(d.join("pgcfont.pgc"), PGC).unwrap();
        }
        if cases.iter().any(|c| c.0 == SUBFONT_TFMS.0) {
            let ttf = Command::new(texbin.join("kpsewhich"))
                .arg("Arvo-Regular.ttf")
                .output()
                .unwrap();
            let ttf = String::from_utf8_lossy(&ttf.stdout).trim().to_string();
            let _ = Command::new(texbin.join("ttf2tfm"))
                .arg(&ttf)
                .args(SUBFONT_TFMS.1)
                .current_dir(d)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
    run(ours, &a, &va, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &vb, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _, _) in &cases {
        eprintln!("pdf_fonts2: {name}");
        let arg = format!("&plain {name}");
        run(ours, &a, &va, &[&arg], true);
        run(&theirs, &b, &vb, &[&arg], false);
        let pdf = format!("{name}.pdf");
        let (x, y) = (std::fs::read(a.join(&pdf)), std::fs::read(b.join(&pdf)));
        match (x, y) {
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
        let lx = backend_lines(
            &std::fs::read_to_string(a.join(&log)).unwrap_or_default(),
            &va,
        );
        let ly = backend_lines(
            &std::fs::read_to_string(b.join(&log)).unwrap_or_default(),
            &vb,
        );
        if lx != ly || lx.is_empty() {
            failures.push(format!("{log}: backend lines {lx:?} vs {ly:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let _ = std::fs::remove_dir_all(&base);
}

/// The display list (docs/protocol/display-list-v3.md §5.1) of a PK font
/// (`type3`, with its bitmaps), a TrueType font and an OpenType font: the
/// `FONT` resources carry their programs, every glyph of a Type 3 font
/// has a bitmap, no page is flagged incomplete, and writing the display
/// list leaves the PDF as it was.
#[test]
fn display_list_describes_type3_truetype_opentype() {
    use flashtex_display_list::client::{decode_event, Event};
    use flashtex_display_list::page::{flags, Item};
    use flashtex_display_list::resource::Type3Bitmaps;
    let Some(texbin) = find_texlive_bin() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let base = std::env::temp_dir().join(format!("flashtex-dl-fonts2-{}", std::process::id()));
    let (a, va) = (base.join("ours"), base.join("ours-var"));
    for d in [&a, &va] {
        std::fs::create_dir_all(d).unwrap();
    }
    let wanted = [
        ("pk-bbm", "type3"),
        ("pk-sizes", "type3"),
        ("ttf-arvo", "truetype"),
        ("otf-bodoni", "opentype"),
    ];
    let cases: Vec<_> = CASES
        .iter()
        .filter_map(|(name, files, body)| {
            let fmt = wanted.iter().find(|w| w.0 == *name)?.1;
            installed(&texbin, files).then_some((*name, fmt, *body))
        })
        .collect();
    for (name, _, body) in &cases {
        std::fs::write(a.join(format!("{name}.tex")), format!("{SETUP}{body}")).unwrap();
    }
    run(ours, &a, &va, &["-ini", "\\input plain \\dump"], true);
    let mut failures = vec![];
    for (name, fmt, _) in &cases {
        eprintln!("display list: {name}");
        let arg = format!("&plain {name}");
        let pdf = a.join(format!("{name}.pdf"));
        // first without a display list: mktexpk makes the PK files then,
        // as it does for a document's first compile
        run(ours, &a, &va, &[&arg], true);
        let plain = std::fs::read(&pdf).unwrap_or_default();
        let dl = a.join(format!("{name}.dl3"));
        let dls = dl.to_string_lossy().into_owned();
        run_env(
            ours,
            &a,
            &va,
            &[&arg],
            true,
            &[("FLASHTEX_DISPLAY_LIST", dls.as_str())],
        );
        let with_dl = std::fs::read(&pdf).unwrap_or_default();
        if plain.is_empty() || plain != with_dl {
            failures.push(format!(
                "{name}: the PDF changes when a display list is written"
            ));
        }
        let bytes = std::fs::read(&dl).unwrap_or_default();
        let mut r = std::io::Cursor::new(bytes);
        let mut fonts = std::collections::HashMap::new();
        let mut glyphs = vec![];
        let mut pages = 0;
        while let Ok(Some((k, body))) = flashtex_display_list::frame::read_frame(&mut r) {
            match decode_event(k, body) {
                Ok(Event::Font(f)) => {
                    fonts.insert(f.id, f);
                }
                Ok(Event::Page(p)) => {
                    pages += 1;
                    if p.flags & flags::INCOMPLETE != 0 {
                        failures.push(format!(
                            "{name}: page flagged incomplete: {:?}",
                            p.unsupported
                        ));
                    }
                    for it in &p.items {
                        if let Item::Glyph { font, code, .. } = it {
                            glyphs.push((*font, *code));
                        }
                    }
                }
                _ => {}
            }
        }
        if pages == 0 {
            failures.push(format!("{name}: no pages in the display list"));
        }
        let of_fmt: Vec<_> = fonts
            .values()
            .filter(|f| f.info.str_field("format") == Some(fmt))
            .collect();
        if of_fmt.is_empty() {
            failures.push(format!("{name}: no `{fmt}' font"));
        }
        for f in of_fmt {
            if f.program.is_empty() {
                failures.push(format!("{name}: `{fmt}' font {} without a program", f.id));
                continue;
            }
            if *fmt == "type3" {
                match Type3Bitmaps::decode(&f.program) {
                    Ok(t) => {
                        for (font, code) in &glyphs {
                            if *font == f.id && t.glyph(*code as u8).is_none() {
                                failures.push(format!("{name}: no bitmap for code {code}"));
                            }
                        }
                        if f.info.str_field("font_matrix").is_none() {
                            failures.push(format!("{name}: type3 font without font_matrix"));
                        }
                    }
                    Err(e) => failures.push(format!("{name}: type3 program: {e}")),
                }
            } else {
                let file = f.info.str_field("file").unwrap_or_default().to_string();
                if std::fs::read(&file).ok().as_deref() != Some(f.program.as_slice()) {
                    failures.push(format!("{name}: `{fmt}' program is not the file {file}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let _ = std::fs::remove_dir_all(&base);
}
