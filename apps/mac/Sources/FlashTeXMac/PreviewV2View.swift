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
    /// The live line is a `display_list_delta` (display-list-v2-delta): the
    /// frame was reconstructed from it and the installed base, so the line is
    /// not a display list on its own and cannot be handed to a list-file tool
    /// (`flashtex-pdf-exact` refuses it: `type "display_list_delta" is not
    /// display_list`). Every edit after the first full frame arrives this way.
    var isDeltaLine: Bool {
        guard case .worker(_, _, _, let line) = self else { return false }
        return RenderingV2Fast.header(line)?.type == DisplayListDelta.messageType
    }

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
    var isFailed: Bool { if case .failed = self { true } else { false } }
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
            // display-list-v2-window: an elided page has no items and paints a
            // placeholder, never a bitmap (a 1000-page window would otherwise
            // rasterize 1000 blanks).
            if index < frame.list.pages.count, !frame.list.pages[index].resident { continue }
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
            // PreviewAnnouncements.swift: a verified frame makes refusals news again; a live
            // one also lists every page of the document (a windowed frame too), which the
            // v2-only reply's elided `pages` did not. (The `.loaded` a live refusal restores
            // below is the kept frame, not a verification, so this is not in the didSet.)
            if source.isLive { previewAnnouncer.noteFrame(revision: frame.list.revision, pageCount: frame.list.pages.count) }
            else { previewAnnouncer.noteFrameVerified() }
            // Installation (proposal r5 §6.1): only a published live frame is a base.
            if source.isLive { deltaInstalled = frame.installedBase } else { deltaInstalled = nil }
            if TypingBench.isBenchActive { FlashTeXLog.write("preview-v2: published \(source.label) revision \(frame.list.revision) at \(MonotonicClock.nowNs())") }
            let windowNote = frame.list.window.map { " — window: pages \($0.firstPage)–\($0.firstPage + $0.pageCount - 1) resident, \($0.documentPageCount - $0.pageCount) elided" } ?? ""
            captureNote = "\(source.isLive ? "Live display list" : "Loaded display list") \(source.label): \(frame.list.pages.count) page(s)\(windowNote), \(frame.fonts.count) font(s) resolved by content hash, \(frame.prepared.reduce(0) { $0 + $1.glyphCount }) glyphs prepared"
            V2ParityEvidence.runIfRequested(frame: frame, source: source)
        case .failed(let error):
            if source.isLive { deltaInstalled = nil } // full resync on the next request
            if source.isLive, let retained = displayListV2?.retained {
                // A refused LIVE sibling (D1 source binding, correlation, validation) never
                // un-verifies the frame already on screen: it stays, and is stale by the
                // header's revision label (`v2-behind`) whenever the applied result moved on.
                displayListV2 = .loaded(retained.frame, retained.source)
                V2Live.note(liveRefusal: error)
                previewAnnouncer.noteLiveRefusal(error) // quiet, deduped: the pages on screen did not change
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

    /// The v2 item under the editor caret, shaped as the same `Hit` a click on
    /// the page produces, so ⌘⇧J and clicking the preview go through one
    /// navigation path (digest attestation, rebase onto an edited buffer, the
    /// multi-span note). Nil when the caret maps to nothing the producer laid
    /// out — a comment, the preamble, a page outside a served window.
    func caretV2Hit(frame: V2Frame, byte: Int) -> (page: Int, hit: V2Geometry.Hit)? {
        for page in frame.list.pages where page.resident {
            for highlight in V2Geometry.caretHighlights(containing: byte, path: activePath, in: page) {
                switch highlight {
                case .cluster(let match):
                    guard case .glyphRun(let run) = page.items[match.itemIndex] else { continue }
                    let cluster = run.clusters[match.clusterIndex]
                    guard let rect = match.hitRects.first else { continue }
                    return (page.number, V2Geometry.Hit(itemIndex: match.itemIndex, clusterIndex: match.clusterIndex,
                                                        text: run.clusterText(match.clusterIndex),
                                                        sources: cluster.sources ?? [],
                                                        syntheticReason: cluster.syntheticReason, rect: rect))
                case .formula(let box):
                    // A formula's span is the whole `$…$` including delimiters
                    // and no single cluster's text equals it, so there is no
                    // expected text to verify — the span itself is the answer.
                    guard let item = box.itemIndices.first else { continue }
                    return (page.number, V2Geometry.Hit(itemIndex: item, clusterIndex: nil, text: nil,
                                                        sources: [box.source], syntheticReason: nil, rect: box.bounds))
                case .paragraph:
                    continue // never requested here; the band is not a navigation target
                }
            }
        }
        return nil
    }

    /// ⌘⇧J against the v2 pane. Returns false when there is no v2 frame, so
    /// the caller can fall through to the runtime-v1 pane.
    @discardableResult
    func revealCaretInV2Preview(byte: Int) -> Bool {
        guard previewV2, let frame = displayListV2?.frame else { return false }
        guard let found = caretV2Hit(frame: frame, byte: byte) else {
            let behind = result.map { $0.revision != frame.list.revision } ?? false
            navigationNote = "Caret byte \(byte) is inside no preview item"
                + (behind ? " (the display list is from an older revision)." : ".")
            return true
        }
        navigateV2Now(found.hit)
        if navigationNote?.hasPrefix("Selected") == true {
            navigationNote! += " — page \(found.page)"
        }
        return true
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
        if let window = frame.list.window {
            // A windowed frame is an incomplete view (window proposal §4.1):
            // exporting/comparing it would write blank pages as evidence.
            FlashTeXLog.write("preview-v2: parity skipped for \(source.label) — windowed frame (\(window.pageCount) of \(window.documentPageCount) pages resident)")
            return
        }
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
struct PreviewV2Pane: View, Equatable {
    @Environment(ShellModel.self) var model
    /// The pane takes no input from its parent: everything it shows comes
    /// from the model (observed by its own body) and its state. Equal
    /// always, so a re-render of the preview container (the page readout
    /// and HUD change as the reader scrolls onto another page) does not
    /// re-evaluate the pane and every page view; measured as a dropped
    /// frame at each page change on the 120 Hz bench (P0-PREVIEW-TILES).
    nonisolated static func == (a: PreviewV2Pane, b: PreviewV2Pane) -> Bool { true }
    /// The caret's paragraph the band follows (CaretParagraphMemo): kept
    /// across the stale frame of every keystroke so the band never flashes.
    @State private var caretParagraph = CaretParagraphMemo()

    var body: some View {
        VStack(spacing: 0) {
            // Debug-only strip (View > Show Preview Debug Status): identity,
            // LIVE/behind badges, font manifest. At rest the pane starts at
            // the pages — Open… and Export moved to the title bar
            // (TitleBar.swift), the V2/loading state to the floating HUD
            // (#653: no header rows over the preview).
            if model.previewDebugStatus {
                header
                Divider()
            }
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
        .overlay(alignment: .bottomLeading) {
            // display-list-v2-images: refused image bytes (stale hash, symlink,
            // unreadable). Non-modal; the frame stays, the item painted nothing.
            // Floats quietly over the ground now that the header row is gone.
            if let notices = model.displayListV2?.frame?.imageNotices, !notices.isEmpty {
                Text(notices.joined(separator: " · "))
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning).lineLimit(1).truncationMode(.middle)
                    .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
                    .background(DS.Colors.surfaceRaised, in: RoundedRectangle(cornerRadius: DS.Radius.control))
                    .overlay(RoundedRectangle(cornerRadius: DS.Radius.control).strokeBorder(DS.Colors.componentBorder, lineWidth: DS.Size.hairline))
                    .padding(DS.Space.l)
                    .help(notices.joined(separator: "\n"))
                    .accessibilityIdentifier("v2-image-notice")
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
        VStack(spacing: DS.Space.s) {
            if let source {
                Button("Go to source (\(source.path) bytes \(source.startByte)..<\(source.endByte))") {
                    Task { @MainActor in await model.navigateOpeningIfNeeded(to: source) }
                }
                .accessibilityIdentifier("v2-refusal-go-to-source")
            }
            if let v1Revision {
                Text("The v1 preview of revision \(v1Revision) remains the product preview (fallback frame); its items navigate exactly.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).multilineTextAlignment(.center)
                    .accessibilityIdentifier("v2-refusal-v1-fallback")
            }
        }
    }

    private func pages(_ frame: V2Frame, stale: Bool) -> some View {
        PreviewV2View(frame: frame, dark: model.darkPreview, stale: stale, caretPath: model.activePath, caretByte: model.caretByte,
                      // The paragraph band: only while the preview follows the caret, and
                      // recomputed only against a verified frame (the previous band stays on
                      // the stale frame, exactly as the page label does).
                      caretParagraph: CaretFollow.isEnabled
                          ? caretParagraph.range(stale: stale, byte: model.caretByte, path: model.activePath, text: model.activeText)
                          : nil,
                      zoom: model.previewZoom, onFitScale: { model.previewFitScale = $0 },
                      // "the pdf moves to where the changes are happening" (CaretFollow.swift)
                      follow: model.caretFollow.request, reveal: model.previewReveal,
                      onUserScroll: { model.caretFollow.userDidScrollPreview() },
                      onVisiblePage: { model.v2WindowSawVisiblePage($0) },
                      navigation: DisplayListLinks.effective(frame.list.navigation, accepted: model.acceptedLayoutCapabilities,
                                                            live: model.displayListV2?.source.isLive == true),
                      onLink: { model.activatePreviewLink($0, in: frame.list) },
                      onPageJump: { model.previewAnnouncer.notePageJump(page: $0, of: model.toolbarPageCount) }) { hit in
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
            VStack(alignment: .leading, spacing: DS.Space.xxs) {
                Text("Display list diagnostics (\(diagnostics.count))").font(DS.Fonts.header)
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: DS.Space.xxs) {
                        ForEach(Array(diagnostics.enumerated()), id: \.offset) { _, d in
                            HStack(alignment: .top) {
                                Image(systemName: d.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                                    .foregroundStyle(d.severity == .error ? .red : .orange)
                                Text("[\(d.code)] \(d.message)").font(DS.Fonts.secondary)
                                if let s = d.sources.first { Button("Go to source") { onNavigate(s) }.controlSize(.mini) }
                            }
                        }
                    }
                }
                .frame(maxHeight: DS.Layout.v2DiagnosticsMaxHeight)
            }
            .padding(DS.Space.m).frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityIdentifier("v2-diagnostics")
        }
    }
}

/// The pane header is its own view: it reads the applied result, worker and
/// negotiation state, so those changes re-evaluate the header, not the pages.
private struct V2PaneHeader: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.xxs) {
            HStack(spacing: DS.Space.m) {
                Text("V2").font(DS.Fonts.header).padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs).background(DS.Colors.statusHistorical.opacity(DS.State.badgeFillOpacity), in: Capsule())
                Text("display-list-v2").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1)
                if model.workerAttached {
                    Text(model.liveV2Accepted ? "LIVE" : "v1 only")
                        .font(DS.Fonts.header).padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
                        .background((model.liveV2Accepted ? DS.Colors.severitySuccess : DS.Colors.textTertiary).opacity(DS.State.badgeFillOpacity), in: Capsule())
                        .help(model.liveV2Accepted ? "The applied compile_result accepted display-list-v2; frames arrive with each compile."
                                                   : "The applied compile_result did not accept display-list-v2 (producer without the capability, or declined for this request).")
                        .accessibilityIdentifier("v2-live")
                }
                if let frame = model.displayListV2?.frame, model.displayListV2?.source.isLive == true,
                   let applied = model.result?.revision, applied != frame.list.revision {
                    Text("frame revision \(frame.list.revision) — applied result is revision \(applied) (no v2 frame for it)")
                        .font(DS.Fonts.header).foregroundStyle(DS.Colors.severityWarning).lineLimit(1)
                        .accessibilityIdentifier("v2-behind")
                }
                if case .loading(let source, _, let previous, _, _) = model.displayListV2 {
                    // Quiet progress indicator: the previous frame stays on screen; no flashing text.
                    ProgressView().controlSize(.mini)
                        .help(previous == nil ? "loading \(source.label)…" : "showing the previous frame while \(source.label) is verified")
                        .accessibilityLabel(previous == nil ? "loading \(source.label)" : "verifying \(source.label); previous frame shown")
                        .accessibilityIdentifier("v2-stale")
                }
                Spacer()
            }
            if model.previewDebugStatus, let frame = model.displayListV2?.frame {
                let fonts = frame.fonts.values.map { "\($0.resource.postscriptName) \($0.resource.sha256.prefix(8))" }.sorted().joined(separator: ", ")
                let windowNote = frame.list.window.map { " · window \($0.firstPage)–\($0.firstPage + $0.pageCount - 1) of \($0.documentPageCount)" } ?? ""
                Text("\(model.displayListV2?.source.label ?? "") · id \(frame.id) · project \(frame.list.projectId) · revision \(frame.list.revision) · \(frame.list.pages.count) page(s)\(windowNote) · fonts by hash: \(fonts)")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1).truncationMode(.middle)
                    .help(frame.fonts.values.map { "\($0.resource.postscriptName): \($0.resource.sha256) → \($0.file.url.lastPathComponent)" }.sorted().joined(separator: "\n"))
            }
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
        .background(DS.Colors.surfaceSecondary)
    }
}

