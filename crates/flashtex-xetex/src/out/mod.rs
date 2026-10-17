//! Unicode mode's output (docs/design/xetex/PLAN.md §3.2, phase S2 part 2):
//! each page `ship_out` writes becomes a `display-list-v3` page, and the
//! document's PDF is written from the display list by FlashTeX's own
//! writer. There is no xdvipdfmx and no XDV file in this path.
//!
//! **How a page gets here.** `xetex.web`'s `ship_out` writes the page as
//! XDV into its DVI buffer whatever the output; in TeX Live, without
//! `-no-pdf`, `dvi_open_out` opens a pipe to xdvipdfmx and `ship_out` calls
//! `fflush` on it after each page. Here the "pipe" is memory: `dvi_open_out`
//! gives the DVI file a memory sink ([`crate::system::ByteFile`]), and at
//! that `fflush` [`page_done`] takes the page's bytes (`bop` to `eop`, some
//! still in `dvi_buf`) and reads them into a display-list page
//! ([`page::Builder`]): the XDV stream is the engine's interface to its
//! output, exactly as pdfTeX's content stream is Classic's (DESIGN.md
//! §6.1: positions are where the engine's own output code put them). The
//! bytes are dropped once read. `xetex.web` is not changed for this.
//!
//! * [`page`]: the XDV page reader and the display-list page it builds;
//! * [`special`]: dvipdfmx's `\special` language, from its documentation
//!   and measured behaviour (not ported);
//! * [`content`]: PDF operators the specials write, read into paths;
//! * [`fonts`], [`fontmap`], [`images`]: the `FONT` and `IMAGE` resources;
//! * [`doc`]: what lasts across pages (named objects, outline, forms);
//! * [`pdfobj`]: PDF object syntax of the specials;
//! * [`pdf`]: the PDF writer, and [`tounicode`] its glyphs' text (with [`agl`],
//!   the Adobe Glyph List), as xdvipdfmx derives it.
//!
//! The display list goes to `FLASHTEX_DISPLAY_LIST` (spec §6.6) as pages
//! complete; at the end [`pdf`] writes the PDF from the display list.

pub mod agl;
pub mod cffcid;
pub mod content;
pub mod doc;
pub mod fontmap;
pub mod fonts;
pub mod images;
pub mod num;
pub mod page;
pub mod pdf;
pub mod pdfobj;
pub mod special;
pub mod tounicode;

use crate::generated::consts::{dvi_buf_size, font_base, int_base, mag_code};
use crate::generated::Globals;
use crate::system::ByteFile;
use doc::{Built, Doc};
use flashtex_display_list::frame::write_frame;
use flashtex_display_list::kind;
use flashtex_display_list::page::Item;
use flashtex_display_list::resource::Font;
use std::collections::HashSet;
use std::io::Write;

/// `n / d` rounded to nearest, halves away from zero (d > 0).
pub fn div_round(n: i128, d: i128) -> i128 {
    flashtex_engine::displaylist::fixed::div_round(n, d)
}

/// The output of one run in PDF mode.
pub struct Output {
    pub doc: Doc,
    /// The PDF file, opened by `dvi_open_out` (as xetex opens its pipe).
    pub pdf: Option<std::fs::File>,
    pub pdf_name: String,
    dl: Option<DlSink>,
    /// A resident engine's run: the output's reports (`FlashTeX output:
    /// ...` lines), which a run in a process of its own writes to its
    /// standard error.
    pub reports: Option<Vec<String>>,
    /// Compress the PDF's streams: not for a host's preview compiles
    /// (`FLASHTEX_UNICODE_PREVIEW=1`, `resident::Spec::preview`), whose PDF is the
    /// preview's, its streams stored, as the pdfTeX host's (spec §6.3);
    /// an export's is compressed.
    pub compress: bool,
}

/// Where display-list frames go, and what has been sent there.
struct DlSink {
    w: Box<dyn Write + Send>,
    fonts: HashSet<u16>,
    images: HashSet<u32>,
    forms: usize,
    /// The source map's spans and files sent (spec §5.3).
    spans: HashSet<u32>,
    files: HashSet<u32>,
    /// `FLASHTEX_DISPLAY_LIST_FONT_FORMATS`: the font formats whose
    /// programs the reader takes (`None`: all).
    formats: Option<HashSet<String>>,
}

