import AppKit
import SwiftUI
import FlashTeXProtocol

// Experimental "v2 preview": paints a rendering-v2 display list (opened from a
// `flashtex-render --v2` JSON file) through `GlyphRunRenderer` and navigates
// from clusters to source bytes. Off by default; the runtime-v1 preview stays
// the product path. A list that fails validation or font resolution shows its
// diagnostic and NO page — never a partial frame.
//
// Threading: decoding, validation, font resolution and page preparation run
// on `V2Loader.queue` (serial, off-main); every result carries the load
// ticket it was started with and is dropped on the main thread if a newer
// load has since started (stale results never overwrite a newer state).
// Page bitmaps are rasterized off-main by `V2PageRasterizer` and installed
// only while their frame is still the one on screen.

/// Where a v2 display list came from.
enum V2Source: Equatable {
    /// `File > Open Display List (v2)…` / `FLASHTEX_V2_FILE`.
    case file(URL)
    /// The negotiated `display_list` sibling line of a `compile_result`
    /// (docs/contracts/runtime-v1-display-list-v2.md): request id, project,
    /// revision, and the line's bytes (kept for the current frame so tools that
    /// take a list file — the exact PDF export — can run on a live frame).
    case worker(requestID: String, projectId: String, revision: Int, line: Data)

    var label: String {
        switch self {
        case .file(let u): u.lastPathComponent
        case .worker(let id, _, let revision, _): "live \(id) (revision \(revision))"
        }
    }
    var url: URL? { if case .file(let u) = self { u } else { nil } }
    var isLive: Bool { if case .worker = self { true } else { false } }

    /// A file holding the list: the opened file, or the live line written to a
    /// temporary file named by request id and revision.
    func listFileURL() throws -> URL {
        switch self {
        case .file(let u): return u
        case .worker(let id, _, let revision, let line):
            let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-v2-live", isDirectory: true)
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            let url = dir.appendingPathComponent("\(id)-r\(revision).json")
            try line.write(to: url, options: .atomic)
            return url
        }
    }
}

/// The negotiated live route: `display-list-v2` in `layout_capabilities`
/// (docs/contracts/runtime-v1-display-list-v2.md). Requested only while the
/// v2 pane is visible; the sibling line is applied only for the currently
/// applied compile_result.
enum V2Live {
    static let capability = "display-list-v2"
    private static let lock = NSLock()
    private(set) static var staleLinesDropped = 0
    private(set) static var unsolicitedLinesDropped = 0
    private(set) static var linesAccepted = 0
    /// Siblings refused before paint because a declared document's sha256 /
    /// byte_length is not the text the applied compile_result was requested
    /// with (D1); the previously verified frame stays on screen.
    private(set) static var sourceMismatchesRefused = 0
    /// The last live sibling refusal that kept the previous frame (tests/evidence).
    private(set) static var lastLiveRefusal: RenderingV2.ValidationError?
    static func note(stale: Bool = false, unsolicited: Bool = false, accepted: Bool = false, sourceMismatch: Bool = false, liveRefusal: RenderingV2.ValidationError? = nil) {
        lock.lock(); defer { lock.unlock() }
        if let liveRefusal { lastLiveRefusal = liveRefusal }
        if stale { staleLinesDropped += 1 }
        if unsolicited { unsolicitedLinesDropped += 1 }
        if accepted { linesAccepted += 1 }
        if sourceMismatch { sourceMismatchesRefused += 1 }
    }

    /// D1 (draft contract L28–30, L62): every document the sibling declares
    /// must be one the applied compile_result was requested with, with that
    /// text's exact byte length and raw SHA-256 — checked BEFORE paint, off
    /// the main thread, against the request text (`compiledDocuments`), the
    /// same binding the helper route applies to durable text
    /// (`DisplayCandidateValidator.validate`). Returns the typed refusal.
    static func sourceBindingFailure(of list: RenderingV2.DisplayList, requestID: String, compiled: [String: String]) -> RenderingV2.ValidationError? {
        guard !list.documents.isEmpty else {
            return RenderingV2.ValidationError(code: "source_mismatch", message: "display_list \(requestID) declares no documents")
        }
        for doc in list.documents {
            guard let text = compiled[doc.path] else {
                return RenderingV2.ValidationError(code: "source_mismatch", message: "display_list \(requestID) declares \(doc.path), which the compile request did not carry")
            }
            guard Int64(text.utf8.count) == doc.byteLength else {
                return RenderingV2.ValidationError(code: "source_mismatch", message: "display_list \(requestID): \(doc.path) byte_length \(doc.byteLength) differs from the requested text (\(text.utf8.count) bytes)")
            }
            guard SourceDigest.sha256Hex(text) == doc.sha256 else {
                return RenderingV2.ValidationError(code: "source_mismatch", message: "display_list \(requestID): \(doc.path) sha256 \(doc.sha256.prefix(12))… differs from the requested text")
            }
        }
        return nil
    }
}

/// The newest list that arrived while another was being prepared: it starts
/// when the in-flight preparation delivers. Lists that arrived in between
/// are dropped undecoded (coalesced), so visible progress keeps up with
/// preparation instead of being starved by strict supersession.
struct V2QueuedLoad {
    var source: V2Source
    var prepare: @Sendable () -> V2Loader.Outcome
    var completion: (() -> Void)?
}

/// What the shell holds for the v2 pane.
enum V2PreviewState {
    /// A load is in flight. `previous` is the frame still on screen, shown
    /// with an explicit stale indicator until the new one is verified
    /// (rendering-v2 proposal: retain the old frame, label it stale).
    /// `queued` is the newest list waiting for this preparation to finish;
    /// `previousSource` is where `previous` came from (restored on a live refusal, D1).
    case loading(V2Source, ticket: Int, previous: V2Frame?, queued: V2QueuedLoad? = nil, previousSource: V2Source? = nil)
    case loaded(V2Frame, V2Source)
    case failed(RenderingV2.ValidationError, V2Source)

    var source: V2Source {
        switch self {
        case .loading(let s, _, _, _, _), .loaded(_, let s), .failed(_, let s): s
        }
    }
    var queued: V2QueuedLoad? { if case .loading(_, _, _, let q, _) = self { q } else { nil } }
    /// The file URL when the list came from a file (tests, evidence hook).
    var url: URL? { source.url }
    /// The verified frame the pane may paint (a stale one while loading).
    var frame: V2Frame? {
        switch self {
        case .loaded(let f, _): f
        case .loading(_, _, let previous, _, _): previous
        case .failed: nil
        }
    }
    var isLoading: Bool { if case .loading = self { true } else { false } }
    var ticket: Int? { if case .loading(_, let t, _, _, _) = self { t } else { nil } }
    /// The previously verified frame retained while loading, with its source.
    var retained: (frame: V2Frame, source: V2Source)? {
        switch self {
        case .loading(_, _, let previous?, _, let previousSource?): (previous, previousSource)
        case .loaded(let f, let s): (f, s)
        default: nil
        }
    }
}

/// Off-main preparation of display lists with monotonically increasing load
/// tickets. `ShellModel.loadDisplayListV2` starts a load; the result reaches
/// the main run loop (same delivery as `WorkerClient`: a run-loop block plus
/// wake-up, not a plain dispatch) and is published only if its ticket is
/// still the one the shell is waiting for.
enum V2Loader {
    static let queue = DispatchQueue(label: "flashtex.preview-v2.prepare", qos: .userInitiated)
    private static let lock = NSLock()
    private static var nextTicket = 1
    /// Results dropped because a newer load superseded them (tests/evidence).
    private(set) static var staleResultsDropped = 0
    private(set) static var resultsPublished = 0
    /// Lists dropped undecoded because a newer one arrived while one was in flight.
    private(set) static var coalescedLoads = 0

    static func issueTicket() -> Int { lock.lock(); defer { lock.unlock() }; let t = nextTicket; nextTicket += 1; return t }
    static func noteDropped() { lock.lock(); staleResultsDropped += 1; lock.unlock() }
    static func notePublished() { lock.lock(); resultsPublished += 1; lock.unlock() }
    static func noteCoalesced() { lock.lock(); coalescedLoads += 1; lock.unlock() }