/// The caret's paragraph bytes for the band, memoised across the stale
/// frame a keystroke shows: while the next frame is verified the previous
/// band stays exactly where it was (an edited buffer's bytes no longer match
/// the stale frame's spans, so recomputing there would flash or drop it).
/// A reference held as view state: mutating it inside `body` is not a state
/// change, and nothing observes it.
final class CaretParagraphMemo {
    private var last: (path: String, range: Range<Int>)?

    /// The paragraph to band on `path`, or nil when there is no caret.
    func range(stale: Bool, byte: Int?, path: String, text: String) -> Range<Int>? {
        if stale, let last, last.path == path { return last.range }
        guard let byte else { return last?.path == path ? last?.range : nil }
        let range = V2Geometry.paragraphBounds(containing: byte, in: text)
        last = (path, range)
        return range
    }
}

/// Scrollable pages of a prepared frame, fit to the pane width.
struct PreviewV2View: View {
    let frame: V2Frame
    let dark: Bool
    var stale = false
    let caretPath: String
    let caretByte: Int?
    /// The caret's paragraph bytes to band (MathCaretHighlight.swift); nil
    /// when the preview does not follow the caret — then no page walks its
    /// items for it.
    var caretParagraph: Range<Int>? = nil
    /// Zoom multiplier over the fit-to-width scale (PreviewZoom.swift).
    var zoom: CGFloat = 1
    var onFitScale: ((CGFloat) -> Void)? = nil
    /// Latest caret-follow request (CaretFollow.swift); acted on once per token.
    var follow: CaretFollowController.Request? = nil
    /// Internal-destination scroll (DisplayListLinks.swift); acted on once per token.
    var reveal: CaretFollowController.Request? = nil
    /// Reported when the reader scrolls this pane by hand.
    var onUserScroll: (() -> Void)? = nil
    /// Page under the viewport's top edge (PreviewAnchorProbe); drives the
    /// header's page indicator and the display-list-v2-window consumer.
    var onVisiblePage: ((Int) -> Void)? = nil
    /// Active `navigation` after capability gating (nil → no link behaviour).
    var navigation: RenderingV2.Navigation? = nil
    var onLink: ((RenderingV2.Navigation.Link) -> Void)? = nil
    /// The page a keyboard Page Up/Down landed on (PreviewPageStep); the
    /// shell announces it to VoiceOver.
    var onPageJump: ((Int) -> Void)? = nil
    let onSelect: (V2Geometry.Hit) -> Void
    @Environment(\.displayScale) private var displayScale
    /// Page Up / Page Down while the pane has keyboard focus (PreviewAnchor.swift).
    @State private var pageJump: PreviewPageJump?

    /// The caret marks and paragraph band for one page, gated on the page's
    /// source bounds so pages that cannot hold either never walk their items.
    static func caretHighlights(page: RenderingV2.Page, prepared: V2PreparedPage, byte: Int?, path: String,
                                paragraph: Range<Int>?) -> [V2Geometry.CaretHighlight] {
        guard let byte else { return [] }
        let paragraph = paragraph.flatMap { prepared.mayOverlap($0, path: path) ? $0 : nil }
        guard paragraph != nil || prepared.mayContain(byte: byte, path: path) else { return [] }
        return V2Geometry.caretHighlights(containing: byte, path: path, in: page, paragraph: paragraph)
    }

    var body: some View {
        GeometryReader { geo in
            let _ = V2ScrollBench.crumb("pane-body")
            let widest = frame.list.pages.map(\.widthPt).max() ?? 612
            let fit = min(1, max(0.2, (geo.size.width - 48) / widest))
            let scale = V2ScrollBench.pinnedScale(displayScale: displayScale) ?? PreviewZoom.scale(fit: fit, zoom: zoom)
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
                LazyVStack(spacing: DS.Preview.pageSpacing) {
                    ForEach(Array(frame.prepared.enumerated()), id: \.element.number) { index, prepared in
                        if let page = frame.page(number: prepared.number) {
                            PageV2View(page: page, prepared: prepared, pageToken: frame.pageToken(at: index), frameRevision: frame.list.revision, expectedDraws: expectedDraws,
                                       totalPages: frame.list.pages.count, dark: dark, stale: stale, scale: scale, displayScale: displayScale,
                                       // Only pages whose cluster sources can contain the caret (or overlap
                                       // its paragraph) walk their items.
                                       caretHighlights: Self.caretHighlights(page: page, prepared: prepared, byte: caretByte, path: caretPath, paragraph: caretParagraph),
                                       navigation: navigation, onLink: onLink, onSelect: onSelect)
                                .equatable()
                                .id(page.number)
                        }
                    }
                }
                .padding(DS.Preview.pageSpacing)
                .background(PreviewAnchorKeeper(layout: layout, follow: follow, reveal: reveal, onUserScroll: onUserScroll, onVisiblePage: onVisiblePage,
                                                pageJump: pageJump, onPageJump: onPageJump,
                                                elidedPages: Set(frame.list.pages.lazy.filter { !$0.resident }.map(\.number))))
            }
            .onChange(of: fit, initial: true) { _, f in onFitScale?(f) }
            // Keyboard: the pane takes focus (Tab under Full Keyboard Access, or
            // VoiceOver's cursor) and Page Up/Down step whole pages (PreviewPageStep).
            .focusable()
            .onKeyPress(.pageDown) { pageJump = PreviewPageJump(token: (pageJump?.token ?? 0) + 1, step: .down); return .handled }
            .onKeyPress(.pageUp) { pageJump = PreviewPageJump(token: (pageJump?.token ?? 0) + 1, step: .up); return .handled }
        }
        .background(dark ? DS.Preview.darkGround : DS.Colors.surfaceGround)
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

enum TapResolution: Equatable {
    case link(RenderingV2.Navigation.Link)
    case select(V2Geometry.Hit)
    case none
}

func resolveTap(location: CGPoint, scale: CGFloat, page: RenderingV2.Page,
                navigation: RenderingV2.Navigation?) -> TapResolution {
    // A non-positive scale would divide `location` into NaN/infinity below,
    // which traps converting to the Int64 tick space in DisplayListLinks.ticks.
    guard scale > 0 else { return .none }
    let pagePoint = CGPoint(x: location.x / scale, y: location.y / scale)
    if let navigation, let link = DisplayListLinks.hit(navigation, page: page.number,
                                                        viewX: pagePoint.x, viewY: pagePoint.y, scale: 1) {
        return .link(link)
    }
    if let hit = V2Geometry.hit(page: page, viewPoint: location, scale: scale) { return .select(hit) }
    return .none
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
    /// Document page count for the VoiceOver label ("Page 3 of 12").
    var totalPages = 0
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
        a.pageToken == b.pageToken && a.page.number == b.page.number && a.totalPages == b.totalPages
            && a.dark == b.dark && a.scale == b.scale && a.displayScale == b.displayScale && a.caretHighlights == b.caretHighlights
            && a.navigation == b.navigation
    }

