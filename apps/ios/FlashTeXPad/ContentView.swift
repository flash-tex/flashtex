import FlashTeXPadKit
import FlashTeXProtocol
import NearbyClient
import SwiftUI
import UniformTypeIdentifiers

enum Panel: String, CaseIterable, Identifiable {
    case capture = "Capture", mac = "Mac link"
    case editor = "Editor", diagnostics = "Diagnostics", review = "Review"
    var id: String { rawValue }
    var symbol: String {
        switch self {
        case .capture: return "pencil.and.outline"
        case .mac: return "laptopcomputer.and.ipad"
        case .editor: return "doc.text"
        case .diagnostics: return "exclamationmark.triangle"
        case .review: return "checkmark.seal"
        }
    }
    static let primary: [Panel] = [.capture, .mac]
    static let reference: [Panel] = [.editor, .diagnostics, .review]
}

struct ContentView: View {
    @EnvironmentObject var model: PadModel
    @State private var panel: Panel? = .capture
    @State private var importingTex = false
    @State private var importingResult = false

    static let texType = UTType(filenameExtension: "tex") ?? .plainText

    var body: some View {
        NavigationSplitView {
            List(selection: $panel) {
                Section("Capture companion") {
                    ForEach(Panel.primary) { p in
                        Label(p.rawValue, systemImage: p.symbol).tag(p).accessibilityIdentifier("panel.\(p.rawValue)")
                    }
                }
                Section("Reference (.tex on the Mac; not the product)") {
                    ForEach(Panel.reference) { p in
                        Label(p.rawValue, systemImage: p.symbol).tag(p).accessibilityIdentifier("panel.\(p.rawValue)")
                    }
                }
            }
            .navigationTitle("FlashTeXPad")
            .safeAreaInset(edge: .bottom) {
                VStack(alignment: .leading, spacing: 6) {
                    Button { importingTex = true } label: { Label("Open .tex…", systemImage: "folder") }
                        .accessibilityIdentifier("open.file")
                    Button { model.openBundledSample() } label: { Label("Open bundled demo.tex", systemImage: "doc.badge.plus") }
                        .accessibilityIdentifier("open.sample")
                    Button { model.openReviewFixture() } label: { Label("Open review fixture", systemImage: "checkmark.seal") }
                        .accessibilityIdentifier("open.fixture")
                    Text(model.documentTitle).font(.footnote).foregroundStyle(.secondary)
                        .accessibilityIdentifier("document.title")
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.bar)
            }
        } detail: {
            switch panel ?? .capture {
            case .capture: CaptureView()
            case .editor: EditorPanel()
            case .diagnostics: DiagnosticsPanel(importing: $importingResult)
            case .review: ReviewPanel()
            case .mac: MacLinkPanel()
            }
        }
        .fileImporter(isPresented: $importingTex, allowedContentTypes: [Self.texType, .plainText]) { r in
            if case .success(let url) = r { model.open(url: url) }
        }
        .fileImporter(isPresented: $importingResult, allowedContentTypes: [.json]) { r in
            if case .success(let url) = r { model.openCompileResult(url: url) }
        }
        .alert("Could not open", isPresented: Binding(get: { model.openError != nil }, set: { if !$0 { model.openError = nil } })) {
            Button("OK") { model.openError = nil }
        } message: { Text(model.openError ?? "") }
    }
}

// MARK: editor

struct EditorPanel: View {
    @EnvironmentObject var model: PadModel

    var body: some View {
        VStack(spacing: 0) {
            if let d = model.document {
                HStack {
                    Text("\(d.path) · revision \(d.revision) · \(d.utf8Count) UTF-8 bytes · caret UTF-16 \(model.caretUTF16)")
                        .font(.footnote.monospaced()).foregroundStyle(.secondary)
                        .accessibilityIdentifier("editor.status")
                    Spacer()
                }.padding(.horizontal).padding(.vertical, 6)
                SourceTextView(text: Binding(get: { d.text }, set: { model.textChanged($0) }), caret: $model.caretUTF16,
                               onCaret: { model.caretMoved($0) })
                    .accessibilityIdentifier("editor.text")
                Divider()
                CompletionBar()
            } else {
                ContentUnavailableView("Open a .tex document", systemImage: "doc.text",
                                       description: Text("Use Open .tex… or the bundled demo.tex in the sidebar."))
            }
        }
        .navigationTitle(model.documentTitle)
    }
}