    enum Outcome {
        case loaded(V2Frame)
        case failed(RenderingV2.ValidationError)
    }

    /// Bitmaps rasterized in the same off-main job as the preparation, at the
    /// pane's last requested pixels-per-point/appearance, so the publish and
    /// the first blit happen in one main-thread pass instead of three. Keyed
    /// by page content token (`V2Frame.pageTokens`).
    struct Prerastered {
        var pixelsPerPoint: Double
        var dark: Bool
        var images: [(token: String, image: CGImage)]
    }

    /// What the pane last asked for and which page tokens it already holds at
    /// that scale/appearance: the loader pre-rasterizes only the pages whose
    /// content is new. Captured on the main thread before the off-main job.
    struct RasterHint {
        var pixelsPerPoint: Double
        var dark: Bool
        var cachedTokens: Set<String> = []
    }

    static func preraster(_ frame: V2Frame, pixelsPerPoint: Double, dark: Bool, skipping cached: Set<String> = []) -> Prerastered {
        var images: [(token: String, image: CGImage)] = []
        for (index, page) in frame.prepared.enumerated() {
            let token = frame.pageToken(at: index)
            if cached.contains(token) { continue }
            if let image = GlyphRunRenderer.rasterize(page, scale: pixelsPerPoint, dark: dark) { images.append((token, image)) }
        }
        return Prerastered(pixelsPerPoint: pixelsPerPoint, dark: dark, images: images)
    }

    static func preraster(_ frame: V2Frame, hint: RasterHint) -> Prerastered {
        preraster(frame, pixelsPerPoint: hint.pixelsPerPoint, dark: hint.dark, skipping: hint.cachedTokens)
    }

    /// Pure preparation: file → decoded, validated, font-resolved, page-prepared frame.
    static func prepare(url: URL, store: V2FontStore = .shared) -> Outcome {
        do {
            return prepare(data: try Data(contentsOf: url), store: store)
        } catch {
            return .failed(RenderingV2.ValidationError(code: "io_error", message: error.localizedDescription))
        }
    }

    /// Pure preparation of one `display_list` envelope's bytes (a file or the
    /// worker's sibling line). Pages whose raw bytes were prepared before
    /// under the same font manifest are reused from `cache` (V2PageCache.swift);
    /// decoding, validation and font resolution are otherwise unchanged.
    static func prepare(data: Data, store: V2FontStore = .shared, cache: V2PageCache? = .shared, images: V2ImageStore = .shared) -> Outcome {
        do {
            return .loaded(try V2Frame.prepare(data: data, store: store, cache: cache, images: images))
        } catch let error as RenderingV2.ValidationError {
            return .failed(error)
        } catch {
            return .failed(RenderingV2.ValidationError(code: "io_error", message: error.localizedDescription))
        }
    }

    /// Runs `block` at the head of the next main run-loop iteration.
    static func deliverOnMain(_ block: @escaping @Sendable () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue, block)
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }
}

