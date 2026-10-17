//! LOOKUP-SKIP's proofs (`flashtex_runtime::lookupproof`, moved there with
//! the resolver and the format cache, docs/design/xetex/PLAN.md §3.3), bound
//! to this engine's run: a check of the run's lookups starts from its
//! `-output-directory` and makes a lookup whose proof does not hold again as
//! the run made it (`system::lookup_again_deps`).

use crate::system::{self, Lookup};
pub use flashtex_runtime::lookupproof::{
    folded_listing, note, process_resolver, resolver_changed, Answer,
};

/// Checks a run's lookups against their proofs, for one pass over a
/// journal's or a key's lookups (`flashtex_runtime::lookupproof::Verifier`,
/// made with the run's output directory).
pub struct Verifier(flashtex_runtime::lookupproof::Verifier);

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for Verifier {
    type Target = flashtex_runtime::lookupproof::Verifier;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Verifier {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Verifier {
    pub fn new() -> Verifier {
        Verifier(flashtex_runtime::lookupproof::Verifier::new(
            system::output_directory(),
        ))
    }

    /// `system::lookup_again_deps`: what looking `l` up finds now, and what
    /// that answer depends on now, each directory with a signature taken
    /// before the listing the answer was checked against. From the proof
    /// where it holds, else by making the lookup again.
    pub fn lookup_again_deps(&mut self, l: &Lookup) -> Answer {
        match self.0.proven_again(&l.name, l.format, l.must_exist) {
            Some(r) => r,
            None => system::lookup_again_deps(l),
        }
    }
}
