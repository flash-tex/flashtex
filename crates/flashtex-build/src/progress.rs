//! Build progress on stderr (lane CLI-PROGRESS): one status line, redrawn in
//! place while a compile runs, and a one-line summary at its end.
//!
//! ```text
//!    Compiling thesis.tex  pass 2  [=========>        ] 312/480 pages  chapters/results.tex  14.2s  ETA 6s
//!      Running bibtex thesis.aux
//!     Finished thesis.pdf — 482 pages, 3 passes, 19.8s
//! ```
//!
//! What it shows comes from the host's messages, never from the engine's
//! work itself: the `progress-v1` heartbeat (`PROGRESS`, spec §6.8: the
//! pass, the pages the run shipped and the file TeX reads, at most every
//! 250 ms at a page or segment checkpoint), `STARTED`, `DONE`, `TOOL`, and
//! the export's `PAGE`s. The expected page count is the last build's, kept
//! in a small cache file (`cache_file`), so the bar has a total from the
//! first pass on; without one it is a spinner and a count.
//!
//! When: only when stderr is a terminal (`Mode::Live`), or with
//! `--progress` (`Mode::Plain` when stderr is not a terminal: a plain line
//! per pass and tool, and one at most every 5 s). Not a terminal, or
//! `--no-progress`/`--quiet`: nothing at all, so what scripts and CI read
//! is byte for byte what it was. Never on stdout. Redrawn at 10 Hz by one
//! thread; `NO_COLOR` turns the colour off; the line fits the terminal's
//! width (`TIOCGWINSZ`, else `$COLUMNS`, else 80).

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How progress is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Nothing (not a terminal, `--no-progress`, `--quiet`).
    Off,
    /// A status line redrawn in place (stderr is a terminal).
    Live,
    /// `--progress` without a terminal: occasional plain lines.
    Plain,
}

impl Mode {
    /// `flag`: `--progress` (Some(true)) or `--no-progress` (Some(false));
    /// `quiet`: `--quiet`; `tty`: stderr is a terminal; `dumb`: `TERM=dumb`.
    pub fn choose(flag: Option<bool>, quiet: bool, tty: bool, dumb: bool) -> Mode {
        match (flag, quiet) {
            (Some(false), _) | (None, true) => Mode::Off,
            (Some(true), _) if tty && !dumb => Mode::Live,
            (Some(true), _) => Mode::Plain,
            (None, false) if tty && !dumb => Mode::Live,
            (None, false) => Mode::Off,
        }
    }

    /// The mode for this process's stderr.
    pub fn detect(flag: Option<bool>, quiet: bool) -> Mode {
        let dumb = std::env::var("TERM").is_ok_and(|t| t == "dumb");
        Mode::choose(flag, quiet, std::io::stderr().is_terminal(), dumb)
    }
}

/// What the build is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// No build: nothing drawn.
    Idle,
    /// Starting the host (it may prepare the format first).
    Host,
    /// The resident compile (and its tools' follow-ups).
    Typeset,
    /// The export run: the PDF pdflatex writes.
    Export,
}

/// The pages a pass is expected to ship: `exact` once a pass of this build
/// has shipped them all, else the last build's count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Total {
    pub pages: usize,
    pub exact: bool,
}

/// Everything one status line shows (`render` is a pure function of it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub verb: &'static str,
    pub job: String,
    /// The pass of the current compile (none before its first heartbeat).
    pub pass: Option<usize>,
    pub page: usize,
    pub total: Option<Total>,
    pub file: Option<String>,
    pub elapsed: Duration,
    pub eta: Option<Duration>,
    /// The spinner's frame, when there is no total.
    pub tick: usize,
    /// Text instead of the page count (before the first heartbeat).
    pub note: Option<&'static str>,
}

const SPIN: [char; 4] = ['-', '\\', '|', '/'];
/// The verb column, right-aligned as cargo does.
const VERB: usize = 12;

/// `14.2s`, `3m05s`.
pub fn secs(d: Duration) -> String {
    let s = d.as_secs_f64();
    if s < 60.0 {
        format!("{s:.1}s")
    } else {
        let t = d.as_secs();
        format!("{}m{:02}s", t / 60, t % 60)
    }
}