extension ShellModel {
    /// `File > Open Display List (v2)…`
    func openDisplayListV2Panel() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.message = "Choose a rendering-v2 display_list JSON file (flashtex-render --v2)"
        if panel.runModal() == .OK, let url = panel.url { loadDisplayListV2(url: url) }
    }

    /// Starts an off-main load. The previous verified frame (if any) stays on
    /// screen with a stale indicator until this load finishes; a result whose
    /// ticket is no longer current is dropped. `completion` runs on the main
    /// thread after the state was (or was not) published.
    func loadDisplayListV2(url: URL, completion: (() -> Void)? = nil) {
        previewV2 = true
        // Image paths of an opened list resolve under the open project, else
        // under the list file's own directory (still rooted, no symlinks).
        V2ImageStore.shared.root = project.projectRoot ?? url.deletingLastPathComponent()
        startDisplayListV2(source: .file(url), completion: completion) { V2Loader.prepare(url: url) }
    }

    /// Requests (or stops requesting) the negotiated live route while the v2
    /// pane is visible: `display-list-v2` joins `requestedLayoutCapabilities`,
    /// whose change re-requests the current revision under auto-compile. The
    /// v1 pages of every result keep painting the product preview.
    func setLiveV2(_ on: Bool) {
        // `display-list-v2-images` and `display-list-v2-links` ride along
        // (each proposal §1: accepted only with `display-list-v2`); one
        // assignment so a switch re-requests once. Compact (#257) is a
        // per-request encoding, not a mode-set sibling — left untouched.
        var caps = requestedLayoutCapabilities
        caps.removeAll { $0 == V2Live.capability || $0 == RenderingV2.imagesCapability || $0 == RenderingV2.linksCapability }
        if on { caps += [V2Live.capability, RenderingV2.imagesCapability, RenderingV2.linksCapability] }
        if caps != requestedLayoutCapabilities { requestedLayoutCapabilities = caps }
    }

    /// Whether the applied result negotiated the live route.
    var liveV2Accepted: Bool { negotiation.accepted.contains(V2Live.capability) }

    /// The worker's `display_list` sibling line for request `id`
    /// (`WorkerClient.Event.displayList`). Applied only when `id` is the id of
    /// the currently applied `compile_result` and that result accepted
    /// `display-list-v2`; anything else is stale or unsolicited and is dropped
    /// — a late v2 frame never replaces a newer one. Preparation is off-main;
    /// the payload's project/revision must match the applied result.
    func receiveDisplayListV2(id: String, line: Data, completion: (() -> Void)? = nil) {
        guard let result, resultID == id, previewSource != .fixture else {
            V2Live.note(stale: true)
            log("ignored stale display_list \(id) (\(line.count) B): applied result is \(resultID ?? "none")")
            completion?()
            return
        }
        guard liveV2Accepted else {
            V2Live.note(unsolicited: true)
            let msg = "display_list \(id) arrived but \(V2Live.capability) was not accepted for that result (accepted: \(negotiation.accepted.joined(separator: ", ")))"
            log("rejected unsolicited " + msg)
            workerStatus = "protocol violation: " + msg
            completion?()
            return
        }
        V2Live.note(accepted: true)
        V2ImageStore.shared.root = project.projectRoot // the directory the request's project_root named
        let expectedProject = result.projectId, expectedRevision = result.revision
        let compiled = compiledDocuments // the exact text the applied compile_result was requested with (D1)
        // display-list-v2-delta: a `display_list_delta` sibling is applied to the
        // installed base captured here (main thread), off-main; the reconstructed
        // list then goes through the unchanged full validation. Any refusal drops
        // the base so the next request asks for a full frame.
        let isDelta = RenderingV2Fast.header(line)?.type == DisplayListDelta.messageType
        let installed = deltaInstalled
        if isDelta {
            guard negotiation.accepted.contains(DisplayListDelta.capability) else {
                let msg = "display_list_delta \(id) arrived but \(DisplayListDelta.capability) was not accepted for that result (accepted: \(negotiation.accepted.joined(separator: ", ")))"
                log("rejected unsolicited " + msg)
                workerStatus = "protocol violation: " + msg
                deltaInstalled = nil
                completion?()
                return
            }
            guard installed != nil else {
                log("preview-v2: display_list_delta \(id) without an installed base; requesting a full frame")
                workerStatus = "display_list_delta refused: [delta_base_mismatch] no installed base"
                completion?()
                return
            }
        }
        startDisplayListV2(source: .worker(requestID: id, projectId: expectedProject, revision: expectedRevision, line: line), completion: completion) {
            let outcome: V2Loader.Outcome
            if isDelta, let installed {
                do {
                    let delta = try RenderingV2Fast.delta(line, maxPages: DisplayListDelta.maxSnapshotPages)
                    let (envelope, pageBytes, target) = try DisplayListDelta.apply(delta, to: installed)
                    var frame = try V2Frame.prepare(envelope)
                    frame.installedBase = DisplayListDelta.installed(from: envelope, pageBytes: pageBytes, lineBytes: target)
                    frame.reusedPages = delta.pageCount - delta.changedPages.count
                    outcome = .loaded(frame)
                } catch let refusal as DisplayListDelta.Refusal {
                    outcome = .failed(refusal.asValidationError)
                } catch let error as RenderingV2.ValidationError {
                    outcome = .failed(error)
                } catch {
                    outcome = .failed(RenderingV2.ValidationError(code: "delta_undecodable", message: "\(error)"))
                }
            } else {
                outcome = V2Loader.prepare(data: line)
            }
            switch outcome {
            case .loaded(let frame) where frame.list.projectId != expectedProject || frame.list.revision != expectedRevision:
                return .failed(RenderingV2.ValidationError(code: "correlation_mismatch",
                                                           message: "display_list \(id) is for project \(frame.list.projectId) revision \(frame.list.revision); the compile_result is project \(expectedProject) revision \(expectedRevision)"))
            case .loaded(var frame):
                if let refusal = V2Live.sourceBindingFailure(of: frame.list, requestID: id, compiled: compiled) {
                    V2Live.note(sourceMismatch: true)
                    return .failed(refusal)
                }
                if !isDelta, DisplayListDelta.enabled, frame.installedBase == nil, let pageBytes = frame.pageBytes {
                    let envelope = RenderingV2.Envelope(protocolVersion: RenderingV2.protocolVersion, id: frame.id, type: RenderingV2.messageType, payload: frame.list)
                    frame.installedBase = DisplayListDelta.installed(from: envelope, pageBytes: pageBytes, lineBytes: line.count)
                }
                return .loaded(frame)
            case .failed: return outcome
            }
        }
    }

    /// Starts an off-main preparation. The previous verified frame (if any)
    /// stays on screen with a stale indicator until it finishes; a result
    /// whose ticket is no longer current is dropped. `completion` runs on the
    /// main thread after the state was (or was not) published.
    private func startDisplayListV2(source: V2Source, completion: (() -> Void)?, prepare: @escaping @Sendable () -> V2Loader.Outcome) {
        // One preparation in flight; the newest arrival waits, older waiting ones are dropped undecoded.
        if case .loading(let inFlight, let ticket, let previous, let queued, let previousSource) = displayListV2 {
            if let queued {
                V2Loader.noteCoalesced()
                if TypingBench.isBenchActive { FlashTeXLog.write("preview-v2: coalesced \(queued.source.label) behind \(source.label) at \(MonotonicClock.nowNs())") }
                queued.completion?()
            }
            displayListV2 = .loading(inFlight, ticket: ticket, previous: previous, queued: V2QueuedLoad(source: source, prepare: prepare, completion: completion), previousSource: previousSource)
            return
        }
        let ticket = V2Loader.issueTicket()
        let retained = displayListV2?.retained
        displayListV2 = .loading(source, ticket: ticket, previous: retained?.frame, previousSource: retained?.source)
        if !source.isLive { captureNote = "Loading display list \(source.label)…" }
        let t0 = MonotonicClock.nowNs()
        if TypingBench.isBenchActive { FlashTeXLog.write("preview-v2: preparing \(source.label) ticket \(ticket) at \(t0)") }
        let rasterHint = V2PageRasterizer.shared.rasterHint
        V2Loader.queue.async {
            let outcome = prepare()
            let t1 = MonotonicClock.nowNs()
            var prerastered: V2Loader.Prerastered?
            if case .loaded(let frame) = outcome, let hint = rasterHint {
                prerastered = V2Loader.preraster(frame, hint: hint)
            }
            let t2 = MonotonicClock.nowNs()
            let prerasteredResult = prerastered // immutable copy for the Sendable delivery closure
            V2Loader.deliverOnMain {
                MainActor.assumeIsolated {
                    if TypingBench.isBenchActive {
                        let reused = { if case .loaded(let f) = outcome { "\(f.reusedPages)/\(f.prepared.count)" } else { "-" } }()
                        FlashTeXLog.write("preview-v2: prepared \(source.label) in \(Double(t1 &- t0) / 1e6) ms (reused pages \(reused)), prerastered \(prerasteredResult?.images.count ?? 0) page(s) in \(Double(t2 &- t1) / 1e6) ms, delivered \(Double(MonotonicClock.nowNs() &- t2) / 1e6) ms later")
                    }
                    self.deliverDisplayListV2(ticket: ticket, source: source, outcome: outcome, prerastered: prerasteredResult)
                    completion?()
                }
            }
        }
    }

    /// Publishes a prepared result only if `ticket` is the load the shell is
    /// still waiting for. Anything else (a superseded load, or the pane was
    /// reset) is dropped: a stale frame never overwrites a newer state.
    /// Returns whether the result was published.
    @discardableResult
    func deliverDisplayListV2(ticket: Int, source: V2Source, outcome: V2Loader.Outcome, prerastered: V2Loader.Prerastered? = nil) -> Bool {
        guard displayListV2?.ticket == ticket else {
            V2Loader.noteDropped()
            FlashTeXLog.write("preview-v2: dropped stale load result ticket \(ticket) for \(source.label) (current: \(displayListV2?.ticket.map(String.init) ?? "none"))")
            return false
        }
        let queued = displayListV2?.queued
        defer {
            // The newest list that arrived meanwhile starts now, over the frame just published.
            if let queued { startDisplayListV2(source: queued.source, completion: queued.completion, prepare: queued.prepare) }
        }
        V2Loader.notePublished()
        switch outcome {
        case .loaded(let frame):
            // Bitmaps first, so the render pass this publish triggers blits them.
            if let prerastered { V2PageRasterizer.shared.preinstall(prerastered, frame: frame) }
            displayListV2 = .loaded(frame, source)
            // Installation (proposal r5 §6.1): only a published live frame is a base.
            if source.isLive { deltaInstalled = frame.installedBase } else { deltaInstalled = nil }
            if TypingBench.isBenchActive { FlashTeXLog.write("preview-v2: published \(source.label) revision \(frame.list.revision) at \(MonotonicClock.nowNs())") }
            captureNote = "\(source.isLive ? "Live display list" : "Loaded display list") \(source.label): \(frame.list.pages.count) page(s), \(frame.fonts.count) font(s) resolved by content hash, \(frame.prepared.reduce(0) { $0 + $1.glyphCount }) glyphs prepared"
            V2ParityEvidence.runIfRequested(frame: frame, source: source)
        case .failed(let error):
            if source.isLive { deltaInstalled = nil } // full resync on the next request
            if source.isLive, let retained = displayListV2?.retained {
                // A refused LIVE sibling (D1 source binding, correlation, validation) never
                // un-verifies the frame already on screen: it stays, and is stale by the
                // header's revision label (`v2-behind`) whenever the applied result moved on.
                displayListV2 = .loaded(retained.frame, retained.source)
                V2Live.note(liveRefusal: error)
                captureNote = "Display list refused (previous frame kept): \(error)"
                workerStatus = "display_list refused: [\(error.code)] \(error.message)"
                log("preview-v2: refused \(source.label): [\(error.code)] \(error.message); keeping \(retained.source.label)")
            } else {
                // A refusal drops the previous frame: nothing verified is on screen.
                displayListV2 = .failed(error, source)
                captureNote = "Display list refused: \(error)"
            }
        }
        return true
    }

    /// Click in the v2 preview → source selection. The display list's document
    /// digest must match the current buffer (the list attests exactly which
    /// bytes it laid out) — or, when the buffer moved on, the recorded compile
    /// text (`compiledDocuments`, the exact request text the applied result and
    /// its sibling were produced for) must carry that digest, in which case
    /// `navigate(to:expectedText:)` rebases the span across the single edited
    /// region (`Navigation.rebaseExactly`: refused when the span overlaps the
    /// edit, verified to spell the same bytes otherwise). Any other mismatch
    /// is refused: a click on a stale frame never lands on other bytes.
    ///
    /// A hit naming a document that is not open in this window (the helper
    /// compiled an include this window never read) opens it through the
    /// helper first (ShellModel+UnopenedNavigation.swift): the navigation
    /// then completes asynchronously, or is refused typed when the durable
    /// revision differs from the declared one.
    func navigateV2(_ hit: V2Geometry.Hit) {
        if hit.syntheticReason == nil, let source = hit.sources.first, !documents.contains(where: { $0.path == source.path }),
           controllerAttached, displayListV2?.frame?.list.documents.contains(where: { $0.path == source.path }) == true {
            navigationNote = "Opening \(source.path) through the helper to navigate…"
            Task { @MainActor [weak self] in _ = await self?.navigateV2Opening(hit) }
            return
        }
        navigateV2Now(hit)
    }

    /// `navigateV2` for a hit whose document is not open: opens it at the
    /// declared durable revision, then navigates on the (re-read) current
    /// frame. Returns the open verdict (nil when the document was open).
    @discardableResult
    func navigateV2Opening(_ hit: V2Geometry.Hit) async -> UnopenedNavigation.Verdict? {
        guard hit.syntheticReason == nil, let source = hit.sources.first else { navigateV2Now(hit); return nil }
        guard !documents.contains(where: { $0.path == source.path }) else { navigateV2Now(hit); return nil }
        guard let declared = displayListV2?.frame?.list.documents.first(where: { $0.path == source.path }) else {
            navigationNote = "Display list does not declare document \(source.path)."
            return nil
        }
        if let why = historicalRefusal(of: "navigation") { navigationNote = why; return nil }
        // The comparator is the helper's durable version the applied preview was compiled
        // from (`compiledRevision(for:)`), never the producer's `documents[].revision`
        // (its compile revision); the declared sha256 binds the opened bytes exactly.
        let verdict = await openForNavigation(path: source.path, compiledRevision: compiledRevision(for: source.path), compiledSHA256: declared.sha256)
        guard case .opened(_, let revision) = verdict else {
            if case .alreadyOpen = verdict { navigateV2Now(hit); return verdict }
            navigationNote = "Preview → source: " + verdict.note
            return verdict
        }
        navigateV2Now(hit)
        if navigationNote?.hasPrefix("Selected") == true { navigationNote! += " (opened \(source.path) at durable r\(revision))" }
        return verdict
    }

    private func navigateV2Now(_ hit: V2Geometry.Hit) {
        if let reason = hit.syntheticReason {
            navigationNote = "Generated content (\(reason)) has no source range."
            return
        }
        guard let source = hit.sources.first else {
            navigationNote = "This item has no source mapping."
            return
        }
        guard let frame = displayListV2?.frame else { return }
        guard let declared = frame.list.documents.first(where: { $0.path == source.path }) else {
            navigationNote = "Display list does not declare document \(source.path)."
            return
        }
        guard let doc = documents.first(where: { $0.path == source.path }) else {
            navigationNote = "No open document named \(source.path)."
            return
        }
        let attestsBuffer = SourceDigest.sha256Hex(doc.text) == declared.sha256
        let compiled = compiledText(for: source.path)
        let attestsCompiled = !attestsBuffer && compiled.map { SourceDigest.sha256Hex($0) == declared.sha256 } == true
        guard attestsBuffer || attestsCompiled else {
            navigationNote = "Display list is for \(source.path) revision \(declared.revision) (sha256 \(declared.sha256.prefix(12))…), which differs from the current buffer and from the recorded compile text; regenerate the display list to navigate."
            return
        }
        // The baseline is the text the list attests: the buffer itself, or the recorded compile text it was rebased from.
        navigateExactly(to: source, expectedText: hit.text, compiledText: attestsBuffer ? doc.text : compiled)
        if attestsCompiled, navigationNote?.hasPrefix("Selected") == true {
            navigationNote! += " (display list revision \(declared.revision) rebased onto the edited buffer)"
        }
        if hit.sources.count > 1, navigationNote?.hasPrefix("Selected") == true {
            navigationNote! += " (+\(hit.sources.count - 1) more source range(s) for this cluster)"
        }
    }

    /// `Export PDF (v2)…` from the v2 pane: the same draw routine as the preview.
    func exportPDFV2() {
        guard case .loaded(let frame, _)? = displayListV2 else {
            captureNote = displayListV2?.isLoading == true ? "Nothing to export yet: a display list is still loading." : "Nothing to export: no display list loaded."
            return
        }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.pdf]
        panel.nameFieldStringValue = "\(frame.list.projectId)-r\(frame.list.revision)-v2.pdf"
        panel.message = "Export the v2 display list as PDF through the preview's draw routine (experimental)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try GlyphRunRenderer.pdfData(frame: frame).write(to: url, options: .atomic)
            captureNote = "Exported \(frame.list.pages.count) page(s) (v2) to \(url.path)"
        } catch {
            captureNote = "PDF export (v2) failed: \(error.localizedDescription)"
        }
    }
}

