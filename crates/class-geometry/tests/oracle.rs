//! Compare the model with pdflatex measurements committed in
//! `tests/data/oracle.txt` (regenerate with `oracle/generate.py`; no TeX
//! runs here). Lengths must match to the scaled point; positions from
//! `\pdfsavepos` must match exactly (the task's tolerance is 0.1pt).

use flashtex_class_geometry::*;
use std::collections::BTreeMap;

const DATA: &str = include_str!("data/oracle.txt");
/// Allowed position error in sp (0 = exact). 0.1pt = 6554sp is the task
/// bound; we require exactness and report the worst error.
const POS_TOL: i64 = 0;

#[derive(Default, Debug)]
struct Fixture {
    id: String,
    class: String,
    options: String,
    geometry: Option<String>,
    calls: Vec<String>,
    pagestyle: Option<String>,
    body: String,
    dims: BTreeMap<String, String>,
    cnts: BTreeMap<String, i32>,
    boxes: BTreeMap<String, (Sp, Sp)>,
    /// (mark, page) -> (x from left, y from bottom)
    pos: BTreeMap<(String, i64), (i64, i64)>,
}

fn fixtures() -> Vec<Fixture> {
    let mut out: Vec<Fixture> = Vec::new();
    for line in DATA.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some(id) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push(Fixture {
                id: id.to_string(),
                ..Default::default()
            });
            continue;
        }
        let f = out.last_mut().expect("fixture header");
        let (tag, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.to_string();
        match tag {
            "class" => f.class = rest,
            "options" => f.options = rest,
            "geometry" => f.geometry = if rest == "-" { None } else { Some(rest) },
            "calls" => {
                f.calls = if rest == "-" {
                    vec![]
                } else {
                    rest.split('|').map(String::from).collect()
                }
            }
            "pagestyle" => f.pagestyle = if rest == "-" { None } else { Some(rest) },
            "body" => f.body = rest,
            "dim" => {
                let (k, v) = rest.split_once(' ').unwrap();
                f.dims.insert(k.to_string(), v.to_string());
            }
            "cnt" => {
                let (k, v) = rest.split_once(' ').unwrap();
                f.cnts.insert(k.to_string(), v.parse().unwrap());
            }
            "box" => {
                let p: Vec<&str> = rest.split(' ').collect();
                f.boxes.insert(
                    p[0].to_string(),
                    (Sp::parse(p[1]).unwrap(), Sp::parse(p[2]).unwrap()),
                );
            }
            "pos" => {
                let p: Vec<&str> = rest.split(' ').collect();
                f.pos.insert(
                    (p[0].to_string(), p[1].parse().unwrap()),
                    (p[2].parse().unwrap(), p[3].parse().unwrap()),
                );
            }
            _ => panic!("bad line {line}"),
        }
    }
    out
}

fn setup(f: &Fixture) -> DocumentSetup {
    let mut s = DocumentSetup::new(ClassKind::parse(&f.class).unwrap(), &f.options);
    s.geometry = f.geometry.as_ref().map(|g| GeometryInput {
        package_options: g.clone(),
        calls: f.calls.clone(),
    });
    s.pagestyle = f.pagestyle.as_deref().and_then(PageStyle::parse);
    s
}

fn model_dims(r: &ResolvedDocument) -> BTreeMap<&'static str, String> {
    let p = &r.params;
    let mut m = BTreeMap::new();
    let mut put = |k: &'static str, v: String| {
        m.insert(k, v);
    };
    put("paperwidth", p.paperwidth.to_string());
    put("paperheight", p.paperheight.to_string());
    put("pdfpagewidth", r.frame.pdf_page_width.to_string());
    put("pdfpageheight", r.frame.pdf_page_height.to_string());
    put("textwidth", p.textwidth.to_string());
    put("textheight", p.textheight.to_string());
    put("oddsidemargin", p.oddsidemargin.to_string());
    put("evensidemargin", p.evensidemargin.to_string());
    put("topmargin", p.topmargin.to_string());
    put("headheight", p.headheight.to_string());
    put("headsep", p.headsep.to_string());
    put("footskip", p.footskip.to_string());
    put("topskip", p.topskip.to_string());
    put("baselineskip", Glue::fixed(p.baselineskip).to_string());
    put("parindent", p.parindent.to_string());
    put("parskip", p.parskip.to_string());
    put("marginparwidth", p.marginparwidth.to_string());
    put("marginparsep", p.marginparsep.to_string());
    put("marginparpush", p.marginparpush.to_string());
    put("columnsep", p.columnsep.to_string());
    put("columnseprule", p.columnseprule.to_string());
    put("columnwidth", p.columnwidth(r.flags.twocolumn).to_string());
    put("maxdepth", p.maxdepth.to_string());
    put("footnotesep", p.footnotesep.to_string());
    put("overfullrule", p.overfullrule.to_string());
    put("leftmargini", p.leftmargini.to_string());
    put("labelsep", p.labelsep.to_string());
    put("skipfootins", p.skip_footins.to_string());
    put("em", r.font.em.to_string());
    put("ex", r.font.ex.to_string());
    if let Some(mi) = p.mathindent {
        put("mathindent", Glue::fixed(mi).to_string());
    }
    m
}