/// The bar: `[=====>    ]`, `inner` cells between the brackets.
fn bar(page: usize, total: usize, inner: usize) -> String {
    let filled = (page.min(total) * inner).checked_div(total).unwrap_or(0);
    let mut s = String::with_capacity(inner + 2);
    s.push('[');
    for i in 0..inner {
        s.push(if i < filled {
            '='
        } else if i == filled && page < total {
            '>'
        } else if i == filled {
            '='
        } else {
            ' '
        });
    }
    s.push(']');
    s
}

/// `…ters/results.tex`: the end of `s` in `n` characters.
fn tail(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len <= n {
        return s.to_string();
    }
    if n == 0 {
        return String::new();
    }
    let skip = len - (n - 1);
    std::iter::once('…').chain(s.chars().skip(skip)).collect()
}

/// The status line for `l`, at most `width - 1` characters (the last column
/// is left empty: a full line wraps on some terminals). `color`: the verb in
/// bold green (ANSI), which takes no columns.
pub fn render(l: &Line, width: usize, color: bool) -> String {
    let max = width.saturating_sub(1).max(20);
    let verb = format!("{:>VERB$}", l.verb);
    let head = format!("{verb} {}", l.job);
    let pass = l.pass.map(|p| format!("pass {p}"));
    let count = match (l.note, l.total) {
        (Some(n), _) => n.to_string(),
        (None, Some(t)) => format!(
            "{}/{}{} pages",
            l.page,
            if t.exact { "" } else { "~" },
            t.pages
        ),
        (None, None) => format!(
            "{} {} page{}",
            SPIN[l.tick % SPIN.len()],
            l.page,
            if l.page == 1 { "" } else { "s" }
        ),
    };
    let time = secs(l.elapsed);
    let eta = l.eta.map(|e| format!("ETA {}", secs(e)));
    let want_bar = l.note.is_none() && l.total.is_some_and(|t| t.pages > 0);

    // Fit: drop the ETA, then shrink and drop the bar, then shorten and drop
    // the file, then the pass.
    let width_of = |s: &str| s.chars().count();
    let assemble = |with_pass: bool, cells: Option<usize>, file: Option<&str>, with_eta: bool| {
        let mut parts: Vec<String> = vec![head.clone()];
        if let (true, Some(p)) = (with_pass, &pass) {
            parts.push(p.clone());
        }
        match (cells, l.total, want_bar) {
            (Some(cells), Some(t), true) => {
                parts.push(format!("{} {count}", bar(l.page, t.pages, cells)))
            }
            _ if count.is_empty() => {}
            _ => parts.push(count.clone()),
        }
        if let Some(f) = file {
            parts.push(f.to_string());
        }
        parts.push(time.clone());
        if let (true, Some(e)) = (with_eta, &eta) {
            parts.push(e.clone());
        }
        parts.join("  ")
    };
    // The bar's width follows the terminal's alone, so it does not jump
    // while the file name and the ETA come and go.
    let cells = [(119, 30), (99, 20), (79, 10)]
        .iter()
        .find(|(w, _)| max >= *w)
        .map(|&(_, c)| c);
    // (pass, bar cells, file: 0 none / 1 whole / 2 its end, ETA), most
    // complete first.
    let levels = [
        (true, cells, 1, true),
        (true, cells, 1, false),
        (true, cells, 2, false),
        (true, None, 2, false),
        (true, None, 0, false),
        (false, None, 0, false),
    ];
    let mut out = None;
    for (with_pass, cells, file_mode, with_eta) in levels {
        let without = assemble(with_pass, cells, None, with_eta);
        let used = width_of(&without);
        match (&l.file, file_mode) {
            (Some(f), 1 | 2) => {
                let room = max.saturating_sub(used);
                let len = width_of(f);
                if room >= 2 + len || (file_mode == 2 && room >= 2 + 12.min(len)) {
                    out = Some(assemble(
                        with_pass,
                        cells,
                        Some(&tail(f, room - 2)),
                        with_eta,
                    ));
                    break;
                }
            }
            _ if used <= max => {
                out = Some(without);
                break;
            }
            _ => {}
        }
    }
    let mut s = out.unwrap_or_else(|| assemble(false, None, None, false));
    if width_of(&s) > max {
        s = s.chars().take(max).collect();
    }
    if color && s.starts_with(&verb) {
        s = format!("\x1b[1;32m{verb}\x1b[0m{}", &s[verb.len()..]);
    }
    s
}