impl Output {
    /// The output for a run, with the display list going where
    /// `FLASHTEX_DISPLAY_LIST` says (spec §6.6), if it is set.
    pub fn from_env() -> Output {
        use flashtex_display_list::endpoint::{Endpoint, ENV};
        let dl = match Endpoint::from_env() {
            None => None,
            Some(Err(e)) => {
                eprintln!("{e}");
                None
            }
            Some(Ok(ep)) => match ep.open_writer() {
                Ok(w) => Some(DlSink {
                    w,
                    fonts: HashSet::new(),
                    images: HashSet::new(),
                    forms: 0,
                    spans: HashSet::new(),
                    files: HashSet::new(),
                    formats: std::env::var("FLASHTEX_DISPLAY_LIST_FONT_FORMATS")
                        .ok()
                        .map(|s| {
                            s.split(',')
                                .map(|f| f.trim().to_string())
                                .filter(|f| !f.is_empty())
                                .collect()
                        }),
                }),
                Err(e) => {
                    eprintln!("{ENV}: {ep}: {e}");
                    None
                }
            },
        };
        let mut o = Output::with_sink(dl);
        o.compress = std::env::var("FLASHTEX_UNICODE_PREVIEW").map_or(true, |v| v != "1");
        o
    }

    /// The output of a resident engine's run (`crate::resident`): the
    /// display list goes to `w` (none: no display list), the programs of
    /// fonts in `formats` only (`None`: all), and the output's reports are
    /// kept ([`Output::reports`]) instead of going to the standard error.
    pub fn to_writer(
        w: Option<Box<dyn Write + Send>>,
        formats: Option<HashSet<String>>,
        cache: Option<std::sync::Arc<fonts::FontCache>>,
    ) -> Output {
        let dl = w.map(|w| DlSink {
            w,
            fonts: HashSet::new(),
            images: HashSet::new(),
            forms: 0,
            spans: HashSet::new(),
            files: HashSet::new(),
            formats,
        });
        let mut o = Output::with_sink(dl);
        o.reports = Some(Vec::new());
        o.doc.fonts.cache = cache;
        o
    }

    fn with_sink(dl: Option<DlSink>) -> Output {
        let mut doc = Doc::new();
        doc.default_paper = default_paper();
        Output {
            doc,
            pdf: None,
            pdf_name: String::new(),
            dl,
            reports: None,
            compress: true,
        }
    }

    /// A resident run restarted from a page checkpoint
    /// (`crate::ckpt::PageCk`): the document as it was there, its PDF file
    /// open again if it was, and its pages and forms sent again to the
    /// display list, in order (each compile's pages are the client's anew).
    pub fn resume(
        &mut self,
        snap: &crate::ckpt::OutSnap,
        g: &crate::generated::Globals,
    ) -> Result<(), String> {
        let cache = self.doc.fonts.cache.take();
        self.doc = snap.doc.clone();
        self.doc.fonts.cache = cache;
        self.pdf_name = snap.pdf_name.clone();
        if snap.pdf_open {
            let f = std::fs::File::create(&self.pdf_name)
                .map_err(|e| format!("{}: {e}", self.pdf_name))?;
            self.pdf = Some(f);
        }
        let pages = self.doc.pages.clone();
        for b in &pages {
            // with the source map's files and spans (the engine's map is
            // the checkpoint's, which holds all those of these pages)
            send_page(self, b, |spans, sent, files| {
                g.dl_sources(spans, sent, files)
            });
        }
        Ok(())
    }

    /// A report of the output: kept (a resident engine's run), else on the
    /// standard error, where the host that started the run reads it.
    fn report(&mut self, line: String) {
        match self.reports.as_mut() {
            Some(r) => r.push(line),
            None => eprintln!("{line}"),
        }
    }
}

