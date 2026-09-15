//! `flashtex-render`: a drop-in runtime-v1 worker (set `FLASHTEX_COMPILER`
//! to this binary). Requests on stdin, one `compile_result` line per request
//! on stdout, logs on stderr.
//!
//! Options:
//!   --tex <main.tex>   convenience mode: render this single file (its
//!                      basename is the project path) instead of reading
//!                      JSON Lines; diagnostics go to stderr as
//!                      `severity[code] message (line:col)`; exit 0 when
//!                      the status is ok/recovered, 1 when failed
//!   --v2 <out.json>    also write the rendering-v2 display list of the
//!                      last request (an explicit `display_list` envelope)
//!   --pdf <out.pdf>    also write a PDF of the last request through the
//!                      pdf sibling (v1 text items, see src/pdf.rs)
//!   --font-dir <dir>   extra font directory (repeatable; probed before
//!                      the TeX Live defaults; `FLASHTEX_FONT_DIRS` too)
//!   --class-options <opts>  class options assumed for body-only input
//!                      (default `12pt`, the compiler's implicit preamble)
//!   --date YYYY-MM-DD  what `\today` renders when a request carries no
//!                      `payload.date` of its own (default 1970-01-01, the
//!                      Unix epoch this binary has always printed)
//!   --secnumdepth <n>  section numbering depth when the source does not
//!                      set the counter (default 2; the oracle preamble is 0)
//!   --timing           print per-request wall time to stderr

use std::io;
use std::path::{Path, PathBuf};

use flashtex_compiler::json;

use flashtex_render_pipeline::TodayDate;
use flashtex_render_pipeline::{protocol, FontSet, RenderOptions, Rendered};

/// Side outputs shared by both modes: `--v2` / `--pdf` of the last render.
struct Outputs {
    v2: Option<PathBuf>,
    pdf: Option<PathBuf>,
    timing: bool,
    /// `--device-color`: `--v2` paints carry `device_color` (proposal).
    device_color: bool,
    /// `--images`: `--v2` serialises image items (FT-063,
    /// `display-list-v2-images`). Off by default, so every existing caller's
    /// `--v2` bytes are unchanged; a caller that wants to see, export or
    /// measure `\includegraphics` output must ask for it, exactly as a
    /// runtime-v1 client asks by negotiating the capability.
    images: bool,
}