/// A permanent line in the verb column: `     Running bibtex thesis.aux`.
pub fn status(verb: &str, rest: &str, color: bool) -> String {
    let v = format!("{verb:>VERB$}");
    if color {
        format!("\x1b[1;32m{v}\x1b[0m {rest}")
    } else {
        format!("{v} {rest}")
    }
}

/// The summary: `thesis.pdf — 482 pages, 3 passes, 19.8s`, and the
/// warnings and errors, pointing at where they are listed.
pub fn summary(
    pdf: &str,
    pages: Option<usize>,
    passes: usize,
    elapsed: Duration,
    warnings: usize,
    errors: usize,
    check: &str,
) -> String {
    let plural = |n: usize, w: &str| format!("{n} {w}{}", if n == 1 { "" } else { "s" });
    let mut s = format!("{pdf} —");
    if let Some(p) = pages {
        s += &format!(" {},", plural(p, "page"));
    }
    if passes > 0 {
        s += &format!(" {},", plural(passes, "pass").replace("passs", "passes"));
    }
    s += &format!(" {}", secs(elapsed));
    if warnings + errors > 0 {
        let mut what = vec![];
        if errors > 0 {
            what.push(plural(errors, "error"));
        }
        if warnings > 0 {
            what.push(plural(warnings, "warning"));
        }
        s += &format!(
            "; {} ({})",
            what.join(", "),
            if errors > 0 {
                "the errors are above".to_string()
            } else {
                format!("`{check}` lists them")
            }
        );
    }
    s
}

/// The terminal's width: the window size of stderr, else `$COLUMNS`, else 80.
pub fn term_width() -> usize {
    #[cfg(unix)]
    {
        // SAFETY: TIOCGWINSZ into a zeroed winsize; stderr is a valid fd.
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(2, libc::TIOCGWINSZ, &mut ws) } == 0 && ws.ws_col > 0 {
            return ws.ws_col as usize;
        }
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .filter(|&c| c > 0)
        .unwrap_or(80)
}

/// Colour unless `NO_COLOR` is set (non-empty) or on Windows (whose older
/// consoles print the escapes).
pub fn color() -> bool {
    !cfg!(windows) && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}

/// The last build's page count for a main file is kept in
/// `<cache>/flashtex/progress/<hash>` (the user's cache directory; the
/// project gets nothing): the expected total of the next build's bar.
pub fn cache_file(main: &Path) -> Option<PathBuf> {
    let base = if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches"))
    } else if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
    }?;
    // FNV-1a of the main file's path: stable across runs and Rust versions.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in main.to_string_lossy().bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    Some(
        base.join("flashtex")
            .join("progress")
            .join(format!("{h:016x}")),
    )
}

/// The page count a cache file holds (`pages=N`).
pub fn read_expected(f: &Path) -> Option<usize> {
    let s = std::fs::read_to_string(f).ok()?;
    s.lines()
        .find_map(|l| l.strip_prefix("pages="))
        .and_then(|n| n.trim().parse().ok())
        .filter(|&n| n > 0)
}

pub fn write_expected(f: &Path, pages: usize) {
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(f, format!("pages={pages}\n"));
}

/// One build's state, shared with the drawing thread.
struct State {
    stage: Stage,
    job: String,
    root: String,
    t0: Instant,
    /// The current compile's pass and the pages it has shipped.
    pass: Option<usize>,
    page: usize,
    pass_t0: Instant,
    /// Passes of this build's earlier compiles (the tools' follow-ups).
    passes_before: usize,
    total: Option<Total>,
    file: Option<String>,
    tick: usize,
    /// Characters of the status line on screen (0: none).
    drawn: usize,
    /// `Plain`: when the last line was printed.
    plain_at: Instant,
    /// Pages the last finished compile reported (`DONE`).
    done_pages: Option<usize>,
}

/// The progress of one command's builds: `Off` costs nothing.
pub struct Progress {
    mode: Mode,
    color: bool,
    shared: Arc<Mutex<State>>,
}

