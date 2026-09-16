import Foundation
import FlashTeXProtocol

/// Consumer of `display-list-v2-window`
/// (protocol/proposals/display-list-v2-window.md, producer PR #294): a
/// bounded resident page window, negotiated per request next to
/// `display-list-v2`, so a document whose full display list (or full v1
/// fallback) exceeds the producer's reply limit still gets a painted reply.
///
/// The window is engaged on demand, never by default: an unwindowed reply is
/// complete (delta base, export authority), a windowed reply is an
/// incomplete view (§4.1) — so the shell asks for a window only after the
/// producer has said the unwindowed reply cannot exist ("over the …-byte
/// reply limit" failure, or a `display_list_declined` diagnostic). Once
/// engaged, the pane's scroll position drives the window: leaving the served
/// window's comfortable interior re-requests the same revision anchored at
/// the viewer (a new request id, exactly like a capability switch).
///
/// Composition (§7): `-window` rides with `-only` (both are needed for a
/// long document to be repliable at all) and is mutually exclusive with
/// `-delta` in r1 — a windowed producer cannot digest pages it has not
/// materialised — so an engaged window suppresses the delta acknowledgement,
/// and a windowed frame is never installed as a delta base.
enum V2Window {
    static let capability = RenderingV2.windowCapability
    /// On unless `FLASHTEX_DISPLAY_V2_WINDOW=0` (same shape as the delta and
    /// v2-only switches).
    static var enabled: Bool { ProcessInfo.processInfo.environment["FLASHTEX_DISPLAY_V2_WINDOW"] != "0" }
    /// Pages per requested window. 16 is the producer lane's measured shape
    /// (≈6 MB of the 500 KB corpus case against the 16 MiB line limit); the
    /// producer narrows an over-limit window and serves it (§8), so this is a
    /// viewing bound, not a byte bound.
    static let pageCount = 16
    /// Re-request when the anchored page comes within this many pages of a
    /// served window edge that is not also a document edge.
    static let margin = 4

    /// Pure window state and decisions (unit-testable; the shell owns one).
    struct Controller: Equatable {
        /// Windowing engaged for this producer attachment. Never on by
        /// default; set by an over-limit failure/decline, cleared when a
        /// positioned request comes back without the echoed capability
        /// (a producer that cannot window is never asked twice per scroll).
        private(set) var engaged = false
        /// 1-based page the next requested window is anchored around.
        private(set) var anchorPage = 1
        /// The page under the viewport's top edge, as last reported.
        private(set) var visiblePage = 1
        /// The `display_list_window` sent with the request whose result is
        /// currently applied (nil = that request was unwindowed).
        var applied: RuntimeV1.CompileRequest.DisplayListWindow?

        /// The `display_list_window` the next compile request should carry.
        /// The window is anchored a few pages above the viewer so a short
        /// upward scroll stays resident; the producer clamps a window that
        /// runs past the last page (§4).
        var desired: RuntimeV1.CompileRequest.DisplayListWindow? {
            guard engaged else { return nil }
            return .init(firstPage: max(1, anchorPage - V2Window.margin), pageCount: V2Window.pageCount)
        }

        mutating func engage() {
            engaged = true
            anchorPage = visiblePage
        }

        mutating func disengage() {
            engaged = false
            applied = nil
        }

        /// The pane reported the page under the viewport's top edge. Returns
        /// true when an engaged window should be re-requested — the viewer
        /// left the served window's comfortable interior (`served` is the
        /// window object of the frame on screen; nil while none is).
        mutating func sawVisiblePage(_ page: Int, served: RenderingV2.Window?) -> Bool {
            visiblePage = max(1, page)
            guard engaged else { return false }
            guard let served else { return false } // nothing windowed on screen yet; the in-flight request will answer
            guard !comfortableRange(served: served).contains(visiblePage) else { return false }
            anchorPage = visiblePage
            return true
        }