    var body: some View {
        if page.resident { resident } else { placeholder }
    }

    /// Elided page of a windowed frame (window proposal §4): geometry only.
    /// It keeps the document's scroll extent — scrolling toward it moves the
    /// anchor, which re-requests the window (V2PageWindow.swift) — with no
    /// bitmap machinery, no hover/tap geometry and no navigation (§4.1: a
    /// windowed reply never authorises a source action outside its coverage).
    private var placeholder: some View {
        let size = CGSize(width: page.widthPt * scale, height: page.heightPt * scale)
        let labelColor: Color = dark ? DS.Preview.darkLabel : DS.Preview.lightLabel
        let pageBackground: Color = dark ? DS.Preview.darkPage : .white
        return Rectangle().fill(pageBackground)
            .frame(width: size.width, height: size.height)
            .shadow(radius: DS.Preview.pageShadowRadius)
            .overlay(alignment: .bottomTrailing) {
                Text("page \(page.number) · not loaded").font(DS.Fonts.secondary).foregroundStyle(labelColor).padding(DS.Space.xs)
            }
            .overlay(alignment: .topLeading) { accessibility(size) } // "Page n of m, not loaded"
            .accessibilityIdentifier("v2-page-elided")
    }

    @ViewBuilder private var resident: some View {
        let _ = V2ScrollBench.crumb("body p\(page.number)")
        let size = CGSize(width: page.widthPt * scale, height: page.heightPt * scale)
        // Above about 3 px/pt the page is tiled (V2TileGrid, DESIGN §6.2): the view
        // holds 512 px tiles of the visible area, rasterized off the main thread;
        // the rasterizer only supplies a low-resolution whole-page backdrop (seen
        // where a tile has not landed yet, and during a pinch).
        let pixelsPerPoint = Double(scale * displayScale)
        let tiles = V2TileGrid.tiles(pixelsPerPoint)
            ? V2TileSource(page: prepared, pageToken: pageToken, pixelsPerPoint: pixelsPerPoint, displayScale: Double(displayScale), dark: dark)
            : nil
        // Reading the slot through `image(for:)` subscribes this page to its bitmap's arrival.
        let bitmap = V2PageRasterizer.shared.image(for: prepared, pageToken: pageToken,
                                                   pixelsPerPoint: tiles == nil ? pixelsPerPoint : V2TileGrid.backdropPixelsPerPoint, dark: dark)
        // A stale page keeps its label and colour: the previous frame stays on screen
        // unchanged while the next one is verified (typing must not flash the pages).
        let label = bitmap == nil && tiles == nil ? "page \(page.number) · rasterizing…" : "page \(page.number)"
        let labelColor: Color = dark ? DS.Preview.darkLabel : DS.Preview.lightLabel
        let pageBackground: Color = dark ? DS.Preview.darkPage : .white
        // The bitmap is the contents of a CALayer (PageBitmapLayer): CoreAnimation
        // composites it on every later pass without any drawing on the main thread;
        // a new bitmap is one `layer.contents` assignment. Caret/hover marks are a
        // separate small overlay that exists only while there is something to mark.
        let canvas = PageBitmapLayer(bitmap: bitmap, tiles: tiles, pageToken: pageToken, pageNumber: page.number, frameRevision: frameRevision,
                                     expectedDraws: expectedDraws, background: dark ? DS.Preview.darkPageCG : DS.Preview.lightPageCG)
            .frame(width: size.width, height: size.height)
            .background(Rectangle().fill(pageBackground).shadow(radius: DS.Preview.pageShadowRadius))
            .overlay {
                if !caretHighlights.isEmpty || hover != nil {
                    PageV2Marks(scale: scale, dark: dark, caretHighlights: caretHighlights, hover: hover).equatable().allowsHitTesting(false)
                }
            }
        canvas
            .contentShape(Rectangle())
            .onContinuousHover { phase in
                switch phase {
                case .active(let p):
                    guard scale > 0 else {
                        linkHover = nil
                        hover = nil
                        if linkCursorPushed { NSCursor.pop(); linkCursorPushed = false }
                        break
                    }
                    let pagePoint = CGPoint(x: p.x / scale, y: p.y / scale)
                    if let nav = navigation, let link = DisplayListLinks.hit(nav, page: page.number, viewX: pagePoint.x, viewY: pagePoint.y, scale: 1) {
                        linkHover = link
                        hover = nil
                        if !linkCursorPushed { NSCursor.pointingHand.push(); linkCursorPushed = true }
                    } else {
                        linkHover = nil
                        hover = V2Geometry.hit(page: page, viewPoint: p, scale: scale)
                        if linkCursorPushed { NSCursor.pop(); linkCursorPushed = false }
                    }
                case .ended:
                    hover = nil
                    linkHover = nil
                    if linkCursorPushed { NSCursor.pop(); linkCursorPushed = false }
                }
            }
            .onTapGesture { location in
                switch resolveTap(location: location, scale: scale, page: page, navigation: navigation) {
                case .link(let link):
                    onLink?(link)
                case .select(let hit):
                    onSelect(hit)
                case .none:
                    break
                }
            }
            .overlay(alignment: .bottomTrailing) {
                // Colored for the PAGE background (white or dark), not the window appearance.
                Text(label).font(DS.Fonts.secondary).foregroundStyle(labelColor).padding(DS.Space.xs)
            }
            .overlay(alignment: .topLeading) { accessibility(size) }
            .help(helpText)
    }