impl Progress {
    pub fn new(mode: Mode) -> Progress {
        let now = Instant::now();
        let shared = Arc::new(Mutex::new(State {
            stage: Stage::Idle,
            job: String::new(),
            root: String::new(),
            t0: now,
            pass: None,
            page: 0,
            pass_t0: now,
            passes_before: 0,
            total: None,
            file: None,
            tick: 0,
            drawn: 0,
            plain_at: now,
            done_pages: None,
        }));
        let p = Progress {
            mode,
            color: mode == Mode::Live && color(),
            shared,
        };
        if mode == Mode::Live {
            let s = p.shared.clone();
            let color = p.color;
            // The only drawing: 10 Hz, whatever the events' pace.
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(100));
                let Ok(mut st) = s.lock() else { return };
                if st.stage != Stage::Idle {
                    st.tick += 1;
                    let line = render(&line_of(&st), term_width(), color);
                    draw(&mut st, &line);
                }
            });
        }
        p
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn on(&self) -> bool {
        self.mode != Mode::Off
    }

    fn with(&self, f: impl FnOnce(&mut State)) {
        if self.mode == Mode::Off {
            return;
        }
        if let Ok(mut st) = self.shared.lock() {
            f(&mut st);
        }
    }

    /// A build of `job` (the main file) in `root` starts: the host first.
    pub fn begin(&self, job: &str, root: &Path, expected: Option<usize>) {
        self.begin_at(job, root, expected, Stage::Host);
    }

    /// A build on a host already running (a watch session's next build);
    /// nothing when a build is under way (`begin` started it).
    pub fn begin_if_idle(&self, job: &str, root: &Path, expected: Option<usize>) {
        let idle =
            self.mode != Mode::Off && self.shared.lock().is_ok_and(|st| st.stage == Stage::Idle);
        if idle {
            self.begin_at(job, root, expected, Stage::Typeset);
        }
    }

    fn begin_at(&self, job: &str, root: &Path, expected: Option<usize>, stage: Stage) {
        self.with(|st| {
            let now = Instant::now();
            st.stage = stage;
            st.job = job.to_string();
            st.root = root.to_string_lossy().into_owned();
            st.t0 = now;
            st.pass = None;
            st.page = 0;
            st.pass_t0 = now;
            st.passes_before = 0;
            st.total = expected.map(|pages| Total {
                pages,
                exact: false,
            });
            st.file = None;
            st.done_pages = None;
            st.plain_at = now;
        });
    }

    /// The host listens: the build's clock keeps running; the line says
    /// "starting" until the first heartbeat.
    pub fn started(&self, mode: &str, follow_up: bool) {
        let plain = self.mode == Mode::Plain;
        self.with(|st| {
            let now = Instant::now();
            if follow_up || st.stage == Stage::Typeset && mode == "export" {
                st.passes_before += st.pass.unwrap_or(0);
                if let Some(p) = st.done_pages.filter(|&p| p > 0) {
                    st.total = Some(Total {
                        pages: p,
                        exact: true,
                    });
                }
            }
            st.stage = if mode == "export" {
                Stage::Export
            } else {
                Stage::Typeset
            };
            st.pass = None;
            st.page = 0;
            st.pass_t0 = now;
            st.file = None;
            if plain && mode == "export" {
                plain_line(st, "flashtex-v3: writing the PDF".into());
            }
        });
    }

    /// A `PROGRESS` heartbeat of the resident compile.
    pub fn heartbeat(&self, pass: usize, page: usize, file: Option<&str>) {
        let plain = self.mode == Mode::Plain;
        self.with(|st| {
            if st.stage != Stage::Typeset {
                return;
            }
            let new_pass = st.pass != Some(pass);
            if new_pass {
                // A further pass re-typesets the document: about as many
                // pages as the last pass (its last heartbeat, up to 250 ms
                // short of its end) or the last build had.
                if st.pass.is_some_and(|p| p < pass) && st.page > 0 {
                    let pages = match st.total {
                        Some(t) if t.exact => t.pages,
                        Some(t) => t.pages.max(st.page),
                        None => st.page,
                    };
                    let exact = st.total.is_some_and(|t| t.exact);
                    st.total = Some(Total { pages, exact });
                }
                st.pass_t0 = Instant::now();
                st.pass = Some(pass);
            }
            st.page = page;
            if let Some(t) = st.total.as_mut() {
                if page > t.pages {
                    t.pages = page; // more than the last build had
                }
            }
            st.file = file.map(|f| shown_file(f, &st.root));
            if plain {
                let total = st
                    .total
                    .map(|t| format!("/{}{}", if t.exact { "" } else { "~" }, t.pages))
                    .unwrap_or_default();
                let at = st
                    .file
                    .as_deref()
                    .map(|f| format!(", {f}"))
                    .unwrap_or_default();
                if new_pass {
                    let pass = st.passes_before + pass;
                    plain_line(st, format!("flashtex-v3: pass {pass}"));
                } else if st.plain_at.elapsed() >= Duration::from_secs(5) {
                    let pass = st.passes_before + pass;
                    plain_line(
                        st,
                        format!("flashtex-v3: pass {pass}, page {page}{total}{at}"),
                    );
                }
            }
        });
    }

    /// A page of the export run.
    pub fn export_page(&self) {
        self.with(|st| {
            if st.stage == Stage::Export {
                st.page += 1;
            }
        });
    }

    /// A compile's `DONE` with its page count.
    pub fn done(&self, pages: Option<usize>) {
        self.with(|st| {
            if pages.is_some() {
                st.done_pages = pages;
            }
        });
    }

    /// An external tool starts: a line of its own above the status line.
    pub fn tool(&self, tool: &str, file: &str) {
        match self.mode {
            Mode::Off => {}
            Mode::Live => self.eprintln(&status("Running", &format!("{tool} {file}"), self.color)),
            Mode::Plain => self.eprintln(&format!("flashtex-v3: running {tool} {file}")),
        }
    }

    /// A line on stderr; the status line is cleared first and redrawn by the
    /// next tick. With `Off`, exactly `eprintln!`.
    pub fn eprintln(&self, line: &str) {
        if self.mode == Mode::Off {
            eprintln!("{line}");
            return;
        }
        if let Ok(mut st) = self.shared.lock() {
            clear(&mut st);
            eprintln!("{line}");
        } else {
            eprintln!("{line}");
        }
    }

    /// The build is over: the status line goes. Returns the passes the
    /// resident compiles ran and the build's time.
    pub fn end(&self) -> (usize, Duration) {
        let mut r = (0, Duration::ZERO);
        self.with(|st| {
            clear(st);
            if st.stage == Stage::Typeset {
                st.passes_before += st.pass.unwrap_or(0);
            }
            st.stage = Stage::Idle;
            r = (st.passes_before, st.t0.elapsed());
        });
        r
    }

    /// `Finished …` in the verb column (Live).
    pub fn finished(&self, text: &str) {
        self.eprintln(&status("Finished", text, self.color));
    }
}