struct CompletionBar: View {
    @EnvironmentObject var model: PadModel
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Completions — local, source-derived. Mac metadata-bound completion is not carried by transfer-v1.")
                .font(.caption2).foregroundStyle(.secondary)
            if model.completions.isEmpty {
                Text("Type \\ or an argument after \\begin{ \\end{ \\ref{ \\cite{ to see suggestions.").font(.caption).foregroundStyle(.tertiary)
            } else {
                ScrollView(.horizontal) {
                    HStack {
                        ForEach(model.completions) { s in
                            Button { model.accept(s) } label: {
                                VStack(alignment: .leading) {
                                    Text(s.text).font(.body.monospaced())
                                    Text(s.detail).font(.caption2).foregroundStyle(.secondary)
                                }
                            }
                            .buttonStyle(.bordered)
                            .accessibilityIdentifier("completion.\(s.text)")
                        }
                    }
                }
                .accessibilityIdentifier("completion.list")
            }
        }
        .padding(8)
    }
}

/// UITextView wrapper: the caret is reported in UTF-16 units and converted to
/// UTF-8 bytes by `PadDocument.byteOffset(ofUTF16:)` (FlashTeXProtocol
/// `utf8ByteRange(of:)`), the Mac editor's discipline.
struct SourceTextView: UIViewRepresentable {
    @Binding var text: String
    @Binding var caret: Int
    var onCaret: (Int) -> Void

    func makeUIView(context: Context) -> UITextView {
        let v = UITextView()
        v.font = .monospacedSystemFont(ofSize: 15, weight: .regular)
        v.autocorrectionType = .no
        v.autocapitalizationType = .none
        v.smartQuotesType = .no
        v.smartDashesType = .no
        v.delegate = context.coordinator
        v.text = text
        v.accessibilityIdentifier = "editor.textview"
        return v
    }

    func updateUIView(_ v: UITextView, context: Context) {
        context.coordinator.parent = self
        if v.text != text {
            let sel = v.selectedRange
            v.text = text
            let loc = min(caret, (text as NSString).length)
            v.selectedRange = NSRange(location: loc, length: 0)
            _ = sel
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    final class Coordinator: NSObject, UITextViewDelegate {
        var parent: SourceTextView
        init(_ p: SourceTextView) { parent = p }
        func textViewDidChange(_ v: UITextView) { parent.text = v.text; parent.onCaret(v.selectedRange.location) }
        func textViewDidChangeSelection(_ v: UITextView) { parent.onCaret(v.selectedRange.location) }
    }
}

// MARK: diagnostics

struct DiagnosticsPanel: View {
    @EnvironmentObject var model: PadModel
    @Binding var importing: Bool

    var body: some View {
        List {
            Section {
                Text("Diagnostics are runtime-v1 compile_result.diagnostics. Nearby-v1 does not carry compile results to a companion (proposal §6); the Mac compiles. Sources here: a bundled fixture or a compile_result JSON you open.")
                    .font(.caption).foregroundStyle(.secondary)
                Text("Source: \(model.diagnosticsSource.label)").font(.footnote).accessibilityIdentifier("diagnostics.source")
                HStack {
                    Button("Open compile_result JSON…") { importing = true }
                    Button("Bundled fixture") { model.openBundledCompileResult() }.accessibilityIdentifier("diagnostics.fixture")
                }
            }
            Section("\(model.diagnostics.count) diagnostic\(model.diagnostics.count == 1 ? "" : "s")") {
                if model.diagnostics.isEmpty {
                    Text("none").foregroundStyle(.secondary)
                }
                ForEach(model.diagnostics) { d in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Image(systemName: d.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                                .foregroundStyle(d.severity == .error ? .red : .orange)
                            Text(d.message)
                        }
                        if let r = d.range {
                            Text("\(r.path) bytes \(r.startByte)..<\(r.endByte)" + (d.excerpt.map { " → “\($0)”" } ?? " (not bound to the open document)"))
                                .font(.caption.monospaced()).foregroundStyle(.secondary)
                        }
                        if let rec = d.recovery { Text("recovery: \(rec)").font(.caption).foregroundStyle(.secondary) }
                    }
                    .accessibilityIdentifier("diagnostic.\(d.id)")
                }
            }
        }
        .navigationTitle("Diagnostics")
    }
}

// MARK: review

struct ReviewPanel: View {
    @EnvironmentObject var model: PadModel