    /// The page's VoiceOver tree (PreviewV2Accessibility.swift): a landmark
    /// whose value is the page text, one element per line. Built lazily by
    /// the AppKit view, so it costs nothing per keystroke unless VoiceOver
    /// is reading the page; never hit-tested, so clicks reach the page.
    private func accessibility(_ size: CGSize) -> some View {
        PageV2AccessibilityOverlay(page: page, pageToken: pageToken, totalPages: totalPages, scale: scale, onSelect: onSelect)
            .frame(width: size.width, height: size.height, alignment: .topLeading)
            .allowsHitTesting(false)
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
/// With `tiles`, the page is tiled (V2TileGrid) and `bitmap` is only the
/// low-resolution backdrop under the tiles.
private struct PageBitmapLayer: NSViewRepresentable {
    let bitmap: CGImage?
    var tiles: V2TileSource? = nil
    let pageToken: String
    let pageNumber: Int
    var frameRevision = 0
    var expectedDraws = 1
    let background: CGColor

    func makeNSView(context: Context) -> PageBitmapView { PageBitmapView() }

    func updateNSView(_ view: PageBitmapView, context: Context) {
        view.show(bitmap, tiles: tiles, pageToken: pageToken, pageNumber: pageNumber, frameRevision: frameRevision, expectedDraws: expectedDraws, background: background)
    }
    /// Takes exactly the proposed size. Without this, SwiftUI measures the
    /// AppKit view through Auto Layout (`AppKitPlatformViewHost.intrinsicLayoutTraits`
    /// → `-[NSView measureMin:max:ideal:]`, an NSISEngine solve) on every layout
    /// pass, scrolling included, and again for every page the lazy stack
    /// materializes (P0-PREVIEW-TILES, 120 Hz bench).
    func sizeThatFits(_ proposal: ProposedViewSize, nsView: PageBitmapView, context: Context) -> CGSize? {
        proposal.replacingUnspecifiedDimensions(by: .zero)
    }

}

/// What a tiled page rasterizes: one prepared page at the pane's pixels per
/// point. Tiles of the same token, scale and appearance are interchangeable.
struct V2TileSource {
    let page: V2PreparedPage
    let pageToken: String
    let pixelsPerPoint: Double
    /// Backing pixels per view point (the window's backing scale).
    let displayScale: Double
    let dark: Bool

    func sameTiles(as other: V2TileSource?) -> Bool {
        guard let other else { return false }
        return other.pageToken == pageToken && other.pixelsPerPoint == pixelsPerPoint && other.displayScale == displayScale && other.dark == dark
    }

    /// Same pixel grid in the same place (scale, backing scale, page size):
    /// a tile of `other` sits exactly where this source's tile of the same
    /// index goes, so it can stay up until its replacement lands.
    func sameGeometry(as other: V2TileSource?) -> Bool {
        guard let other else { return false }
        return other.pixelsPerPoint == pixelsPerPoint && other.displayScale == displayScale
            && other.page.widthPt == page.widthPt && other.page.heightPt == page.heightPt
    }

    /// The whole page's bitmap size, which the tiles partition.
    var pixelSize: (width: Int, height: Int) { GlyphRunRenderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: pixelsPerPoint) }
    /// The page view's height in view points at this scale.
    var viewHeight: Double { page.heightPt * pixelsPerPoint / displayScale }

    /// A rectangle in page points (top-left origin) → this page view's y-up points.
    func viewRect(points r: CGRect) -> CGRect {
        let k = CGFloat(pixelsPerPoint / displayScale)
        return CGRect(x: r.minX * k, y: CGFloat(viewHeight) - r.maxY * k, width: r.width * k, height: r.height * k)
    }
}

/// High-zoom tiling (DESIGN §6.2, P0-PREVIEW-TILES). Above `threshold`
/// pixels per point a whole-page bitmap is large (8 px/pt: 4896×6336 px,
/// 124 MB for a US-letter page) and slow (about 18 ms), while the viewport
/// shows a small part of it. A tiled page instead holds 512 px tiles of its
/// visible area (plus a prefetch margin). Every tile is rasterized off the
/// main thread (DESIGN §1.2: no drawing on main), in parallel
/// (`DispatchQueue.concurrentPerform` on `queue`), and installed on main as
/// a `contents` assignment: compositing only. Until a tile lands, the page
/// shows what it showed before: the 2 px/pt backdrop under the tiles, the
/// previous content's tile at the same place, or, after a zoom, the previous
/// scale's tiles stretched to the new one. The tiles are plain sublayers of
/// the page view, not a `CATiledLayer`: no fade-in and no resize jitter. A
/// tile is a pixel-exact window of `rasterize(page:)` (PreviewV2TileTests),
/// so V2Parity's zero tolerance covers tiled pages.
enum V2TileGrid {
    static let tilePixels = 512
    /// Pixels per point above which pages tile. `FLASHTEX_V2_TILE_THRESHOLD`
    /// overrides it for measurements (a huge value turns tiling off).
    static let threshold: Double = Double(ProcessInfo.processInfo.environment["FLASHTEX_V2_TILE_THRESHOLD"] ?? "") ?? 3
    /// The whole-page bitmap under a tiled page's tiles: shown, stretched
    /// with linear filtering, wherever no tile is yet (and during a pinch).
    static let backdropPixelsPerPoint = 2.0
    /// Tiles within this many pixels of the visible rect are rasterized too,
    /// so a scroll step usually finds its next row ready.
    static let prefetchPixels: CGFloat = 256
    /// Every tile job runs here (serial: jobs keep their order, visible tiles
    /// first; each job draws its tiles in parallel).
    static let queue = DispatchQueue(label: "flashtex.preview-v2.tiles", qos: .userInteractive)

    struct PixelRect: Hashable { var x: Int; var y: Int; var width: Int; var height: Int }
    struct Index: Hashable { var column: Int; var row: Int }

    static func tiles(_ pixelsPerPoint: Double) -> Bool { pixelsPerPoint > threshold }

    /// Tile `index` of a page bitmap `pageWidth`×`pageHeight` px, top-left origin.
    static func rect(_ index: Index, pageWidth: Int, pageHeight: Int) -> PixelRect {
        let x = index.column * tilePixels, y = index.row * tilePixels
        return PixelRect(x: x, y: y, width: min(tilePixels, pageWidth - x), height: min(tilePixels, pageHeight - y))
    }

    /// The tiles intersecting `pixels` (page pixels, top-left origin), row-major.
    static func indices(covering pixels: CGRect, pageWidth: Int, pageHeight: Int) -> [Index] {
        let r = pixels.intersection(CGRect(x: 0, y: 0, width: pageWidth, height: pageHeight))
        guard !r.isNull, r.width > 0, r.height > 0 else { return [] }
        let c0 = Int(r.minX.rounded(.down)) / tilePixels, c1 = (Int(r.maxX.rounded(.up)) - 1) / tilePixels
        let r0 = Int(r.minY.rounded(.down)) / tilePixels, r1 = (Int(r.maxY.rounded(.up)) - 1) / tilePixels
        var out: [Index] = []
        out.reserveCapacity((c1 - c0 + 1) * (r1 - r0 + 1))
        for row in r0...r1 { for column in c0...c1 { out.append(Index(column: column, row: row)) } }
        return out
    }

    /// Parallel workers for a tile pass.
    static let workers = max(1, min(ProcessInfo.processInfo.activeProcessorCount, 8))

    /// Rasterizes `indices` of `source` in parallel (worker k draws tiles
    /// k, k + n, k + 2n…), returned in the order given. A page with path or
    /// image items is rasterized whole once and the tiles cut from it
    /// (`V2PreparedPage.tilesByTranslation`).
    static func rasterize(_ indices: [Index], of source: V2TileSource) -> [CGImage?] {
        let (w, h) = source.pixelSize
        guard source.page.tilesByTranslation else {
            // Path or image items: cut from one whole-page raster (exact).
            return GlyphRunRenderer.cutTiles(source.page, scale: source.pixelsPerPoint, dark: source.dark,
                                             rects: indices.map { rect($0, pageWidth: w, pageHeight: h) })
        }
        let n = min(workers, indices.count)
        var out = [CGImage?](repeating: nil, count: indices.count)
        guard n > 0 else { return out }
        out.withUnsafeMutableBufferPointer { buffer in
            let base = buffer.baseAddress!
            DispatchQueue.concurrentPerform(iterations: n) { worker in
                var k = worker
                while k < indices.count {
                    base[k] = GlyphRunRenderer.rasterizeTile(source.page, scale: source.pixelsPerPoint, dark: source.dark,
                                                             rect: rect(indices[k], pageWidth: w, pageHeight: h))
                    k += n
                }
            }
        }
        return out
    }

    /// The image as CoreAnimation would convert it for a window in
    /// `space`, done here (off-main) instead: a layer's CGImage contents in
    /// another colour space is redrawn into the window's space during the
    /// commit, on the main thread (measured: about 0.4 ms per 512 px tile,
    /// 3 ms per 2 px/pt backdrop; the scroll bench's dropped frames). The
    /// sRGB raster stays the one parity compares; this is display only.
    static func displayImage(_ image: CGImage, in space: CGColorSpace?) -> CGImage {
        guard let space, space.model == .rgb, image.colorSpace != space,
              let ctx = CGContext(data: nil, width: image.width, height: image.height, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: space, bitmapInfo: GlyphRunRenderer.bitmapInfo) else { return image }
        ctx.interpolationQuality = .none
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        return ctx.makeImage() ?? image
    }

    /// Tile-job counters for the scroll bench, updated on main when a job's
    /// tiles are installed (the jobs themselves run on `queue`).
    @MainActor static var jobs = 0
    @MainActor static var jobTiles = 0
    @MainActor static var jobMs = 0.0
    @MainActor static var maxJobMs = 0.0
    /// Queue-to-install latency of the slowest job (ms): how long a page
    /// showed its placeholder for a tile.
    @MainActor static var maxLatencyMs = 0.0
    @MainActor static func resetCounters() { jobs = 0; jobTiles = 0; jobMs = 0; maxJobMs = 0; maxLatencyMs = 0 }

    /// Actions disabled on tile layers: contents, frames and filters change
    /// without CoreAnimation's implicit fades (the jitter CATiledLayer shows).
    static let noActions: [String: CAAction] = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull(), "frame": NSNull(),
                                                "hidden": NSNull(), "magnificationFilter": NSNull(), "minificationFilter": NSNull(),
                                                "sublayers": NSNull(), "onOrderIn": NSNull(), "onOrderOut": NSNull()]
}

/// A page view's tile generation, readable from the tile queue: a job
/// whose generation is no longer current (the page, scale or appearance
/// changed after it was queued) is skipped before it draws anything.
final class V2TileGeneration: @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0
    func bump() -> Int { lock.lock(); defer { lock.unlock() }; value &+= 1; return value }
    var current: Int { lock.lock(); defer { lock.unlock() }; return value }
}

/// The AppKit view behind `PageBitmapLayer` (test-visible: `shown`, `show`).
final class PageBitmapView: NSView {
    private(set) var shown: CGImage?
    /// The token of the page `shown` belongs to.
    private var shownToken: String?
    /// How many times a new bitmap (or, tiled, a new page's tiles) was installed (tests).
    private(set) var installs = 0

