//! External tools for the engine host (lane P5-EXTERNAL-TOOLS, owner
//! decision 2026-09-30 "4A"): bibtex, biber and makeindex from the user's
//! TeX Live, run for a compile when latexmk would run them, and folded into
//! the incremental compile (DESIGN.md §4.4, §4.5, §5.5).
//!
//! **When (latexmk's rules, `latexmk.pl` 4.87 `rdb_set_latex_deps`,
//! `parse_aux`, `parse_bcf`, `rdb_rerun_needed`).** After a compile:
//!
//! * **biber** when the run wrote `JOB.bcf` (biblatex): its sources are the
//!   `.bcf` and the data sources it names (`<bcf:datasource type="file"
//!   datatype="bibtex">`). No bibtex rule then.
//! * **bibtex**, otherwise, for each `.bbl` the run read or looked for
//!   (`\bibliography`: `\@input@{\jobname.bbl}`; bibunits, chapterbib) whose
//!   `.aux` the run wrote and names `\bibdata`: its sources are the `.aux`
//!   (with the `.aux` files it `\@input`s), the `.bib` files of `\bibdata`
//!   and the `.bst` of `\bibstyle`. Only the lines bibtex reads
//!   (`\citation`, `\bibdata`, `\bibstyle`, `\@input`) count: bibtex ignores
//!   the rest, so its `.bbl` is a function of them and of the files.
//! * **makeindex** for each `.idx` the run wrote: `makeindex -o X.ind X.idx`.
//!
//! A rule runs when its sources' state differs from the state of its last
//! run (latexmk compares checksums the same way), or its output is missing
//! or not what that run made. Like latexmk's default (`$bibtex_use = 1`),
//! bibtex and biber are **not** run when a `.bib` file they need does not
//! exist: the document then uses the `.bbl` it has (an arXiv source ships
//! one). After the tools, the host compiles again (the changed `.bbl` or
//! `.ind` is a changed input: the resident engine restarts before its
//! first read, L3, and the `.aux` passes follow, L5), and then asks again,
//! until nothing changes or `MAX_ROUNDS` rounds.
//!
//! **How.** Never on the keystroke path: the engine thread only copies the
//! few files a rule reads (the `.aux` chain, the `.bcf`, the `.idx`: a
//! consistent snapshot, since the next compile rewrites them) after `DONE`
//! is out, and a worker thread does the rest: checksums, the run (in a
//! scratch directory, with a timeout, its terminal captured), and the
//! installation of the outputs (an atomic rename into the output directory,
//! and only when they changed). The worker reports `TOOL` messages and the
//! tools' warnings and errors as `DIAGNOSTIC`s to the client, then asks the
//! engine thread for the follow-up compile. The programs run as latexmk
//! runs them (`$bibtex_fudge`, `$makeindex_fudge`): bibtex and makeindex in
//! the directory of the `.aux`/`.idx` with `BIBINPUTS`/`BSTINPUTS` starting
//! with the project directory; biber with `--input-directory` the project.
//!
//! **Trust (DESIGN.md §4.5).** External programs are off unless the client
//! says `"external_tools": "auto"` in its `COMPILE` (the app does so for a
//! trusted project) or the host was started with `--external-tools auto`.
//! When off, the host says what it would have run (`TOOL` `skip`).

use super::server::{self, Conn};
use crate::persist::hash128;
use crate::system::ReadLog;
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::kind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Tool rounds (tools, then a compile) after one compile of the client's,
/// at most: latexmk's `$max_repeat`.
pub const MAX_ROUNDS: usize = 5;

/// Diagnostics sent per tool run, at most.
const MAX_DIAGNOSTICS: usize = 50;

/// Whether the host may run external programs for a compile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Policy {
    Off,
    Auto,
}

impl Policy {
    pub fn parse(s: &str) -> Option<Policy> {
        match s {
            "off" => Some(Policy::Off),
            "auto" => Some(Policy::Auto),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Policy::Off => "off",
            Policy::Auto => "auto",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tool {
    Bibtex,
    Biber,
    Makeindex,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Bibtex => "bibtex",
            Tool::Biber => "biber",
            Tool::Makeindex => "makeindex",
        }
    }

    /// The extension of the file the rule makes, and of its log.
    fn outputs(self) -> (&'static str, &'static str) {
        match self {
            Tool::Bibtex | Tool::Biber => ("bbl", "blg"),
            Tool::Makeindex => ("ind", "ilg"),
        }
    }
}

/// The user's TeX Live programs (found as the engine finds TeX Live,
/// without a shell environment: `crate::resolver::discover_texlive`).
#[derive(Clone, Debug, Default)]
pub struct Programs {
    pub bin: Option<PathBuf>,
    pub bibtex: Option<PathBuf>,
    pub biber: Option<PathBuf>,
    pub makeindex: Option<PathBuf>,
    pub kpsewhich: Option<PathBuf>,
}

impl Programs {
    pub fn discover() -> Programs {
        let Some(bin) = crate::resolver::find_texlive_bin() else {
            return Programs::default();
        };
        let get = |n: &str| Some(bin.join(n)).filter(|p| p.is_file());
        Programs {
            bibtex: get("bibtex"),
            biber: get("biber"),
            makeindex: get("makeindex"),
            kpsewhich: get("kpsewhich"),
            bin: Some(bin),
        }
    }

