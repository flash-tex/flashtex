//! The documents the harness measures, and the generators for the large
//! synthetic ones.
//!
//! Nothing large is committed. `synthetic-500kb` and `synthetic-2mb` come out
//! of [`scaling_document`], which reproduces the generator in
//! `crates/compiler/src/bin/scaling_bench.rs` byte for byte, so a number here
//! can be compared with the FT-065 evidence and with `gen500.py`. The two
//! shape-specific documents are generated the same way: a fixed sequence, no
//! clock, no RNG crate, no environment — `--dump-corpus` writes them out if a
//! human or an oracle needs the actual bytes.

use std::path::{Path, PathBuf};

/// One source file of a case.
#[derive(Clone)]
pub struct Doc {
    pub path: String,
    pub text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    RealWorld,
    Synthetic,
}

impl Group {
    pub fn as_str(self) -> &'static str {
        match self {
            Group::RealWorld => "real-world",
            Group::Synthetic => "synthetic",
        }
    }
}

#[derive(Clone)]
pub struct Case {
    pub id: String,
    pub group: Group,
    pub entry: String,
    pub docs: Vec<Doc>,
    /// Directory `\includegraphics` resolves under. `None` for generated
    /// documents, which reference no files.
    pub project_root: Option<PathBuf>,
}

impl Case {
    /// Total source bytes across every document of the project — the size the
    /// targets are stated in.
    pub fn bytes(&self) -> usize {
        self.docs.iter().map(|d| d.text.len()).sum()
    }

    pub fn entry_text(&self) -> &str {
        self.docs.iter().find(|d| d.path == self.entry).map_or("", |d| d.text.as_str())
    }
}

/// SplitMix64 — the generators must produce identical bytes on every host and
/// every run, so nothing here may reach for a seeded-from-entropy RNG.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
}

/// The compiler's `scaling_bench` document, reproduced exactly: a section /
/// prose / inline-math / display-math mix driven by one LCG. Changing a byte
/// of this invalidates every committed baseline for the synthetic cases.
pub fn scaling_document(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 512);
    out.push_str("\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n");
    let mut n = 0usize;
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    while out.len() < target_bytes {
        let seed = rng.next();
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!("\\section{{Section {n}}}\n")),
            1 => out.push_str(&format!("Paragraph {n} of \\proj{{}} with inline $x^{{{n}}} + \\alpha$ maths.\n\n")),
            2 => out.push_str(&format!(
                "Paragraph {n} discusses results in some detail, with enough words to wrap \
                 across a line and exercise the paragraph breaker properly.\n\n"
            )),
            _ => out.push_str(&format!("Displayed: $$\\frac{{a_{{{n}}}}}{{b}}$$\n\n")),
        }
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

/// Dense mathematics and almost no prose: the profile that puts math-layout
/// and the Appendix G boxes on top of the flamegraph instead of the
/// paragraph breaker. Every construct used here is already covered by
/// `fixtures/real-world/math-sheet`, so the document renders without
/// diagnostics — the harness asserts that anyway before it times anything.
pub fn math_heavy_document(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 1024);
    out.push_str(
        "\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage{amssymb}\n\\begin{document}\n",
    );
    let mut n = 0usize;
    let mut rng = Rng(0x243F_6A88_85A3_08D3);
    while out.len() < target_bytes {
        let seed = rng.next();
        let a = n % 9 + 1;
        let b = (n / 3) % 7 + 2;
        match (seed >> 33) % 6 {
            0 => out.push_str(&format!(
                "\\begin{{equation}}\n  \\int_{{0}}^{{{a}}} \\frac{{x^{{{b}}} + \\alpha_{{{n}}}}}{{1 + x^{{2}}}}\\,dx\n   = \\sum_{{k={a}}}^{{\\infty}} \\frac{{(-1)^{{k}}}}{{k^{{{b}}}}}.\n\\end{{equation}}\n\n"
            )),
            1 => out.push_str(&format!(
                "\\begin{{align}}\n  f_{{{n}}}(x) &= \\sum_{{i=1}}^{{{a}}} \\beta_i x^{{i}} \\\\\n  g_{{{n}}}(x) &= \\prod_{{j=1}}^{{{b}}} \\left( 1 - \\frac{{x}}{{\\lambda_j}} \\right)\n\\end{{align}}\n\n"
            )),
            2 => out.push_str(&format!(
                "\\begin{{equation*}}\n  A_{{{n}}} = \\begin{{pmatrix}} a_{{11}} & a_{{12}} \\\\ a_{{21}} & a_{{22}} \\end{{pmatrix}},\\qquad\n  \\det A_{{{n}}} = a_{{11}}a_{{22}} - a_{{12}}a_{{21}}.\n\\end{{equation*}}\n\n"
            )),
            3 => out.push_str(&format!(
                "For every $\\varepsilon > 0$ there is $\\delta_{{{n}}} > 0$ with $\\lvert f(x) - f(y) \\rvert < \\varepsilon$ whenever $\\lvert x - y \\rvert < \\delta_{{{n}}}$, and $\\lim_{{h \\to 0}} \\frac{{f(x+h) - f(x)}}{{h}} = f'(x)$ holds on $\\mathbb{{R}}$.\n\n"
            )),
            4 => out.push_str(&format!(
                "\\begin{{equation}}\n  \\phi_{{{n}}}(t) = \\begin{{cases}}\n    t^{{{a}}} & t \\geq 0, \\\\\n    -\\lvert t \\rvert^{{{b}}} & t < 0.\n  \\end{{cases}}\n\\end{{equation}}\n\n"
            )),
            _ => out.push_str(&format!(
                "\\begin{{gather}}\n  \\binom{{{a}{b}}}{{{a}}} \\leq \\frac{{({a}{b})^{{{a}}}}}{{{a}!}}, \\qquad\n  \\bigl\\lVert T_{{{n}}} \\bigr\\rVert_{{\\mathrm{{op}}}} \\leq \\sqrt{{\\sum_{{k=1}}^{{{b}}} \\sigma_k^{{2}}}}\n\\end{{gather}}\n\n"
        )),
        }
        // One command-free, maths-free line of prose per block. The harness
        // places its paragraph keystroke on a line like this, and a document
        // made only of equations would have nowhere to put one.
        out.push_str(&format!(
            "The statement above is proved in the same way as the previous one and is recorded here for reference in step {n} of the argument.\n\n"
        ));
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

/// Many `tikzpicture` environments: the vector-graphics reader and the
/// picture path, which neither the prose nor the math corpus touches.
/// Constructs are taken from `crates/render-pipeline/fixtures/tikz/*.tex`.
pub fn tikz_heavy_document(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 1024);
    out.push_str("\\documentclass{article}\n\\usepackage{tikz}\n\\begin{document}\n");
    let mut n = 0usize;
    let mut rng = Rng(0xB504_F333_F9DE_6484);
    while out.len() < target_bytes {
        let seed = rng.next();
        let a = (n % 4) as f64 * 0.5 + 1.0;
        let b = (n % 3) as f64 * 0.4 + 0.8;
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\draw (0,0) -- ({a},0) -- ({a},{b}) -- cycle;\n\\draw[thick] (0,{b}) -- ({a},{b});\n\\draw[dashed] (0,0) -- ({a},{b});\n\\end{{tikzpicture}}\n\n"
            )),
            1 => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\draw[step=0.5,gray,very thin] (0,0) grid ({a},{b});\n\\draw[->] (0,0) -- ({a},0);\n\\draw[->] (0,0) -- (0,{b});\n\\end{{tikzpicture}}\n\n"
            )),
            2 => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\draw ({a},{b}) circle ({b});\n\\fill[black!20] (0,0) rectangle ({a},{b});\n\\draw[very thick] (0,0) -- ({a},{b});\n\\end{{tikzpicture}}\n\n"
            )),
            _ => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\node (a{n}) at (0,0) {{Start {n}}};\n\\node (b{n}) at ({a},{b}) {{End {n}}};\n\\draw[->] (a{n}) -- (b{n});\n\\end{{tikzpicture}}\n\n"
            )),
        }
        // A caption-like paragraph after every picture. Without any prose the
        // document would have nowhere a keystroke could land, and a corpus
        // entry with no warm scenario measures half of what it should.
        out.push_str(&format!(
            "Figure {n} shows the construction described above, drawn to the same scale as the preceding one so the two can be compared directly.\n\n"
        ));
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

