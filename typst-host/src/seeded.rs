//! The seeded 1-pass loop (DESIGN.md §15.3).
//!
//! `typst::compile` lays the document out until its introspection is
//! stable, starting from an empty introspector: about 4 layout iterations
//! per keystroke. This runs the same fixed-point loop from public crates
//! (`typst_eval::eval`, `Engine`, `Output::create`, `comemo::Constraint`),
//! but seeds the first iteration with the **previous keystroke's**
//! document's introspector: after a small edit it is already stable, so one
//! iteration suffices. Measured by Track A: 192/192 edits page-identical to
//! the standard compile at about ⅓ of the time and memory.
//!
//! It re-implements a private function (`typst::compile_impl` of 0.15.1),
//! so it must be re-verified on every Typst release (the
//! `typst_version_matches_the_lockfile` test pins the version this was
//! checked against), and the host re-checks it against the standard
//! compile ([`standard`]) when idle. Anything unusual -- no previous
//! document, an error anywhere, delayed errors, no convergence within
//! Typst's iteration limit -- falls back to the standard compile, so its
//! diagnostics are exactly Typst's.

use comemo::Track;
use typst::diag::{SourceResult, Warned};
use typst::engine::{Engine, Route, Sink, Traced};
use typst::foundations::{Output, StyleChain, Target, TargetElem};
use typst::introspection::{Introspector, MAX_ITERS};
use typst::utils::Protected;
use typst::World;
use typst_layout::PagedDocument;

/// A compile's result and how it was obtained.
pub struct Compiled {
    pub output: Warned<SourceResult<PagedDocument>>,
    /// Layout iterations the seeded loop ran (0: the standard compile ran).
    pub iterations: usize,
    /// The standard compile ran (no seed, or the seeded loop declined).
    pub standard: bool,
}

/// `typst::compile`, unchanged.
pub fn standard(world: &dyn World) -> Compiled {
    Compiled {
        output: typst::compile::<PagedDocument>(world),
        iterations: 0,
        standard: true,
    }
}

/// The seeded loop, seeded with `prev`'s introspector; the standard compile
/// when there is no `prev` or anything is not the plain successful case.
pub fn compile(world: &dyn World, prev: Option<&PagedDocument>) -> Compiled {
    let Some(prev) = prev else {
        return standard(world);
    };
    match seeded(world, prev) {
        Some((doc, warnings, iterations)) => Compiled {
            output: Warned {
                output: Ok(doc),
                warnings,
            },
            iterations,
            standard: false,
        },
        None => standard(world),
    }
}

type Warnings = typst::ecow::EcoVec<typst::diag::SourceDiagnostic>;

fn seeded(world: &dyn World, prev: &PagedDocument) -> Option<(PagedDocument, Warnings, usize)> {
    let world = world.track();
    let traced = Traced::default();
    let mut sink = Sink::new();
    let library = world.library();
    let base = StyleChain::new(&library.styles);
    let target = TargetElem::target.set(Target::Paged).wrap();
    let styles = base.chain(&target);
    let main = world.source(world.main()).ok()?;
    let content = typst_eval::eval(
        world,
        library,
        traced.track(),
        sink.track_mut(),
        Route::default().track(),
        &main,
    )
    .ok()?
    .content();
    let mut history: Vec<PagedDocument> = Vec::new();
    loop {
        let introspector: &dyn Introspector = match history.last() {
            Some(d) => <PagedDocument as Output>::introspector(d),
            None => <PagedDocument as Output>::introspector(prev),
        };
        let constraint = comemo::Constraint::new();
        let mut subsink = Sink::new();
        let mut engine = Engine {
            library,
            world,
            introspector: Protected::new(introspector.track_with(&constraint)),
            traced: traced.track(),
            sink: subsink.track_mut(),
            route: Route::default(),
        };
        let doc = PagedDocument::create(&mut engine, &content, styles).ok()?;
        if constraint.validate(<PagedDocument as Output>::introspector(&doc)) {
            sink.extend_from_sink(subsink);
            if !sink.delayed().is_empty() {
                return None;
            }
            let n = history.len() + 1;
            return Some((doc, sink.warnings(), n));
        }
        // Typst gives up (with warnings) after MAX_ITERS; the standard
        // compile reproduces that exactly.
        if history.len() + 1 >= MAX_ITERS {
            return None;
        }
        history.push(doc);
    }
}