    fn get(&self, t: Tool) -> Option<&PathBuf> {
        match t {
            Tool::Bibtex => self.bibtex.as_ref(),
            Tool::Biber => self.biber.as_ref(),
            Tool::Makeindex => self.makeindex.as_ref(),
        }
    }

    /// For `HELLO.texmf.tools`.
    pub fn json(&self) -> Json {
        let p = |x: &Option<PathBuf>| {
            x.as_ref()
                .map(|p| js(p.display().to_string()))
                .unwrap_or(Json::Null)
        };
        obj([
            ("bibtex", p(&self.bibtex)),
            ("biber", p(&self.biber)),
            ("makeindex", p(&self.makeindex)),
        ])
    }
}

/// The host's settings for external tools.
#[derive(Clone, Debug)]
pub struct Config {
    pub programs: Programs,
    /// For a `COMPILE` that does not say (`external_tools`).
    pub default: Policy,
    /// Per tool run.
    pub timeout: Duration,
}

/// One rule: a tool and the file it runs on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuleKey {
    pub tool: Tool,
    /// The source's base name relative to the output directory, without
    /// its extension (`main` for `main.aux`, `main.bcf`, `main.idx`).
    pub base: String,
}

impl RuleKey {
    fn source(&self) -> String {
        let ext = match self.tool {
            Tool::Bibtex => "aux",
            Tool::Biber => "bcf",
            Tool::Makeindex => "idx",
        };
        format!("{}.{ext}", self.base)
    }
}

/// A rule as a compile left it: its sources, copied.
#[derive(Clone, Debug)]
struct Rule {
    key: RuleKey,
    /// What the tool reads of the files the engine writes (the lines
    /// bibtex reads of the `.aux` chain; the `.bcf`; the `.idx`).
    relevant: Vec<u8>,
    /// Files to put in the scratch directory: (path relative to the
    /// output directory, content).
    files: Vec<(String, Vec<u8>)>,
    /// Data files the tool reads, by the names the document gives
    /// (`\bibdata`, `bcf:datasource`), and the style (`\bibstyle`).
    bib: Vec<String>,
    bst: Option<String>,
}

/// What a compile left for the tools (taken on the engine thread).
#[derive(Clone, Debug)]
pub struct Snapshot {
    root: PathBuf,
    out: PathBuf,
    rules: Vec<Rule>,
}

impl Snapshot {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

fn read(p: &Path) -> Option<Vec<u8>> {
    std::fs::read(p).ok()
}

/// `path` (as the run named it) relative to `out`, if it is there.
fn rel_to(root: &Path, out: &Path, path: &str) -> Option<String> {
    let p = Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(path.strip_prefix("./").unwrap_or(path))
    };
    let r = abs.strip_prefix(out).ok()?;
    let s = r.to_string_lossy().into_owned();
    (!s.is_empty()).then_some(s)
}

/// The `.aux` lines bibtex reads, and the files it `\@input`s, from `out`.
/// `\@input` paths are relative to the directory bibtex runs in (the
/// output directory).
fn aux_chain(
    out: &Path,
    rel: &str,
    relevant: &mut Vec<u8>,
    files: &mut Vec<(String, Vec<u8>)>,
    bib: &mut Vec<String>,
    bst: &mut Option<String>,
    depth: usize,
) {
    if depth > 20 || files.iter().any(|(r, _)| r == rel) {
        return;
    }
    let Some(d) = read(&out.join(rel)) else {
        return;
    };
    for line in d.split(|&c| c == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let arg = |p: &[u8]| -> Option<String> {
            let rest = line.strip_prefix(p)?;
            let end = rest.iter().rposition(|&c| c == b'}')?;
            Some(String::from_utf8_lossy(&rest[..end]).into_owned())
        };
        if line.starts_with(b"\\citation{") {
            relevant.extend_from_slice(line);
            relevant.push(b'\n');
        } else if let Some(a) = arg(b"\\bibdata{") {
            relevant.extend_from_slice(line);
            relevant.push(b'\n');
            for n in a.split(',') {
                if !bib.iter().any(|b| b == n) {
                    bib.push(n.to_string());
                }
            }
        } else if let Some(a) = arg(b"\\bibstyle{") {
            relevant.extend_from_slice(line);
            relevant.push(b'\n');
            *bst = Some(a);
        } else if let Some(a) = arg(b"\\@input{") {
            relevant.extend_from_slice(line);
            relevant.push(b'\n');
            files.push((rel.to_string(), d.clone()));
            aux_chain(out, &a, relevant, files, bib, bst, depth + 1);
            continue;
        }
    }
    if !files.iter().any(|(r, _)| r == rel) {
        files.push((rel.to_string(), d));
    }
}