    /// Tiled mode: what the tiles show, and the tiles held (visible area plus margin).
    private(set) var tileSource: V2TileSource?
    /// Bumped per tile source; a tile job installs only if it is unchanged.
    private let generation = V2TileGeneration()
    private var tileGeneration = 0
    /// Tiles of the current generation queued or being rasterized off-main.
    private var requested: Set<V2TileGrid.Index> = []
    private var tileLayers: [V2TileGrid.Index: CALayer] = [:]
    /// Tiles painted from an earlier source of the same geometry (new page
    /// content, or the other appearance): they stay up until replaced.
    private var staleTiles: Set<V2TileGrid.Index> = []
    /// Tiles of an earlier scale, re-framed to the current one (stretched,
    /// linear) and kept under the current tiles until every visible tile of
    /// the current scale has landed. `points`: the tile's page rectangle in
    /// points, top-left origin.
    private var outgoing: [(layer: CALayer, points: CGRect)] = []
    static let maxOutgoing = 96
    /// The typing bench's paint point, recorded when the new page's first tiles land.
    private var pendingPaint: (revision: Int, draws: Int, page: Int, token: String)?
    /// Tiles installed over this view's life, and the last job's raster time (tests, bench).
    private(set) var tileRasterizations = 0
    private(set) var lastTilePassMs = 0.0
    /// Tile jobs dispatched (tests: none is dispatched for a no-op update).
    private(set) var tileJobs = 0
    private var observers: [NSObjectProtocol] = []
    private weak var observedClip: NSClipView?
    private var pinching = false

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        // The whole bitmap fills the layer: at the pane's pixels-per-point the
        // bitmap's pixel size equals the layer's backing size (1:1, no resampling).
        layer?.contentsGravity = .resize
        layer?.magnificationFilter = .nearest
        layer?.minificationFilter = .nearest
        layer?.masksToBounds = true
        V2PinchTransform.register(self)
        V2ScrollBench.crumb("view-init")
    }
    required init?(coder: NSCoder) { nil }
    deinit { for o in observers { NotificationCenter.default.removeObserver(o) } }
    override var isOpaque: Bool { true }
    /// Mouse events belong to the SwiftUI page view around this layer
    /// (hover geometry, tap navigation); the bitmap never takes them.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    /// The tiles held now (tests, bench).
    var tileCount: Int { tileLayers.count }
    var tileIndices: Set<V2TileGrid.Index> { Set(tileLayers.keys) }
    var outgoingCount: Int { outgoing.count }
    var outgoingFrames: [CGRect] { outgoing.map(\.layer.frame) }
    var staleCount: Int { staleTiles.count }
    /// Tiles queued or being rasterized for the current source.
    var pendingTiles: Int { requested.count }
    func tileImage(_ index: V2TileGrid.Index) -> CGImage? { tileLayers[index].map { $0.contents as! CGImage } }
    func tileFrame(_ index: V2TileGrid.Index) -> CGRect? { tileLayers[index]?.frame }
    /// Bitmap bytes this view holds: its tiles (outgoing included) plus the whole-page bitmap or backdrop.
    var retainedBytes: Int {
        func bytes(_ l: CALayer) -> Int { (l.contents as! CGImage?).map { $0.bytesPerRow * $0.height } ?? 0 }
        return tileLayers.values.reduce(shown.map { $0.bytesPerRow * $0.height } ?? 0) { $0 + bytes($1) }
            + outgoing.reduce(0) { $0 + bytes($1.layer) }
    }
    /// Visible tiles with no tile of the current geometry yet (the page shows
    /// its backdrop or the previous scale there). Bench instrumentation.
    var missingVisibleTiles: Int {
        guard let source = tileSource else { return 0 }
        return wantedTiles(source).visible.filter { tileLayers[$0] == nil }.count
    }

    /// Installs `bitmap` as the layer contents when it is not the one shown.
    /// With `tiles`, `bitmap` is the backdrop and the visible tiles are
    /// requested from the tile queue (never drawn here). Returns whether
    /// anything on screen changed or will change.
    @MainActor
    @discardableResult
    func show(_ bitmap: CGImage?, tiles: V2TileSource? = nil, pageToken: String, pageNumber: Int, frameRevision: Int, expectedDraws: Int, background: CGColor) -> Bool {
        layer?.backgroundColor = background
        V2ScrollBench.crumb("show p\(pageNumber)\(tiles == nil ? "" : " tiled")")
        if let tiles {
            var changed = false
            if !tiles.sameTiles(as: tileSource) {
                if tiles.sameGeometry(as: tileSource) {
                    // Same grid (new content, or the other appearance): the held
                    // tiles stay up, each until its replacement lands.
                    staleTiles.formUnion(tileLayers.keys)
                } else if let old = tileSource {
                    retire(old)
                }
                if tileSource?.pageToken != tiles.pageToken || tileSource == nil {
                    pendingPaint = (frameRevision, expectedDraws, pageNumber, tiles.pageToken)
                }
                tileSource = tiles
                tileGeneration = generation.bump()
                requested.removeAll()
                reframeOutgoing(tiles)
                changed = true
            }
            // The backdrop is stretched: linear, and never counted as a paint.
            setRootFilter(linear: true)
            if let bitmap, bitmap !== shown {
                shown = bitmap
                shownToken = pageToken
                installBackdrop(bitmap)
                changed = true
            }
            if changed { updateTiles() }
            return changed
        }
        if tileSource != nil { removeTiles() }
        guard bitmap !== shown else { return false }
        // A zoom or appearance change of the same page keeps the previous
        // bitmap (stretched, linear) until the new one arrives: the page never
        // flashes to its background while it rasterizes.
        if bitmap == nil, shown != nil, shownToken == pageToken { setRootFilter(linear: true); return false }
        shown = bitmap
        shownToken = bitmap == nil ? nil : pageToken
        setRootFilter(linear: pinching)
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

    /// The window's colour space, which CoreAnimation converts contents to.
    /// `NSWindow.colorSpace` is nil unless set: the window then renders in its
    /// screen's colour space.
    private var displayColorSpace: CGColorSpace? { (window?.colorSpace ?? window?.screen?.colorSpace)?.cgColorSpace }
    /// The colour space the held tiles were converted to.
    private var tileColorSpace: CGColorSpace?

    /// A new screen (or a changed display profile): the held tiles are in the
    /// old colour space, so they are redone off-main, each staying up until
    /// its replacement lands (as for new page content).
    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        guard tileSource != nil, let space = displayColorSpace, tileColorSpace.map({ $0 != space }) ?? false else { return }
        staleTiles.formUnion(tileLayers.keys)
        tileGeneration = generation.bump()
        requested.removeAll()
        if let shown { installBackdrop(shown) }
        updateTiles()
    }

    /// A tiled page's backdrop, converted to the window's colour space on
    /// the tile queue and then installed (the previous contents stay up).
    /// Not before the view is in a window: SwiftUI shows a page's view
    /// before it inserts it, and an sRGB backdrop assigned then was converted
    /// by CoreAnimation on the main thread in the commit that inserted the
    /// page (about 8 MB at 2 px/pt; the 120 Hz bench's page-entry hitch).
    /// `viewDidMoveToWindow` installs it.
    private func installBackdrop(_ bitmap: CGImage) {
        V2ScrollBench.crumb("backdrop \(bitmap.width)")
        guard window != nil else { return }
        guard let space = displayColorSpace, bitmap.colorSpace != space else { layer?.contents = bitmap; return }
        V2TileGrid.queue.async { [weak self] in
            let converted = V2TileGrid.displayImage(bitmap, in: space)
            DispatchQueue.main.async { [weak self] in
                MainActor.assumeIsolated {
                    guard let self, self.shown === bitmap, self.tileSource != nil else { return }
                    CATransaction.begin(); CATransaction.setDisableActions(true)
                    self.layer?.contents = converted
                    CATransaction.commit()
                }
            }
        }
    }

    private func setRootFilter(linear: Bool) {
        let filter: CALayerContentsFilter = linear ? .linear : .nearest
        guard layer?.magnificationFilter != filter else { return }
        layer?.magnificationFilter = filter
        layer?.minificationFilter = filter
    }

    /// Pinch in progress (V2PinchTransform): every bitmap resamples linearly
    /// under the GPU transform; 1:1 nearest sampling returns when it settles.
    func setPinching(_ on: Bool) {
        pinching = on
        CATransaction.begin(); CATransaction.setDisableActions(true)
        let filter: CALayerContentsFilter = on ? .linear : .nearest
        for l in tileLayers.values { l.magnificationFilter = filter; l.minificationFilter = filter }
        setRootFilter(linear: on || tileSource != nil)
        CATransaction.commit()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        V2ScrollBench.crumb("moved-to-window \(window != nil)")
        if window != nil, tileSource != nil, let shown { installBackdrop(shown) }
        observeScroll()
        updateTiles()
        V2ScrollBench.startIfRequested(from: self)
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        V2ScrollBench.crumb("setFrameSize")
        updateTiles()
    }

    /// Follows the enclosing scroll view's visible rect (scrolling, resizing).
    private func observeScroll() {
        guard let clip = enclosingScrollView?.contentView, clip !== observedClip else { return }
        for o in observers { NotificationCenter.default.removeObserver(o) }
        observers.removeAll()
        observedClip = clip
        clip.postsBoundsChangedNotifications = true
        clip.postsFrameChangedNotifications = true
        for name in [NSView.boundsDidChangeNotification, NSView.frameDidChangeNotification] {
            observers.append(NotificationCenter.default.addObserver(forName: name, object: clip, queue: nil) { [weak self] _ in
                MainActor.assumeIsolated { self?.updateTiles() }
            })
        }
    }

    private func removeTiles() {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for l in tileLayers.values { l.removeFromSuperlayer() }
        for o in outgoing { o.layer.removeFromSuperlayer() }
        CATransaction.commit()
        tileLayers.removeAll()
        staleTiles.removeAll()
        outgoing.removeAll()
        tileSource = nil
        tileGeneration = generation.bump()
        requested.removeAll()
        pendingPaint = nil
    }

    /// A new scale: the held tiles become `outgoing`, stretched to the new
    /// scale by `reframeOutgoing` and removed once the new tiles cover the view.
    private func retire(_ old: V2TileSource) {
        let (pw, ph) = old.pixelSize
        let ppp = CGFloat(old.pixelsPerPoint)
        for (index, layer) in tileLayers {
            let r = V2TileGrid.rect(index, pageWidth: pw, pageHeight: ph)
            layer.magnificationFilter = .linear
            layer.minificationFilter = .linear
            outgoing.append((layer, CGRect(x: CGFloat(r.x) / ppp, y: CGFloat(r.y) / ppp, width: CGFloat(r.width) / ppp, height: CGFloat(r.height) / ppp)))
        }
        tileLayers.removeAll()
        staleTiles.removeAll()
        if outgoing.count > Self.maxOutgoing {
            let excess = outgoing.count - Self.maxOutgoing
            CATransaction.begin(); CATransaction.setDisableActions(true)
            for o in outgoing.prefix(excess) { o.layer.removeFromSuperlayer() }
            CATransaction.commit()
            outgoing.removeFirst(excess)
        }
    }

    private func reframeOutgoing(_ source: V2TileSource) {
        guard !outgoing.isEmpty else { return }
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for o in outgoing { o.layer.frame = source.viewRect(points: o.points) }
        CATransaction.commit()
    }

    /// The tiles for the visible rect: `visible` (on screen now), `want`
    /// (plus the prefetch margin) and `keep` (one tile more; anything beyond
    /// is dropped). All empty for a page out of view.
    private func wantedTiles(_ source: V2TileSource) -> (visible: Set<V2TileGrid.Index>, want: [V2TileGrid.Index], keep: Set<V2TileGrid.Index>) {
        let visible = visibleRect
        guard !visible.isEmpty else { return ([], [], []) }
        let (pw, ph) = source.pixelSize
        let ds = CGFloat(source.displayScale), viewHeight = CGFloat(source.viewHeight)
        // View points (y up) → page pixels (top-left origin).
        let px = CGRect(x: visible.minX * ds, y: (viewHeight - visible.maxY) * ds, width: visible.width * ds, height: visible.height * ds)
        let m = V2TileGrid.prefetchPixels, t = CGFloat(V2TileGrid.tilePixels)
        return (Set(V2TileGrid.indices(covering: px, pageWidth: pw, pageHeight: ph)),
                V2TileGrid.indices(covering: px.insetBy(dx: -m, dy: -m), pageWidth: pw, pageHeight: ph),
                Set(V2TileGrid.indices(covering: px.insetBy(dx: -m - t, dy: -m - t), pageWidth: pw, pageHeight: ph)))
    }

    /// Brings the tiles in line with the visible rect, drawing nothing on
    /// the main thread: tiles beyond a one-tile margin are dropped (nothing
    /// is kept or rasterized for a page scrolled out of view), and missing
    /// or stale tiles of the visible rect, then of the prefetch margin, are
    /// queued as two jobs on `V2TileGrid.queue`. Meanwhile the page shows
    /// its backdrop, its stale tiles or the previous scale's tiles there.
    func updateTiles() {
        // Not before the view is in a window: tiles are converted to the
        // window's colour space, and SwiftUI updates a page's view before it
        // inserts it (viewDidMoveToWindow asks again).
        guard let source = tileSource, window != nil else { return }
        if observedClip == nil { observeScroll() }
        let (visible, want, keep) = wantedTiles(source)
        // Beyond the keep margin, and stale tiles outside the wanted rect
        // (never re-rasterized there, so never current again).
        let wanted = Set(want)
        let drop = tileLayers.keys.filter { !keep.contains($0) || (staleTiles.contains($0) && !wanted.contains($0)) }
        if !drop.isEmpty {
            CATransaction.begin(); CATransaction.setDisableActions(true)
            for index in drop { tileLayers.removeValue(forKey: index)?.removeFromSuperlayer(); staleTiles.remove(index) }
            CATransaction.commit()
        }
        retireOutgoingIfCovered(visible)
        let missing = want.filter { (tileLayers[$0] == nil || staleTiles.contains($0)) && !requested.contains($0) }
        guard !missing.isEmpty else { return }
        let now = missing.filter { visible.contains($0) }, later = missing.filter { !visible.contains($0) }
        if !now.isEmpty { request(now, source: source) }
        if !later.isEmpty { request(later, source: source) }
    }

    /// The previous scale's tiles go once every visible tile of this scale is
    /// up, or when the page is out of view. (A view whose frame has not yet
    /// followed the new scale may compute no visible tile: they stay then.)
    private func retireOutgoingIfCovered(_ visible: Set<V2TileGrid.Index>) {
        guard !outgoing.isEmpty,
              visibleRect.isEmpty || (!visible.isEmpty && visible.allSatisfy({ tileLayers[$0] != nil })) else { return }
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for o in outgoing { o.layer.removeFromSuperlayer() }
        CATransaction.commit()
        outgoing.removeAll()
    }

    /// One tile job: rasterized on the tile queue, installed on main if the
    /// generation is unchanged and the tile is still held or wanted.
    private func request(_ indices: [V2TileGrid.Index], source: V2TileSource) {
        requested.formUnion(indices)
        tileJobs += 1
        let expected = tileGeneration, generation = self.generation, space = displayColorSpace
        tileColorSpace = space
        let queued = MonotonicClock.nowNs()
        V2TileGrid.queue.async { [weak self] in
            // Queued before the page, scale or appearance changed: skip it undrawn.
            guard generation.current == expected else { return }
            dispatchPrecondition(condition: .notOnQueue(.main)) // DESIGN §1.2: never on main
            let t0 = MonotonicClock.nowNs()
            var rastered = V2TileGrid.rasterize(indices, of: source)
            if space != nil {
                rastered.withUnsafeMutableBufferPointer { b in
                    DispatchQueue.concurrentPerform(iterations: b.count) { i in b[i] = b[i].map { V2TileGrid.displayImage($0, in: space) } }
                }
            }
            let images = rastered, ms = Double(MonotonicClock.nowNs() &- t0) / 1e6
            DispatchQueue.main.async { [weak self] in
                MainActor.assumeIsolated {
                    guard let self, self.tileGeneration == expected, let current = self.tileSource else { return }
                    self.requested.subtract(indices)
                    let keep = self.wantedTiles(current).keep
                    let ready = zip(indices, images).compactMap { index, image in
                        image.flatMap { keep.contains(index) && (self.tileLayers[index] == nil || self.staleTiles.contains(index)) ? (index, $0) : nil }
                    }
                    self.install(ready.map(\.1), at: ready.map(\.0), source: current, ms: ms)
                    V2TileGrid.jobs += 1
                    V2TileGrid.jobTiles += indices.count
                    V2TileGrid.jobMs += ms
                    V2TileGrid.maxJobMs = max(V2TileGrid.maxJobMs, ms)
                    V2TileGrid.maxLatencyMs = max(V2TileGrid.maxLatencyMs, Double(MonotonicClock.nowNs() &- queued) / 1e6)
                    self.retireOutgoingIfCovered(self.wantedTiles(current).visible)
                    V2ScrollBench.crumb("install-end")
                }
            }
        }
    }

    /// Compositing only: each image becomes a tile layer's contents.
    private func install(_ images: [CGImage], at indices: [V2TileGrid.Index], source: V2TileSource, ms: Double) {
        guard let root = layer, !indices.isEmpty else { return }
        V2ScrollBench.crumb("install \(indices.count)")
        let (pw, ph) = source.pixelSize
        let ds = CGFloat(source.displayScale), viewHeight = CGFloat(source.viewHeight)
        CATransaction.begin(); CATransaction.setDisableActions(true)
        let filter: CALayerContentsFilter = pinching ? .linear : .nearest
        for (index, image) in zip(indices, images) {
            let r = V2TileGrid.rect(index, pageWidth: pw, pageHeight: ph)
            let tile: CALayer
            if let existing = tileLayers[index] { tile = existing } else {
                tile = CALayer()
                tile.actions = V2TileGrid.noActions
                tile.contentsGravity = .resize
                tile.isOpaque = true
                root.addSublayer(tile) // above the backdrop and any outgoing tiles
                tileLayers[index] = tile
            }
            tile.magnificationFilter = filter
            tile.minificationFilter = filter
            tile.contentsScale = ds
            // Top-left pixel rect → this view's y-up points, 1 px = 1/ds pt.
            tile.frame = CGRect(x: CGFloat(r.x) / ds, y: viewHeight - CGFloat(r.y + r.height) / ds,
                                width: CGFloat(r.width) / ds, height: CGFloat(r.height) / ds)
            tile.contents = image
            staleTiles.remove(index)
        }
        CATransaction.commit()
        tileRasterizations += indices.count
        lastTilePassMs = ms
        if let paint = pendingPaint, paint.token == source.pageToken {
            pendingPaint = nil
            installs += 1
            TypingBench.shared.willRender(revision: paint.revision, pages: paint.draws)
            TypingBench.shared.didDraw(page: paint.page)
            if TypingBench.isBenchActive { FlashTeXLog.write("preview-v2: tiles page \(paint.page) \(paint.token.prefix(24)) \(indices.count) tile(s) in \(ms) ms at \(MonotonicClock.nowNs())") }
        }
    }
}

