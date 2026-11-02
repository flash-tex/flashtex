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

/// The calls below define `\reserved@a` before they read it, as LaTeX's
/// `\@ifnextchar` and expl3's `\tl_set:Nn` do; between the calls the
/// document gives it other meanings (BOX-MEMO.md §15).
const RESERVED: &str =
    "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\@=11 \\scrollmode\n\
    \\def\\show{\\message{[\\the\\dimen1|\\the\\count1|\\meaning\\reserved@a|\\meaning\\tmp]}}\n";

#[test]
fn a_meaning_assigned_before_it_is_read_is_not_keyed() {
    // `\def\reserved@a` first: what it held is not an input (its first
    // access is a write), so a call hits whatever the document left there;
    // its new meaning, read later in the call, is the call's own
    let s = check(
        "write-first",
        &[
            RESERVED,
            "\\def\\measure#1{\\def\\reserved@a{#1}\\begingroup\\setbox0\\hbox{\\vrule width\\reserved@a}\
             \\global\\dimen1=\\wd0 \\endgroup}\n\
             \\def\\reserved@a{x}\\measure{1pt}\\show\\def\\reserved@a{yy}\\measure{1pt}\\show\
             \\let\\reserved@a\\relax\\measure{1pt}\\show\\def\\reserved@a{2pt}\\measure{1pt}\\show\
             \\measure{2pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
    assert!(stat(&s, "k3_written_first") >= 1, "{s}");
}

#[test]
fn a_target_passed_as_an_argument_is_not_keyed() {
    // expl3's `\tl_set:Nn \l__cmd_tmpa_tl {..}`: the control sequence is
    // matched as an argument, then defined; neither reads its meaning
    let s = check(
        "write-first-arg",
        &[
            RESERVED,
            "\\def\\set#1#2{\\def#1{#2}}\n\
             \\def\\measure#1{\\set\\reserved@a{#1}\\begingroup\\setbox0\\hbox{\\vrule width\\reserved@a}\
             \\global\\dimen1=\\wd0 \\endgroup}\n\
             \\def\\reserved@a{x}\\measure{1pt}\\show\\def\\reserved@a{yy}\\measure{1pt}\\show\
             \\let\\reserved@a\\relax\\measure{1pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn a_meaning_read_before_it_is_assigned_is_keyed() {
    // `\ifx\reserved@a\relax` reads it before the `\def`, and an `\edef`
    // reads its target in its own body: both keep the meaning in the key,
    // so a call after a change is a miss (and the logs agree)
    let s = check(
        "read-first",
        &[
            RESERVED,
            "\\def\\measure#1{\\ifx\\reserved@a\\relax\\gdef\\tmp{R}\\else\\gdef\\tmp{M}\\fi\\def\\reserved@a{#1}\
             \\begingroup\\setbox0\\hbox{\\vrule width\\reserved@a}\\global\\dimen1=\\wd0 \\endgroup}\n\
             \\let\\reserved@a\\relax\\measure{1pt}\\show\\let\\reserved@a\\relax\\measure{1pt}\\show\
             \\def\\reserved@a{x}\\measure{1pt}\\show\\let\\reserved@a\\relax\\measure{1pt}\\show\n\
             \\def\\measure#1{\\edef\\tmp{\\tmp#1}\\begingroup\\setbox0\\hbox{\\vrule width 1pt}\
             \\global\\dimen1=\\wd0 \\endgroup}\n\
             \\def\\tmp{a}\\measure{b}\\show\\def\\tmp{a}\\measure{b}\\show\\def\\tmp{c}\\measure{b}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
}

#[test]
fn a_meaning_an_endgroup_brings_back_is_keyed() {
    // a local `\def` in a group of the call's own is undone at its
    // `\endgroup`: the read after it sees what the document left, which is
    // then an input of the call
    let s = check(
        "restored",
        &[
            RESERVED,
            "\\def\\measure#1{\\begingroup\\def\\reserved@a{#1}\\endgroup\
             \\begingroup\\setbox0\\hbox{\\vrule width\\reserved@a}\\global\\dimen1=\\wd0 \\endgroup}\n\
             \\def\\reserved@a{1pt}\\measure{9pt}\\show\\measure{9pt}\\show\
             \\def\\reserved@a{2pt}\\measure{9pt}\\show\\measure{9pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn an_argument_that_turns_outer_is_a_miss() {
    // `\x`, an argument of `\set` inside the call, is read only as a token:
    // the key holds that it is not `\outer` (nor `#`, `&`, `\if`, `\fi`).
    // Once it is, matching `\set`'s argument is an error, so that call must
    // not be replayed
    let s = check(
        "token-outer",
        &[
            RESERVED,
            "\\def\\set#1{\\def\\tmp{#1}}\n\
             \\def\\measure#1{\\set\\x\\begingroup\\setbox0\\hbox{\\vrule width #1}\
             \\global\\dimen1=\\wd0 \\endgroup}\n\
             \\let\\x\\relax\\measure{1pt}\\show\\measure{1pt}\\show\\def\\x{}\\measure{1pt}\\show\
             \\outer\\def\\x{}\\measure{1pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
    assert!(stat(&s, "k3_tokens") >= 1, "{s}");
}

/// The calls below assign a register inside a group of their own before
/// they read it, as pgf's `\pgfmathsetlength\pgf@x{..}` does (`\begingroup
/// \pgf@x=..\expandafter\endgroup\expandafter\pgf@x\the\pgf@x`); between
/// the calls the document leaves other values there (BOX-MEMO.md §15.8).
const REGS: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\def\\show{\\message{[\\the\\dimen1|\\the\\dimen3|\\the\\dimen300|\\the\\count3]}}\n";

#[test]
fn a_register_assigned_in_a_group_before_it_is_read_is_not_keyed() {
    // `\dimen3` (and the sparse `\dimen300`) first assigned inside the
    // call's group: what it held is not an input, and `\the\dimen300`
    // before the `\endgroup` reads the call's own value
    let s = check(
        "reg-write-first",
        &[
            REGS,
            "\\def\\measure#1{\\begingroup\\dimen3=#1\\dimen300=\\dimen3\\advance\\dimen300 1pt\
             \\expandafter\\endgroup\\expandafter\\dimen\\expandafter3\\expandafter=\\the\\dimen300\\relax\
             \\begingroup\\setbox0\\hbox{\\vrule width\\dimen3}\\global\\dimen1=\\wd0 \\endgroup}\n\
             \\dimen3=5pt \\dimen300=6pt \\measure{1pt}\\show\\dimen3=7pt \\measure{1pt}\\show\
             \\dimen300=8pt \\measure{1pt}\\show\\dimen3=9pt \\measure{2pt}\\show\\measure{2pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 3, "{s}");
}

#[test]
fn a_register_a_group_end_brings_back_is_keyed() {
    // assigned inside a group of the call, then read after the group's
    // end (`\advance` too): what the document left there is an input, so
    // a call after a change is a miss (and the logs agree)
    let s = check(
        "reg-restored",
        &[
            REGS,
            "\\def\\measure#1{\\begingroup\\dimen3=#1\\count3=2 \\endgroup\\advance\\count3 1 \
             \\begingroup\\setbox0\\hbox{\\vrule width\\dimen3}\\global\\dimen1=\\wd0 \\endgroup}\n\
             \\dimen3=1pt \\count3=0 \\measure{9pt}\\show\\count3=0 \\measure{9pt}\\show\
             \\dimen3=2pt \\count3=0 \\measure{9pt}\\show\\count3=5 \\measure{9pt}\\show\
             \\def\\measure#1{\\begingroup\\dimen3=#1\\setbox0\\hbox{\\vrule width\\dimen3}\
             \\global\\dimen1=\\wd0 \\endgroup}\n\
             \\dimen3=3pt \\measure{3pt}\\show\\dimen3=4pt \\measure{3pt}\\show\\measure{3pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn call_level_runs_aftergroup_and_fis_before_a_trailing_ignorespaces() {
    // BOX-MEMO.md §15: a call that defines one macro again and again at the
    // level it is called at (the run is replayed as one assignment), `\let`s
    // from it in between (which ends the run), saves an `\aftergroup` token
    // at that level, and ends in `\ignorespaces` before its own `\fi`. With
    // `\tracingrestores` on, the caller's group end shows every save-stack
    // entry the call left, in order, and the saved token runs after it.
    let call = "{\\measure{q} \\message{[in \\meaning\\tmp|\\meaning\\y|\\the\\count1]}}\n";
    let s = check(
        "calllevel",
        &[
            "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
             \\tracingrestores=1 \\tracingonline=0\n\
             \\def\\tmp{old}\\def\\other{old}\\let\\y\\relax\n\
             \\def\\measure#1{\\def\\tmp{a}\\def\\tmp{#1}\\def\\other{b}\\def\\tmp{#1#1}\\let\\y\\tmp\
             \\def\\tmp{c}\\def\\tmp{#1c}\\count1=5 \\count1=6 \\aftergroup\\done\\ifnum1=1 \\ignorespaces\\fi}\n\
             \\def\\done{\\message{[after \\meaning\\tmp|\\meaning\\y|\\meaning\\other|\\the\\count1]}}\n",
            call,
            call,
            call,
            call,
            "\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}