/// Off-main page bitmaps for the pane, keyed by page content identity
/// (`V2Frame.pageTokens`: the page bytes' hash, length and font manifest
/// digest), scale and appearance. A page is rasterized once through
/// `GlyphRunRenderer.rasterize` (the same routine and bitmap configuration
/// the export parity check compares) and then only blitted, so hover/caret
/// repaints never re-run glyph drawing on the main thread, and a frame that
/// carries a page with the same bytes as the frame before it keeps that
/// page's bitmap. Bitmaps whose page is not in the current frame are dropped
/// when they arrive (stale paint suppression) and evicted when the frame
/// changes. Retention is bounded in bytes; the least recently requested
/// pages go first.
@MainActor
@Observable
final class V2PageRasterizer {
    struct Key: Hashable {
        var pageToken: String
        /// Pixels per PDF point (display scale × backing scale).
        var pixelsPerPoint: Double
        var dark: Bool
    }

    static let shared = V2PageRasterizer()
    static let queue = DispatchQueue(label: "flashtex.preview-v2.raster", qos: .userInteractive)

    /// One observable slot per key: a page body reads its own slot's `image`,
    /// so a bitmap arriving for page 3 re-evaluates page 3 only.
    @Observable final class Slot { var image: CGImage? }
    @ObservationIgnored private var slots: [Key: Slot] = [:]
    /// Ready bitmaps (bookkeeping/tests; views observe their slot, not this).
    @ObservationIgnored private(set) var images: [Key: CGImage] = [:]
    @ObservationIgnored private var order: [Key] = []
    @ObservationIgnored private var inFlight: Set<Key> = []
    /// Observed: a page body that asked before its frame became current
    /// re-evaluates (and re-requests) when `setCurrent` runs.
    private(set) var currentTokens: Set<String> = []
    @ObservationIgnored private(set) var retainedBytes = 0
    @ObservationIgnored let maxBytes: Int
    @ObservationIgnored private(set) var staleBitmapsDropped = 0
    @ObservationIgnored private(set) var rasterizations = 0
    /// The pane's most recent pixels-per-point/appearance: the loader
    /// pre-rasterizes new frames with it off-main.
    @ObservationIgnored private(set) var lastRequest: (pixelsPerPoint: Double, dark: Bool)?
    @ObservationIgnored private(set) var preinstalled = 0

    init(maxBytes: Int = 192 << 20) { self.maxBytes = maxBytes }

    /// Marks the pages of `frame` as the ones on screen; bitmaps of any other
    /// page are evicted now and dropped if still in flight.
    func setCurrent(frame: V2Frame?) { setCurrent(pageTokens: frame.map { Set($0.pageTokens.indices.map($0.pageToken(at:))) } ?? []) }

    func setCurrent(pageTokens: Set<String>) {
        guard currentTokens != pageTokens else { return }
        currentTokens = pageTokens
        for key in order where !pageTokens.contains(key.pageToken) { drop(key) }
        order.removeAll { !pageTokens.contains($0.pageToken) }
    }

    /// The loader's pre-raster hint: the last requested scale/appearance and
    /// the page tokens already held at it (nil before the pane asked once).
    var rasterHint: V2Loader.RasterHint? {
        guard let last = lastRequest else { return nil }
        return V2Loader.RasterHint(pixelsPerPoint: last.pixelsPerPoint, dark: last.dark, cachedTokens: cachedTokens(pixelsPerPoint: last.pixelsPerPoint, dark: last.dark))
    }

