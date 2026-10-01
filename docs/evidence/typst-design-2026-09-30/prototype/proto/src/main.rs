//! Throwaway Typst embedding prototype for FlashTeX TYPST-DESIGN-A.
//! Measures cold/warm compile, per-page change sets, display-list conversion,
//! memory growth, and construct coverage. Not product code.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::{Abs, Frame, FrameItem, Point, Transform};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::visualize::{Color, Geometry, Paint};
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

// ---------------------------------------------------------------- world

pub struct BenchWorld {
    library: LazyHash<Library>,
    fonts: FontStore,
    main: FileId,
    root: PathBuf,
    sources: HashMap<FileId, Source>,
    files: std::sync::Mutex<HashMap<FileId, Bytes>>,
}

fn fid(path: &str) -> FileId {
    RootedPath::new(VirtualRoot::Project, VirtualPath::new(path).unwrap()).intern()
}

impl BenchWorld {
    fn new(root: &Path, main: &str, system_fonts: bool) -> Self {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        if system_fonts {
            fonts.extend(typst_kit::fonts::system());
        }
        let main_id = fid(main);
        let text = std::fs::read_to_string(root.join(main)).unwrap();
        let mut sources = HashMap::new();
        sources.insert(main_id, Source::new(main_id, text));
        Self {
            library: LazyHash::new(Library::builder().build()),
            fonts,
            main: main_id,
            root: root.to_path_buf(),
            sources,
            files: Default::default(),
        }
    }
    fn main_source_mut(&mut self) -> &mut Source {
        self.sources.get_mut(&self.main).unwrap()
    }
    fn main_source(&self) -> &Source {
        self.sources.get(&self.main).unwrap()
    }
}

impl World for BenchWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if let Some(s) = self.sources.get(&id) {
            return Ok(s.clone());
        }
        let bytes = self.file(id)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.into()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(s) = self.sources.get(&id) {
            return Ok(Bytes::from_string(s.clone()));
        }
        let mut files = self.files.lock().unwrap();
        if let Some(b) = files.get(&id) {
            return Ok(b.clone());
        }
        let p = self.root.join(id.vpath().get_without_slash());
        let data = std::fs::read(&p).map_err(|e| FileError::from_io(e, &p))?;
        let b = Bytes::new(data);
        files.insert(id, b.clone());
        Ok(b)
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 9, 30)
    }
}

impl typst_ide::IdeWorld for BenchWorld {
    fn upcast(&self) -> &dyn World {
        self
    }
}

// ---------------------------------------------------------------- memory

fn phys_footprint() -> u64 {
    unsafe {
        let mut info: libc::rusage_info_v2 = std::mem::zeroed();
        let r = libc::proc_pid_rusage(
            libc::getpid(),
            libc::RUSAGE_INFO_V2,
            &mut info as *mut _ as *mut libc::rusage_info_t,
        );
        if r == 0 { info.ri_phys_footprint } else { 0 }
    }
}

// ---------------------------------------------------------------- generator

const WORDS: &[&str] = &[
    "the",
    "of",
    "and",
    "a",
    "to",
    "in",
    "is",
    "that",
    "for",
    "it",
    "as",
    "was",
    "with",
    "be",
    "by",
    "on",
    "not",
    "this",
    "are",
    "which",
    "from",
    "or",
    "have",
    "an",
    "they",
    "one",
    "you",
    "were",
    "all",
    "we",
    "can",
    "their",
    "has",
    "there",
    "been",
    "if",
    "more",
    "when",
    "will",
    "would",
    "who",
    "so",
    "no",
    "compact",
    "operator",
    "manifold",
    "spectral",
    "sequence",
    "theorem",
    "estimate",
    "boundary",
    "convergence",
    "lattice",
    "invariant",
    "measure",
    "functional",
    "resolvent",
    "categorical",
    "homotopy",
    "typesetting",
    "incremental",
    "paragraph",
    "hyphenation",
    "justification",
];

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn word(&mut self) -> &'static str {
        WORDS[(self.next() % WORDS.len() as u64) as usize]
    }
}

fn para(rng: &mut Rng, words: usize, out: &mut String) {
    for i in 0..words {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(rng.word());
    }
}

