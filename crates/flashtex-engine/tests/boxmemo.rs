//! BOX-MEMO (docs/design/engine-v2/BOX-MEMO.md, src/boxmemo.rs): a
//! registered macro call that typesets only what it throws away is recorded
//! and replayed while its key holds. These tests run INITEX on small files
//! with `FLASHTEX_BOXMEMO_NAMES` naming the macro, and check that
//!
//! * the log is byte-identical with BOX-MEMO off, on, and in the
//!   both-paths verification mode, and the verifier finds no difference;
//! * calls with an equal key are replayed;
//! * the key falls back exactly when it must: a parameter, a register, a
//!   macro it reads, a name it looked up and did not find, a font
//!   parameter, an argument;
//! * a call outside the model (output, a box it did not make, the outer
//!   list) is never replayed.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = common::fresh_dir(&format!("flashtex-bm-{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Run {
    log: String,
    stats: String,
    /// The PDF, when the test made one.
    pdf: Option<Vec<u8>>,
}

/// Run `\input t` in INITEX with BOX-MEMO in `mode`, offering `names`.
fn run(dir: &Path, mode: &str, names: &str) -> Run {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = dir.join(format!("stats-{mode}.txt"));
    let _ = std::fs::remove_file(&stats);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "-etex", "\\input t"])
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_BOXMEMO", mode)
        .env("FLASHTEX_BOXMEMO_NAMES", names)
        .env("FLASHTEX_BOXMEMO_STATS", &stats)
        .env("FLASHTEX_BOXMEMO_VERIFY_FAIL", "1")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("boxmemo verify"), "{err}");
    let pdf = std::fs::read(dir.join("t.pdf")).ok();
    let _ = std::fs::remove_file(dir.join("t.pdf"));
    Run {
        log: std::fs::read_to_string(dir.join("t.log")).unwrap(),
        stats: std::fs::read_to_string(&stats).unwrap_or_default(),
        pdf,
    }
}

fn stat(stats: &str, key: &str) -> u64 {
    stats
        .lines()
        .find_map(|l| {
            let l = l.trim().trim_end_matches(',');
            let (k, v) = l.split_once(':')?;
            (k.trim().trim_matches('"') == key).then(|| v.trim().parse().ok())?
        })
        .unwrap_or(0)
}

/// Run `src` off, on and verify; the logs must agree and the verifier find
/// nothing. Returns the "on" statistics.
fn check(tag: &str, src: &str) -> String {
    let d = scratch(tag);
    std::fs::write(d.join("t.tex"), src).unwrap();
    let off = run(&d, "off", "measure");
    let on = run(&d, "on", "measure");
    let verify = run(&d, "verify", "measure");
    assert_eq!(off.log, on.log, "{tag}: log with BOX-MEMO on differs");
    assert!(off.pdf == on.pdf, "{tag}: PDF with BOX-MEMO on differs");
    assert!(
        off.pdf == verify.pdf,
        "{tag}: PDF in verification mode differs"
    );
    assert_eq!(
        off.log, verify.log,
        "{tag}: log in verification mode differs"
    );
    assert_eq!(
        stat(&verify.stats, "verify_differences"),
        0,
        "{tag}: {}",
        verify.stats
    );
    // every admitted call is also compared on the whole engine state
    assert_eq!(
        stat(&verify.stats, "full_verified"),
        stat(&verify.stats, "verified"),
        "{tag}: {}",
        verify.stats
    );
    assert_eq!(
        stat(&verify.stats, "full_differences"),
        0,
        "{tag}: {}",
        verify.stats
    );
    assert_eq!(
        stat(&verify.stats, "verified"),
        stat(&on.stats, "hits"),
        "{tag}: {}\n{}",
        verify.stats,
        on.stats
    );
    on.stats
}

/// `\measure{W}` measures what a frame adds around a rule of width W, in a
/// box it throws away (framed.sty's `\fb@sizeofframe` in miniature), and
/// keeps two global dimensions; on the way it makes local and global
/// assignments of every kind the log replays.
const PRELUDE: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\countdef\\n=1 \\dimendef\\w=2 \\dimendef\\h=3 \\skipdef\\s=4 \\toksdef\\t=5\n\
    \\def\\pad{3pt}\\def\\frame#1{\\hbox{\\vrule width 1pt\\kern\\pad\\vbox{\\hrule\\kern\\pad#1\\kern\\pad\\hrule}\\kern\\pad\\vrule width 1pt}}\n\
    \\def\\measure#1{\\begingroup\\setbox0\\vbox{\\frame{\\hbox{\\vrule width #1 height 4pt depth 1pt}}}%\n\
    \\global\\w=\\wd0 \\global\\h=\\ht0 \\global\\advance\\n by 1 \\global\\s=1pt plus 2pt\n\
    \\def\\inner{x}\\xdef\\last{\\the\\w/\\the\\h}\\global\\let\\copy\\last\\global\\t{\\inner}\n\
    \\ifcsname absent\\endcsname\\global\\advance\\n by 100 \\fi\\endgroup}\n\
    \\def\\show{\\message{[\\the\\w|\\the\\h|\\the\\n|\\the\\s|\\meaning\\last|\\meaning\\copy|\\the\\t|\\the\\badness]}}\n";