struct Checker<'a> {
    f: &'a Fixture,
    failures: Vec<String>,
    checked: usize,
    worst: i64,
}

impl Checker<'_> {
    fn raw(&self, mark: &str, page: i64) -> Option<(i64, i64)> {
        self.f.pos.get(&(mark.to_string(), page)).copied()
    }
    /// (x from left, y from top) in sp.
    fn at(&self, mark: &str, page: i64, paper_h: Sp) -> Option<(Sp, Sp)> {
        self.raw(mark, page).map(|(x, y)| (Sp(x), paper_h - Sp(y)))
    }
    fn page_of(&self, mark: &str) -> Option<i64> {
        self.f.pos.keys().find(|(m, _)| m == mark).map(|(_, p)| *p)
    }
    fn eq(&mut self, what: &str, got: Sp, want: Sp) {
        self.checked += 1;
        let d = (got.0 - want.0).abs();
        self.worst = self.worst.max(d);
        if d > POS_TOL {
            self.failures.push(format!(
                "{}: {what}: pdflatex {} model {} (diff {}sp)",
                self.f.id,
                got,
                want,
                got.0 - want.0
            ));
        }
    }
}

#[test]
fn oracle_lengths_and_positions() {
    let fixtures = fixtures();
    assert!(fixtures.len() >= 60, "only {} fixtures", fixtures.len());
    let mut dim_checked = 0;
    let mut dim_fail = Vec::new();
    let mut pos_checked = 0;
    let mut pos_fail = Vec::new();
    let mut worst = 0;
    let mut fixtures_pass = 0;
    for f in &fixtures {
        let r = resolve(&setup(f));
        let dims = model_dims(&r);
        let before = dim_fail.len() + pos_fail.len();
        for (k, want) in &f.dims {
            let got = dims
                .get(k.as_str())
                .cloned()
                .unwrap_or_else(|| "<missing>".into());
            dim_checked += 1;
            if &got != want {
                dim_fail.push(format!("{}: \\{k}: pdflatex {want} model {got}", f.id));
            }
        }
        for (k, v) in &f.cnts {
            dim_checked += 1;
            let got = if k == "secnumdepth" {
                r.secnumdepth
            } else {
                r.tocdepth
            };
            if got != *v {
                dim_fail.push(format!("{}: {k}: pdflatex {v} model {got}", f.id));
            }
        }

        let ph = r.frame.pdf_page_height;
        let fr = &r.frame;
        let mut c = Checker {
            f,
            failures: Vec::new(),
            checked: 0,
            worst: 0,
        };
        let p = &r.params;
        let base = r.options.size;

        if f.body == "section" {
            let (x1, y1) = c.at("b1", 1, ph).unwrap();
            c.eq("b1.x", x1, fr.text_left(1));
            c.eq("b1.y", y1, fr.first_baseline);
            let (x2, y2) = c.at("b2", 1, ph).unwrap();
            c.eq("b2.x", x2, fr.text_left(1) + p.parindent);
            c.eq("b2.y", y2, y1 + p.baselineskip + p.parskip.natural);
            let mut prev = y2;
            for (mark, body, name) in [
                ("sec", "a1", "section"),
                ("sub", "a2", "subsection"),
                ("ssub", "a3", "subsubsection"),
            ] {
                let h = r.heading(name).unwrap();
                let (_, hy) = c.at(mark, 1, ph).unwrap();
                c.eq(
                    &format!("{mark}.y"),
                    hy,
                    prev + h.baseline_after_body(p, base),
                );
                let (bx, by) = c.at(body, 1, ph).unwrap();
                c.eq(
                    &format!("{body}.x (no indent after heading)"),
                    bx,
                    fr.text_left(1),
                );
                c.eq(&format!("{body}.y"), by, hy + h.body_after_heading(p));
                prev = by;
            }
            let h = r.heading("paragraph").unwrap();
            let (px, py) = c.at("para", 1, ph).unwrap();
            c.eq("para.x", px, fr.text_left(1) + h.indent);
            c.eq("para.y", py, prev + h.baseline_after_body(p, base));
            let (_, p1y) = c.at("p1", 1, ph).unwrap();
            c.eq("p1.y (run-in)", p1y, py);
            if r.flags.twocolumn {
                let (cx, cy) = c.at("col2", 1, ph).expect("col2");
                c.eq(
                    "col2.x (indented)",
                    cx,
                    fr.text_left(1) + fr.columns[1].offset + p.parindent,
                );
                c.eq("col2.y", cy, fr.first_baseline);
            }
            for (mark, page) in [("e1", 2), ("o1", 3)] {
                let (x, y) = c.at(mark, page, ph).unwrap();
                c.eq(&format!("{mark}.x"), x, fr.text_left(page));
                c.eq(&format!("{mark}.y"), y, fr.first_baseline);
            }
            for page in 1..=3 {
                check_page_number(&mut c, &r, page, ph);
            }
        } else {
            let starred = f.body == "chapterstar";
            let ch = r.chapter.as_ref().unwrap();
            let (numht, _) = f.boxes["chapnum"];
            let (titleht, _) = f.boxes["chaptitle"];
            assert!(
                numht <= ch.number_size.metrics(base).1 && titleht <= ch.title_size.metrics(base).1
            );
            let page = c.page_of("chap").unwrap();
            if let Some(t0) = c.page_of("t0") {
                let (_, y) = c.at("t0", t0, ph).unwrap();
                c.eq("t0.y", y, fr.first_baseline);
                let expected_page = if r.options.openright && r.flags.twoside {
                    3
                } else {
                    2
                };
                c.checked += 1;
                if page != expected_page {
                    c.failures.push(format!(
                        "{}: chapter on page {page}, model {expected_page}",
                        f.id
                    ));
                }
            }
            let (cx, cy) = c.at("chap", page, ph).unwrap();
            c.eq("chap.x", cx, fr.text_left(page));
            c.eq(
                "chap.y",
                cy,
                fr.text_top + ch.title_baseline(p, base, starred),
            );
            let (bx, by) = c.at("c1", page, ph).unwrap();
            c.eq("c1.x (no indent)", bx, fr.text_left(page));
            c.eq("c1.y", by, cy + ch.body_after_title(p));
            let h = r.heading("section").unwrap();
            let (_, sy) = c.at("sec", page, ph).unwrap();
            c.eq("sec.y", sy, by + h.baseline_after_body(p, base));
            // Chapter pages use \thispagestyle{plain}: number in the foot.
            let (pgx, pgy) = c.at("pg", page, ph).expect("plain page number");
            let _ = pgx;
            c.eq("chapter page number y (plain foot)", pgy, fr.foot_baseline);
        }
        pos_checked += c.checked;
        worst = worst.max(c.worst);
        pos_fail.extend(c.failures);
        if dim_fail.len() + pos_fail.len() == before {
            fixtures_pass += 1;
        }
    }
    eprintln!(
        "oracle: {fixtures_pass}/{} fixtures pass; lengths {}/{dim_checked}; positions {}/{pos_checked}; worst position error {worst}sp",
        fixtures.len(),
        dim_checked - dim_fail.len(),
        pos_checked - pos_fail.len()
    );
    let all: Vec<String> = dim_fail.into_iter().chain(pos_fail).collect();
    assert!(
        all.is_empty(),
        "{} mismatches:\n{}",
        all.len(),
        all.join("\n")
    );
}