/// The data sources a `.bcf` names (`parse_bcf`).
fn bcf_sources(d: &[u8]) -> Vec<String> {
    let t = String::from_utf8_lossy(d);
    let mut v = vec![];
    for l in t.lines() {
        let l = l.trim();
        if !l.starts_with("<bcf:datasource") || !l.contains("type=\"file\"") {
            continue;
        }
        if !l.contains("datatype=\"bibtex\"") {
            continue;
        }
        let (Some(a), Some(b)) = (l.find('>'), l.rfind("</bcf:datasource>")) else {
            continue;
        };
        if a + 1 <= b {
            let n = l[a + 1..b].to_string();
            if !v.contains(&n) {
                v.push(n);
            }
        }
    }
    v
}

/// What the tools may need after a compile of `jobname` whose output
/// directory is `out` (relative paths are the project's), from the run's
/// journal: the files it wrote and the `.bbl` files it read or looked for.
pub fn snapshot(root: &Path, out: &Path, jobname: &str, journal: Option<&ReadLog>) -> Snapshot {
    let out = if out.is_absolute() {
        out.to_path_buf()
    } else {
        root.join(out)
    };
    let mut rules = vec![];
    let Some(j) = journal else {
        return Snapshot {
            root: root.to_path_buf(),
            out,
            rules,
        };
    };
    let written: Vec<String> = j
        .outputs
        .iter()
        .filter_map(|p| rel_to(root, &out, p))
        .collect();
    let bcf = format!("{jobname}.bcf");
    if written.contains(&bcf) {
        if let Some(d) = read(&out.join(&bcf)) {
            let bib = bcf_sources(&d);
            rules.push(Rule {
                key: RuleKey {
                    tool: Tool::Biber,
                    base: jobname.to_string(),
                },
                relevant: d.clone(),
                files: vec![(bcf, d)],
                bib,
                bst: None,
            });
        }
    } else {
        // The .bbl files the run read or looked for, by base name.
        let mut bbls: Vec<String> = vec![];
        let mut note = |name: &str| {
            let n = name.strip_prefix("./").unwrap_or(name);
            let Some(b) = n.strip_suffix(".bbl") else {
                return;
            };
            let base = match rel_to(root, &out, n) {
                Some(r) => r.strip_suffix(".bbl").unwrap_or(&r).to_string(),
                None if !Path::new(n).is_absolute() => b.to_string(),
                None => return,
            };
            if !bbls.contains(&base) {
                bbls.push(base);
            }
        };
        for l in &j.lookups {
            note(&l.name);
        }
        for f in &j.files {
            note(&f.path);
        }
        for base in bbls {
            let aux = format!("{base}.aux");
            if !written.contains(&aux) {
                continue; // a foreign .bbl (latexmk: not this run's .aux)
            }
            let (mut relevant, mut files, mut bib, mut bst) = (vec![], vec![], vec![], None);
            aux_chain(&out, &aux, &mut relevant, &mut files, &mut bib, &mut bst, 0);
            if bib.is_empty() {
                continue; // no \bibdata: nothing for bibtex
            }
            rules.push(Rule {
                key: RuleKey {
                    tool: Tool::Bibtex,
                    base,
                },
                relevant,
                files,
                bib,
                bst,
            });
        }
    }
    for w in &written {
        let Some(base) = w.strip_suffix(".idx") else {
            continue;
        };
        if let Some(d) = read(&out.join(w)) {
            rules.push(Rule {
                key: RuleKey {
                    tool: Tool::Makeindex,
                    base: base.to_string(),
                },
                relevant: d.clone(),
                files: vec![(w.clone(), d)],
                bib: vec![],
                bst: None,
            });
        }
    }
    Snapshot {
        root: root.to_path_buf(),
        out,
        rules,
    }
}

