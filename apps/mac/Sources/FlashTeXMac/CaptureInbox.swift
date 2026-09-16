import AppKit
import Combine
import Foundation
import SwiftUI
import FlashTeXProtocol

/// The Captures inspector (lane mac-capture-fluid): every capture a paired
/// companion sent this session, newest first, with its image, instruction and
/// where it is in the pipeline — received → converting → proposal ready →
/// inserted — plus the proposed LaTeX and the explicit Insert click.
///
/// What is stored here is only what the wire delivered (image bytes,
/// instruction, sender, arrival time) and the proposal text once the bridge
/// answered `capture_convert`; the *state* is derived on every read from the
/// shell's existing sources of truth (`bridgeCaptures`, `proposals`,
/// `appliedCaptureIDs`), so the panel can never disagree with the review
/// sheet or the bridge bar. Insertion goes through the same
/// `approveBridgeProposal` / `approveProposal` path the sheet uses: no new
/// insertion semantics, one less click.
@MainActor
final class CaptureInbox: ObservableObject {
    struct Item: Identifiable, Equatable {
        var id: String // capture_id
        var receivedAt: Date
        var pairId: String?
        var instructions: String
        var mimeType: String
        var image: Data
        var destinationId: String
        var baseRevision: Int
        /// The destination was pinned automatically at the caret when the
        /// companion asked (no ⌘⌥P); an explicit pin sets this false.
        var autoPinned: Bool
        /// LaTeX/TikZ from `capture_convert`, kept after the proposal leaves
        /// the review queue so an inserted row still shows what went in.
        var latex: String?
        /// Last failure (conversion refused, provider disabled, …), plain text.
        var failure: String?
        /// Set when the shell rejected the proposal from this panel or the sheet.
        var rejected = false

        static func == (a: Item, b: Item) -> Bool {
            a.id == b.id && a.latex == b.latex && a.failure == b.failure && a.rejected == b.rejected && a.instructions == b.instructions
        }
    }

    /// Pipeline state as the panel and the companion see it.
    enum State: Equatable {
        case received(note: String)
        case journaled(note: String)
        case converting
        case proposalReady
        case inserted
        case rejected
        case failed(String)

        var label: String {
            switch self {
            case .received: return "received"
            case .journaled: return "journaled"
            case .converting: return "converting…"
            case .proposalReady: return "proposal ready"
            case .inserted: return "inserted"
            case .rejected: return "rejected"
            case .failed: return "failed"
            }
        }
        var detail: String? {
            switch self {
            case .received(let n), .journaled(let n), .failed(let n): return n
            default: return nil
            }
        }
        var symbol: String {
            switch self {
            case .received, .journaled: return "tray.and.arrow.down"
            case .converting: return "arrow.triangle.2.circlepath"
            case .proposalReady: return "doc.text.magnifyingglass"
            case .inserted: return "checkmark.circle.fill"
            case .rejected: return "xmark.circle"
            case .failed: return "exclamationmark.triangle.fill"
            }
        }
        var isFinal: Bool {
            switch self { case .inserted, .rejected, .failed: return true; default: return false }
        }
    }

    @Published private(set) var items: [Item] = []
    /// The anchor id the shell pinned at the caret for a companion's
    /// destination query (`nearbyDestinationPinningCaretIfNeeded`); an explicit
    /// ⌘⌥P pin has a different id, so the panel can say "caret" vs "pinned".
    @Published var autoPinnedDestinationId: String?
    static let maxItems = 50
    /// Retained image bytes across `items`; the oldest final rows go first.
    static let maxImageBytes = 48 * 1024 * 1024

    func item(_ id: String) -> Item? { items.first { $0.id == id } }

    /// Records a capture the moment it arrives (before any bridge reply), so
    /// the panel shows it immediately. A retried `capture_id` updates nothing.
    func received(_ submit: RuntimeV1.CaptureSubmit, pairId: String?, autoPinned: Bool) {
        guard item(submit.captureId) == nil else { return }
        let bytes = Data(base64Encoded: submit.image.dataBase64) ?? Data()
        let row = Item(id: submit.captureId, receivedAt: Date(), pairId: pairId, instructions: submit.instructions,
                       mimeType: submit.image.mimeType, image: bytes, destinationId: submit.destinationId,
                       baseRevision: submit.baseRevision, autoPinned: autoPinned)
        items.insert(row, at: 0)
        trim(finalIDs: [])
    }