/// The status line's content for the state.
fn line_of(st: &State) -> Line {
    let (verb, note) = match st.stage {
        Stage::Host => ("Starting", Some("")),
        Stage::Export => ("Writing", None),
        _ => ("Compiling", st.pass.is_none().then_some("starting")),
    };
    let elapsed = st.t0.elapsed();
    let pass_elapsed = st.pass_t0.elapsed();
    let eta = match st.total {
        Some(t) if st.page > 0 && st.page < t.pages && pass_elapsed > Duration::from_secs(1) => {
            Some(pass_elapsed.mul_f64((t.pages - st.page) as f64 / st.page as f64))
        }
        _ => None,
    };
    Line {
        verb,
        job: match st.stage {
            Stage::Export => format!("{}.pdf", st.job.strip_suffix(".tex").unwrap_or(&st.job)),
            Stage::Host => "the engine host".into(),
            _ => st.job.clone(),
        },
        pass: st.pass.map(|p| st.passes_before + p),
        page: st.page,
        total: st.total,
        file: st.file.clone(),
        elapsed,
        eta,
        tick: st.tick,
        note,
    }
}

/// A file as TeX opened it, as the line shows it: relative to the project
/// (`chapters/results.tex`), else its name (`article.cls`).
pub fn shown_file(f: &str, root: &str) -> String {
    let f = f.strip_prefix("./").unwrap_or(f);
    let root = root.trim_end_matches(['/', '\\']);
    if !root.is_empty() {
        if let Some(rest) = f.strip_prefix(root) {
            if let Some(r) = rest.strip_prefix(['/', '\\']) {
                return r.to_string();
            }
        }
    }
    if Path::new(f).is_absolute() {
        return Path::new(f)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| f.to_string());
    }
    f.to_string()
}

