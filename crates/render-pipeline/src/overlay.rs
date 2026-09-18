//! beamer overlays (issue #944, Tier 2): a frame with overlay
//! specifications is set once per slide.
//!
//! `beamerbaseframe.sty` runs the frame's material once per slide
//! (`\beamer@slideinframe` 1..N, N the largest slide any specification
//! names -- the compiler's `BeamerFrameBegin::slides`), each slide a page of
//! its own with the same frametitle. The compiler emits the material once,
//! with zero-width markers ([`AItem::Overlay`]); [`expand_frames`] replaces
//! every frame of the adapted block list by one copy per slide, each copy
//! filtered for its slide:
//!
//! - `\only`/`onlyenv` on a slide the specification does not select: the
//!   material is **omitted**, so the page reflows (`\beamer@doifnotinframe`
//!   is empty);
//! - `\uncover`, `\onslide`, `\pause`, `\item<spec>`, `\visible`,
//!   `\invisible` (inverted): the material is **covered** -- it is shaped,
//!   measured and broken like visible text so the space is kept, but its
//!   glyph runs are not emitted ([`TextStyle::hidden`], read by
//!   `typeset::assemble_block`). `\setbeamercovered{invisible}`, the
//!   default; `transparent` is not modelled;
//! - `\alert<spec>`: the alert colour (rgb 1,0,0 in the default theme) on
//!   the selected slides, the surrounding colour otherwise.
//!
//! An `\item<2->`'s label is covered with the item: its `ListGeom` takes the
//! state in force at the item's first material.
//!
//! Measured (pdflatex TeX Live 2026, `fixtures/real-world/beamer-overlays`):
//! covered text is written 2000 bp off the page (`\pgfsys@begininvisible`),
//! every visible word of a slide sits where it sits on the frame's other
//! slides, and the `\only` frame's alternatives each take their own space.

use flashtex_compiler::color::DeviceColor;
use flashtex_compiler::overlay::OverlayKind;

use crate::adapter::{Block, Item as AItem, OverlayMark, ParaPart, TextStyle};

/// beamer's `alerted text` colour in the default theme
/// (`beamercolorthemedefault.sty`: `\setbeamercolor{alerted text}{fg=red}`).
pub fn alert_color() -> DeviceColor {
    DeviceColor::RED
}

/// One open overlay scope on the slide being set.
#[derive(Debug, Clone, Copy)]
struct Scope {
    kind: OverlayKind,
    /// The specification selects this slide (for `Invisible`, already
    /// inverted: `true` means shown).
    shown: bool,
}

/// The overlay state while one slide's blocks are walked in order.
#[derive(Debug, Clone)]
struct State {
    slide: u32,
    /// `\onslide<spec>`/`\pause` in force selects this slide.
    onslide: bool,
    stack: Vec<Scope>,
}

impl State {
    fn new(slide: u32) -> State {
        State { slide, onslide: true, stack: Vec::new() }
    }

    fn omitted(&self) -> bool {
        self.stack.iter().any(|s| s.kind == OverlayKind::Only && !s.shown)
    }

    fn covered(&self) -> bool {
        !self.onslide || self.stack.iter().any(|s| matches!(s.kind, OverlayKind::Cover | OverlayKind::Visible | OverlayKind::Invisible) && !s.shown)
    }

    fn alerted(&self) -> bool {
        self.stack.iter().any(|s| s.kind == OverlayKind::Alert && s.shown)
    }

    fn mark(&mut self, mark: &OverlayMark) {
        match mark {
            OverlayMark::Begin { spec, kind } => {
                let selected = spec.contains(self.slide);
                let shown = if *kind == OverlayKind::Invisible { !selected } else { selected };
                self.stack.push(Scope { kind: *kind, shown });
            }
            OverlayMark::End => {
                self.stack.pop();
            }
            OverlayMark::Onslide { spec } => self.onslide = spec.contains(self.slide),
        }
    }

    /// `style` as this state paints it.
    fn apply(&self, style: &mut TextStyle) {
        if self.covered() {
            style.hidden = true;
        }
        if self.alerted() {
            style.color = Some(alert_color());
        }
    }
}

/// Whether `spec`-free blocks need the walk at all: a frame without a
/// single marker and one slide is left as it is.
fn has_markers(items: &[AItem]) -> bool {
    items.iter().any(|i| match i {
        AItem::Overlay(_) => true,
        AItem::Footnote { text: Some(t), .. } | AItem::Marginpar { text: t, .. } | AItem::Lap { items: t } => has_markers(t),
        AItem::ColorBox(b) => has_markers(&b.items),
        AItem::Underline(u) => has_markers(&u.items),
        AItem::TextScript(t) => has_markers(&t.items),
        _ => false,
    })
}

