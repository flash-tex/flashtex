//! Conservative dependency-aware incremental compilation.
//!
//! The parser always executes the complete finite language so macro definitions,
//! group restoration, diagnostics, and recovery retain exactly the clean-build
//! semantics. Reuse is limited to positioned per-block layout fragments, and is
//! allowed only when source mapping, the definitions actually read by the block,
//! preamble bytes, layout constraints, and entering flow state all match.
//!
//! This is not a general TeX incremental algorithm. Mutable category codes,
//! registers, assignments, conditionals, auxiliary files, output routines,
//! external effects, unsupported constructs, and malformed input force a full
//! layout rebuild. Any future construct is unsafe until its complete state and
//! side effects are represented in these cache checks. When in doubt, rebuild.

use crate::diagnostics::{limit_repeats, Diagnostic};
use crate::layout::{self, FlowState, LayoutCursor, Page, PlacedItem, TextItem};
use crate::math::{MathAtom, MathList, Nucleus};
use crate::parser::{self, Block, Inline, MacroDependency, MathRow, SourceDocument, VerbatimLine};
use crate::Span;
use std::collections::HashMap;
use std::ops::Range;

pub use crate::layout::LayoutConstraints;

/// Per-revision evidence of how much block layout work was reused.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReuseStats {
    pub blocks_total: usize,
    pub blocks_reused: usize,
    pub blocks_recomputed: usize,
    /// Full dependency-and-block equality checks after indexed lookup.
    pub candidate_comparisons: usize,
    pub full_recompile: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompileOutput {
    pub blocks: Vec<Block>,
    pub diagnostics: Vec<Diagnostic>,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncrementalResult {
    pub output: CompileOutput,
    pub stats: ReuseStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedBytes {
    pub old: Range<usize>,
    pub new: Range<usize>,
}

/// Smallest changed UTF-8-safe ranges in two snapshots.
pub fn changed_bytes(old: &str, new: &str) -> ChangedBytes {
    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    let shared = old_bytes.len().min(new_bytes.len());
    let mut prefix = 0;
    while prefix < shared && old_bytes[prefix] == new_bytes[prefix] {
        prefix += 1;
    }
    while prefix > 0 && (!old.is_char_boundary(prefix) || !new.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let mut suffix = 0;
    while suffix < old_bytes.len().saturating_sub(prefix)
        && suffix < new_bytes.len().saturating_sub(prefix)
        && old_bytes[old_bytes.len() - 1 - suffix] == new_bytes[new_bytes.len() - 1 - suffix]
    {
        suffix += 1;
    }
    while suffix > 0
        && (!old.is_char_boundary(old_bytes.len() - suffix)
            || !new.is_char_boundary(new_bytes.len() - suffix))
    {
        suffix -= 1;
    }
    ChangedBytes {
        old: prefix..old_bytes.len() - suffix,
        new: prefix..new_bytes.len() - suffix,
    }
}

#[derive(Debug, Clone)]
struct CachedBlock {
    /// The block itself is `Revision::output.blocks` at the same index.
    dependencies: Vec<MacroDependency>,
    prepared_state: FlowState,
    end_state: FlowState,
    placed: Vec<PlacedItem>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct Revision {
    documents: Vec<(String, String)>,
    entry_path: String,
    constraints: LayoutConstraints,
    /// The per-request inputs this revision was compiled with. A warm session
    /// must not hand yesterday's pages back when the request's date changes.
    options: parser::ParseOptions,
    preamble_source: String,
    incremental_safe: bool,
    document_global_state: bool,
    output: CompileOutput,
    blocks: Vec<CachedBlock>,
}

/// Previous revision and reusable block-layout fragments for one document.
#[derive(Debug, Default)]
pub struct Session {
    previous: Option<Revision>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compile(&mut self, text: &str, constraints: LayoutConstraints) -> IncrementalResult {
        self.compile_project(&[SourceDocument { path: "", text }], "", constraints)
    }

    /// Compile a complete supplied project while retaining reusable block layout.
    pub fn compile_project(
        &mut self,
        documents: &[SourceDocument<'_>],
        entry_path: &str,
        constraints: LayoutConstraints,
    ) -> IncrementalResult {
        self.compile_project_with(
            documents,
            entry_path,
            constraints,
            &parser::ParseOptions::default(),
        )
    }

    /// Compile a complete supplied project with explicit per-request inputs
    /// (today the date `\today` renders; see `parser::ParseOptions`).
    pub fn compile_project_with(
        &mut self,
        documents: &[SourceDocument<'_>],
        entry_path: &str,
        constraints: LayoutConstraints,
        options: &parser::ParseOptions,
    ) -> IncrementalResult {
        let snapshot: Vec<(String, String)> = documents
            .iter()
            .map(|document| (document.path.to_string(), document.text.to_string()))
            .collect();
        if let Some(previous) = &self.previous {
            if previous.documents == snapshot
                && previous.entry_path == entry_path
                && previous.constraints == constraints
                && previous.options == *options
            {
                let total = previous.output.blocks.len();
                return IncrementalResult {
                    output: previous.output.clone(),
                    stats: ReuseStats {
                        blocks_total: total,
                        blocks_reused: total,
                        blocks_recomputed: 0,
                        candidate_comparisons: 0,
                        full_recompile: false,
                    },
                };
            }
        }

        let mut parsed = parser::parse_project_with(documents, entry_path, options);
        let constraints = parsed.preamble_constraints(constraints);
        let same_document_set = self.previous.as_ref().is_some_and(|previous| {
            previous.entry_path == entry_path
                && previous.documents.len() == snapshot.len()
                && previous
                    .documents
                    .iter()
                    .zip(&snapshot)
                    .all(|((old_path, _), (new_path, _))| old_path == new_path)
        });
        let can_reuse = self.previous.as_ref().is_some_and(|previous| {
            same_document_set
                && previous.incremental_safe
                && parsed.incremental_safe
                && previous.constraints == constraints
                && previous.preamble_source == parsed.preamble_source
                && !previous.document_global_state
                && !parsed.document_global_state
        });
        let changes: Vec<ChangedBytes> = self.previous.as_ref().map_or_else(Vec::new, |previous| {
            previous
                .documents
                .iter()
                .zip(&snapshot)
                .map(|((_, old), (_, new))| changed_bytes(old, new))
                .collect()
        });
        let deltas: Vec<isize> = self.previous.as_ref().map_or_else(Vec::new, |previous| {
            previous
                .documents
                .iter()
                .zip(&snapshot)
                .map(|((_, old), (_, new))| new.len() as isize - old.len() as isize)
                .collect()
        });

        let mut cursor = LayoutCursor::new(constraints);
        let mut cache = Vec::with_capacity(parsed.blocks.len());
        let mut stats = ReuseStats {
            blocks_total: parsed.blocks.len(),
            full_recompile: !can_reuse,
            ..ReuseStats::default()
        };

        if parsed.document_global_state {
            let (pages, layout_diagnostics) = layout::layout_converged_with_options(
                &parsed.blocks,
                constraints,
                &parsed.cleveref,
            );
            let mut diagnostics = parsed.diagnostics;
            diagnostics.append(&mut limit_repeats(layout_diagnostics));
            stats.full_recompile = true;
            stats.blocks_recomputed = parsed.blocks.len();
            let output = CompileOutput {
                blocks: parsed.blocks,
                diagnostics,
                pages,
            };
            self.previous = Some(Revision {
                options: *options,
                documents: snapshot,
                entry_path: entry_path.to_string(),
                constraints,
                preamble_source: parsed.preamble_source,
                incremental_safe: parsed.incremental_safe,
                document_global_state: true,
                output: output.clone(),
                blocks: Vec::new(),
            });
            return IncrementalResult { output, stats };
        }

        // Shift each cached block ONCE, not once per comparison.
        //
        // This lookup used to shift a cached block inside the inner scan, so a
        // 500-block document performed 500 x 500 deep clone-and-shift operations
        // per edit. That made a one-word edit six times slower than a full cold
        // compile despite reusing 499 of 500 blocks, and pushed the measured
        // warm-edit p95 from 21 ms to 177 ms against a 200 ms target.
        // Index the shifted candidates by a cheap signature so the lookup below is
        // not a scan over every cached block.
        //
        // Measured before this change: a one-word edit was 0.801 ms p95 at 5 KB,
        // 7.402 ms at 50 KB and 473.759 ms at 7 754 blocks (500 KB) — 64x the
        // time for 10x the blocks, i.e. quadratic, and far past the 200 ms
        // budget. The signature is the block's first and last source offsets,
        // which are already computed; full structural equality still gates
        // acceptance, so a signature collision can never cause a wrong reuse.
        // Build the index without eagerly materializing a second copy of the
        // whole cache. A shifted signature needs only the boundary spans. Each
        // signature hit is still shifted and compared structurally below, and
        // every reused placed item still needs a current-revision source span;
        // those confirmation/output walks are linear in reused content.
        //
        // Issue #65: the previous revision is consumed, so its blocks, placed
        // items and diagnostics are shifted IN PLACE exactly once (no clone of
        // every word's text per comparison), and a reused fragment is moved into
        // the new cache instead of being copied again. A slot whose spans cannot
        // all be mapped is never indexed, and a slot is consumed by its first
        // reuse; either case only falls back to recomputation.
        let mut previous = if can_reuse {
            self.previous.take()
        } else {
            None
        };
        let mut available = Vec::new();
        let mut candidate_index: HashMap<BlockSignature, Vec<usize>> = HashMap::new();
        if let Some(previous) = previous.as_mut() {
            available.resize(previous.blocks.len(), false);
            for (slot, (block, cached)) in previous
                .output
                .blocks
                .iter_mut()
                .zip(previous.blocks.iter_mut())
                .enumerate()
            {
                if shift_block(block, &changes, &deltas).is_some()
                    && shift_placed(&mut cached.placed, &changes, &deltas).is_some()
                    && shift_diagnostics(&mut cached.diagnostics, &changes, &deltas).is_some()
                {
                    available[slot] = true;
                    candidate_index
                        .entry(block_signature(block))
                        .or_default()
                        .push(slot);
                }
            }
        }

        let mut block_dependencies = std::mem::take(&mut parsed.block_dependencies);
        for (index, block) in parsed.blocks.iter().enumerate() {
            let dependencies = std::mem::take(&mut block_dependencies[index]);
            let prepared_state = cursor.prepare_block(block);
            let diagnostics_start = cursor.diagnostics_len();
            let candidate = previous.as_ref().and_then(|previous| {
                candidate_index
                    .get(&block_signature(block))
                    .into_iter()
                    .flatten()
                    .copied()
                    .find(|&slot| {
                        if !available[slot] {
                            return false;
                        }
                        stats.candidate_comparisons += 1;
                        // Full equality still decides: the signature only
                        // narrows the search, it never authorises a reuse.
                        previous.blocks[slot].dependencies == dependencies
                            && previous.output.blocks[slot] == *block
                    })
            });

            let reusable = candidate.zip(previous.as_mut()).filter(|(slot, previous)| {
                prepared_state.same_geometry(previous.blocks[*slot].prepared_state)
            });
            let placed = if let Some((slot, previous)) = reusable {
                let cached = &mut previous.blocks[slot];
                available[slot] = false;
                cursor.append_reused(&cached.placed, &cached.diagnostics, cached.end_state);
                stats.blocks_reused += 1;
                std::mem::take(&mut cached.placed)
            } else {
                stats.blocks_recomputed += 1;
                cursor.render_prepared_block(block)
            };
            let end_state = cursor.state();
            let block_diagnostics = cursor.diagnostics_since(diagnostics_start).to_vec();
            cache.push(CachedBlock {
                dependencies,
                prepared_state,
                end_state,
                placed,
                diagnostics: block_diagnostics,
            });
        }
        drop(previous);

        let (pages, layout_diagnostics) = cursor.into_pages_and_diagnostics();
        let mut diagnostics = parsed.diagnostics;
        diagnostics.append(&mut limit_repeats(layout_diagnostics));
        let output = CompileOutput {
            blocks: parsed.blocks,
            diagnostics,
            pages,
        };
        self.previous = Some(Revision {
            options: *options,
            documents: snapshot,
            entry_path: entry_path.to_string(),
            constraints,
            preamble_source: parsed.preamble_source,
            incremental_safe: parsed.incremental_safe,
            document_global_state: false,
            output: output.clone(),
            blocks: cache,
        });
        IncrementalResult { output, stats }
    }
}

/// Authoritative clean compile for equivalence checks and callers without a session.
pub fn compile_full(text: &str, constraints: LayoutConstraints) -> CompileOutput {
    compile_full_project(&[SourceDocument { path: "", text }], "", constraints)
}

/// Authoritative clean compile for a complete supplied project.
pub fn compile_full_project(
    documents: &[SourceDocument<'_>],
    entry_path: &str,
    constraints: LayoutConstraints,
) -> CompileOutput {
    compile_full_project_with(
        documents,
        entry_path,
        constraints,
        &parser::ParseOptions::default(),
    )
}

/// Authoritative clean compile with explicit per-request inputs.
pub fn compile_full_project_with(
    documents: &[SourceDocument<'_>],
    entry_path: &str,
    constraints: LayoutConstraints,
    options: &parser::ParseOptions,
) -> CompileOutput {
    let parsed = parser::parse_project_with(documents, entry_path, options);
    let constraints = parsed.preamble_constraints(constraints);
    let (pages, layout_diagnostics) = layout::layout_converged_with_options(
        &parsed.blocks,
        constraints,
        &parsed.cleveref,
    );
    // Parser diagnostics are already bounded (`parse_project_with`); the
    // layout's are bounded on their own, so a summary is never re-counted.
    let mut diagnostics = parsed.diagnostics;
    diagnostics.append(&mut limit_repeats(layout_diagnostics));
    CompileOutput {
        blocks: parsed.blocks,
        diagnostics,
        pages,
    }
}

fn mapped_span(span: Span, changes: &[ChangedBytes], deltas: &[isize]) -> Option<Span> {
    let change = changes.get(span.document.0)?;
    let delta = *deltas.get(span.document.0)?;
    if span.end <= change.old.start {
        Some(span)
    } else if span.start >= change.old.end {
        Some(shift_span(span, delta))
    } else {
        None
    }
}

fn shift_span(span: Span, delta: isize) -> Span {
    Span::in_document(
        span.document,
        span.start
            .checked_add_signed(delta)
            .expect("valid span shift"),
        span.end
            .checked_add_signed(delta)
            .expect("valid span shift"),
    )
}

/// Map one span in place; `None` when it overlaps the changed range.
///
/// Every `shift_*` below destructures each variant's fields explicitly (no
/// `..`), so a new span-carrying field fails to compile until it is mapped.
/// A `None` may leave the value partially shifted; callers then never reuse it.
fn map_span(span: &mut Span, changes: &[ChangedBytes], deltas: &[isize]) -> Option<()> {
    *span = mapped_span(*span, changes, deltas)?;
    Some(())
}

fn shift_block(block: &mut Block, changes: &[ChangedBytes], deltas: &[isize]) -> Option<()> {
    match block {
        Block::Paragraph(inlines) => shift_inlines(inlines, changes, deltas),
        Block::Heading {
            level: _,
            number: _,
            number_span,
            content,
        } => {
            map_span(number_span, changes, deltas)?;
            shift_inlines(content, changes, deltas)
        }
        Block::FigureCaption { content } => shift_inlines(content, changes, deltas),
        Block::Styled {
            style: _,
            content,
            lists,
            line_break_before,
        } => {
            for frame in lists.iter_mut() {
                map_span(&mut frame.begin_span, changes, deltas)?;
            }
            if let Some(line_break) = line_break_before {
                map_span(&mut line_break.span, changes, deltas)?;
            }
            shift_inlines(content, changes, deltas)
        }
        Block::ListItem {
            level: _,
            label,
            content,
            extra_gap_before_pt: _,
            extra_gap_after_pt: _,
            leftmargin: _,
            widest_label: _,
            lists,
            item,
        } => {
            if let Some((_, span)) = label {
                map_span(span, changes, deltas)?;
            }
            for frame in lists.iter_mut() {
                map_span(&mut frame.begin_span, changes, deltas)?;
            }
            if let Some(crate::parser::ItemLabel::Explicit { content, span, .. }) = item {
                map_span(span, changes, deltas)?;
                shift_inlines(content, changes, deltas)?;
            }
            shift_inlines(content, changes, deltas)
        }
        Block::VSpace { .. } => Some(()),
        Block::Rule { span } => map_span(span, changes, deltas),
        Block::PageBreak => Some(()),
        Block::Verbatim { lines, span } => {
            for VerbatimLine { text: _, span } in lines.iter_mut() {
                map_span(span, changes, deltas)?;
            }
            map_span(span, changes, deltas)
        }
        Block::TableOfContents { span } => map_span(span, changes, deltas),
        Block::TitleBlock {
            title,
            authors,
            date,
        } => {
            shift_inlines(title, changes, deltas)?;
            shift_inlines(authors, changes, deltas)?;
            if let Some(date) = date {
                shift_inlines(date, changes, deltas)?;
            }
            Some(())
        }
        Block::VFill => Some(()),
        Block::Penalty {
            value: _,
            fil: _,
            span,
        } => map_span(span, changes, deltas),
        Block::LetterBlock {
            part: _,
            lines,
            extra_gap_after_pt: _,
            gap_before_pt: _,
            gap_after_pt: _,
            indent_pt: _,
            span,
        } => {
            for line in lines.iter_mut() {
                shift_inlines(line, changes, deltas)?;
            }
            map_span(span, changes, deltas)
        }
    }
}

fn shift_inlines(inlines: &mut [Inline], changes: &[ChangedBytes], deltas: &[isize]) -> Option<()> {
    for inline in inlines {
        match inline {
            Inline::Text {
                text: _,
                span,
                style: _,
                space_before: _,
            } => map_span(span, changes, deltas)?,
            Inline::LineBreak { span, skip_pt: _ } => map_span(span, changes, deltas)?,
            Inline::TextGlue { em: _, span } => map_span(span, changes, deltas)?,
            Inline::Math {
                list,
                display: _,
                number: _,
                number_span,
                span,
                space_before: _,
                color: _,
                color_ranges,
            } => {
                shift_math_list(list, changes, deltas)?;
                for (range, _) in color_ranges.iter_mut() {
                    map_span(range, changes, deltas)?;
                }
                if let Some(number_span) = number_span {
                    map_span(number_span, changes, deltas)?;
                }
                map_span(span, changes, deltas)?;
            }
            Inline::MathRows {
                rows,
                aligned: _,
                span,
            } => {
                for MathRow {
                    cells,
                    number: _,
                    span,
                    intertext,
                    shove: _,
                } in rows
                {
                    for cell in cells {
                        shift_math_list(cell, changes, deltas)?;
                    }
                    map_span(span, changes, deltas)?;
                    for text in intertext {
                        shift_inlines(&mut text.content, changes, deltas)?;
                        map_span(&mut text.span, changes, deltas)?;
                    }
                }
                map_span(span, changes, deltas)?;
            }
            Inline::Label {
                key: _,
                value: _,
                kind: _,
                span,
            } => map_span(span, changes, deltas)?,
            Inline::Reference {
                key: _,
                page: _,
                equation: _,
                span,
                space_before: _,
            } => map_span(span, changes, deltas)?,
            Inline::CleverReference { span, .. } => map_span(span, changes, deltas)?,
            Inline::HFill { span, .. } => map_span(span, changes, deltas)?,
            Inline::HSpace { pt: _, span } => map_span(span, changes, deltas)?,
            Inline::Footnote {
                number: _,
                span,
                mark: _,
                text,
                space_before: _,
            } => {
                map_span(span, changes, deltas)?;
                if let Some(text) = text {
                    shift_inlines(text, changes, deltas)?;
                }
            }
            Inline::Logo { span, .. } | Inline::Rule { span, .. } | Inline::Kern { span, .. } => {
                map_span(span, changes, deltas)?
            }
            Inline::Penalty {
                value: _,
                span,
                unskip: _,
            } => map_span(span, changes, deltas)?,
            Inline::PagePenalty { value: _, span } => map_span(span, changes, deltas)?,
            Inline::Discretionary {
                pre: _,
                post: _,
                nobreak: _,
                hyphen: _,
                span,
                style: _,
            } => map_span(span, changes, deltas)?,
            Inline::Tabular(table) => {
                // `Tabular` only offers a mapping copy; its nested inlines are
                // shifted through the same in-place walk.
                let shifted = table.try_map_spans(
                    &mut |span| mapped_span(span, changes, deltas),
                    &mut |inlines| {
                        let mut owned = inlines.to_vec();
                        shift_inlines(&mut owned, changes, deltas)?;
                        Some(owned)
                    },
                )?;
                **table = shifted;
            }
            Inline::Verbatim {
                text: _,
                span,
                space_before: _,
            } => map_span(span, changes, deltas)?,
            Inline::ColorBox(b) => {
                map_span(&mut b.span, changes, deltas)?;
                shift_inlines(&mut b.content, changes, deltas)?;
            }
            Inline::Underline(u) => {
                map_span(&mut u.span, changes, deltas)?;
                shift_inlines(&mut u.content, changes, deltas)?;
            }
            Inline::Graphic(graphic) => map_span(&mut graphic.span, changes, deltas)?,
            Inline::Transform(transform) => {
                let shifted = transform.try_map_spans(
                    &mut |span| mapped_span(span, changes, deltas),
                    &mut |inlines| {
                        let mut owned = inlines.to_vec();
                        shift_inlines(&mut owned, changes, deltas)?;
                        Some(owned)
                    },
                )?;
                **transform = shifted;
            }
        }
    }
    Some(())
}

fn shift_math_list(list: &mut MathList, changes: &[ChangedBytes], deltas: &[isize]) -> Option<()> {
    for MathAtom {
        nucleus,
        span,
        superscript,
        subscript,
        class_override: _,
        width_em: _,
        ams_symbol: _,
    } in &mut list.atoms
    {
        match nucleus {
            Nucleus::Symbol(_) | Nucleus::Text(_) | Nucleus::Bold(_) => {}
            Nucleus::TextRun(pieces) => {
                for piece in pieces {
                    if let crate::math::TextPiece::Math(list) = piece {
                        shift_math_list(list, changes, deltas)?;
                    }
                }
            }
            Nucleus::SizedDelimiter { .. } => {}
            Nucleus::Space { .. } => {}
            Nucleus::Rule(_) => {}
            Nucleus::Fraction {
                numerator,
                denominator,
            } => {
                shift_math_list(numerator, changes, deltas)?;
                shift_math_list(denominator, changes, deltas)?;
            }
            Nucleus::Radical(list) => shift_math_list(list, changes, deltas)?,
            Nucleus::Framed { body, frame: _ } => shift_math_list(body, changes, deltas)?,
            Nucleus::Stacked { base, over, under } => {
                shift_math_list(base, changes, deltas)?;
                if let Some(list) = over {
                    shift_math_list(list, changes, deltas)?;
                }
                if let Some(list) = under {
                    shift_math_list(list, changes, deltas)?;
                }
            }
            Nucleus::Matrix {
                rows,
                columns: _,
                left: _,
                right: _,
            } => {
                for cell in rows.iter_mut().flatten() {
                    shift_math_list(cell, changes, deltas)?;
                }
            }
            Nucleus::Accent { accent: _, body } => shift_math_list(body, changes, deltas)?,
            Nucleus::Group(inner) => shift_math_list(inner, changes, deltas)?,
            Nucleus::GenFraction {
                numerator,
                denominator,
                ..
            } => {
                shift_math_list(numerator, changes, deltas)?;
                shift_math_list(denominator, changes, deltas)?;
            }
            Nucleus::Phantom { body, .. } | Nucleus::Operator { body, .. } => {
                shift_math_list(body, changes, deltas)?
            }
            Nucleus::ExtArrow { above, below, .. } => {
                shift_math_list(above, changes, deltas)?;
                shift_math_list(below, changes, deltas)?;
            }
            Nucleus::SubArray { rows, align: _ } => {
                for row in rows.iter_mut() {
                    shift_math_list(row, changes, deltas)?;
                }
            }
        }
        map_span(span, changes, deltas)?;
        // A present script that cannot be shifted fails the whole mapping, so
        // the block is recomputed rather than emitting a stale span.
        if let Some(list) = superscript {
            shift_math_list(list, changes, deltas)?;
        }
        if let Some(list) = subscript {
            shift_math_list(list, changes, deltas)?;
        }
    }
    Some(())
}

fn shift_placed(
    items: &mut [PlacedItem],
    changes: &[ChangedBytes],
    deltas: &[isize],
) -> Option<()> {
    for PlacedItem {
        page_index: _,
        item:
            TextItem {
                text: _,
                x_pt: _,
                baseline_y_pt: _,
                font_size_pt: _,
                span,
                font: _,
                rule: _,
            },
    } in items
    {
        map_span(span, changes, deltas)?;
    }
    Some(())
}

fn shift_diagnostics(
    diagnostics: &mut [Diagnostic],
    changes: &[ChangedBytes],
    deltas: &[isize],
) -> Option<()> {
    for Diagnostic {
        severity: _,
        message: _,
        span,
        recovery: _,
        code: _,
        suggestion: _,
        labels,
        notes: _,
        help,
    } in diagnostics
    {
        if let Some(span) = span {
            map_span(span, changes, deltas)?;
        }
        for label in labels {
            map_span(&mut label.span, changes, deltas)?;
        }
        if let Some(help) = help {
            if let Some(repl) = &mut help.replacement {
                map_span(&mut repl.span, changes, deltas)?;
            }
        }
    }
    Some(())
}

/// A cheap, collision-tolerant signature used only to narrow candidate search.
///
/// It is NOT an identity: two different blocks may share a signature. Full
/// structural equality still gates every reuse, so a collision costs one extra
/// comparison and can never produce a wrong result.
type BlockSignature = (usize, usize, usize, usize, usize);

fn block_signature(block: &Block) -> BlockSignature {
    let inlines: &[Inline] = match block {
        Block::Paragraph(inlines) => inlines,
        Block::ListItem { content, .. } => content,
        Block::Heading { content, .. } => content,
        Block::FigureCaption { content } => content,
        Block::Styled { content, .. } => content,
        Block::VSpace { .. }
        | Block::Rule { .. }
        | Block::PageBreak
        | Block::Verbatim { .. }
        | Block::TableOfContents { .. }
        | Block::VFill
        | Block::Penalty { .. } => &[],
        // Signature only (see the doc comment above): the first line is
        // enough to narrow the candidate set, and `shift_block`'s full
        // equality check still gates every reuse.
        Block::LetterBlock { lines, .. } => lines.first().map_or(&[][..], |line| &line[..]),
        // Signature only, not identity (see the doc comment above): using
        // just `title` here (never `authors`/`date`) can only widen the
        // candidate set on an author/date-only edit, never produce a wrong
        // reuse, since `shift_block`'s full equality check still gates that.
        Block::TitleBlock { title, .. } => title,
    };
    let span_of = |inline: &Inline| match inline {
        Inline::Text { span, .. } => *span,
        Inline::LineBreak { span, .. } => *span,
        Inline::TextGlue { span, .. } => *span,
        Inline::Math { span, .. } => *span,
        Inline::MathRows { span, .. } => *span,
        Inline::Label { span, .. } => *span,
        Inline::Reference { span, .. } => *span,
        Inline::CleverReference { span, .. } => *span,
        Inline::HFill { span, .. } => *span,
        Inline::HSpace { span, .. } => *span,
        Inline::Footnote { span, .. } => *span,
        Inline::Tabular(table) => table.span,
        Inline::Verbatim { span, .. } => *span,
        Inline::ColorBox(b) => b.span,
        Inline::Underline(u) => u.span,
        Inline::Graphic(graphic) => graphic.span,
        Inline::Transform(transform) => transform.span,
        Inline::Logo { span, .. } | Inline::Rule { span, .. } | Inline::Kern { span, .. } => *span,
        Inline::Penalty { span, .. }
        | Inline::PagePenalty { span, .. }
        | Inline::Discretionary { span, .. } => *span,
    };
    let first = inlines.first().map(span_of);
    let last = inlines.last().map(span_of);
    (
        first.map_or(usize::MAX, |s| s.document.0),
        first.map_or(usize::MAX, |s| s.start),
        last.map_or(usize::MAX, |s| s.document.0),
        last.map_or(usize::MAX, |s| s.end),
        inlines.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_byte_identical_to_full(
        result: &IncrementalResult,
        text: &str,
        constraints: LayoutConstraints,
    ) {
        let full = compile_full(text, constraints);
        let incremental_bytes = format!("{:#?}", result.output).into_bytes();
        let full_bytes = format!("{full:#?}").into_bytes();
        assert_eq!(incremental_bytes, full_bytes);
    }

    fn compile_edit(old: &str, new: &str) -> IncrementalResult {
        let constraints = LayoutConstraints::default();
        let mut session = Session::new();
        let cold = session.compile(old, constraints);
        assert_byte_identical_to_full(&cold, old, constraints);
        let result = session.compile(new, constraints);
        eprintln!("ReuseStats: {:?}", result.stats);
        assert_byte_identical_to_full(&result, new, constraints);
        result
    }

    #[test]
    fn edit_inside_one_paragraph_matches_clean_build() {
        let result = compile_edit(
            "Alpha beta.\n\nMiddle words here.\n\nOmega final.",
            "Alpha beta.\n\nMiddle changed here.\n\nOmega final.",
        );
        assert_eq!(result.stats.blocks_total, 3);
        assert!(result.stats.blocks_reused >= 2);
    }

    #[test]
    fn edit_inside_verbatim_matches_clean_build() {
        let result = compile_edit(
            "Intro.\n\n\\begin{verbatim}\nold line\n\\end{verbatim}\n\nTail.",
            "Intro.\n\n\\begin{verbatim}\nnew line\n\\end{verbatim}\n\nTail.",
        );
        assert_eq!(result.stats.blocks_total, 3);
    }

    #[test]
    fn edit_before_verbatim_still_reuses_it() {
        let result = compile_edit(
            "Intro.\n\n\\begin{verbatim}\nkept line\n\\end{verbatim}\n\nTail.",
            "Intro changed.\n\n\\begin{verbatim}\nkept line\n\\end{verbatim}\n\nTail.",
        );
        assert!(result.stats.blocks_reused >= 1);
    }

    #[test]
    fn heading_and_math_use_the_same_cursor_as_clean_layout() {
        let result = compile_edit(
            "\\section{Measured $x^2$ heading}\n\nBody with $\\frac{1}{2}$.\n\nTail.",
            "\\section{Measured $x^2$ heading}\n\nEdited body with $\\frac{1}{2}$.\n\nTail.",
        );
        assert!(result.stats.blocks_reused >= 1);
    }

    #[test]
    fn edits_at_start_and_end_match_clean_build() {
        let constraints = LayoutConstraints::default();
        let original = "First paragraph.\n\nSecond paragraph.\n\nLast paragraph.";
        let at_start = "New first paragraph.\n\nSecond paragraph.\n\nLast paragraph.";
        let at_end = "New first paragraph.\n\nSecond paragraph.\n\nLast paragraph changed.";
        let mut session = Session::new();
        session.compile(original, constraints);
        let start_result = session.compile(at_start, constraints);
        eprintln!("start ReuseStats: {:?}", start_result.stats);
        assert_byte_identical_to_full(&start_result, at_start, constraints);
        let end_result = session.compile(at_end, constraints);
        eprintln!("end ReuseStats: {:?}", end_result.stats);
        assert_byte_identical_to_full(&end_result, at_end, constraints);
    }

    #[test]
    fn edit_spanning_paragraph_boundary_matches_clean_build() {
        let result = compile_edit(
            "One paragraph.\n\nTwo paragraph.\n\nThree paragraph.",
            "One paragraph joined to Two paragraph.\n\nThree paragraph.",
        );
        assert_eq!(result.stats.blocks_total, 2);
    }

    #[test]
    fn justified_multi_line_paragraphs_reuse_identically() {
        let body = "Several words wrap across lines and get justified. ".repeat(6);
        let old = format!("{body}\n\n{body}\n\n{body}");
        let new = format!("{body}\n\nEdited {body}\n\n{body}");
        let result = compile_edit(&old, &new);
        assert!(result.stats.blocks_reused >= 1);
    }

    #[test]
    fn multibyte_insertion_shifts_later_spans_by_byte_delta() {
        let new = "A café closes.\n\nLater paragraph stays.";
        let result = compile_edit("A cafe closes.\n\nLater paragraph stays.", new);
        let later = result
            .output
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.text == "Later")
            .expect("later item");
        assert_eq!(&new[later.span.start..later.span.end], "Later");
        assert!(result.stats.blocks_reused >= 1);
    }

    #[test]
    fn changed_line_count_repositions_later_blocks() {
        let short = "short opening.\n\nLater paragraph.";
        let long_words = "wide ".repeat(120);
        let long = format!("{}\n\nLater paragraph.", long_words);
        let result = compile_edit(short, &long);
        assert_eq!(result.stats.blocks_reused, 0);
        let later = result
            .output
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.text == "Later")
            .expect("later item");
        assert!(later.baseline_y_pt > 100.0);
    }

    #[test]
    fn redefining_macro_invalidates_untouched_reader() {
        let old = "\\newcommand{\\term}{base}\\renewcommand{\\term}{old}\n\nPlain before.\n\nUse \\term here.";
        let new = "\\newcommand{\\term}{base}\\renewcommand{\\term}{new}\n\nPlain before.\n\nUse \\term here.";
        let result = compile_edit(old, new);
        assert!(result
            .output
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .any(|item| item.text == "new"));
        assert_eq!(result.stats.blocks_reused, 1);
        assert_eq!(result.stats.blocks_recomputed, 1);
    }

    #[test]
    fn scoped_macro_change_does_not_invalidate_global_reader() {
        let old = "\\newcommand{\\term}{global}{\\renewcommand{\\term}{local}Scoped \\term.}\n\nGlobal \\term.";
        let new = "\\newcommand{\\term}{global}{\\renewcommand{\\term}{inner}Scoped \\term.}\n\nGlobal \\term.";
        let result = compile_edit(old, new);
        assert_eq!(result.stats.blocks_total, 2);
        assert_eq!(result.stats.blocks_reused, 1);
        assert_eq!(result.stats.blocks_recomputed, 1);
    }

    #[test]
    fn adding_usepackage_forces_full_recompile() {
        let old = "\\documentclass{article}\n\\begin{document}\nOne.\n\nTwo.\n\\end{document}";
        let new = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\nOne.\n\nTwo.\n\\end{document}";
        let result = compile_edit(old, new);
        assert!(result.stats.full_recompile);
        assert_eq!(result.stats.blocks_reused, 0);
        assert_eq!(result.stats.blocks_recomputed, 2);
    }

    #[test]
    fn malformed_edit_has_partial_output_and_source_diagnostics() {
        let new = "Good text {unclosed and \\unknown here.\n\nLater text.";
        let result = compile_edit("Good text.\n\nLater text.", new);
        assert!(result.stats.full_recompile);
        assert!(!result.output.pages[0].items.is_empty());
        assert!(result.output.diagnostics.iter().all(|diag| {
            diag.span.is_some()
                && diag
                    .recovery
                    .as_deref()
                    .is_some_and(|text| !text.is_empty())
        }));
    }

    #[test]
    fn layout_constraint_change_forces_full_recompile() {
        let text = "One paragraph.\n\nTwo paragraph.";
        let mut session = Session::new();
        session.compile(text, LayoutConstraints::default());
        let constraints = LayoutConstraints {
            font_size_pt: 13.0,
            measure_pt: 320.0,
            parskip_pt: None,
        };
        let result = session.compile(text, constraints);
        eprintln!("constraint ReuseStats: {:?}", result.stats);
        assert!(result.stats.full_recompile);
        assert_byte_identical_to_full(&result, text, constraints);
    }

    #[test]
    fn reused_hfill_block_keeps_its_resolved_right_edge() {
        let result = compile_edit(
            "left\\hfill right\n\nTail.",
            "left\\hfill right\n\nTail changed.",
        );
        assert!(result.stats.blocks_reused >= 1, "{:?}", result.stats);
        let right = result
            .output
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.text == "right")
            .expect("right-hand text");
        let width = layout::text_width("right", 12.0, layout::Font::TimesRoman);
        assert!((right.x_pt + width - 540.0).abs() < 0.02);
    }

    #[test]
    fn reused_tabular_block_shifts_every_nested_span() {
        let table = "\\begin{tabular}{|l|c|}\\hline A & $x$ \\\\ \\multicolumn{2}{@{:}c|}{B}\\\\\\hline\\end{tabular}";
        let result = compile_edit(
            &format!("First words.\n\n{table}\n\nTail."),
            &format!("First changed words.\n\n{table}\n\nTail."),
        );
        assert!(result.stats.blocks_reused >= 2);
    }

    fn alpah_diagnostic(output: &CompileOutput) -> &crate::diagnostics::Diagnostic {
        output
            .diagnostics
            .iter()
            .find(|d| d.message.contains("\\alpah") || d.message.contains("\\alpax"))
            .expect("unknown-command diagnostic")
    }

    #[test]
    fn shift_diagnostics_maps_label_and_replacement_spans() {
        use crate::diagnostics::Diagnostic;
        let span = Span::new(20, 26);
        let mut diagnostics = vec![Diagnostic::error("\\alpah is not supported", Some(span), None)
            .with_label(span, "this command", true)
            .with_help("did you mean \\alpha?")
            .with_replacement(span, "\\alpha")];
        let before = [ChangedBytes { old: 0..5, new: 0..10 }];
        assert!(shift_diagnostics(&mut diagnostics, &before, &[5]).is_some());
        let shifted = Span::new(25, 31);
        assert_eq!(diagnostics[0].span, Some(shifted));
        assert_eq!(diagnostics[0].labels[0].span, shifted);
        assert_eq!(
            diagnostics[0]
                .help
                .as_ref()
                .unwrap()
                .replacement
                .as_ref()
                .unwrap()
                .span,
            shifted
        );

        let inside = [ChangedBytes { old: 25..31, new: 25..32 }];
        assert!(shift_diagnostics(&mut diagnostics, &inside, &[1]).is_none());
    }

    #[test]
    fn edit_before_a_labelled_help_replacement_shifts_both_spans() {
        let old = "First paragraph.\n\nLater \\alpah here.";
        let new = "First changed paragraph.\n\nLater \\alpah here.";
        let result = compile_edit(old, new);
        // Parser diagnostics force a full recompile today; output spans must
        // still match a clean build (compile_edit checks that) and move by the
        // same delta shift_diagnostics would apply.
        assert!(result.stats.full_recompile, "{:?}", result.stats);
        let diag = alpah_diagnostic(&result.output);
        let span = diag.span.expect("command span");
        assert_eq!(&new[span.start..span.end], "\\alpah");
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(
            &new[diag.labels[0].span.start..diag.labels[0].span.end],
            "\\alpah"
        );
        let repl = diag
            .help
            .as_ref()
            .and_then(|h| h.replacement.as_ref())
            .expect("help.replacement");
        assert_eq!(&new[repl.span.start..repl.span.end], "\\alpah");
        assert_eq!(repl.text, "\\alpha");
        let delta = new.len() as isize - old.len() as isize;
        let old_output = compile_full(old, LayoutConstraints::default());
        let old_diag = alpah_diagnostic(&old_output);
        let old_span = old_diag.span.expect("old span");
        assert_eq!(span.start, (old_span.start as isize + delta) as usize);
        assert_eq!(
            diag.labels[0].span.start,
            (old_diag.labels[0].span.start as isize + delta) as usize
        );
        assert_eq!(
            repl.span.start,
            (old_diag
                .help
                .as_ref()
                .unwrap()
                .replacement
                .as_ref()
                .unwrap()
                .span
                .start as isize
                + delta) as usize
        );
    }

    #[test]
    fn edit_inside_a_labelled_help_replacement_recomputes() {
        let old = "First paragraph.\n\nLater \\alpah here.";
        let new = "First paragraph.\n\nLater \\alpax here.";
        let result = compile_edit(old, new);
        assert!(result.stats.full_recompile, "{:?}", result.stats);
        assert!(result.stats.blocks_recomputed >= 1, "{:?}", result.stats);
        let diag = alpah_diagnostic(&result.output);
        let span = diag.span.expect("command span");
        assert_eq!(&new[span.start..span.end], "\\alpax");
        assert_eq!(
            &new[diag.labels[0].span.start..diag.labels[0].span.end],
            "\\alpax"
        );
        let repl = diag
            .help
            .as_ref()
            .and_then(|h| h.replacement.as_ref())
            .expect("help.replacement");
        assert_eq!(&new[repl.span.start..repl.span.end], "\\alpax");
        assert_eq!(repl.text, "\\alpha");
    }
}