/// Writes main.typ, refs.bib, fig.svg for a document of `sections` sections.
/// `chapter_every`: 0 = continuous flow; k = `#pagebreak()` before every k-th section.
fn generate(dir: &Path, sections: usize, chapter_every: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let nbib = (sections / 2).max(8);
    let mut bib = String::new();
    for k in 0..nbib {
        writeln!(
            bib,
            "@article{{ref{k},\n  author = {{Author{k}, A. and Writer, B.}},\n  title = {{On the {} {} of {} {}}},\n  journal = {{Journal of {}}},\n  volume = {{{}}},\n  pages = {{{}--{}}},\n  year = {{{}}}\n}}\n",
            rng.word(), rng.word(), rng.word(), rng.word(), rng.word(),
            k % 40 + 1, k * 3 + 1, k * 3 + 17, 1950 + (k % 75)
        )
        .unwrap();
    }
    std::fs::write(dir.join("refs.bib"), bib).unwrap();
    std::fs::write(
        dir.join("fig.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">
<rect x="5" y="5" width="190" height="90" fill="#eef" stroke="#336" stroke-width="2"/>
<circle cx="60" cy="50" r="30" fill="#c33"/><path d="M110 80 L150 20 L190 80 Z" fill="#3a3"/>
</svg>"##,
    )
    .unwrap();

    let mut s = String::new();
    s.push_str(
        r#"#set document(title: [Bench document])
#set page(paper: "a4", numbering: "1", header: context [_Bench_ #h(1fr) #counter(page).display()])
#set heading(numbering: "1.1")
#set math.equation(numbering: "(1)")
#set par(justify: true)
#set text(size: 11pt)
#outline()
#pagebreak()
"#,
    );
    let marks = [
        (sections / 20).max(1),
        sections / 2,
        sections.saturating_sub(2).max(1),
    ];
    for i in 1..=sections {
        if chapter_every > 0 && i > 1 && (i - 1) % chapter_every == 0 {
            s.push_str("#pagebreak()\n");
        }
        writeln!(s, "= Section {i} on {} {}\n", rng.word(), rng.word()).unwrap();
        // paragraph 1 with citation, footnote, inline math
        if marks.contains(&i) {
            let tag = if i == marks[0] {
                "EDITSTART"
            } else if i == marks[1] {
                "EDITMID"
            } else {
                "EDITEND"
            };
            write!(s, "{tag} ").unwrap();
        }
        para(&mut rng, 70, &mut s);
        write!(s, " @ref{} and $x_{i}^2 + y^2 = z^2$ ", i % nbib).unwrap();
        para(&mut rng, 40, &mut s);
        write!(s, "#footnote[A note on {} {}.] ", rng.word(), rng.word()).unwrap();
        para(&mut rng, 30, &mut s);
        s.push_str(".\n\n");
        // display equation
        writeln!(
            s,
            "$ integral_0^oo e^(-{i} t^2) dif t = sqrt(pi) / (2 sqrt({i})), quad sum_(k=1)^n k^{i} <= n^({i}+1) $ <eq{i}>\n"
        )
        .unwrap();
        // paragraph 2 with ref
        write!(s, "As shown in @eq{i}, ").unwrap();
        para(&mut rng, 90, &mut s);
        s.push_str(".\n\n");
        if i % 3 == 0 {
            writeln!(
                s,
                "#figure(image(\"fig.svg\", width: 45%), caption: [Figure for section {i}: {} {}.]) <fig{i}>\n",
                rng.word(),
                rng.word()
            )
            .unwrap();
        }
        if i % 5 == 0 {
            s.push_str("#figure(table(columns: 4, stroke: 0.5pt, [*A*], [*B*], [*C*], [*D*]");
            for r in 0..4 {
                write!(
                    s,
                    ", [{r}], [{}], [{}], [$alpha_{r}$]",
                    rng.word(),
                    rng.word()
                )
                .unwrap();
            }
            writeln!(s, "), caption: [Table {i}.])\n").unwrap();
        }
        // paragraph 3
        para(&mut rng, 80, &mut s);
        if i > 3 && i % 3 == 0 {
            write!(s, " (see @fig{i})").unwrap();
        }
        s.push_str(".\n\n");
    }
    s.push_str("#bibliography(\"refs.bib\")\n");
    std::fs::write(dir.join("main.typ"), s).unwrap();
}

// ---------------------------------------------------------------- display list

/// A display-list-v3-like encoding of one page: enough to measure conversion
/// cost and size. Glyph = 15 bytes (u16 font, u16 gid, i32 x, i32 y, u16 col)
/// like v3's GLYPH item plus opcode.
#[derive(Default)]
struct DlStats {
    glyphs: usize,
    text_runs: usize,
    shapes: usize,
    images: usize,
    links: usize,
    groups: usize,
    transformed_groups: usize,
    clips: usize,
    tags: usize,
    non_solid_paint: usize,
    alpha_paint: usize,
    text_stroke: usize,
    dashes: usize,
    img_raster: usize,
    img_svg: usize,
    img_pdf: usize,
    color_spaces: HashMap<&'static str, usize>,
    paint_kinds: HashMap<&'static str, usize>,
    link_kinds: HashMap<&'static str, usize>,
    geom: HashMap<&'static str, usize>,
    fonts: HashMap<String, usize>,
    variable_font_runs: usize,
    color_glyph_runs: usize,
    frame_kinds_hard: usize,
    skewed_text: usize,
}

const SP_PER_PT: f64 = 65536.0;

fn to_sp(a: Abs) -> i32 {
    (a.to_pt() * SP_PER_PT).round() as i32
}

fn paint_note(st: &mut DlStats, p: &Paint) {
    match p {
        Paint::Solid(c) => {
            *st.paint_kinds.entry("solid").or_default() += 1;
            let space = match c {
                Color::Process(pc) => match pc.space() {
                    typst::visualize::ProcessColorSpace::Oklab => "oklab",
                    typst::visualize::ProcessColorSpace::Oklch => "oklch",
                    typst::visualize::ProcessColorSpace::Srgb => "srgb",
                    typst::visualize::ProcessColorSpace::D65Gray => "luma",
                    typst::visualize::ProcessColorSpace::LinearRgb => "linear-rgb",
                    typst::visualize::ProcessColorSpace::Hsl => "hsl",
                    typst::visualize::ProcessColorSpace::Hsv => "hsv",
                    typst::visualize::ProcessColorSpace::Cmyk => "cmyk",
                },
                Color::Spot(_) => "spot",
            };
            *st.color_spaces.entry(space).or_default() += 1;
            if c.alpha().map(|a| a < 1.0).unwrap_or(false) {
                st.alpha_paint += 1;
            }
        }
        Paint::Gradient(g) => {
            st.non_solid_paint += 1;
            let k = match g {
                typst::visualize::Gradient::Linear(_) => "gradient-linear",
                typst::visualize::Gradient::Radial(_) => "gradient-radial",
                typst::visualize::Gradient::Conic(_) => "gradient-conic",
            };
            *st.paint_kinds.entry(k).or_default() += 1;
        }
        Paint::Tiling(_) => {
            st.non_solid_paint += 1;
            *st.paint_kinds.entry("tiling").or_default() += 1;
        }
    }
}

/// Walks a frame, appending v3-like records to `out`. Returns nothing; stats in `st`.
fn walk(
    frame: &Frame,
    ts: Transform,
    out: &mut Vec<u8>,
    st: &mut DlStats,
    fonts: &mut HashMap<Font, u16>,
) {
    if frame.kind().is_hard() {
        st.frame_kinds_hard += 1;
    }
    for (pos, item) in frame.items() {
        let ts_here = ts.pre_concat(Transform::translate(pos.x, pos.y));
        match item {
            FrameItem::Group(g) => {
                st.groups += 1;
                if !g.transform.is_identity() {
                    st.transformed_groups += 1;
                }
                if g.clip.is_some() {
                    st.clips += 1;
                    out.push(0x07); // SAVE + CLIP
                }
                walk(&g.frame, ts_here.pre_concat(g.transform), out, st, fonts);
                if g.clip.is_some() {
                    out.push(0x08);
                }
            }
            FrameItem::Text(t) => {
                st.text_runs += 1;
                paint_note(st, &t.fill);
                if let Some(s) = &t.stroke {
                    st.text_stroke += 1;
                    paint_note(st, &s.paint);
                }
                let font = t.font.font().clone();
                let n = fonts.len() as u16;
                let fid = *fonts.entry(font.clone()).or_insert(n);
                *st.fonts.entry(font.info().family.clone()).or_default() += 1;
                if !t.font.variations().0.is_empty() {
                    st.variable_font_runs += 1;
                }
                let ttf = t.font.ttf();
                if ttf.tables().colr.is_some()
                    || ttf.tables().sbix.is_some()
                    || ttf.tables().cbdt.is_some()
                    || ttf.tables().svg.is_some()
                {
                    st.color_glyph_runs += 1;
                }
                if ts_here.kx != typst::layout::Ratio::zero()
                    || ts_here.ky != typst::layout::Ratio::zero()
                {
                    st.skewed_text += 1;
                }
                // Matrix op per run (size), then glyphs.
                out.push(0x0B);
                out.extend_from_slice(&(t.size.to_pt() as f32).to_le_bytes());
                let mut x = Abs::zero();
                for g in &t.glyphs {
                    let gx = x + g.x_offset.at(t.size);
                    let gy = -g.y_offset.at(t.size);
                    let p = Point::new(gx, gy).transform(ts_here);
                    out.push(0x01);
                    out.extend_from_slice(&fid.to_le_bytes());
                    out.extend_from_slice(&g.id.to_le_bytes());
                    out.extend_from_slice(&to_sp(p.x).to_le_bytes());
                    out.extend_from_slice(&to_sp(p.y).to_le_bytes());
                    out.extend_from_slice(&g.span.1.to_le_bytes());
                    x += g.x_advance.at(t.size);
                    st.glyphs += 1;
                }
            }
            FrameItem::Shape(s, _span) => {
                st.shapes += 1;
                let k = match &s.geometry {
                    Geometry::Line(_) => "line",
                    Geometry::Rect(_) => "rect",
                    Geometry::Curve(_) => "curve",
                };
                *st.geom.entry(k).or_default() += 1;
                if let Some(f) = &s.fill {
                    paint_note(st, f);
                }
                if let Some(stroke) = &s.stroke {
                    paint_note(st, &stroke.paint);
                    if stroke.dash.is_some() {
                        st.dashes += 1;
                    }
                }
                out.push(0x03);
                out.extend_from_slice(&[0u8; 4]);
                // path segments: approximate cost
                if let Geometry::Curve(c) = &s.geometry {
                    for _ in c.0.iter() {
                        out.extend_from_slice(&[0u8; 17]);
                    }
                } else {
                    out.extend_from_slice(&[0u8; 34]);
                }
            }
            FrameItem::Image(img, _size, _span) => {
                st.images += 1;
                match img.kind() {
                    typst::visualize::ImageKind::Raster(_) => st.img_raster += 1,
                    typst::visualize::ImageKind::Svg(_) => st.img_svg += 1,
                    typst::visualize::ImageKind::Pdf(_) => st.img_pdf += 1,
                }
                out.push(0x05);
                out.extend_from_slice(&[0u8; 8]);
            }
            FrameItem::Link(dest, _size) => {
                st.links += 1;
                let k = match dest {
                    typst::model::Destination::Url(_) => "url",
                    typst::model::Destination::Position(_) => "position",
                    typst::model::Destination::Location(_) => "location",
                };
                *st.link_kinds.entry(k).or_default() += 1;
            }
            FrameItem::Tag(_) => {
                st.tags += 1;
            }
        }
    }
}

fn page_dl(doc: &PagedDocument, i: usize, st: &mut DlStats) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 * 1024);
    let mut fonts = HashMap::new();
    walk(
        &doc.pages()[i].frame,
        Transform::identity(),
        &mut out,
        st,
        &mut fonts,
    );
    out
}