/// What the host remembers of a document's tool runs (shared by the engine
/// thread and the worker).
#[derive(Default, Debug)]
pub struct Memory {
    /// Per rule: the state of its sources at its last run, and the hash of
    /// the output it made (or `None`: it made none).
    last: HashMap<RuleKey, ([u64; 2], Option<[u64; 2]>)>,
    /// Per rule: the state last reported as skipped (policy off, a missing
    /// `.bib`), so that each is said once.
    said: HashMap<RuleKey, [u64; 2]>,
    /// kpsewhich results: (format, name) -> path.
    found: HashMap<(String, String), Option<PathBuf>>,
}

/// One tool run, as the engine thread learns of it.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub key: RuleKey,
    pub status: String,
    /// The installed output differs from what was there.
    pub changed: bool,
}

/// What the worker tells the engine thread.
pub(crate) struct Report {
    pub outcomes: Vec<Outcome>,
}

static SCRATCH: AtomicU64 = AtomicU64::new(0);

/// A worker's job: one snapshot, for one client's compile.
pub(crate) struct Job {
    pub snap: Snapshot,
    pub policy: Policy,
    pub cfg: Arc<Config>,
    pub memory: Arc<Mutex<Memory>>,
    pub conn: Arc<Conn>,
    pub id: i64,
}

impl Job {
    fn tool_msg(&self, kv: Vec<(&str, Json)>) {
        if self.conn.minor < 2 {
            return; // a 3.2 message
        }
        let mut v = vec![("id".to_string(), Json::Int(self.id))];
        v.extend(kv.into_iter().map(|(k, j)| (k.to_string(), j)));
        server::send_json(&self.conn.out, kind::TOOL, &Json::Obj(v));
    }

    /// Where a data file the document names is found, as the tool would
    /// find it: the project directory, then the output directory (latexmk
    /// runs bibtex there with `BIBINPUTS` starting with the project), then
    /// kpathsea's path for `format`.
    fn find(&self, name: &str, ext: &str, format: &str) -> Option<PathBuf> {
        let with = if name.ends_with(&format!(".{ext}")) {
            name.to_string()
        } else {
            format!("{name}.{ext}")
        };
        let p = Path::new(&with);
        if p.is_absolute() {
            return Some(p.to_path_buf()).filter(|p| p.is_file());
        }
        for d in [&self.snap.root, &self.snap.out] {
            let c = d.join(p);
            if c.is_file() {
                return Some(c);
            }
        }
        let key = (format.to_string(), with.clone());
        if let Some(r) = self.memory.lock().unwrap().found.get(&key) {
            if r.as_ref().is_none_or(|p| p.is_file()) {
                return r.clone();
            }
        }
        let r = self.cfg.programs.kpsewhich.as_ref().and_then(|k| {
            let o = Command::new(k)
                .arg(format!("-format={format}"))
                .arg(&with)
                .current_dir(&self.snap.root)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()?;
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (!s.is_empty()).then(|| {
                let p = PathBuf::from(&s);
                if p.is_absolute() {
                    p
                } else {
                    self.snap.root.join(p)
                }
            })
        });
        self.memory.lock().unwrap().found.insert(key, r.clone());
        r
    }

