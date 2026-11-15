//! External tools for the Unicode host (spec §6.4, "External tools"):
//! bibtex, biber and makeindex, when latexmk's rules say a compile needs
//! them, then a follow-up compile. bibtex and makeindex are the engine's
//! own ports, run in a child of this program (`proc::builtin`), so they
//! need no TeX Live, as in the pdfTeX host; biber is the user's TeX Live
//! program.
//!
//! The rules (latexmk 4.87's, as the pdfTeX host applies them; the pdfTeX
//! host's own implementation, `flashtex_engine::host::external`, is bound
//! to its resident engine): **biber** when the run wrote `JOB.bcf`;
//! otherwise **bibtex** when `JOB.aux` names `\bibdata`, unless a `.bib`
//! file it names is missing (the document keeps its `.bbl`); **makeindex**
//! for `JOB.idx`. A rule runs when what its program reads changed since
//! its last run on this connection, or its output is missing.

use super::compile::Running;
use super::compile::{send_json, Job, Out};
use super::proc::{kill_group, Children};
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::kind;
use flashtex_display_list::sha256::sha256;
use flashtex_engine::host::external::{self, Programs};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
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

/// Where bibtex finds database `name` (with its `.bib`), as the pdfTeX
/// host's rule looks (`host::external`, latexmk's `$bibtex_fudge`): the
/// project, the output directory (biblatex's `JOB-blx.bib` with
/// `backend=bibtex`), then the TeX tree through the engine's resolver (the
/// bundle's without a TeX Live: `xampl.bib`, `biblatex-examples.bib`).
///
/// Under read confinement (`FLASHTEX_CONFINE_READS`) the pdfTeX host's
/// rules apply (`system::host_tool_read_ok`): an absolute, `~`, `$` or `..`
/// name is refused, and what is found must lie in the project, the output
/// directory or a confinement root, or be a TeX tree file. A refused name
/// is missing: it is neither read nor hashed, and bibtex is not run.
///
/// The rules are checked on the canonical path of the file actually opened
/// and read through that handle (`external::read_checked`), so a link
/// swapped between the check and the read is refused (REVIEW-SEC2 on #2403).
/// A search hit counts as a TeX tree file only if its path has no link.
fn read_bib(job: &Job, name: &str) -> Option<Vec<u8>> {
    if !flashtex_engine::system::input_name_confined_ok(name) {
        return None;
    }
    let read = |p: &Path, searched: bool| {
        external::read_checked(p, |real| {
            flashtex_engine::system::host_tool_read_ok(
                name,
                real,
                searched && real == p,
                &[&job.root, &job.output_dir],
            )
        })
    };
    let p = Path::new(name);
    if p.is_absolute() {
        return read(p, false);
    }
    for d in [&job.root, &job.output_dir] {
        let c = d.join(p);
        if c.is_file() {
            return read(&c, false);
        }
    }
    flashtex_engine::system::with_resolver_for("xelatex", |r| {
        r.find_ex(name, flashtex_engine::resolver::Format::Bib, true)
            .0
    })
    .filter(|p| p.is_absolute())
    .and_then(|p| read(&p, true))
}

/// A file in the output directory (a tool's output or log), read through
/// the capped, handle-checked reader; confined, it must resolve into the
/// project or the output directory.
fn read_output(job: &Job, name: &str) -> Option<Vec<u8>> {
    external::read_checked(&job.output_dir.join(name), |real| {
        flashtex_engine::system::host_tool_read_ok(name, real, false, &[&job.root, &job.output_dir])
    })
}

/// The data sources a `.bcf` names that biber would read. Confined, a
/// source that is not a plain file (`location=remote`, a URL) or not found
/// within the rules is missing, so biber does not run; unconfined nothing
/// is missing (biber resolves them itself), and found ones are hashed.
fn bcf_state(job: &Job, d: &[u8]) -> ([u8; 32], Vec<String>) {
    let confined = !flashtex_engine::system::input_name_confined_ok("/");
    let mut state = sha256(d).to_vec();
    let mut missing = vec![];
    for l in String::from_utf8_lossy(d).lines() {
        let l = l.trim();
        if !l.starts_with("<bcf:datasource") {
            continue;
        }
        let name = match (l.find('>'), l.rfind("</bcf:datasource>")) {
            (Some(a), Some(b)) if a < b => l[a + 1..b].to_string(),
            _ => l.to_string(),
        };
        let file = l.contains("type=\"file\"");
        match file.then(|| read_bib(job, &name)).flatten() {
            Some(b) => state.extend_from_slice(&sha256(&b)),
            None if confined && !missing.contains(&name) => missing.push(name),
            None => {}
        }
    }
    (sha256(&state), missing)
}

