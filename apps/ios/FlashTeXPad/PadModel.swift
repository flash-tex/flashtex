import Foundation
import FlashTeXPadKit
import FlashTeXProtocol
import NearbyClient
import SwiftUI

/// App state for FlashTeXPad. Everything the Mac does not send over
/// nearby-v1 is labelled by `Provenance` so no panel can pass off a local
/// fixture as a Mac reply.
@MainActor
final class PadModel: ObservableObject {
    enum Provenance: Equatable {
        case mac(String)
        case localFixture(String)
        case notCarried
        var banner: String {
            switch self {
            case .mac(let s): return "from the Mac over nearby-v1: \(s)"
            case .localFixture(let s): return "local fixture — not carried by transfer-v1: \(s)"
            case .notCarried: return "not carried by transfer-v1 (nearby-v1 §6): stays on the Mac"
            }
        }
    }

    // Document
    @Published var document: PadDocument?
    @Published var documentTitle = "No document"
    @Published var caretUTF16 = 0
    @Published var openError: String?

    // Diagnostics (runtime-v1 compile_result)
    @Published var diagnostics: [DiagnosticItem] = []
    @Published var diagnosticsSource: DiagnosticsSource = .none

    // Completions (local, source-derived)
    @Published var completions: [LocalCompletion.Suggestion] = []

    // Reviewed proposal
    @Published var review: ReviewSession?
    @Published var reviewProvenance: Provenance = .notCarried
    @Published var lastReceipt: ReviewSession.Receipt?
    @Published var reviewError: String?
    @Published var insertionCount = 0

    // Mac link
    let link: MacLink
    @Published var transcript: [MacLink.TranscriptLine] = []
    @Published var linkStatus = "not paired"
    @Published var linkError: String?
    @Published var destination: NearbyWire.Destination?
    @Published var lastCaptureReceived: NearbyWire.CaptureReceived?
    @Published var pairedMac: PairedMac?

    // Captures (the product: draw/pick → capture_submit → receipt)
    let queue: CaptureQueue
    @Published var captures: [CaptureRecord] = []

    /// Production: pairings in the Keychain (`KeychainPairStore`), drafts /
    /// receipts / outcomes in Application Support (`CaptureStore`). Tests pass
    /// their own link (no Keychain) and, when persistence is under test, a
    /// store in a temporary directory. `-flashtexpad-fresh` (UI tests) wipes
    /// both so a run never sees a previous run's captures or pairing.
    init(link: MacLink? = nil, captureStore: CaptureStore? = nil) {
        let fresh = ProcessInfo.processInfo.arguments.contains("-flashtexpad-fresh")
        // Hosted XCTest (TEST_HOST) constructs its own PadModel; the app's
        // @StateObject must not load Keychain/disk or poll leftover captures.
        let hostedUnitTests = ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil
        let production = link == nil && !hostedUnitTests
        let link = link ?? {
            if hostedUnitTests { return MacLink(store: nil) }
            let keychain = KeychainPairStore()
            if fresh { try? keychain.removeAll() }
            return MacLink(store: keychain)
        }()
        let store: CaptureStore? = captureStore ?? (production ? CaptureStore() : nil)
        if fresh { store?.wipe() }
        self.link = link
        self.queue = CaptureQueue(link: link, store: store)
        self.captures = queue.records
        link.onTranscript = { [weak self] line in Task { @MainActor in self?.transcript.append(line) } }
        if let p = link.store?.pairs.last { pairedMac = p; linkStatus = "stored pairing: \(p.macName) (\(p.pairId)) — reconnect with the stored key" }
    }

    // MARK: captures

    @discardableResult
    func draft(_ r: CaptureRecord) -> CaptureRecord {
        let d = queue.draft(r)
        captures = queue.records
        return d
    }

    func discard(_ id: String) { queue.discard(id); captures = queue.records }

    // MARK: one-tap send (lane mac-capture-fluid)

    /// Instruction chips: the last few instructions sent, newest first, seeded
    /// with the three the Mac's converter is best at. Persisted per iPad.
    static let defaultInstructions = ["Convert to TikZ", "Transcribe as LaTeX", "This is a matrix"]
    static let recentInstructionsKey = "flashtexpad.recentInstructions"
    static let maxRecentInstructions = 6
    @Published var recentInstructions: [String] = {
        let saved = UserDefaults.standard.stringArray(forKey: PadModel.recentInstructionsKey) ?? []
        return PadModel.merged(recent: saved, defaults: PadModel.defaultInstructions)
    }()