fn check_page_number(c: &mut Checker, r: &ResolvedDocument, page: i64, ph: Sp) {
    let (head, foot) = r.head_foot(page);
    let fr = &r.frame;
    let got = c.at("pg", page, ph);
    let has = |l: &Line| [l.left, l.center, l.right].contains(&Field::PageNumber);
    match (has(&head), has(&foot), got) {
        (true, _, Some((x, y))) => {
            c.eq(&format!("page {page} number y (head)"), y, fr.head_baseline);
            if head.left == Field::PageNumber {
                c.eq(
                    &format!("page {page} number x (head left)"),
                    x,
                    fr.text_left(page),
                );
            }
        }
        (false, true, Some((_, y))) => {
            c.eq(&format!("page {page} number y (foot)"), y, fr.foot_baseline)
        }
        (false, false, None) => c.checked += 1,
        (h, ft, g) => {
            c.checked += 1;
            c.failures.push(format!(
                "{}: page {page}: model head={h} foot={ft}, pdflatex mark {:?}",
                c.f.id, g
            ));
        }
    }
}

#[test]
fn preamble_scanner() {
    let src = r"\documentclass[11pt,a4paper]{report} % comment {x}
\usepackage{amsmath}
\usepackage[margin=1in, includehead]{geometry}
\geometry{top=2cm}
\pagestyle{headings}
\begin{document}\pagestyle{empty}";
    let s = DocumentSetup::from_preamble(src).unwrap();
    assert_eq!(s.class, ClassKind::Report);
    assert_eq!(s.class_options, "11pt,a4paper");
    let g = s.geometry.clone().unwrap();
    assert_eq!(g.package_options, "margin=1in, includehead");
    assert_eq!(g.calls, vec!["top=2cm".to_string()]);
    assert_eq!(s.pagestyle, Some(PageStyle::Headings));
    assert!(DocumentSetup::from_preamble(r"\documentclass{memoir}").is_none());
}