fn page_hashes(doc: &PagedDocument) -> Vec<u128> {
    doc.pages()
        .iter()
        .map(|p| typst::utils::hash128(&p.frame))
        .collect()
}

/// Simulated typing just before the marker at `off`: every state is new
/// (a word grows, a space every 7th key), so comemo never sees a repeat.
/// `backspace`: delete a character of the original prose before the marker.
fn type_char(world: &mut BenchWorld, off: usize, k: usize, backspace: bool) {
    if backspace {
        // delete the first character of the prose after "MARKER " (ASCII);
        // the marker starts a paragraph, so this eats its first words.
        let text = world.main_source().text();
        let start = off + text[off..].find(' ').unwrap() + 1;
        world.main_source_mut().edit(start..start + 1, "");
        return;
    }
    let c = if k % 7 == 6 {
        " ".to_string()
    } else {
        char::from(b'a' + ((k * 7 + 3) % 26) as u8).to_string()
    };
    world.main_source_mut().edit(off..off, &c);
}

// ---------------------------------------------------------------- stats

fn pct(v: &mut Vec<f64>, p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((p / 100.0) * (v.len() as f64 - 1.0)).round() as usize;
    v[idx]
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn compile(world: &BenchWorld) -> PagedDocument {
    let r = typst::compile::<PagedDocument>(world);
    match r.output {
        Ok(d) => d,
        Err(e) => {
            for d in e.iter().take(5) {
                eprintln!("error: {:?}", d.message);
            }
            panic!("compile failed");
        }
    }
}

/// `typst::compile`'s loop (typst/src/lib.rs `compile_impl`), but the first
/// iteration is seeded with the previous compile's introspector instead of
/// the empty one. Returns the document and the number of layout iterations.
fn compile_seeded(world: &BenchWorld, prev: Option<&PagedDocument>) -> (PagedDocument, usize) {
    use comemo::Track;
    use typst::engine::{Engine, Route, Sink, Traced};
    use typst::foundations::{Output, StyleChain, Target, TargetElem};
    use typst::introspection::{EmptyIntrospector, Introspector};
    let world_dyn: &dyn World = world;
    let world_t = world_dyn.track();
    let traced = Traced::default();
    let mut sink = Sink::new();
    let library = world.library();
    let base = StyleChain::new(&library.styles);
    let target = TargetElem::target.set(Target::Paged).wrap();
    let styles = base.chain(&target);
    let main = world.source(world.main()).unwrap();
    let content = typst_eval::eval(
        world_t,
        library,
        traced.track(),
        sink.track_mut(),
        Route::default().track(),
        &main,
    )
    .unwrap()
    .content();
    let empty = EmptyIntrospector;
    let mut history: Vec<PagedDocument> = vec![];
    let mut iters = 0;
    loop {
        iters += 1;
        let introspector: &dyn Introspector = match (history.last(), prev) {
            (Some(d), _) => <PagedDocument as Output>::introspector(d),
            (None, Some(p)) => <PagedDocument as Output>::introspector(p),
            (None, None) => &empty,
        };
        let constraint = comemo::Constraint::new();
        let mut subsink = Sink::new();
        let mut engine = Engine {
            library,
            world: world_t,
            introspector: typst::utils::Protected::new(introspector.track_with(&constraint)),
            traced: traced.track(),
            sink: subsink.track_mut(),
            route: Route::default(),
        };
        let doc = PagedDocument::create(&mut engine, &content, styles).unwrap();
        if constraint.validate(<PagedDocument as Output>::introspector(&doc)) || iters >= 5 {
            return (doc, iters);
        }
        history.push(doc);
    }
}

/// Seeded vs standard: latency and, with `validate`, whether every page equals
/// the standard compile's page.
fn mode_seeded(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let n: usize = args[1].parse().unwrap();
    let validate = args.get(2).map(|s| s == "validate").unwrap_or(false);
    let label = args.get(3).cloned().unwrap_or_default();
    let mut world = BenchWorld::new(&dir, "main.typ", false);
    let (mut prev, it0) = compile_seeded(&world, None);
    println!(
        "{{\"label\":\"{label}\",\"cold_iters\":{it0},\"pages\":{}}}",
        prev.pages().len()
    );
    for (tag, backspace) in [
        ("EDITSTART", false),
        ("EDITMID", false),
        ("EDITEND", false),
        ("EDITMID", true),
    ] {
        let mut lat = vec![];
        let mut iters = vec![];
        let mut mismatches = 0;
        for k in 0..n {
            let off = world.main_source().text().find(tag).unwrap();
            let t0 = Instant::now();
            type_char(&mut world, off, k, backspace);
            let (doc, it) = compile_seeded(&world, Some(&prev));
            lat.push(ms(t0));
            iters.push(it as f64);
            if validate {
                let std_doc = compile(&world);
                if page_hashes(&std_doc) != page_hashes(&doc) {
                    mismatches += 1;
                }
            }
            prev = doc;
            comemo::evict(10);
        }
        let mut l2: Vec<f64> = lat.iter().skip(2).copied().collect();
        let mut i2: Vec<f64> = iters.iter().skip(2).copied().collect();
        println!(
            "{{\"label\":\"{label}\",\"loc\":\"{tag}{}\",\"validate\":{validate},\"p50\":{:.2},\"p95\":{:.2},\"max\":{:.2},\"iters_p50\":{},\"iters_max\":{},\"mismatches\":{mismatches},\"fp_mb\":{:.1}}}",
            if backspace { "-backspace" } else { "" },
            pct(&mut l2, 50.0),
            pct(&mut l2, 95.0),
            pct(&mut l2, 100.0),
            pct(&mut i2, 50.0),
            pct(&mut i2, 100.0),
            phys_footprint() as f64 / 1e6
        );
    }
}

/// Source-mapping costs: cursor -> page (typst-ide jump_from_cursor) and
/// click -> source (jump_from_click) on the edited-marker paragraph.
fn mode_jumps(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let world = BenchWorld::new(&dir, "main.typ", false);
    let doc = compile(&world);
    for tag in ["EDITSTART", "EDITMID", "EDITEND"] {
        let off = world.main_source().text().find(tag).unwrap() + 3;
        let mut tc = vec![];
        let mut pos = vec![];
        for _ in 0..20 {
            let t = Instant::now();
            pos = typst_ide::jump_from_cursor(&doc, world.main_source(), off);
            tc.push(ms(t));
        }
        let p = pos.first().cloned().unwrap();
        let mut tk = vec![];
        let mut jump = None;
        for _ in 0..20 {
            let t = Instant::now();
            jump = typst_ide::jump_from_click(&world, &doc, &p);
            tk.push(ms(t));
        }
        let back = match jump {
            Some(typst_ide::Jump::File(_, o)) => o as i64,
            _ => -1,
        };
        println!(
            "{{\"loc\":\"{tag}\",\"page\":{},\"cursor_to_page_p50_ms\":{:.3},\"click_to_source_p50_ms\":{:.3},\"cursor_offset\":{off},\"click_back_offset\":{back}}}",
            p.page,
            pct(&mut tc, 50.0),
            pct(&mut tk, 50.0)
        );
    }
    // Span statistics per page: distinct spans referenced by glyphs.
    let mut spans = std::collections::HashSet::new();
    fn rec(f: &Frame, s: &mut std::collections::HashSet<u128>, glyphs: &mut usize) {
        for (_, it) in f.items() {
            match it {
                FrameItem::Group(g) => rec(&g.frame, s, glyphs),
                FrameItem::Text(t) => {
                    for g in &t.glyphs {
                        *glyphs += 1;
                        s.insert(typst::utils::hash128(&g.span.0));
                    }
                }
                FrameItem::Shape(_, sp) | FrameItem::Image(_, _, sp) => {
                    s.insert(typst::utils::hash128(sp));
                }
                _ => {}
            }
        }
    }
    let mut glyphs = 0;
    for p in doc.pages() {
        rec(&p.frame, &mut spans, &mut glyphs);
    }
    // Time resolving every distinct glyph span to a line (the v3 re-declaration cost).
    let mut all = vec![];
    fn rec2(f: &Frame, out: &mut Vec<typst::syntax::Span>) {
        for (_, it) in f.items() {
            match it {
                FrameItem::Group(g) => rec2(&g.frame, out),
                FrameItem::Text(t) => {
                    for g in &t.glyphs {
                        out.push(g.span.0);
                    }
                }
                _ => {}
            }
        }
    }
    for p in doc.pages() {
        rec2(&p.frame, &mut all);
    }
    all.sort_by_key(|s| typst::utils::hash128(s));
    all.dedup();
    let t = Instant::now();
    use typst::WorldExt;
    let mut lines = 0usize;
    for s in &all {
        if let Some(r) = world.range(*s) {
            lines += world
                .main_source()
                .lines()
                .byte_to_line(r.start)
                .unwrap_or(0)
                & 1;
        }
    }
    println!(
        "{{\"pages\":{},\"glyphs\":{glyphs},\"distinct_spans\":{},\"distinct_glyph_spans\":{},\"resolve_all_glyph_spans_to_lines_ms\":{:.2},\"chk\":{lines}}}",
        doc.pages().len(),
        spans.len(),
        all.len(),
        ms(t)
    );
}

// ---------------------------------------------------------------- modes

fn mode_gen(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let target: usize = args[1].parse().unwrap();
    let chapter_every: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(0);
    let mut sections = target;
    for _ in 0..3 {
        generate(&dir, sections, chapter_every);
        let w = BenchWorld::new(&dir, "main.typ", false);
        let d = compile(&w);
        let pages = d.pages().len();
        eprintln!("sections={sections} pages={pages}");
        if pages == target {
            break;
        }
        let ns =
            ((sections as f64) * (target as f64 - 2.0) / (pages as f64 - 2.0)).round() as usize;
        if ns == sections {
            break;
        }
        sections = ns.max(1);
        comemo::evict(0);
    }
}

/// Edit benchmark: cold compile, then alternating insert/delete of one char
/// before a marker, `n` times per location.
fn mode_bench(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let n: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(30);
    let evict_every: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(0);
    let label = args.get(3).cloned().unwrap_or_default();

    let t = Instant::now();
    let mut world = BenchWorld::new(&dir, "main.typ", false);
    let world_ms = ms(t);
    let fp0 = phys_footprint();

    let t = Instant::now();
    let doc = compile(&world);
    let cold_ms = ms(t);
    let t = Instant::now();
    let mut hashes = page_hashes(&doc);
    let hash_ms = ms(t);
    let t = Instant::now();
    let mut st = DlStats::default();
    let mut dl_bytes = 0;
    for i in 0..doc.pages().len() {
        dl_bytes += page_dl(&doc, i, &mut st).len();
    }
    let dl_all_ms = ms(t);
    let t = Instant::now();
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    let pdf_ms = ms(t);
    let t = Instant::now();
    let _pix = typst_render::render(
        &doc.pages()[0],
        &typst_render::RenderOptions {
            pixel_per_pt: 2.0.into(),
            ..Default::default()
        },
    );
    let render_ms = ms(t);
    let fp_cold = phys_footprint();
    // Second compile with no change (memo hit).
    let t = Instant::now();
    let _ = compile(&world);
    let nochange_ms = ms(t);

    let pages = doc.pages().len();
    println!(
        "{{\"label\":\"{label}\",\"pages\":{pages},\"world_ms\":{world_ms:.1},\"cold_ms\":{cold_ms:.1},\"nochange_ms\":{nochange_ms:.2},\"hash_all_ms\":{hash_ms:.2},\"dl_all_ms\":{dl_all_ms:.2},\"dl_bytes\":{dl_bytes},\"glyphs\":{},\"pdf_ms\":{pdf_ms:.1},\"pdf_bytes\":{},\"render_p1_2x_ms\":{render_ms:.1},\"fp_world_mb\":{:.1},\"fp_cold_mb\":{:.1}}}",
        st.glyphs,
        pdf.len(),
        fp0 as f64 / 1e6,
        fp_cold as f64 / 1e6
    );
    drop(doc);

    for (tag, backspace) in [
        ("EDITSTART", false),
        ("EDITMID", false),
        ("EDITEND", false),
        ("EDITMID", true),
    ] {
        let mut lat = vec![];
        let mut edited_page_lat = vec![];
        let mut changed = vec![];
        let mut hashv = vec![];
        let mut first_changed_v = vec![];
        let mut last_changed_v = vec![];
        let mut edited_page_idx = 0;
        for k in 0..n {
            let off = world.main_source().text().find(tag).unwrap();
            let t0 = Instant::now();
            type_char(&mut world, off, k, backspace);
            let t_edit = ms(t0);
            let t1 = Instant::now();
            let doc = compile(&world);
            let t_comp = ms(t1);
            let t2 = Instant::now();
            let nh = page_hashes(&doc);
            let t_hash = ms(t2);
            let ch: Vec<usize> = (0..nh.len())
                .filter(|&i| hashes.get(i) != Some(&nh[i]))
                .collect();
            // find the edited page via the cursor->document jump (typst-ide)
            let off = world.main_source().text().find(tag).unwrap();
            let pos = typst_ide::jump_from_cursor(&doc, world.main_source(), off + 2);
            if let Some(p) = pos.first() {
                edited_page_idx = p.page.get() - 1;
            }
            let t3 = Instant::now();
            let mut st = DlStats::default();
            let _ = page_dl(&doc, edited_page_idx, &mut st);
            let t_dl = ms(t3);
            lat.push(t_edit + t_comp);
            edited_page_lat.push(t_edit + t_comp + t_hash + t_dl);
            hashv.push(t_hash);
            changed.push(ch.len() as f64);
            first_changed_v.push(ch.first().copied().unwrap_or(usize::MAX) as f64);
            last_changed_v.push(ch.last().copied().unwrap_or(0) as f64);
            hashes = nh;
            drop(doc);
            if evict_every > 0 {
                comemo::evict(evict_every);
            }
        }
        // skip the first 2 edits of each location as warmup
        let mut l2: Vec<f64> = lat.iter().skip(2).copied().collect();
        let mut e2: Vec<f64> = edited_page_lat.iter().skip(2).copied().collect();
        let mut c2: Vec<f64> = changed.iter().skip(2).copied().collect();
        let mut h2: Vec<f64> = hashv.iter().skip(2).copied().collect();
        println!(
            "{{\"label\":\"{label}\",\"loc\":\"{tag}{}\",\"edited_page\":{},\"n\":{},\"compile_p50\":{:.2},\"compile_p95\":{:.2},\"compile_max\":{:.2},\"edited_page_p50\":{:.2},\"edited_page_p95\":{:.2},\"hash_p50\":{:.2},\"changed_pages_p50\":{},\"changed_pages_max\":{},\"first_changed_min\":{},\"fp_mb\":{:.1}}}",
            if backspace { "-backspace" } else { "" },
            edited_page_idx + 1,
            l2.len(),
            pct(&mut l2, 50.0),
            pct(&mut l2, 95.0),
            pct(&mut l2, 100.0),
            pct(&mut e2, 50.0),
            pct(&mut e2, 95.0),
            pct(&mut h2, 50.0),
            pct(&mut c2, 50.0),
            pct(&mut c2, 100.0),
            first_changed_v
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min),
            phys_footprint() as f64 / 1e6
        );
    }
}