impl Outputs {
    fn write(&self, id: &str, r: &Rendered) {
        if self.timing {
            eprintln!("flashtex-render: {id} rendered in {:.2} ms", r.elapsed_ms);
        }
        if let Some(p) = &self.v2 {
            let wire = flashtex_render_pipeline::display::Wire { images: self.images, device_color: self.device_color, diagnostics: false };
            let text = r.v2.write_json_wire(id, wire);
            if let Err(e) = std::fs::write(p, text) {
                eprintln!("flashtex-render: cannot write {}: {e}", p.display());
            }
        }
        if let Some(p) = &self.pdf {
            match flashtex_render_pipeline::pdf::write_pdf(&r.v2) {
                Ok(pdf) => {
                    for w in &pdf.warnings {
                        eprintln!("flashtex-render: pdf: {w}");
                    }
                    if let Err(e) = std::fs::write(p, &pdf.bytes) {
                        eprintln!("flashtex-render: cannot write {}: {e}", p.display());
                    }
                }
                Err(e) => eprintln!("flashtex-render: pdf failed: {e}"),
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut tex_in: Option<PathBuf> = None;
    let mut outputs = Outputs {
        v2: None,
        pdf: None,
        timing: false,
        device_color: false,
        images: false,
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut options = RenderOptions::default();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--tex" => tex_in = args.next().map(PathBuf::from),
            "--v2" => outputs.v2 = args.next().map(PathBuf::from),
            "--pdf" => outputs.pdf = args.next().map(PathBuf::from),
            "--font-dir" => {
                if let Some(d) = args.next() {
                    dirs.push(PathBuf::from(d));
                }
            }
            "--class-options" => {
                if let Some(o) = args.next() {
                    options.default_class_options = o;
                }
            }
            "--secnumdepth" => {
                if let Some(n) = args.next().and_then(|n| n.parse::<u8>().ok()) {
                    options.default_secnumdepth = n;
                }
            }
            "--project-root" => {
                // FT-063: default directory `\includegraphics` files are read
                // from when a request carries no `project_root`.
                options.project_root = args.next().map(PathBuf::from);
            }
            "--date" => {
                // The date `\today` renders when a request carries no `date`
                // of its own. This binary is a caller as well as a worker, so
                // it may resolve the clock -- the pipeline itself may not.
                // protocol/proposals/runtime-v1-request-date.md
                match args.next().as_deref().map(TodayDate::parse_iso) {
                    Some(Ok(date)) => options.today = date,
                    Some(Err(error)) => {
                        eprintln!("flashtex-render: --date {error}");
                        std::process::exit(2);
                    }
                    None => {
                        eprintln!("flashtex-render: --date needs a YYYY-MM-DD value");
                        std::process::exit(2);
                    }
                }
            }
            "--timing" => outputs.timing = true,
            "--device-color" => outputs.device_color = true,
            "--images" => outputs.images = true,
            "-h" | "--help" => {
                eprintln!("usage: flashtex-render [--tex main.tex] [--v2 out.json] [--pdf out.pdf] [--font-dir DIR]... [--class-options OPTS] [--secnumdepth N] [--date YYYY-MM-DD] [--timing] [--device-color] [--images]");
                eprintln!("  --images: --v2 also serialises image items (display-list-v2-images); off by default");
                eprintln!("  --date: what \\today renders (default 1970-01-01); a request's own payload.date wins");
                eprintln!("  without --tex: runtime-v1 JSON Lines worker (compile requests on stdin, one compile_result per line on stdout)");
                return;
            }
            other => {
                eprintln!("flashtex-render: unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }
    let fonts = FontSet::with_default_dirs(&dirs);
    if let Some(path) = tex_in {
        std::process::exit(run_tex_file(&path, &fonts, &options, &outputs));
    }
    // The worker loop (`protocol::serve`, shared with `flashtex worker`):
    // a block cache across requests means a keystroke retypesets only the
    // paragraph it touched; output is identical to a fresh compile.
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut input = stdin.lock();
    if let Err(e) = protocol::serve(&mut input, &mut out, &fonts, &options, |id, r| outputs.write(id, r)) {
        eprintln!("flashtex-render: read error: {e}");
    }
}

/// `--tex FILE`: wraps the file in a single-document compile request (path =
/// its basename), runs it through the same `protocol::handle_line` as the
/// JSON Lines worker, prints readable diagnostics, writes `--v2`/`--pdf`.
/// Returns the process exit code: 0 for ok/recovered, 1 for failed, 2 when
/// the file cannot be read.
fn run_tex_file(path: &Path, fonts: &FontSet, options: &RenderOptions, outputs: &Outputs) -> i32 {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("flashtex-render: cannot read {}: {e}", path.display());
            return 2;
        }
    };
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("main.tex")
        .to_string();
    let stem = Path::new(&name).file_stem().and_then(|s| s.to_str()).unwrap_or("main").to_string();

    let mut doc = json::Value::obj();
    doc.set("path", json::str_(name.clone()));
    doc.set("text", json::str_(text.clone()));
    let mut payload = json::Value::obj();
    payload.set("project_id", json::str_(stem));
    payload.set("revision", json::num(1.0));
    payload.set("entry_path", json::str_(name.clone()));
    payload.set("documents", json::Value::Arr(vec![doc]));
    let mut request = json::Value::obj();
    request.set("protocol_version", json::num(flashtex_compiler::protocol::PROTOCOL_VERSION as f64));
    request.set("id", json::str_(name.clone()));
    request.set("type", json::str_("compile"));
    request.set("payload", payload);

    let reply = protocol::handle_line(&json::write(&request), fonts, options, None);
    let envelope = match json::parse(&reply.line) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("flashtex-render: internal error: reply is not JSON: {}", e.0);
            return 1;
        }
    };
    if envelope.get("type").and_then(|v| v.as_str()) == Some("error") {
        let p = envelope.get("payload");
        let code = p.and_then(|p| p.get("code")).and_then(|v| v.as_str()).unwrap_or("error");
        let message = p.and_then(|p| p.get("message")).and_then(|v| v.as_str()).unwrap_or("");
        eprintln!("error[{code}] {message}");
        return 1;
    }
    let payload = envelope.get("payload");
    let status = payload.and_then(|p| p.get("status")).and_then(|v| v.as_str()).unwrap_or("failed");
    let empty = Vec::new();
    let diagnostics = payload.and_then(|p| p.get("diagnostics")).and_then(|v| v.as_arr()).unwrap_or(&empty);
    for d in diagnostics {
        let severity = d.get("severity").and_then(|v| v.as_str()).unwrap_or("error");
        let code = d.get("code").and_then(|v| v.as_str()).unwrap_or("unknown");
        let message = d.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let at = d
            .get("source")
            .and_then(|s| s.get("start_byte"))
            .and_then(|v| v.as_i64())
            .and_then(|b| usize::try_from(b).ok())
            .map(|b| line_col(&text, b));
        match at {
            Some((line, col)) => eprintln!("{severity}[{code}] {message} ({line}:{col})"),
            None => eprintln!("{severity}[{code}] {message}"),
        }
        if let Some(r) = d.get("recovery").and_then(|v| v.as_str()) {
            eprintln!("    recovery: {r}");
        }
    }
    let pages = payload.and_then(|p| p.get("pages")).and_then(|v| v.as_arr()).map_or(0, Vec::len);
    let mut wrote = Vec::new();
    if let Some(r) = &reply.rendered {
        outputs.write(&reply.id, r);
        if let Some(p) = &outputs.pdf {
            wrote.push(format!("pdf {}", p.display()));
        }
        if let Some(p) = &outputs.v2 {
            wrote.push(format!("v2 {}", p.display()));
        }
    }
    let arrow = if wrote.is_empty() { String::new() } else { format!(" -> {}", wrote.join(", ")) };
    eprintln!("flashtex-render: {name}: {status}, {pages} page{}{arrow}", if pages == 1 { "" } else { "s" });
    if status == "failed" {
        1
    } else {
        0
    }
}

/// 1-based line and column (in characters) of a byte offset; an offset past
/// the end or inside a multi-byte sequence is clamped to the nearest boundary.
fn line_col(text: &str, byte: usize) -> (usize, usize) {
    let mut b = byte.min(text.len());
    while b > 0 && !text.is_char_boundary(b) {
        b -= 1;
    }
    let before = &text[..b];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, col)
}
