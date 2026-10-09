//! One `COMPILE` of the Unicode host: the job, its runs of the engine (a
//! child process of this program invoked as `xelatex`, as the pdfTeX
//! host's export runs), the `.aux` passes, the external tools, and the
//! diagnostics read from the log.
//!
//! Every run is a full, cold run: the engine's display list streams from
//! the child through a socket of its own (`FLASHTEX_DISPLAY_LIST`) to the
//! client as pages are shipped. A run whose `.aux` (or table of contents,
//! list of figures, ...) changed is followed by another, as latexmk does,
//! each pass's pages replacing the previous pass's (spec §6.4).

use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::page::{flags, Item, Page, StreamKind};
use flashtex_display_list::resource::Font;
use flashtex_display_list::{frame, kind};
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Where replies go.
pub type Out = Arc<Mutex<Box<dyn Write + Send>>>;

pub fn send(out: &Out, k: u8, body: &[u8]) -> bool {
    let mut w = out.lock().unwrap();
    frame::write_frame(&mut *w, k, body).is_ok() && w.flush().is_ok()
}

pub fn send_json(out: &Out, k: u8, j: &Json) -> bool {
    send(out, k, j.to_string().as_bytes())
}

/// The most passes one compile makes (latexmk's `$max_repeat` is 5).
pub const MAX_PASSES: usize = 5;
/// The most tool rounds (spec §6.4).
pub const MAX_ROUNDS: usize = 5;

/// A compile request (spec §6.3).
#[derive(Clone, Debug)]
pub struct Job {
    pub id: i64,
    pub root: PathBuf,
    pub main: String,
    pub format: String,
    pub shell_escape: String,
    pub halt_on_error: bool,
    pub output_dir: PathBuf,
    pub jobname: String,
    pub have_fonts: HashSet<[u8; 32]>,
    pub font_formats: HashSet<String>,
    pub export: bool,
    pub external_tools: Option<String>,
    pub buffers: Vec<(String, String)>,
    pub edits: Vec<(String, u64, u64, String)>,
    /// TeX code run before the main file on the first line (`[fonts]`,
    /// `fonts.rs`); empty: the main file's name alone.
    pub preamble: String,
}