    func cachedTokens(pixelsPerPoint: Double, dark: Bool) -> Set<String> {
        Set(images.keys.filter { $0.pixelsPerPoint == pixelsPerPoint && $0.dark == dark }.map(\.pageToken))
    }

    /// The bitmap for `page` if ready; otherwise starts rasterizing it
    /// off-main and returns nil (the page paints its background until the
    /// bitmap arrives on the next run-loop turn).
    func image(for page: V2PreparedPage, pageToken: String, pixelsPerPoint: Double, dark: Bool) -> CGImage? {
        let key = Key(pageToken: pageToken, pixelsPerPoint: pixelsPerPoint, dark: dark)
        lastRequest = (pixelsPerPoint, dark)
        let slot: Slot
        if let existing = slots[key] { slot = existing } else { slot = Slot(); slots[key] = slot }
        if let image = slot.image { // observed read: this body follows this page's bitmap only
            touch(key)
            return image
        }
        guard currentTokens.contains(pageToken), !inFlight.contains(key) else { return nil }
        inFlight.insert(key)
        let t0 = MonotonicClock.nowNs()
        Self.queue.async {
            let t1 = MonotonicClock.nowNs()
            let image = GlyphRunRenderer.rasterize(page, scale: pixelsPerPoint, dark: dark)
            let t2 = MonotonicClock.nowNs()
            V2Loader.deliverOnMain {
                MainActor.assumeIsolated {
                    if TypingBench.isBenchActive {
                        FlashTeXLog.write("preview-v2: raster page \(page.number) \(key.pageToken.prefix(24)): queued \(Double(t1 &- t0) / 1e6) ms, drew \(Double(t2 &- t1) / 1e6) ms, delivered \(Double(MonotonicClock.nowNs() &- t2) / 1e6) ms later at \(MonotonicClock.nowNs())")
                    }
                    self.install(image, for: key)
                }
            }
        }
        return nil
    }

    /// Installs an arrived bitmap unless its page is no longer current.
    func install(_ image: CGImage?, for key: Key) {
        inFlight.remove(key)
        rasterizations += 1
        guard currentTokens.contains(key.pageToken), let image else {
            staleBitmapsDropped += 1
            FlashTeXLog.write("preview-v2: dropped stale page bitmap \(key.pageToken.prefix(24)) (not in the current frame)")
            return
        }
        images[key] = image
        (slots[key] ?? { let s = Slot(); slots[key] = s; return s }()).image = image
        order.removeAll { $0 == key }
        order.append(key)
        retainedBytes += image.bytesPerRow * image.height
        while retainedBytes > maxBytes, let oldest = order.first, oldest != key {
            order.removeFirst()
            drop(oldest)
        }
    }

    /// Installs bitmaps rasterized off-main together with `frame` and makes
    /// its pages current (evicting other pages' bitmaps; pages the new frame
    /// shares with the previous one keep theirs), immediately before the
    /// frame is published — the pass that shows the frame finds its bitmaps
    /// ready.
    func preinstall(_ prerastered: V2Loader.Prerastered, frame: V2Frame) {
        setCurrent(frame: frame)
        for (token, image) in prerastered.images {
            install(image, for: Key(pageToken: token, pixelsPerPoint: prerastered.pixelsPerPoint, dark: prerastered.dark))
            preinstalled += 1
        }
    }

    func clear() { for key in order { drop(key) }; order.removeAll() }

    private func touch(_ key: Key) {
        if let i = order.firstIndex(of: key) { order.remove(at: i); order.append(key) }
    }

    private func drop(_ key: Key) {
        if let image = images.removeValue(forKey: key) { retainedBytes -= image.bytesPerRow * image.height }
        slots[key]?.image = nil
        slots.removeValue(forKey: key)
    }
}

/// `FLASHTEX_V2_PARITY_OUT=<dir>`: after every successful load, run the
/// zero-tolerance export-versus-preview comparison off-main and write
/// `parity.json`, `export.pdf` and per-page `page-N-preview.png` /
/// `page-N-export.png` there (evidence capture; never in the product path).
/// `FLASHTEX_V2_PARITY_SCALE` sets pixels per point (default 2).
enum V2ParityEvidence {
    static func runIfRequested(frame: V2Frame, source: V2Source) {
        guard let dir = ProcessInfo.processInfo.environment["FLASHTEX_V2_PARITY_OUT"], !dir.isEmpty else { return }
        let scale = Double(ProcessInfo.processInfo.environment["FLASHTEX_V2_PARITY_SCALE"] ?? "") ?? 2
        V2Loader.queue.async {
            var out = URL(fileURLWithPath: dir)
            if case .worker(let id, _, let revision, _) = source { out.appendPathComponent("live-\(id)-r\(revision)") }
            try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
            let report = V2Parity.compare(frame: frame, scale: scale) { page in
                write(page.preview, to: out.appendingPathComponent("page-\(page.page)-preview.png"))
                write(page.export, to: out.appendingPathComponent("page-\(page.page)-export.png"))
            }
            try? GlyphRunRenderer.pdfData(frame: frame).write(to: out.appendingPathComponent("export.pdf"))
            let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            var object: [String: Any] = [:]
            if let data = try? encoder.encode(report), let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] { object = json }
            object["source"] = source.url?.path ?? source.label
            object["identical"] = report.identical
            object["total_differing_pixels"] = report.totalDifferingPixels
            if let data = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys]) {
                try? data.write(to: out.appendingPathComponent("parity.json"))
            }
            FlashTeXLog.write("preview-v2: parity \(report.identical ? "identical" : "DIFFERS") — \(report.totalDifferingPixels) differing pixel(s) over \(report.pages.count) page(s) at \(scale)px/pt for \(source.label) → \(out.path)")
        }
    }

    static func write(_ image: CGImage, to url: URL) {
        let rep = NSBitmapImageRep(cgImage: image)
        try? rep.representation(using: .png, properties: [:])?.write(to: url)
    }
}

