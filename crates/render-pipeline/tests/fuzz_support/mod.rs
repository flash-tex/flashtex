//! Deterministic mutation fuzzer for the render pipeline and the exact PDF
//! route (stable Rust, no deps).
//!
//! The same approach as the compiler's harness (`crates/compiler/tests/
//! fuzz_support`, PR #577), one level up: each case is written to a project
//! directory and run through
//!
//! 1. `parse`: the compiler alone (`parser::parse_project`), so a hang or
//!    overflow that is the compiler's own is attributed to it;
//! 2. `render`: `protocol::handle_line`, the runtime-v1 request
//!    `flashtex-render --tex` and the worker (`flashtex worker`) serve
//!    (parse → adapt → typeset → display list → v1 fallback and the
//!    `display_list` envelope), with the documents that project-files'
//!    discovery reads off disk, as `flashtex build` loads them;
//! 3. `pdf`: `pdf::write_pdf_exact`, the route `flashtex build` writes
//!    (v2 envelope → crates/pdf `v2` adapter → `exact` writer → self-check).
//!
//! Shared by the `#[ignore]` integration test `tests/fuzz_render.rs` and
//! `cargo run --release --example fuzz_render`. Every case is a pure function
//! of `(rng seed, case index)`, so any finding replays exactly.
//!
//! Isolation: a stack overflow aborts the whole process and a hung thread
//! cannot be killed, so cases run in worker *processes* (the same binary,
//! re-executed). A worker announces each case and stage on stdout before
//! running it; when it dies (overflow, abort, or the supervisor's memory
//! watchdog) or reports a hang and exits, the supervisor knows exactly which
//! case and stage did it and restarts a worker after that case.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use flashtex_compiler::json;
use flashtex_compiler::parser::SourceDocument;
use flashtex_project_files::{ProjectGraph, ProjectPath};
use flashtex_render_pipeline::{pdf, protocol, FontSet, RenderOptions};

/// The entry document's path in every case; `\input{main}` names itself.
pub const ENTRY: &str = "main.tex";
/// Stack for a case thread: the size of a macOS/Linux main thread, where
/// `flashtex build` and `flashtex-render` run the render.
/// `FLASHTEX_FUZZ_STACK_KB` overrides.
pub const CASE_STACK: usize = 8 * 1024 * 1024;
/// Resident-set ceiling per worker process before the supervisor kills it
/// and records the case (`--max-rss-mb`).
pub const DEFAULT_MAX_RSS_MB: u64 = 4096;

// ---------------------------------------------------------------- RNG

pub struct Rng(u64);