fn parse_key(h: &str) -> Option<[u8; 32]> {
    if h.len() != 64 {
        return None;
    }
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = u8::from_str_radix(h.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(k)
}

/// A path relative to the root, `/`-separated, without `..` (spec §1).
fn safe_rel(p: &str) -> Option<&str> {
    let ok = !p.is_empty() && !p.starts_with('/') && !p.split('/').any(|c| c == "..");
    ok.then_some(p)
}

impl Job {
    /// The job of a `COMPILE`; `scratch` is the connection's default
    /// output directory.
    pub fn parse(j: &Json, scratch: &Path) -> Result<Job, String> {
        let id = j.int_field("id").ok_or("COMPILE without an id")?;
        let root = PathBuf::from(j.str_field("root").ok_or("COMPILE without a root")?);
        if !root.is_absolute() {
            return Err("root must be absolute".into());
        }
        let main = j.str_field("main").ok_or("COMPILE without a main file")?;
        safe_rel(main).ok_or("main must be relative to the root, without `..`")?;
        // The Unicode engine's counterpart of the pdfTeX formats a client
        // may name by default.
        let format = match j.str_field("format").unwrap_or("xelatex") {
            "pdflatex" | "latex" => "xelatex",
            "pdftex" | "etex" | "tex" => "xetex",
            f => f,
        }
        .to_string();
        let jobname = match j.str_field("jobname") {
            Some(n) => n.to_string(),
            None => Path::new(main)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "texput".into()),
        };
        let strs = |k: &str| -> Vec<String> {
            j.get(k)
                .and_then(Json::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut buffers = vec![];
        for b in j.get("buffers").and_then(Json::as_array).unwrap_or(&[]) {
            let (Some(p), Some(t)) = (b.str_field("path"), b.str_field("text")) else {
                continue;
            };
            safe_rel(p).ok_or_else(|| format!("bad buffer path {p}"))?;
            buffers.push((p.to_string(), t.to_string()));
        }
        let mut edits = vec![];
        for e in j.get("edits").and_then(Json::as_array).unwrap_or(&[]) {
            let p = e.str_field("path").ok_or("an edit without a path")?;
            safe_rel(p).ok_or_else(|| format!("bad edit path {p}"))?;
            edits.push((
                p.to_string(),
                e.int_field("offset").unwrap_or(0).max(0) as u64,
                e.int_field("delete").unwrap_or(0).max(0) as u64,
                e.str_field("insert").unwrap_or("").to_string(),
            ));
        }
        Ok(Job {
            id,
            root,
            main: main.to_string(),
            format,
            shell_escape: j.str_field("shell_escape").unwrap_or("default").to_string(),
            halt_on_error: j
                .get("halt_on_error")
                .and_then(Json::as_bool)
                .unwrap_or(false),
            output_dir: j
                .str_field("output_dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| scratch.to_path_buf()),
            jobname,
            have_fonts: strs("have_fonts")
                .iter()
                .filter_map(|h| parse_key(h))
                .collect(),
            font_formats: strs("font_formats").into_iter().collect(),
            export: j.get("export").and_then(Json::as_bool).unwrap_or(false),
            external_tools: j.str_field("external_tools").map(String::from),
            buffers,
            edits,
            preamble: String::new(),
        })
    }

    /// Write the editor's buffers and apply the edits (spec §6.3), as
    /// saving would, before compiling.
    pub fn apply_files(&self) -> Result<(), String> {
        for (p, t) in &self.buffers {
            let f = self.root.join(p);
            if std::fs::read(&f).ok().as_deref() != Some(t.as_bytes()) {
                std::fs::write(&f, t).map_err(|e| format!("{}: {e}", f.display()))?;
            }
        }
        for (p, off, del, ins) in &self.edits {
            let f = self.root.join(p);
            let mut d = std::fs::read(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            let (a, b) = (*off as usize, (*off + *del) as usize);
            if a > d.len() || b > d.len() {
                return Err(format!("{p}: edit outside the file"));
            }
            d.splice(a..b, ins.bytes());
            std::fs::write(&f, d).map_err(|e| format!("{}: {e}", f.display()))?;
        }
        Ok(())
    }

    /// The engine's command line, as xelatex would be run.
    pub fn argv(&self) -> Vec<String> {
        let mut a = vec![
            "xelatex".to_string(),
            format!("-fmt={}", self.format),
            "-interaction=nonstopmode".into(),
            "-file-line-error".into(),
        ];
        if self.halt_on_error {
            a.push("-halt-on-error".into());
        }
        if self.output_dir != self.root {
            a.push(format!("-output-directory={}", self.output_dir.display()));
        }
        a.push(format!("-jobname={}", self.jobname));
        match self.shell_escape.as_str() {
            "on" => a.push("-shell-escape".into()),
            "off" => a.push("-no-shell-escape".into()),
            "restricted" => a.push("-shell-restricted".into()),
            _ => {}
        }
        if self.preamble.is_empty() {
            a.push(self.main.clone());
        } else {
            a.push(format!("{}\\input{{{}}}", self.preamble, self.main));
        }
        a
    }
}

/// What the client said it draws (`HELLO.accept`).
#[derive(Clone, Debug, Default)]
pub struct Accept {
    pub color_spaces: bool,
    pub line_state: bool,
}

/// One run of the engine: what it sent and how it ended.
#[derive(Debug, Default)]
pub struct RunResult {
    pub exit_code: Option<i32>,
    pub pages: u32,
    pub first_page_ms: Option<f64>,
    pub cancelled: bool,
    /// What the engine wrote to its standard error (the output's reports).
    pub stderr: String,
}

/// A running engine the connection can stop (`CANCEL`, a newer `COMPILE`).
#[derive(Default)]
pub struct Running {
    pub child: Option<Child>,
    pub cancelled: bool,
}

/// The files whose change asks for another pass (latexmk's rerun rule,
/// for the auxiliary files LaTeX writes and reads back).
const PASS_FILES: &[&str] = &[
    "aux", "toc", "lof", "lot", "out", "nav", "snm", "vrb", "loa", "thm",
];

fn pass_state(dir: &Path, job: &str) -> Vec<Option<Vec<u8>>> {
    PASS_FILES
        .iter()
        .map(|e| std::fs::read(dir.join(format!("{job}.{e}"))).ok())
        .collect()
}

/// Run the engine once for `job`, streaming its pages to `out` (unless an
/// export). `t0` is when the compile arrived.
#[allow(clippy::too_many_arguments)]
pub fn run_once(
    exe: &Path,
    format_dir: &Path,
    job: &Job,
    accept: &Accept,
    out: &Out,
    running: &Arc<Mutex<Running>>,
    t0: Instant,
    sock_dir: &Path,
) -> Result<RunResult, String> {
    std::fs::create_dir_all(&job.output_dir)
        .map_err(|e| format!("{}: {e}", job.output_dir.display()))?;
    let sock = sock_dir.join(format!("dl-{}-{}.sock", std::process::id(), job.id));
    let _ = std::fs::remove_file(&sock);
    let listener = (!job.export)
        .then(|| UnixListener::bind(&sock))
        .transpose()
        .map_err(|e| format!("{}: {e}", sock.display()))?;
    let argv = job.argv();
    let mut cmd = Command::new(exe);
    use std::os::unix::process::CommandExt;
    cmd.arg0(&argv[0])
        .args(&argv[1..])
        .current_dir(&job.root)
        .env("TEXFORMATS", format!("{}:", format_dir.display()))
        .env_remove("FLASHTEX_DISPLAY_LIST")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if listener.is_some() {
        cmd.env(
            "FLASHTEX_DISPLAY_LIST",
            format!("socket:{}", sock.display()),
        );
        // the client's font formats decide which programs travel (spec §5.1)
        let mut f: Vec<&str> = job.font_formats.iter().map(String::as_str).collect();
        f.sort_unstable();
        cmd.env("FLASHTEX_DISPLAY_LIST_FONT_FORMATS", f.join(","));
    }
    {
        let r = running.lock().unwrap();
        if r.cancelled {
            return Ok(RunResult {
                cancelled: true,
                ..RunResult::default()
            });
        }
    }
    let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
    // the output's own reports (`flashtex_xetex::out`), read as they come so
    // that the pipe never fills
    let err_pipe = child.stderr.take();
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut p) = err_pipe {
            use std::io::Read;
            let _ = p.read_to_string(&mut s);
        }
        s
    });
    running.lock().unwrap().child = Some(child);
    let mut res = RunResult::default();
    if let Some(l) = listener {
        l.set_nonblocking(true).ok();
        // the child connects once its first page is shipped
        let stream = loop {
            match l.accept() {
                Ok((s, _)) => break Some(s),
                Err(_) => {
                    let mut r = running.lock().unwrap();
                    let exited = r
                        .child
                        .as_mut()
                        .is_none_or(|c| c.try_wait().ok().flatten().is_some());
                    if exited {
                        break None;
                    }
                    drop(r);
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        };
        if let Some(s) = stream {
            s.set_nonblocking(false).ok();
            forward(s, job, accept, out, &mut res, t0);
        }
        let _ = std::fs::remove_file(&sock);
    }
    let child = running.lock().unwrap().child.take();
    if let Some(mut c) = child {
        let status = c.wait().map_err(|e| e.to_string())?;
        res.exit_code = status.code();
    }
    res.stderr = err_reader.join().unwrap_or_default();
    res.cancelled = running.lock().unwrap().cancelled;
    Ok(res)
}

/// Forward the child's frames to the client: fonts the client holds or
/// does not take go without their program (spec §5.1); 3.3 items the client
/// did not accept are left out of the page, which is then incomplete
/// (spec §11.7).
fn forward(
    s: std::os::unix::net::UnixStream,
    job: &Job,
    accept: &Accept,
    out: &Out,
    res: &mut RunResult,
    t0: Instant,
) {
    let mut r = BufReader::new(s);
    let mut font_keys: HashMap<u16, [u8; 32]> = HashMap::new();
    let mut image_keys: HashMap<u32, [u8; 32]> = HashMap::new();
    while let Ok(Some((k, body))) = frame::read_frame(&mut r) {
        let body = match k {
            kind::FONT => match Font::decode(&body) {
                Ok(mut f) => {
                    font_keys.insert(f.id, f.key);
                    if job.have_fonts.contains(&f.key) {
                        f.program.clear();
                    }
                    f.encode()
                }
                Err(_) => body,
            },
            kind::IMAGE => {
                if let Ok(j) = Json::parse(&String::from_utf8_lossy(&body)) {
                    if let (Some(id), Some(key)) =
                        (j.int_field("id"), j.str_field("key").and_then(parse_key))
                    {
                        image_keys.insert(id as u32, key);
                    }
                }
                body
            }
            kind::PAGE | kind::FORM => {
                let sk = if k == kind::PAGE {
                    StreamKind::Page
                } else {
                    StreamKind::Form
                };
                adapt(&body, sk, accept, &font_keys, &image_keys).unwrap_or(body)
            }
            _ => body,
        };
        if k == kind::PAGE {
            res.pages += 1;
            if res.first_page_ms.is_none() {
                res.first_page_ms = Some(t0.elapsed().as_secs_f64() * 1e3);
            }
        }
        if !send(out, k, &body) {
            break;
        }
    }
}

/// `body` without the 3.3 items the client did not accept, or `None` when
/// it has none.
fn adapt(
    body: &[u8],
    sk: StreamKind,
    accept: &Accept,
    fonts: &HashMap<u16, [u8; 32]>,
    images: &HashMap<u32, [u8; 32]>,
) -> Option<Vec<u8>> {
    let mut p = Page::decode(sk, body).ok()?;
    let refuse = |i: &Item| match i {
        Item::FillAlpha(_)
        | Item::StrokeAlpha(_)
        | Item::FillColorCs { .. }
        | Item::StrokeColorCs { .. } => !accept.color_spaces,
        Item::LineState(_) => !accept.line_state,
        _ => false,
    };
    if !p.items.iter().any(refuse) {
        return None;
    }
    let n = p.items.len();
    let mut items = Vec::with_capacity(n);
    for it in std::mem::take(&mut p.items) {
        if refuse(&it) {
            let what = match it {
                Item::LineState(_) => "stroked glyphs (accept line-state)",
                _ => "constant alpha or colour spaces (accept color-spaces)",
            };
            let k = p.unsupported.len() as u32;
            p.unsupported.push(what.to_string());
            items.push(Item::Unsupported(k));
        } else {
            items.push(it);
        }
    }
    p.items = items;
    p.flags |= flags::INCOMPLETE;
    let fk = |id: u16| fonts.get(&id).copied().unwrap_or([0; 32]);
    let ik = |id: u32| images.get(&id).copied().unwrap_or([0; 32]);
    p.hash = p.content_hash(&fk, &ik);
    Some(p.encode())
}

/// One diagnostic from the log.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// TeX's errors (`file:line: message` with `-file-line-error`, `! message`)
/// and LaTeX's and packages' warnings, from the log.
pub fn log_diagnostics(log: &str) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    let lines: Vec<&str> = log.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if let Some(m) = l.strip_prefix("! ") {
            out.push(Diagnostic {
                severity: "error",
                message: m.trim().to_string(),
                file: None,
                line: None,
            });
        } else if let Some((file, line, msg)) = file_line_error(l) {
            out.push(Diagnostic {
                severity: "error",
                message: msg,
                file: Some(file),
                line: Some(line),
            });
        } else if let Some(pos) = l.find(" Warning: ") {
            let head = &l[..pos];
            let package = head
                .strip_prefix("Package ")
                .or_else(|| head.strip_prefix("Class "))
                .map(|n| format!("({n})"));
            if head.starts_with("LaTeX") || package.is_some() {
                // A package's warning goes on in lines that start with its
                // name in parentheses; LaTeX's wraps until it ends with `.`.
                let mut msg = l[pos + 10..].trim().to_string();
                let mut j = i + 1;
                while j < lines.len() && j < i + 8 {
                    let c = lines[j];
                    let cont = match &package {
                        Some(n) => c.strip_prefix(n.as_str()).map(str::trim),
                        None if !msg.ends_with('.') && !c.trim().is_empty() => Some(c.trim()),
                        None => None,
                    };
                    match cont {
                        Some(t) => {
                            msg.push(' ');
                            msg.push_str(t);
                            j += 1;
                        }
                        None => break,
                    }
                }
                let line = msg
                    .rsplit_once("on input line ")
                    .and_then(|(_, n)| n.trim_end_matches('.').parse::<u32>().ok());
                out.push(Diagnostic {
                    severity: "warning",
                    message: msg,
                    file: None,
                    line,
                });
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out.dedup();
    out
}

/// `./main.tex:12: Undefined control sequence.`
fn file_line_error(l: &str) -> Option<(String, u32, String)> {
    let mut parts = l.splitn(3, ':');
    let file = parts.next()?;
    let line = parts.next()?;
    let msg = parts.next()?;
    let line: u32 = line.parse().ok()?;
    if file.is_empty() || file.contains(' ') && !file.contains('/') || !file.contains('.') {
        return None;
    }
    let msg = msg.strip_prefix(' ')?;
    Some((
        file.trim_start_matches("./").to_string(),
        line,
        msg.trim().to_string(),
    ))
}

/// A whole compile: its passes, its pages streamed, then `DONE` (the
/// caller sends `STARTED`).
pub struct Outcome {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub pages: u32,
    pub passes: usize,
    pub first_page_ms: Option<f64>,
    pub diagnostics: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn compile(
    exe: &Path,
    format_dir: &Path,
    job: &Job,
    accept: &Accept,
    out: &Out,
    running: &Arc<Mutex<Running>>,
    t0: Instant,
    sock_dir: &Path,
) -> Outcome {
    let mut passes = 0;
    let mut last;
    let mut first_page_ms = None;
    loop {
        let before = pass_state(&job.output_dir, &job.jobname);
        passes += 1;
        last = match run_once(exe, format_dir, job, accept, out, running, t0, sock_dir) {
            Ok(r) => r,
            Err(e) => {
                send_json(
                    out,
                    kind::DIAGNOSTIC,
                    &obj([
                        ("id", Json::Int(job.id)),
                        ("severity", js("error")),
                        ("message", js(e)),
                    ]),
                );
                return Outcome {
                    status: "failed",
                    exit_code: None,
                    pages: 0,
                    passes,
                    first_page_ms,
                    diagnostics: 1,
                };
            }
        };
        first_page_ms = first_page_ms.or(last.first_page_ms);
        for l in last.stderr.lines() {
            let (sev, m) = match l.strip_prefix("! FlashTeX output: ") {
                Some(m) => ("error", m),
                None => match l.strip_prefix("FlashTeX output: ") {
                    Some(m) => ("warning", m),
                    None => continue,
                },
            };
            send_json(
                out,
                kind::DIAGNOSTIC,
                &obj([
                    ("id", Json::Int(job.id)),
                    ("severity", js(sev)),
                    ("message", js(m)),
                ]),
            );
        }
        if last.cancelled {
            break;
        }
        let after = pass_state(&job.output_dir, &job.jobname);
        // a run that stopped at an error (halt-on-error) is not repeated
        let halted = last.exit_code != Some(0) && job.halt_on_error;
        if after == before || passes >= MAX_PASSES || halted {
            break;
        }
    }
    let log = std::fs::read_to_string(job.output_dir.join(format!("{}.log", job.jobname)))
        .unwrap_or_default();
    let diags = log_diagnostics(&log);
    if !last.cancelled {
        for d in &diags {
            let mut j = vec![
                ("id".to_string(), Json::Int(job.id)),
                ("severity".to_string(), js(d.severity)),
                ("message".to_string(), js(d.message.clone())),
            ];
            if let Some(f) = &d.file {
                j.push(("file".into(), js(f.clone())));
            }
            if let Some(l) = d.line {
                j.push(("line".into(), Json::Int(l as i64)));
            }
            send_json(out, kind::DIAGNOSTIC, &Json::Obj(j));
        }
    }
    let status = if last.cancelled {
        "cancelled"
    } else {
        match last.exit_code {
            Some(0) => "ok",
            Some(_) => "error",
            None => "failed",
        }
    };
    Outcome {
        status,
        exit_code: last.exit_code,
        pages: last.pages,
        passes,
        first_page_ms,
        diagnostics: diags.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_diagnostics_are_errors_and_warnings() {
        let log = "\
(./main.tex
./main.tex:12: Undefined control sequence.
l.12 \\foo
! Emergency stop.
LaTeX Warning: Reference `x' on page 1 undefined on input line 7.
Package hyperref Warning: Token not allowed in a PDF string (Unicode):
(hyperref)                removing `math shift' on input line 9.
Overfull \\hbox (1.0pt too wide) in paragraph at lines 3--4
";
        let d = log_diagnostics(log);
        assert_eq!(d[0].file.as_deref(), Some("main.tex"));
        assert_eq!(d[0].line, Some(12));
        assert_eq!(d[0].message, "Undefined control sequence.");
        assert_eq!(d[1].message, "Emergency stop.");
        assert_eq!(d[2].severity, "warning");
        assert_eq!(d[2].line, Some(7));
        assert!(d[3].message.contains("removing `math shift'"), "{:?}", d[3]);
        assert_eq!(d[3].line, Some(9));
        assert_eq!(d.len(), 4);
    }

    #[test]
    fn jobs_parse_and_name_xelatex_for_pdflatex() {
        let j = Json::parse(
            r#"{"id": 3, "root": "/p", "main": "main.tex", "format": "pdflatex",
               "have_fonts": ["00"], "export": true, "buffers": [{"path": "a.tex", "text": "x"}]}"#,
        )
        .unwrap();
        let job = Job::parse(&j, Path::new("/tmp/x")).unwrap();
        assert_eq!(job.format, "xelatex");
        assert_eq!(job.jobname, "main");
        assert!(job.export);
        assert_eq!(job.output_dir, PathBuf::from("/tmp/x"));
        assert_eq!(
            job.argv(),
            [
                "xelatex",
                "-fmt=xelatex",
                "-interaction=nonstopmode",
                "-file-line-error",
                "-output-directory=/tmp/x",
                "-jobname=main",
                "main.tex"
            ]
        );
        let bad = Json::parse(r#"{"id": 1, "root": "/p", "main": "../x.tex"}"#).unwrap();
        assert!(Job::parse(&bad, Path::new("/t")).is_err());
    }
}