/// The paper of a page without `pdf:pagesize`: TeX Live's `dvipdfmx.cfg`
/// (`p letter` or `p a4`, as `tlmgr paper` set it), A4 without one.
fn default_paper() -> Option<(
    flashtex_engine::displaylist::fixed::Fx,
    flashtex_engine::displaylist::fixed::Fx,
)> {
    use flashtex_engine::displaylist::fixed::{Fx, ONE};
    let a4 = (page::fx(595.27559), page::fx(841.88976));
    let letter = (Fx(612 * ONE), Fx(792 * ONE));
    // TEXMFCONFIG (the user's), TEXMFSYSCONFIG (tlmgr's), TEXMFDIST.
    for var in ["TEXMFCONFIG", "TEXMFSYSCONFIG", "TEXMFDIST"] {
        let Some(dir) = flashtex_engine::system::texmf_var(var) else {
            continue;
        };
        let dir = dir.trim_start_matches('!');
        let Ok(text) = std::fs::read_to_string(format!("{dir}/dvipdfmx/dvipdfmx.cfg")) else {
            continue;
        };
        for line in text.lines() {
            let mut w = line.split_whitespace();
            if w.next() == Some("p") {
                return Some(match w.next() {
                    Some("letter") => letter,
                    _ => a4,
                });
            }
        }
    }
    Some(a4)
}

/// TFM character dimensions from the engine's font tables.
struct EngineMetrics<'g>(&'g Globals);

impl page::Metrics for EngineMetrics<'_> {
    fn tfm_char(&self, k: i32, c: u32) -> Option<(i32, i32, i32)> {
        let g = self.0;
        // dvi_font_def numbers font f as f - font_base - 1
        let f = (k + font_base + 1) as usize;
        if f > g.font_ptr as usize {
            return None;
        }
        let c = c as i32;
        if c < g.font_bc[f] || c > g.font_ec[f] {
            return None;
        }
        let ci = g.font_info[(g.char_base[f] + c) as usize];
        let wi = ci.qqqq_b0();
        if wi == 0 {
            return None;
        }
        let hd = ci.qqqq_b1();
        let w = g.font_info[(g.width_base[f] + wi) as usize].int();
        let h = g.font_info[(g.height_base[f] + hd / 16) as usize].int();
        let d = g.font_info[(g.depth_base[f] + hd % 16) as usize].int();
        Some((w, h, d))
    }
}

/// `ship_out` has written a page (its `fflush`): read it into the
/// display list.
pub fn page_done(g: &mut Globals, f: &mut ByteFile) {
    g.resident_poll();
    let Some(mut out) = g.host.out.take() else {
        return;
    };
    let start = g.last_bop as i64;
    let end = g.dvi_offset as i64 + g.dvi_ptr as i64;
    let gone = g.dvi_gone as i64;
    let mut bytes = Vec::with_capacity((end - start).max(0) as usize);
    let mut missing = None;
    if let Some(m) = f.mem.as_mut() {
        let byte = |p: i64| -> Option<u8> {
            if p < gone {
                let i = usize::try_from(p - m.base as i64).ok()?;
                m.buf.get(i).copied()
            } else {
                let off = g.dvi_offset as i64;
                let i = if p >= off {
                    p - off
                } else {
                    p - off + dvi_buf_size as i64
                };
                usize::try_from(i)
                    .ok()
                    .filter(|&i| i < dvi_buf_size as usize)
                    .map(|i| g.dvi_buf[i] as u8)
            }
        };
        // The first page: the preamble's comment, which xdvipdfmx makes
        // the PDF's `/Creator`.
        if out.doc.dvi_comment.is_none() && byte(0) == Some(247) {
            if let Some(k) = byte(14) {
                out.doc.dvi_comment = (15..15 + k as i64).map(byte).collect();
            }
        }
        for p in start..end {
            match byte(p) {
                Some(b) => bytes.push(b),
                None => {
                    missing = Some(p);
                    break;
                }
            }
        }
        m.consumed(end as u64);
    }
    // (a resident run keeps what the page is built from: `Built::replay`)
    let resident = g.host.resident.is_some();
    let pictures: Vec<String> = if resident {
        g.host.pictures.iter().cloned().collect()
    } else {
        vec![]
    };
    out.doc.pictures.append(&mut g.host.pictures);
    let mag = g.eqtb[(int_base + mag_code - 1) as usize].int();
    let page_no = g.total_pages as u32; // already counted
                                        // The source map's markers of this page, as offsets into its bytes
                                        // (`crate::dlsource`); each page starts unmarked.
    let marks: Vec<(u32, u32, u16)> = std::mem::take(&mut g.host.dl.markers)
        .into_iter()
        .filter(|m| (m.pos as i64) >= start && (m.pos as i64) < end)
        .map(|m| ((m.pos as i64 - start) as u32, m.span, m.col))
        .collect();
    g.host.dl.page_done();
    let replay = (resident && missing.is_none()).then(|| doc::Replay {
        xdv: bytes.clone(),
        marks: marks.clone(),
        mag,
        pictures,
    });
    let built = match missing {
        Some(p) => Err(format!("byte {p} of the XDV stream is not in memory")),
        None => {
            page::Builder::page_marked(&mut out.doc, &EngineMetrics(g), mag, page_no, &bytes, marks)
        }
    };
    match built {
        Ok(mut b) => {
            b.forms_at = out.doc.forms.len();
            b.replay = replay.map(std::sync::Arc::new);
            send_page(&mut out, &b, |spans, sent, files| {
                g.dl_sources(spans, sent, files)
            });
            out.doc.pages.push(std::sync::Arc::new(b));
        }
        Err(e) => out.doc.errors.push(format!("page {page_no}: {e}")),
    }
    g.host.out = Some(out);
}