fn rules(job: &Job) -> Vec<Rule> {
    let out = &job.output_dir;
    let jn = &job.jobname;
    let mut v = vec![];
    let bcf = out.join(format!("{jn}.bcf"));
    // Every source goes through the pdfTeX host's reader (`external::read`):
    // a regular file of at most the tools' 64 MiB limit, else missing.
    if let Some(d) = external::read(&bcf) {
        let (state, missing) = bcf_state(job, &d);
        v.push(Rule {
            tool: "biber",
            file: format!("{jn}.bcf"),
            state,
            output: format!("{jn}.bbl"),
            missing,
            argv: vec![
                "--input-directory".into(),
                job.root.to_string_lossy().into_owned(),
                "--output-directory".into(),
                out.to_string_lossy().into_owned(),
                jn.clone(),
            ],
        });
    } else if let Some(aux) =
        external::read(&out.join(format!("{jn}.aux"))).and_then(|d| String::from_utf8(d).ok())
    {
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
                match read_bib(job, &name) {
                    Some(d) => state.extend_from_slice(&sha256(&d)),
                    None => missing.push(name),
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
    if let Some(d) = external::read(&out.join(format!("{jn}.idx"))) {
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

/// Run a program with a timeout, in a process group a cancel kills
/// (`proc.rs`); its exit code (`None`: timed out, cancelled or did not
/// start) and how long it took.
fn run(
    prog: &Path,
    argv: &[String],
    dir: &Path,
    env: &[(&str, String)],
    timeout: Duration,
    children: &Children,
    running: &Arc<Mutex<Running>>,
) -> (Option<i32>, f64) {
    let t = Instant::now();
    let mut c = children.tool(prog);
    c.args(argv)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in env {
        c.env(k, v);
    }
    let mut child = {
        let mut r = running.lock().unwrap();
        if r.cancelled {
            return (None, 0.0);
        }
        let Ok(child) = c.spawn() else {
            return (None, 0.0);
        };
        r.group = Some(child.id());
        child
    };
    let code = loop {
        if let Ok(Some(s)) = child.try_wait() {
            break s.code();
        }
        if t.elapsed() > timeout {
            kill_group(child.id());
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // reaped (a cancel killed the group already, and the wait ended)
    let mut r = running.lock().unwrap();
    r.group = None;
    let code = if r.cancelled { None } else { code };
    (code, t.elapsed().as_secs_f64() * 1e3)
}

/// After a compile: run what is due (policy `auto`) or say what would have
/// run (`off`). True when an output changed (a follow-up compile is due).
#[allow(clippy::too_many_arguments)]
pub fn after_compile(
    job: &Job,
    auto: bool,
    programs: &Programs,
    memory: &mut Memory,
    out: &Out,
    timeout: Duration,
    children: &Children,
    running: &Arc<Mutex<Running>>,
) -> (bool, bool) {
    let (mut ran, mut changed) = (false, false);
    for r in rules(job) {
        if running.lock().unwrap().cancelled {
            break;
        }
        let key = format!("{}:{}", r.tool, r.file);
        // Through the capped, handle-checked reader: a `.bbl` that is a
        // FIFO, a device or a link out of the project is missing, never
        // opened blocking or read without limit (REVIEW-SEC2 on #2403).
        let have = read_output(job, &r.output).map(|d| sha256(&d));
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
        let builtin = super::proc::builtin(r.tool)
            .then(|| PathBuf::from(format!("{}{}", super::proc::BUILTIN, r.tool)));
        // With no TeX Live biber: the pinned one (`bundle::biber`), fetched
        // now if the user agreed, else why not.
        let mut missing_why = format!("{} is not installed", r.tool);
        let fetched =
            if builtin.is_none() && r.tool == "biber" && program(programs, r.tool).is_none() {
                match flashtex_engine::bundle::biber::ensure() {
                    Ok(p) => Some(p),
                    Err(e) => {
                        missing_why = e;
                        None
                    }
                }
            } else {
                None
            };
        let Some(prog) = builtin
            .as_ref()
            .or_else(|| program(programs, r.tool))
            .or(fetched.as_ref())
        else {
            skip(&missing_why, memory);
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
        let (code, ms) = run(
            prog,
            &r.argv,
            &job.output_dir,
            &env,
            timeout,
            children,
            running,
        );
        if running.lock().unwrap().cancelled {
            // stopped part way: its output is not the run's, nor remembered
            break;
        }
        ran = true;
        memory.last.insert(key.clone(), r.state);
        let now = read_output(job, &r.output).map(|d| sha256(&d));
        let ch = now.is_some() && now != have;
        changed |= ch && code.is_some();
        let log_ext = if r.tool == "makeindex" { "ilg" } else { "blg" };
        let log_name = format!(
            "{}.{log_ext}",
            r.file.rsplit_once('.').map_or(r.file.as_str(), |x| x.0)
        );
        let log_path = job.output_dir.join(&log_name);
        let log = read_output(job, &log_name)
            .map(|d| String::from_utf8_lossy(&d).into_owned())
            .unwrap_or_default();
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

    /// A scratch project (`p/`, output `p/out/`) whose `.aux` names a
    /// project `.bib`, sentinel `.bib` files outside it (by absolute path,
    /// through `..`, through a link in the project) and a project `.bib`
    /// over the 64 MiB limit (sparse). Returns the job and the scratch
    /// directory.
    fn sentinel_job(tag: &str) -> (Job, PathBuf) {
        let base =
            std::env::temp_dir().join(format!("ft-xetex-tools-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (p, out, outside) = (base.join("p"), base.join("p/out"), base.join("outside"));
        std::fs::create_dir_all(&out).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let secret = outside.join("secret.bib");
        std::fs::write(&secret, "SECRET-OUTSIDE-READ").unwrap();
        std::fs::write(p.join("refs.bib"), "@misc{a}").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, p.join("link.bib")).unwrap();
        std::fs::File::create(p.join("huge.bib"))
            .unwrap()
            .set_len((64 << 20) + 1)
            .unwrap();
        std::fs::write(
            out.join("main.aux"),
            format!(
                "\\bibdata{{refs,{},../outside/secret,link,huge}}\n",
                outside.join("secret").display()
            ),
        )
        .unwrap();
        let j = Json::parse(&format!(
            r#"{{"id": 1, "root": "{}", "main": "main.tex", "output_dir": "{}"}}"#,
            p.display(),
            out.display()
        ))
        .unwrap();
        (Job::parse(&j, &out).unwrap(), base)
    }

    fn missing_bibs(job: &Job) -> Vec<String> {
        let r = rules(job);
        assert_eq!(r.len(), 1, "one bibtex rule");
        assert_eq!(r[0].tool, "bibtex");
        r[0].missing.clone()
    }

    /// REVIEW-SEC (#2324, #2026): the rules read and hashed `.bib` files
    /// outside the project after every compile, tools on or off, with no
    /// size limit. Confined, only the project's own `.bib` is read; the
    /// sentinels outside count as missing. Runs in a child process, because
    /// confinement is read from the environment once per process.
    #[test]
    fn confined_rules_read_no_bib_outside_the_project() {
        const CHILD: &str = "FLASHTEX_TEST_TOOLS_CONFINED_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let (job, base) = sentinel_job("confined");
            let missing = missing_bibs(&job);
            let _ = std::fs::remove_dir_all(&base);
            assert_eq!(
                missing,
                [
                    format!("{}.bib", base.join("outside/secret").display()),
                    "../outside/secret.bib".into(),
                    "link.bib".into(),
                    "huge.bib".into(),
                ]
            );
            println!("CONFINED-CHILD-CHECKED");
            return;
        }
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "host::tools::tests::confined_rules_read_no_bib_outside_the_project",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("FLASHTEX_CONFINE_READS", "1")
            .env_remove("FLASHTEX_CONFINE_ROOTS")
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&o.stdout);
        assert!(
            o.status.success() && text.contains("CONFINED-CHILD-CHECKED"),
            "confined child failed: {text}{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }

    /// The control: unconfined (pdfTeX's own rules), every sentinel is
    /// found, so the confined test above can see a read. The over-limit
    /// file is missing either way.
    #[test]
    fn unconfined_rules_find_every_bib_but_the_huge_one() {
        if std::env::var_os("FLASHTEX_CONFINE_READS").is_some() {
            return;
        }
        let (job, base) = sentinel_job("open");
        let missing = missing_bibs(&job);
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(missing, ["huge.bib"]);
    }

    /// REVIEW-SEC2 on #2403: `after_compile` read the `.bbl` with
    /// `std::fs::read` before the `auto` check, so a `main.bbl` FIFO blocked
    /// the host on `open` with tools off, and a link to `/dev/zero` read
    /// without limit. Now it is missing, and the call returns at once.
    #[cfg(unix)]
    #[test]
    fn a_fifo_or_device_bbl_does_not_block_or_flood_after_compile() {
        for (tag, device) in [("fifo", false), ("zero", true)] {
            let (job, base) = sentinel_job(&format!("bbl-{tag}"));
            let bbl = job.output_dir.join("main.bbl");
            if device {
                std::os::unix::fs::symlink("/dev/zero", &bbl).unwrap();
            } else {
                let c = std::ffi::CString::new(bbl.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
            }
            assert_eq!(read_output(&job, "main.bbl"), None);
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let out: Out = Arc::new(Mutex::new(Box::new(std::io::sink())));
                let programs = Programs {
                    bin: None,
                    bibtex: None,
                    biber: None,
                    makeindex: None,
                    kpsewhich: None,
                };
                let children = Children::new(Path::new("/nonexistent/flashtex-sentinel")).unwrap();
                let running = Arc::new(Mutex::new(Running::default()));
                let r = after_compile(
                    &job,
                    false,
                    &programs,
                    &mut Memory::default(),
                    &out,
                    Duration::from_secs(1),
                    &children,
                    &running,
                );
                let _ = tx.send(r);
            });
            let r = rx.recv_timeout(Duration::from_secs(5));
            let _ = std::fs::remove_dir_all(&base);
            assert_eq!(r, Ok((false, false)), "after_compile with a {tag} main.bbl");
        }
    }

    /// REVIEW-SEC2 on #2403: biber reads the `.bcf`'s data sources itself,
    /// unconfined. Confined, a source outside the project, through a link,
    /// or remote is missing, so biber does not run; the project's own is not.
    #[test]
    fn confined_biber_rule_checks_the_bcf_data_sources() {
        const CHILD: &str = "FLASHTEX_TEST_TOOLS_BCF_CHILD";
        let bcf = |job: &Job, base: &Path| {
            let src = |s: &str| {
                format!("<bcf:datasource type=\"file\" datatype=\"bibtex\" glob=\"false\">{s}</bcf:datasource>\n")
            };
            let d = format!(
                "<bcf:bibdata section=\"0\">\n{}{}{}{}<bcf:datasource type=\"url\" datatype=\"bibtex\">https://example.invalid/x.bib</bcf:datasource>\n</bcf:bibdata>\n",
                src("refs.bib"),
                src(&base.join("outside/secret.bib").display().to_string()),
                src("../outside/secret.bib"),
                src("link.bib"),
            );
            std::fs::write(job.output_dir.join("main.bcf"), d).unwrap();
            let r = rules(job);
            assert_eq!(r[0].tool, "biber");
            r[0].missing.clone()
        };
        if std::env::var_os(CHILD).is_some() {
            let (job, base) = sentinel_job("bcf-confined");
            let missing = bcf(&job, &base);
            let _ = std::fs::remove_dir_all(&base);
            assert_eq!(
                missing,
                [
                    base.join("outside/secret.bib").display().to_string(),
                    "../outside/secret.bib".into(),
                    "link.bib".into(),
                    "https://example.invalid/x.bib".into(),
                ]
            );
            println!("BCF-CHILD-CHECKED");
            return;
        }
        if std::env::var_os("FLASHTEX_CONFINE_READS").is_none() {
            // The control: unconfined, biber resolves its sources itself.
            let (job, base) = sentinel_job("bcf-open");
            let missing = bcf(&job, &base);
            let _ = std::fs::remove_dir_all(&base);
            assert!(missing.is_empty(), "{missing:?}");
        }
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "host::tools::tests::confined_biber_rule_checks_the_bcf_data_sources",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("FLASHTEX_CONFINE_READS", "1")
            .env_remove("FLASHTEX_CONFINE_ROOTS")
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&o.stdout);
        assert!(
            o.status.success() && text.contains("BCF-CHILD-CHECKED"),
            "confined child failed: {text}{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}
