//! Read confinement (`FLASHTEX_CONFINE_READS=1`, a Live Share host:
//! docs/design/live-collab/PROPOSAL.md §6.2) in PDF mode's output: the
//! files specials name (`pdf:fstream`, `pdf:image`) are read only through
//! the confined resolver, so a document cannot pull a file from outside
//! its project into the output, by an absolute path, `..`, or the path a
//! `\XeTeXpicfile` lookup refused (graphicx's xetex driver measures a
//! picture with `\XeTeXpicfile`, then names the same path in `pdf:image`).
//! Without confinement the same document does read the outside files,
//! which shows the test can see them.

use flashtex_display_list::frame::read_frame;
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A 2×2 grey PNG.
const PNG: [u8; 71] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x00, 0x00, 0x00, 0x00, 0x57, 0xdd, 0x52,
    0xf8, 0x00, 0x00, 0x00, 0x0e, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x68, 0x68, 0x60, 0x68,
    0x68, 0x00, 0x00, 0x06, 0x06, 0x02, 0x01, 0x2c, 0xc1, 0x50, 0xd7, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

const SECRET: &str = "secret-token-12345";

fn setup(tag: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!(
        "flashtex-xetex-confine-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let job = base.join("job");
    let outside = base.join("outside");
    std::fs::create_dir_all(&job).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), SECRET).unwrap();
    std::fs::write(outside.join("outside.png"), PNG).unwrap();
    std::fs::write(job.join("inside.png"), PNG).unwrap();
    let o = outside.canonicalize().unwrap();
    let o = o.to_str().unwrap();
    let input = format!(
        r#"\catcode`\{{=1 \catcode`\}}=2 \catcode`\#=6
\pdfpagewidth=200bp \pdfpageheight=200bp
\setbox0\hbox{{\XeTeXpicfile "{o}/outside.png"}}
\setbox2\hbox{{\XeTeXpicfile "inside.png" width 10bp}}
\shipout\hbox{{\special{{pdf:fstream @s ({o}/secret.txt)}}%
\special{{pdf:fstream @t (../outside/secret.txt)}}%
\special{{pdf:put @resources <</X1 @s /X2 @t>>}}%
\special{{pdf:image width 10bp ({o}/outside.png)}}%
\special{{pdf:image width 10bp (../outside/outside.png)}}%
\box2}}
\end
"#
    );
    std::fs::write(job.join("t.tex"), input).unwrap();
    (base, job)
}

/// Run the document; the stderr and the `file` of every IMAGE frame.
fn run(job: &Path, confine: bool) -> (String, Vec<String>) {
    let dl = job.join("t.dl3");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"));
    cmd.current_dir(job)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("FLASHTEX_DISPLAY_LIST", &dl)
        .args(["-ini", "-etex", "-interaction=nonstopmode", "t.tex"]);
    if confine {
        cmd.env("FLASHTEX_CONFINE_READS", "1");
    } else {
        cmd.env_remove("FLASHTEX_CONFINE_READS");
    }
    let out = cmd.output().expect("run flashtex-xetex");
    let mut files = Vec::new();
    let mut r = std::io::BufReader::new(std::fs::File::open(&dl).unwrap());
    while let Some((k, body)) = read_frame(&mut r).unwrap() {
        if k == kind::IMAGE {
            let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
            files.push(j.str_field("file").unwrap().to_string());
        }
    }
    (String::from_utf8_lossy(&out.stderr).into_owned(), files)
}

/// The data of the stream the page's resources name `/X1` (the
/// `pdf:fstream` of the outside file), if it is a stream.
fn fstream_data(job: &Path) -> Option<Vec<u8>> {
    use flashtex_pdf::reader::{Obj, PdfFile};
    let bytes = std::fs::read(job.join("t.pdf")).unwrap();
    let pdf = PdfFile::parse(&bytes).unwrap();
    let page = pdf.pages().unwrap()[0];
    let res = pdf
        .page_attr(page, "Resources")
        .and_then(Obj::as_dict)
        .unwrap();
    let x1 = pdf.resolve(res.get("X1")?);
    match x1 {
        Obj::Stream { .. } => pdf.decode_stream(x1).ok(),
        _ => None,
    }
}

#[test]
fn confined_reads_keep_outside_files_out_of_the_output() {
    let (base, job) = setup("on");
    let (err, files) = run(&job, true);
    // only the project's own picture is drawn
    assert_eq!(files.len(), 1, "{files:?}\n{err}");
    assert!(files[0].ends_with("inside.png"), "{files:?}");
    assert!(err.contains("pdf:fstream: cannot read"), "{err}");
    assert!(err.contains("pdf:image: cannot find"), "{err}");
    // the PDF: the stream `pdf:fstream` named is not there
    assert_eq!(fstream_data(&job), None);
    let log = std::fs::read_to_string(job.join("t.log")).unwrap();
    assert!(log.contains("Unable to load picture"), "{log}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn without_confinement_the_same_files_are_read() {
    let (base, job) = setup("off");
    let (err, files) = run(&job, false);
    // the absolute path and `../`: two names of the outside picture
    assert_eq!(
        files.iter().filter(|f| f.ends_with("outside.png")).count(),
        2,
        "{files:?}\n{err}"
    );
    assert!(!err.contains("pdf:fstream: cannot read"), "{err}");
    assert_eq!(fstream_data(&job).as_deref(), Some(SECRET.as_bytes()));
    let _ = std::fs::remove_dir_all(&base);
}