    /// The state of a rule's sources, and the `.bib` files that are
    /// missing.
    fn state(&self, r: &Rule) -> ([u64; 2], Vec<String>) {
        let mut h = r.relevant.clone();
        let mut missing = vec![];
        let mut add = |name: &str, found: Option<PathBuf>| {
            h.extend_from_slice(name.as_bytes());
            h.push(0);
            match found.and_then(|p| read(&p).map(|d| (p, d))) {
                Some((p, d)) => {
                    h.extend_from_slice(p.to_string_lossy().as_bytes());
                    h.extend_from_slice(&hash128(&d)[0].to_le_bytes());
                    h.extend_from_slice(&hash128(&d)[1].to_le_bytes());
                }
                None => missing.push(name.to_string()),
            }
        };
        for b in &r.bib {
            let found = if r.key.tool == Tool::Biber {
                // biber: relative to --input-directory (the project), then
                // kpathsea
                let p = Path::new(b);
                let c = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    self.snap.root.join(p)
                };
                if c.is_file() {
                    Some(c)
                } else {
                    self.find(b, "bib", "bib")
                }
            } else {
                self.find(b, "bib", "bib")
            };
            add(b, found);
        }
        if let Some(s) = &r.bst {
            let found = self.find(s, "bst", "bst");
            let n = format!("\u{1}{s}");
            add(&n, found);
            // A missing style is bibtex's error to report, not a veto.
            missing.retain(|m| m != &n);
        }
        (hash128(&h), missing)
    }

    /// Plan and run the snapshot's rules; report to the client as they go.
    pub fn run(self) -> Report {
        let mut outcomes = vec![];

        for r in &self.snap.rules {
            let (state, missing) = self.state(r);
            let (ext, _) = r.key.tool.outputs();
            let out_file = self.snap.out.join(format!("{}.{ext}", r.key.base));
            let have = read(&out_file).map(|d| hash128(&d));
            let last = self.memory.lock().unwrap().last.get(&r.key).cloned();
            let reason = match last {
                None => "first run".to_string(),
                Some((s, _)) if s != state => "its sources changed".to_string(),
                Some((_, made)) if made.is_some() && made != have => {
                    format!("{} changed or is missing", out_file.display())
                }
                Some(_) => continue, // up to date
            };

            let file = js(r.key.source());
            let tool = js(r.key.tool.name());
            let skip = |why: String| {
                let mut m = self.memory.lock().unwrap();
                if m.said.get(&r.key) == Some(&state) {
                    return;
                }
                m.said.insert(r.key.clone(), state);
                drop(m);
                self.tool_msg(vec![
                    ("event", js("skip")),
                    ("tool", tool.clone()),
                    ("file", file.clone()),
                    ("reason", js(why)),
                ]);
            };
            if self.policy == Policy::Off {
                skip(format!(
                    "{} would run ({reason}), but external tools are off for this project",
                    r.key.tool.name()
                ));
                continue;
            }
            if !missing.is_empty() {
                // latexmk's $bibtex_use = 1: not while a .bib file is missing
                skip(format!("not found: {}", missing.join(", ")));
                continue;
            }
            let Some(prog) = self.cfg.programs.get(r.key.tool).cloned() else {
                skip(format!(
                    "{} is not in the TeX Live the host found",
                    r.key.tool.name()
                ));
                // remembered: not retried until the sources change
                self.memory
                    .lock()
                    .unwrap()
                    .last
                    .insert(r.key.clone(), (state, None));
                continue;
            };
            self.tool_msg(vec![
                ("event", js("run")),
                ("tool", tool.clone()),
                ("file", file.clone()),
                ("reason", js(reason)),
            ]);
            let o = self.run_one(r, &prog);
            let made = read(&out_file).map(|d| hash128(&d));
            self.memory
                .lock()
                .unwrap()
                .last
                .insert(r.key.clone(), (state, made));
            outcomes.push(o);
        }
        Report { outcomes }
    }

    fn run_one(&self, r: &Rule, prog: &Path) -> Outcome {
        let t0 = Instant::now();
        let n = SCRATCH.fetch_add(1, Ordering::Relaxed);
        let scratch =
            std::env::temp_dir().join(format!("flashtex-tools-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let mut problem = None;
        for (rel, d) in &r.files {
            let p = scratch.join(rel);
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&p, d) {
                problem = Some(format!("{}: {e}", p.display()));
            }
        }
        let (ext, log_ext) = r.key.tool.outputs();
        let base = &r.key.base;
        let mut cmd = Command::new(prog);
        match r.key.tool {
            Tool::Bibtex => {
                cmd.arg(base);
                let root = self.snap.root.display().to_string();
                let out = self.snap.out.display().to_string();
                for var in ["BIBINPUTS", "BSTINPUTS"] {
                    // latexmk's path_fudge: the document's directory first,
                    // then the default path (a trailing separator)
                    let old = std::env::var(var).unwrap_or_default();
                    cmd.env(var, format!("{root}:{out}:{old}"));
                }
            }
            Tool::Biber => {
                cmd.arg("--input-directory")
                    .arg(&self.snap.root)
                    .arg("--output-directory")
                    .arg(&scratch)
                    .arg(scratch.join(r.key.source()));
            }
            Tool::Makeindex => {
                cmd.arg("-o")
                    .arg(format!("{base}.{ext}"))
                    .arg(format!("{base}.idx"));
            }
        }
        if let Some(bin) = &self.cfg.programs.bin {
            let old = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("{}:{old}", bin.display()));
        }
        let term = scratch.join(".terminal");
        let result = match (problem, std::fs::File::create(&term)) {
            (Some(p), _) => Err(p),
            (None, Err(e)) => Err(e.to_string()),
            (None, Ok(f)) => {
                let f2 = f.try_clone().ok();
                cmd.current_dir(&scratch)
                    .stdin(Stdio::null())
                    .stdout(Stdio::from(f));
                if let Some(f2) = f2 {
                    cmd.stderr(Stdio::from(f2));
                }
                run_with_timeout(&mut cmd, self.cfg.timeout)
            }
        };
        let terminal = std::fs::read_to_string(&term).unwrap_or_default();
        let (status, exit_code) = match &result {
            Ok(Some(c)) => (if *c == 0 { "ok" } else { "error" }, Some(*c)),
            Ok(None) => ("timeout", None),
            Err(_) => ("failed", None),
        };
        // Install what it made (not after a timeout: it may be partial).
        let mut changed = false;
        let log_path = self.snap.out.join(format!("{base}.{log_ext}"));
        let mut log_text = String::new();
        if status != "timeout" {
            for e in [ext, log_ext] {
                let made = scratch.join(format!("{base}.{e}"));
                let Some(d) = read(&made) else { continue };
                if e == log_ext {
                    log_text = String::from_utf8_lossy(&d).into_owned();
                }
                let dest = self.snap.out.join(format!("{base}.{e}"));
                if read(&dest).as_deref() == Some(&d[..]) {
                    continue;
                }
                if install(&dest, &d).is_ok() && e == ext {
                    changed = true;
                }
            }
        }
        let _ = std::fs::remove_dir_all(&scratch);
        let diags = match r.key.tool {
            Tool::Bibtex => bibtex_messages(&log_text),
            Tool::Biber => biber_messages(&log_text),
            Tool::Makeindex => makeindex_messages(&log_text),
        };
        let (mut warnings, mut errors) = (0, 0);
        for d in &diags {
            if d.error {
                errors += 1;
            } else {
                warnings += 1;
            }
        }
        for d in diags.iter().take(MAX_DIAGNOSTICS) {
            let mut kv = vec![
                ("id".to_string(), Json::Int(self.id)),
                (
                    "severity".to_string(),
                    js(if d.error { "error" } else { "warning" }),
                ),
                (
                    "message".to_string(),
                    js(format!("{}: {}", r.key.tool.name(), d.message)),
                ),
                ("source".to_string(), js(r.key.tool.name())),
            ];
            if let Some(f) = &d.file {
                let p = Path::new(f);
                let abs = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    self.snap.root.join(p)
                };
                kv.push(("file".into(), js(abs.display().to_string())));
            }
            if let Some(l) = d.line {
                kv.push(("line".into(), Json::Int(l)));
            }
            server::send_json(&self.conn.out, kind::DIAGNOSTIC, &Json::Obj(kv));
        }
        let status = match status {
            "ok" if errors > 0 => "errors",
            "ok" if warnings > 0 => "warnings",
            "error" if errors == 0 && diags.is_empty() => "error",
            "error" => "errors",
            s => s,
        };
        let mut kv = vec![
            ("event", js("done")),
            ("tool", js(r.key.tool.name())),
            ("file", js(r.key.source())),
            ("status", js(status)),
            (
                "exit_code",
                exit_code.map(|c| Json::Int(c as i64)).unwrap_or(Json::Null),
            ),
            (
                "ms",
                Json::Num((t0.elapsed().as_secs_f64() * 1e4).round() / 10.0),
            ),
            ("changed", Json::Bool(changed)),
            ("warnings", Json::Int(warnings)),
            ("errors", Json::Int(errors)),
            (
                "log",
                if log_path.is_file() {
                    js(log_path.display().to_string())
                } else {
                    Json::Null
                },
            ),
        ];
        match &result {
            Err(e) => kv.push(("message", js(e.as_str()))),
            Ok(None) => kv.push((
                "message",
                js(format!(
                    "stopped after {} s",
                    self.cfg.timeout.as_secs_f64()
                )),
            )),
            Ok(Some(c)) if *c != 0 && log_text.is_empty() => kv.push((
                "message",
                js(terminal
                    .chars()
                    .rev()
                    .take(600)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>()),
            )),
            _ => {}
        }
        self.tool_msg(kv);
        Outcome {
            key: r.key.clone(),
            status: status.to_string(),
            changed,
        }
    }
}