/// The v2 preview pane: header, pages or the refusal, and the list's diagnostics.
struct PreviewV2Pane: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            // The pages sit at ONE structural position whether the frame is the
            // loaded one or the previous one shown stale while a load is in
            // flight. Under typing the state toggles loaded -> stale -> loaded
            // for every keystroke; as separate `switch` branches each toggle
            // tore down and rebuilt the scroll view, every page view and its
            // bitmap layer (measured: every visible page re-blitted twice per
            // keystroke, 374 blits of an unchanged page over 187 revisions).
            if let shown = Self.shownFrame(model.displayListV2) {
                pages(shown.frame, stale: shown.stale)
                diagnostics(shown.frame)
            } else {
                switch model.displayListV2 {
                case .loaded, .loading(_, _, .some, _, _):
                    EmptyView() // shown above
                case .loading(_, _, nil, _, _):
                    ContentUnavailableView("Loading display list…", systemImage: "hourglass")
                case .failed(let error, let source):
                ContentUnavailableView {
                    Label("Display list refused — nothing rendered", systemImage: "xmark.octagon")
                } description: {
                    Text("\(source.label)\n[\(error.code)] \(error.message)")
                        .textSelection(.enabled)
                } actions: {
                    refusalActions(source: error.source, v1Revision: model.result?.revision)
                }
                .accessibilityIdentifier("v2-refusal")
                case nil:
                    if let refusal = model.displayCandidates.lastInvalidCandidate {
                        // The helper's candidate for the applied v1 preview was refused by the
                        // validator: name it, point at the source span it named, and offer the
                        // v1 preview of that revision (on screen in the product pane) as the fallback.
                        ContentUnavailableView {
                            Label("Display candidate refused — nothing rendered", systemImage: "xmark.octagon")
                        } description: {
                            Text("\(refusal.requestID) for revision \(refusal.editorRevision)\n\(refusal.why)")
                                .textSelection(.enabled)
                        } actions: {
                            refusalActions(source: refusal.source, v1Revision: refusal.editorRevision)
                        }
                        .accessibilityIdentifier("v2-candidate-refusal")
                    } else {
                        ContentUnavailableView("No v2 display list yet", systemImage: "doc.richtext",
                                               description: Text(model.workerAttached
                                                                 ? "Requesting display-list-v2 from the attached worker (\(model.liveV2Accepted ? "accepted" : "not accepted yet")); or use File > Open Display List (v2)…"
                                                                 : "Attach a producer that accepts display-list-v2, or use File > Open Display List (v2)… with a flashtex-render --v2 JSON file."))
                    }
                }
            }
        }
        .onAppear {
            // While visible, the pane asks the producer for the live route.
            model.setLiveV2(true)
            // Automation hook: FLASHTEX_V2_FILE seeds the pane at launch.
            if model.displayListV2 == nil, let path = ProcessInfo.processInfo.environment["FLASHTEX_V2_FILE"] {
                model.loadDisplayListV2(url: URL(fileURLWithPath: path))
            }
        }
        .onDisappear { model.setLiveV2(false) }
    }

    /// The frame the pane shows and whether it is stale: the loaded frame, or
    /// the previous frame retained while a load is in flight. Nil when there
    /// is nothing to show (first load, a refusal, no list yet).
    static func shownFrame(_ state: V2PreviewState?) -> (frame: V2Frame, stale: Bool)? {
        switch state {
        case .loaded(let frame, _): (frame, false)
        case .loading(_, _, let previous?, _, _): (previous, true)
        case .loading(_, _, nil, _, _), .failed, nil: nil
        }
    }

    /// Under a refusal: the source span the validator named (navigable) and
    /// the v1 preview of the same revision, which stays the product preview.
    @ViewBuilder
    private func refusalActions(source: RuntimeV1.SourceRange?, v1Revision: Int?) -> some View {
        VStack(spacing: 6) {
            if let source {
                Button("Go to source (\(source.path) bytes \(source.startByte)..<\(source.endByte))") {
                    Task { @MainActor in await model.navigateOpeningIfNeeded(to: source) }
                }
                .accessibilityIdentifier("v2-refusal-go-to-source")
            }
            if let v1Revision {
                Text("The v1 preview of revision \(v1Revision) remains the product preview (fallback frame); its items navigate exactly.")
                    .font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center)
                    .accessibilityIdentifier("v2-refusal-v1-fallback")
            }
        }
    }

    private func pages(_ frame: V2Frame, stale: Bool) -> some View {
        PreviewV2View(frame: frame, dark: model.darkPreview, stale: stale, caretPath: model.activePath, caretByte: model.caretByte,
                      zoom: model.previewZoom, onFitScale: { model.previewFitScale = $0 },
                      // "the pdf moves to where the changes are happening" (CaretFollow.swift)
                      follow: model.caretFollow.request, reveal: model.previewReveal,
                      onUserScroll: { model.caretFollow.userDidScrollPreview() },
                      navigation: DisplayListLinks.effective(frame.list.navigation, accepted: model.acceptedLayoutCapabilities,
                                                            live: model.displayListV2?.source.isLive == true),
                      onLink: { model.activatePreviewLink($0, in: frame.list) }) { hit in
            model.navigateV2(hit)
        }
    }

    @ViewBuilder
    private func diagnostics(_ frame: V2Frame) -> some View {
        // Equatable on the diagnostics array: a new frame with the same
        // diagnostics (every keystroke of a document with 130 recovered errors)
        // does not rebuild 130 rows; the list is lazy and bounded in height so
        // the pages keep their room.
        if model.previewDebugStatus {
            V2DiagnosticsList(diagnostics: frame.list.diagnostics) { model.navigate(to: $0) }.equatable()
        }
    }

    private var header: some View { V2PaneHeader() }
}

/// The display list's diagnostics under the pages. Re-evaluated only when the
/// diagnostics differ (`Equatable`); rows are lazy and the list scrolls within
/// a bounded height (measured on HW1.tex, 130 recovered errors: the eager
/// 130-row VStack rebuilt on every frame was the dominant paint cost).
struct V2DiagnosticsList: View, Equatable {
    let diagnostics: [RenderingV2.Diagnostic]
    let onNavigate: (RenderingV2.SourceRange) -> Void

    static func == (a: V2DiagnosticsList, b: V2DiagnosticsList) -> Bool { a.diagnostics == b.diagnostics }

    var body: some View {
        if !diagnostics.isEmpty {
            Divider()
            VStack(alignment: .leading, spacing: 2) {
                Text("Display list diagnostics (\(diagnostics.count))").font(.caption.bold())
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(diagnostics.enumerated()), id: \.offset) { _, d in
                            HStack(alignment: .top) {
                                Image(systemName: d.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                                    .foregroundStyle(d.severity == .error ? .red : .orange)
                                Text("[\(d.code)] \(d.message)").font(.caption)
                                if let s = d.sources.first { Button("Go to source") { onNavigate(s) }.controlSize(.mini) }
                            }
                        }
                    }
                }
                .frame(maxHeight: 160)
            }
            .padding(8).frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityIdentifier("v2-diagnostics")
        }
    }
}

/// The pane header is its own view: it reads the applied result, worker and
/// negotiation state, so those changes re-evaluate the header, not the pages.
private struct V2PaneHeader: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 8) {
                Text("V2").font(.caption.bold()).padding(.horizontal, 6).padding(.vertical, 2).background(Color.purple.opacity(0.25), in: Capsule())
                if model.previewDebugStatus {
                Text("display-list-v2").font(.caption).foregroundStyle(.secondary).lineLimit(1)
                if model.workerAttached {
                    Text(model.liveV2Accepted ? "LIVE" : "v1 only")
                        .font(.caption.bold()).padding(.horizontal, 6).padding(.vertical, 2)
                        .background((model.liveV2Accepted ? Color.green : Color.gray).opacity(0.25), in: Capsule())
                        .help(model.liveV2Accepted ? "The applied compile_result accepted display-list-v2; frames arrive with each compile."
                                                   : "The applied compile_result did not accept display-list-v2 (producer without the capability, or declined for this request).")
                        .accessibilityIdentifier("v2-live")
                }
                if let frame = model.displayListV2?.frame, model.displayListV2?.source.isLive == true,
                   let applied = model.result?.revision, applied != frame.list.revision {
                    Text("frame revision \(frame.list.revision) — applied result is revision \(applied) (no v2 frame for it)")
                        .font(.caption.bold()).foregroundStyle(.orange).lineLimit(1)
                        .accessibilityIdentifier("v2-behind")
                }
                }
                if case .loading(let source, _, let previous, _, _) = model.displayListV2 {
                    // Quiet progress indicator: the previous frame stays on screen; no flashing text.
                    ProgressView().controlSize(.mini)
                        .help(previous == nil ? "loading \(source.label)…" : "showing the previous frame while \(source.label) is verified")
                        .accessibilityLabel(previous == nil ? "loading \(source.label)" : "verifying \(source.label); previous frame shown")
                        .accessibilityIdentifier("v2-stale")
                }
                Spacer()
                Button("Open…") { model.openDisplayListV2Panel() }.controlSize(.small).fixedSize()
                Button("Export PDF (v2)…") { model.exportPDFV2() }.controlSize(.small).fixedSize()
                    .disabled({ if case .loaded = model.displayListV2 { false } else { true } }())
            }
            if let notices = model.displayListV2?.frame?.imageNotices, !notices.isEmpty {
                // display-list-v2-images: refused image bytes (stale hash, symlink,
                // unreadable). Non-modal; the frame stays, the item painted nothing.
                Text(notices.joined(separator: " · "))
                    .font(.caption).foregroundStyle(.orange).lineLimit(1).truncationMode(.middle)
                    .help(notices.joined(separator: "\n"))
                    .accessibilityIdentifier("v2-image-notice")
            }
            if model.previewDebugStatus, let frame = model.displayListV2?.frame {
                let fonts = frame.fonts.values.map { "\($0.resource.postscriptName) \($0.resource.sha256.prefix(8))" }.sorted().joined(separator: ", ")
                Text("\(model.displayListV2?.source.label ?? "") · id \(frame.id) · project \(frame.list.projectId) · revision \(frame.list.revision) · \(frame.list.pages.count) page(s) · fonts by hash: \(fonts)")
                    .font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
                    .help(frame.fonts.values.map { "\($0.resource.postscriptName): \($0.resource.sha256) → \($0.file.url.lastPathComponent)" }.sorted().joined(separator: "\n"))
            }
        }
        .padding(.horizontal, 8).padding(.vertical, 4)
        .background(.bar)
    }
}