        /// The served range minus `margin` at each edge, except an edge that
        /// touches the document's (page 1 stays comfortable at the top of the
        /// first window; the last page at the end of the last).
        func comfortableRange(served: RenderingV2.Window) -> ClosedRange<Int> {
            var lo = served.firstPage
            var hi = served.firstPage + served.pageCount - 1
            if lo > 1 { lo += V2Window.margin }
            if hi < served.documentPageCount { hi -= V2Window.margin }
            return lo > hi ? served.pageRange : lo...hi
        }
    }

    /// The producer refused the whole reply: `status: failed`, no pages, and
    /// the diagnostic naming the reply limit ("compile_result would be N
    /// bytes for P pages, over the L-byte reply limit"; `protocol.rs`).
    static func overLimitFailure(_ result: RuntimeV1.CompileResult) -> Bool {
        guard result.status == .failed, result.pages.isEmpty else { return false }
        return result.diagnostics.contains { $0.message.contains("over the") && $0.message.contains("-byte reply limit") }
    }

    /// The reply survived but its `display_list` sibling was declined as
    /// over-limit (`display_list_declined`, `protocol.rs`): the v2 pane has
    /// nothing to paint, which a window fixes.
    static func declinedSibling(_ result: RuntimeV1.CompileResult) -> Bool {
        result.diagnostics.contains { $0.code == "display_list_declined" }
    }
}

extension ShellModel {
    /// The `display_list_window` the next compile request should carry, or
    /// nil when windowing is off, not engaged, or the request will not ask
    /// for `display-list-v2` at all.
    func v2WindowDesired(capabilities: [String]) -> RuntimeV1.CompileRequest.DisplayListWindow? {
        guard V2Window.enabled, previewV2, capabilities.contains(V2Live.capability) else { return nil }
        return v2Window.desired
    }

    /// True when the applied result was requested under a window other than
    /// the one now desired: `compile()` must re-request even though the
    /// buffers and the capability set are unchanged (engagement after a
    /// failure, or a scroll that left the served window).
    var v2WindowResendNeeded: Bool {
        v2WindowDesired(capabilities: requestedLayoutCapabilities) != v2Window.applied
    }

    /// Applied-result hook (`handle(.result)`): binds the applied request's
    /// window, engages windowing when the producer said the unwindowed reply
    /// cannot exist, and disengages when a positioned request came back
    /// without the echoed capability (the producer cannot window; asking
    /// again on every scroll would be a loop). Returns whether the same
    /// buffers should be re-requested under the now-desired window.
    func v2WindowNote(applied: RuntimeV1.CompileResult, sentWindow: RuntimeV1.CompileRequest.DisplayListWindow?) -> Bool {
        v2Window.applied = sentWindow
        let accepted = applied.layoutCapabilities ?? []
        if sentWindow != nil, !accepted.contains(V2Window.capability) {
            if v2Window.engaged {
                log("display-list-v2-window: the producer did not honour the positioned window; disengaging")
                v2Window.disengage()
            }
            return false
        }
        if !v2Window.engaged, V2Window.enabled, previewV2, requestedLayoutCapabilities.contains(V2Live.capability),
           V2Window.overLimitFailure(applied) || V2Window.declinedSibling(applied) {
            v2Window.engage()
            log("display-list-v2-window: engaging a \(V2Window.pageCount)-page window at page \(v2Window.visiblePage) — the unwindowed reply cannot fit the producer's line limit")
        }
        return v2WindowResendNeeded
    }

    /// The v2 pane's scroll anchor moved to `page` (PreviewAnchorKeeper).
    /// While a window is engaged and the viewer leaves the served window's
    /// interior, the same revision is re-requested anchored at the viewer.
    func v2WindowSawVisiblePage(_ page: Int) {
        // The preview HUD's page readout, for both panes (ShellModel).
        if previewVisiblePage != page { previewVisiblePage = page }
        let served = displayListV2?.frame?.list.window
        if v2Window.sawVisiblePage(page, served: served), workerAttached {
            compile()
        }
    }
}