#[test]
fn replays_and_matches() {
    let s = check(
        "basic",
        &[
            PRELUDE,
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn the_key_falls_back() {
    // a different argument, a register it reads changed (\n), a macro it
    // reads redefined (\pad), a parameter changed (\baselineskip... here
    // \boxmaxdepth), a name it looked up defined: each is a miss, and the
    // logs still agree
    let s = check(
        "fallback",
        &[
            PRELUDE,
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{6pt}\\show\
             \\n=7 \\measure{5pt}\\show\\n=0 \\def\\pad{4pt}\\measure{5pt}\\show\
             \\n=0 \\boxmaxdepth=0pt \\measure{5pt}\\show\
             \\n=0 \\def\\absent{}\\measure{5pt}\\show\
             \\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 1, "{s}");
}

#[test]
fn outside_the_model_never_replays() {
    // output, the contents of a box register it did not make (`\\copy`),
    // and the main vertical list: recorded calls are abandoned, so nothing
    // replays
    let s = check(
        "impure",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\setbox9\\hbox{\\vrule width 2pt}\n\
         \\def\\measure#1{\\begingroup\\message{#1}\\endgroup}\n\
         \\measure{a}\\measure{a}\\measure{a}\n\
         \\def\\measure#1{\\begingroup\\setbox0\\copy9 \\global\\dimen1=\\wd0 \\endgroup}\n\
         \\measure{a}\\measure{a}\\measure{a}\n\
         \\def\\measure#1{\\kern#1}\n\
         \\measure{1pt}\\measure{1pt}\\measure{1pt}\\showlists\\end\n",
    );
    assert_eq!(stat(&s, "hits"), 0, "{s}");
}

#[test]
fn void_box_reads_are_part_of_the_key() {
    let s = check(
        "void",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\measure#1{\\begingroup\\setbox0\\copy9 \\global\\dimen1=\\wd0 \\endgroup}\n\
         \\measure{a}\\measure{a}\\measure{a}\\message{[\\the\\dimen1]}\n\
         \\setbox9\\hbox{\\vrule width 2pt}\\measure{a}\\message{[\\the\\dimen1]}\\end\n",
    );
    assert!(stat(&s, "hits") >= 1, "{s}");
}

#[test]
fn draw_calls_replay_with_their_hole() {
    // a call in restricted horizontal mode that moves a box out of a
    // register it did not make (`\\box9`) and appends a frame around it:
    // replayed, the box of the moment goes into the recorded frame; a box
    // of other dimensions is another key
    let s = check(
        "draw",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\showboxdepth=100 \\showboxbreadth=100 \\tracingonline=0\n\
         \\def\\measure#1{\\begingroup\\dimen0=#1\\relax\\vrule width\\dimen0\\vbox{\\hrule height 1pt\\kern 2pt\\box9\\kern2pt\\hrule}\\kern3pt\\penalty7\\hskip 1pt plus 2pt\\endgroup}\n\
         \\def\\one#1{\\setbox9\\hbox{\\vrule width #1 height 2pt}\\setbox1\\hbox{\\measure{1pt}}\\showbox1 \\showbox9 }\n\
         \\one{3pt}\\one{3pt}\\one{3pt}\\one{4pt}\\one{4pt}\\one{3pt}\\end\n",
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
}

/// pdfTeX's output in INITEX: pages of rules, a link's object, a form.
const PDF_PRELUDE: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\pdfoutput=1 \\pdfpagewidth=100pt \\pdfpageheight=100pt \\pdfcompresslevel=0\n";

#[test]
fn links_replay_with_their_objects() {
    // a draw call that makes two links (beamer's navigation symbols): a
    // replay makes the links' objects again, in order, writes them into the
    // rebuilt nodes and sets \pdflastlink; the pages' annotations are the
    // normal path's, byte for byte
    let s = check(
        "links",
        &[
            PDF_PRELUDE,
            "\\def\\measure#1{\\begingroup\\pdfstartlink user{/S /URI /URI (#1)}\\vrule width 2pt height 1pt\\pdfendlink\
             \\kern1pt\\pdfstartlink attr{/Border[0 0 0]} goto page 1 {/Fit}\\vrule width 3pt height 1pt\\pdfendlink\\endgroup}\n\
             \\def\\one{\\setbox1\\hbox{\\measure{a}}\\message{[\\the\\pdflastlink]}\\shipout\\box1 }\n\
             \\one\\one\\one\\one\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn links_recorded_after_an_object_stream_began() {
    // With object streams, a shipout takes the stream's number from
    // `sys_obj_ptr` and leaves `obj_ptr` one behind (pdftex.web: "object
    // stream is not accessible to user"); the next object made puts them
    // level again. A call recorded there (beamer-30's first block frame,
    // whose footline links follow the shipout that began a stream) makes
    // its links' objects from `sys_obj_ptr` on, and is recorded; counted
    // by `obj_ptr` it seemed to make one more and was refused for good.
    let s = check(
        "links-os",
        &[
            PDF_PRELUDE,
            "\\pdfminorversion=5 \\pdfobjcompresslevel=2 \\shipout\\hbox{\\vrule width 1pt height 1pt}\n\
             \\def\\measure#1{\\begingroup\\pdfstartlink user{/S /URI /URI (#1)}\\vrule width 2pt height 1pt\\pdfendlink\
             \\kern1pt\\pdfstartlink attr{/Border[0 0 0]} goto page 1 {/Fit}\\vrule width 3pt height 1pt\\pdfendlink\\endgroup}\n\
             \\def\\one{\\setbox1\\hbox{\\measure{a}}\\message{[\\the\\pdflastlink]}\\shipout\\box1 }\n\
             \\one\\one\\one\\one\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
}

#[test]
fn form_references_are_part_of_the_key() {
    // \pdfrefxform of an existing form (a pgf shading's): the node copies
    // the form's dimensions, so the form joins the key; another form is
    // another key
    let s = check(
        "forms",
        &[
            PDF_PRELUDE,
            "\\setbox2\\hbox{\\vrule width 5pt height 2pt}\\immediate\\pdfxform2 \\edef\\f{\\the\\pdflastxform}\n\
             \\def\\measure#1{\\begingroup\\pdfrefxform\\f\\kern#1\\endgroup}\n\
             \\def\\one{\\setbox1\\hbox{\\measure{1pt}}\\shipout\\box1 }\n\
             \\one\\one\\one\n\
             \\setbox2\\hbox{\\vrule width 7pt height 2pt}\\immediate\\pdfxform2 \\edef\\f{\\the\\pdflastxform}\n\
             \\one\\one\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn lastskip_glue_and_local_groups() {
    // `\\skip0=\\lastskip` of a glue the call made (beamer's
    // `\\reset@color`), removed and put back, and a group whose
    // assignments are all local (a replay skips it); both replay to the
    // normal path's state
    let s = check(
        "lastskip",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\showboxdepth=100 \\showboxbreadth=100 \\tracingonline=0\n\
         \\def\\measure#1{\\begingroup\\begingroup\\def\\a{#1}\\count5=3 \\dimen5=\\a pt\\endgroup\
         \\hbox{\\hskip 3pt plus 1pt\\skip0=\\lastskip\\unskip\\vrule width 1pt\\hskip\\skip0}\\endgroup}\n\
         \\def\\one{\\setbox1\\hbox{\\measure{2}}\\showbox1 }\n\
         \\one\\one\\one\\end\n",
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn font_parameters_are_part_of_the_key() {
    // a call that reads a font parameter: a \fontdimen assignment makes the
    // fonts' next version, so the call after it is another key; versions
    // are named by their changes (`next_version`), and the logs agree
    let s = check(
        "fontdimen",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\measure#1{\\begingroup\\dimen0=\\fontdimen2\\nullfont\\advance\\dimen0 by #1\\global\\dimen1=\\dimen0 \\endgroup}\n\
         \\def\\show{\\message{[\\the\\dimen1]}}\n\
         \\measure{1pt}\\show\\measure{1pt}\\show\\fontdimen2\\nullfont=3pt \\measure{1pt}\\show\
         \\measure{1pt}\\show\\fontdimen2\\nullfont=4pt \\measure{1pt}\\show\\measure{1pt}\\show\\end\n",
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
    assert!(stat(&s, "recordings") >= 3, "{s}");
}