/// Scrollable pages of a prepared frame, fit to the pane width.
struct PreviewV2View: View {
    let frame: V2Frame
    let dark: Bool
    var stale = false
    let caretPath: String
    let caretByte: Int?
    /// Zoom multiplier over the fit-to-width scale (PreviewZoom.swift).
    var zoom: CGFloat = 1
    var onFitScale: ((CGFloat) -> Void)? = nil
    /// Latest caret-follow request (CaretFollow.swift); acted on once per token.
    var follow: CaretFollowController.Request? = nil
    /// Internal-destination scroll (DisplayListLinks.swift); acted on once per token.
    var reveal: CaretFollowController.Request? = nil
    /// Reported when the reader scrolls this pane by hand.
    var onUserScroll: (() -> Void)? = nil
    /// Active `navigation` after capability gating (nil → no link behaviour).
    var navigation: RenderingV2.Navigation? = nil
    var onLink: ((RenderingV2.Navigation.Link) -> Void)? = nil
    let onSelect: (V2Geometry.Hit) -> Void
    @Environment(\.displayScale) private var displayScale

    var body: some View {
        GeometryReader { geo in
            let widest = frame.list.pages.map(\.widthPt).max() ?? 612
            let fit = min(1, max(0.2, (geo.size.width - 48) / widest))
            let scale = PreviewZoom.scale(fit: fit, zoom: zoom)
            // Scroll anchoring (PreviewAnchor.swift): the (page, fraction) under the
            // viewport's top edge survives a frame with another page count and a
            // pane resize; a frame with the same page geometry never moves the scroll.
            let layout = PreviewPageLayout(pages: frame.prepared.map { PreviewPageLayout.Page(number: $0.number, widthPt: $0.widthPt, heightPt: $0.heightPt) }, scale: scale)
            // Paint instrumentation (TypingBench.swift): the pages whose content
            // changed since the last rendered frame are the ones expected to blit
            // for this revision; a frame that changed no page paints nothing new.
            let expectedDraws = V2RenderTracker.shared.changedPages(frame: frame)
            let _ = expectedDraws == 0 ? TypingBench.shared.willRender(revision: frame.list.revision, pages: 0) : ()
            let _ = TypingBench.isBenchActive ? FlashTeXLog.write("preview-v2: pass revision \(frame.list.revision) \(stale ? "stale" : "loaded") changed \(expectedDraws) at \(MonotonicClock.nowNs())") : ()
            ScrollView([.vertical, .horizontal]) {
                LazyVStack(spacing: 24) {
                    ForEach(Array(frame.prepared.enumerated()), id: \.element.number) { index, prepared in
                        if let page = frame.page(number: prepared.number) {
                            PageV2View(page: page, prepared: prepared, pageToken: frame.pageToken(at: index), frameRevision: frame.list.revision, expectedDraws: expectedDraws,
                                       dark: dark, stale: stale, scale: scale, displayScale: displayScale,
                                       // Only pages whose cluster sources can contain the caret walk their clusters.
                                       caretHighlights: caretByte.flatMap { prepared.mayContain(byte: $0, path: caretPath) ? V2Geometry.caretHighlights(containing: $0, path: caretPath, in: page) : nil } ?? [],
                                       navigation: navigation, onLink: onLink, onSelect: onSelect)
                                .equatable()
                                .id(page.number)
                        }
                    }
                }
                .padding(24)
                .background(PreviewAnchorKeeper(layout: layout, follow: follow, reveal: reveal, onUserScroll: onUserScroll))
            }
            .onChange(of: fit, initial: true) { _, f in onFitScale?(f) }
        }
        .background(dark ? Color(white: 0.12) : Color(nsColor: .windowBackgroundColor))
        .onAppear { V2PageRasterizer.shared.setCurrent(frame: frame) }
        .onChange(of: frame.pageTokens) { _, _ in V2PageRasterizer.shared.setCurrent(frame: frame) }
    }
}

/// Which pages of the frame being rendered differ (by content token) from the
/// frame rendered before it. Main thread (SwiftUI render pass) only.
@MainActor
final class V2RenderTracker {
    static let shared = V2RenderTracker()
    private var lastTokens: [Int: String] = [:]
    private var lastRevision: Int?

    /// Number of pages whose token is new for this frame's revision. Repeated
    /// evaluations for the same revision return the count of the first.
    private var lastCount = 0
    func changedPages(frame: V2Frame) -> Int {
        if lastRevision == frame.list.revision, lastTokens.count == frame.prepared.count { return lastCount }
        var tokens: [Int: String] = [:]
        var changed = 0
        for (index, page) in frame.prepared.enumerated() {
            let token = frame.pageToken(at: index)
            tokens[page.number] = token
            if lastTokens[page.number] != token { changed += 1 }
        }
        lastTokens = tokens
        lastRevision = frame.list.revision
        lastCount = changed
        return changed
    }

    func reset() { lastTokens = [:]; lastRevision = nil; lastCount = 0 }
}

/// Identity of a prepared frame instance: the envelope id, revision and the
/// per-preparation nonce (header/evidence; bitmaps key on page tokens).
enum V2FrameIdentity {
    static func token(_ frame: V2Frame) -> String { "\(frame.id)#r\(frame.list.revision)#\(frame.preparedNonce)" }
}

/// One page: bitmap lookup, hover/tap geometry and the stale/label overlays.
/// Equatable on everything that changes what is drawn, so a pane
/// re-evaluation for another page's bitmap, a caret move elsewhere or a
/// header change skips this page; the blit itself lives in `PageV2Canvas`,
/// which redraws only when its bitmap, caret or hover changes (a stale
/// toggle or a new frame with the same page bytes never re-blits).
private struct PageV2View: View, Equatable {
    let page: RenderingV2.Page
    let prepared: V2PreparedPage
    let pageToken: String
    /// Display-list revision and expected page draws, for the typing bench's
    /// paint point (not part of the equality: they do not change the pixels).
    var frameRevision = 0
    var expectedDraws = 1
    let dark: Bool
    let stale: Bool
    let scale: CGFloat
    let displayScale: CGFloat
    /// Exact caret / whole cluster for text, the enclosing formula box for a
    /// caret inside math (MathCaretHighlight.swift).
    let caretHighlights: [V2Geometry.CaretHighlight]
    var navigation: RenderingV2.Navigation? = nil
    var onLink: ((RenderingV2.Navigation.Link) -> Void)? = nil
    let onSelect: (V2Geometry.Hit) -> Void
    @State private var hover: V2Geometry.Hit?
    @State private var linkHover: RenderingV2.Navigation.Link?
    @State private var linkCursorPushed = false

    // `stale` is not part of the equality: nothing drawn depends on it, and the
    // loaded -> stale -> loaded toggle of every keystroke re-evaluated every page.
    static func == (a: PageV2View, b: PageV2View) -> Bool {
        a.pageToken == b.pageToken && a.page.number == b.page.number
            && a.dark == b.dark && a.scale == b.scale && a.displayScale == b.displayScale && a.caretHighlights == b.caretHighlights
            && a.navigation == b.navigation
    }

