import Foundation
@testable import FlashTeXMac

/// Test-only helpers that read `Completion.Vocabulary` (generated from the
/// compiler's `supported-latex.json` inventory) at runtime, so ordering and
/// membership assertions in `CompletionTests`, `SnippetTests`,
/// `SignatureHelpTests` and `CompletionLatencyTests` track the inventory as
/// it grows instead of pinning a snapshot of it (a hard-coded candidate list
/// or a hard-coded "first item" name).
///
/// `names(forPrefix:in:)` mirrors only the two table-order rules documented
/// on `Completion.suggestions` — the name spelled exactly as typed first (if
/// it exists in `supported`), then vocabulary table order — not the rest of
/// `commandSuggestions` (open-environment closers, project-declared
/// overrides, document-typed fallbacks, the fuzzy subsequence fallback),
/// which the tests that need those exercise directly against the real
/// vocabulary text the scenario carries.
///
/// For a test whose whole point is the ranking *rule* rather than the
/// compiler's actual vocabulary, pass a small synthetic `supported` list
/// (through this helper or directly as `Completion.suggestions(supported:)`)
/// so the assertion never depends on what the inventory happens to contain.
enum CompletionTestVocabulary {
    /// Command names `Completion.suggestions` offers for `prefix` against
    /// `supported` (default: the full compiler vocabulary, in table order),
    /// in rank order.
    static func names(forPrefix prefix: String, in supported: [String] = Completion.defaultSupported) -> [String] {
        var offered = Set<String>()
        var out: [String] = []
        let exact = supported.contains(prefix) ? [prefix] : []
        for name in exact + supported where name.hasPrefix(prefix) && offered.insert(name).inserted {
            out.append(name)
        }
        return out
    }

    /// `names(forPrefix:in:)` rendered as popup labels (`\section{...}`); a
    /// name outside `Completion.Vocabulary` (a synthetic `supported` entry)
    /// falls back to the bare `\name` label, matching `Vocabulary.generic`.
    static func labels(forPrefix prefix: String, in supported: [String] = Completion.defaultSupported) -> [String] {
        names(forPrefix: prefix, in: supported).map { Completion.Vocabulary.byName[$0]?.label ?? ("\\" + $0) }
    }

    /// `names(forPrefix:in:)` rendered as insertion text (`\section`, no
    /// argument shape) — what AppKit's `NSTextView` completion list carries.
    static func insertTexts(forPrefix prefix: String, in supported: [String] = Completion.defaultSupported) -> [String] {
        names(forPrefix: prefix, in: supported).map { "\\" + $0 }
    }
}
