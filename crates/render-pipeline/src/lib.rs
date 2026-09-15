//! FlashTeX render pipeline.
//!
//! Original Rust implementation: compiler parse tree -> styled blocks ->
//! font-engine shaping (kerning + ligatures, source-byte clusters) ->
//! paragraph-layout Knuth–Plass line breaking and page breaking ->
//! math-layout Appendix G boxes with explicit rules -> TeX page builder
//! (`pagebuild`) -> display list v2 (glyph runs + rules, content-addressed
//! fonts, original glyph ids) -> runtime-v1 `compile_result` fallback. No
//! TeX engine is invoked at any point. See README.md for scope, sibling
//! pins and limitations.

pub mod date;
pub use date::TodayDate;
pub mod abstractenv;
pub mod adapter;
pub(crate) mod amsthm;
pub mod cff;
pub mod delta;
pub mod display;
pub mod floats;
pub mod fonts;
pub mod graphics;
pub mod ids;
pub mod incremental;
pub mod listings;
pub mod longtable;
pub mod mathalpha;
pub mod mathfont;
pub mod mathgrid;
pub mod mathtex;
pub mod mathtext;
pub mod nfss;
pub mod packages;
pub mod pagebuild;
pub mod params;
pub mod pdf;
pub mod protocol;
pub mod shape;
pub mod style;
pub mod table;
pub mod tablecolor;
pub mod tfm;
pub mod tikz;
pub mod toc;
pub mod typeset;
pub mod v1;

pub use display::DisplayList;
pub use fonts::FontSet;
pub use incremental::RenderCache;
pub use style::Stylesheet;

use flashtex_compiler::parser::SourceDocument;

/// Everything `render` produces. The runtime-v1 payload is derived per
/// request by `v1::fallback` because it depends on negotiated capabilities.
pub struct Rendered {
    /// Display list v2 (the authoritative geometry).
    pub v2: DisplayList,
    /// Wall-clock milliseconds spent in `render` (parse + layout + output).
    pub elapsed_ms: f64,
    /// Layout passes run (1 unless `\pageref` needed page numbers).
    pub passes: u32,
}

/// `\pageref` values converge in two passes in practice; the cap bounds a
/// document whose page numbers oscillate (reported, not looped forever).
pub const MAX_LABEL_PASSES: u32 = 3;

/// Options that runtime-v1 cannot carry and the compiler does not expose.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Class options assumed when the source has no `\documentclass` (the
    /// visual-oracle harness and the Mac app send body-only documents). The
    /// FlashTeX compiler's implicit preamble is `12pt`, US Letter, 1in
    /// margins, `\parindent 0pt`; that is the default here too.
    pub default_class_options: String,
    /// `\parindent` when the source sets neither a class nor the length.
    pub default_parindent_pt: f64,
    /// `secnumdepth` when the source does not set the counter: 2 numbers
    /// `\section` and `\subsection` (article); 0 numbers nothing (the
    /// visual-oracle preamble, which the harness strips before sending the
    /// body).
    pub default_secnumdepth: u8,
    /// The project directory `\includegraphics` files are read from
    /// (through project-files' rooted reads). `None`: images are reported
    /// unavailable.
    pub project_root: Option<std::path::PathBuf>,
    /// The date `\today` renders, supplied by the caller in the compile
    /// request (`payload.date`) rather than read from the clock here: this
    /// pipeline must stay a pure function of its inputs
    /// (`protocol/proposals/runtime-v1-request-date.md`).
    ///
    /// The default is [`TodayDate::EPOCH`], byte-for-byte what the pipeline
    /// rendered before the field existed, so every old client and every
    /// committed fixture is unchanged.
    ///
    /// **Reaches the parser.** `vendor/compiler` (pin `ea4ee5c8`) carries
    /// `parser::parse_project_with`, and `request-date` is a default Cargo
    /// feature (`Cargo.toml`), so this value is threaded, cache-keyed and
    /// handed to the compiler by default: `\today` renders the supplied date,
    /// not the epoch, in an ordinary build. Same convention as
    /// `amsmath-inline`, which went default after its own re-pin.
    /// `--no-default-features` still builds against the epoch-only path.
    pub today: TodayDate,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            default_class_options: "12pt".into(),
            default_parindent_pt: 0.0,
            default_secnumdepth: 2,
            project_root: None,
            today: TodayDate::EPOCH,
        }
    }
}