    /// Pure: `recent` first (deduplicated, trimmed), the defaults appended once, bounded.
    static func merged(recent: [String], defaults: [String], limit: Int = maxRecentInstructions) -> [String] {
        var out: [String] = []
        for s in recent + defaults {
            let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !t.isEmpty, !out.contains(where: { $0.caseInsensitiveCompare(t) == .orderedSame }) else { continue }
            out.append(t)
            if out.count == limit { break }
        }
        return out
    }

    func rememberInstruction(_ text: String) {
        recentInstructions = Self.merged(recent: [text] + recentInstructions, defaults: Self.defaultInstructions)
        UserDefaults.standard.set(recentInstructions, forKey: Self.recentInstructionsKey)
    }

    /// Prepare + Send as one tap: validates (the same `CaptureQueue.validate`
    /// as before), drafts the record so it is on disk before the bytes leave,
    /// sends it, and starts outcome polling. Returns the problem instead of
    /// sending when validation fails or no Mac is connected; the capture id
    /// otherwise.
    @discardableResult
    func sendNow(png: Data, source: CaptureRecord.Source, instructions: String, pixelSize: (width: Int, height: Int)? = nil) async -> Result<String, SendProblem> {
        let text = instructions.trimmingCharacters(in: .whitespacesAndNewlines)
        if let why = CaptureQueue.validate(png: png, instructions: text) { return .failure(.invalid(why)) }
        guard link.isConnected else { return .failure(.notConnected(linkStatus)) }
        let r = draft(CaptureRecord(source: source, png: png, instructions: text, pixelSize: pixelSize))
        rememberInstruction(text)
        await send(r.id)
        return .success(r.id)
    }

    enum SendProblem: Error, Equatable, CustomStringConvertible {
        case invalid(String), notConnected(String)
        var description: String {
            switch self {
            case .invalid(let why): return why
            case .notConnected(let status): return "not connected to a Mac (\(status)) — pair or reconnect first"
            }
        }
    }

    // MARK: auto-reconnect and discovery

    /// Last address the stored pairing was reached at (per Mac fingerprint),
    /// so a relaunch tries it before browsing Bonjour.
    static func lastEndpointKey(_ fingerprint: String) -> String { "flashtexpad.lastEndpoint.\(fingerprint)" }
    func rememberEndpoint(host: String, port: UInt16, fingerprint: String) {
        UserDefaults.standard.set("\(host):\(port)", forKey: Self.lastEndpointKey(fingerprint))
    }
    static func lastEndpoint(fingerprint: String, defaults: UserDefaults = .standard) -> (host: String, port: UInt16)? {
        guard let s = defaults.string(forKey: lastEndpointKey(fingerprint)), let colon = s.lastIndex(of: ":"),
              let port = UInt16(s[s.index(after: colon)...]) else { return nil }
        return (String(s[..<colon]), port)
    }

    @Published var reconnecting = false
    /// Macs advertising `_flashtex-nearby._tcp` right now (Bonjour), for tap-to-pair.
    @Published var nearbyMacs: [DiscoveredMac] = []
    @Published var browsing = false

    /// At launch (and from the Mac link panel): connect to the last paired Mac
    /// with the stored key — the remembered address first, else the Mac found
    /// by Bonjour with the same fingerprint. Never pairs; a Mac that is not
    /// there leaves the pairing stored and the status honest. `endpoint`
    /// overrides both lookups (tests: the fake Mac on loopback).
    @discardableResult
    func autoReconnect(endpoint: (host: String, port: UInt16)? = nil, browseSeconds: TimeInterval = 4) async -> Bool {
        guard let pair = pairedMac, !link.isConnected, !reconnecting else { return link.isConnected }
        reconnecting = true
        defer { reconnecting = false }
        linkError = nil
        var candidates: [(String, UInt16)] = []
        if let endpoint { candidates.append(endpoint) }
        else if let last = Self.lastEndpoint(fingerprint: pair.fingerprint) { candidates.append(last) }
        for (host, port) in candidates {
            linkStatus = "reconnecting to \(pair.macName) at \(host):\(port)…"
            do {
                try await link.connect(host: host, port: port, pair: pair)
                connected(pair, host: host, port: port)
                return true
            } catch { linkStatus = "\(pair.macName) not at \(host):\(port); browsing…" }
        }
        guard endpoint == nil else { linkError = "reconnect failed"; linkStatus = "stored pairing: \(pair.macName) — Mac not reachable"; return false }
        do {
            let found = try await NearbyBrowser.discover(seconds: browseSeconds) { $0.fingerprint == pair.fingerprint }
            guard let mac = found.first(where: { $0.fingerprint == pair.fingerprint }) else {
                linkStatus = "stored pairing: \(pair.macName) — not advertising nearby (open Captures on the Mac)"
                return false
            }
            let session = try await NearbyClient.connect(endpoint: mac.endpoint, pair: pair)
            link.adopt(session: session, pair: pair)
            if case .hostPort(let h, let p) = mac.endpoint { rememberEndpoint(host: "\(h)", port: p.rawValue, fingerprint: pair.fingerprint) }
            destination = link.destination
            linkStatus = "connected to \(pair.macName)"
            resumeOutcomePolling()
            return true
        } catch {
            linkError = "\(error)"
            linkStatus = "stored pairing: \(pair.macName) — reconnect failed"
            return false
        }
    }

