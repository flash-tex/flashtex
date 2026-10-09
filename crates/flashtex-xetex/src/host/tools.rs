//! External tools for the Unicode host (spec §6.4, "External tools"):
//! bibtex, biber and makeindex from the user's TeX Live, when latexmk's
//! rules say a compile needs them, then a follow-up compile.
//!
//! The rules (latexmk 4.87's, as the pdfTeX host applies them; the pdfTeX
//! host's own implementation, `flashtex_engine::host::external`, is bound
//! to its resident engine): **biber** when the run wrote `JOB.bcf`;
//! otherwise **bibtex** when `JOB.aux` names `\bibdata`, unless a `.bib`
//! file it names is missing (the document keeps its `.bbl`); **makeindex**
//! for `JOB.idx`. A rule runs when what its program reads changed since
//! its last run on this connection, or its output is missing.

use super::compile::{send_json, Job, Out};
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::kind;
use flashtex_display_list::sha256::sha256;
use flashtex_engine::host::external::{self, Programs};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What the connection remembers of each rule: the state of its sources
/// at its last run, or when it was last reported skipped.
#[derive(Default)]
pub struct Memory {
    last: HashMap<String, [u8; 32]>,
    said: HashMap<String, [u8; 32]>,
}

struct Rule {
    tool: &'static str,
    /// The source, relative to the output directory.
    file: String,
    state: [u8; 32],
    /// The output it makes.
    output: String,
    /// `.bib` files it needs that do not exist.
    missing: Vec<String>,
    argv: Vec<String>,
}

/// What bibtex reads of the `.aux`: `\citation`, `\bibdata`, `\bibstyle`
/// and `\@input` lines.
fn aux_lines(aux: &str) -> (String, Vec<String>) {
    let mut rel = String::new();
    let mut bibs = vec![];
    for l in aux.lines() {
        if l.starts_with("\\citation")
            || l.starts_with("\\bibstyle")
            || l.starts_with("\\@input")
            || l.starts_with("\\bibdata")
        {
            rel.push_str(l);
            rel.push('\n');
            if let Some(d) = l
                .strip_prefix("\\bibdata{")
                .and_then(|r| r.strip_suffix('}'))
            {
                bibs.extend(
                    d.split(',')
                        .map(|b| b.trim().to_string())
                        .filter(|b| !b.is_empty()),
                );
            }
        }
    }
    (rel, bibs)
}

fn rules(job: &Job) -> Vec<Rule> {
    let out = &job.output_dir;
    let jn = &job.jobname;
    let mut v = vec![];
    let bcf = out.join(format!("{jn}.bcf"));
    if let Ok(d) = std::fs::read(&bcf) {
        v.push(Rule {
            tool: "biber",
            file: format!("{jn}.bcf"),
            state: sha256(&d),
            output: format!("{jn}.bbl"),
            missing: vec![],
            argv: vec![
                "--input-directory".into(),
                job.root.to_string_lossy().into_owned(),
                "--output-directory".into(),
                out.to_string_lossy().into_owned(),
                jn.clone(),
            ],
        });
    } else if let Ok(aux) = std::fs::read_to_string(out.join(format!("{jn}.aux"))) {
        let (rel, bibs) = aux_lines(&aux);
        if !bibs.is_empty() {
            let mut state = rel.into_bytes();
            let mut missing = vec![];
            for b in &bibs {
                let name = if b.ends_with(".bib") {
                    b.clone()
                } else {
                    format!("{b}.bib")
                };
                match std::fs::read(job.root.join(&name)) {
                    Ok(d) => state.extend_from_slice(&sha256(&d)),
                    Err(_) => missing.push(name),
                }
            }
            v.push(Rule {
                tool: "bibtex",
                file: format!("{jn}.aux"),
                state: sha256(&state),
                output: format!("{jn}.bbl"),
                missing,
                argv: vec![jn.clone()],
            });
        }
    }
    if let Ok(d) = std::fs::read(out.join(format!("{jn}.idx"))) {
        v.push(Rule {
            tool: "makeindex",
            file: format!("{jn}.idx"),
            state: sha256(&d),
            output: format!("{jn}.ind"),
            missing: vec![],
            argv: vec!["-o".into(), format!("{jn}.ind"), format!("{jn}.idx")],
        });
    }
    v
}

fn program<'a>(p: &'a Programs, tool: &str) -> Option<&'a PathBuf> {
    match tool {
        "biber" => p.biber.as_ref(),
        "bibtex" => p.bibtex.as_ref(),
        _ => p.makeindex.as_ref(),
    }
}

