//! Replays the main vertical lists captured from pdflatex (oracle/gen.py)
//! through the page builder and the LaTeX output model, and compares
//! against the real pdflatex run: every `\tracingpages` line, every
//! `\outputpenalty`, the node signature of every `\box255`, and the
//! baseline of every line of every shipped column (tolerance 0.1pt).
//! TeX is never run here.

use std::fs;
use std::path::{Path, PathBuf};

use flashtex_page_builder::format::{node_line, parse_glue, parse_nodes};
use flashtex_page_builder::latex::{Column, FootnoteSpec, LatexOutput};
use flashtex_page_builder::node::{GlueSpec, Node};
use flashtex_page_builder::page::{FiredPage, InsertClass, OutputResult, OutputRoutine, PageBuilder, PageParams};
use flashtex_page_builder::scaled::{pt, Scaled, MAX_DIMEN};

const TOLERANCE: Scaled = 6554; // 0.1pt

#[derive(Debug, Default)]
struct Expected {
    params: std::collections::HashMap<String, i64>,
    glues: std::collections::HashMap<String, GlueSpec>,
    stream: Vec<Node>,
    events: Vec<ExpectedEvent>,
    tail_trace: Vec<String>,
}

#[derive(Debug, Default)]
struct ExpectedEvent {
    penalty: i32,
    trace: Vec<String>,
    boxsig: Vec<String>,
    column: Option<Vec<(Scaled, Scaled, Scaled)>>,
}

fn parse(path: &Path) -> Result<Expected, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = text.lines().collect();
    let mut e = Expected::default();
    let mut pos = 0;
    let mut next_id = 0u32;
    while pos < lines.len() {
        let line = lines[pos];
        pos += 1;
        let (head, rest) = line.split_once(' ').unwrap_or((line, ""));
        match head {
            "ftpb" | "name" | "#" | "end" | "text" => {}
            "param" => {
                let (k, v) = rest.split_once(' ').ok_or("param")?;
                e.params.insert(k.to_string(), v.parse().map_err(|_| "param value")?);
            }
            "glue" => {
                let f: Vec<&str> = rest.split_whitespace().collect();
                e.glues.insert(f[0].to_string(), parse_glue(&f[1..])?);
            }
            "stream" => {
                let n: usize = rest.parse().map_err(|_| "stream count")?;
                e.stream = parse_nodes(&lines, &mut pos, n, &mut next_id)?;
            }
            "event" => e.events.push(ExpectedEvent { penalty: rest.parse().map_err(|_| "event")?, ..Default::default() }),
            "trace" => e.events.last_mut().ok_or("trace before event")?.trace.push(rest.to_string()),
            "tailtrace" => e.tail_trace.push(rest.to_string()),
            "box" => {
                let n: usize = rest.parse().map_err(|_| "box count")?;
                let ev = e.events.last_mut().ok_or("box before event")?;
                ev.boxsig = lines[pos..pos + n].iter().map(|s| s.to_string()).collect();
                pos += n;
            }
            "column" => {
                let n: usize = rest.parse().map_err(|_| "column count")?;
                let mut col = Vec::with_capacity(n);
                for l in &lines[pos..pos + n] {
                    let f: Vec<Scaled> = l.split_whitespace().skip(1).map(|x| x.parse().unwrap()).collect();
                    col.push((f[0], f[1], f[2]));
                }
                pos += n;
                e.events.last_mut().ok_or("column before event")?.column = Some(col);
            }
            other => return Err(format!("unknown line kind {other:?}")),
        }
    }
    Ok(e)
}

struct Recorder {
    inner: LatexOutput,
    events: Vec<(i32, Vec<String>, usize, Option<Column>)>,
}

impl OutputRoutine for Recorder {
    fn output(&mut self, builder: &mut PageBuilder, page: FiredPage) -> OutputResult {
        let sig = page.box255.list.iter().map(node_line).collect();
        let penalty = page.output_penalty;
        let trace_len = builder.trace.as_ref().map_or(0, Vec::len);
        let shipped_before = self.inner.columns.len();
        let r = self.inner.output(builder, page);
        let col = if self.inner.columns.len() > shipped_before { self.inner.columns.last().cloned() } else { None };
        self.events.push((penalty, sig, trace_len, col));
        r
    }
}

struct Outcome {
    pages: usize,
    lines: usize,
    max_dev: Scaled,
}