    private func connected(_ pair: PairedMac, host: String, port: UInt16) {
        rememberEndpoint(host: host, port: port, fingerprint: pair.fingerprint)
        destination = link.destination
        linkStatus = "connected to \(pair.macName)"
        resumeOutcomePolling()
    }

    /// Lists nearby FlashTeX Macs for `seconds`; the panel shows them as
    /// tap-to-pair rows (the code is still required — nearby-v1 §2).
    func browseNearby(seconds: TimeInterval = 3) async {
        guard !browsing else { return }
        browsing = true
        defer { browsing = false }
        do { nearbyMacs = try await NearbyBrowser.discover(seconds: seconds).filter(\.isSupported) }
        catch { linkError = "browse: \(error)" }
    }

    /// Tap-to-pair with a discovered Mac: the same bootstrap as the QR path.
    @discardableResult
    func pair(mac: DiscoveredMac, code: String) async -> Bool {
        linkError = nil
        linkStatus = "pairing with \(mac.macName)…"
        do {
            let pair = try await link.pair(discovered: mac, code: code, companionName: "FlashTeXPad (\(UIDevice.current.name))")
            pairedMac = pair
            destination = link.destination
            linkStatus = "paired with \(pair.macName) (\(pair.pairId)); connected"
            if case .hostPort(let h, let p) = mac.endpoint { rememberEndpoint(host: "\(h)", port: p.rawValue, fingerprint: pair.fingerprint) }
            resumeOutcomePolling()
            return true
        } catch { linkError = "\(error)"; linkStatus = "pairing failed"; return false }
    }

    func send(_ id: String) async {
        captures = queue.records
        await queue.send(id)
        captures = queue.records
        destination = link.destination
        if queue.shouldPoll(id) { pollOutcome(id) }
    }

    // MARK: outcome polling (additive capture_status)

    /// Polls `capture_status` every `pollInterval` until the Mac reports a
    /// final state, says it cannot answer, or `maxPolls` is reached. One task
    /// per capture; a relaunch resumes it for every received, non-final record.
    var pollInterval: TimeInterval = 2
    var maxPolls = 150
    private var pollers: [String: Task<Void, Never>] = [:]
    var isPollingOutcomes: Bool { !pollers.isEmpty }

    func pollOutcome(_ id: String) {
        pollers[id]?.cancel()
        pollers[id] = Task { @MainActor [weak self] in
            var n = 0
            while !Task.isCancelled {
                let interval: TimeInterval
                do {
                    guard let self else { return }
                    if n >= self.maxPolls || !self.queue.shouldPoll(id) {
                        self.pollers[id] = nil
                        return
                    }
                    n += 1
                    interval = self.pollInterval
                    await self.queue.refreshOutcome(id)
                }
                do {
                    guard let self else { return }
                    self.captures = self.queue.records
                    if !self.queue.shouldPoll(id) { break }
                }
                try? await Task.sleep(nanoseconds: UInt64(interval * 1_000_000_000))
            }
            self?.pollers[id] = nil
        }
    }

    /// One immediate probe (the list's Refresh button / tests).
    func refreshOutcome(_ id: String) async {
        await queue.refreshOutcome(id)
        captures = queue.records
    }

    /// After (re)connecting: resume polling for captures still awaiting an outcome.
    func resumeOutcomePolling() {
        for r in queue.records where queue.shouldPoll(r.id) && pollers[r.id] == nil { pollOutcome(r.id) }
    }