    func noteProposal(_ id: String, latex: String) { update(id) { $0.latex = latex; $0.failure = nil } }
    func noteFailure(_ id: String, _ note: String) { update(id) { $0.failure = note } }
    func noteRejected(_ id: String) { update(id) { $0.rejected = true } }
    func noteInstructions(_ id: String, _ text: String) { update(id) { $0.instructions = text } }
    func remove(_ id: String) { items.removeAll { $0.id == id } }
    func clearFinished(_ finalIDs: Set<String>) { items.removeAll { finalIDs.contains($0.id) } }

    private func update(_ id: String, _ f: (inout Item) -> Void) {
        guard let i = items.firstIndex(where: { $0.id == id }) else { return }
        f(&items[i])
    }

    /// Bounded like `NearbyInbox`: count first, then image bytes; rows the
    /// caller marks final are dropped before live ones.
    func trim(finalIDs: Set<String>) {
        func drop(where pred: (Item) -> Bool) -> Bool {
            guard let i = items.lastIndex(where: pred) else { return false }
            items.remove(at: i); return true
        }
        while items.count > Self.maxItems || items.reduce(0, { $0 + $1.image.count }) > Self.maxImageBytes {
            if items.count <= 1 { break }
            if !drop(where: { finalIDs.contains($0.id) }), !drop(where: { _ in true }) { break }
        }
    }

    // MARK: derived state (pure; tested)

    /// The row's pipeline state from the shell's sources of truth. `bridge`
    /// is the bridge session's row for this id (nil: inbox-only, no bridge),
    /// `queued` whether a proposal awaits review, `applied` whether the shell
    /// inserted it.
    static func state(for item: Item, bridge: BridgeSession.CaptureState?, bridgeNote: String?, queued: Bool, applied: Bool,
                      bridgeAttached: Bool, conversionEnabled: Bool) -> State {
        if applied { return .inserted }
        if item.rejected { return .rejected }
        if let f = item.failure { return .failed(f) }
        if queued || item.latex != nil { return .proposalReady }
        switch bridge {
        case .converting: return .converting
        case .proposed, .prepared: return .proposalReady
        case .applied, .confirmed: return .inserted
        case .rejected: return .rejected
        case .failed: return .failed(bridgeNote ?? "conversion failed")
        case .needsReselection: return .failed(bridgeNote ?? "the insertion point changed; pin again and resend")
        case .uncertain: return .failed(bridgeNote ?? "bridge relaunched; journaling uncertain — the iPad can resend")
        case .received:
            return .journaled(note: conversionEnabled ? "journaled by the bridge; converting next"
                              : "journaled; choose a conversion provider in Preferences (⌘,) → Capture conversion, then Convert")
        case nil:
            return .received(note: bridgeAttached ? "waiting for the bridge's receipt"
                             : "in the Mac's inbox; attach the capture bridge to convert it")
        }
    }
}

extension ShellModel {
    /// Pipeline state of one inbox row (CaptureInbox.state with this shell's inputs).
    func captureInboxState(_ item: CaptureInbox.Item) -> CaptureInbox.State {
        let row = bridgeCaptures.first { $0.captureId == item.id }
        return CaptureInbox.state(for: item, bridge: row?.state, bridgeNote: row?.note,
                                  queued: proposals.contains { $0.captureId == item.id },
                                  applied: appliedCaptureIDs.contains(item.id),
                                  bridgeAttached: bridgeAttached, conversionEnabled: bridge?.conversionEnabled == true)
    }

    /// The proposal for a row: the queued one, else rebuilt from the kept
    /// LaTeX (a row whose proposal already left the queue is not insertable).
    func captureInboxProposal(_ item: CaptureInbox.Item) -> RuntimeV1.CaptureProposal? {
        proposals.first { $0.captureId == item.id }
    }

    /// Rows the panel's Clear button removes: inserted, rejected or failed.
    var captureInboxFinalIDs: Set<String> {
        Set(captureInbox.items.filter { captureInboxState($0).isFinal }.map(\.id))
    }