/// Write `data` to `dest` atomically (a file beside it, renamed over it).
fn install(dest: &Path, data: &[u8]) -> std::io::Result<()> {
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dest.with_file_name(format!(".{name}.flashtex-{}", std::process::id()));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, dest).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Run `cmd`; `Ok(Some(code))` when it exits (-1 when killed by a signal),
/// `Ok(None)` when it ran out of time (it is killed then, and only it).
fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Result<Option<i32>, String> {
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let t0 = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return Ok(Some(st.code().unwrap_or(-1))),
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
        if t0.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// One message of a tool's log.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub error: bool,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<i64>,
}

/// bibtex's `.blg`: `Warning--...` lines (with the `--line N of file F`
/// that may follow), and errors: a message followed by
/// `---line N of file F` (or `---this can't happen`), or `I couldn't
/// open ...`, `I found no ...`.
pub fn bibtex_messages(log: &str) -> Vec<Message> {
    let mut v: Vec<Message> = vec![];
    let lines: Vec<&str> = log.lines().collect();
    let where_ = |l: &str| -> Option<(i64, String)> {
        let p = l.find("line ")?;
        let rest = &l[p + 5..];
        let (n, rest) = rest.split_once(" of file ")?;
        Some((n.trim().parse().ok()?, rest.trim().to_string()))
    };
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if let Some(m) = l.strip_prefix("Warning--") {
            let mut msg = Message {
                error: false,
                message: m.to_string(),
                file: None,
                line: None,
            };
            if let Some(n) = lines.get(i + 1) {
                if n.starts_with("--line ") {
                    if let Some((ln, f)) = where_(n) {
                        msg.line = Some(ln);
                        msg.file = Some(f);
                    }
                    i += 1;
                }
            }
            v.push(msg);
        } else if l.starts_with("I couldn't open")
            || l.starts_with("I found no")
            || l.starts_with("Illegal, another \\bib")
            || l.starts_with("Sorry---you've exceeded")
        {
            let mut msg = l.to_string();
            let mut line = None;
            let mut file = None;
            if let Some(n) = lines.get(i + 1) {
                if n.starts_with("---") {
                    if let Some((ln, f)) = where_(n) {
                        line = Some(ln);
                        file = Some(f);
                    } else {
                        msg.push(' ');
                        msg.push_str(n.trim_start_matches('-'));
                    }
                    i += 1;
                }
            }
            v.push(Message {
                error: true,
                message: msg,
                file,
                line,
            });
        } else if let Some(n) = lines.get(i + 1) {
            // "<message>---line N of file F", split over two lines when
            // long, then the offending line: an error.
            if let Some(p) = l.find("---line ") {
                if let Some((ln, f)) = where_(&l[p..]) {
                    v.push(Message {
                        error: true,
                        message: l[..p].to_string(),
                        file: Some(f),
                        line: Some(ln),
                    });
                }
            } else if n.starts_with("---line ") && !l.starts_with("Warning--") {
                if let Some((ln, f)) = where_(n) {
                    v.push(Message {
                        error: true,
                        message: l.to_string(),
                        file: Some(f),
                        line: Some(ln),
                    });
                    i += 1;
                }
            }
        }
        i += 1;
    }
    v
}