/// Pinch-zoom on the GPU (DESIGN §6.2): while the gesture runs, the preview's
/// scroll content is scaled by a CoreAnimation `sublayerTransform` on the
/// clip view, around the top centre of the viewport, with every page bitmap
/// and tile sampled linearly. Nothing is laid out or rasterized until the
/// gesture ends; then the zoom is committed once and the transform removed
/// in the same pass. Tiled pages then show the previous scale's tiles,
/// stretched, until the new scale's visible tiles land from the tile queue.
/// The top-centre anchor matches PreviewAnchor, which keeps the page point
/// under the viewport's top edge across the committed zoom.
@MainActor
enum V2PinchTransform {
    private static let views = NSHashTable<PageBitmapView>.weakObjects()
    private static weak var clip: NSClipView?
    static var isActive: Bool { clip != nil }

    static func register(_ view: PageBitmapView) { views.add(view) }

    /// Shows the preview at `magnification` × its current zoom. False when
    /// no v2 page is in a scroll view (the caller then zooms directly).
    @discardableResult
    static func update(_ magnification: CGFloat) -> Bool {
        if clip == nil {
            guard let found = views.allObjects.first(where: { $0.window != nil && $0.enclosingScrollView != nil })?.enclosingScrollView?.contentView,
                  found.layer != nil else { return false }
            clip = found
            for view in views.allObjects { view.setPinching(true) }
        }
        guard let clip, let layer = clip.layer else { return false }
        let b = clip.bounds
        let anchor = CGPoint(x: b.midX, y: clip.isFlipped ? b.minY : b.maxY)
        CATransaction.begin(); CATransaction.setDisableActions(true)
        layer.sublayerTransform = transform(magnification, about: anchor, in: layer)
        CATransaction.commit()
        return true
    }