    /// Insert at caret: the one explicit approval click, through the existing
    /// review path. Bridge captures insert at the destination the bridge
    /// bound at receipt (the caret when the companion asked, unless pinned);
    /// inbox-only captures use the local anchor. `latex` may be edited in the
    /// panel; the bridge refuses edited text (transfer-v1), which the note says.
    @discardableResult
    func insertCaptureFromInbox(_ item: CaptureInbox.Item, latex: String) async -> ApproveOutcome {
        guard let proposal = captureInboxProposal(item) else {
            captureNote = "Capture \(item.id) has no proposal awaiting review."
            return .refused("no proposal")
        }
        if isBridgeCapture(item.id) { return await approveBridgeProposal(proposal, latex: latex) }
        return approveProposal(proposal, latex: latex)
    }

    func rejectCaptureFromInbox(_ item: CaptureInbox.Item) {
        if let p = captureInboxProposal(item) { rejectProposal(p) } else { bridgeReject(captureId: item.id) }
        captureInbox.noteRejected(item.id)
    }

    /// `Convert` for a journaled row (a provider was added after receipt, or
    /// auto-convert was off): the same `capture_convert` as Edit > Convert Capture.
    func convertCaptureFromInbox(_ item: CaptureInbox.Item) {
        Task { await convertCaptureForInbox(captureId: item.id) }
    }

    @discardableResult
    func convertCaptureForInbox(captureId: String) async -> RuntimeV1.CaptureProposal? {
        let proposal = await convertCapture(captureId: captureId)
        if let proposal { captureInbox.noteProposal(captureId, latex: proposal.latex) }
        else if let note = captureNote { captureInbox.noteFailure(captureId, note) }
        return proposal
    }

    /// Opens the full review sheet (shadow compile, ambiguities) for a row.
    func reviewCaptureFromInbox(_ item: CaptureInbox.Item) {
        guard let p = captureInboxProposal(item) else { return }
        reviewing = p
    }
}

// MARK: - panel