/// biber's `.blg`: `[N] Module:line> WARN - message` and `ERROR - `;
/// a BibTeX-parse message names `file.bib_NNNN.utf8, line L`.
pub fn biber_messages(log: &str) -> Vec<Message> {
    let mut v = vec![];
    for l in log.lines() {
        let (error, m) = if let Some(p) = l.find("> WARN - ") {
            (false, &l[p + 9..])
        } else if let Some(p) = l.find("> ERROR - ") {
            (true, &l[p + 10..])
        } else {
            continue;
        };
        let mut msg = Message {
            error,
            message: m.to_string(),
            file: None,
            line: None,
        };
        // "... from file 'refs.bib' ..." or "refs.bib_1234.utf8, line 5, ..."
        if let Some(p) = m.find(".bib_") {
            let start = m[..p].rfind([' ', '\'', '/', '\\']).map_or(0, |s| s + 1);
            msg.file = Some(format!("{}.bib", &m[start..p]));
            if let Some(q) = m[p..].find(", line ") {
                let rest = &m[p + q + 7..];
                let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                msg.line = n.parse().ok();
            }
        }
        v.push(msg);
    }
    v
}

/// makeindex's `.ilg`: `## Warning (input = F, line = N; ...):` and `!!
/// Input index error (file = F, line = N):`, each followed by `   -- message`.
pub fn makeindex_messages(log: &str) -> Vec<Message> {
    let mut v = vec![];
    let lines: Vec<&str> = log.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let error = if l.starts_with("## Warning") {
            false
        } else if l.starts_with("!! ") {
            true
        } else {
            continue;
        };
        let field = |k: &str| -> Option<String> {
            let p = l.find(k)?;
            let rest = &l[p + k.len()..];
            let end = rest.find([',', ';', ')']).unwrap_or(rest.len());
            Some(rest[..end].trim().to_string())
        };
        let file = field("input = ").or_else(|| field("file = "));
        let line = field("line = ").and_then(|n| n.parse().ok());
        let message = lines
            .get(i + 1)
            .map(|n| n.trim().trim_start_matches("-- ").to_string())
            .unwrap_or_else(|| l.to_string());
        v.push(Message {
            error,
            message,
            file,
            line,
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aux_lines_and_inputs() {
        let d = std::env::temp_dir().join(format!("ext-aux-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("ch")).unwrap();
        std::fs::write(
            d.join("main.aux"),
            "\\relax\n\\citation{a}\n\\@input{ch/one.aux}\n\\newlabel{x}{{1}{1}}\n\\bibstyle{plain}\n\\bibdata{refs,more}\n",
        )
        .unwrap();
        std::fs::write(
            d.join("ch/one.aux"),
            "\\citation{b}\n\\newlabel{y}{{2}{1}}\n",
        )
        .unwrap();
        let (mut rel, mut files, mut bib, mut bst) = (vec![], vec![], vec![], None);
        aux_chain(&d, "main.aux", &mut rel, &mut files, &mut bib, &mut bst, 0);
        assert_eq!(
            String::from_utf8(rel).unwrap(),
            "\\citation{a}\n\\@input{ch/one.aux}\n\\citation{b}\n\\bibstyle{plain}\n\\bibdata{refs,more}\n"
        );
        assert_eq!(bib, vec!["refs", "more"]);
        assert_eq!(bst.as_deref(), Some("plain"));
        let names: Vec<&str> = files.iter().map(|(r, _)| r.as_str()).collect();
        assert_eq!(names, vec!["main.aux", "ch/one.aux"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bcf_datasources() {
        let bcf = br#"<?xml version="1.0" encoding="UTF-8"?>
  <bcf:bibdata section="0">
    <bcf:datasource type="file" datatype="bibtex" glob="false">refs.bib</bcf:datasource>
    <bcf:datasource type="file" datatype="bibtex" glob="false">sub/more.bib</bcf:datasource>
  </bcf:bibdata>
"#;
        assert_eq!(bcf_sources(bcf), vec!["refs.bib", "sub/more.bib"]);
    }

    #[test]
    fn bibtex_log() {
        let blg = "This is BibTeX, Version 0.99d\n\
Database file #1: refs.bib\n\
Warning--I didn't find a database entry for \"nope\"\n\
Warning--empty journal in knuth84\n\
I was expecting a `,' or a `}'---line 7 of file refs.bib\n\
 : \n\
 :        title = {x}\n\
I'm skipping whatever remains of this entry\n\
(There was 1 error message)\n";
        let m = bibtex_messages(blg);
        assert_eq!(m.len(), 3, "{m:?}");
        assert!(!m[0].error && m[0].message.contains("nope"));
        assert!(m[2].error);
        assert_eq!(m[2].file.as_deref(), Some("refs.bib"));
        assert_eq!(m[2].line, Some(7));
    }

    #[test]
    fn makeindex_log() {
        let ilg = "This is makeindex, version 2.17\n\
Scanning input file main.idx...\n\
!! Input index error (file = main.idx, line = 3):\n\
   -- Extra `@' at position 4 of first argument.\n\
## Warning (input = main.idx, line = 5; output = main.ind, line = 9):\n\
   -- Unmatched range closing operator ).\n";
        let m = makeindex_messages(ilg);
        assert_eq!(m.len(), 2);
        assert!(m[0].error);
        assert_eq!(m[0].line, Some(3));
        assert_eq!(m[0].file.as_deref(), Some("main.idx"));
        assert!(m[1].message.starts_with("Unmatched"));
    }

    #[test]
    fn biber_log() {
        let blg = "[0] Config.pm:311> INFO - This is Biber 2.22\n\
[12] Utils.pm:410> WARN - I didn't find a database entry for 'nope' (section 0)\n\
[20] Utils.pm:410> ERROR - BibTeX subsystem: /tmp/x/refs.bib_1234.utf8, line 4, syntax error: found \"}\"\n";
        let m = biber_messages(blg);
        assert_eq!(m.len(), 2);
        assert!(!m[0].error);
        assert!(m[1].error);
        assert_eq!(m[1].file.as_deref(), Some("refs.bib"));
        assert_eq!(m[1].line, Some(4));
    }

    #[test]
    fn relative_to_out() {
        let root = Path::new("/p");
        assert_eq!(
            rel_to(root, Path::new("/p"), "./main.bcf").as_deref(),
            Some("main.bcf")
        );
        assert_eq!(
            rel_to(root, Path::new("/o"), "/o/main.idx").as_deref(),
            Some("main.idx")
        );
        assert_eq!(rel_to(root, Path::new("/o"), "main.idx"), None);
    }
}