/// The convergence jump (`crate::conv`): page `page_no` built again into
/// this run's document from what the converged run built it from
/// ([`doc::Built::replay`]), and sent to the display list, as `page_done`
/// does. The engine's state is the converged run's after the page (its
/// fonts are loaded).
pub fn replay_page(g: &mut Globals, old: &doc::Built, page_no: u32) -> Result<(), String> {
    let r = old.replay.clone().ok_or("the page's XDV was not kept")?;
    let mut out = g.host.out.take().ok_or("no output")?;
    out.doc.pictures.extend(r.pictures.iter().cloned());
    let built = page::Builder::page_marked(
        &mut out.doc,
        &EngineMetrics(g),
        r.mag,
        page_no,
        &r.xdv,
        r.marks.clone(),
    );
    match built {
        Ok(mut b) => {
            b.forms_at = out.doc.forms.len();
            b.replay = Some(r);
            send_page(&mut out, &b, |spans, sent, files| {
                g.dl_sources(spans, sent, files)
            });
            out.doc.pages.push(std::sync::Arc::new(b));
        }
        Err(e) => out.doc.errors.push(format!("page {page_no}: {e}")),
    }
    g.host.out = Some(out);
    Ok(())
}

/// The end of the run (`dvi_close`): the PDF. 0, or a nonzero driver
/// return code.
pub fn finish(g: &mut Globals) -> i32 {
    let Some(mut out) = g.host.out.take() else {
        return 0;
    };
    let mut rc = 0;
    if let Some(dl) = out.dl.as_mut() {
        if dl.w.flush().is_err() {
            out.dl = None;
        }
    }
    let mut reports = vec![];
    for (what, n) in &out.doc.unknown {
        reports.push(format!("FlashTeX output: {n} unknown special(s) `{what}'"));
    }
    for d in &out.doc.diagnostics {
        reports.push(format!("FlashTeX output: {d}"));
    }
    for e in &out.doc.errors {
        reports.push(format!("! FlashTeX output: {e}"));
        rc = 1;
    }
    if let Some(mut file) = out.pdf.take() {
        let opts = pdf::Options {
            producer: format!("FlashTeX (Unicode mode, {})", crate::system::BANNER),
            date: pdf_date(g.host.clock.map(|c| c.0)),
            compress: out.compress,
        };
        match pdf::write(&out.doc, &opts) {
            Ok((bytes, warnings)) => {
                for w in warnings {
                    reports.push(format!("FlashTeX output: {w}"));
                }
                if let Err(e) = file.write_all(&bytes).and_then(|_| file.flush()) {
                    reports.push(format!("FlashTeX output: writing {}: {e}", out.pdf_name));
                    rc = 1;
                }
            }
            Err(e) => {
                reports.push(format!("FlashTeX output: {e}"));
                rc = 1;
            }
        }
    }
    for r in reports {
        out.report(r);
    }
    g.host.out = Some(out);
    rc
}

/// `D:YYYYMMDDHHmmSSZ` of `SOURCE_DATE_EPOCH` when it is set (with
/// `FORCE_SOURCE_DATE` or not: the PDF's dates are the output's), else of
/// `session` (the pinned clock, `Host::clock`), else of now.
fn pdf_date(session: Option<i64>) -> Option<String> {
    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .or(session)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64)
        });
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // civil from days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    Some(format!(
        "D:{y:04}{m:02}{d:02}{:02}{:02}{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    ))
}