/// Memory: `n` edits cycling over the 3 locations; report footprint every 100.
fn mode_mem(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let n: usize = args[1].parse().unwrap();
    let evict_every: usize = args[2].parse().unwrap();
    let seeded = args.get(3).map(|s| s == "seeded").unwrap_or(false);
    let mut world = BenchWorld::new(&dir, "main.typ", false);
    let (mut prev, _) = compile_seeded(&world, None);
    let tags = ["EDITSTART", "EDITMID", "EDITEND"];
    let mut lat = vec![];
    let mut ev: Vec<f64> = vec![];
    let fp_start = phys_footprint();
    println!("{{\"edit\":0,\"fp_mb\":{:.1}}}", fp_start as f64 / 1e6);
    for k in 0..n {
        let tag = tags[(k / 2) % 3];
        let off = world.main_source().text().find(tag).unwrap();
        let t0 = Instant::now();
        type_char(&mut world, off, k, false);
        let doc = if seeded {
            compile_seeded(&world, Some(&prev)).0
        } else {
            compile(&world)
        };
        prev = doc;
        let t_after_compile = ms(t0);
        let te = Instant::now();
        if evict_every > 0 {
            comemo::evict(evict_every);
        }
        ev.push(ms(te));
        lat.push(t_after_compile);
        if (k + 1) % 100 == 0 {
            let mut w: Vec<f64> = lat[lat.len() - 100..].to_vec();
            let mut e: Vec<f64> = ev[ev.len() - 100..].to_vec();
            println!(
                "{{\"edit\":{},\"fp_mb\":{:.1},\"lat_p50\":{:.2},\"lat_p95\":{:.2},\"evict_p50\":{:.2},\"evict_p95\":{:.2}}}",
                k + 1,
                phys_footprint() as f64 / 1e6,
                pct(&mut w, 50.0),
                pct(&mut w, 95.0),
                pct(&mut e, 50.0),
                pct(&mut e, 95.0)
            );
        }
    }
    let t = Instant::now();
    comemo::evict(0);
    let evict_ms = ms(t);
    println!(
        "{{\"after_evict0_fp_mb\":{:.1},\"evict0_ms\":{:.2}}}",
        phys_footprint() as f64 / 1e6,
        evict_ms
    );
}