    var body: some View {
        List {
            Section {
                Text("Reviewed-proposal contract: assistant-context proposal_review → explicit approval → approved_group applied once against revision + SHA-256 + exact removed text (crates/assistant-context/README.md). Nearby-v1 does not return proposals to a companion, so the Mac's own review stays on the Mac.")
                    .font(.caption).foregroundStyle(.secondary)
                Text(model.reviewProvenance.banner).font(.footnote).accessibilityIdentifier("review.provenance")
            }
            if let r = model.review {
                Section("Proposal \(r.review.reviewId.prefix(12))…") {
                    Text(r.review.payload.explanation).accessibilityIdentifier("review.explanation")
                    ForEach(Array(r.review.payload.edits.enumerated()), id: \.offset) { i, e in
                        VStack(alignment: .leading) {
                            Text("\(e.location.path) bytes \(e.location.startByte)..<\(e.location.endByte)").font(.caption.monospaced())
                            Text("− \(e.removedText)").font(.body.monospaced()).foregroundStyle(.red)
                            Text("+ \(e.replacement)").font(.body.monospaced()).foregroundStyle(.green)
                        }.accessibilityIdentifier("review.edit.\(i)")
                    }
                    Text("bound to revision \(r.approved.payload.group.expectedRevision), sha256 \(r.approved.payload.group.expectedSha256.prefix(16))…")
                        .font(.caption.monospaced()).foregroundStyle(.secondary)
                }
                Section("Decision") {
                    Text(stateLabel(r.state)).accessibilityIdentifier("review.state")
                    HStack {
                        Button("Cancel") { model.cancelReview() }.buttonStyle(.bordered).disabled(!r.isPending)
                            .accessibilityIdentifier("review.cancel")
                        Button("Approve and insert") { model.approveReview() }.buttonStyle(.borderedProminent).disabled(!r.isPending)
                            .accessibilityIdentifier("review.approve")
                    }
                    if let e = model.reviewError { Text(e).foregroundStyle(.red).accessibilityIdentifier("review.error") }
                }
            } else {
                Section { Text("No pending proposal. Open the review fixture from the sidebar.").foregroundStyle(.secondary) }
            }
            if let rc = model.lastReceipt {
                Section("Receipt (echo of the single insertion)") {
                    Text("command \(rc.commandId)").font(.caption.monospaced())
                    Text("review \(rc.reviewId)").font(.caption.monospaced())
                    Text("revision \(rc.expectedRevision) → \(rc.newRevision); \(rc.editCount) edit(s); inserted at \(rc.insertedRanges.map { "\($0.start)..<\($0.end)" }.joined(separator: ", "))")
                        .font(.caption.monospaced())
                    Text("sha256 after \(rc.sha256After)").font(.caption.monospaced())
                    Text("insertions this session: \(model.insertionCount)").accessibilityIdentifier("review.insertions")
                }
                .accessibilityIdentifier("review.receipt")
            }
        }
        .navigationTitle("Review")
    }

    func stateLabel(_ s: ReviewSession.State) -> String {
        switch s {
        case .pending: return "pending — nothing inserted"
        case .cancelled(let why): return "cancelled (\(why)) — nothing inserted"
        case .applied(let r): return "applied once — revision \(r.newRevision)"
        case .refused(let why): return "refused: \(why) — nothing inserted"
        }
    }
}

// MARK: Mac link

struct MacLinkPanel: View {
    @EnvironmentObject var model: PadModel
    @State private var host = "127.0.0.1"
    @State private var port = ""
    @State private var salt = ""
    @State private var fingerprint = ""
    @State private var macName = "Mac"
    @State private var code = ""
    @State private var instructions = "Transcribe this capture"
    @State private var qrText = ""
    @State private var scanning = false
    @State private var focusNote: String?

