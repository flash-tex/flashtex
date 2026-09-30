//! One compile over the socket, timed from this side: from writing the
//! `COMPILE` frame to the decoded frame that shows what the user is looking
//! at (the edited page, or the first visible page), and to `DONE`.

use flashtex_display_list::client::{Client, CompileRequest, Edit, Event};
use flashtex_display_list::json::Json;
use flashtex_display_list::page::Item;
use std::collections::HashMap;
use std::time::Instant;

/// What the compile is waiting to show.
#[derive(Clone, Copy, Debug)]
pub enum Watch {
    /// The page holding a glyph of (line, col ± `COL_WINDOW`) of the main
    /// file: the edited page.
    Edit { line: u32, col: u32 },
    /// Page `n` being current: its `PAGE` frame, or a `PAGES` message that
    /// lists it as current (an incremental client is not resent a page that
    /// did not change).
    Page(u32),
}

/// Columns either side of the edit that still count as "the edit's glyphs".
/// A hyphenated word's glyphs carry its first letter's column, and the edit
/// sits at a word start, so a dozen bytes always catch a glyph of the edit's
/// word or its neighbours.
const COL_WINDOW: u32 = 12;

#[derive(Debug)]
pub struct Timing {
    /// COMPILE to the watched page (ms), None if it never arrived.
    pub watched_ms: Option<f64>,
    /// Its page index.
    pub watched_page: Option<u32>,
    /// Whether the watched page was seen as a `PAGE` frame (true) or a
    /// `PAGES` listing (false).
    pub watched_by_frame: bool,
    /// COMPILE to the first `PAGE` frame.
    pub first_frame_ms: Option<f64>,
    /// COMPILE to DONE (the background pass included).
    pub done_ms: f64,
    /// PAGE frames received, and how many were after the watched page.
    pub frames: u32,
    pub frames_after: u32,
    pub done: Json,
    /// The first few `DIAGNOSTIC`s (errors explain a failed compile).
    pub diagnostics: Vec<Json>,
}

/// The client side of one connection: its span table (spec §5.3: span ids
/// outlive compiles and are re-declared when lines move).
pub struct Conn {
    pub c: Client,
    main_path: String,
    /// `COMPILE.output_dir`: fixed per document, as the app's cache
    /// directory is, so a new host's S₀ key (which covers the `.aux` it
    /// read) still holds on reopen.
    output_dir: String,
    main_file: Option<u32>,
    spans: HashMap<u32, (u32, u32)>,
    next_id: i64,
    /// Pages this client holds (an incremental client's page count).
    pub pages: u32,
    /// Per page held: the main file's lines it shows, with their column
    /// range (to find the page an edit site is on: the viewport).
    shown: HashMap<u32, Vec<(u32, u32, u32)>>,
}

impl Conn {
    pub fn new(c: Client, main_abs: &str, output_dir: &str) -> Conn {
        Conn {
            c,
            main_path: main_abs.to_string(),
            output_dir: output_dir.to_string(),
            main_file: None,
            spans: HashMap::new(),
            next_id: 1,
            pages: 0,
            shown: HashMap::new(),
        }
    }