/// Send a page, and the forms and resources it needs first, to the
/// display-list sink: the forms made since the last page, which were
/// `b.forms_at` when it shipped (a page sent again by `Output::resume`
/// comes after the forms it came after), and before them the source map's
/// files and spans they use (`sources`: `Globals::dl_sources`).
fn send_page(
    out: &mut Output,
    b: &Built,
    sources: impl FnOnce(&[u32], &mut HashSet<u32>, &mut HashSet<u32>) -> Option<Vec<u8>>,
) {
    let Some(dl) = out.dl.as_mut() else {
        return;
    };
    let doc = &out.doc;
    // The source map's spans of the page and of the forms sent with it,
    // declared before them (spec §5.3).
    let forms_to = b.forms_at.min(doc.forms.len());
    let mut spans: Vec<u32> = doc.forms[dl.forms.min(forms_to)..forms_to]
        .iter()
        .map(|f| &**f)
        .chain(std::iter::once(b))
        .flat_map(|x| x.dl.items.iter())
        .filter_map(|it| match it {
            Item::Span(s) => Some(*s),
            _ => None,
        })
        .collect();
    spans.sort_unstable();
    spans.dedup();
    let src = sources(&spans, &mut dl.spans, &mut dl.files);
    let mut ok = true;
    let mut send = |k: u8, body: &[u8], w: &mut Box<dyn Write + Send>| {
        if ok && write_frame(w, k, body).is_err() {
            ok = false;
        }
    };
    if let Some(src) = src {
        send(kind::SOURCES, &src, &mut dl.w);
    }
    // Forms made since the last page.
    while dl.forms < b.forms_at.min(doc.forms.len()) {
        let form = &doc.forms[dl.forms];
        dl.forms += 1;
        send_resources(dl, doc, form, &mut send);
        let body = encode(doc, form);
        send(kind::FORM, &body, &mut dl.w);
    }
    send_resources(dl, doc, b, &mut send);
    let body = encode(doc, b);
    send(kind::PAGE, &body, &mut dl.w);
    if !ok || dl.w.flush().is_err() {
        out.dl = None;
    }
    flashtex_page_sent();
}

/// A page's frame has been written to the display-list sink. It does
/// nothing: it is a marker at which a profiler can take its counts
/// (`tools/incr-bench/unicode.py --callgrind` runs the host under callgrind
/// with `--dump-after=flashtex_page_sent`, so the instructions up to each
/// page are known exactly, on the engine's own thread).
#[no_mangle]
#[inline(never)]
pub extern "C" fn flashtex_page_sent() {
    std::hint::black_box(());
}

fn send_resources(
    dl: &mut DlSink,
    doc: &Doc,
    b: &Built,
    send: &mut impl FnMut(u8, &[u8], &mut Box<dyn Write + Send>),
) {
    for &id in &b.fonts {
        if !dl.fonts.insert(id) {
            continue;
        }
        let Some(r) = doc.fonts.resource(id) else {
            continue;
        };
        let format = r.info.str_field("format").unwrap_or("none");
        let withheld = dl
            .formats
            .as_ref()
            .is_some_and(|ok| !matches!(format, "type1" | "none") && !ok.contains(format));
        let font = Font {
            id,
            key: r.key,
            info: r.info.clone(),
            program: if withheld {
                Vec::new()
            } else {
                r.program.to_vec()
            },
        };
        send(kind::FONT, &font.encode(), &mut dl.w);
    }
    for &id in &b.images {
        if !dl.images.insert(id) {
            continue;
        }
        let Some(img) = doc.images.get(id) else {
            continue;
        };
        send(kind::IMAGE, img.info.to_string().as_bytes(), &mut dl.w);
    }
}

/// A built page's or form's `PAGE`/`FORM` body, with its content hash
/// (made once a page: `Built::frame`).
pub fn encode(doc: &Doc, b: &Built) -> Vec<u8> {
    b.frame
        .get_or_init(|| {
            let mut page = b.dl.clone();
            // The rules as FlashTeX's PDF draws them (spec §4.4).
            page.rule_geometry = pdf::rule_geometry(&page);
            let font_key = |id: u16| doc.fonts.resource(id).map_or([0; 32], |r| r.key);
            let image_key = |id: u32| doc.images.get(id).map_or([0; 32], |i| i.key);
            page.hash = page.content_hash(&font_key, &image_key);
            page.encode()
        })
        .clone()
}