fn block_has_markers(block: &Block) -> bool {
    match block {
        Block::Paragraph { parts, .. } => parts.iter().any(|p| match p {
            ParaPart::Lines(items) => has_markers(items),
            ParaPart::Rows { rows, .. } => rows.iter().any(|r| r.intertext.iter().any(|t| has_markers(&t.items))),
            ParaPart::Display { .. } => false,
        }),
        Block::Heading { items, .. } | Block::Chapter { items, .. } | Block::Part { items, .. } => has_markers(items),
        _ => false,
    }
}

/// Replaces every beamer frame of `blocks` (a `FrameBegin`..`FrameEnd`
/// run) by one copy per slide, each filtered for its slide. A frame with
/// one slide and no marker is left untouched.
pub fn expand_frames(blocks: &mut Vec<Block>) {
    if !blocks.iter().any(|b| matches!(b, Block::FrameBegin { .. })) {
        return;
    }
    let old = std::mem::take(blocks);
    let mut out: Vec<Block> = Vec::with_capacity(old.len());
    let mut i = 0;
    while i < old.len() {
        let Block::FrameBegin { slides, .. } = &old[i] else {
            out.push(old[i].clone());
            i += 1;
            continue;
        };
        let end = (i + 1..old.len())
            .find(|&j| matches!(old[j], Block::FrameEnd { .. } | Block::FrameBegin { .. }))
            .map_or(old.len() - 1, |j| if matches!(old[j], Block::FrameEnd { .. }) { j } else { j - 1 });
        let frame = &old[i..=end];
        let n = (*slides).max(1);
        if n == 1 && !frame.iter().any(block_has_markers) {
            out.extend_from_slice(frame);
        } else {
            for slide in 1..=n {
                out.extend(slide_view(frame, slide));
            }
        }
        i = end + 1;
    }
    *blocks = out;
}

/// The frame's blocks as slide `slide` sets them.
fn slide_view(frame: &[Block], slide: u32) -> Vec<Block> {
    let mut state = State::new(slide);
    let mut out = Vec::with_capacity(frame.len());
    for block in frame {
        let mut block = block.clone();
        let keep = match &mut block {
            Block::FrameBegin { slide: s, .. } => {
                *s = slide;
                true
            }
            Block::Paragraph { parts, list, .. } => {
                let mut first_material: Option<State> = None;
                let mut any_material = false;
                for part in parts.iter_mut() {
                    match part {
                        ParaPart::Lines(items) => {
                            transform_items(items, &mut state, &mut first_material);
                            any_material |= !items.is_empty();
                        }
                        ParaPart::Rows { rows, .. } => {
                            for row in rows.iter_mut() {
                                for t in row.intertext.iter_mut() {
                                    transform_items(&mut t.items, &mut state, &mut first_material);
                                }
                            }
                            any_material = true;
                        }
                        ParaPart::Display { .. } => any_material = true,
                    }
                }
                // The label takes the state at the item's first material
                // (`\item<2->` opens the item inside its `actionenv`); an
                // item whose whole content is omitted goes with it.
                let mut keep = any_material;
                if let Some(geom) = list.as_mut() {
                    if geom.label.is_some() {
                        let at = first_material.as_ref().unwrap_or(&state);
                        if at.omitted() {
                            keep = false;
                        } else {
                            keep = true;
                            geom.hidden = at.covered();
                        }
                    }
                }
                keep
            }
            Block::Heading { items, .. } | Block::Chapter { items, .. } | Block::Part { items, .. } => {
                let mut first = None;
                transform_items(items, &mut state, &mut first);
                true
            }
            _ => true,
        };
        if keep {
            out.push(block);
        }
    }
    out
}

/// Applies `state` to `items` in order: markers update it and are removed,
/// omitted material is dropped, covered and alerted material restyled.
/// `first_material` receives the state at the first item that is no marker.
fn transform_items(items: &mut Vec<AItem>, state: &mut State, first_material: &mut Option<State>) {
    let taken = std::mem::take(items);
    for mut item in taken {
        if let AItem::Overlay(mark) = &item {
            state.mark(mark);
            continue;
        }
        if first_material.is_none() {
            *first_material = Some(state.clone());
        }
        if state.omitted() {
            continue;
        }
        restyle(&mut item, state);
        items.push(item);
    }
}