impl Rng {
    /// splitmix64 over `seed` and `index`: independent streams per case.
    pub fn for_case(seed: u64, index: u64) -> Rng {
        let mut z = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

// ---------------------------------------------------------------- seeds

pub struct Seed {
    pub name: String,
    pub text: String,
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every UTF-8 `.tex` under `fixtures/` and `crates/*/{tests,oracle,fixtures}`,
/// in sorted order (never `vendor/` or a `target*` directory).
pub fn load_seeds(root: &Path) -> Vec<Seed> {
    let mut files = Vec::new();
    walk(&root.join("fixtures"), &mut files, ".tex");
    if let Ok(entries) = std::fs::read_dir(root.join("crates")) {
        for entry in entries.flatten() {
            for sub in ["tests", "oracle", "fixtures"] {
                walk(&entry.path().join(sub), &mut files, ".tex");
            }
        }
    }
    files.sort();
    files.dedup();
    files
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some(Seed { name, text })
        })
        .collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, suffix: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if name != "vendor" && !name.starts_with("target") && !name.starts_with('.') {
                walk(&path, out, suffix);
            }
        } else if name.ends_with(suffix) {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------- cases

#[derive(Clone, Debug)]
pub struct Case {
    /// `(path, text)`; the first is [`ENTRY`].
    pub documents: Vec<(String, String)>,
    pub seed_name: String,
    pub mutations: Vec<&'static str>,
}

impl Case {
    pub fn from_text(text: String) -> Case {
        Case {
            documents: vec![(ENTRY.into(), text)],
            seed_name: "file".into(),
            mutations: vec![],
        }
    }
    pub fn main(&self) -> &str {
        &self.documents[0].1
    }
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn random_offset(rng: &mut Rng, text: &str) -> usize {
    floor_boundary(text, rng.below(text.len() + 1))
}

/// Mostly a body offset (between `\begin{document}` and `\end{document}`),
/// where the pipeline-specific pieces reach the typesetter; sometimes
/// anywhere.
fn body_offset(rng: &mut Rng, text: &str) -> usize {
    if rng.chance(75) {
        if let Some(begin) = text.find("\\begin{document}") {
            let start = begin + "\\begin{document}".len();
            let end = text.rfind("\\end{document}").filter(|e| *e >= start).unwrap_or(text.len());
            return floor_boundary(text, start + rng.below(end - start + 1));
        }
    }
    random_offset(rng, text)
}

/// Byte ranges of the structural tokens the mutations delete or duplicate.
fn structural_tokens(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'}' | b'[' | b']' | b'$' | b'&' => out.push((i, i + 1)),
            b'\\' => {
                let rest = &text[i..];
                if rest.starts_with("\\begin{") || rest.starts_with("\\end{") {
                    if let Some(close) = rest.find('}') {
                        out.push((i, i + close + 1));
                        i += close + 1;
                        continue;
                    }
                } else if rest.starts_with("\\begin") || rest.starts_with("\\end") {
                    let len = if rest.starts_with("\\begin") { 6 } else { 4 };
                    out.push((i, i + len));
                } else if rest.starts_with("\\\\") || rest.starts_with("\\item") || rest.starts_with("\\newpage") {
                    let len = if rest.starts_with("\\\\") { 2 } else if rest.starts_with("\\item") { 5 } else { 8 };
                    out.push((i, i + len));
                }
                // Skip the escaped character so `\{` is not a brace.
                i += 1;
                if i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                    i += text[i..].chars().next().map_or(1, char::len_utf8);
                    continue;
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

const CONDITIONALS: &[&str] = &[
    "\\iftrue ",
    "\\iffalse ",
    "\\fi ",
    "\\else ",
    "\\ifx\\a\\b ",
    "\\ifnum1<2 ",
    "\\ifdim1pt<2pt ",
    "\\ifcase3 ",
    "\\or ",
    "\\ifcat a b",
    "\\if aa",
    "\\unless\\ifx ",
    "\\ifdefined ",
    "\\newif\\ifq \\ifq ",
    "\\ifmmode ",
    "\\ifhmode ",
    "\\csname iftrue\\endcsname ",
];

const RECURSIONS: &[&str] = &[
    "\\def\\a{\\a}\\a ",
    "\\def\\a{x\\a}\\a ",
    "\\def\\a{\\a x}\\a ",
    "\\def\\a#1{\\a{#1#1}}\\a x",
    "\\def\\a{\\b}\\def\\b{\\a}\\a ",
    "\\newcommand{\\a}{\\a}\\a ",
    "\\newcommand\\a[1]{\\a{#1}}\\a{x}",
    "\\renewcommand\\a{{\\a}}\\a ",
    "\\let\\a\\relax\\def\\a{\\a\\a}\\a ",
    "\\edef\\a{\\a}\\a ",
    "\\def\\a{\\expandafter\\a}\\a ",
    "\\def\\a{\\csname a\\endcsname}\\a ",
    "\\def\\a{\\begin{a}}\\newenvironment{a}{\\a}{}\\a ",
    "\\newenvironment{rec}{\\begin{rec}}{\\end{rec}}\\begin{rec}\\end{rec}",
    "\\def\\a{\\section{\\a}}\\a ",
    "\\def\\a{$\\a$}\\a ",
    "\\def\\a{\\iftrue\\a\\fi}\\a ",
    "\\def\\a{\\def\\a{\\a}\\a}\\a ",
    "\\def\\a{\\input{main}}\\a ",
    "\\DeclareRobustCommand\\a{\\a}\\a ",
    "\\providecommand\\a{\\a}\\a ",
    "\\def\\a{\\a#}\\a ",
];

const CHARS: &[&str] = &[
    "\\char\"D800 ",
    "\\char\"DFFF ",
    "\\char55296 ",
    "\\char\"110000 ",
    "\\char-1 ",
    "\\char\"FFFFFFFF ",
    "\\char99999999999999999999 ",
    "\\symbol{\"D800}",
    "\\symbol{-5}",
    "^^^^d800",
    "^^^^^^10ffff",
    "^^^^^^110000",
    "\\char`\\",
    "\\char'777777777 ",
    "\\char\"",
    "\\Uchar\"D800 ",
    "\\mathchar\"FFFFFF ",
    "\\delimiter\"FFFFFFFFF ",
    "\\catcode`\\{=12 ",
    "\\catcode 300=1 ",
    "\\lccode\"D800=1 ",
    "$\\char\"D800$",
    "\\textsuperscript{\\char\"DC00}",
];

const INPUTS: &[&str] = &[
    "\\input{main}",
    "\\input{main.tex}",
    "\\input main ",
    "\\include{main}",
    "\\input{sub}",
    "\\subfile{main}",
    "\\include{sub}",
    "\\InputIfFileExists{main}{}{}",
    "\\input{./main}",
];

const NESTERS: &[(&str, &str)] = &[
    ("{", "}"),
    ("\\begin{itemize}\\item ", "\\end{itemize}"),
    ("\\begin{quote}", "\\end{quote}"),
    ("$\\left(", "\\right)$"),
    ("\\left(", "\\right)"),
    ("\\textbf{", "}"),
    ("[", "]"),
    ("\\begingroup ", "\\endgroup "),
    ("\\sqrt{", "}"),
    ("\\footnote{", "}"),
    ("\\begin{minipage}{1cm}", "\\end{minipage}"),
    ("\\fbox{", "}"),
    ("\\begin{tabular}{c}", "\\end{tabular}"),
];

// ------------------------------------------------ pipeline-specific pieces

/// Floats (and float-like environments the pipeline masks before parsing).
const FLOATS: &[&str] = &[
    "\\begin{figure}[h]\\centering x\\caption{c}\\label{fig:f}\\end{figure}",
    "\\begin{table}[!b]\\begin{tabular}{c}a\\end{tabular}\\caption{t}\\end{table}",
    "\\begin{figure*}[p]\\includegraphics{huge.png}\\end{figure*}",
    "\\begin{figure}[H]x\\end{figure}",
    "\\begin{figure}[tbp!]\\end{figure}",
    "\\begin{figure}",
    "\\end{figure}",
    "\\begin{table}[]\\caption{}\\end{table}",
    "\\begin{figure}[x]\\caption{\\caption{y}}\\end{figure}",
    "\\begin{wrapfigure}{r}{-5pt}x\\end{wrapfigure}",
    "\\begin{figure}[h]\\begin{figure}[h]x\\end{figure}\\end{figure}",
    "\\begin{figure}[h]\\vspace{-1000pt}\\caption{\\begin{table}x\\end{table}}\\end{figure}",
    "\\begin{figure}[b]\\footnote{\\begin{figure}y\\end{figure}}\\end{figure}",
    "\\begin{table*}[t]\\begin{multicols}{2}x\\end{multicols}\\end{table*}",
    "\\begin{figure}[p]\\rule{1pt}{2000pt}\\end{figure}",
    "\\begin{figure}[t]\\includegraphics[width=\\textwidth,height=3\\textheight]{images/red-72.png}\\end{figure}",
    "\\begin{figure}[h]\\begin{minipage}{0.5\\linewidth}\\includegraphics{nofile}\\end{minipage}\\end{figure}",
    "\\begin{figure}[!htbp]\\newpage\\clearpage\\end{figure}",
    "\\begin{table}\\begin{longtable}{c}a\\\\\\end{longtable}\\end{table}",
    "\\begin{figure}[h]\\begin{lstlisting}\n\\end{figure}\n\\end{lstlisting}\\end{figure}",
    "\\begin{figure}\\end{table}",
    "\\begin{figure}[h]\\marginpar{m}\\end{figure}",
];

/// Odd places for a float: `{F}` is replaced.
const FLOAT_CONTEXTS: &[&str] = &[
    "${F}$",
    "$${F}$$",
    "\\[{F}\\]",
    "\\begin{tabular}{c}{F}&{F}\\\\\\end{tabular}",
    "\\footnote{{F}}",
    "\\section{{F}}",
    "\\begin{itemize}\\item {F}\\end{itemize}",
    "\\begin{minipage}{2cm}{F}\\end{minipage}",
    "\\mbox{{F}}",
    "\\parbox{0pt}{{F}}",
    "\\caption{{F}}",
    "\\begin{multicols}{2}{F}\\end{multicols}",
    "\\marginpar{{F}}",
    "\\begin{center}{F}\\end{center}",
    "{F}",
    "\\begin{equation}{F}\\end{equation}",
    "\\title{{F}}\\maketitle ",
    "\\begin{abstract}{F}\\end{abstract}",
    "\\begin{description}\\item[{F}] x\\end{description}",
    "\\fbox{{F}}",
    "\\begin{align}a&{F}\\\\\\end{align}",
    "\\begin{tikzpicture}\\node {{F}};\\end{tikzpicture}",
    "\\begin{verbatim}{F}\\end{verbatim}",
    "\\verb|{F}|",
    "\\raisebox{-1000pt}{{F}}",
    "\\begin{thebibliography}{9}\\bibitem{k}{F}\\end{thebibliography}",
    "\\twocolumn[{F}]",
    "\\begin{enumerate}\\item\\begin{tabular}{p{1cm}}{F}\\end{tabular}\\end{enumerate}",
    "\\begin{theorem}{F}\\end{theorem}",
    "\\begin{proof}{F}\\end{proof}",
    "\\begin{longtable}{c}{F}\\\\\\end{longtable}",
];

/// Openers/closers for nested lists and tables (depths 5..400).
const STRUCTURE_NESTERS: &[(&str, &str)] = &[
    ("\\begin{enumerate}\\item a ", "\\end{enumerate}"),
    ("\\begin{description}\\item[x] ", "\\end{description}"),
    ("\\begin{itemize}\\item a\\item ", "\\item b\\end{itemize}"),
    ("\\begin{tabular}{|c|c|}\\hline a&", "\\\\\\hline\\end{tabular}"),
    ("\\begin{tabular}{p{1cm}}", "\\\\\\end{tabular}"),
    ("$\\begin{array}{cc}a&", "\\end{array}$"),
    ("\\begin{array}{c}", "\\end{array}"),
    ("\\begin{center}", "\\end{center}"),
    ("\\begin{enumerate}[label=(\\alph*)]\\item ", "\\end{enumerate}"),
    ("\\begin{tabular}{c}\\begin{itemize}\\item ", "\\end{itemize}\\end{tabular}"),
    ("\\begin{minipage}{\\linewidth}", "\\end{minipage}"),
    ("\\begin{multicols}{2}", "\\end{multicols}"),
    ("\\begin{tabularx}{\\linewidth}{X}", "\\end{tabularx}"),
    ("\\begin{longtable}{c}", "\\\\\\end{longtable}"),
    ("\\parbox{\\linewidth}{", "}"),
    ("\\begin{list}{}{}\\item ", "\\end{list}"),
    ("\\begin{trivlist}\\item ", "\\end{trivlist}"),
    ("\\begin{verse}", "\\end{verse}"),
    ("\\begin{flushright}", "\\end{flushright}"),
    ("\\begin{tikzpicture}\\node{", "};\\end{tikzpicture}"),
    ("\\begin{align}\\begin{aligned}", "\\end{aligned}\\end{align}"),
    ("\\begin{cases}", "\\end{cases}"),
    ("\\subsection{", "}"),
];

const TABLE_PIECES: &[&str] = &[
    "\\begin{tabular}{c}\\multicolumn{99999}{c}{x}\\end{tabular}",
    "\\begin{tabular}{cc}\\multicolumn{0}{c}{x}\\end{tabular}",
    "\\begin{tabular}{cc}\\multicolumn{-3}{c}{x}&\\end{tabular}",
    "\\begin{tabular}{cc}a&b\\\\\\cline{5-2}\\end{tabular}",
    "\\begin{tabular}{cc}a&b\\\\\\cline{1-99999}\\end{tabular}",
    "\\begin{tabular}{}x&y\\end{tabular}",
    "\\begin{tabular}{@{}}\\end{tabular}",
    "\\begin{tabular}{c}a\\\\[-1000pt]b\\\\[16383pt]c\\end{tabular}",
    "\\begin{tabular}{p{-10pt}}x y z\\end{tabular}",
    "\\begin{tabular}{p{0pt}}xxxxxxxxxxxxxxxxxxxxx\\end{tabular}",
    "\\begin{tabular}{*{100000}{c}}a\\end{tabular}",
    "\\begin{tabular}{*{-1}{c}}a\\end{tabular}",
    "\\begin{tabular}{*{2147483647}{c}}a\\end{tabular}",
    "\\begin{tabular}{m{-1cm}b{16383pt}}a&b\\end{tabular}",
    "\\begin{tabular}{|||||||}\\hline\\hline\\end{tabular}",
    "\\setlength{\\tabcolsep}{-1000pt}\\begin{tabular}{cc}a&b\\end{tabular}",
    "\\renewcommand{\\arraystretch}{-5}\\begin{tabular}{c}a\\\\b\\end{tabular}",
    "\\renewcommand{\\arraystretch}{100000}\\begin{tabular}{c}a\\\\b\\end{tabular}",
    "\\setcounter{enumi}{2147483647}\\begin{enumerate}\\item a\\item b\\end{enumerate}",
    "\\begin{enumerate}\\setcounter{enumi}{-5}\\item a\\end{enumerate}",
    "\\setcounter{section}{99999999}\\renewcommand\\thesection{\\Alph{section}}\\section{x}",
    "\\setcounter{page}{-1}\\pagenumbering{roman}x\\newpage y",
    "\\pagenumbering{alph}\\setcounter{page}{30}x\\newpage",
    "\\setcounter{footnote}{-2147483648}\\footnote{x}\\renewcommand\\thefootnote{\\fnsymbol{footnote}}\\footnote{y}",
    "\\begin{enumerate}[start=-2147483648,label=\\roman*]\\item a\\item b\\end{enumerate}",
    "\\begin{multicols}{0}x\\end{multicols}",
    "\\begin{multicols}{-3}x\\end{multicols}",
    "\\begin{multicols}{100000}x\\end{multicols}",
    "\\begin{longtable}{c}\\endhead\\endfoot\\endlastfoot\\end{longtable}",
    "\\begin{tabular}{c}\\hline\\hline\\hline\\hline\\end{tabular}",
];

const DIMENSIONS: &[&str] = &[
    "\\includegraphics[width=16383pt]{huge.png}",
    "\\includegraphics[scale=1e9]{images/red-72.png}",
    "\\includegraphics[scale=100000]{images/green-144dpi.png}",
    "\\includegraphics[scale=0]{images/red-72.png}",
    "\\includegraphics[scale=-1]{images/red-72.png}",
    "\\includegraphics[height=-5pt]{images/tall-72.png}",
    "\\includegraphics[angle=1e308]{images/blue-96dpi.jpg}",
    "\\includegraphics[angle=-90,width=0pt]{images/box-crop.pdf}",
    "\\includegraphics[width=0pt,height=0pt]{images/red-72.png}",
    "\\includegraphics[width=\\maxdimen,height=\\maxdimen]{images/red-72.png}",
    "\\includegraphics[width=16383.99999pt,keepaspectratio]{wide.png}",
    "\\includegraphics{huge.png}",
    "\\includegraphics{wide.png}",
    "\\includegraphics{tall.png}",
    "\\includegraphics{zero.png}",
    "\\includegraphics{trunc.png}",
    "\\includegraphics{badidat.png}",
    "\\includegraphics{huge.jpg}",
    "\\includegraphics{zero.jpg}",
    "\\includegraphics{trunc.jpg}",
    "\\includegraphics{bad.pdf}",
    "\\includegraphics{loop.pdf}",
    "\\includegraphics{empty.pdf}",
    "\\includegraphics[page=4294967295]{images/box-media.pdf}",
    "\\includegraphics[page=0]{images/box-media.pdf}",
    "\\includegraphics[page=-1]{bad.pdf}",
    "\\includegraphics[viewport=0 0 1e30 1e30,clip]{images/box-crop.pdf}",
    "\\includegraphics[trim=-1000 -1000 99999 99999,clip]{images/red-72.png}",
    "\\includegraphics[bb=0 0 0 0]{images/red-72.png}",
    "\\includegraphics[natwidth=-1,natheight=0]{images/red-72.png}",
    "\\includegraphics[totalheight=1e10pt]{images/red-72.png}",
    "\\scalebox{100000}{x}",
    "\\scalebox{0}{x}",
    "\\scalebox{-1e30}[1e30]{x}",
    "\\resizebox{16383pt}{!}{x}",
    "\\resizebox{0pt}{0pt}{x}",
    "\\resizebox{-5pt}{-5pt}{\\includegraphics{huge.png}}",
    "\\rotatebox{1e308}{x}",
    "\\rule{16383pt}{16383pt}",
    "\\rule{-16383pt}{-16383pt}",
    "\\rule[1e5pt]{1pt}{1pt}",
    "\\hspace{16383.99999pt}",
    "\\hspace*{\\maxdimen}\\hspace*{\\maxdimen}x",
    "\\setlength{\\textwidth}{-100pt}",
    "\\setlength{\\textwidth}{16383pt}",
    "\\setlength{\\paperheight}{0pt}",
    "\\setlength{\\paperwidth}{-1pt}",
    "\\usepackage[paperwidth=100000pt,paperheight=1pt]{geometry}",
    "\\usepackage[margin=-10in]{geometry}",
    "\\usepackage[textwidth=0pt,textheight=0pt]{geometry}",
    "\\geometry{paperwidth=1sp,paperheight=1sp}",
    "\\setlength{\\linewidth}{0pt}",
    "\\setlength{\\columnwidth}{-5pt}",
    "\\begin{tikzpicture}\\draw (0,0) -- (1e10,1e10);\\end{tikzpicture}",
    "\\begin{tikzpicture}\\draw (0,0) circle (-1);\\fill (0,0) rectangle (16383pt,16383pt);\\end{tikzpicture}",
    "\\begin{tikzpicture}[scale=1e9]\\draw (0,0) grid (10,10);\\end{tikzpicture}",
    "\\begin{tikzpicture}[scale=0]\\node{x};\\end{tikzpicture}",
    "\\begin{tikzpicture}\\draw[step=1sp] (0,0) grid (100,100);\\end{tikzpicture}",
    "\\begin{tikzpicture}\\foreach \\i in {1,...,1000000} {\\draw (\\i,0) -- (0,\\i);}\\end{tikzpicture}",
    "\\begin{tikzpicture}\\draw (0,0) arc (0:1e9:1);\\end{tikzpicture}",
    "\\color[rgb]{2,-1,1e30}x",
    "\\textcolor[HTML]{ZZZZZZ}{x}",
    "\\colorbox{red!-500}{x}",
    "\\definecolor{c}{rgb}{nan,inf,-inf}\\color{c}x",
    "\\makebox[\\maxdimen][s]{a b}",
    "\\framebox[-100pt][r]{x}",
    "\\parbox[t][-100pt][s]{16383pt}{x}",
    "\\begin{minipage}[t][16383pt][s]{-1pt}x\\end{minipage}",
    "\\setlength{\\fboxsep}{-100pt}\\fbox{x}",
    "\\setlength{\\fboxrule}{16383pt}\\fbox{x}",
    "\\setlength{\\unitlength}{16383pt}\\begin{picture}(1000,1000)\\put(1000,1000){x}\\end{picture}",
    "\\begin{picture}(-10,-10)(1e10,1e10)\\line(1,0){1e10}\\end{picture}",
];

const MISSING_GRAPHICS: &[&str] = &[
    "\\includegraphics{nofile}",
    "\\includegraphics{nofile.png}",
    "\\includegraphics{../../etc/passwd}",
    "\\includegraphics{/etc/passwd}",
    "\\includegraphics{/dev/zero}",
    "\\includegraphics{}",
    "\\includegraphics{ }",
    "\\includegraphics{main.tex}",
    "\\includegraphics{sub.tex}",
    "\\includegraphics{images}",
    "\\includegraphics{images/}",
    "\\includegraphics{.}",
    "\\includegraphics{dir.png}",
    "\\includegraphics{a\\space b.png}",
    "\\includegraphics{\\jobname}",
    "\\includegraphics{%\n}",
    "\\includegraphics[draft]{nofile.pdf}",
    "\\includegraphics[width=\\textwidth]{missing/deep/path/x.jpg}",
    "\\includegraphics[]{nofile}",
    "\\includegraphics[width=]{nofile}",
    "\\includegraphics[=,=,==]{nofile}",
    "\\includegraphics*{nofile}",
    "\\includegraphics[page=2]{nofile}",
    "\\graphicspath{{../}{/}{images/}}\\includegraphics{red-72}",
    "\\graphicspath{{}}\\includegraphics{x}",
    "\\DeclareGraphicsExtensions{.xyz}\\includegraphics{images/red-72}",
    "\\includegraphics{nul\u{0}file.png}",
    "\\includegraphics{\u{10FFFF}.png}",
    "\\includegraphics{CON}",
    "\\includegraphics{images/red-72.png\\includegraphics{x}}",
    "\\input{nofile}",
    "\\include{missing/chapter}",
    "\\lstinputlisting{nofile.py}",
    "\\bibliography{nofile}",
];

const FONTS: &[&str] = &[
    "\\fontsize{0.0001pt}{0pt}\\selectfont x y z",
    "\\fontsize{16383pt}{1pt}\\selectfont xy",
    "\\fontsize{16383.99pt}{16383.99pt}\\selectfont x\\par y",
    "\\fontsize{-10pt}{-10pt}\\selectfont x",
    "\\fontsize{0pt}{0pt}\\selectfont x\\par y",
    "\\fontsize{1sp}{1sp}\\selectfont $x^2_{y}\\sqrt{\\frac ab}$",
    "\\fontsize{2000pt}{2400pt}\\selectfont $\\left(\\sum_{i}^{n}\\right)$",
    "\\fontsize{nan}{inf}\\selectfont x",
    "\\fontsize{}{}\\selectfont x",
    "\\fontsize{1e10}{1e10}\\selectfont x",
    "{\\Huge\\Huge\\Huge x}",
    "{\\tiny\\tiny\\scriptsize $\\scriptscriptstyle x$}",
    "\\usefont{T1}{nonexistent}{b}{it}x",
    "\\fontfamily{zzz}\\fontseries{qq}\\fontshape{zz}\\selectfont x",
    "\\fontencoding{XYZ}\\selectfont x",
    "\\font\\x=cmr10 at 0.00001pt \\x a",
    "\\font\\x=cmr10 at 2047pt \\x a",
    "\\font\\x=cmr10 at -5pt \\x a",
    "\\font\\x=cmr10 scaled 0 \\x a",
    "\\font\\x=cmr10 scaled 32768 \\x a",
    "\\font\\x=nonexistentfont \\x a",
    "\\font\\x=lmroman10-regular at 16383pt \\x a",
    "\\DeclareMathSizes{10}{0}{0}{0}$x^{y^z}$",
    "\\DeclareMathSizes{12}{16383}{1}{-1}$x^{y^z}$",
    "\\linespread{0}\\selectfont a\\par b",
    "\\linespread{-1}\\selectfont a\\par b",
    "\\linespread{100000}\\selectfont a\\par b",
    "\\renewcommand{\\baselinestretch}{-3}\\selectfont a\\par b",
    "\\usepackage[scale=1000]{tgheros}",
    "\\usepackage{nonexistentfontpackage}",
    "\\fontsize{16383pt}{0pt}\\selectfont\\section{x}\\footnote{y}",
    "\\setlength{\\baselineskip}{0pt plus -1fil}x\\par y",
    "\\scriptsize\\fontsize{5000pt}{1pt}\\selectfont\\begin{tabular}{c}a\\\\b\\end{tabular}",
    "\\fontsize{1000pt}{1000pt}\\selectfont\\begin{lstlisting}\nx\n\\end{lstlisting}",
];

const LENGTHS: &[&str] = &[
    "\\setlength{\\baselineskip}{0pt}",
    "\\setlength{\\baselineskip}{-100pt}",
    "\\setlength{\\parskip}{-100pt}",
    "\\setlength{\\parskip}{0pt plus 1fil minus 1fil}",
    "\\setlength{\\columnsep}{-500pt}",
    "\\setlength{\\columnsep}{16383pt}",
    "\\setlength{\\textheight}{0pt}",
    "\\setlength{\\textheight}{-5pt}",
    "\\setlength{\\textheight}{1sp}",
    "\\setlength{\\hsize}{0pt}",
    "\\hsize=-10pt ",
    "\\vsize=0pt ",
    "\\begin{minipage}{0pt}some words here\\end{minipage}",
    "\\begin{minipage}{-10pt}x\\end{minipage}",
    "\\parbox{-10pt}{x y}",
    "\\parbox{0pt}{abcdefghijklmnopqrstuvwxyz}",
    "\\setlength{\\topmargin}{-10000pt}",
    "\\setlength{\\topmargin}{16383pt}",
    "\\setlength{\\footskip}{-1pt}",
    "\\setlength{\\headheight}{-100pt}",
    "\\setlength{\\headsep}{16383pt}",
    "\\setlength{\\leftmargini}{-100pt}",
    "\\setlength{\\leftmargini}{16383pt}",
    "\\setlength{\\itemsep}{-50pt}",
    "\\setlength{\\labelwidth}{-5pt}\\setlength{\\labelsep}{-1000pt}",
    "\\setlength{\\emergencystretch}{-1pt}",
    "\\setlength{\\emergencystretch}{\\maxdimen}",
    "\\tolerance=-1 ",
    "\\tolerance=10000 \\pretolerance=-1 ",
    "\\hyphenpenalty=-10000 ",
    "\\exhyphenpenalty=10000 \\hyphenpenalty=10000 ",
    "\\setlength{\\parindent}{-16383pt}",
    "\\setlength{\\parindent}{16383pt}",
    "\\setlength{\\oddsidemargin}{-16383pt}\\setlength{\\evensidemargin}{16383pt}",
    "\\setlength{\\marginparwidth}{-1pt}\\marginpar{x}",
    "\\setlength{\\abovedisplayskip}{-1000pt}\\setlength{\\belowdisplayskip}{-1000pt}",
    "\\setlength{\\mathindent}{-100pt}",
    "\\setlength{\\jot}{-100pt}",
    "\\setlength{\\lineskip}{-100pt}\\setlength{\\lineskiplimit}{16383pt}",
    "\\setlength{\\topsep}{-100pt}\\setlength{\\partopsep}{-100pt}",
    "\\setlength{\\floatsep}{-500pt}\\setlength{\\textfloatsep}{-500pt}\\setlength{\\intextsep}{-500pt}",
    "\\renewcommand{\\topfraction}{-1}\\renewcommand{\\textfraction}{2}",
    "\\setcounter{topnumber}{-1}\\setcounter{totalnumber}{0}",
    "\\setlength{\\footnotesep}{-100pt}\\setlength{\\skip\\footins}{-100pt}",
    "\\setlength{\\maxdepth}{-10pt}",
    "\\setlength{\\lineskiplimit}{-\\maxdimen}",
    "\\setlength{\\overfullrule}{16383pt}",
    "\\setlength{\\arrayrulewidth}{-5pt}",
    "\\setlength{\\doublerulesep}{16383pt}",
    "\\setlength{\\cmidrulewidth}{-1pt}",
    "\\setlength{\\multicolsep}{-1000pt}",
    "\\setlength{\\columnseprule}{-10pt}",
    "\\setlength{\\parfillskip}{-100pt}",
    "\\setlength{\\rightskip}{16383pt}\\setlength{\\leftskip}{16383pt}",
    "\\hangindent=-16383pt \\hangafter=-1 ",
    "\\parshape 2 0pt -10pt 16383pt 0pt ",
    "\\looseness=-1000 ",
    "\\setlength{\\itemindent}{-16383pt}",
    "\\setlength{\\lineskip}{0pt}\\setlength{\\baselineskip}{0pt}\\setlength{\\lineskiplimit}{0pt}",
];

const VSPACE: &[&str] = &[
    "\\vspace{-1000pt}",
    "\\vspace*{-1000pt}",
    "\\vspace{16383pt}",
    "\\vspace*{16383pt}",
    "\\vspace{-\\maxdimen}",
    "\\vskip -\\maxdimen ",
    "\\vskip 16383pt plus -1fill minus 1filll ",
    "\\vspace{1fill}",
    "\\vspace{-1fill}",
    "\\vfill\\vfill\\vfill",
    "\\hspace{-1000pt}",
    "\\kern-16383pt ",
    "\\raisebox{-1000pt}{x}",
    "\\raisebox{16383pt}[-5pt][-5pt]{x}",
    "\\enlargethispage{-1000pt}",
    "\\enlargethispage*{16383pt}",
    "\\addvspace{-10pt}",
    "\\addvspace{16383pt}",
    "\\bigskip\\medskip\\smallskip\\vspace{-1000pt}\\vspace{-1000pt}",
    "\\\\[-1000pt]",
    "x\\\\[16383pt]y",
    "\\vspace{-1000pt}\\section{x}",
    "\\vspace{-1000pt}\\footnote{x}",
    "\\vspace{-1000pt}\\begin{figure}[t]x\\end{figure}",
    "\\vspace{-1000pt}\\newpage\\vspace*{-1000pt}",
    "\\begin{center}\\vspace{-1000pt}\\end{center}",
    "\\item\\vspace{-1000pt}",
    "\\vspace{\\textheight}\\vspace{\\textheight}x",
    "\\vbox to -100pt{x}",
    "\\vbox to 16383pt{\\vss x\\vss}",
    "\\vtop{\\vskip-1000pt x}",
    "$$\\vspace{-1000pt}$$",
    "\\vspace{1000pt}\\vspace{-2000pt}\\vspace{1000pt}",
    "\\vspace{0pt plus 1fill}\\vspace{0pt minus 1fill}",
];

const EMPTY_PAGES: &[&str] = &[
    "\\newpage",
    "\\clearpage\\clearpage",
    "\\null\\newpage",
    "\\cleardoublepage",
    "\\thispagestyle{empty}\\mbox{}\\newpage",
    "$\\pagebreak[4]$",
    "\\begin{tabular}{c}\\newpage\\end{tabular}",
    "\\twocolumn\\onecolumn",
    "\\vspace*{\\fill}",
    "\\maketitle\\maketitle",
    "\\newpage\\thispagestyle{empty}\\newpage",
    "\\pagebreak[4]\\nopagebreak[4]",
    "\\clearpage\\vspace*{\\fill}\\clearpage",
    "\\twocolumn[]\\newpage\\twocolumn[]",
    "\\begin{titlepage}\\end{titlepage}",
    "\\begin{titlepage}\\begin{titlepage}x\\end{titlepage}\\end{titlepage}",
    "\\footnote{\\newpage}",
    "\\section{\\newpage}",
    "\\begin{multicols}{2}\\columnbreak\\columnbreak\\newpage\\end{multicols}",
    "\\tableofcontents\\newpage\\listoffigures\\listoftables",
    "\\appendix\\newpage",
    "\\end{document}",
    "\\begin{document}\\end{document}",
    "\\end{document}\\begin{document}",
    "\\stop",
    "\\enddocument",
];

fn overfull_piece(rng: &mut Rng) -> String {
    let len = if rng.chance(20) { 20_000 } else { 50 + rng.below(3000) };
    let word: String = std::iter::repeat('m').take(len).collect();
    match rng.below(14) {
        0 => format!("\\hbox to 1pt{{{word}}}"),
        1 => format!("\\mbox{{{word}}}"),
        2 => word,
        3 => format!("\\makebox[0pt]{{{word}}}"),
        4 => format!("\\fbox{{\\parbox{{2\\textwidth}}{{{word}}}}}"),
        5 => format!("\\underline{{{word}}}"),
        6 => format!("\\url{{https://{word}}}"),
        7 => format!("\\sloppy {word} \\fussy {word}"),
        8 => format!("\\hbox{{\\vrule width 100000pt}}{word}"),
        9 => format!("\\begin{{tabular}}{{c}}{word}\\end{{tabular}}"),
        10 => format!("${}$", "x+".repeat(len / 2)),
        11 => format!("\\texttt{{{}}}", "a/".repeat(len / 2)),
        12 => format!("\\section{{{word}}}"),
        _ => format!("\\begin{{itemize}}\\item[{word}] x\\end{{itemize}}"),
    }
}

pub fn generate(seeds: &[Seed], rng_seed: u64, index: u64) -> Case {
    let mut rng = Rng::for_case(rng_seed, index);
    let seed = rng.pick(seeds);
    let mut text = seed.text.clone();
    let mut documents_extra: Vec<(String, String)> = Vec::new();
    let mut mutations = Vec::new();
    let count = 1 + rng.below(4);
    for _ in 0..count {
        let kind = rng.below(21);
        match kind {
            0 => {
                mutations.push("truncate");
                let at = random_offset(&mut rng, &text);
                text.truncate(at);
            }
            1 | 2 => {
                let tokens = structural_tokens(&text);
                if tokens.is_empty() {
                    continue;
                }
                let deletions = 1 + rng.below(8);
                if kind == 1 {
                    mutations.push("delete-token");
                } else {
                    mutations.push("duplicate-token");
                }
                // Apply right-to-left so earlier ranges stay valid.
                let mut chosen: Vec<(usize, usize)> =
                    (0..deletions).map(|_| *rng.pick(&tokens)).collect();
                chosen.sort();
                chosen.dedup();
                // Overlapping ranges (a `\begin{..}` containing a `{`) are
                // skipped so every range still indexes the edited text.
                let mut floor = usize::MAX;
                for (a, b) in chosen.into_iter().rev() {
                    if b > floor {
                        continue;
                    }
                    floor = a;
                    if kind == 1 {
                        text.replace_range(a..b, "");
                    } else {
                        let piece = text[a..b].to_string();
                        let copies = if rng.chance(10) { 1 + rng.below(50) } else { 1 };
                        text.insert_str(b, &piece.repeat(copies));
                    }
                }
            }
            3 => {
                mutations.push("splice-lines");
                let other = rng.pick(seeds);
                let lines: Vec<&str> = other.text.lines().collect();
                if lines.is_empty() {
                    continue;
                }
                let start = rng.below(lines.len());
                let end = (start + 1 + rng.below(20)).min(lines.len());
                let chunk = lines[start..end].join("\n") + "\n";
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, &chunk);
            }
            4 => {
                mutations.push("deep-nesting");
                let (open, close) = *rng.pick(NESTERS);
                let depth = if rng.chance(50) {
                    10_000
                } else {
                    50 + rng.below(3000)
                };
                let balanced = rng.chance(60);
                let mut piece = open.repeat(depth);
                piece.push('x');
                if balanced {
                    piece.push_str(&close.repeat(depth));
                }
                let at = body_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            5 => {
                mutations.push("long-control-sequence");
                let len = if rng.chance(50) {
                    100_000
                } else {
                    1 + rng.below(5000)
                };
                let letter = (b'a' + rng.below(26) as u8) as char;
                let mut piece = String::from("\\");
                piece.extend(std::iter::repeat(letter).take(len));
                if rng.chance(50) {
                    // Define it too, so the long name is looked up and stored.
                    piece = format!("\\def{piece}{{y}}{piece} ");
                }
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            6 => {
                mutations.push("conditional");
                for _ in 0..1 + rng.below(6) {
                    let piece = if rng.chance(5) {
                        rng.pick(CONDITIONALS).repeat(1 + rng.below(5000))
                    } else {
                        rng.pick(CONDITIONALS).to_string()
                    };
                    let at = random_offset(&mut rng, &text);
                    text.insert_str(at, &piece);
                }
            }
            7 => {
                mutations.push("recursion");
                let piece = *rng.pick(RECURSIONS);
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, piece);
            }
            8 => {
                mutations.push("invalid-char");
                for _ in 0..1 + rng.below(3) {
                    let piece = *rng.pick(CHARS);
                    let at = random_offset(&mut rng, &text);
                    text.insert_str(at, piece);
                }
            }
            9 => {
                mutations.push("self-input");
                let piece = *rng.pick(INPUTS);
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, piece);
                if documents_extra.is_empty() {
                    let sub = if rng.chance(50) {
                        "\\input{main}\n"
                    } else {
                        "x\\input{sub}\n"
                    };
                    documents_extra.push(("sub.tex".into(), sub.into()));
                }
            }
            10 => {
                mutations.push("delete-range");
                let a = random_offset(&mut rng, &text);
                let b = floor_boundary(&text, a + rng.below(200));
                text.replace_range(a..b, "");
            }
            11 => {
                mutations.push("duplicate-range");
                let a = random_offset(&mut rng, &text);
                let b = floor_boundary(&text, a + rng.below(400));
                let piece = text[a..b].to_string();
                let limit = if rng.chance(10) { 200 } else { 3 };
                let copies = 1 + rng.below(limit);
                text.insert_str(b, &piece.repeat(copies));
            }
            12 => {
                mutations.push("float-odd-place");
                let piece = if rng.chance(15) {
                    // Too many floats: the float queue and float pages.
                    let float = *rng.pick(FLOATS);
                    let sep = if rng.chance(50) { "\\clearpage " } else { " " };
                    format!("{float}{sep}").repeat(20 + rng.below(300))
                } else {
                    let float = *rng.pick(FLOATS);
                    rng.pick(FLOAT_CONTEXTS).replace("{F}", float)
                };
                let at = body_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            13 => {
                mutations.push("nested-structure");
                let piece = if rng.chance(40) {
                    let n = if rng.chance(20) { 20_000 } else { 1 + rng.below(300) };
                    match rng.below(4) {
                        0 => format!(
                            "\\begin{{tabular}}{{{}}}{}\\\\\\end{{tabular}}",
                            "c".repeat(n),
                            "a&".repeat(n)
                        ),
                        1 => format!("\\begin{{itemize}}{}\\end{{itemize}}", "\\item x ".repeat(n)),
                        2 => format!("\\begin{{tabular}}{{c}}{}\\end{{tabular}}", "a\\\\".repeat(n)),
                        _ => rng.pick(TABLE_PIECES).to_string(),
                    }
                } else {
                    let (open, close) = *rng.pick(STRUCTURE_NESTERS);
                    let depth = 5 + rng.below(400);
                    let mut piece = open.repeat(depth);
                    piece.push('x');
                    if rng.chance(70) {
                        piece.push_str(&close.repeat(depth));
                    }
                    piece
                };
                let at = body_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            14 => {
                mutations.push("huge-dimension");
                for _ in 0..1 + rng.below(3) {
                    let piece = *rng.pick(DIMENSIONS);
                    let at = body_offset(&mut rng, &text);
                    text.insert_str(at, piece);
                }
            }
            15 => {
                mutations.push("missing-graphics");
                let piece = *rng.pick(MISSING_GRAPHICS);
                let at = body_offset(&mut rng, &text);
                text.insert_str(at, piece);
            }
            16 => {
                mutations.push("extreme-font");
                for _ in 0..1 + rng.below(2) {
                    let piece = *rng.pick(FONTS);
                    let at = body_offset(&mut rng, &text);
                    text.insert_str(at, piece);
                }
            }
            17 => {
                mutations.push("zero-negative-length");
                for _ in 0..1 + rng.below(4) {
                    let piece = *rng.pick(LENGTHS);
                    // Preamble or body: both reach the page frame.
                    let at = random_offset(&mut rng, &text);
                    text.insert_str(at, piece);
                }
            }
            18 => {
                mutations.push("vspace");
                for _ in 0..1 + rng.below(4) {
                    let piece = *rng.pick(VSPACE);
                    let at = body_offset(&mut rng, &text);
                    text.insert_str(at, piece);
                }
            }
            19 => {
                mutations.push("overfull");
                let piece = overfull_piece(&mut rng);
                let at = body_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            _ => {
                mutations.push("empty-pages");
                let piece = *rng.pick(EMPTY_PAGES);
                let copies = if rng.chance(20) { 100 + rng.below(2000) } else { 1 + rng.below(5) };
                let at = body_offset(&mut rng, &text);
                text.insert_str(at, &piece.repeat(copies));
            }
        }
    }
    let mut documents = vec![(ENTRY.to_string(), text)];
    documents.extend(documents_extra);
    Case {
        documents,
        seed_name: seed.name.clone(),
        mutations,
    }
}

// ---------------------------------------------------------------- project directory

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// A zlib stream of `data` in one stored block.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01, 0x01];
    let len = data.len() as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

/// A PNG whose header claims `width` x `height` (8-bit RGB) with `idat` as
/// its only pixel data.
pub fn png(width: u32, height: u32, idat: &[u8]) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut out, b"IHDR", &ihdr);
    png_chunk(&mut out, b"IDAT", idat);
    png_chunk(&mut out, b"IEND", &[]);
    out
}

/// A baseline JPEG header claiming `width` x `height` (no scan data).
pub fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    out.extend_from_slice(b"JFIF\0\x01\x01\x00\x00\x00\x00\x00\x00\x00");
    out.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// Writes the fixed assets every case can reference: the float fixtures'
/// real images, and synthetic images with hostile headers.
pub fn prepare_project(root: &Path) {
    let _ = std::fs::create_dir_all(root.join("images"));
    let _ = std::fs::create_dir_all(root.join("plots"));
    let _ = std::fs::create_dir_all(root.join("dir.png"));
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/floats/images");
    if let Ok(entries) = std::fs::read_dir(&fixtures) {
        for entry in entries.flatten() {
            let _ = std::fs::copy(entry.path(), root.join("images").join(entry.file_name()));
        }
    }
    let red = root.join("images/red-72.png");
    for alias in ["plot.png", "pic.png", "plots/a.png", "a.png", "x.png", "fig.png"] {
        let _ = std::fs::copy(&red, root.join(alias));
    }
    let crop = root.join("images/box-crop.pdf");
    for alias in ["fig.pdf", "network.pdf", "c.pdf", "pipeline-diagram.pdf"] {
        let _ = std::fs::copy(&crop, root.join(alias));
    }
    let pixel = zlib_stored(&[0, 255, 0, 0]);
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("huge.png", png(0x7FFF_FFFF, 0x7FFF_FFFF, &pixel)),
        ("wide.png", png(0xFFFF_FFFF, 1, &pixel)),
        ("tall.png", png(1, 1_000_000_000, &pixel)),
        ("zero.png", png(0, 0, &zlib_stored(&[]))),
        ("trunc.png", b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0".to_vec()),
        ("badidat.png", png(4, 4, b"\x78\x9c\xff\xff\xff\xff")),
        ("huge.jpg", jpeg(65535, 65535)),
        ("zero.jpg", jpeg(0, 0)),
        ("trunc.jpg", vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00]),
        (
            "bad.pdf",
            b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1/MediaBox[0 0 1e308 -1e308]>>endobj\n3 0 obj<</Type/Page/Parent 2 0 R/Rotate 45/CropBox[1e30 1e30 -1e30 -1e30]>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n".to_vec(),
        ),
        (
            "loop.pdf",
            b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[2 0 R]/Count 1/Parent 2 0 R>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n".to_vec(),
        ),
        ("empty.pdf", Vec::new()),
    ];
    for (name, bytes) in files {
        let _ = std::fs::write(root.join(name), bytes);
    }
}

// ---------------------------------------------------------------- running

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Ok,
    /// `location`, `message`.
    Panic(String, String),
    /// The stage that did not finish.
    Hang(String),
    /// The process died: stack overflow, abort, or the memory watchdog.
    Crash(String),
}

impl Outcome {
    /// The dedup key: panics by location and message with digits removed
    /// (indices and lengths vary between inputs hitting the same bug).
    pub fn signature(&self) -> String {
        match self {
            Outcome::Ok => "ok".into(),
            Outcome::Panic(loc, msg) => {
                let msg: String = msg
                    .chars()
                    .filter(|c| !c.is_ascii_digit())
                    .take(120)
                    .collect();
                format!("panic {loc} {msg}")
            }
            Outcome::Hang(stage) => format!("hang in {stage}"),
            Outcome::Crash(what) => format!("crash {what}"),
        }
    }
}

pub const STAGES: [&str; 4] = ["setup", "parse", "render", "pdf"];
static STAGE: AtomicU8 = AtomicU8::new(0);
static PANIC_SLOT: OnceLock<Mutex<Option<(String, String)>>> = OnceLock::new();

fn set_stage(stage: u8) {
    STAGE.store(stage, Ordering::SeqCst);
    if std::env::var_os("FLASHTEX_FUZZ_WORKER_PROTOCOL").is_some() {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "STAGE\t{}", STAGES[stage as usize]);
        let _ = out.flush();
    }
}

/// Records the panic location and message instead of printing them.
pub fn install_quiet_panic_hook() {
    let slot = PANIC_SLOT.get_or_init(|| Mutex::new(None));
    panic::set_hook(Box::new(move |info| {
        let loc = info.location().map_or("?".into(), |l| {
            format!("{}:{}:{}", l.file(), l.line(), l.column())
        });
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic>".into()
        };
        if let Ok(mut guard) = slot.lock() {
            guard.get_or_insert((loc, msg.replace(['\n', '\t'], " ")));
        }
    }));
}

/// Stage timings of the last [`render_case`] on this thread, for
/// `--diagnostics`.
pub struct Timings {
    pub parse_ms: f64,
    pub render_ms: f64,
    pub pdf_ms: f64,
    pub status: String,
    pub pages: usize,
    pub pdf: Result<usize, String>,
}

/// One case through the pipeline, exactly as the shipped entry points run
/// it. `root` is the case's project directory ([`prepare_project`]).
pub fn render_case(case: &Case, fonts: &FontSet, root: &Path) -> Timings {
    set_stage(0);
    // The project on disk, as `flashtex build` reads it.
    for name in ["sub.tex"] {
        if !case.documents.iter().any(|(p, _)| p == name) {
            let _ = std::fs::remove_file(root.join(name));
        }
    }
    for (path, text) in &case.documents {
        std::fs::write(root.join(path), text).expect("write case document");
    }
    let entry = ProjectPath::normalize(ENTRY).expect("entry path");
    let documents: Vec<(String, String)> = match ProjectGraph::discover(root, &entry) {
        Ok(graph) => graph.documents().into_iter().map(|d| (d.path, d.text)).collect(),
        Err(_) => case.documents.clone(),
    };
    let sources: Vec<SourceDocument<'_>> = documents
        .iter()
        .map(|(path, text)| SourceDocument { path, text })
        .collect();

    set_stage(1);
    let started = std::time::Instant::now();
    std::hint::black_box(flashtex_compiler::parser::parse_project(&sources, ENTRY));
    let parse_ms = started.elapsed().as_secs_f64() * 1000.0;

    set_stage(2);
    let started = std::time::Instant::now();
    // `flashtex-render --tex`/the worker: a runtime-v1 compile request with
    // every layout capability the IDE may negotiate.
    let mut payload = json::Value::obj();
    payload.set("project_id", json::str_("fuzz"));
    payload.set("revision", json::num(1.0));
    payload.set("entry_path", json::str_(ENTRY));
    payload.set("project_root", json::str_(root.display().to_string()));
    payload.set(
        "documents",
        json::Value::Arr(
            documents
                .iter()
                .map(|(path, text)| {
                    let mut d = json::Value::obj();
                    d.set("path", json::str_(path.clone()));
                    d.set("text", json::str_(text.clone()));
                    d
                })
                .collect(),
        ),
    );
    payload.set(
        "layout_capabilities",
        json::Value::Arr(
            ["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-images", "display-list-v2-device-color"]
                .iter()
                .map(|c| json::str_(*c))
                .collect(),
        ),
    );
    let mut request = json::Value::obj();
    request.set("protocol_version", json::num(flashtex_compiler::protocol::PROTOCOL_VERSION as f64));
    request.set("id", json::str_("fuzz"));
    request.set("type", json::str_("compile"));
    request.set("payload", payload);
    let line = json::write(&request);
    let options = RenderOptions {
        project_root: Some(root.to_path_buf()),
        ..RenderOptions::default()
    };
    let rendered = if line.len() <= protocol::MAX_LINE_BYTES {
        let reply = protocol::handle_line(&line, fonts, &options, None);
        std::hint::black_box((&reply.line, &reply.extra_lines));
        reply.rendered
    } else {
        // Over the worker's line limit: `flashtex build` still renders it.
        Some(flashtex_render_pipeline::render(&sources, ENTRY, 1, "fuzz", fonts, &options))
    };
    let render_ms = started.elapsed().as_secs_f64() * 1000.0;

    set_stage(3);
    let started = std::time::Instant::now();
    let (status, pages, pdf) = match &rendered {
        Some(rendered) => {
            // `flashtex build`: the exact PDF route (`compile::exact_pdf`).
            let pdf = pdf::write_pdf_exact(&rendered.v2, fonts.dirs(), Some(root)).map(|out| out.bytes.len());
            ("rendered".to_string(), rendered.v2.pages.len(), pdf)
        }
        None => ("rejected".to_string(), 0, Err("no render".into())),
    };
    Timings {
        parse_ms,
        render_ms,
        pdf_ms: started.elapsed().as_secs_f64() * 1000.0,
        status,
        pages,
        pdf,
    }
}

/// A long-lived case thread (one `FontSet`, as a worker keeps one across
/// requests) with [`CASE_STACK`], `catch_unwind` per case and a watchdog.
pub struct Runner {
    cases: mpsc::Sender<Case>,
    results: mpsc::Receiver<bool>,
    timeout: Duration,
}

impl Runner {
    pub fn new(timeout: Duration, root: PathBuf) -> Runner {
        let (case_tx, case_rx) = mpsc::channel::<Case>();
        let (result_tx, result_rx) = mpsc::channel();
        let stack = std::env::var("FLASHTEX_FUZZ_STACK_KB")
            .ok()
            .and_then(|kb| kb.parse::<usize>().ok())
            .map_or(CASE_STACK, |kb| kb * 1024);
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(move || {
                prepare_project(&root);
                let mut fonts = FontSet::with_default_dirs(&[]);
                for case in case_rx {
                    let ok = panic::catch_unwind(AssertUnwindSafe(|| {
                        render_case(&case, &fonts, &root);
                    }))
                    .is_ok();
                    if !ok {
                        // A panic may leave the font caches half-filled:
                        // start the next case from a fresh set.
                        fonts = FontSet::with_default_dirs(&[]);
                    }
                    if result_tx.send(ok).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn case thread");
        Runner {
            cases: case_tx,
            results: result_rx,
            timeout,
        }
    }

    /// On `Hang` the thread is still running: the caller must exit.
    pub fn run(&self, case: Case) -> Outcome {
        let slot = PANIC_SLOT.get_or_init(|| Mutex::new(None));
        *slot.lock().unwrap() = None;
        STAGE.store(0, Ordering::SeqCst);
        if self.cases.send(case).is_err() {
            return Outcome::Crash("case thread died".into());
        }
        match self.results.recv_timeout(self.timeout) {
            Ok(true) => Outcome::Ok,
            Ok(false) => {
                let (loc, msg) = slot
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or(("?".into(), "?".into()));
                Outcome::Panic(loc, msg)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Outcome::Crash("case thread died".into()),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Outcome::Hang(STAGES[STAGE.load(Ordering::SeqCst) as usize].into())
            }
        }
    }
}

/// A per-process project directory under `out_dir`.
pub fn scratch_dir(out_dir: &Path) -> PathBuf {
    let dir = out_dir.join(format!("work-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ---------------------------------------------------------------- worker / supervisor

pub struct Config {
    pub rng_seed: u64,
    pub cases: u64,
    pub jobs: usize,
    pub timeout: Duration,
    pub out_dir: PathBuf,
    pub max_rss_mb: u64,
}

/// Worker loop: runs `[start, end)`, one protocol line per event. Exits the
/// process after a hang (the stuck thread cannot be stopped).
pub fn worker(seeds: &[Seed], rng_seed: u64, start: u64, end: u64, timeout: Duration, out_dir: &Path) {
    std::env::set_var("FLASHTEX_FUZZ_WORKER_PROTOCOL", "1");
    install_quiet_panic_hook();
    let scratch = scratch_dir(out_dir);
    let runner = Runner::new(timeout, scratch.clone());
    let stdout = std::io::stdout();
    for index in start..end {
        let case = generate(seeds, rng_seed, index);
        {
            let mut out = stdout.lock();
            let _ = writeln!(out, "START\t{index}");
            let _ = out.flush();
        }
        let outcome = runner.run(case);
        let mut out = stdout.lock();
        match &outcome {
            Outcome::Ok => {
                let _ = writeln!(out, "OK\t{index}");
            }
            Outcome::Panic(loc, msg) => {
                let _ = writeln!(out, "PANIC\t{index}\t{loc}\t{msg}");
            }
            Outcome::Hang(stage) => {
                let _ = writeln!(out, "HANG\t{index}\t{stage}");
                let _ = out.flush();
                let _ = std::fs::remove_dir_all(&scratch);
                std::process::exit(3);
            }
            Outcome::Crash(what) => {
                let _ = writeln!(out, "CRASH\t{index}\t{what}");
                let _ = out.flush();
                std::process::exit(4);
            }
        }
        let _ = out.flush();
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

pub struct Finding {
    pub signature: String,
    pub first_case: u64,
    pub count: u64,
}

pub struct Report {
    pub cases_run: u64,
    pub findings: BTreeMap<String, Finding>,
}

/// Resident set size of `pid` in KiB, through `ps` (no libc dependency).
fn rss_kb(pid: u32) -> Option<u64> {
    let out = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// Runs `config.cases` cases across `config.jobs` worker processes built by
/// `worker_command(start, end)`, restarting after crashes and hangs.
pub fn supervise(
    seeds: &[Seed],
    config: &Config,
    worker_command: &(dyn Fn(u64, u64) -> Command + Sync),
) -> Report {
    let chunk = 250u64;
    let next = Arc::new(Mutex::new(0u64));
    let report = Arc::new(Mutex::new(Report {
        cases_run: 0,
        findings: BTreeMap::new(),
    }));
    let _ = std::fs::create_dir_all(&config.out_dir);
    let progress_every = (config.cases / 20).max(1);
    std::thread::scope(|scope| {
        for _ in 0..config.jobs {
            let next = Arc::clone(&next);
            let report = Arc::clone(&report);
            scope.spawn(move || loop {
                let (mut start, end) = {
                    let mut n = next.lock().unwrap();
                    if *n >= config.cases {
                        return;
                    }
                    let s = *n;
                    *n = (s + chunk).min(config.cases);
                    (s, *n)
                };
                while start < end {
                    let mut child = worker_command(start, end)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .expect("spawn worker");
                    let pid = child.id();
                    let finished = Arc::new(AtomicBool::new(false));
                    let killed = Arc::new(AtomicBool::new(false));
                    let watchdog = {
                        let finished = Arc::clone(&finished);
                        let killed = Arc::clone(&killed);
                        let limit_kb = config.max_rss_mb * 1024;
                        std::thread::spawn(move || {
                            while !finished.load(Ordering::SeqCst) {
                                std::thread::sleep(Duration::from_millis(400));
                                if rss_kb(pid).is_some_and(|kb| kb > limit_kb) {
                                    killed.store(true, Ordering::SeqCst);
                                    let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
                                    return;
                                }
                            }
                        })
                    };
                    let stderr = child.stderr.take().unwrap();
                    let stderr_tail = std::thread::spawn(move || {
                        let mut tail = String::new();
                        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                            if line.contains("overflow")
                                || line.contains("abort")
                                || line.contains("fatal")
                                || line.contains("memory allocation")
                            {
                                tail = line;
                            }
                        }
                        tail
                    });
                    let mut open: Option<u64> = None;
                    let mut stage = "setup".to_string();
                    let mut resume = end;
                    for line in BufReader::new(child.stdout.take().unwrap())
                        .lines()
                        .map_while(Result::ok)
                    {
                        let fields: Vec<&str> = line.splitn(4, '\t').collect();
                        if fields[0] == "STAGE" {
                            stage = fields.get(1).unwrap_or(&"?").to_string();
                            continue;
                        }
                        let Some(index) = fields.get(1).and_then(|s| s.parse::<u64>().ok()) else {
                            continue;
                        };
                        let outcome = match fields[0] {
                            "START" => {
                                open = Some(index);
                                stage = "setup".into();
                                continue;
                            }
                            "OK" => Outcome::Ok,
                            "PANIC" => Outcome::Panic(
                                fields.get(2).unwrap_or(&"?").to_string(),
                                fields.get(3).unwrap_or(&"?").to_string(),
                            ),
                            "HANG" => Outcome::Hang(fields.get(2).unwrap_or(&"?").to_string()),
                            _ => Outcome::Crash(fields.get(2).unwrap_or(&"?").to_string()),
                        };
                        open = None;
                        let run = record(&report, seeds, config, index, &outcome);
                        if run % progress_every == 0 {
                            eprintln!("fuzz: {run}/{} cases", config.cases);
                        }
                        if matches!(outcome, Outcome::Hang(_) | Outcome::Crash(_)) {
                            resume = index + 1;
                        }
                    }
                    let status = child.wait().expect("wait worker");
                    finished.store(true, Ordering::SeqCst);
                    let _ = watchdog.join();
                    let tail = stderr_tail.join().unwrap_or_default();
                    if let Some(index) = open {
                        let what = if killed.load(Ordering::SeqCst) {
                            format!("memory over {} MiB in {stage}", config.max_rss_mb)
                        } else if tail.contains("stack overflow") || tail.contains("overflowed its stack") {
                            format!("stack overflow in {stage}")
                        } else if tail.contains("memory allocation") {
                            format!("allocation failure in {stage}")
                        } else {
                            format!("{status} in {stage} {tail}")
                        };
                        record(&report, seeds, config, index, &Outcome::Crash(what));
                        resume = index + 1;
                    }
                    start = if status.success() && open.is_none() && resume == end {
                        end
                    } else {
                        resume
                    };
                }
            });
        }
    });
    // Directories of workers that died mid-case.
    if let Ok(entries) = std::fs::read_dir(&config.out_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with("work-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Arc::try_unwrap(report).ok().unwrap().into_inner().unwrap()
}

fn write_case(config: &Config, index: u64, case: &Case) -> PathBuf {
    let path = config.out_dir.join(format!("case-{index}.tex"));
    let _ = std::fs::write(&path, case.main());
    if let Some((_, sub)) = case.documents.get(1) {
        let _ = std::fs::write(config.out_dir.join(format!("case-{index}.sub.tex")), sub);
    }
    path
}

/// Records one outcome; returns the number of cases run so far.
fn record(report: &Mutex<Report>, seeds: &[Seed], config: &Config, index: u64, outcome: &Outcome) -> u64 {
    let mut report = report.lock().unwrap();
    report.cases_run += 1;
    let run = report.cases_run;
    if *outcome == Outcome::Ok {
        return run;
    }
    let signature = outcome.signature();
    let entry = report
        .findings
        .entry(signature.clone())
        .or_insert_with(|| Finding {
            signature: signature.clone(),
            first_case: index,
            count: 0,
        });
    entry.count += 1;
    let first = entry.count == 1;
    entry.first_case = entry.first_case.min(index);
    // Hangs and crashes share one signature per stage whatever their cause,
    // so keep a sample of inputs to triage, not only the first.
    let sample = entry.count <= 50 && matches!(outcome, Outcome::Hang(_) | Outcome::Crash(_));
    if first || sample {
        let case = generate(seeds, config.rng_seed, index);
        let path = write_case(config, index, &case);
        if first {
            eprintln!(
                "finding: {signature}\n  case {index} seed {} mutations {:?} -> {}",
                case.seed_name,
                case.mutations,
                path.display()
            );
        }
    }
    run
}

pub fn print_report(report: &Report) {
    println!(
        "fuzz: {} cases, {} unique findings",
        report.cases_run,
        report.findings.len()
    );
    for finding in report.findings.values() {
        println!(
            "  {:>6}x first case {:>6}: {}",
            finding.count, finding.first_case, finding.signature
        );
    }
}

// ---------------------------------------------------------------- minimising

/// Classic ddmin over lines, then characters, keeping `interesting` true.
pub fn minimise(text: &str, interesting: &mut dyn FnMut(&str) -> bool) -> String {
    let mut current: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    current = ddmin(current, interesting);
    let joined: String = current.concat();
    let chars: Vec<String> = joined.chars().map(|c| c.to_string()).collect();
    ddmin(chars, interesting).concat()
}

fn ddmin(mut items: Vec<String>, interesting: &mut dyn FnMut(&str) -> bool) -> Vec<String> {
    let mut granularity = 2usize;
    while items.len() >= 2 {
        let chunk = items.len().div_ceil(granularity);
        let mut reduced = false;
        let mut start = 0;
        while start < items.len() {
            let end = (start + chunk).min(items.len());
            let candidate: Vec<String> = items[..start]
                .iter()
                .chain(&items[end..])
                .cloned()
                .collect();
            if !candidate.is_empty() && interesting(&candidate.concat()) {
                items = candidate;
                reduced = true;
                granularity = (granularity - 1).max(2);
                break;
            }
            start = end;
        }
        if !reduced {
            if granularity >= items.len() {
                break;
            }
            granularity = (granularity * 2).min(items.len());
        }
    }
    items
}
