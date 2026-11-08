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
    check_with(tag, src, &[])
}

/// [`check`] with `files` (name, bytes) next to `t.tex`.
fn check_with(tag: &str, src: &str, files: &[(&str, Vec<u8>)]) -> String {
    let d = scratch(tag);
    for (n, b) in files {
        std::fs::write(d.join(n), b).unwrap();
    }
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

#[test]
fn calls_ending_in_expansions_replay() {
    // (lane EDITED-PAGE) a body whose last tokens only expand -- LaTeX's
    // `\end`: `\if@endpe ... \fi` and its hooks after the last
    // `\endgroup` -- is used up inside `big_switch`'s `get_x_token`, which
    // then reads the token after the call: the recording ends where the
    // last level is popped (`flashtex_bm_popped`), a draw call (a frame
    // around a box it moves, followed by `\hss`) and a measure call alike,
    // and replays
    let s = check(
        "tail",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\showboxdepth=100 \\showboxbreadth=100 \\tracingonline=0 \\let\\ifendpe\\iffalse\n\
         \\def\\measure#1{\\begingroup\\vrule width#1\\vbox{\\hrule\\kern 2pt\\box9\\kern2pt\\hrule}\\endgroup\\ifendpe\\relax\\fi\\ifx aa\\fi}\n\
         \\def\\one#1{\\setbox9\\hbox{\\vrule width #1 height 2pt}\\setbox1\\hbox{\\hss\\measure{1pt}\\hss}\\showbox1 }\n\
         \\one{3pt}\\one{3pt}\\one{3pt}\\one{4pt}\\one{3pt}\n\
         \\def\\measure#1{\\begingroup\\setbox0\\hbox{\\vrule width#1}\\global\\dimen1=\\wd0 \\endgroup\\ifx aa\\fi}\n\
         \\measure{2pt}\\measure{2pt}\\measure{2pt}\\message{[\\the\\dimen1]}\\end\n",
    );
    assert!(stat(&s, "hits") >= 4, "{s}");
}

#[test]
fn a_tail_that_reads_past_the_call_never_replays() {
    // the same bodies, but their last expansion takes the token after the
    // call as a parameter (`\gobble`) or a primitive reads past it
    // (`\number` looking for more digits): the read abandons the recording
    // (`Level`), as before, and nothing replays
    let s = check(
        "tail-reads",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\gobble#1{}\n\
         \\def\\measure#1{\\begingroup\\setbox0\\hbox{\\vrule width#1}\\global\\dimen1=\\wd0 \\endgroup\\gobble}\n\
         \\measure{2pt}x\\measure{2pt}x\\measure{2pt}x\\message{[\\the\\dimen1]}\n\
         \\def\\measure#1{\\begingroup\\setbox0\\hbox{\\vrule width#1}\\global\\dimen1=\\wd0 \\endgroup\\count1=\\number 1}\n\
         \\measure{2pt}2\\measure{2pt}2\\measure{2pt}2\\message{[\\the\\dimen1|\\the\\count1]}\\end\n",
    );
    assert_eq!(stat(&s, "hits"), 0, "{s}");
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

#[test]
fn boxes_set_at_the_calls_level_replay() {
    // (lane FANCYHDR) fancyhdr's head and foot run `\normalsize` at the
    // call's own level, whose `\size@update` sets `\strutbox` there: a box
    // that outlives the call. Taken by value at the end of the call (after
    // its own changes to it: `\wd`, an inner `\setbox` undone, a second
    // `\setbox`), it is made again by the replay, and what reads it after
    // the call -- in the same box and after it, `\showbox` -- sees the
    // normal path's box
    let s = check(
        "outer-box",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\showboxdepth=100 \\showboxbreadth=100 \\tracingonline=0\n\
         \\setbox8\\hbox{\\vrule width 9pt}\n\
         \\def\\measure#1{\\setbox8\\hbox{\\vrule height 7pt depth 3pt width 0pt}\\wd8=#1\
         \\begingroup\\setbox8\\hbox{\\kern1pt}\\endgroup\\vrule width\\wd8\\copy8\
         \\setbox7\\hbox{\\kern 2pt\\vrule}\\setbox7\\hbox{\\kern 3pt\\vrule\\penalty5}\\unhcopy7 }\n\
         \\def\\one#1{\\setbox1\\hbox{\\measure{#1}\\vrule height\\ht8 depth\\dp8\\showbox8 \\showbox7 }\\showbox1 \\showbox8 \\showbox7 }\n\
         \\one{1pt}\\one{1pt}\\one{1pt}\\one{2pt}\\one{2pt}\\one{1pt}\\end\n",
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
}

#[test]
fn boxes_at_the_calls_level_not_its_own_never_replay() {
    // a box left at the call's level that is not the call's alone: one
    // moved out of a register it did not make (`\\box9`, a hole), one whose
    // glue is a register's (`\\hskip\\skip3` shares the specification),
    // and one the call moved out again (`\\box8` voids the register behind
    // `eq_define`'s back). Each is abandoned, and nothing replays
    let s = check(
        "outer-box-shared",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\showboxdepth=100 \\showboxbreadth=100 \\tracingonline=0 \\skip3=4pt plus 1pt\n\
         \\def\\measure#1{\\setbox8\\box9 \\kern#1\\relax}\n\
         \\def\\one{\\setbox9\\hbox{\\vrule width 2pt}\\setbox1\\hbox{\\measure{1pt}\\showbox8 }\\showbox1 }\n\
         \\one\\one\\one\n\
         \\def\\measure#1{\\setbox8\\hbox{\\hskip\\skip3 \\kern#1}}\n\
         \\def\\one{\\setbox1\\hbox{\\measure{1pt}\\showbox8 }\\showbox1 }\n\
         \\one\\one\\one\n\
         \\def\\measure#1{\\setbox8\\hbox{\\kern#1}\\box8 }\n\
         \\one\\one\\one\\end\n",
    );
    assert_eq!(stat(&s, "hits"), 0, "{s}");
}

#[test]
fn a_box_made_in_a_group_is_not_the_calls_after_it() {
    // (lane FANCYHDR, a hole found on the way) a box register the call sets
    // inside a group gets back its old box when the group ends; read after
    // that (`\\wd0`), it is the box of before the call, an input of the
    // call, which must be keyed as one: the later keys' box 0 is wider,
    // and the call must not replay the first width. Nested groups and
    // e-TeX's sparse registers (above 255) alike
    let s = check(
        "group-box",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\measure#1{\\begingroup{\\setbox0\\hbox{\\kern1pt}}\\global\\dimen1=\\wd0\
         \\setbox300\\hbox{\\kern2pt}{\\setbox300\\hbox{\\kern3pt}{\\setbox300\\hbox{}}}\\global\\dimen2=\\wd300\
         {\\setbox301\\hbox{\\kern2pt}}\\global\\dimen3=\\wd301 \\endgroup}\n\
         \\def\\show{\\message{[\\the\\dimen1|\\the\\dimen2|\\the\\dimen3]}}\n\
         \\setbox0\\hbox{\\vrule width 1pt}\\setbox301\\hbox{\\vrule width 7pt}\\measure{a}\\show\\measure{a}\\show\\measure{a}\\show\n\
         \\setbox0\\hbox{\\vrule width 5pt}\\measure{a}\\show\\measure{a}\\show\n\
         \\setbox301\\hbox{\\vrule width 8pt}\\measure{a}\\show\\measure{a}\\show\\end\n",
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
}

/// TeX Live's `cmr10.tfm`, or `None` (the test then passes with a note).
fn cmr10() -> Option<Vec<u8>> {
    let bin = flashtex_engine::resolver::find_texlive_bin()?;
    let out = Command::new(bin.join("kpsewhich"))
        .arg("cmr10.tfm")
        .output()
        .ok()?;
    std::fs::read(String::from_utf8_lossy(&out.stdout).trim()).ok()
}

#[test]
fn protrusion_around_a_replayed_call_is_the_normal_paths() {
    // (lane FANCYHDR) pdfTeX's protrusion keeps the character `char_pw`
    // last looked at (`last_leftmost_char`, `last_rightmost_char`). A call
    // that breaks a paragraph with protrusion on (fancyhdr's head: its
    // `\\parbox`es under microtype) leaves them set; a replay does not.
    // Every read is right after a `char_pw` on the same path (`total_pw`
    // then try_break's margin-kern variations; post_line_break's margin
    // kerns), so they are the verifier's scratch. The paragraphs broken
    // before and after the call, with protrusion and expansion, must be
    // the normal path's, and the whole state otherwise agree
    let Some(tfm) = cmr10() else {
        common::no_texlive();
        return;
    };
    let s = check_with(
        "protrusion",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\showboxdepth=100 \\showboxbreadth=1000 \\tracingonline=0 \\pdfoutput=1\n\
         \\font\\f=cmr10 \\f \\rpcode\\f`.=700 \\lpcode\\f`A=600 \\rpcode\\f`d=200\n\
         \\efcode\\f`A=1000 \\efcode\\f`d=1000 \\pdffontexpand\\f 30 30 10 autoexpand\n\
         \\pdfprotrudechars=2 \\pdfadjustspacing=2 \\parindent=0pt \\tolerance=10000 \\hsize=80pt\n\
         \\hbadness=10000 \\vbadness=10000 \\hfuzz=16383pt \\vfuzz=16383pt\n\
         \\def\\body#1{\\hsize=45pt A word and A word. #1 And A word.\\par}\n\
         \\setbox9\\vbox{\\body{x}\\body{yy}}\n\
         \\def\\measure#1{\\begingroup\\setbox0\\vbox{\\body{#1}}\\global\\dimen1=\\ht0 \\endgroup}\n\
         \\def\\one#1{\\setbox1\\vbox{A b dd. A c dd.\\par\\measure{#1}A d and A word dd. A b.\\par}\\showbox1 \\message{[\\the\\dimen1]}}\n\
         \\one{x}\\one{x}\\one{x}\\one{yy}\\one{x}\\end\n",
        &[("cmr10.tfm", tfm)],
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn calls_in_math_groups_compare_their_save_stack() {
    // (lane FANCYHDR) a call in a math group (`{...}` in math mode) of a
    // display, and of a display's `\\eqno` (cleveref's prefix under
    // `\\label` in `equation`). The full verifier walks the save stack
    // group by group as `show_save_groups` does; it took the display's
    // nest level for inline math under a math group, read the `$$` as an
    // `\\eqno` with a saved word, and misread every entry below
    let s = check(
        "math-group",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\$=3 \\scrollmode\n\
         \\hsize=100pt \\tracingonline=0\n\
         \\def\\measure#1{\\begingroup\\setbox0\\hbox{\\vrule width #1}\\global\\dimen1=\\wd0 \\endgroup}\n\
         \\def\\one{\\begingroup\\count1=1 $$ {\\measure{2pt}} \\eqno {\\measure{3pt}} $$\\endgroup\\message{[\\the\\dimen1]}}\n\
         \\one\\one\\one\\end\n",
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

/// #2211's review (REVIEW-C2 M1): a replay that swaps which of two control
/// sequences shares a third's list. Normal path: `\IntrR` shares
/// `\IntrSrc`'s list, `\IntrS` holds an equal copy; the replay
/// (FLASHTEX_BOXMEMO_FAULT=swap-share): `\IntrS` shares it, `\IntrR` holds
/// the copy. The reference counts are equal, so no word of the shared list
/// differs: only the pairing of the list with itself through `\IntrSrc`,
/// whose entry neither path wrote, shows it. The full walk finds it; the
/// restricted walk must not hold where the full one does not
/// (FLASHTEX_BOXMEMO_WALK_CHECK=1 aborts if it does).
#[test]
fn a_swapped_share_is_found_by_the_restricted_walk_too() {
    let d = scratch("swap-share");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\IntrSrc{abc}\n\
         \\def\\share#1{\\begingroup\\setbox0\\hbox{#1}\\global\\let\\IntrR\\IntrSrc\\gdef\\IntrS{abc}\\endgroup}\n\
         \\share{x}\\share{x}\\share{x}\\message{[\\meaning\\IntrR|\\meaning\\IntrS]}\\end\n",
    )
    .unwrap();
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let go = |every: &str| {
        let stats = d.join(format!("stats-{every}.txt"));
        let _ = std::fs::remove_file(&stats);
        let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
            .args(["-ini", "-etex", "\\input t"])
            .current_dir(&d)
            .env("FLASHTEX_POOL", &pool)
            .env("FLASHTEX_RESOLVER", "cwd")
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .env("FLASHTEX_BOXMEMO", "verify")
            .env("FLASHTEX_BOXMEMO_NAMES", "share")
            .env("FLASHTEX_BOXMEMO_STATS", &stats)
            .env("FLASHTEX_BOXMEMO_FAULT", "swap-share")
            .env("FLASHTEX_BOXMEMO_FULL_EVERY", every)
            .env("FLASHTEX_BOXMEMO_WALK_CHECK", "1")
            .env_remove("FLASHTEX_BOXMEMO_VERIFY_FAIL")
            .stdin(Stdio::null())
            .output()
            .expect("run flashtex-initex");
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        (
            out.status.code(),
            err,
            std::fs::read_to_string(&stats).unwrap_or_default(),
        )
    };
    // the full walk alone (as before #2211) finds the swap
    let (code, err, s) = go("1");
    assert!(stat(&s, "full_verified") >= 1, "{code:?} {err}\n{s}");
    assert!(
        stat(&s, "full_differences") >= 1,
        "the full walk missed the swap: {err}\n{s}"
    );
    // the restricted walk on every call but the first: never holds where
    // the full walk does not
    let (code, err, s) = go("1000000000");
    assert!(!err.contains("WALK_CHECK"), "{code:?}: {err}");
    assert_eq!(code, Some(0), "{err}");
    assert!(stat(&s, "full_verified") >= 2, "{err}\n{s}");
    assert_eq!(
        stat(&s, "full_differences"),
        stat(&s, "full_verified"),
        "every swapped replay is a difference: {err}\n{s}"
    );
}