    /// Removes the transform; the caller commits the zoom in the same pass.
    static func end() {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        clip?.layer?.sublayerTransform = CATransform3DIdentity
        CATransaction.commit()
        clip = nil
        for view in views.allObjects { view.setPinching(false) }
    }

    /// Scale by `m` about `point` (in `layer`'s bounds space): a sublayer
    /// transform acts about the layer's anchor point, so the fixed point is
    /// moved there and back.
    static func transform(_ m: CGFloat, about point: CGPoint, in layer: CALayer) -> CATransform3D {
        let a = CGPoint(x: layer.bounds.minX + layer.anchorPoint.x * layer.bounds.width,
                        y: layer.bounds.minY + layer.anchorPoint.y * layer.bounds.height)
        return CATransform3DConcat(CATransform3DMakeScale(m, m, 1),
                                   CATransform3DMakeTranslation((1 - m) * (point.x - a.x), (1 - m) * (point.y - a.y), 0))
    }
}

/// `FLASHTEX_V2_SCROLL_BENCH=<seconds>` (evidence capture, never in the
/// product path): once a v2 page is on screen, scrolls the preview down and
/// up at `FLASHTEX_V2_SCROLL_SPEED` points per second (default 2400) from the
/// display link, and logs frame intervals, dropped frames, the main thread's
/// scroll-step time, the off-main tile jobs, frames with visible tiles still
/// missing and the bitmap bytes held (FLASHTEX_LOG, and JSON to
/// `FLASHTEX_V2_SCROLL_BENCH_OUT` when set). `FLASHTEX_V2_PINCH_HOLD=<m>`
/// instead applies a pinch transform of `m` and holds it (screenshot evidence).
@MainActor
final class V2ScrollBench: NSObject {
    private static var started = false
    private static var running: V2ScrollBench?
    /// What the preview did in the current main run-loop pass (bench only).
    private static var crumbs: [String] = []
    private static var passStartNs: UInt64 = 0
    static func crumb(_ s: @autoclosure () -> String) {
        guard running != nil else { return }
        crumbs.append(s() + String(format: "@%.1f", Double(MonotonicClock.nowNs() &- passStartNs) / 1e6))
    }
    private var observer: CFRunLoopObserver?
    private var longPasses: [String] = []
    private var stalled = false
    private weak var scroll: NSScrollView?
    private let duration: Double
    private let speed: Double
    private var link: CADisplayLink?
    private var start: CFTimeInterval = 0
    private var last: CFTimeInterval = 0
    private var direction: CGFloat = 1
    private var intervals: [Double] = []
    private var nominal: [Double] = []
    private var dropped = 0
    private var framesMissingTiles = 0
    private var missingTilesMax = 0
    private var loadStart: [Double] = []
    private var maxBytes = 0
    private var maxFootprint = 0
    private var pixelsPerPoint = 0.0
    private var tiledPages = 0
    private var bitmapWidth = 0
    private var hitches: [String] = []
    private var lastMissing = 0
    private var lastScrollMs = 0.0
    private var scrollMsMax = 0.0

    private init(scroll: NSScrollView, duration: Double, speed: Double) {
        self.scroll = scroll
        self.duration = duration
        self.speed = speed
    }

    /// `FLASHTEX_V2_SCROLL_PPP=<px/pt>` (evidence capture): pins the pages at
    /// that many pixels per point whatever the pane's width, so bench runs
    /// compare at one scale.
    static let pinnedPixelsPerPoint = ProcessInfo.processInfo.environment["FLASHTEX_V2_SCROLL_PPP"].flatMap(Double.init)
    static func pinnedScale(displayScale: CGFloat) -> CGFloat? {
        pinnedPixelsPerPoint.map { CGFloat($0) / max(displayScale, 1) }
    }

