//! tex.ch's DVI length check ([32.598] `dvi_swap`, [32.599] the last bytes,
//! [32.642] the postamble): a DVI file may not grow past `"7FFFFFFF` bytes.
//! When it would, TeX sets `cur_s:=-2` and stops with the fatal error
//! `dvi length exceeds "7FFFFFFF`; no postamble is written and the file keeps
//! what was already flushed.
//!
//! * `dvi_length_limit_against_pdftex` runs the real thing against TeX
//!   Live's pdftex: a 300000-rule box shipped out until the file passes 2 GiB.
//!   It writes about 2 GiB twice (one engine after the other, each file
//!   deleted as soon as it has been measured) and takes about a minute, so it
//!   runs only with `FLASHTEX_LONG_TESTS=1`. Terminal, log (after the
//!   banner), exit status, and the DVI file left behind (length and
//!   contents) must all be pdftex's.
//! * `crafted_dvi_offset_*` reaches both checks cheaply, in process, by
//!   moving `dvi_offset` close to the limit at a checkpoint after the first
//!   shipout. That state is not one pdfTeX can be put in, so these check
//!   only that the ported code does what tex.ch says (the run ends, exit
//!   status 1, no postamble, the file holds exactly the bytes flushed); they
//!   prove nothing about parity with pdfTeX.
//!
//! Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::arena::CheckpointId;
use flashtex_engine::checkpoint::{Action, Observer, Point};
use flashtex_engine::resolver::find_texlive_bin;
use flashtex_engine::{cli, host, system, Globals};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

const LONG: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6\n\
\\batchmode \\pdfoutput=0\n\
\\def\\a{\\vrule width 1pt height 1pt \\advance\\count2 by 1 \\ifnum\\count2<300000 \\expandafter\\a\\fi}\n\
\\setbox0\\hbox{\\count2=0 \\a}\n\
\\def\\b{\\shipout\\copy0 \\advance\\count1 by 1 \\ifnum\\count1<2000 \\expandafter\\b\\fi}\n\
\\count1=0 \\b\n\
\\end\n";

/// What one engine's run left: exit status, terminal, log after its banner,
/// and the DVI file's length and FNV-1a hash.
#[derive(Debug, PartialEq)]
struct Outcome {
    code: Option<i32>,
    terminal: String,
    log: String,
    dvi_len: u64,
    dvi_hash: u64,
}

fn fnv(path: &Path) -> u64 {
    let mut f = std::fs::File::open(path).unwrap();
    let mut h: u64 = 0xcbf29ce484222325;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            return h;
        }
        for &b in &buf[..n] {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
}

/// Run `bin` on `t.tex` in `dir`; a run that writes more than `"7FFFFFFF`
/// bytes and 64 MiB is killed (the engine before the port wrote on to
/// several GiB). The DVI file is deleted once measured.
fn run_long(bin: &Path, dir: &Path, ours: bool) -> Outcome {
    let mut c = Command::new(bin);
    c.args(["-ini", "-output-format=dvi", "t.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    let mut child = c.spawn().unwrap();
    let mut out = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut s = Vec::new();
        out.read_to_end(&mut s).unwrap();
        s
    });
    let dvi = dir.join("t.dvi");
    let cap = 0x8000_0000u64 + (64 << 20);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break Some(s);
        }
        if std::fs::metadata(&dvi).map(|m| m.len()).unwrap_or(0) > cap {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let terminal = String::from_utf8_lossy(&reader.join().unwrap()).into_owned();
    let dvi_len = std::fs::metadata(&dvi).map(|m| m.len()).unwrap_or(0);
    if status.is_none() {
        let _ = std::fs::remove_file(&dvi);
        panic!(
            "{} wrote {dvi_len} bytes of DVI, past \"7FFFFFFF, and was killed",
            bin.display()
        );
    }
    let dvi_hash = if dvi_len > 0 { fnv(&dvi) } else { 0 };
    let _ = std::fs::remove_file(&dvi);
    let log = std::fs::read_to_string(dir.join("t.log")).unwrap();
    let log = log
        .split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Outcome {
        code: status.unwrap().code(),
        terminal,
        log,
        dvi_len,
        dvi_hash,
    }
}

#[test]
fn dvi_length_limit_against_pdftex() {
    if std::env::var_os("FLASHTEX_LONG_TESTS").is_none_or(|v| v != "1") {
        eprintln!(
            "skipping: writes ~2 GiB of DVI twice and takes about a minute; \
             set FLASHTEX_LONG_TESTS=1 to run it"
        );
        return;
    }
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-texch-dvilen");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("t.tex"), LONG).unwrap();
    }
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    // One after the other: each writes ~2 GiB.
    let tex = run_long(&texbin.join("pdftex"), &b, false);
    assert!(
        tex.log.contains("dvi length exceeds \"7FFFFFFF"),
        "pdfTeX's log lacks the message:\n{}",
        tex.log
    );
    let mine = run_long(&ours, &a, true);
    eprintln!(
        "pdftex: exit {:?}, DVI {} bytes; ours: exit {:?}, DVI {} bytes",
        tex.code, tex.dvi_len, mine.code, mine.dvi_len
    );
    assert_eq!(mine, tex, "ours differs from TeX Live's pdftex");
    let _ = std::fs::remove_dir_all(&base);
}