    /// Test/automation hook: `-flashtexpad-test-mac host:port:saltHex:fp:code`
    /// pairs with a listener the UI-test runner hosts on loopback.
    func pairFromLaunchArgument() {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: "-flashtexpad-test-mac"), i + 1 < args.count else { return }
        let parts = args[i + 1].split(separator: ":").map(String.init)
        guard parts.count == 5 else { linkError = "bad -flashtexpad-test-mac argument"; return }
        Task { await pair(host: parts[0], port: parts[1], saltHex: parts[2], fingerprint: parts[3], macName: "Test Mac", code: parts[4]) }
    }

    // MARK: documents

    static let bundledSample = "demo.tex"

    func openBundledSample() {
        guard let url = Bundle.main.url(forResource: "demo", withExtension: "tex") else { openError = "demo.tex missing from bundle"; return }
        open(url: url, title: PadModel.bundledSample)
    }

    func open(url: URL, title: String? = nil) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        do {
            let data = try Data(contentsOf: url)
            guard let text = String(data: data, encoding: .utf8) else { openError = "\(url.lastPathComponent) is not UTF-8"; return }
            setDocument(PadDocument(path: url.lastPathComponent, text: text), title: title ?? url.lastPathComponent)
        } catch { openError = error.localizedDescription }
    }

    /// The assistant-context recorded review fixture: loads its `main.tex`,
    /// its compiler diagnostics and its `proposal_review` as a pending review.
    func openReviewFixture() {
        do {
            let f = try loadWorkflowFixture()
            setDocument(PadDocument(path: f.sourcePath, text: f.sourceText, revision: f.compileResult.revision), title: "review-fixture/main.tex")
            diagnostics = DiagnosticsModel.items(from: f.compileResult, boundTo: document)
            diagnosticsSource = .fixture(name: "review-workflow.json compile_result", revision: f.compileResult.revision)
            attachFixtureReview(f)
        } catch { openError = "review fixture: \(error)" }
    }

    func loadWorkflowFixture() throws -> ReviewedProposal.WorkflowFixture {
        guard let url = Bundle.main.url(forResource: "review-workflow", withExtension: "json") else {
            throw ReviewedProposal.DecodeError.missingStep("bundled review-workflow.json")
        }
        return try ReviewedProposal.decodeWorkflowFixture(try Data(contentsOf: url))
    }

    func attachFixtureReview(_ f: ReviewedProposal.WorkflowFixture) {
        review = ReviewSession(review: f.review, approved: f.approved)
        reviewProvenance = .localFixture("crates/assistant-context/examples/review-workflow.json (provider_called:false)")
        reviewError = nil
    }

    private func setDocument(_ doc: PadDocument, title: String) {
        document = doc
        documentTitle = title
        caretUTF16 = 0
        openError = nil
        review = nil
        lastReceipt = nil
        diagnostics = []
        diagnosticsSource = .none
        refreshCompletions()
    }

    func textChanged(_ text: String) {
        guard var d = document, !d.text.sameBytes(as: text) else { return }
        d.text = text
        d.revision += 1
        document = d
        // A buffer edit invalidates a pending review's binding (sha/revision); keep it
        // pending so approve() refuses it with the exact reason rather than hiding it.
        refreshCompletions()
    }

    func caretMoved(_ utf16: Int) { caretUTF16 = utf16; refreshCompletions() }

    func refreshCompletions() {
        guard let d = document, let byte = d.byteOffset(ofUTF16: caretUTF16) else { completions = []; return }
        completions = LocalCompletion.suggestions(in: d.text, caretByte: byte)
    }

    /// Applies a completion at its byte range (the same offset discipline as an edit).
    func accept(_ s: LocalCompletion.Suggestion) {
        guard var d = document, let r = d.text.rangeOfUTF8(start: s.replaceStart, end: s.replaceEnd) else { return }
        d.text.replaceSubrange(r, with: s.text)
        d.revision += 1
        document = d
        caretUTF16 = NSRange(d.text.startIndex..<d.text.index(r.lowerBound, offsetBy: s.text.count), in: d.text).length
        refreshCompletions()
    }

    // MARK: diagnostics from a compile_result file

    func openCompileResult(url: URL) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        do {
            let r = try DiagnosticsModel.decode(try Data(contentsOf: url))
            diagnostics = DiagnosticsModel.items(from: r, boundTo: document)
            diagnosticsSource = .file(name: url.lastPathComponent, revision: r.revision)
        } catch { openError = "compile_result: \(error)" }
    }

    func openBundledCompileResult() {
        guard let url = Bundle.main.url(forResource: "compile-result", withExtension: "json") else { return }
        do {
            let r = try DiagnosticsModel.decode(try Data(contentsOf: url))
            diagnostics = DiagnosticsModel.items(from: r, boundTo: document)
            diagnosticsSource = .fixture(name: "protocol/fixtures/compile-result.json", revision: r.revision)
        } catch { openError = "compile_result: \(error)" }
    }

    // MARK: review gate

    func cancelReview() {
        review?.cancel()
    }

    func approveReview() {
        guard var r = review, var d = document else { return }
        do {
            let receipt = try r.approve(applyingTo: &d)
            document = d
            lastReceipt = receipt
            insertionCount += 1
            reviewError = nil
            refreshCompletions()
        } catch {
            reviewError = "\(error)"
        }
        review = r
    }

    // MARK: Mac link

    func pair(host: String, port: String, saltHex: String, fingerprint: String, macName: String, code: String) async {
        guard let p = UInt16(port) else { linkError = "port must be a number"; return }
        linkError = nil
        linkStatus = "pairing…"
        do {
            let pair = try await link.pair(host: host, port: p, saltHex: saltHex, fingerprint: fingerprint, macName: macName,
                                           code: code, companionName: "FlashTeXPad (\(UIDevice.current.name))")
            pairedMac = pair
            destination = link.destination
            linkStatus = "paired with \(pair.macName) (\(pair.pairId)); connected"
            rememberEndpoint(host: host, port: p, fingerprint: pair.fingerprint)
            resumeOutcomePolling()
        } catch { linkError = "\(error)"; linkStatus = "pairing failed" }
    }

    func forgetPairing() {
        if let p = pairedMac { try? link.store?.remove(fingerprint: p.fingerprint) }
        link.disconnect()
        pairedMac = nil
        linkStatus = "not paired"
    }

    /// QR pairing: the Mac's `flashtex-nearby://pair?v=1&code&salt&fp&name`
    /// (scanned with VisionKit, or pasted). Host/port typed when Bonjour
    /// cannot find the Mac (the simulator has no camera and, in tests, no
    /// advertising listener).
    @discardableResult
    func pair(bootstrapText: String, host: String, port: String) async -> Bool {
        linkError = nil
        let payload: NearbyBootstrapPayload
        do { payload = try NearbyBootstrapPayload.parse(bootstrapText) } catch { linkError = "QR payload: \(error)"; linkStatus = "pairing failed"; return false }
        let h = host.trimmingCharacters(in: .whitespaces)
        let p = UInt16(port.trimmingCharacters(in: .whitespaces))
        if !h.isEmpty, !port.isEmpty, p == nil { linkError = "port must be a number"; return false }
        linkStatus = h.isEmpty ? "pairing with \(payload.macName) (browsing Bonjour for fp \(payload.fingerprint))…" : "pairing with \(payload.macName)…"
        do {
            let pair = try await link.pair(bootstrap: payload, host: h.isEmpty ? nil : h, port: h.isEmpty ? nil : p,
                                           companionName: "FlashTeXPad (\(UIDevice.current.name))")
            pairedMac = pair
            destination = link.destination
            linkStatus = "paired with \(pair.macName) (\(pair.pairId)); connected"
            resumeOutcomePolling()
            return true
        } catch { linkError = "\(error)"; linkStatus = "pairing failed"; return false }
    }

    func reconnect(host: String, port: String) async {
        guard let pair = pairedMac else { linkError = "no stored pairing"; return }
        guard let p = UInt16(port) else { linkError = "port must be a number"; return }
        linkError = nil
        do {
            try await link.connect(host: host, port: p, pair: pair)
            connected(pair, host: host, port: p)
        } catch { linkError = "\(error)"; linkStatus = "connect failed" }
    }

    func refreshDestination() async {
        do { destination = try await link.destinationQuery() } catch { linkError = "\(error)" }
    }

    /// Sends the bundled 1×1 PNG (protocol/fixtures/capture-submission.json) as a
    /// capture with the given instructions and keeps the Mac's receipt.
    @discardableResult
    func sendTestCapture(instructions: String) async -> NearbyWire.CaptureReceived? {
        linkError = nil
        do {
            let r = try await link.submitCapture(image: PadModel.png1x1, mimeType: "image/png", instructions: instructions)
            lastCaptureReceived = r
            return r
        } catch { linkError = "\(error)"; return nil }
    }

    func disconnect() { link.disconnect(); linkStatus = "disconnected" }

    static let png1x1 = Data(base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4//8/AAX+Av4N70a4AAAAAElFTkSuQmCC")!
}