fn replay(e: &Expected) -> Result<Outcome, String> {
    let p = |k: &str| *e.params.get(k).unwrap_or_else(|| panic!("param {k}")) as Scaled;
    let params = PageParams {
        vsize: p("vsize"),
        max_depth: p("maxdepth"),
        top_skip: e.glues["topskip"],
        ..PageParams::default()
    };
    let mut pb = PageBuilder::new(params).with_trace();
    let kludge = p("kludgeins") as u8;
    let footins = p("footins") as u8;
    pb.classes.insert(kludge, InsertClass { count: 1000, dimen: MAX_DIMEN, skip: GlueSpec::ZERO, contents: None });
    pb.classes.insert(
        footins,
        InsertClass { count: p("footcount"), dimen: p("footdimen"), skip: e.glues["footskip"], contents: None },
    );
    let mut out = LatexOutput::new(p("colht"), p("maxdepth"), p("ragged") != 0, kludge);
    out.split_top_skip = e.glues["splittopskip"];
    out.split_max_depth = p("splitmaxdepth");
    out.footnotes = Some(FootnoteSpec {
        class: footins,
        rule: vec![
            Node::kern(pt(-3.0)),
            Node::Rule { width: Some(p("hsize") * 2 / 5), height: pt(0.4), depth: 0 },
            Node::kern(pt(2.6)),
        ],
    });
    let mut rec = Recorder { inner: out, events: Vec::new() };
    pb.contribute(e.stream.iter().cloned());
    pb.run(&mut rec);
    let trace = pb.trace.clone().unwrap_or_default();

    let mut prev_trace = 0;
    let mut max_dev = 0;
    let mut lines = 0;
    let mut pages = 0;
    for (i, exp) in e.events.iter().enumerate() {
        let Some((penalty, sig, trace_len, col)) = rec.events.get(i) else {
            return Err(format!("output {i}: pdflatex fired (penalty {}) but the builder did not", exp.penalty));
        };
        let got_trace = &trace[prev_trace..*trace_len];
        if got_trace != exp.trace.as_slice() {
            let k = got_trace.iter().zip(&exp.trace).position(|(a, b)| a != b).unwrap_or(got_trace.len().min(exp.trace.len()));
            return Err(format!(
                "output {i}: trace differs at line {k}: got {:?}, pdflatex {:?}",
                got_trace.get(k),
                exp.trace.get(k)
            ));
        }
        prev_trace = *trace_len;
        if *penalty != exp.penalty {
            return Err(format!("output {i}: \\outputpenalty {penalty} vs pdflatex {}", exp.penalty));
        }
        if sig != &exp.boxsig {
            let k = sig.iter().zip(&exp.boxsig).position(|(a, b)| a != b).unwrap_or(sig.len().min(exp.boxsig.len()));
            return Err(format!(
                "output {i}: box255 differs at node {k} (len {} vs {}): got {:?}, pdflatex {:?}",
                sig.len(),
                exp.boxsig.len(),
                sig.get(k),
                exp.boxsig.get(k)
            ));
        }
        match (&exp.column, col) {
            (None, None) => {}
            (Some(expc), Some(c)) => {
                pages += 1;
                if expc.len() != c.lines.len() {
                    return Err(format!("output {i}: column has {} boxes vs pdflatex {}", c.lines.len(), expc.len()));
                }
                for (k, ((h, d, y), (b, gy))) in expc.iter().zip(&c.lines).enumerate() {
                    if (*h, *d) != (b.height, b.depth) {
                        return Err(format!("output {i}: column box {k} dims differ"));
                    }
                    let dev = (y - gy).abs();
                    max_dev = max_dev.max(dev);
                    if dev > TOLERANCE {
                        return Err(format!(
                            "output {i}: column box {k} baseline {:.4}pt vs pdflatex {:.4}pt",
                            f64::from(*gy) / 65536.0,
                            f64::from(*y) / 65536.0
                        ));
                    }
                    lines += 1;
                }
            }
            (Some(_), None) => return Err(format!("output {i}: pdflatex shipped a page, the model did not")),
            (None, Some(_)) => return Err(format!("output {i}: the model shipped a page, pdflatex did not")),
        }
    }
    if rec.events.len() != e.events.len() {
        return Err(format!("{} outputs vs pdflatex {}", rec.events.len(), e.events.len()));
    }
    if trace[prev_trace..] != e.tail_trace[..] {
        return Err("trailing trace differs".into());
    }
    Ok(Outcome { pages, lines, max_dev })
}

fn fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("oracle/expected");
    let mut v: Vec<PathBuf> = fs::read_dir(dir).expect("oracle/expected").map(|e| e.unwrap().path()).collect();
    v.retain(|p| p.extension().is_some_and(|x| x == "ftpb"));
    v.sort();
    v
}

#[test]
fn pdflatex_page_builder_oracle() {
    let files = fixtures();
    assert!(files.len() >= 50, "expected at least 50 oracle fixtures, found {}", files.len());
    let mut failures = Vec::new();
    let (mut pages, mut lines, mut max_dev) = (0, 0, 0);
    for f in &files {
        let name = f.file_stem().unwrap().to_string_lossy().to_string();
        match parse(f).and_then(|e| replay(&e)) {
            Ok(o) => {
                pages += o.pages;
                lines += o.lines;
                max_dev = max_dev.max(o.max_dev);
            }
            Err(msg) => failures.push(format!("{name}: {msg}")),
        }
    }
    eprintln!(
        "page-builder oracle: {}/{} fixtures identical to pdflatex ({} pages, {} lines, max baseline deviation {:.5}pt)",
        files.len() - failures.len(),
        files.len(),
        pages,
        lines,
        f64::from(max_dev) / 65536.0
    );
    for f in &failures {
        eprintln!("  FAIL {f}");
    }
    assert!(failures.is_empty(), "{} oracle fixtures differ", failures.len());
}