/// What `Shift` runs once, at the first shipout checkpoint.
type ShiftFn = Box<dyn FnMut(&mut Globals)>;

/// Moves `dvi_offset` at the first checkpoint after a shipout.
struct Shift(Option<ShiftFn>);

impl Observer for Shift {
    fn on_checkpoint(&mut self, g: &mut Globals, _id: CheckpointId, why: Point) -> Action {
        if why == Point::Shipout {
            if let Some(mut f) = self.0.take() {
                f(g);
            }
        }
        Action::Continue
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

/// Run `tex` in process, with `shift` applied after the first shipout;
/// returns the exit status, the log, the DVI file's length and the engine.
fn crafted(
    dir: &Path,
    tex: &str,
    shift: impl FnMut(&mut Globals) + 'static,
) -> (i32, String, u64, Box<Globals>) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("c.tex"), tex).unwrap();
    std::env::set_current_dir(dir).unwrap();
    std::env::set_var(
        "FLASHTEX_POOL",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
    );
    let argv: Vec<String> = ["pdftex", "-ini", "-output-format=dvi", "c.tex"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let o = cli::parse(&argv);
    let line = host::first_line_of(&o);
    system::configure(o);
    system::capture_terminal(true);
    system::set_command_line(vec![line]);
    let mut g = Globals::new();
    g.checkpoint_every_shipout(true);
    g.layer().observer = Some(Box::new(Shift(Some(Box::new(shift)))));
    let status = g.run_to_end().unwrap();
    system::capture_terminal(false);
    let log = std::fs::read_to_string(dir.join("c.log")).unwrap();
    let len = std::fs::metadata(dir.join("c.dvi"))
        .map(|m| m.len())
        .unwrap_or(0);
    (status, log, len, g)
}

const HEAD: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6\n\
\\batchmode \\pdfoutput=0\n\
\\def\\a{\\vrule width 1pt height 1pt \\advance\\count2 by 1 \\ifnum\\count2<\\count3 \\expandafter\\a\\fi}\n";

/// Both cases in one test: the run changes the process's directory and
/// the engine's process-wide state.
#[test]
fn crafted_dvi_offset_reaches_both_checks() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-texch-dvilen-crafted");
    let msg = "! Emergency stop.";
    let what = "dvi length exceeds \"7FFFFFFF";

    // [32.598]: a one-rule page, then `dvi_offset` two buffers short of
    // 2^31 (a multiple of `dvi_buf_size`, as it always is), then pages of
    // 3000 rules: the second full-buffer `dvi_swap` would pass the limit.
    let tex = format!(
        "{HEAD}\\count3=1 \\setbox0\\hbox{{\\count2=0 \\a}}\\shipout\\box0\n\
         \\count3=3000 \\setbox0\\hbox{{\\count2=0 \\a}}\n\
         \\shipout\\copy0 \\shipout\\copy0 \\shipout\\copy0\n\\end\n"
    );
    let (st, log, len, g) = crafted(&base.join("swap"), &tex, |g| {
        assert_eq!(g.dvi_offset, 0);
        g.dvi_offset = 0x8000_0000u32.wrapping_sub(2 * 16384) as i32;
    });
    assert_eq!(st, 1, "{log}");
    assert!(log.contains(msg) && log.contains(what), "{log}");
    assert!(!log.contains("Output written"), "{log}");
    assert_eq!(g.cur_s, -2);
    assert_eq!(
        len as i64, g.dvi_gone as i64,
        "the file holds what was flushed"
    );
    assert_eq!(len, 16384, "{log}");
    drop(g);

    // [32.599]: a one-rule page (nothing flushed yet), then `dvi_offset`
    // 8 bytes short of the limit: the postamble crosses it without a
    // `dvi_swap`, and the check before the last bytes stops the run.
    let tex = format!("{HEAD}\\count3=1 \\setbox0\\hbox{{\\count2=0 \\a}}\\shipout\\box0\n\\end\n");
    let (st, log, len, g) = crafted(&base.join("last"), &tex, |g| {
        assert_eq!(g.dvi_offset, 0);
        g.dvi_offset = 0x7FFF_FFFF - g.dvi_ptr - 8;
    });
    assert_eq!(st, 1, "{log}");
    assert!(log.contains(msg) && log.contains(what), "{log}");
    assert!(!log.contains("Output written"), "{log}");
    assert_eq!(g.cur_s, -2);
    assert_eq!(len, 0, "nothing was flushed: {log}");
    drop(g);
    let _ = std::env::set_current_dir(std::env::temp_dir());
    let _ = std::fs::remove_dir_all(&base);
}