fn draw(st: &mut State, line: &str) {
    let mut e = std::io::stderr().lock();
    // `\r`, the line, then blanks over what is left of the last one:
    // no escape sequence needed (Windows' console included).
    let shown = visible(line);
    let pad = st.drawn.saturating_sub(shown);
    let _ = write!(e, "\r{line}{:pad$}", "");
    if pad > 0 {
        let _ = write!(e, "\r{line}");
    }
    let _ = e.flush();
    st.drawn = shown;
}

fn clear(st: &mut State) {
    if st.drawn > 0 {
        let mut e = std::io::stderr().lock();
        let _ = write!(e, "\r{:w$}\r", "", w = st.drawn);
        let _ = e.flush();
        st.drawn = 0;
    }
}

fn plain_line(st: &mut State, s: String) {
    st.plain_at = Instant::now();
    eprintln!("{s}");
}

/// Characters a line takes on screen (ANSI colour sequences take none).
fn visible(s: &str) -> usize {
    let mut n = 0;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            if c == 'm' {
                esc = false;
            }
        } else if c == '\x1b' {
            esc = true;
        } else {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Line {
        Line {
            verb: "Compiling",
            job: "thesis.tex".into(),
            pass: Some(2),
            page: 312,
            total: Some(Total {
                pages: 480,
                exact: false,
            }),
            file: Some("chapters/results.tex".into()),
            elapsed: Duration::from_millis(14_200),
            eta: Some(Duration::from_millis(7_600)),
            tick: 0,
            note: None,
        }
    }

    #[test]
    fn a_wide_terminal_shows_everything() {
        let s = render(&line(), 140, false);
        assert_eq!(
            s,
            "   Compiling thesis.tex  pass 2  [===================>          ] 312/~480 pages  chapters/results.tex  14.2s  ETA 7.6s"
        );
        assert!(s.chars().count() < 140);
    }

    #[test]
    fn every_width_fits_and_drops_the_least_useful_first() {
        for w in 20..200 {
            let s = render(&line(), w, false);
            assert!(s.chars().count() < w.max(21), "width {w}: {s:?}");
            assert!(s.contains("Compiling"), "width {w}: {s:?}");
        }
        // 80 columns: no ETA and no bar before the file goes.
        let s = render(&line(), 80, false);
        assert!(!s.contains("ETA") && s.contains("results.tex"), "{s}");
        let s = render(&line(), 110, false);
        assert!(s.contains("[") && s.contains("results.tex"), "{s}");
        // Narrow: the file goes before the count.
        let s = render(&line(), 50, false);
        assert!(s.contains("312/~480 pages") && !s.contains('['), "{s}");
    }

    #[test]
    fn without_a_total_a_spinner_and_a_count() {
        let mut l = line();
        l.total = None;
        l.eta = None;
        l.tick = 1;
        let s = render(&l, 120, false);
        assert_eq!(
            s,
            "   Compiling thesis.tex  pass 2  \\ 312 pages  chapters/results.tex  14.2s"
        );
        l.page = 1;
        assert!(render(&l, 120, false).contains("\\ 1 page "));
    }

    #[test]
    fn an_exact_total_has_no_tilde_and_a_full_bar_ends_in_equals() {
        let mut l = line();
        l.total = Some(Total {
            pages: 480,
            exact: true,
        });
        l.page = 480;
        let s = render(&l, 140, false);
        assert!(s.contains(&format!("[{}]", "=".repeat(30))), "{s}");
        assert!(s.contains("480/480 pages"), "{s}");
    }

    #[test]
    fn colour_is_only_on_the_verb_and_takes_no_columns() {
        let plain = render(&line(), 100, false);
        let col = render(&line(), 100, true);
        assert!(col.starts_with("\x1b[1;32m   Compiling\x1b[0m"));
        assert_eq!(visible(&col), plain.chars().count());
    }

    #[test]
    fn a_long_file_name_keeps_its_end() {
        let mut l = line();
        l.file = Some("a/very/long/path/to/some/deeply/nested/chapter/results.tex".into());
        let s = render(&l, 100, false);
        assert!(s.contains("…") && s.contains("results.tex"), "{s}");
        assert!(s.chars().count() < 100);
    }

    #[test]
    fn modes() {
        // Not a terminal: nothing unless asked.
        assert_eq!(Mode::choose(None, false, false, false), Mode::Off);
        assert_eq!(Mode::choose(Some(true), false, false, false), Mode::Plain);
        // A terminal: live, unless off, quiet or dumb.
        assert_eq!(Mode::choose(None, false, true, false), Mode::Live);
        assert_eq!(Mode::choose(None, true, true, false), Mode::Off);
        assert_eq!(Mode::choose(Some(false), false, true, false), Mode::Off);
        assert_eq!(Mode::choose(None, false, true, true), Mode::Off);
        assert_eq!(Mode::choose(Some(true), false, true, true), Mode::Plain);
        // --progress wins over --quiet.
        assert_eq!(Mode::choose(Some(true), true, true, false), Mode::Live);
    }

    #[test]
    fn summaries() {
        let d = Duration::from_millis(19_800);
        assert_eq!(
            summary(
                "thesis.pdf",
                Some(482),
                3,
                d,
                0,
                0,
                "flashtex-v3 check thesis.tex"
            ),
            "thesis.pdf — 482 pages, 3 passes, 19.8s"
        );
        assert_eq!(
            summary("a.pdf", Some(1), 1, d, 2, 0, "flashtex-v3 check a.tex"),
            "a.pdf — 1 page, 1 pass, 19.8s; 2 warnings (`flashtex-v3 check a.tex` lists them)"
        );
        assert_eq!(
            summary("a.pdf", None, 0, d, 1, 1, "c"),
            "a.pdf — 19.8s; 1 error, 1 warning (the errors are above)"
        );
        assert_eq!(secs(Duration::from_secs(185)), "3m05s");
    }

    #[test]
    fn files_are_shown_relative_to_the_project() {
        assert_eq!(shown_file("./chapters/a.tex", "/p"), "chapters/a.tex");
        assert_eq!(shown_file("/p/chapters/a.tex", "/p/"), "chapters/a.tex");
        assert_eq!(
            shown_file(
                "/usr/local/texlive/2026/texmf-dist/tex/latex/base/article.cls",
                "/p"
            ),
            "article.cls"
        );
        assert_eq!(shown_file("/pq/x.tex", "/p"), "x.tex");
    }

    #[test]
    fn the_cache_holds_the_last_page_count() {
        let d = std::env::temp_dir().join(format!("ftx-progress-test-{}", std::process::id()));
        let f = d.join("x/y");
        assert_eq!(read_expected(&f), None);
        write_expected(&f, 482);
        assert_eq!(read_expected(&f), Some(482));
        let _ = std::fs::remove_dir_all(&d);
        let a = cache_file(Path::new("/p/a.tex"));
        let b = cache_file(Path::new("/p/b.tex"));
        assert!(a.is_none() || a != b);
    }

    #[test]
    fn off_records_nothing_and_draws_nothing() {
        let p = Progress::new(Mode::Off);
        p.begin("a.tex", Path::new("/p"), Some(3));
        p.started("resident", false);
        p.heartbeat(1, 2, Some("/p/a.tex"));
        assert_eq!(p.end(), (0, Duration::ZERO));
    }

    #[test]
    fn passes_and_totals_follow_the_heartbeats() {
        let p = Progress::new(Mode::Plain);
        p.begin("a.tex", Path::new("/p"), Some(10));
        p.started("resident", false);
        p.heartbeat(1, 4, Some("/p/ch/1.tex"));
        {
            let st = p.shared.lock().unwrap();
            let l = line_of(&st);
            assert_eq!(
                l.total,
                Some(Total {
                    pages: 10,
                    exact: false
                })
            );
            assert_eq!(l.file.as_deref(), Some("ch/1.tex"));
        }
        p.heartbeat(1, 12, None);
        p.heartbeat(2, 1, None);
        {
            let st = p.shared.lock().unwrap();
            assert_eq!(
                line_of(&st).total,
                Some(Total {
                    pages: 12,
                    exact: false
                })
            );
            assert_eq!(line_of(&st).pass, Some(2));
        }
        p.done(Some(12));
        // The tools' follow-up: its passes count after the first compile's.
        p.started("resident", true);
        p.heartbeat(1, 3, None);
        assert_eq!(line_of(&p.shared.lock().unwrap()).pass, Some(3));
        p.done(Some(12));
        p.started("export", false);
        p.export_page();
        {
            let st = p.shared.lock().unwrap();
            let l = line_of(&st);
            assert_eq!((l.verb, l.page, l.job.as_str()), ("Writing", 1, "a.pdf"));
        }
        assert_eq!(p.end().0, 3);
    }
}