/// Run a program with a timeout; its exit code (`None`: timed out or did
/// not start) and how long it took.
fn run(
    prog: &Path,
    argv: &[String],
    dir: &Path,
    env: &[(&str, String)],
    timeout: Duration,
) -> (Option<i32>, f64) {
    let t = Instant::now();
    let mut c = Command::new(prog);
    c.args(argv)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in env {
        c.env(k, v);
    }
    let Ok(mut child) = c.spawn() else {
        return (None, 0.0);
    };
    loop {
        if let Ok(Some(s)) = child.try_wait() {
            return (s.code(), t.elapsed().as_secs_f64() * 1e3);
        }
        if t.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return (None, t.elapsed().as_secs_f64() * 1e3);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// After a compile: run what is due (policy `auto`) or say what would have
/// run (`off`). True when an output changed (a follow-up compile is due).
pub fn after_compile(
    job: &Job,
    auto: bool,
    programs: &Programs,
    memory: &mut Memory,
    out: &Out,
    timeout: Duration,
) -> (bool, bool) {
    let (mut ran, mut changed) = (false, false);
    for r in rules(job) {
        let key = format!("{}:{}", r.tool, r.file);
        let out_file = job.output_dir.join(&r.output);
        let have = std::fs::read(&out_file).ok().map(|d| sha256(&d));
        let reason = match memory.last.get(&key) {
            None if have.is_none() => Some("its output is missing"),
            None => Some("first run"),
            Some(s) if *s != r.state => Some("its sources changed"),
            Some(_) if have.is_none() => Some("its output is missing"),
            Some(_) => None,
        };
        let Some(reason) = reason else { continue };
        let skip = |why: &str, memory: &mut Memory| {
            if memory.said.get(&key) == Some(&r.state) {
                return;
            }
            memory.said.insert(key.clone(), r.state);
            send_json(
                out,
                kind::TOOL,
                &obj([
                    ("id", Json::Int(job.id)),
                    ("event", js("skip")),
                    ("tool", js(r.tool)),
                    ("file", js(r.file.clone())),
                    ("reason", js(why)),
                ]),
            );
        };
        if !auto {
            skip("external tools are off for this project", memory);
            continue;
        }
        if !r.missing.is_empty() {
            skip(&format!("{} not found", r.missing.join(", ")), memory);
            continue;
        }
        let Some(prog) = program(programs, r.tool) else {
            skip(&format!("{} is not installed", r.tool), memory);
            continue;
        };
        send_json(
            out,
            kind::TOOL,
            &obj([
                ("id", Json::Int(job.id)),
                ("event", js("run")),
                ("tool", js(r.tool)),
                ("file", js(r.file.clone())),
                ("reason", js(reason)),
            ]),
        );
        let root = job.root.to_string_lossy().into_owned();
        let dir = job.output_dir.to_string_lossy().into_owned();
        let env = [
            ("BIBINPUTS", format!("{root}:{dir}:")),
            ("BSTINPUTS", format!("{root}:{dir}:")),
            ("INDEXSTYLE", format!("{root}:{dir}:")),
        ];
        let (code, ms) = run(prog, &r.argv, &job.output_dir, &env, timeout);
        ran = true;
        memory.last.insert(key.clone(), r.state);
        let now = std::fs::read(&out_file).ok().map(|d| sha256(&d));
        let ch = now.is_some() && now != have;
        changed |= ch && code.is_some();
        let log_ext = if r.tool == "makeindex" { "ilg" } else { "blg" };
        let log_path = job.output_dir.join(format!(
            "{}.{log_ext}",
            r.file.rsplit_once('.').map_or(r.file.as_str(), |x| x.0)
        ));
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        let msgs = match r.tool {
            "biber" => external::biber_messages(&log),
            "bibtex" => external::bibtex_messages(&log),
            _ => external::makeindex_messages(&log),
        };
        let errors = msgs.iter().filter(|m| m.error).count();
        let warnings = msgs.len() - errors;
        let status = match code {
            None => "timeout",
            Some(_) if errors > 0 => "errors",
            Some(0) if warnings > 0 => "warnings",
            Some(0) => "ok",
            Some(_) => "error",
        };
        for m in &msgs {
            let mut j = vec![
                ("id".to_string(), Json::Int(job.id)),
                (
                    "severity".to_string(),
                    js(if m.error { "error" } else { "warning" }),
                ),
                ("message".to_string(), js(m.message.clone())),
                ("source".to_string(), js(r.tool)),
            ];
            if let Some(f) = &m.file {
                j.push(("file".into(), js(f.clone())));
            }
            if let Some(l) = m.line {
                j.push(("line".into(), Json::Int(l)));
            }
            send_json(out, kind::DIAGNOSTIC, &Json::Obj(j));
        }
        send_json(
            out,
            kind::TOOL,
            &obj([
                ("id", Json::Int(job.id)),
                ("event", js("done")),
                ("tool", js(r.tool)),
                ("file", js(r.file.clone())),
                ("status", js(status)),
                (
                    "exit_code",
                    code.map_or(Json::Null, |c| Json::Int(c as i64)),
                ),
                ("ms", Json::Num(ms)),
                ("changed", Json::Bool(ch)),
                ("warnings", Json::Int(warnings as i64)),
                ("errors", Json::Int(errors as i64)),
                ("log", js(log_path.to_string_lossy().into_owned())),
            ]),
        );
    }
    (ran, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bibtex_reads_its_lines_of_the_aux() {
        let (rel, bibs) = aux_lines(
            "\\relax\n\\citation{a}\n\\newlabel{x}{{1}{1}}\n\\bibstyle{plain}\n\\bibdata{refs,more.bib}\n",
        );
        assert_eq!(
            rel,
            "\\citation{a}\n\\bibstyle{plain}\n\\bibdata{refs,more.bib}\n"
        );
        assert_eq!(bibs, ["refs", "more.bib"]);
    }
}