    var body: some View {
        Form {
            Section("Link") {
                Text(model.linkStatus).accessibilityIdentifier("link.status")
                if let e = model.linkError { Text(e).foregroundStyle(.red).accessibilityIdentifier("link.error") }
                if let p = model.pairedMac {
                    Text("Stored in the Keychain (this iPad only): \(p.macName) fp=\(p.fingerprint) pair_id=\(p.pairId), paired \(p.pairedAt.formatted(date: .abbreviated, time: .shortened))")
                        .font(.caption.monospaced()).foregroundStyle(.secondary).accessibilityIdentifier("pair.stored")
                    if !model.link.isConnected {
                        Button { Task { await model.autoReconnect() } } label: {
                            Label(model.reconnecting ? "Reconnecting…" : "Reconnect to \(p.macName)", systemImage: "arrow.clockwise")
                        }
                        .buttonStyle(.borderedProminent).disabled(model.reconnecting).accessibilityIdentifier("link.reconnect")
                    }
                }
            }
            // Bonjour discovery: nearby FlashTeX Macs, tap to pair (the Mac's code is still required).
            Section("Nearby Macs (Bonjour \(NearbyWire.serviceType))") {
                HStack {
                    Button { Task { await model.browseNearby() } } label: { Label(model.browsing ? "Browsing…" : "Find nearby Macs", systemImage: "dot.radiowaves.left.and.right") }
                        .buttonStyle(.bordered).disabled(model.browsing).accessibilityIdentifier("pair.browse")
                    TextField("6-digit code from the Mac", text: $code).keyboardType(.numberPad).frame(maxWidth: 220).accessibilityIdentifier("pair.code")
                }
                if model.nearbyMacs.isEmpty {
                    Text(model.browsing ? "listening…" : "none found yet — on the Mac open Captures (⌘⇧I) or Nearby Companion (⌘⇧N); advertising starts there")
                        .font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("pair.nearby.none")
                } else {
                    ForEach(model.nearbyMacs, id: \.self) { mac in
                        Button {
                            Task { await model.pair(mac: mac, code: code) }
                        } label: {
                            HStack {
                                VStack(alignment: .leading) {
                                    Text(mac.macName)
                                    Text("fp \(mac.fingerprint ?? "?") · \(mac.fingerprint == model.pairedMac?.fingerprint ? "already paired" : "tap to pair with the code")")
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Image(systemName: "laptopcomputer")
                            }
                        }
                        .disabled(code.count != 6)
                        .accessibilityIdentifier("pair.nearby.\(mac.fingerprint ?? mac.name)")
                    }
                }
            }
            Section("Pair by QR (the Mac's Nearby window shows it beside the code)") {
                Text("Scan the Mac's QR (flashtex-nearby://pair?v=1&code&salt&fp&name) with the camera, or paste its text. The Mac is found by Bonjour (fp); if that fails — the simulator has no camera and no Bonjour listener in tests — type its host and port below.")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    if PairingScannerView.isAvailable {
                        Button { scanning = true } label: { Label("Scan QR…", systemImage: "qrcode.viewfinder") }.buttonStyle(.bordered)
                    } else {
                        Text("Camera scanner unavailable on this device (VisionKit DataScanner not supported here) — paste the payload instead.")
                            .font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("pair.qr.unavailable")
                    }
                }
                TextField("flashtex-nearby://pair?v=1&code=…&salt=…&fp=…&name=…", text: $qrText, axis: .vertical)
                    .font(.caption.monospaced()).accessibilityIdentifier("pair.qr.text")
                HStack {
                    Button("Paste") { if let t = UIPasteboard.general.string { qrText = t } }.buttonStyle(.bordered).accessibilityIdentifier("pair.qr.paste")
                    Button("Pair from payload") { Task { await model.pair(bootstrapText: qrText, host: host, port: port) } }
                        .buttonStyle(.borderedProminent).disabled(qrText.isEmpty).accessibilityIdentifier("pair.qr.go")
                }
            }
            .sheet(isPresented: $scanning) {
                NavigationStack {
                    PairingScannerView(onPayload: { text in
                        qrText = text; scanning = false
                        Task { await model.pair(bootstrapText: text, host: host, port: port) }
                    }, onFocus: { outcome in focusNote = outcome.note })
                        .navigationTitle("Scan the Mac's pairing QR")
                        .toolbar { Button("Cancel") { scanning = false } }
                        // Only set when the camera cannot autofocus, so the
                        // owner is told rather than left wondering why the
                        // code will not resolve.
                        .safeAreaInset(edge: .bottom) {
                            if let n = focusNote {
                                Text(n).font(.footnote).padding(8)
                                    .frame(maxWidth: .infinity)
                                    .background(.bar)
                                    .accessibilityIdentifier("pair.qr.focusNote")
                            }
                        }
                }
                .onDisappear { focusNote = nil }
            }
            Section("Nearby-v1 pairing by typed code (apps/mac/docs/nearby-v1-proposal.md §2, §7)") {
                Text("Or enter what the Mac's Nearby window shows (Edit > Nearby Companion… > Show Pairing Code): host/port, TXT salt and fp, and the code.")
                    .font(.caption).foregroundStyle(.secondary)
                TextField("Host", text: $host).accessibilityIdentifier("pair.host")
                TextField("Port", text: $port).keyboardType(.numberPad).accessibilityIdentifier("pair.port")
                TextField("TXT salt (32 hex)", text: $salt).font(.body.monospaced()).accessibilityIdentifier("pair.salt")
                TextField("TXT fp (16 hex)", text: $fingerprint).font(.body.monospaced()).accessibilityIdentifier("pair.fp")
                TextField("Mac name", text: $macName)
                TextField("Pairing code", text: $code).font(.body.monospaced()).accessibilityIdentifier("pair.code")
                HStack {
                    Button("Pair") { Task { await model.pair(host: host, port: port, saltHex: salt, fingerprint: fingerprint, macName: macName, code: code) } }
                        .buttonStyle(.borderedProminent).accessibilityIdentifier("pair.go")
                    Button("Reconnect with stored key") { Task { await model.reconnect(host: host, port: port) } }.disabled(model.pairedMac == nil)
                    Button("Disconnect") { model.disconnect() }
                    Button("Forget pairing", role: .destructive) { model.forgetPairing() }.disabled(model.pairedMac == nil)
                }

            }
            Section("Destination (hello_ack / destination_query)") {
                if let d = model.destination {
                    Text("\(d.projectId)/\(d.path) destination \(d.destinationId) base_revision \(d.baseRevision)").font(.caption.monospaced())
                    // What kind of place the Mac's caret is in, so it is clear
                    // before sending whether a formula will come back wrapped in
                    // $ … $, in \[ … \], or not wrapped at all. Absent from a Mac
                    // that predates `caret_context` (nearby-v1, additive).
                    if let caret = d.caretContext {
                        Text(caret.label)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .accessibilityIdentifier("destination.caret")
                    }
                } else { Text("null — pin an insertion point on the Mac (⌘⇧P)").foregroundStyle(.secondary) }
                Button("Refresh") { Task { await model.refreshDestination() } }.disabled(!model.link.isConnected)
            }
            Section("capture_submit → capture_received (transfer-v1)") {
                TextField("Instructions", text: $instructions)
                Button("Send test capture (1×1 PNG fixture)") { Task { await model.sendTestCapture(instructions: instructions) } }
                    .disabled(!model.link.isConnected).accessibilityIdentifier("capture.send")
                if let r = model.lastCaptureReceived {
                    Text(verbatim: "capture_received \(r.captureId) durable=\(r.durable) has_proposal=\(r.hasProposal) applied=\(r.applied)")
                        .font(.caption.monospaced()).accessibilityIdentifier("capture.receipt")
                }
                Text("The Mac converts and reviews on its side; the receipt is followed by capture_status polling on the Capture screen.").font(.caption).foregroundStyle(.secondary)
            }
            Section("Wire transcript (keys and image bytes redacted)") {
                ForEach(model.transcript) { l in
                    Text("\(l.direction.rawValue) \(l.text)").font(.caption2.monospaced())
                        .foregroundStyle(l.direction == .note ? .secondary : .primary)
                }
            }
        }
        .navigationTitle("Mac link")
    }
}