/// Timing breakdown of one warm edit via typst-timing.
fn mode_trace(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let tag = args.get(1).cloned().unwrap_or("EDITMID".into());
    let out = PathBuf::from(&args[2]);
    let seeded = args.get(3).map(|s| s == "seeded").unwrap_or(false);
    let mut world = BenchWorld::new(&dir, "main.typ", false);
    let (mut prev, _) = compile_seeded(&world, None);
    for k in 0..4 {
        let off = world.main_source().text().find(&tag).unwrap();
        type_char(&mut world, off, k, false);
        prev = if seeded {
            compile_seeded(&world, Some(&prev)).0
        } else {
            compile(&world)
        };
    }
    let off = world.main_source().text().find(&tag).unwrap();
    type_char(&mut world, off, 99, false);
    typst_timing::clear();
    typst_timing::enable();
    let _ = if seeded {
        compile_seeded(&world, Some(&prev)).0
    } else {
        compile(&world)
    };
    typst_timing::disable();
    let f = std::fs::File::create(&out).unwrap();
    typst_timing::export_json(f, |_| ("?".into(), 0)).unwrap();
}

/// Construct census over Typst's test suite (`--- name ---` separated tests).
fn mode_census(args: &[String]) {
    let root = PathBuf::from(&args[0]); // typst-src/tests
    let suite = root.join("suite");
    let mut files = vec![];
    fn rec(p: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(p).unwrap() {
            let e = e.unwrap().path();
            if e.is_dir() {
                rec(&e, out)
            } else if e.extension().map(|x| x == "typ").unwrap_or(false) {
                out.push(e)
            }
        }
    }
    rec(&suite, &mut files);
    files.sort();
    let mut world = BenchWorld::new(&root, "suite/layout/flow/flow.typ", true);
    world
        .fonts
        .extend(typst_kit::fonts::scan(&root.join("assets/fonts")));
    let mut total = DlStats::default();
    let (mut ok, mut failed, mut tests) = (0, 0, 0);
    let mut pages = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let rel = f.strip_prefix(&root).unwrap().to_str().unwrap().to_string();
        let mut chunks: Vec<String> = vec![];
        let mut cur = String::new();
        for line in text.lines() {
            if line.starts_with("--- ") {
                if !cur.trim().is_empty() {
                    chunks.push(std::mem::take(&mut cur));
                }
                cur.clear();
                continue;
            }
            cur.push_str(line);
            cur.push('\n');
        }
        if !cur.trim().is_empty() {
            chunks.push(cur);
        }
        for c in chunks {
            tests += 1;
            let id = fid(&rel);
            world.main = id;
            world.sources.insert(id, Source::new(id, c));
            let r = typst::compile::<PagedDocument>(&world);
            match r.output {
                Ok(doc) => {
                    ok += 1;
                    for i in 0..doc.pages().len() {
                        pages += 1;
                        let _ = page_dl(&doc, i, &mut total);
                    }
                }
                Err(_) => failed += 1,
            }
        }
        comemo::evict(0);
    }
    let mut fonts: Vec<_> = total.fonts.iter().collect();
    fonts.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "{{\"tests\":{tests},\"ok\":{ok},\"failed\":{failed},\"pages\":{pages},\"glyphs\":{},\"text_runs\":{},\"text_stroke\":{},\"shapes\":{},\"geom\":{:?},\"dashes\":{},\"images\":{},\"img_raster\":{},\"img_svg\":{},\"img_pdf\":{},\"links\":{},\"link_kinds\":{:?},\"groups\":{},\"transformed_groups\":{},\"clips\":{},\"hard_frames\":{},\"paint_kinds\":{:?},\"color_spaces\":{:?},\"alpha_paints\":{},\"variable_font_runs\":{},\"color_glyph_runs\":{},\"skewed_text_runs\":{},\"top_fonts\":{:?}}}",
        total.glyphs,
        total.text_runs,
        total.text_stroke,
        total.shapes,
        total.geom,
        total.dashes,
        total.images,
        total.img_raster,
        total.img_svg,
        total.img_pdf,
        total.links,
        total.link_kinds,
        total.groups,
        total.transformed_groups,
        total.clips,
        total.frame_kinds_hard,
        total.paint_kinds,
        total.color_spaces,
        total.alpha_paint,
        total.variable_font_runs,
        total.color_glyph_runs,
        total.skewed_text,
        &fonts[..fonts.len().min(12)]
    );
}