/// Non-modal inspector: View > Captures (⌘⇧I) / the toolbar's Captures button.
/// Opening it starts advertising (the companion can connect without the
/// Nearby window); the status pill and the pairing-code button sit on top.
struct CaptureInboxPanel: View {
    @Environment(ShellModel.self) private var model
    @EnvironmentObject private var nearby: NearbyState
    @ObservedObject var inbox: CaptureInbox
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            if inbox.items.isEmpty {
                emptyState
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: DS.Space.m) {
                        ForEach(inbox.items) { item in
                            CaptureInboxRow(item: item, state: model.captureInboxState(item))
                        }
                    }
                    .padding(DS.Space.m)
                }
            }
        }
        .frame(minWidth: DS.Layout.inspectorMinWidth)
        .onAppear { model.prepareCaptureInbox(nearby: nearby) }
        .accessibilityIdentifier("captures.panel")
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            HStack(spacing: DS.Space.m) {
                Text("Captures").font(.headline)
                Spacer()
                let finished = model.captureInboxFinalIDs
                if !finished.isEmpty {
                    Button("Clear \(finished.count)") { inbox.clearFinished(finished) }
                        .controlSize(.small)
                        .help("Remove inserted, rejected and failed rows")
                }
            }
            HStack(spacing: DS.Space.m) {
                StatusPill(on: nearby.isAdvertising,
                           text: nearby.isAdvertising ? "Advertising" + (nearby.connectedPairIds.isEmpty ? "" : " · \(nearby.connectedPairIds.count) connected") : "Not advertising")
                    .accessibilityIdentifier("captures.advertising")
                Button(nearby.pairingCode == nil ? "Pairing code…" : "Code \(nearby.pairingCode!)") {
                    nearby.codeRequested = true
                    openWindow(id: "nearby")
                }
                .controlSize(.small)
                .help("Show a pairing code and QR image in the Nearby Companion window (⌘⇧N)")
                .accessibilityIdentifier("captures.pairing-code")
            }
            HStack(spacing: DS.Space.m) {
                StatusPill(on: model.bridgeAttached, text: model.bridgeAttached ? (model.bridge?.conversionEnabled == true ? "Bridge · \(model.bridge!.conversionProvider.displayName)" : "Bridge · no provider") : "No bridge")
                    .help(model.bridgeStatus)
                if !model.bridgeAttached {
                    Button("Attach bridge") { model.attachDiscoveredBridge() }.controlSize(.small)
                        .help("Edit > Attach Capture Bridge: journals captures and converts them")
                }
                Spacer()
                destinationLine
            }
            if let note = model.captureNote {
                Text(note).font(.caption2).foregroundStyle(.secondary).lineLimit(2).help(note)
            }
        }
        .padding(DS.Space.m)
    }

    private var destinationLine: some View {
        Group {
            if let d = model.nearbyDestination {
                Text("→ \(d.path) rev \(d.baseRevision)\(model.captureDestinationIsAutomatic ? " (caret)" : " (pinned)")")
                    .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    .help(model.captureDestinationIsAutomatic ? "The caret is the destination; the next capture is bound where the caret is when it arrives. Pin (⌘⌥P) to override." : "Pinned insertion point \(d.destinationId); captures insert there until you pin again.")
            } else {
                Text("→ caret").font(.caption).foregroundStyle(.secondary)
                    .help("No pin: the next capture is bound at the caret when the iPad asks. Pin (⌘⌥P) to override.")
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: DS.Space.m) {
            Image(systemName: "ipad.and.iphone").font(.largeTitle).foregroundStyle(.secondary)
            Text("No captures yet").font(.subheadline)
            Text(nearby.pairs.isEmpty ? "Show a pairing code, scan it on the iPad, then draw or photograph something and tap Send."
                                       : "Draw or photograph something on \(nearby.pairs.last?.companionName ?? "the iPad") and tap Send; it appears here and inserts at the caret after you approve it.")
                .font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(DS.Space.xl)
    }
}

private struct StatusPill: View {
    let on: Bool
    let text: String
    var body: some View {
        HStack(spacing: DS.Space.xs) {
            Circle().fill(on ? DS.Colors.severitySuccess : DS.Colors.textSecondary).frame(width: DS.Size.statusDot, height: DS.Size.statusDot)
            Text(text).font(.caption)
        }
        .padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
        .background(Capsule().fill(.quaternary))
        .accessibilityLabel(text)
    }
}

/// One capture: thumbnail, instruction, state pill, then the proposal (syntax
/// coloured, editable) and Insert / Review… / Reject once a proposal exists.
struct CaptureInboxRow: View {
    @Environment(ShellModel.self) private var model
    let item: CaptureInbox.Item
    let state: CaptureInbox.State
    @State private var latex = ""
    @State private var editing = false

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            HStack(alignment: .top, spacing: DS.Space.m) {
                thumbnail
                VStack(alignment: .leading, spacing: DS.Space.xxs) {
                    HStack(spacing: DS.Space.s) {
                        Image(systemName: state.symbol).foregroundStyle(color)
                        Text(state.label).font(.caption.bold()).foregroundStyle(color)
                            .accessibilityIdentifier("captures.row.state")
                        if case .converting = state { ProgressView().controlSize(.mini) }
                        Spacer()
                        Text(item.receivedAt, style: .time).font(.caption2).foregroundStyle(.tertiary)
                    }
                    Text(item.instructions.isEmpty ? "(no instruction)" : item.instructions)
                        .font(.caption).lineLimit(2).help(item.instructions)
                    if let d = state.detail { Text(d).font(.caption2).foregroundStyle(.secondary).lineLimit(3).help(d) }
                    Text("\(item.id)\(item.pairId.map { " · from \($0)" } ?? "") · \(item.autoPinned ? "at caret" : "pinned") rev \(item.baseRevision)")
                        .font(.caption2).foregroundStyle(.tertiary).lineLimit(1)
                }
            }
            if let text = displayedLaTeX {
                if editing {
                    TextEditor(text: $latex)
                        .font(.system(.caption, design: .monospaced))
                        .frame(minHeight: DS.Layout.searchPreviewMinHeight, maxHeight: DS.Layout.searchPreviewMaxHeight)
                        .border(.separator)
                        .accessibilityIdentifier("captures.row.editor")
                } else {
                    ScrollView(.horizontal) {
                        Text(CaptureInboxRow.highlighted(text))
                            .font(.system(.caption, design: .monospaced))
                            .textSelection(.enabled)
                            .padding(DS.Space.s)
                    }
                    .frame(maxHeight: DS.Layout.searchPreviewMaxHeight)
                    .background(RoundedRectangle(cornerRadius: DS.Radius.control).fill(.quaternary.opacity(DS.State.restingControlOpacity)))
                    .accessibilityIdentifier("captures.row.latex")
                }
            }
            actions
        }
        .padding(DS.Space.m)
        .background(RoundedRectangle(cornerRadius: DS.Radius.tab).fill(.background))
        .overlay(RoundedRectangle(cornerRadius: DS.Radius.tab).stroke(.separator))
        .onAppear { latex = item.latex ?? "" }
        .onChange(of: item.latex) { _, new in if !editing { latex = new ?? "" } }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Capture \(item.id), \(state.label)")
    }

    private var displayedLaTeX: String? {
        if case .proposalReady = state { return model.captureInboxProposal(item)?.latex ?? item.latex }
        return item.latex
    }

    @ViewBuilder private var actions: some View {
        switch state {
        case .proposalReady:
            HStack(spacing: DS.Space.s) {
                Button("Insert at caret") {
                    let text = editing ? latex : (model.captureInboxProposal(item)?.latex ?? latex)
                    Task { _ = await model.insertCaptureFromInbox(item, latex: text) }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(model.captureInboxProposal(item) == nil || latex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && editing)
                .help(model.isBridgeCapture(item.id)
                      ? "Approve: the bridge prepares the edit at the destination bound at receipt (\(item.autoPinned ? "the caret" : "your pin")), verifies it, inserts once (undo with ⌘Z)"
                      : "Approve: inserts at the pinned insertion point as one undoable edit")
                .accessibilityIdentifier("captures.row.insert")
                Toggle(isOn: $editing) { Text("Edit") }.toggleStyle(.button).controlSize(.small)
                    .help(model.isBridgeCapture(item.id) ? "Edit before insert (the bridge refuses edited text; edit after inserting, or reject and resend)" : "Edit the LaTeX before inserting")
                Button("Review…") { model.reviewCaptureFromInbox(item) }.controlSize(.small)
                    .help("Open the review sheet with the shadow compile and ambiguities")
                Spacer()
                Button("Reject", role: .destructive) { model.rejectCaptureFromInbox(item) }.controlSize(.small)
                    .accessibilityIdentifier("captures.row.reject")
            }
            .controlSize(.small)
        case .journaled:
            HStack {
                Button("Convert") { model.convertCaptureFromInbox(item) }.controlSize(.small)
                    .disabled(!model.bridgeAttached)
                    .accessibilityIdentifier("captures.row.convert")
                Spacer()
                Button("Reject", role: .destructive) { model.rejectCaptureFromInbox(item) }.controlSize(.small)
            }
        case .failed:
            HStack {
                if model.bridgeAttached, model.isBridgeCapture(item.id) {
                    Button("Retry conversion") { model.convertCaptureFromInbox(item) }.controlSize(.small)
                }
                Spacer()
                Button("Remove") { model.captureInbox.remove(item.id) }.controlSize(.small)
            }
        case .inserted, .rejected:
            HStack { Spacer(); Button("Remove") { model.captureInbox.remove(item.id) }.controlSize(.small) }
        default:
            EmptyView()
        }
    }

    private var color: Color {
        switch state {
        case .inserted: return .green
        case .failed, .rejected: return .orange
        case .proposalReady: return .accentColor
        default: return .secondary
        }
    }

    private var thumbnail: some View {
        Group {
            if let img = NSImage(data: item.image) {
                Image(nsImage: img).resizable().aspectRatio(contentMode: .fit)
            } else {
                Image(systemName: "photo").foregroundStyle(.secondary)
            }
        }
        .frame(width: DS.Size.thumbnail, height: DS.Size.thumbnail)
        .background(RoundedRectangle(cornerRadius: DS.Radius.control).fill(.quaternary))
        .accessibilityLabel("capture image, \(item.mimeType), \(item.image.count) bytes")
    }

    /// Syntax colours from the editor's lexer (SyntaxHighlighter.swift) as an
    /// AttributedString for the read-only proposal.
    static func highlighted(_ text: String) -> AttributedString {
        var out = AttributedString(text)
        let ns = text as NSString
        for run in SyntaxHighlighter.runs(of: ns) {
            guard let color = SyntaxTheme.color(for: run.kind),
                  let r = Range(run.range, in: text),
                  let lo = AttributedString.Index(r.lowerBound, within: out),
                  let hi = AttributedString.Index(r.upperBound, within: out) else { continue }
            out[lo..<hi].foregroundColor = Color(nsColor: color)
        }
        return out
    }
}