fn generated(id: &str, text: String) -> Case {
    Case {
        id: id.to_string(),
        group: Group::Synthetic,
        entry: "main.tex".to_string(),
        docs: vec![Doc { path: "main.tex".to_string(), text }],
        project_root: None,
    }
}

/// The synthetic half of the corpus, in a fixed order.
pub fn synthetic_cases() -> Vec<Case> {
    vec![
        generated("synthetic-500kb", scaling_document(500_000)),
        generated("synthetic-2mb", scaling_document(2_000_000)),
        generated("math-heavy", math_heavy_document(150_000)),
        generated("tikz-heavy", tikz_heavy_document(120_000)),
    ]
}

/// The committed fixtures under `fixtures/real-world`, discovered the same
/// way `tools/real-world-corpus/run.py` does it: `main.tex` if present, else
/// the single `.tex`, with every `.tex` below the directory as a project
/// document so `\input` resolves.
pub fn real_world_cases(fixtures: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    let Ok(entries) = std::fs::read_dir(fixtures) else {
        return cases;
    };
    let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for dir in dirs {
        let Some(case) = real_world_case(&dir) else { continue };
        cases.push(case);
    }
    cases
}

fn real_world_case(dir: &Path) -> Option<Case> {
    let id = dir.file_name()?.to_string_lossy().to_string();
    let mut docs = Vec::new();
    collect_tex(dir, dir, &mut docs);
    docs.sort_by(|a: &Doc, b: &Doc| a.path.cmp(&b.path));
    if docs.is_empty() {
        return None;
    }
    let top: Vec<&Doc> = docs.iter().filter(|d| !d.path.contains('/')).collect();
    let entry = if top.iter().any(|d| d.path == "main.tex") {
        "main.tex".to_string()
    } else if top.len() == 1 {
        top[0].path.clone()
    } else {
        // Several top-level .tex files and no main.tex: ambiguous, same rule
        // as the corpus harness.
        return None;
    };
    Some(Case { id, group: Group::RealWorld, entry, docs, project_root: Some(dir.to_path_buf()) })
}

fn collect_tex(root: &Path, dir: &Path, out: &mut Vec<Doc>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            collect_tex(root, &p, out);
        } else if p.extension().is_some_and(|e| e == "tex") {
            if let Ok(text) = std::fs::read_to_string(&p) {
                let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                out.push(Doc { path: rel, text });
            }
        }
    }
}

/// Real-world fixtures then synthetic ones, in a stable order so the table
/// and the baseline line up run to run.
pub fn all_cases(repo_root: &Path) -> Vec<Case> {
    let mut cases = real_world_cases(&repo_root.join("fixtures/real-world"));
    cases.extend(synthetic_cases());
    cases
}