/// Startup costs: library build, embedded fonts, system font scan.
fn mode_startup(_args: &[String]) {
    let t = Instant::now();
    let lib = Library::builder().build();
    let lib_ms = ms(t);
    let t = Instant::now();
    let emb: Vec<_> = typst_kit::fonts::embedded().collect();
    let emb_ms = ms(t);
    let t = Instant::now();
    let sys: Vec<_> = typst_kit::fonts::system().collect();
    let sys_ms = ms(t);
    drop(lib);
    println!(
        "{{\"library_ms\":{lib_ms:.1},\"embedded_fonts\":{},\"embedded_ms\":{emb_ms:.1},\"system_faces\":{},\"system_scan_ms\":{sys_ms:.1},\"fp_mb\":{:.1}}}",
        emb.len(),
        sys.len(),
        phys_footprint() as f64 / 1e6
    );
}

/// Emit page glyph positions (pt, y-down) + font file refs as JSON, and the PDF,
/// for the Core Graphics pixel-parity experiment.
fn mode_dump(args: &[String]) {
    let dir = PathBuf::from(&args[0]);
    let page: usize = args[1].parse().unwrap();
    let out = PathBuf::from(&args[2]);
    let world = BenchWorld::new(
        &dir,
        "main.typ",
        args.get(3).map(|s| s == "sys").unwrap_or(false),
    );
    let doc = compile(&world);
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    std::fs::write(out.with_extension("pdf"), &pdf).unwrap();
    // Single-page export of the same page (for per-page PDF-derived positions).
    let mut times = vec![];
    let mut single_ok = true;
    for _ in 0..20 {
        let t = Instant::now();
        let n1 = std::num::NonZeroUsize::new(page + 1);
        let opts = typst_pdf::PdfOptions {
            page_ranges: Some(typst::layout::PageRanges::new(vec![n1..=n1])),
            tagged: false,
            ..Default::default()
        };
        match typst_pdf::pdf(&doc, &opts) {
            Ok(bytes) => std::fs::write(out.with_extension("single.pdf"), &bytes).unwrap(),
            Err(e) => {
                single_ok = false;
                eprintln!(
                    "single-page export failed: {:?}",
                    e.first().map(|d| d.message.clone())
                );
            }
        }
        times.push(ms(t));
    }
    eprintln!(
        "single_page_export_ok={single_ok} p50_ms={:.3} p95_ms={:.3}",
        pct(&mut times.clone(), 50.0),
        pct(&mut times, 95.0)
    );
    let fr = &doc.pages()[page].frame;
    let mut fonts: Vec<Font> = vec![];
    let mut runs = String::from("[");
    let mut shapes = String::from("[");
    fn rec(
        frame: &Frame,
        ts: Transform,
        fonts: &mut Vec<Font>,
        runs: &mut String,
        shapes: &mut String,
    ) {
        for (pos, item) in frame.items() {
            let ts_here = ts.pre_concat(Transform::translate(pos.x, pos.y));
            match item {
                FrameItem::Group(g) => rec(
                    &g.frame,
                    ts_here.pre_concat(g.transform),
                    fonts,
                    runs,
                    shapes,
                ),
                FrameItem::Text(t) => {
                    let f = t.font.font().clone();
                    let fi = match fonts.iter().position(|x| *x == f) {
                        Some(i) => i,
                        None => {
                            fonts.push(f);
                            fonts.len() - 1
                        }
                    };
                    let rgb = match &t.fill {
                        Paint::Solid(c) => c.to_vec4_u8(),
                        _ => [0, 0, 0, 255],
                    };
                    if runs.len() > 1 {
                        runs.push(',');
                    }
                    write!(
                        runs,
                        "{{\"font\":{fi},\"size\":{},\"m\":[{},{},{},{},{},{}],\"rgba\":[{},{},{},{}],\"g\":[",
                        t.size.to_pt(),
                        ts_here.sx.get(), ts_here.ky.get(), ts_here.kx.get(), ts_here.sy.get(),
                        ts_here.tx.to_pt(), ts_here.ty.to_pt(),
                        rgb[0], rgb[1], rgb[2], rgb[3]
                    )
                    .unwrap();
                    let mut x = Abs::zero();
                    for (k, g) in t.glyphs.iter().enumerate() {
                        let gx = x + g.x_offset.at(t.size);
                        let gy = -g.y_offset.at(t.size);
                        if k > 0 {
                            runs.push(',');
                        }
                        write!(runs, "[{},{},{}]", g.id, gx.to_pt(), gy.to_pt()).unwrap();
                        x += g.x_advance.at(t.size);
                    }
                    runs.push_str("]}");
                }
                FrameItem::Shape(s, _) => {
                    let fill = s.fill.as_ref().and_then(|p| match p {
                        Paint::Solid(c) => Some(c.to_vec4_u8()),
                        _ => None,
                    });
                    let stroke = s.stroke.as_ref().and_then(|st| match &st.paint {
                        Paint::Solid(c) => Some((c.to_vec4_u8(), st.thickness.to_pt())),
                        _ => None,
                    });
                    let geom = match &s.geometry {
                        Geometry::Line(p) => {
                            format!("{{\"line\":[{},{}]}}", p.x.to_pt(), p.y.to_pt())
                        }
                        Geometry::Rect(sz) => {
                            format!("{{\"rect\":[{},{}]}}", sz.x.to_pt(), sz.y.to_pt())
                        }
                        Geometry::Curve(c) => {
                            let mut v = String::from("{\"curve\":[");
                            for (i, it) in c.0.iter().enumerate() {
                                if i > 0 {
                                    v.push(',');
                                }
                                match it {
                                    typst::visualize::CurveItem::Move(p) => {
                                        write!(v, "[\"m\",{},{}]", p.x.to_pt(), p.y.to_pt())
                                            .unwrap()
                                    }
                                    typst::visualize::CurveItem::Line(p) => {
                                        write!(v, "[\"l\",{},{}]", p.x.to_pt(), p.y.to_pt())
                                            .unwrap()
                                    }
                                    typst::visualize::CurveItem::Cubic(a, b, c) => write!(
                                        v,
                                        "[\"c\",{},{},{},{},{},{}]",
                                        a.x.to_pt(),
                                        a.y.to_pt(),
                                        b.x.to_pt(),
                                        b.y.to_pt(),
                                        c.x.to_pt(),
                                        c.y.to_pt()
                                    )
                                    .unwrap(),
                                    typst::visualize::CurveItem::Close => v.push_str("[\"z\"]"),
                                }
                            }
                            v.push_str("]}");
                            v
                        }
                    };
                    if shapes.len() > 1 {
                        shapes.push(',');
                    }
                    write!(
                        shapes,
                        "{{\"m\":[{},{},{},{},{},{}],\"geom\":{geom},\"fill\":{},\"stroke\":{},\"evenodd\":{}}}",
                        ts_here.sx.get(), ts_here.ky.get(), ts_here.kx.get(), ts_here.sy.get(),
                        ts_here.tx.to_pt(), ts_here.ty.to_pt(),
                        fill.map(|f| format!("{:?}", f.to_vec())).unwrap_or("null".into()), stroke.map(|(c, w)| format!("[{:?},{}]", c.to_vec(), w)).unwrap_or("null".into()),
                        matches!(s.fill_rule, typst::visualize::FillRule::EvenOdd)
                    )
                    .unwrap();
                }
                _ => {}
            }
        }
    }
    rec(
        fr,
        Transform::identity(),
        &mut fonts,
        &mut runs,
        &mut shapes,
    );
    runs.push(']');
    shapes.push(']');
    // write font blobs
    let mut fjson = String::from("[");
    for (i, f) in fonts.iter().enumerate() {
        let p = out.with_file_name(format!("font{i}.bin"));
        std::fs::write(&p, f.data().as_slice()).unwrap();
        if i > 0 {
            fjson.push(',');
        }
        write!(
            fjson,
            "{{\"path\":{:?},\"index\":{},\"family\":{:?}}}",
            p.to_str().unwrap(),
            f.index(),
            f.info().family
        )
        .unwrap();
    }
    fjson.push(']');
    let size = fr.size();
    std::fs::write(
        &out,
        format!(
            "{{\"page\":{page},\"w\":{},\"h\":{},\"fonts\":{fjson},\"runs\":{runs},\"shapes\":{shapes}}}",
            size.x.to_pt(),
            size.y.to_pt()
        ),
    )
    .unwrap();
    eprintln!("pages={} fonts={}", doc.pages().len(), fonts.len());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rest = &args[2..];
    match args[1].as_str() {
        "gen" => mode_gen(rest),
        "bench" => mode_bench(rest),
        "mem" => mode_mem(rest),
        "trace" => mode_trace(rest),
        "census" => mode_census(rest),
        "startup" => mode_startup(rest),
        "dump" => mode_dump(rest),
        "seeded" => mode_seeded(rest),
        "jumps" => mode_jumps(rest),
        m => panic!("unknown mode {m}"),
    }
}
