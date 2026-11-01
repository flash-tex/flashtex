import Foundation

/// Per-request layout capability negotiation (runtime-v1-layout-capabilities.md).
/// A result is bound to the capability set of *its own* request; the consumer
/// never infers support from a response and never lets a late response for a
/// different request change the renderer mode.
public struct LayoutNegotiation: Equatable {
    /// Capabilities the request carried (`layout_capabilities`), in order.
    public var requested: [String]
    /// Capabilities the producer accepted for this result (empty = legacy route).
    public var accepted: [String]

    public init(requested: [String] = [], accepted: [String] = []) {
        self.requested = requested; self.accepted = accepted
    }

    public static let legacy = LayoutNegotiation()

    /// True when at least one capability was accepted: typed rules/font hints
    /// may appear and unknown primitives are reported instead of skipped.
    public var isNegotiated: Bool { !accepted.isEmpty }
    public var rulesAccepted: Bool { accepted.contains(RuntimeV1.LayoutCapabilities.rulesV1) }
    public var fontHintsAccepted: Bool { accepted.contains(RuntimeV1.LayoutCapabilities.fontHintsV1) }
    /// Requested but not accepted — reported explicitly, never guessed around.
    public var missing: [String] { requested.filter { !accepted.contains($0) } }

    /// Checks a result against the capabilities its request carried. Returns
    /// a violation description (the result must be rejected) or nil.
    ///
    /// Violations: an accepted capability that was not requested; a `rule`
    /// item without accepted `rules-v1`; a text `font` hint without accepted
    /// `font-hints-v1`. Acceptance is what the producer echoed — a shape the
    /// producer did not negotiate is rejected even if the client asked for it.
    public static func violation(in result: RuntimeV1.CompileResult, requested: [String]) -> String? {
        let accepted = result.layoutCapabilities ?? []
        if let extra = accepted.first(where: { !requested.contains($0) }) {
            return "accepted capability \(extra) was not requested (requested: \(describe(requested)))"
        }
        let negotiation = LayoutNegotiation(requested: requested, accepted: accepted)
        for page in result.pages {
            for item in page.items {
                switch item {
                case .rule where !negotiation.rulesAccepted:
                    return "page \(page.number) contains a rule item but \(RuntimeV1.LayoutCapabilities.rulesV1) was not accepted (accepted: \(describe(accepted)))"
                case .text(let t) where t.font != nil && !negotiation.fontHintsAccepted:
                    return "page \(page.number) text item carries a font hint but \(RuntimeV1.LayoutCapabilities.fontHintsV1) was not accepted (accepted: \(describe(accepted)))"
                default: continue
                }
            }
        }
        return nil
    }

    /// Source-aware error diagnostics for every unknown primitive kind. Empty
    /// on the legacy route, where unknown kinds are still skipped silently.
    public static func unsupportedPrimitiveDiagnostics(in result: RuntimeV1.CompileResult,
                                                       negotiation: LayoutNegotiation) -> [RuntimeV1.Diagnostic] {
        guard negotiation.isNegotiated else { return [] }
        var out: [RuntimeV1.Diagnostic] = []
        for page in result.pages {
            for case .unknown(let kind, let source) in page.items {
                let at = source.map { " at \($0.path) bytes \($0.startByte)..<\($0.endByte)" } ?? " (no source mapping)"
                out.append(.init(severity: .error,
                                 message: "unsupported layout primitive '\(kind)'\(at) on page \(page.number)",
                                 source: source,
                                 recovery: "item not drawn; the rest of the page is shown"))
            }
        }
        return out
    }

    /// "none" or the comma-joined set, for log and banner text.
    public static func describe(_ caps: [String]) -> String { caps.isEmpty ? "none" : caps.joined(separator: ", ") }
}