    static func startIfRequested(from view: PageBitmapView) {
        let env = ProcessInfo.processInfo.environment
        guard !started, view.window != nil, let scroll = view.enclosingScrollView else { return }
        if let hold = env["FLASHTEX_V2_PINCH_HOLD"].flatMap(Double.init) {
            started = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                MainActor.assumeIsolated {
                    V2PinchTransform.update(CGFloat(hold))
                    FlashTeXLog.write("preview-v2: pinch hold \(hold) applied")
                }
            }
            return
        }
        guard let seconds = env["FLASHTEX_V2_SCROLL_BENCH"].flatMap(Double.init) else { return }
        started = true
        let speed = env["FLASHTEX_V2_SCROLL_SPEED"].flatMap(Double.init) ?? 2400
        moveToFastestScreen(view.window)
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            MainActor.assumeIsolated {
                let bench = V2ScrollBench(scroll: scroll, duration: seconds, speed: speed)
                running = bench
                bench.begin()
            }
        }
    }

    /// The window restores its saved frame, possibly on a 60 Hz display:
    /// the bench runs on the fastest screen (the built-in 120 Hz panel).
    /// A window covered by other apps' windows gets no display-link
    /// callbacks (occlusion), so the bench also orders it in front of them;
    /// `orderFrontRegardless` does not activate the app or take key focus.
    @discardableResult
    private static func moveToFastestScreen(_ window: NSWindow?) -> Bool {
        guard let window else { return false }
        window.orderFrontRegardless()
        guard let fastest = NSScreen.screens.max(by: { $0.maximumFramesPerSecond < $1.maximumFramesPerSecond }),
              window.screen != fastest else { return false }
        window.setFrame(fastest.visibleFrame, display: true)
        return true
    }

    private func begin() {
        guard let scroll else { return }
        // Again, if it was restored after launch; then let layout settle first.
        if let window = scroll.window, Self.moveToFastestScreen(window) {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { MainActor.assumeIsolated { self.begin() } }
            return
        }
        // The screen's own link: a view's link created right after the window
        // moved screens can stay bound to the old (possibly sleeping) display.
        let screen = NSScreen.screens.max(by: { $0.maximumFramesPerSecond < $1.maximumFramesPerSecond }) ?? scroll.window?.screen
        let link = screen?.displayLink(target: self, selector: #selector(tick(_:))) ?? scroll.displayLink(target: self, selector: #selector(tick(_:)))
        link.preferredFrameRateRange = CAFrameRateRange(minimum: 120, maximum: 120, preferred: 120)
        link.add(to: .main, forMode: .common)
        self.link = link
        V2TileGrid.resetCounters()
        loadStart = Self.loadAverage()
        // Main run-loop passes longer than a frame, with what this code did in them.
        var passStart: UInt64 = 0
        let observer = CFRunLoopObserverCreateWithHandler(nil, CFRunLoopActivity.afterWaiting.rawValue | CFRunLoopActivity.beforeWaiting.rawValue, true, 0) { [weak self] _, activity in
            MainActor.assumeIsolated {
                guard let self else { return }
                let now = MonotonicClock.nowNs()
                if activity == .afterWaiting { passStart = now; Self.passStartNs = now; Self.crumbs.removeAll(); return }
                guard passStart > 0 else { return }
                let ms = Double(now &- passStart) / 1e6
                if ms > 8, self.longPasses.count < 40 {
                    self.longPasses.append(String(format: "t=%.3fs %.1fms %@", CACurrentMediaTime() - self.start, ms, Self.crumbs.joined(separator: ",")))
                }
            }
        }
        CFRunLoopAddObserver(CFRunLoopGetMain(), observer, .commonModes)
        self.observer = observer
        // A display link that stops calling back (display asleep, window
        // occluded) must not leave the bench hanging: report what ran.
        DispatchQueue.main.asyncAfter(deadline: .now() + duration + 3) { [weak self] in
            MainActor.assumeIsolated {
                guard let self, self.link != nil else { return }
                FlashTeXLog.write("preview-v2: scroll bench stalled after \(self.intervals.count) frames (window visible \(self.scroll?.window?.occlusionState.contains(.visible) == true), screen \(self.scroll?.window?.screen?.localizedName ?? "none"))")
                self.stalled = true
                self.finish()
            }
        }
        FlashTeXLog.write("preview-v2: scroll bench started (\(duration) s at \(speed) pt/s, screen \(scroll.window?.screen?.localizedName ?? "?") max \(scroll.window?.screen?.maximumFramesPerSecond ?? 0) fps, visible \(scroll.window?.occlusionState.contains(.visible) == true), window \(scroll.window.map { NSStringFromRect($0.frame) } ?? "-"), pane \(NSStringFromRect(scroll.frame)))")
    }

    @objc private func tick(_ link: CADisplayLink) {
        Self.crumb("tick")
        defer { Self.crumb("tick-end") }
        guard let scroll, let doc = scroll.documentView else { return finish() }
        if start == 0 { start = link.timestamp; last = link.timestamp }
        let dt = link.timestamp - last
        if dt > 0 {
            intervals.append(dt * 1000)
            let frame = link.targetTimestamp - link.timestamp
            nominal.append(frame * 1000)
            if frame > 0, dt > frame * 1.5 {
                dropped += Int((dt / frame).rounded()) - 1
                // What the previous tick's scroll step cost on the main thread, and the visible tiles still missing then.
                hitches.append(String(format: "t=%.3fs %.1fms prev-scroll=%.2fms missing-tiles=%d", link.timestamp - start, dt * 1000,
                                      lastScrollMs, lastMissing))
            }
        }
        last = link.timestamp
        let views = Self.pageViews(in: doc)
        let bytes = views.reduce(0) { $0 + $1.retainedBytes }
        for view in views {
            if let source = view.tileSource { pixelsPerPoint = source.pixelsPerPoint; tiledPages = max(tiledPages, views.filter { $0.tileSource != nil }.count) }
            else if let shown = view.shown { bitmapWidth = max(bitmapWidth, shown.width) }
        }
        maxBytes = max(maxBytes, bytes)
        // Visible tiles not yet landed (the backdrop shows there this frame).
        lastMissing = views.reduce(0) { $0 + $1.missingVisibleTiles }
        if lastMissing > 0 { framesMissingTiles += 1; missingTilesMax = max(missingTilesMax, lastMissing) }
        maxFootprint = max(maxFootprint, Self.footprint())
        // Scroll by the time elapsed, bouncing between the ends.
        let clip = scroll.contentView
        var y = clip.bounds.origin.y + direction * CGFloat(speed * max(dt, 0))
        let maxY = max(0, doc.frame.height - clip.bounds.height)
        if y >= maxY { y = maxY; direction = -1 } else if y <= 0 { y = 0; direction = 1 }
        let t0 = MonotonicClock.nowNs()
        clip.scroll(to: NSPoint(x: clip.bounds.origin.x, y: y))
        scroll.reflectScrolledClipView(clip)
        lastScrollMs = Double(MonotonicClock.nowNs() &- t0) / 1e6
        scrollMsMax = max(scrollMsMax, lastScrollMs)
        if link.timestamp - start >= duration { finish() }
    }

    private func finish() {
        link?.invalidate()
        link = nil
        if let observer { CFRunLoopRemoveObserver(CFRunLoopGetMain(), observer, .commonModes) }
        fflush(stdout)
        observer = nil
        let sorted = intervals.sorted()
        func pct(_ p: Double) -> Double { sorted.isEmpty ? 0 : sorted[min(sorted.count - 1, Int(Double(sorted.count - 1) * p))] }
        let nominalMs = nominal.isEmpty ? 0 : nominal.reduce(0, +) / Double(nominal.count)
        let result: [String: Any] = [
            "frames": intervals.count, "dropped_frames": dropped, "nominal_frame_ms": nominalMs,
            "interval_ms_p50": pct(0.5), "interval_ms_p99": pct(0.99), "interval_ms_max": sorted.last ?? 0,
            "tile_jobs_off_main": V2TileGrid.jobs, "tiles_rastered_off_main": V2TileGrid.jobTiles, "tile_job_ms_total": V2TileGrid.jobMs,
            "tile_job_ms_max": V2TileGrid.maxJobMs, "tile_queue_to_install_ms_max": V2TileGrid.maxLatencyMs, "scroll_step_main_ms_max": scrollMsMax,
            "frames_with_missing_visible_tiles": framesMissingTiles, "missing_visible_tiles_max": missingTilesMax,
            "load_average_start": loadStart, "load_average_end": Self.loadAverage(),
            "page_bitmap_bytes_max": maxBytes, "footprint_bytes_max": maxFootprint,
            "tile_threshold_px_per_pt": V2TileGrid.threshold, "speed_pt_per_s": speed, "seconds": duration,
            "tiled_px_per_pt": pixelsPerPoint, "tiled_pages_seen": tiledPages, "whole_page_bitmap_px_width_max": bitmapWidth, "hitches": Array(hitches.prefix(40)),
            "long_main_passes": longPasses, "stalled": stalled,
        ]
        let data = (try? JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])) ?? Data()
        FlashTeXLog.write("preview-v2: scroll bench " + (String(data: data, encoding: .utf8) ?? "").replacingOccurrences(of: "\n", with: " "))
        if let out = ProcessInfo.processInfo.environment["FLASHTEX_V2_SCROLL_BENCH_OUT"] {
            try? data.write(to: URL(fileURLWithPath: out))
        }
        Self.running = nil
    }

    private static func pageViews(in view: NSView) -> [PageBitmapView] {
        var out: [PageBitmapView] = []
        var stack = [view]
        while let v = stack.popLast() {
            if let page = v as? PageBitmapView { out.append(page) } else { stack.append(contentsOf: v.subviews) }
        }
        return out
    }

    /// The 1, 5 and 15 minute load averages (the machine is shared).
    static func loadAverage() -> [Double] {
        var l = [Double](repeating: 0, count: 3)
        return getloadavg(&l, 3) == 3 ? l : []
    }

    /// The process's physical footprint (what Activity Monitor's Memory shows).
    static func footprint() -> Int {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count) }
        }
        return kr == KERN_SUCCESS ? Int(info.phys_footprint) : 0
    }
}

/// Caret and hover marks over a page, drawn only when there is something to mark.
private struct PageV2Marks: View, Equatable {
    let scale: CGFloat
    /// The page is drawn dark (the preview's own toggle): the band needs more opacity there.
    let dark: Bool
    let caretHighlights: [V2Geometry.CaretHighlight]
    let hover: V2Geometry.Hit?

    private func viewRect(_ r: RenderingV2.Rect) -> CGRect {
        CGRect(x: RenderingV2.points(r.x) * scale, y: RenderingV2.points(r.top) * scale,
               width: RenderingV2.points(r.width) * scale, height: RenderingV2.points(r.height) * scale)
    }

    var body: some View {
        Canvas(rendersAsynchronously: false) { context, _ in
            // The caret's paragraph, one faint band per row, under every caret mark.
            for case .paragraph(let band) in caretHighlights {
                let tint = DS.Colors.accentSelection.opacity(dark ? DS.Preview.paragraphBandOpacityDark : DS.Preview.paragraphBandOpacity)
                let overhang = DS.Preview.paragraphBandOverhang * scale
                for row in band.rows {
                    context.fill(Path(roundedRect: viewRect(row).insetBy(dx: -overhang, dy: -overhang / 2), cornerRadius: DS.Space.xxs), with: .color(tint))
                }
            }
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
                        for r in m.hitRects { context.fill(Path(viewRect(r)), with: .color(DS.Colors.accentSelection.opacity(DS.Preview.hoverHighlightOpacity))) }
                    } else {
                        for r in m.hitRects { context.fill(Path(viewRect(r)), with: .color(DS.Colors.accentSelection.opacity(DS.Preview.caretHighlightOpacity))) }
                    }
                case .formula(let box):
                    let outline = viewRect(box.bounds).insetBy(dx: -2, dy: -2)
                    context.fill(Path(roundedRect: outline, cornerRadius: DS.Space.xxs), with: .color(DS.Colors.accentSelection.opacity(DS.Preview.hoverHighlightOpacity)))
                    context.stroke(Path(roundedRect: outline, cornerRadius: DS.Space.xxs), with: .color(DS.Colors.accentSelection.opacity(DS.Preview.linkBoxStrokeOpacity)), lineWidth: DS.Size.hairline)
                    for r in box.rects { context.fill(Path(viewRect(r)), with: .color(DS.Colors.accentSelection.opacity(DS.Preview.linkBoxFillOpacity))) }
                case .paragraph:
                    break // drawn first, above
                }
            }
            if let hover { context.fill(Path(viewRect(hover.rect).insetBy(dx: -DS.Size.hairline, dy: -DS.Size.hairline)), with: .color(DS.Colors.accentSelection.opacity(DS.Preview.caretHighlightOpacity))) }
        }
    }
}