    /// The page (0-based) that showed (line, col) when last sent.
    pub fn page_of(&self, line: u32, col: u32) -> Option<u32> {
        let mut best: Option<(u32, u32)> = None; // (distance, page)
        for (&page, lines) in &self.shown {
            if page >= self.pages {
                continue;
            }
            for &(l, lo, hi) in lines {
                if l != line {
                    continue;
                }
                let d = lo.saturating_sub(col).max(col.saturating_sub(hi));
                if best.is_none_or(|(bd, bp)| d < bd || (d == bd && page < bp)) {
                    best = Some((d, page));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    fn record_shown(&mut self, index: u32, items: &[Item]) {
        let Some(f) = self.main_file else {
            return;
        };
        let mut v: Vec<(u32, u32, u32)> = Vec::new();
        let mut cur = None;
        for it in items {
            match it {
                Item::Span(sp) => cur = self.spans.get(sp).copied(),
                Item::Glyph { col, .. } if *col != u16::MAX => {
                    if let Some((file, l)) = cur {
                        if file != f {
                            continue;
                        }
                        let c = *col as u32;
                        match v.iter_mut().find(|e| e.0 == l) {
                            Some(e) => {
                                e.1 = e.1.min(c);
                                e.2 = e.2.max(c);
                            }
                            None => v.push((l, c, c)),
                        }
                    }
                }
                _ => {}
            }
        }
        self.shown.insert(index, v);
    }

    fn page_shows(&self, items: &[Item], line: u32, col: u32) -> bool {
        let Some(f) = self.main_file else {
            return false;
        };
        let mut cur = None;
        for it in items {
            match it {
                Item::Span(sp) => cur = self.spans.get(sp).copied(),
                Item::Glyph { col: c, .. } => {
                    if let Some((file, l)) = cur {
                        if file == f && l == line && (*c as u32).abs_diff(col) <= COL_WINDOW {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Send one `COMPILE` (incremental, with `edits`, showing `viewport`)
    /// and read until its `DONE`.
    pub fn compile(
        &mut self,
        root: &str,
        main: &str,
        edits: Vec<Edit>,
        viewport: Option<u32>,
        watch: Option<Watch>,
    ) -> Result<Timing, String> {
        let id = self.next_id;
        self.next_id += 1;
        let mut req = CompileRequest::new(id, root, main);
        req.incremental = true;
        req.output_dir = Some(self.output_dir.clone());
        req.viewport = viewport;
        req.edits = edits;
        let t0 = Instant::now();
        self.c
            .compile(&req)
            .map_err(|e| format!("send COMPILE: {e}"))?;
        let ms = |t0: Instant| t0.elapsed().as_secs_f64() * 1e3;
        let mut t = Timing {
            watched_ms: None,
            watched_page: None,
            watched_by_frame: false,
            first_frame_ms: None,
            done_ms: 0.0,
            frames: 0,
            frames_after: 0,
            done: Json::Null,
            diagnostics: Vec::new(),
        };
        loop {
            let ev = self
                .c
                .next_event()
                .map_err(|e| format!("read: {e}"))?
                .ok_or("the host closed the connection")?;
            match ev {
                Event::Sources(src) => {
                    for (id, path) in &src.files {
                        if *path == self.main_path {
                            self.main_file = Some(*id);
                        }
                    }
                    for &(span, file, line) in &src.spans {
                        self.spans.insert(span, (file, line));
                    }
                }
                Event::Page(p) => {
                    let now = ms(t0);
                    t.frames += 1;
                    t.first_frame_ms.get_or_insert(now);
                    if t.watched_ms.is_some() {
                        t.frames_after += 1;
                    } else {
                        let hit = match watch {
                            Some(Watch::Edit { line, col }) => self.page_shows(&p.items, line, col),
                            Some(Watch::Page(n)) => p.index == n,
                            None => false,
                        };
                        if hit {
                            t.watched_ms = Some(now);
                            t.watched_page = Some(p.index);
                            t.watched_by_frame = true;
                        }
                    }
                    self.pages = self.pages.max(p.index + 1);
                    self.record_shown(p.index, &p.items);
                }
                Event::Pages(j) => {
                    if let (None, Some(Watch::Page(n))) = (t.watched_ms, watch) {
                        let current = j.get("current").and_then(Json::as_array).unwrap_or(&[]);
                        let listed = current.iter().any(|r| {
                            let r = r.as_array().unwrap_or(&[]);
                            let a = r.first().and_then(Json::as_i64).unwrap_or(-1);
                            let b = r.get(1).and_then(Json::as_i64).unwrap_or(-2);
                            (a..=b).contains(&(n as i64))
                        });
                        if listed {
                            t.watched_ms = Some(ms(t0));
                            t.watched_page = Some(n);
                        }
                    }
                    if let Some(c) = j.int_field("count") {
                        self.pages = c as u32;
                    }
                }
                Event::Diagnostic(j) => {
                    if t.diagnostics.len() < 5 {
                        t.diagnostics.push(j);
                    }
                }
                Event::Error(j) => return Err(format!("host ERROR: {j}")),
                Event::Done(j) => {
                    t.done_ms = ms(t0);
                    if let Some(c) = j.int_field("pages") {
                        self.pages = c as u32;
                    }
                    t.done = j;
                    return Ok(t);
                }
                _ => {}
            }
        }
    }

    /// Compile without edits until the host reports nothing changed (the
    /// client's rerun after an `.aux` change, spec §6.3), at most `max`
    /// times. Returns (runs, total ms, last mode).
    pub fn settle(
        &mut self,
        root: &str,
        main: &str,
        max: u32,
    ) -> Result<(u32, f64, String), String> {
        let (mut runs, mut total) = (0, 0.0);
        loop {
            let t = self.compile(root, main, vec![], None, None)?;
            runs += 1;
            total += t.done_ms;
            let mode = t.done.str_field("mode").unwrap_or("").to_string();
            let status = t.done.str_field("status").unwrap_or("");
            if status == "failed" {
                return Err(format!("compile failed: {}", t.done));
            }
            if mode == "unchanged" || runs >= max {
                return Ok((runs, total, mode));
            }
        }
    }
}