/// Renders one project. `documents` are indexed by the compiler's
/// `DocumentId`; `entry_path` selects the root (the first document when
/// absent). `revision` is echoed into both outputs.
pub fn render(
    documents: &[SourceDocument<'_>],
    entry_path: &str,
    revision: u64,
    project_id: &str,
    fonts: &FontSet,
    options: &RenderOptions,
) -> Rendered {
    render_cached(documents, entry_path, revision, project_id, fonts, options, None)
}

/// [`render`] with a block cache that outlives requests (`RenderCache`):
/// unchanged paragraphs are reused, the output is byte-identical.
#[allow(clippy::too_many_arguments)]
pub fn render_cached(
    documents: &[SourceDocument<'_>],
    entry_path: &str,
    revision: u64,
    project_id: &str,
    fonts: &FontSet,
    options: &RenderOptions,
    cache: Option<&RenderCache>,
) -> Rendered {
    let started = std::time::Instant::now();
    // FT-063: float environments are blanked (same byte length) before the
    // compiler parses the document and are laid out by `typeset::floatpage`.
    let float_envs: Vec<Vec<floats::FloatEnv>> = documents.iter().enumerate().map(|(i, d)| floats::scan(d.text, flashtex_compiler::DocumentId(i))).collect();
    let any_floats = float_envs.iter().any(|e| !e.is_empty());
    let masked: Vec<String> = documents.iter().zip(&float_envs).map(|(d, e)| if e.is_empty() { String::new() } else { floats::mask(d.text, e) }).collect();
    let texts: Vec<&str> = documents.iter().zip(&float_envs).zip(&masked).map(|((d, e), m)| if e.is_empty() { d.text } else { m.as_str() }).collect();
    // `multicols` environments are laid out by `typeset::multicol`: their
    // markup is blanked before the compiler parses (offsets unchanged).
    let multicol_scans: Vec<typeset::multicol::Scan> = texts.iter().map(|t| typeset::multicol::scan(t)).collect();
    let multicol_masked: Vec<Option<String>> = texts.iter().zip(&multicol_scans).map(|(t, s)| s.masked(t)).collect();
    let texts: Vec<&str> = texts.iter().zip(&multicol_masked).map(|(t, m)| m.as_deref().unwrap_or(t)).collect();
    let parse_docs: Vec<SourceDocument<'_>> = documents.iter().zip(&texts).map(|(d, t)| SourceDocument { path: d.path, text: t }).collect();
    // The request's date reaches `\today` here. `request-date` is a default
    // Cargo feature (vendor/compiler carries `parser::parse_project_with`),
    // so this is the normal build path; `--no-default-features` falls back to
    // the vendored parser rendering the epoch, as it always has. See
    // `RenderOptions::today`.
    #[cfg(feature = "request-date")]
    let parsed = flashtex_compiler::parser::parse_project_with(
        &parse_docs,
        entry_path,
        &flashtex_compiler::parser::ParseOptions {
            today: flashtex_compiler::date::TodayDate::new(
                options.today.year(),
                options.today.month(),
                options.today.day(),
            )
            .expect("RenderOptions::today is already a validated civil date"),
        },
    );
    #[cfg(not(feature = "request-date"))]
    let parsed = flashtex_compiler::parser::parse_project(&parse_docs, entry_path);
    // report/book number floats within the chapter (`floats::number`).
    let float_chapters = texts.get(documents.iter().position(|d| d.path == entry_path).unwrap_or(0)).and_then(|t| flashtex_class_geometry::DocumentSetup::from_preamble(t)).and_then(|s| match s.class {
        flashtex_class_geometry::ClassKind::Report => Some(false),
        flashtex_class_geometry::ClassKind::Book => Some(true),
        _ => None,
    });
    let (float_numbers, float_label_values) = floats::number(&float_envs, &texts, float_chapters);
    let mut image_cache = floats::ImageCache::default();
    let paths: Vec<&str> = documents.iter().map(|d| d.path).collect();
    let entry_index = documents.iter().position(|d| d.path == entry_path).unwrap_or(0);
    // The compiler does not know `tikzpicture`: it reports the environment
    // and every TikZ command inside it, and the pipeline typesets the
    // picture itself (`adapter` / `tikz`). Those compiler diagnostics are
    // superseded by the TikZ reader's own.
    let picture_ranges: Vec<Vec<(usize, usize)>> = texts
        .iter()
        .map(|t| flashtex_vector_graphics::tikz::find_pictures(t).into_iter().map(|p| (p.start, p.end)).collect())
        .collect();
    let in_picture = |s: &flashtex_compiler::Span| picture_ranges.get(s.document.0).is_some_and(|r| r.iter().any(|(a, b)| s.start >= *a && s.start < *b));
    let mut labels = adapter::Labels::from_parsed(&parsed);
    labels.values.extend(float_label_values);
    // `\label` given inside an `lstlisting`'s keys (`crate::listings`).
    labels.values.extend(listings::label_values(&texts));
    // Contents lists: entry pages come from the previous pass (`toc`).
    let entry_text = texts.get(entry_index).copied().unwrap_or("");
    let has_lists = toc::has_lists(entry_text);
    let has_class = adapter::class_options(entry_text).is_some();
    labels.floats = toc::float_entries(&float_envs, &documents.iter().map(|d| d.text).collect::<Vec<_>>());
    // Entry titles from source bytes (`\addcontentsline`, `\chapter`,
    // `\part`, captions) are set as body text: one parse per document.
    // A `listings` caption may hold any body command
    // (`caption={Generating a starter \texttt{ftxc.toml}}`), so its range
    // is parsed the same way.
    let has_listings = listings::present(&texts);
    if has_lists || has_listings {
        let mut spans = if has_lists {
            toc::entry_spans(entry_text, flashtex_compiler::DocumentId(entry_index), &labels.floats)
        } else {
            Vec::new()
        };
        spans.extend(listings::caption_spans(&texts));
        labels.entry_items = toc::entry_items(documents, entry_index, &texts, options, &labels, &spans);
    }
    // The compiler reports the list commands, `\addcontentsline` and
    // `\appendix` it has no model for; the pipeline sets them.
    let superseded = toc::superseded_commands(entry_text);
    let is_superseded = |s: &flashtex_compiler::Span| s.document.0 == entry_index && superseded.binary_search(&s.start).is_ok();
    let max_passes = if adapter::Labels::needs_pages(&parsed) || has_lists { MAX_LABEL_PASSES } else { 1 };
    let mut passes = 0;
    loop {
        passes += 1;
        let doc = adapter::adapt_cached(&texts, entry_index, &parsed, options, &labels, cache);
        let mut diagnostics: Vec<display::Diagnostic> = parsed
            .diagnostics
            .iter()
            .filter(|d| !d.span.as_ref().is_some_and(&in_picture))
            .filter(|d| !d.span.as_ref().is_some_and(&is_superseded))
            .filter(|d| !packages::preamble_command_superseded(&d.message, has_class))
            .map(|d| display::Diagnostic::from_compiler(d, &paths))
            // `\usepackage` gaps the pipeline fills (`packages`).
            .filter_map(|mut d| {
                d.message = packages::supersede_message(&d.message)?;
                Some(d)
            })
            .collect();
        // `abstract`: the pipeline sets what the compiler reported as an
        // unimplemented environment (`adapter::Doc::superseded`).
        diagnostics.retain(|d| {
            !doc.superseded.iter().any(|s| {
                d.sources.iter().any(|r| {
                    r.start_byte == s.start && paths.get(s.document.0).copied() == Some(&*r.path)
                })
            })
        });
        diagnostics.extend(doc.diagnostics.iter().cloned());
        diagnostics.extend(doc.limitations.iter().map(|(code, span, message)| {
            display::Diagnostic::warning(
                code,
                message.clone(),
                vec![display::SourceRange {
                    path: std::rc::Rc::from(paths.get(span.document.0).copied().unwrap_or("")),
                    start_byte: span.start,
                    end_byte: span.end,
                }],
            )
        }));
        let (mut float_specs, float_diagnostics) = if any_floats {
            floats::prepare(&float_envs, &float_numbers, documents, entry_index, &texts, &doc.style, options, &labels, &mut image_cache)
        } else {
            (Vec::new(), Vec::new())
        };
        // `prepare` makes one spec per float, in `float_envs` order: the
        // caption's `\addcontentsline` lands on the float's page.
        if has_lists {
            let keys = float_envs.iter().enumerate().flat_map(|(d, envs)| (0..envs.len()).map(move |i| toc::float_key(d, i)));
            for (spec, key) in float_specs.iter_mut().zip(keys) {
                spec.labels.push(key);
            }
        }
        diagnostics.extend(float_diagnostics);
        let mut ctx = typeset::Context::with_texts(fonts, &doc.style, &paths, &texts);
        ctx.set_math_colors(doc.math_colors.clone());
        typeset::multicol::attach(&mut ctx, &multicol_scans);
        let laid = typeset::build_with_floats(&mut ctx, &doc, cache, &float_specs);
        diagnostics.extend(ctx.take_diagnostics());
        if max_passes > 1 {
            let mut pages = typeset::label_pages(&laid);
            let toc_pages = typeset::toc_pages(&laid, &pages);
            pages.retain(|key, _| !toc::is_key(key));
            if pages == labels.pages && toc_pages == labels.toc_pages {
                // Converged: the numbers shown are the pages they sit on.
            } else if passes < max_passes {
                labels.pages = pages;
                labels.toc_pages = toc_pages;
                continue;
            } else {
                labels.pages = pages;
                labels.toc_pages = toc_pages;
                diagnostics.push(display::Diagnostic::warning(
                    "labels_unstable",
                    format!("\\pageref values and contents-list page numbers did not converge after {max_passes} layout passes; the last pass is shown"),
                    Vec::new(),
                ));
            }
        }
        let v2 = typeset::assemble(project_id, revision, documents, &doc.style, fonts, laid, diagnostics, cache, doc.page_color, doc.default_color);
        return Rendered {
            v2,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            passes,
        };
    }
}