    var body: some View {
        let size = CGSize(width: page.widthPt * scale, height: page.heightPt * scale)
        // Reading the slot through `image(for:)` subscribes this page to its bitmap's arrival.
        let bitmap = V2PageRasterizer.shared.image(for: prepared, pageToken: pageToken, pixelsPerPoint: Double(scale * displayScale), dark: dark)
        // A stale page keeps its label and colour: the previous frame stays on screen
        // unchanged while the next one is verified (typing must not flash the pages).
        let label = bitmap == nil ? "page \(page.number) · v2 · rasterizing…" : "page \(page.number) · v2"
        let labelColor: Color = dark ? Color(white: 0.7) : Color(white: 0.35)
        let pageBackground: Color = dark ? Color(white: 0.16) : .white
        // The bitmap is the contents of a CALayer (PageBitmapLayer): CoreAnimation
        // composites it on every later pass without any drawing on the main thread;
        // a new bitmap is one `layer.contents` assignment. Caret/hover marks are a
        // separate small overlay that exists only while there is something to mark.
        let canvas = PageBitmapLayer(bitmap: bitmap, pageToken: pageToken, pageNumber: page.number, frameRevision: frameRevision,
                                     expectedDraws: expectedDraws, background: dark ? CGColor(gray: 0.16, alpha: 1) : CGColor(gray: 1, alpha: 1))
            .frame(width: size.width, height: size.height)
            .background(Rectangle().fill(pageBackground).shadow(radius: 4))
            .overlay {
                if !caretHighlights.isEmpty || hover != nil {
                    PageV2Marks(scale: scale, caretHighlights: caretHighlights, hover: hover).equatable().allowsHitTesting(false)
                }
            }
        canvas
            .contentShape(Rectangle())
            .onContinuousHover { phase in
                switch phase {
                case .active(let p):
                    let pageX = p.x / scale, pageY = p.y / scale
                    if let nav = navigation, let link = DisplayListLinks.hit(nav, page: page.number, viewX: pageX, viewY: pageY, scale: 1) {
                        linkHover = link
                        hover = nil
                        if !linkCursorPushed { NSCursor.pointingHand.push(); linkCursorPushed = true }
                    } else {
                        linkHover = nil
                        hover = V2Geometry.hit(page: page, atPointX: pageX, y: pageY)
                        if linkCursorPushed { NSCursor.pop(); linkCursorPushed = false }
                    }
                case .ended:
                    hover = nil
                    linkHover = nil
                    if linkCursorPushed { NSCursor.pop(); linkCursorPushed = false }
                }
            }
            .onTapGesture { location in
                let pageX = location.x / scale, pageY = location.y / scale
                if let nav = navigation, let link = DisplayListLinks.hit(nav, page: page.number, viewX: pageX, viewY: pageY, scale: 1) {
                    onLink?(link)
                    return
                }
                if let hit = V2Geometry.hit(page: page, atPointX: pageX, y: pageY) { onSelect(hit) }
            }
            .overlay(alignment: .bottomTrailing) {
                // Colored for the PAGE background (white or dark), not the window appearance.
                Text(label).font(.caption2).foregroundStyle(labelColor).padding(4)
            }
            .help(helpText)
    }

    private var helpText: String {
        if let link = linkHover { return DisplayListLinks.tooltip(for: link) }
        guard let h = hover else { return "" }
        let what = h.text.map { "“\($0)” → " } ?? "rule → "
        let target = h.syntheticReason.map { "generated: \($0)" } ?? h.sources.map { "\($0.path) \($0.startByte)..<\($0.endByte)" }.joined(separator: ", ")
        return what + target
    }
}

/// One page bitmap as CALayer contents. `updateNSView` runs in the SwiftUI
/// render pass; the layer is touched only when the bitmap object changes
/// (page content, scale or appearance), and that assignment is the v2 paint
/// point for the typing bench (the CoreAnimation commit that follows the
/// pass uploads the new contents; `finishPaint` runs on the next turn).
private struct PageBitmapLayer: NSViewRepresentable {
    let bitmap: CGImage?
    let pageToken: String
    let pageNumber: Int
    var frameRevision = 0
    var expectedDraws = 1
    let background: CGColor

    func makeNSView(context: Context) -> PageBitmapView { PageBitmapView() }

    func updateNSView(_ view: PageBitmapView, context: Context) {
        view.show(bitmap, pageToken: pageToken, pageNumber: pageNumber, frameRevision: frameRevision, expectedDraws: expectedDraws, background: background)
    }
}

/// The AppKit view behind `PageBitmapLayer` (test-visible: `shown`, `show`).
final class PageBitmapView: NSView {
    private(set) var shown: CGImage?
    /// How many times a new bitmap was installed (tests).
    private(set) var installs = 0

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        // The whole bitmap fills the layer: at the pane's pixels-per-point the
        // bitmap's pixel size equals the layer's backing size (1:1, no resampling).
        layer?.contentsGravity = .resize
        layer?.magnificationFilter = .nearest
        layer?.minificationFilter = .nearest
        layer?.masksToBounds = true
    }
    required init?(coder: NSCoder) { nil }
    override var isOpaque: Bool { true }
    /// Mouse events belong to the SwiftUI page view around this layer
    /// (hover geometry, tap navigation); the bitmap never takes them.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    /// Installs `bitmap` as the layer contents when it is not the one shown.
    /// Returns whether the contents changed.
    @MainActor
    @discardableResult
    func show(_ bitmap: CGImage?, pageToken: String, pageNumber: Int, frameRevision: Int, expectedDraws: Int, background: CGColor) -> Bool {
        layer?.backgroundColor = background
        guard bitmap !== shown else { return false }
        shown = bitmap
        if bitmap != nil {
            installs += 1
            // Paint instrumentation (TypingBench.swift): the pass that installs a page
            // bitmap of the frame's revision; pages still rasterizing do not count.
            TypingBench.shared.willRender(revision: frameRevision, pages: expectedDraws)
            TypingBench.shared.didDraw(page: pageNumber)
            if TypingBench.isBenchActive { FlashTeXLog.write("preview-v2: blit page \(pageNumber) \(pageToken.prefix(24)) at \(MonotonicClock.nowNs())") }
        }
        layer?.contents = bitmap
        return true
    }
}

/// Caret and hover marks over a page, drawn only when there is something to mark.
private struct PageV2Marks: View, Equatable {
    let scale: CGFloat
    let caretHighlights: [V2Geometry.CaretHighlight]
    let hover: V2Geometry.Hit?

    private func viewRect(_ r: RenderingV2.Rect) -> CGRect {
        CGRect(x: RenderingV2.points(r.x) * scale, y: RenderingV2.points(r.top) * scale,
               width: RenderingV2.points(r.width) * scale, height: RenderingV2.points(r.height) * scale)
    }

    var body: some View {
        Canvas(rendersAsynchronously: false) { context, _ in
            // Caret highlight: exact caret bar when the compiler supplied one for
            // that byte, else the whole cluster's hit rectangles (documented fallback);
            // a caret inside a formula gets the enclosing formula box (every glyph
            // cluster and rule carrying the formula's span, MathCaretHighlight.swift).
            for h in caretHighlights {
                switch h {
                case .cluster(let m):
                    if let k = m.caret {
                        let bar = CGRect(x: RenderingV2.points(k.x) * scale - 0.75, y: RenderingV2.points(k.top) * scale, width: 1.5, height: RenderingV2.points(k.height) * scale)
                        context.fill(Path(bar), with: .color(Color.accentColor))
                        for r in m.hitRects { context.fill(Path(viewRect(r)), with: .color(Color.accentColor.opacity(0.10))) }
                    } else {
                        for r in m.hitRects { context.fill(Path(viewRect(r)), with: .color(Color.accentColor.opacity(0.22))) }
                    }
                case .formula(let box):
                    let outline = viewRect(box.bounds).insetBy(dx: -2, dy: -2)
                    context.fill(Path(roundedRect: outline, cornerRadius: 2), with: .color(Color.accentColor.opacity(0.10)))
                    context.stroke(Path(roundedRect: outline, cornerRadius: 2), with: .color(Color.accentColor.opacity(0.8)), lineWidth: 1)
                    for r in box.rects { context.fill(Path(viewRect(r)), with: .color(Color.accentColor.opacity(0.12))) }
                }
            }
            if let hover { context.fill(Path(viewRect(hover.rect).insetBy(dx: -1, dy: -1)), with: .color(Color.accentColor.opacity(0.25))) }
        }
    }
}