fn restyle(item: &mut AItem, state: &mut State) {
    match item {
        AItem::Word(w) => {
            for seg in w.segments.iter_mut() {
                state.apply(&mut seg.style);
            }
        }
        AItem::Space { style, .. }
        | AItem::Quad { style, .. }
        | AItem::HFill { style, .. }
        | AItem::Logo { style, .. }
        | AItem::Rule { style, .. }
        | AItem::QedBox { style, .. }
        | AItem::Kern { style, .. } => state.apply(style),
        AItem::Footnote { text: Some(t), .. } | AItem::Marginpar { text: t, .. } | AItem::Lap { items: t } => {
            let mut first = None;
            transform_items(t, state, &mut first);
        }
        AItem::ColorBox(b) => {
            let mut first = None;
            transform_items(&mut b.items, state, &mut first);
        }
        AItem::Underline(u) => {
            let mut first = None;
            transform_items(&mut u.items, state, &mut first);
        }
        AItem::TextScript(t) => {
            let mut first = None;
            transform_items(&mut t.items, state, &mut first);
        }
        // Formulas, tables and graphics carry no text style: covered ones
        // are still painted (a known gap; the corpus has none).
        AItem::Math { .. }
        | AItem::Table(_)
        | AItem::Graphic { .. }
        | AItem::Footnote { text: None, .. }
        | AItem::LineBreak { .. }
        | AItem::Label { .. }
        | AItem::ItalicCorrection
        | AItem::NoteParBreak
        | AItem::HSpace { .. }
        | AItem::Overlay(_)
        | AItem::LeaveVmode => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{CharSrc, Segment, Word};
    use flashtex_compiler::overlay::OverlaySpec;
    use flashtex_compiler::Span;

    fn word(text: &str) -> AItem {
        AItem::Word(Word {
            segments: vec![Segment {
                text: text.to_string(),
                chars: vec![CharSrc { document: Default::default(), start: 0, end: text.len() }],
                style: TextStyle::default(),
            }],
        })
    }

    fn begin(kind: OverlayKind, raw: &str) -> AItem {
        let mut pauses = 1;
        AItem::Overlay(OverlayMark::Begin { spec: OverlaySpec::parse(raw, &mut pauses), kind })
    }

    fn para(items: Vec<AItem>) -> Block {
        Block::Paragraph {
            parts: vec![ParaPart::Lines(items)],
            indent: false,
            style: Default::default(),
            env_open: None,
            env_close: false,
            eject_before: false,
            vspace_before: 0.0,
            addvspace_before: 0.0,
            addvspace_flex: (0.0, 0.0),
            vspace_flex: (0.0, 0.0),
            endlist_adjust: 0.0,
            list: None,
            sized: None,
            leading_pt: None,
        }
    }

    fn frame(slides: u32, body: Vec<Block>) -> Vec<Block> {
        let mut v = vec![Block::FrameBegin {
            title: Vec::new(),
            subtitle: Vec::new(),
            align: flashtex_class_geometry::beamer::FrameAlign::Center,
            plain: false,
            slides,
            slide: 1,
            span: Span::new(0, 0),
        }];
        v.extend(body);
        v.push(Block::FrameEnd { span: Span::new(0, 0), addvspace_before: 0.0, addvspace_flex: (0.0, 0.0), vspace_before: 0.0 });
        v
    }

    fn words(block: &Block) -> Vec<(String, bool, bool)> {
        let Block::Paragraph { parts, .. } = block else { return Vec::new() };
        parts
            .iter()
            .flat_map(|p| match p {
                ParaPart::Lines(items) => items.iter().filter_map(|i| match i {
                    AItem::Word(w) => Some((w.text(), w.segments[0].style.hidden, w.segments[0].style.color.is_some())),
                    _ => None,
                }).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect()
    }

    #[test]
    fn only_omits_and_uncover_covers_per_slide() {
        let mut blocks = frame(
            2,
            vec![para(vec![
                begin(OverlayKind::Only, "1"),
                word("first"),
                AItem::Overlay(OverlayMark::End),
                begin(OverlayKind::Only, "2"),
                word("second"),
                AItem::Overlay(OverlayMark::End),
                begin(OverlayKind::Cover, "2-"),
                word("late"),
                AItem::Overlay(OverlayMark::End),
                begin(OverlayKind::Alert, "2"),
                word("hot"),
                AItem::Overlay(OverlayMark::End),
                AItem::Overlay(OverlayMark::Onslide { spec: OverlaySpec::from(2) }),
                word("tail"),
            ])],
        );
        expand_frames(&mut blocks);
        assert_eq!(blocks.len(), 6, "{blocks:?}");
        assert_eq!(
            words(&blocks[1]),
            vec![("first".into(), false, false), ("late".into(), true, false), ("hot".into(), false, false), ("tail".into(), true, false)]
        );
        assert_eq!(
            words(&blocks[4]),
            vec![("second".into(), false, false), ("late".into(), false, false), ("hot".into(), false, true), ("tail".into(), false, false)]
        );
        assert!(matches!(blocks[3], Block::FrameBegin { slide: 2, .. }));
    }

    #[test]
    fn a_plain_frame_is_left_alone() {
        let mut blocks = frame(1, vec![para(vec![word("only")])]);
        let before = blocks.clone();
        expand_frames(&mut blocks);
        assert_eq!(blocks, before);
    }
}
